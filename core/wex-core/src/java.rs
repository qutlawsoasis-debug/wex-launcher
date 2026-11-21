use crate::config::WexPaths;
use crate::download::{download_pool_cancellable, DownloadTask, HashType};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

/// Written into a runtime folder only after every file of the runtime manifest was
/// downloaded and verified. Holds the SHA-1 of that manifest.
pub const RUNTIME_COMPLETE_MARKER: &str = ".wex-complete";

pub const MOJANG_JAVA_ALL_JSON: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRuntimeManifestRef {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRuntimeEntry {
    pub manifest: MojangRuntimeManifestRef,
    pub version: Option<MojangRuntimeVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRuntimeVersionInfo {
    pub name: String,
    pub released: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangFileDownloadInfo {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangFileDownloads {
    pub raw: Option<MojangFileDownloadInfo>,
    pub lzma: Option<MojangFileDownloadInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRuntimeFileEntry {
    #[serde(rename = "type")]
    pub file_type: String,
    pub executable: Option<bool>,
    pub downloads: Option<MojangFileDownloads>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MojangRuntimePackage {
    pub files: HashMap<String, MojangRuntimeFileEntry>,
}

pub fn get_java_component_for_version(version: &str) -> &'static str {
    if version == "1.21.1" {
        "java-runtime-delta" // Java 21
    } else {
        "java-runtime-gamma" // Java 17
    }
}

pub fn get_platform_key() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "windows-arm64"
        } else if cfg!(target_arch = "x86") {
            "windows-x86"
        } else {
            "windows-x64"
        }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "mac-os-arm64"
        } else {
            "mac-os"
        }
    } else {
        if cfg!(target_arch = "x86") {
            "linux-i386"
        } else {
            "linux"
        }
    }
}

pub fn get_java_binary_path(runtime_dir: &Path, component: &str, use_javaw: bool) -> PathBuf {
    let comp_dir = runtime_dir.join(component);
    if cfg!(target_os = "windows") {
        let bin_name = if use_javaw { "javaw.exe" } else { "java.exe" };
        comp_dir.join("bin").join(bin_name)
    } else {
        comp_dir.join("bin").join("java")
    }
}

pub fn runtime_complete_marker(runtime_dir: &Path, component: &str) -> PathBuf {
    runtime_dir.join(component).join(RUNTIME_COMPLETE_MARKER)
}

/// A runtime is usable only when its download finished (the marker) and the
/// launcher binary is there. `javaw.exe` + `release` alone can be a half download.
pub fn is_runtime_complete(runtime_dir: &Path, component: &str) -> bool {
    let has_binary = get_java_binary_path(runtime_dir, component, true).exists()
        || get_java_binary_path(runtime_dir, component, false).exists();
    has_binary && runtime_complete_marker(runtime_dir, component).is_file()
}

pub async fn ensure_mojang_java(
    paths: &WexPaths,
    mc_version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<PathBuf> {
    ensure_mojang_java_cancellable(paths, mc_version, client, events, concurrency, None).await
}

pub async fn ensure_mojang_java_cancellable(
    paths: &WexPaths,
    mc_version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
    cancel: Option<&CancellationToken>,
) -> Result<PathBuf> {
    ensure_mojang_java_from(
        paths,
        mc_version,
        client,
        events,
        concurrency,
        cancel,
        MOJANG_JAVA_ALL_JSON,
    )
    .await
}

async fn ensure_mojang_java_from(
    paths: &WexPaths,
    mc_version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
    cancel: Option<&CancellationToken>,
    all_json_url: &str,
) -> Result<PathBuf> {
    let component = get_java_component_for_version(mc_version);
    let javaw_exe = get_java_binary_path(&paths.runtime_dir, component, true);

    if is_runtime_complete(&paths.runtime_dir, component) {
        events.on_event(&Event::Log {
            level: LogLevel::Info,
            message: format!(
                "Using existing Mojang Java runtime for {} ({})",
                mc_version, component
            ),
        });
        return Ok(javaw_exe);
    }
    if cancel.is_some_and(|t| t.is_cancelled()) {
        return Err(WexError::Cancelled);
    }

    match download_mojang_java(
        paths,
        mc_version,
        component,
        client,
        events,
        concurrency,
        cancel,
        all_json_url,
    )
    .await
    {
        Ok(path) => Ok(path),
        Err(e)
            if e.is_network_unavailable() && is_legacy_runtime(&paths.runtime_dir, component) =>
        {
            // Runtime installed by an older Wex (no marker yet) and no network to
            // verify it: keep players able to play offline; the next online start
            // checks every file and writes the marker.
            events.on_event(&Event::Log {
                level: LogLevel::Warn,
                message: format!(
                    "Network unavailable ({}); using unverified Java runtime {} until the next online start",
                    e, component
                ),
            });
            Ok(javaw_exe)
        }
        Err(e) => Err(e),
    }
}

/// A runtime from before the completion marker existed.
fn is_legacy_runtime(runtime_dir: &Path, component: &str) -> bool {
    (get_java_binary_path(runtime_dir, component, true).exists()
        || get_java_binary_path(runtime_dir, component, false).exists())
        && runtime_dir.join(component).join("release").exists()
}

#[allow(clippy::too_many_arguments)]
async fn download_mojang_java(
    paths: &WexPaths,
    mc_version: &str,
    component: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
    cancel: Option<&CancellationToken>,
    all_json_url: &str,
) -> Result<PathBuf> {
    let comp_dir = paths.runtime_dir.join(component);
    let javaw_exe = get_java_binary_path(&paths.runtime_dir, component, true);
    let java_exe = get_java_binary_path(&paths.runtime_dir, component, false);
    let marker = runtime_complete_marker(&paths.runtime_dir, component);
    // Whatever happens below, the runtime is not complete until the pool succeeds.
    match fs::remove_file(&marker) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }

    events.on_event(&Event::StageStarted {
        stage: format!("Java runtime ({})", component),
        step: 1,
        total_steps: 1,
    });

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Downloading Mojang Java runtime {} for Minecraft {}...",
            component, mc_version
        ),
    });

    let platform_key = get_platform_key();
    let all_resp = client.get(all_json_url).send().await?.error_for_status()?;
    let all_data: serde_json::Value = all_resp.json().await?;

    let platform_obj = all_data.get(platform_key).ok_or_else(|| {
        WexError::JavaError(format!(
            "Platform '{}' not found in Mojang Java manifest",
            platform_key
        ))
    })?;

    let comp_entries = platform_obj.get(component).ok_or_else(|| {
        WexError::JavaError(format!(
            "Component '{}' not found for platform '{}'",
            component, platform_key
        ))
    })?;

    let manifest_ref: MojangRuntimeManifestRef = comp_entries
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("manifest"))
        .and_then(|m| serde_json::from_value(m.clone()).ok())
        .ok_or_else(|| {
            WexError::JavaError("Failed to parse Mojang runtime manifest URL".to_string())
        })?;

    let pkg_bytes = client
        .get(&manifest_ref.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let pkg_sha1 = {
        use sha1::Digest as _;
        hex::encode(sha1::Sha1::digest(&pkg_bytes))
    };
    if !pkg_sha1.eq_ignore_ascii_case(&manifest_ref.sha1) {
        return Err(WexError::HashMismatch {
            path: PathBuf::from(&manifest_ref.url),
            expected: manifest_ref.sha1.clone(),
            actual: pkg_sha1,
        });
    }
    let pkg: MojangRuntimePackage = serde_json::from_slice(&pkg_bytes)?;

    fs::create_dir_all(&comp_dir)?;

    let mut download_tasks = Vec::new();

    for (rel_path, file_entry) in &pkg.files {
        let target_path = crate::safe_path::safe_join(&comp_dir, rel_path)?;

        if file_entry.file_type == "directory" {
            let _ = fs::create_dir_all(&target_path);
            continue;
        }

        if let Some(downloads) = &file_entry.downloads {
            if let Some(raw) = &downloads.raw {
                download_tasks.push(DownloadTask {
                    url: raw.url.clone(),
                    fallback_url: None,
                    target_path,
                    size: Some(raw.size),
                    hash: Some((raw.sha1.clone(), HashType::Sha1)),
                });
            }
        }
    }

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Downloading {} files for Mojang Java runtime {}...",
            download_tasks.len(),
            component
        ),
    });

    download_pool_cancellable(
        client,
        download_tasks,
        concurrency,
        events,
        &format!("Java {}", component),
        cancel.cloned(),
    )
    .await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (rel_path, file_entry) in &pkg.files {
            let clean_path = rel_path.replace('\\', "/");
            let target_path = comp_dir.join(&clean_path);
            if (file_entry.executable == Some(true) || clean_path.contains("bin/"))
                && target_path.exists()
            {
                let _ = fs::set_permissions(&target_path, fs::Permissions::from_mode(0o755));
            }
        }
    }

    if !javaw_exe.exists() && !java_exe.exists() {
        return Err(WexError::JavaError(format!(
            "Java executable not found after download at {}",
            javaw_exe.display()
        )));
    }

    crate::fs_util::write_atomic(&marker, manifest_ref.sha1.as_bytes())?;

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!("Mojang Java runtime {} successfully installed", component),
    });

    Ok(javaw_exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_java_component_selection() {
        assert_eq!(
            get_java_component_for_version("1.20.1"),
            "java-runtime-gamma"
        );
        assert_eq!(
            get_java_component_for_version("1.21.1"),
            "java-runtime-delta"
        );
    }

    #[test]
    fn test_platform_key() {
        let key = get_platform_key();
        assert!(!key.is_empty());
    }

    use crate::event::NoopEventHandler;
    use sha1::Digest as _;
    use tempfile::tempdir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sha1_hex(data: &[u8]) -> String {
        hex::encode(sha1::Sha1::digest(data))
    }

    /// Mojang-style all.json + runtime manifest with two binaries and one extra file.
    /// `broken` makes the extra file 404.
    async fn mount_runtime(server: &MockServer, component: &str, broken: bool) {
        let file = |name: &str, body: &[u8]| {
            serde_json::json!({"type": "file", "downloads": {"raw": {
                "sha1": sha1_hex(body), "size": body.len(), "url": format!("{}/files/{}", server.uri(), name)
            }}})
        };
        let manifest = serde_json::json!({"files": {
            "bin": {"type": "directory"},
            "bin/javaw.exe": file("javaw.exe", b"javaw"),
            "bin/java.exe": file("java.exe", b"java"),
            "bin/java": file("java", b"java"),
            "release": file("release", b"JAVA_VERSION=21"),
            "lib/modules": file("modules", b"modules"),
        }});
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let all = serde_json::json!({ get_platform_key(): { component: [{
            "manifest": {"sha1": sha1_hex(&manifest_bytes), "size": manifest_bytes.len(),
                "url": format!("{}/manifest.json", server.uri())}
        }]}});
        Mock::given(method("GET"))
            .and(path("/all.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(all))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/manifest.json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(manifest_bytes))
            .mount(server)
            .await;
        for (name, body) in [
            ("javaw.exe", &b"javaw"[..]),
            ("java.exe", b"java"),
            ("java", b"java"),
            ("release", b"JAVA_VERSION=21"),
        ] {
            Mock::given(method("GET"))
                .and(path(format!("/files/{name}")))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
                .mount(server)
                .await;
        }
        let modules = if broken {
            ResponseTemplate::new(404)
        } else {
            ResponseTemplate::new(200).set_body_bytes(b"modules".to_vec())
        };
        Mock::given(method("GET"))
            .and(path("/files/modules"))
            .respond_with(modules)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn half_downloaded_runtime_is_never_treated_as_installed() {
        for version in ["1.20.1", "1.21.1"] {
            let component = get_java_component_for_version(version);
            let tmp = tempdir().unwrap();
            let paths = WexPaths::new(Some(tmp.path().join("Wex")));
            let client = reqwest::Client::new();
            let broken = MockServer::start().await;
            mount_runtime(&broken, component, true).await;
            let url = format!("{}/all.json", broken.uri());
            // Leftovers of an earlier download that died half-way.
            let comp_dir = paths.runtime_dir.join(component);
            fs::create_dir_all(comp_dir.join("bin")).unwrap();
            fs::write(
                get_java_binary_path(&paths.runtime_dir, component, true),
                b"javaw",
            )
            .unwrap();
            fs::write(
                get_java_binary_path(&paths.runtime_dir, component, false),
                b"java",
            )
            .unwrap();
            fs::write(comp_dir.join("release"), b"JAVA_VERSION=21").unwrap();

            // The pool fails on one file after the binaries and `release` landed.
            let first =
                ensure_mojang_java_from(&paths, version, &client, &NoopEventHandler, 1, None, &url)
                    .await;
            assert!(first.is_err(), "{version}: {first:?}");
            assert!(!runtime_complete_marker(&paths.runtime_dir, component).exists());
            assert!(!is_runtime_complete(&paths.runtime_dir, component));

            // Next start must retry the download instead of reusing the half runtime.
            let second =
                ensure_mojang_java_from(&paths, version, &client, &NoopEventHandler, 1, None, &url)
                    .await;
            assert!(second.is_err(), "{version}: half runtime was reused");

            let good = MockServer::start().await;
            mount_runtime(&good, component, false).await;
            let url = format!("{}/all.json", good.uri());
            let third =
                ensure_mojang_java_from(&paths, version, &client, &NoopEventHandler, 1, None, &url)
                    .await
                    .unwrap();
            assert_eq!(
                third,
                get_java_binary_path(&paths.runtime_dir, component, true)
            );
            assert!(is_runtime_complete(&paths.runtime_dir, component));
            assert!(paths
                .runtime_dir
                .join(component)
                .join("lib/modules")
                .exists());
        }
    }

    #[tokio::test]
    async fn legacy_runtime_without_marker_is_used_offline() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let component = get_java_component_for_version("1.21.1");
        let exe = get_java_binary_path(&paths.runtime_dir, component, true);
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, b"x").unwrap();
        fs::write(
            get_java_binary_path(&paths.runtime_dir, component, false),
            b"x",
        )
        .unwrap();
        fs::write(paths.runtime_dir.join(component).join("release"), b"x").unwrap();
        let client = reqwest::Client::new();
        let res = ensure_mojang_java_from(
            &paths,
            "1.21.1",
            &client,
            &NoopEventHandler,
            1,
            None,
            "http://127.0.0.1:1/all.json",
        )
        .await;
        assert_eq!(res.unwrap(), exe);
        assert!(!is_runtime_complete(&paths.runtime_dir, component));
    }

    #[tokio::test]
    async fn cancelled_java_install_makes_no_requests() {
        let server = MockServer::start().await;
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let token = CancellationToken::new();
        token.cancel();
        let res = ensure_mojang_java_from(
            &paths,
            "1.20.1",
            &reqwest::Client::new(),
            &NoopEventHandler,
            1,
            Some(&token),
            &format!("{}/all.json", server.uri()),
        )
        .await;
        assert!(matches!(res, Err(WexError::Cancelled)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
