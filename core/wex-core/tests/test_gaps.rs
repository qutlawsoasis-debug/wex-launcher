//! Local-only regression tests for download integrity and instance installation.
use std::fs;
use std::sync::Mutex;
use std::time::Duration;
use tempfile::tempdir;
use tokio_util::sync::CancellationToken;
use wex_core::config::{LauncherConfig, WexPaths};
use wex_core::download::{
    compute_file_hash, download_file_resumable, download_file_with_fallback, download_pool,
    download_pool_cancellable, verify_file, DownloadTask, HashType,
};
use wex_core::error::WexError;
use wex_core::event::{ChannelEventHandler, Event, EventHandler, NoopEventHandler};
use wex_core::fabric::{
    install_fabric_libraries, resolve_fabric_libraries, FabricLibrary, FabricProfile,
};
use wex_core::java::{ensure_mojang_java, get_java_binary_path, get_java_component_for_version};
use wex_core::minecraft::{install_minecraft_client, VersionPackage};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const VERSIONS: [&str; 2] = ["1.20.1", "1.21.1"];
const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap()
}

#[derive(Default)]
struct Recorder(Mutex<Vec<Event>>);
impl EventHandler for Recorder {
    fn on_event(&self, event: &Event) {
        self.0.lock().unwrap().push(event.clone());
    }
}

#[test]
fn verify_file_accepts_case_insensitive_hash_and_size() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("payload");
    fs::write(&file, b"hello").unwrap();
    assert!(verify_file(
        &file,
        Some(5),
        Some((&HELLO_SHA1.to_uppercase(), HashType::Sha1))
    ));
}

#[test]
fn verify_file_rejects_missing_size_and_hash_mismatches() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("payload");
    assert!(!verify_file(&file, None, None));
    fs::write(&file, b"hello").unwrap();
    assert!(!verify_file(&file, Some(6), None));
    assert!(!verify_file(
        &file,
        Some(5),
        Some(("deadbeef", HashType::Sha1))
    ));
}

#[test]
fn compute_file_hash_sha256_known_vector() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("payload");
    fs::write(&file, b"hello").unwrap();
    assert_eq!(
        compute_file_hash(&file, HashType::Sha256).unwrap(),
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn compute_file_hash_missing_file_returns_io_error() {
    let tmp = tempdir().unwrap();
    assert!(matches!(
        compute_file_hash(&tmp.path().join("missing"), HashType::Sha1),
        Err(WexError::Io(_))
    ));
}

#[tokio::test]
async fn download_file_resumable_verifies_promotes_and_emits_event() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/client"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("nested/client.jar");
    let events = Recorder::default();
    assert!(download_file_resumable(
        &client(),
        &format!("{}/client", server.uri()),
        &file,
        Some(5),
        Some((HELLO_SHA1, HashType::Sha1)),
        &events
    )
    .await
    .unwrap());
    assert_eq!(fs::read(&file).unwrap(), b"hello");
    assert!(!file.with_extension("jar.part").exists());
    assert!(events
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|e| matches!(e, Event::FileDownloaded { bytes: 5, .. })));
}

#[tokio::test]
async fn download_file_resumable_cached_file_avoids_http() {
    let server = MockServer::start().await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("client.jar");
    fs::write(&file, b"hello").unwrap();
    assert!(!download_file_resumable(
        &client(),
        &server.uri(),
        &file,
        Some(5),
        Some((HELLO_SHA1, HashType::Sha1)),
        &NoopEventHandler
    )
    .await
    .unwrap());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn download_file_resumable_range_206_appends_partial_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("range", "bytes=2-"))
        .respond_with(ResponseTemplate::new(206).set_body_bytes(b"llo".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("client.jar");
    fs::write(tmp.path().join("client.jar.part"), b"he").unwrap();
    download_file_resumable(
        &client(),
        &server.uri(),
        &file,
        Some(5),
        Some((HELLO_SHA1, HashType::Sha1)),
        &NoopEventHandler,
    )
    .await
    .unwrap();
    assert_eq!(fs::read(file).unwrap(), b"hello");
}

#[tokio::test]
async fn download_file_resumable_range_ignored_restarts_without_duplicate_prefix() {
    let server = MockServer::start().await;
    Mock::given(header("range", "bytes=2-"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("client.jar");
    fs::write(tmp.path().join("client.jar.part"), b"he").unwrap();
    download_file_resumable(
        &client(),
        &server.uri(),
        &file,
        Some(5),
        Some((HELLO_SHA1, HashType::Sha1)),
        &NoopEventHandler,
    )
    .await
    .unwrap();
    assert_eq!(fs::read(file).unwrap(), b"hello");
}

#[tokio::test]
async fn download_file_resumable_bad_hash_never_replaces_existing_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(3)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("client.jar");
    fs::write(&file, b"original").unwrap();
    let result = download_file_resumable(
        &client(),
        &server.uri(),
        &file,
        Some(5),
        Some(("bad", HashType::Sha1)),
        &NoopEventHandler,
    )
    .await;
    assert!(matches!(result, Err(WexError::HashMismatch { .. })));
    assert_eq!(fs::read(file).unwrap(), b"original");
    assert!(!tmp.path().join("client.jar.part").exists());
}

#[tokio::test]
async fn download_file_resumable_bad_size_cleans_partial_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"short".to_vec()))
        .expect(3)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("client.jar");
    assert!(matches!(
        download_file_resumable(
            &client(),
            &server.uri(),
            &file,
            Some(99),
            None,
            &NoopEventHandler
        )
        .await,
        Err(WexError::SizeMismatch {
            expected: 99,
            actual: 5,
            ..
        })
    ));
    assert!(!file.exists());
    assert!(!tmp.path().join("client.jar.part").exists());
}

#[tokio::test]
async fn download_file_resumable_http_error_leaves_no_destination() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .expect(3)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("missing.jar");
    assert!(matches!(
        download_file_resumable(
            &client(),
            &server.uri(),
            &file,
            None,
            None,
            &NoopEventHandler
        )
        .await,
        Err(WexError::DownloadError { .. })
    ));
    assert!(!file.exists());
}

#[tokio::test]
async fn download_file_with_fallback_recovers_from_primary_failure() {
    let server = MockServer::start().await;
    Mock::given(path("/primary"))
        .respond_with(ResponseTemplate::new(404))
        .expect(3)
        .mount(&server)
        .await;
    Mock::given(path("/fallback"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("client.jar");
    assert!(download_file_with_fallback(
        &client(),
        &format!("{}/primary", server.uri()),
        Some(&format!("{}/fallback", server.uri())),
        &file,
        Some(5),
        Some((HELLO_SHA1, HashType::Sha1)),
        &NoopEventHandler,
        None
    )
    .await
    .unwrap());
    assert_eq!(fs::read(file).unwrap(), b"hello");
}

#[tokio::test]
async fn download_file_with_fallback_preserves_cancellation_without_http() {
    let server = MockServer::start().await;
    let tmp = tempdir().unwrap();
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        download_file_with_fallback(
            &client(),
            &server.uri(),
            Some(&server.uri()),
            &tmp.path().join("client.jar"),
            None,
            None,
            &NoopEventHandler,
            Some(&token)
        )
        .await,
        Err(WexError::Cancelled)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn download_pool_reports_total_bytes_and_completion() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(2)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let events = Recorder::default();
    let tasks = VERSIONS
        .iter()
        .map(|v| DownloadTask {
            url: server.uri(),
            fallback_url: None,
            target_path: tmp.path().join(v),
            size: Some(5),
            hash: Some((HELLO_SHA1.into(), HashType::Sha1)),
        })
        .collect();
    assert_eq!(
        download_pool(&client(), tasks, 2, &events, "clients")
            .await
            .unwrap(),
        10
    );
    assert!(events.0.lock().unwrap().iter().any(|e| matches!(
        e,
        Event::Progress {
            current: 2,
            total: 2,
            ..
        }
    )));
}

#[tokio::test]
async fn download_pool_cancellable_rejects_without_http() {
    let server = MockServer::start().await;
    let tmp = tempdir().unwrap();
    let token = CancellationToken::new();
    token.cancel();
    let task = DownloadTask {
        url: server.uri(),
        fallback_url: None,
        target_path: tmp.path().join("never"),
        size: None,
        hash: None,
    };
    assert!(matches!(
        download_pool_cancellable(
            &client(),
            vec![task],
            2,
            &NoopEventHandler,
            "clients",
            Some(token)
        )
        .await,
        Err(WexError::Cancelled)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn download_pool_empty_is_successful_noop() {
    assert_eq!(
        download_pool(&client(), Vec::new(), 2, &NoopEventHandler, "empty")
            .await
            .unwrap(),
        0
    );
}

fn package(version: &str, url: &str) -> VersionPackage {
    serde_json::from_value(serde_json::json!({"id":version,"mainClass":"example.Main","downloads":{"client":{"sha1":HELLO_SHA1,"size":5,"url":url}},"libraries":[],"assetIndex":{"id":"test","sha1":"unused","size":0,"url":"unused"},"javaVersion":null})).unwrap()
}

#[tokio::test]
async fn install_minecraft_client_success_both_versions_are_isolated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(2)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
    for version in VERSIONS {
        let file = install_minecraft_client(
            &paths,
            version,
            &package(version, &server.uri()),
            &client(),
            &NoopEventHandler,
        )
        .await
        .unwrap();
        assert_eq!(file, paths.client_jar(version));
        assert_eq!(fs::read(file).unwrap(), b"hello");
    }
}

#[tokio::test]
async fn install_minecraft_client_rejects_integrity_failure_both_versions() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"wrong".to_vec()))
        .expect(6)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
    for version in VERSIONS {
        assert!(matches!(
            install_minecraft_client(
                &paths,
                version,
                &package(version, &server.uri()),
                &client(),
                &NoopEventHandler
            )
            .await,
            Err(WexError::HashMismatch { .. })
        ));
        assert!(!paths.client_jar(version).exists());
    }
}

fn fabric(url: &str) -> FabricProfile {
    FabricProfile {
        id: "fixture".into(),
        main_class: "example.Main".into(),
        libraries: vec![FabricLibrary {
            name: "org.example:loader:1.0".into(),
            url: Some(url.into()),
            sha1: Some(HELLO_SHA1.into()),
            sha256: None,
            size: Some(5),
        }],
    }
}

#[tokio::test]
async fn install_fabric_libraries_verifies_and_matches_resolution() {
    let server = MockServer::start().await;
    Mock::given(path("/org/example/loader/1.0/loader-1.0.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
        .expect(1)
        .mount(&server)
        .await;
    let tmp = tempdir().unwrap();
    let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
    let profile = fabric(&server.uri());
    let jars = install_fabric_libraries(&paths, &profile, &client(), &NoopEventHandler, 2)
        .await
        .unwrap();
    assert_eq!(jars, resolve_fabric_libraries(&paths, &profile));
    assert_eq!(fs::read(&jars[0]).unwrap(), b"hello");
}

#[tokio::test]
async fn install_fabric_libraries_bad_coordinate_errors_before_network() {
    let server = MockServer::start().await;
    let tmp = tempdir().unwrap();
    let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
    let mut profile = fabric(&server.uri());
    profile.libraries[0].name = "invalid".into();
    assert!(matches!(
        install_fabric_libraries(&paths, &profile, &client(), &NoopEventHandler, 2).await,
        Err(WexError::ManifestError(_))
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn ensure_mojang_java_cached_runtime_never_accesses_network_both_versions() {
    let tmp = tempdir().unwrap();
    let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
    for version in VERSIONS {
        let component = get_java_component_for_version(version);
        let exe = get_java_binary_path(&paths.runtime_dir, component, true);
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, b"fixture, not executable").unwrap();
        fs::write(
            paths.runtime_dir.join(component).join("release"),
            b"JAVA_VERSION=fixture",
        )
        .unwrap();
        assert_eq!(
            ensure_mojang_java(&paths, version, &client(), &NoopEventHandler, 2)
                .await
                .unwrap(),
            exe
        );
    }
}

#[test]
fn launcher_config_save_load_roundtrip_keeps_custom_settings() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("nested/config.json");
    let cfg = LauncherConfig {
        nickname: "Fixture".into(),
        selected_version: "1.20.1".into(),
        max_memory_mb: 6144,
        reduce_motion: true,
        ..Default::default()
    };
    cfg.save(&file).unwrap();
    let read = LauncherConfig::load_or_default(&file);
    assert_eq!(read.nickname, "Fixture");
    assert_eq!(read.selected_version, "1.20.1");
    assert_eq!(read.max_memory_mb, 6144);
    assert!(read.reduce_motion);
}

#[test]
fn launcher_config_save_file_as_parent_returns_io_error() {
    let tmp = tempdir().unwrap();
    let blocker = tmp.path().join("blocker");
    fs::write(&blocker, b"file").unwrap();
    assert!(matches!(
        LauncherConfig::default().save(&blocker.join("config.json")),
        Err(WexError::Io(_))
    ));
}

#[test]
fn launcher_config_corrupt_json_uses_defaults_without_overwriting() {
    let tmp = tempdir().unwrap();
    let file = tmp.path().join("config.json");
    fs::write(&file, b"not json").unwrap();
    let cfg = LauncherConfig::load_or_default(&file);
    assert_eq!(cfg.nickname, LauncherConfig::default().nickname);
    assert_eq!(fs::read(file).unwrap(), b"not json");
}

#[tokio::test]
async fn channel_event_handler_keeps_order_and_payload() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let handler = ChannelEventHandler::new(tx);
    handler.on_event(&Event::GameLaunched { pid: 42 });
    handler.on_event(&Event::GameOutput {
        line: "ready".into(),
    });
    assert!(matches!(
        rx.recv().await,
        Some(Event::GameLaunched { pid: 42 })
    ));
    assert!(matches!(rx.recv().await,Some(Event::GameOutput{line}) if line=="ready"));
}

#[test]
fn channel_event_handler_disconnected_receiver_does_not_panic() {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    drop(rx);
    ChannelEventHandler::new(tx).on_event(&Event::GameExited { exit_code: Some(0) });
}
