use super::*;
use crate::download::{compute_file_hash, HashType};
use crate::event::NoopEventHandler;
use serde_json::json;
use tempfile::{tempdir, TempDir};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct CachedEnvironment {
    _temp: TempDir,
    paths: WexPaths,
    client: reqwest::Client,
}

fn write_json(path: &Path, value: serde_json::Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

fn cached_environment(version: &str) -> CachedEnvironment {
    let temp = tempdir().unwrap();
    let paths = WexPaths::new(Some(temp.path().join("Wex")));
    paths.ensure_dirs().unwrap();
    paths.ensure_instance_dirs(version).unwrap();
    let jar = paths.client_jar(version);
    fs::create_dir_all(jar.parent().unwrap()).unwrap();
    fs::write(&jar, b"verified client").unwrap();
    let index = paths.assets_dir.join("indexes/5.json");
    write_json(&index, json!({"objects": {}}));
    write_json(
        &paths
            .versions_dir
            .join(version)
            .join(format!("{version}.json")),
        json!({
            "id": version, "mainClass": "net.minecraft.client.main.Main",
            "downloads": {"client": {"sha1": compute_file_hash(&jar, HashType::Sha1).unwrap(),
                "size": fs::metadata(&jar).unwrap().len(), "url": "http://127.0.0.1:1/client.jar"}},
            "libraries": [], "assetIndex": {"id": "5",
                "sha1": compute_file_hash(&index, HashType::Sha1).unwrap(),
                "size": fs::metadata(&index).unwrap().len(), "url": "http://127.0.0.1:1/index.json"}
        }),
    );
    write_json(
        &paths
            .versions_dir
            .join(version)
            .join(format!("fabric-{version}.json")),
        json!({
            "id": "fabric-test", "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient", "libraries": []
        }),
    );
    write_json(
        &paths.instance_dir(version).join("manifest.cache.json"),
        json!({
            "name": "test", "packVersion": "test", "minecraft": version, "fabricLoader": "0.19.5", "files": []
        }),
    );
    write_json(
        &paths.instance_dir(version).join("pack.state.json"),
        json!({
            "packVersion": "test", "minecraft": version, "fabricLoader": "0.19.5", "installedAtUtc": "test", "installedMods": []
        }),
    );
    let runtime = paths
        .runtime_dir
        .join(crate::java::get_java_component_for_version(version));
    fs::create_dir_all(runtime.join("bin")).unwrap();
    fs::write(runtime.join("release"), "JAVA_VERSION=test\n").unwrap();
    fs::write(runtime.join("bin/javaw.exe"), b"not executed").unwrap();
    fs::write(runtime.join("bin/java"), b"not executed").unwrap();
    fs::write(
        runtime.join(crate::java::RUNTIME_COMPLETE_MARKER),
        b"fixture",
    )
    .unwrap();
    // Block all metadata hosts deterministically; only literal loopback artifact URLs are used.
    let client = reqwest::Client::builder()
        .no_proxy()
        .resolve_to_addrs("piston-meta.mojang.com", &[])
        .resolve_to_addrs("meta.fabricmc.net", &[])
        .resolve_to_addrs("raw.githubusercontent.com", &[])
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    CachedEnvironment {
        _temp: temp,
        paths,
        client,
    }
}

#[tokio::test]
async fn integrity_verified_offline_environment_succeeds_both_versions() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let result = install_full_environment(
            &fixture.paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            2,
        )
        .await;
        assert!(result.is_ok(), "{version}: {result:?}");
        assert!(fast_path_state_path(&fixture.paths, version).exists());
    }
}

async fn rejects_failed_stage(stage: &str) {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"wrong".to_vec()))
            .mount(&server)
            .await;
        let pkg_path = paths
            .versions_dir
            .join(version)
            .join(format!("{version}.json"));
        let mut pkg: serde_json::Value =
            serde_json::from_slice(&fs::read(&pkg_path).unwrap()).unwrap();
        let artifact = json!({"path": "test/broken.jar", "url": format!("{}/broken", server.uri()), "size": 5, "sha1": "0000000000000000000000000000000000000000"});
        match stage {
            "client" => {
                pkg["downloads"]["client"] = artifact.clone();
                fs::write(paths.client_jar(version), b"wrong").unwrap();
            }
            "vanilla" => {
                pkg["libraries"] =
                    json!([{"name": "test:broken:1", "downloads": {"artifact": artifact}}])
            }
            "assets" => {
                pkg["assetIndex"]["sha1"] = json!("0000000000000000000000000000000000000000");
                pkg["assetIndex"]["url"] = json!(format!("{}/broken", server.uri()));
                pkg["assetIndex"]["size"] = json!(5);
            }
            "fabric" => write_json(
                &paths
                    .versions_dir
                    .join(version)
                    .join(format!("fabric-{version}.json")),
                json!({
                    "id": "fabric-test", "mainClass": "test", "libraries": [{"name": "test:broken:1",
                        "url": server.uri(), "size": 5, "sha1": "0000000000000000000000000000000000000000"}]
                }),
            ),
            "pack" => write_json(
                &paths.instance_dir(version).join("manifest.cache.json"),
                json!({
                    "name": "test", "packVersion": "test", "minecraft": version, "fabricLoader": "0.19.5", "files": [{
                        "path": "mods/broken.jar", "size": 5, "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                        "url": format!("{}/broken", server.uri())
                    }]
                }),
            ),
            _ => panic!("unknown stage"),
        }
        write_json(&pkg_path, pkg);
        // A previously successful launch must not leave a stale ready marker after failure.
        mark_fast_path_verified(paths, version).unwrap();
        let result =
            install_full_environment(paths, version, &fixture.client, &NoopEventHandler, 2).await;
        assert!(
            result.is_err(),
            "{version} {stage}: failure was swallowed: {result:?}"
        );
        assert!(
            !fast_path_state_path(paths, version).exists(),
            "{version} {stage}: stale ready marker"
        );
    }
}

#[tokio::test]
async fn integrity_client_hash_failure_is_not_offline_success() {
    rejects_failed_stage("client").await;
}
#[tokio::test]
async fn integrity_vanilla_failure_is_not_offline_success() {
    rejects_failed_stage("vanilla").await;
}
#[tokio::test]
async fn integrity_assets_failure_is_not_offline_success() {
    rejects_failed_stage("assets").await;
}
#[tokio::test]
async fn integrity_fabric_failure_is_not_offline_success() {
    rejects_failed_stage("fabric").await;
}
#[tokio::test]
async fn integrity_pack_failure_is_not_offline_success() {
    rejects_failed_stage("pack").await;
}

#[tokio::test]
async fn integrity_offline_fallback_rejects_http_json_and_disk_errors() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    Mock::given(wiremock::matchers::path("/malformed"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not JSON"))
        .mount(&server)
        .await;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let rejected = client
        .get(format!("{}/missing", server.uri()))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap_err();
    assert!(!WexError::Network(rejected).is_network_unavailable());
    let malformed = client
        .get(format!("{}/malformed", server.uri()))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap_err();
    assert!(!WexError::Network(malformed).is_network_unavailable());
    assert!(
        !WexError::Io(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            .is_network_unavailable()
    );
    assert!(!WexError::Cancelled.is_network_unavailable());
    let fixture = cached_environment("1.21.1");
    let offline = fixture
        .client
        .get(crate::minecraft::PISTON_VERSION_MANIFEST_URL)
        .send()
        .await
        .unwrap_err();
    assert!(WexError::Network(offline).is_network_unavailable());
}

#[test]
fn integrity_ready_marker_write_failure_is_reported() {
    let fixture = cached_environment("1.21.1");
    fs::create_dir(fast_path_state_path(&fixture.paths, "1.21.1")).unwrap();
    assert!(mark_fast_path_verified(&fixture.paths, "1.21.1").is_err());
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(data))
}

fn installed_pack_state(paths: &WexPaths, version: &str) -> crate::pack::PackState {
    crate::pack::read_pack_state(paths, version).expect("pack.state.json")
}

/// A fast launch must not keep an old Aura-Pack forever: a newer published
/// manifest is synced before the game starts, then the fast path is quiet again.
#[tokio::test]
async fn fast_launch_syncs_pack_when_manifest_version_bumps() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        // First install leaves a ready fast path with pack "test".
        install_full_environment(paths, version, &fixture.client, &NoopEventHandler, 2)
            .await
            .unwrap();
        assert!(is_fast_path_ready(paths, version));
        assert_eq!(installed_pack_state(paths, version).pack_version, "test");

        let server = MockServer::start().await;
        let body = b"new mod".to_vec();
        let manifest = json!({
            "name": "test", "packVersion": "v2", "minecraft": version, "fabricLoader": "0.19.5",
            "files": [{"path": "mods/new.jar", "size": body.len(), "sha256": sha256_hex(&body),
                "url": format!("{}/files/new.jar", server.uri())}]
        });
        Mock::given(wiremock::matchers::path("/manifest.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&manifest))
            .mount(&server)
            .await;
        Mock::given(wiremock::matchers::path("/files/new.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let url = format!("{}/manifest.json", server.uri());

        ensure_pack_current_at(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            2,
            None,
            &url,
        )
        .await
        .unwrap();
        let mod_path = paths.instance_dir(version).join("mods/new.jar");
        assert_eq!(fs::read(&mod_path).unwrap(), body, "{version}");
        assert_eq!(installed_pack_state(paths, version).pack_version, "v2");
        assert!(is_fast_path_ready(paths, version));

        // Same manifest on the next Play: nothing is downloaded again.
        ensure_pack_current_at(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            2,
            None,
            &url,
        )
        .await
        .unwrap();
        server.verify().await;
    }
}

#[tokio::test]
async fn fast_launch_offline_keeps_installed_pack() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        install_full_environment(paths, version, &fixture.client, &NoopEventHandler, 2)
            .await
            .unwrap();
        let events = WarnCollector::default();
        let res = ensure_pack_current_at(
            paths,
            version,
            &fixture.client,
            &events,
            2,
            None,
            "http://127.0.0.1:1/manifest.json",
        )
        .await;
        assert!(res.is_ok(), "{version}: {res:?}");
        assert_eq!(installed_pack_state(paths, version).pack_version, "test");
        assert!(is_fast_path_ready(paths, version));
        let logged = events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.contains("Aura-Pack"));
        assert!(
            logged,
            "{version}: offline fast path must say why the pack was not checked"
        );
    }
}

#[test]
fn fast_path_requires_complete_java_runtime() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        mark_fast_path_verified(&fixture.paths, version).unwrap();
        assert!(is_fast_path_ready(&fixture.paths, version));
        let component = crate::java::get_java_component_for_version(version);
        fs::remove_file(crate::java::runtime_complete_marker(
            &fixture.paths.runtime_dir,
            component,
        ))
        .unwrap();
        assert!(!is_fast_path_ready(&fixture.paths, version), "{version}");
    }
}

/// Cancels the token when the first request arrives, like the Cancel button
/// pressed while the first library downloads.
struct CancelOnRequest(CancellationToken);
impl wiremock::Respond for CancelOnRequest {
    fn respond(&self, _: &wiremock::Request) -> ResponseTemplate {
        self.0.cancel();
        ResponseTemplate::new(200).set_body_bytes(b"lib".to_vec())
    }
}

#[tokio::test]
async fn cancelled_install_stops_and_makes_no_further_requests() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        let server = MockServer::start().await;
        let token = CancellationToken::new();
        Mock::given(wiremock::matchers::path_regex("^/libs/"))
            .respond_with(CancelOnRequest(token.clone()))
            .mount(&server)
            .await;
        Mock::given(wiremock::matchers::path_regex("^/pack/"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let pkg_path = paths
            .versions_dir
            .join(version)
            .join(format!("{version}.json"));
        let mut pkg: serde_json::Value =
            serde_json::from_slice(&fs::read(&pkg_path).unwrap()).unwrap();
        let lib = |n: u32| {
            json!({"name": format!("test:lib{n}:1"), "downloads": {"artifact": {
                "path": format!("test/lib{n}.jar"), "url": format!("{}/libs/{n}", server.uri()),
                "size": 3, "sha1": compute_sha1(b"lib")}}})
        };
        pkg["libraries"] = json!([lib(1), lib(2), lib(3), lib(4)]);
        write_json(&pkg_path, pkg);
        write_json(
            &paths.instance_dir(version).join("manifest.cache.json"),
            json!({"name": "test", "packVersion": "test", "minecraft": version, "fabricLoader": "0.19.5",
                "files": [{"path": "mods/x.jar", "size": 1, "sha256": "00",
                    "url": format!("{}/pack/x.jar", server.uri())}]}),
        );
        let res = install_full_environment_cancellable(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            1,
            Some(&token),
        )
        .await;
        assert!(
            matches!(res, Err(WexError::Cancelled)),
            "{version}: {res:?}"
        );
        let lib_requests = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path().starts_with("/libs/"))
            .count();
        assert_eq!(
            lib_requests, 1,
            "{version}: downloads continued after Cancel"
        );
        assert!(!fast_path_state_path(paths, version).exists());
        server.verify().await;

        // Already cancelled: not a single request.
        let before = server.received_requests().await.unwrap().len();
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        let res = install_full_environment_cancellable(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            1,
            Some(&cancelled),
        )
        .await;
        assert!(matches!(res, Err(WexError::Cancelled)));
        assert_eq!(server.received_requests().await.unwrap().len(), before);
    }
}

#[derive(Default)]
struct WarnCollector(std::sync::Mutex<Vec<String>>);
impl EventHandler for WarnCollector {
    fn on_event(&self, event: &Event) {
        if let Event::Log {
            level: LogLevel::Warn,
            message,
        } = event
        {
            self.0.lock().unwrap().push(message.clone());
        }
    }
}

fn compute_sha1(data: &[u8]) -> String {
    use sha1::Digest as _;
    hex::encode(sha1::Sha1::digest(data))
}

/// Installs a published pack "v1" with `mods/a.jar` (`old`) through the fast-path check.
async fn install_pack_v1(fixture: &CachedEnvironment, version: &str, server: &MockServer) {
    install_full_environment(
        &fixture.paths,
        version,
        &fixture.client,
        &NoopEventHandler,
        2,
    )
    .await
    .unwrap();
    let old = b"old".to_vec();
    let v1 = json!({"name": "test", "packVersion": "v1", "minecraft": version, "fabricLoader": "0.19.5",
        "files": [{"path": "mods/a.jar", "size": old.len(), "sha256": sha256_hex(&old),
            "url": format!("{}/v1/a.jar", server.uri())}]});
    Mock::given(wiremock::matchers::path("/v1.json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&v1))
        .mount(server)
        .await;
    Mock::given(wiremock::matchers::path("/v1/a.jar"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(old))
        .mount(server)
        .await;
    let url = format!("{}/v1.json", server.uri());
    assert!(ensure_pack_current_at(
        &fixture.paths,
        version,
        &fixture.client,
        &NoopEventHandler,
        2,
        None,
        &url
    )
    .await
    .unwrap());
    assert_eq!(
        installed_pack_state(&fixture.paths, version).pack_version,
        "v1"
    );
}

/// Manifest "v2": `mods/a.jar` becomes `new`, plus `mods/b.jar` at `b_url`.
fn manifest_v2(version: &str, server: &MockServer, b_url: &str) -> serde_json::Value {
    let new = b"new".to_vec();
    let b = b"bbb".to_vec();
    json!({"name": "test", "packVersion": "v2", "minecraft": version, "fabricLoader": "0.19.5",
    "files": [
        {"path": "mods/a.jar", "size": new.len(), "sha256": sha256_hex(&new),
            "url": format!("{}/v2/a.jar", server.uri())},
        {"path": "mods/b.jar", "size": b.len(), "sha256": sha256_hex(&b), "url": b_url}
    ]})
}

fn mods_listing(paths: &WexPaths, version: &str) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<_> = fs::read_dir(paths.instance_dir(version).join("mods"))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

/// The connection drops in the middle of a pack update: nothing new reaches
/// mods/, the old pack stays whole and the fast launch goes on with it.
#[tokio::test]
async fn dropped_network_mid_sync_keeps_old_pack_and_launches() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        let server = MockServer::start().await;
        install_pack_v1(&fixture, version, &server).await;
        let before = mods_listing(paths, version);

        Mock::given(wiremock::matchers::path("/v2/a.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"new".to_vec()))
            .mount(&server)
            .await;
        // b.jar: the connection is refused (network gone).
        let v2 = manifest_v2(version, &server, "http://127.0.0.1:1/b.jar");
        Mock::given(wiremock::matchers::path("/v2.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v2))
            .mount(&server)
            .await;
        let url = format!("{}/v2.json", server.uri());
        let events = WarnCollector::default();
        let verified =
            ensure_pack_current_at(paths, version, &fixture.client, &events, 1, None, &url)
                .await
                .expect("a failed update must not block Play");
        assert!(!verified);
        assert_eq!(
            mods_listing(paths, version),
            before,
            "{version}: mods/ changed"
        );
        assert_eq!(installed_pack_state(paths, version).pack_version, "v1");
        assert!(!crate::pack::has_pending_pack_commit(paths, version));
        assert!(
            is_fast_path_ready(paths, version),
            "{version}: old pack must launch"
        );
        assert!(events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|m| m.contains("Aura-Pack")));
    }
}

/// Cancel while the update downloads: mods/ is exactly as before.
#[tokio::test]
async fn cancel_mid_sync_leaves_mods_unchanged() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        let server = MockServer::start().await;
        install_pack_v1(&fixture, version, &server).await;
        let before = mods_listing(paths, version);
        let token = CancellationToken::new();
        Mock::given(wiremock::matchers::path("/v2/a.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"new".to_vec()))
            .mount(&server)
            .await;
        Mock::given(wiremock::matchers::path("/v2/b.jar"))
            .respond_with(CancelOnRequest(token.clone()))
            .mount(&server)
            .await;
        let v2 = manifest_v2(version, &server, &format!("{}/v2/b.jar", server.uri()));
        Mock::given(wiremock::matchers::path("/v2.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v2))
            .mount(&server)
            .await;
        let url = format!("{}/v2.json", server.uri());
        let res = ensure_pack_current_at(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            1,
            Some(&token),
            &url,
        )
        .await;
        assert!(
            matches!(res, Err(WexError::Cancelled)),
            "{version}: {res:?}"
        );
        assert_eq!(
            mods_listing(paths, version),
            before,
            "{version}: mods/ changed"
        );
        assert_eq!(installed_pack_state(paths, version).pack_version, "v1");
    }
}

/// A crash between staging and commit: the fast path refuses the instance and
/// the next start finishes the commit, offline.
#[tokio::test]
async fn interrupted_commit_is_finished_before_launch() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        let server = MockServer::start().await;
        install_pack_v1(&fixture, version, &server).await;
        let staging = crate::pack::pack_staging_dir(paths, version);
        fs::create_dir_all(staging.join("mods")).unwrap();
        fs::write(staging.join("mods/a.jar"), b"new").unwrap();
        fs::write(staging.join("mods/b.jar"), b"bbb").unwrap();
        let v2 = manifest_v2(version, &server, "http://127.0.0.1:1/b.jar");
        write_json(
            &paths
                .instance_dir(version)
                .join(crate::pack::PACK_COMMIT_PLAN),
            json!({"manifest": v2, "staged": ["mods/a.jar", "mods/b.jar"]}),
        );
        assert!(
            !is_fast_path_ready(paths, version),
            "{version}: half-committed pack"
        );

        ensure_pack_current_at(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            1,
            None,
            "http://127.0.0.1:1/manifest.json",
        )
        .await
        .unwrap();
        assert_eq!(
            mods_listing(paths, version),
            vec![
                ("a.jar".to_string(), b"new".to_vec()),
                ("b.jar".to_string(), b"bbb".to_vec())
            ]
        );
        assert_eq!(installed_pack_state(paths, version).pack_version, "v2");
        assert!(!staging.exists());
        assert!(is_fast_path_ready(paths, version));
    }
}

/// Files staged by an interrupted download are reused: the next sync does not
/// download them again.
#[tokio::test]
async fn staged_downloads_resume_on_the_next_sync() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let paths = &fixture.paths;
        let server = MockServer::start().await;
        install_pack_v1(&fixture, version, &server).await;
        let staging = crate::pack::pack_staging_dir(paths, version);
        fs::create_dir_all(staging.join("mods")).unwrap();
        fs::write(staging.join("mods/a.jar"), b"new").unwrap();
        assert!(
            is_fast_path_ready(paths, version),
            "leftover downloads keep the old pack"
        );
        Mock::given(wiremock::matchers::path("/v2/a.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"new".to_vec()))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(wiremock::matchers::path("/v2/b.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"bbb".to_vec()))
            .expect(1)
            .mount(&server)
            .await;
        let v2 = manifest_v2(version, &server, &format!("{}/v2/b.jar", server.uri()));
        Mock::given(wiremock::matchers::path("/v2.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&v2))
            .mount(&server)
            .await;
        let url = format!("{}/v2.json", server.uri());
        assert!(ensure_pack_current_at(
            paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            1,
            None,
            &url
        )
        .await
        .unwrap());
        assert_eq!(
            fs::read(paths.instance_dir(version).join("mods/a.jar")).unwrap(),
            b"new"
        );
        assert_eq!(installed_pack_state(paths, version).pack_version, "v2");
        server.verify().await;
    }
}

/// After the fast-path check verified the pack, a fallback full install does not
/// fetch the manifest and hash every mod again.
#[tokio::test]
async fn verified_pack_is_not_synced_twice_by_the_fallback_install() {
    for version in ["1.20.1", "1.21.1"] {
        let fixture = cached_environment(version);
        let server = MockServer::start().await;
        Mock::given(wiremock::matchers::method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        write_json(
            &fixture
                .paths
                .instance_dir(version)
                .join("manifest.cache.json"),
            json!({"name": "test", "packVersion": "test", "minecraft": version, "fabricLoader": "0.19.5",
                "files": [{"path": "mods/x.jar", "size": 1, "sha256": "00",
                    "url": format!("{}/pack/x.jar", server.uri())}]}),
        );
        let res = install_full_environment_impl(
            &fixture.paths,
            version,
            &fixture.client,
            &NoopEventHandler,
            2,
            None,
            true,
        )
        .await;
        assert!(res.is_ok(), "{version}: {res:?}");
        server.verify().await;
    }
}
