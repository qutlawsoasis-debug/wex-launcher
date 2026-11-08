use crate::config::WexPaths;
use crate::download::{download_pool, DownloadTask, HashType};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

pub async fn ensure_mojang_java(
    paths: &WexPaths,
    mc_version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<PathBuf> {
    let component = get_java_component_for_version(mc_version);
    let comp_dir = paths.runtime_dir.join(component);
    let javaw_exe = get_java_binary_path(&paths.runtime_dir, component, true);
    let java_exe = get_java_binary_path(&paths.runtime_dir, component, false);

    // If javaw.exe exists and release file exists, we consider it installed
    let release_file = comp_dir.join("release");
    if (javaw_exe.exists() || java_exe.exists()) && release_file.exists() {
        events.on_event(&Event::Log {
            level: LogLevel::Info,
            message: format!(
                "Using existing Mojang Java runtime for {} ({})",
                mc_version, component
            ),
        });
        return Ok(javaw_exe);
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
    let all_resp = client
        .get(MOJANG_JAVA_ALL_JSON)
        .send()
        .await?
        .error_for_status()?;
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

    let manifest_url = comp_entries
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("manifest"))
        .and_then(|m| m.get("url"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| {
            WexError::JavaError("Failed to parse Mojang runtime manifest URL".to_string())
        })?;

    let pkg_resp = client.get(manifest_url).send().await?.error_for_status()?;
    let pkg: MojangRuntimePackage = pkg_resp.json().await?;

    fs::create_dir_all(&comp_dir)?;

    let mut download_tasks = Vec::new();

    for (rel_path, file_entry) in &pkg.files {
        let clean_path = rel_path.replace('\\', "/");
        let target_path = comp_dir.join(&clean_path);

        if file_entry.file_type == "directory" {
            let _ = fs::create_dir_all(&target_path);
            continue;
        }

        if let Some(downloads) = &file_entry.downloads {
            if let Some(raw) = &downloads.raw {
                download_tasks.push(DownloadTask {
                    url: raw.url.clone(),
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

    download_pool(
        client,
        download_tasks,
        concurrency,
        events,
        &format!("Java {}", component),
    )
    .await?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (rel_path, file_entry) in &pkg.files {
            let clean_path = rel_path.replace('\\', "/");
            let target_path = comp_dir.join(&clean_path);
            if file_entry.executable == Some(true) || clean_path.contains("bin/") {
                if target_path.exists() {
                    let _ = fs::set_permissions(&target_path, fs::Permissions::from_mode(0o755));
                }
            }
        }
    }

    if !javaw_exe.exists() && !java_exe.exists() {
        return Err(WexError::JavaError(format!(
            "Java executable not found after download at {}",
            javaw_exe.display()
        )));
    }

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
}
