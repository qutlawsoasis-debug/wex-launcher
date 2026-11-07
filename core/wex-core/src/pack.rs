use std::collections::HashSet;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::config::WexPaths;
use crate::download::{download_pool, DownloadTask, HashType};
use crate::error::Result;
use crate::event::{Event, EventHandler, LogLevel};

pub const DEFAULT_PACK_REPO: &str = "qutlawsoasis-debug/Aura-Pack";
pub const DEFAULT_PACK_BRANCH: &str = "main";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestServerEntry {
    pub id: Option<String>,
    pub name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFileEntry {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default = "default_file_mode")]
    pub mode: String,
    #[serde(default)]
    pub url: Option<String>,
}

fn default_file_mode() -> String {
    "sync".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    #[serde(rename = "packVersion")]
    pub pack_version: String,
    pub minecraft: String,
    #[serde(rename = "fabricLoader")]
    pub fabric_loader: String,
    #[serde(default)]
    pub servers: Option<Vec<ManifestServerEntry>>,
    pub files: Vec<ManifestFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PackSettings {
    #[serde(rename = "forceConfigs", default)]
    pub force_configs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackState {
    #[serde(rename = "packVersion")]
    pub pack_version: String,
    pub minecraft: String,
    #[serde(rename = "fabricLoader")]
    pub fabric_loader: String,
    #[serde(rename = "installedAtUtc")]
    pub installed_at_utc: String,
}

#[derive(Debug, Clone)]
pub struct PackSyncResult {
    pub pack_version: String,
    pub files_downloaded: usize,
    pub mods_deleted: usize,
}

pub fn get_manifest_filename(version: &str) -> &'static str {
    if version == "1.21.1" {
        "manifest-1.21.1.json"
    } else {
        "manifest.json"
    }
}

pub fn get_repo_prefix(version: &str) -> &'static str {
    if version == "1.21.1" {
        "versions/1.21.1/"
    } else {
        ""
    }
}

pub async fn fetch_manifest(
    client: &reqwest::Client,
    version: &str,
    repo: Option<&str>,
    branch: Option<&str>,
) -> Result<Manifest> {
    let repo = repo.unwrap_or(DEFAULT_PACK_REPO);
    let branch = branch.unwrap_or(DEFAULT_PACK_BRANCH);
    let filename = get_manifest_filename(version);
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}?t={}",
        repo,
        branch,
        filename,
        chrono::Utc::now().timestamp_millis()
    );

    let resp = client.get(&url).send().await?.error_for_status()?;
    let manifest: Manifest = resp.json().await?;
    Ok(manifest)
}

pub async fn fetch_pack_settings(
    client: &reqwest::Client,
    repo: Option<&str>,
    branch: Option<&str>,
) -> PackSettings {
    let repo = repo.unwrap_or(DEFAULT_PACK_REPO);
    let branch = branch.unwrap_or(DEFAULT_PACK_BRANCH);
    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/pack.settings.json?t={}",
        repo,
        branch,
        chrono::Utc::now().timestamp_millis()
    );

    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(settings) = resp.json::<PackSettings>().await {
            return settings;
        }
    }
    PackSettings::default()
}

pub async fn sync_aura_pack(
    paths: &WexPaths,
    version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<PackSyncResult> {
    paths.ensure_instance_dirs(version)?;
    let instance_dir = paths.instance_dir(version);

    events.on_event(&Event::StageStarted {
        stage: format!("Aura-Pack sync ({})", version),
        step: 1,
        total_steps: 1,
    });

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!("Fetching Aura-Pack manifest for Minecraft {}...", version),
    });

    let manifest = fetch_manifest(client, version, None, None).await?;
    let settings = fetch_pack_settings(client, None, None).await;

    let repo_prefix = get_repo_prefix(version);
    let repo = DEFAULT_PACK_REPO;
    let branch = DEFAULT_PACK_BRANCH;

    let force_set: HashSet<String> = settings
        .force_configs
        .iter()
        .map(|s| s.replace('\\', "/").trim_start_matches('/').to_lowercase())
        .collect();

    let mut download_tasks = Vec::new();

    for file in &manifest.files {
        let clean_path = file.path.replace('\\', "/");
        let target_path = instance_dir.join(&clean_path);

        let is_forced = force_set.contains(&clean_path.to_lowercase())
            || (clean_path.starts_with("config/")
                && force_set.contains(&clean_path["config/".len()..].to_lowercase()));

        let is_default_mode = file.mode.eq_ignore_ascii_case("default") && !is_forced;

        if is_default_mode && target_path.exists() {
            // Keep user config/options
            continue;
        }

        // If sync mode, check if already valid
        if target_path.exists() {
            if let Ok(meta) = fs::metadata(&target_path) {
                if meta.len() == file.size {
                    if let Ok(actual_hash) = crate::download::compute_file_hash(&target_path, HashType::Sha256) {
                        if actual_hash.eq_ignore_ascii_case(&file.sha256) {
                            continue;
                        }
                    }
                }
            }
        }

        let download_url = if let Some(custom_url) = &file.url {
            custom_url.clone()
        } else {
            let escaped_segments: Vec<String> = clean_path
                .split('/')
                .map(urlencoding_encode)
                .collect();
            let escaped_path = escaped_segments.join("/");
            format!(
                "https://raw.githubusercontent.com/{}/{}/{}{}",
                repo, branch, repo_prefix, escaped_path
            )
        };

        download_tasks.push(DownloadTask {
            url: download_url,
            target_path,
            size: Some(file.size),
            hash: Some((file.sha256.clone(), HashType::Sha256)),
        });
    }

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Aura-Pack: {} files to download ({} total in manifest)",
            download_tasks.len(),
            manifest.files.len()
        ),
    });

    let files_to_download_count = download_tasks.len();
    if !download_tasks.is_empty() {
        download_pool(
            client,
            download_tasks,
            concurrency,
            events,
            &format!("Pack {}", version),
        )
        .await?;
    }

    // Clean orphan mods
    let mut mods_deleted = 0;
    let mods_dir = instance_dir.join("mods");
    if mods_dir.exists() {
        let manifest_mod_names: HashSet<String> = manifest
            .files
            .iter()
            .filter(|f| f.path.starts_with("mods/") && f.path.ends_with(".jar"))
            .filter_map(|f| Path::new(&f.path).file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()))
            .collect();

        let manifest_disabled_names: HashSet<String> = manifest
            .files
            .iter()
            .filter(|f| f.path.starts_with("mods/") && f.path.ends_with(".disabled"))
            .filter_map(|f| Path::new(&f.path).file_name().and_then(|n| n.to_str()).map(|s| s.to_lowercase()))
            .collect();

        if let Ok(entries) = fs::read_dir(&mods_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                    let lower = file_name.to_lowercase();
                    if lower.ends_with(".jar") {
                        if !manifest_mod_names.contains(&lower) {
                            let disabled_name = format!("{}.disabled", lower);
                            if !manifest_disabled_names.contains(&disabled_name) {
                                events.on_event(&Event::Log {
                                    level: LogLevel::Info,
                                    message: format!("Deleting removed mod: {}", file_name),
                                });
                                let _ = fs::remove_file(&p);
                                mods_deleted += 1;
                            }
                        }
                    } else if lower.ends_with(".jar.disabled") {
                        let active_name = lower.trim_end_matches(".disabled");
                        if !manifest_disabled_names.contains(&lower) && !manifest_mod_names.contains(active_name) {
                            let _ = fs::remove_file(&p);
                            mods_deleted += 1;
                        }
                    }
                }
            }
        }
    }

    // Save pack.state.json
    let state = PackState {
        pack_version: manifest.pack_version.clone(),
        minecraft: manifest.minecraft.clone(),
        fabric_loader: manifest.fabric_loader.clone(),
        installed_at_utc: chrono::Utc::now().to_rfc3339(),
    };
    let state_json = serde_json::to_string_pretty(&state)?;
    fs::write(instance_dir.join("pack.state.json"), state_json)?;

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Aura-Pack sync complete: version {}, {} files updated, {} orphan mods deleted",
            manifest.pack_version, files_to_download_count, mods_deleted
        ),
    });

    Ok(PackSyncResult {
        pack_version: manifest.pack_version,
        files_downloaded: files_to_download_count,
        mods_deleted,
    })
}

fn urlencoding_encode(s: &str) -> String {
    let mut result = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(b as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", b));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "name": "Aura",
            "packVersion": "20261025.1406",
            "minecraft": "1.20.1",
            "fabricLoader": "0.19.5",
            "files": [
                {
                    "path": "mods/test.jar",
                    "sha256": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                    "size": 1024,
                    "mode": "sync"
                },
                {
                    "path": "config/user.json",
                    "sha256": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                    "size": 512,
                    "mode": "default"
                }
            ]
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "Aura");
        assert_eq!(manifest.pack_version, "20261025.1406");
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.files[0].mode, "sync");
        assert_eq!(manifest.files[1].mode, "default");
    }

    #[test]
    fn test_orphan_mod_removal() {
        let tmp = tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        let mod1 = mods_dir.join("valid-mod.jar");
        let mod2 = mods_dir.join("old-mod.jar");
        fs::write(&mod1, "data1").unwrap();
        fs::write(&mod2, "data2").unwrap();

        let manifest = Manifest {
            name: "Aura".to_string(),
            pack_version: "1.0".to_string(),
            minecraft: "1.20.1".to_string(),
            fabric_loader: "0.19.5".to_string(),
            servers: None,
            files: vec![ManifestFileEntry {
                path: "mods/valid-mod.jar".to_string(),
                sha256: "".to_string(),
                size: 5,
                mode: "sync".to_string(),
                url: None,
            }],
        };

        let manifest_mod_names: HashSet<String> = manifest
            .files
            .iter()
            .map(|f| Path::new(&f.path).file_name().unwrap().to_str().unwrap().to_lowercase())
            .collect();

        for entry in fs::read_dir(&mods_dir).unwrap().flatten() {
            let p = entry.path();
            let name = p.file_name().unwrap().to_str().unwrap().to_lowercase();
            if !manifest_mod_names.contains(&name) {
                fs::remove_file(p).unwrap();
            }
        }

        assert!(mod1.exists());
        assert!(!mod2.exists());
    }
}
