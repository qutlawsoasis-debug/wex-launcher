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
