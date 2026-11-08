use crate::config::WexPaths;
use crate::download::{download_pool, DownloadTask, HashType};
use crate::error::Result;
use crate::event::{Event, EventHandler, LogLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

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
    #[serde(rename = "installedMods", default)]
    pub installed_mods: Vec<String>,
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

        // Check if mod was disabled by user (.disabled)
        if clean_path.starts_with("mods/") && clean_path.ends_with(".jar") {
            let disabled_target_path = instance_dir.join(format!("{}.disabled", clean_path));
            if disabled_target_path.exists() {
                if let Ok(meta) = fs::metadata(&disabled_target_path) {
                    if meta.len() == file.size {
                        // User disabled this mod and file size matches manifest. Keep it disabled!
                        continue;
                    }
                }
            }
        }

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
                    if let Ok(actual_hash) =
                        crate::download::compute_file_hash(&target_path, HashType::Sha256)
                    {
                        if actual_hash.eq_ignore_ascii_case(&file.sha256) {
                            continue;
                        }
                    }
                }
            }
        }

        let escaped_segments: Vec<String> = clean_path.split('/').map(urlencoding_encode).collect();
        let escaped_path = escaped_segments.join("/");
        let raw_pack_url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}{}",
            repo, branch, repo_prefix, escaped_path
        );

        let (download_url, fallback_url) = if let Some(custom_url) = &file.url {
            (custom_url.clone(), Some(raw_pack_url))
        } else {
            (raw_pack_url, None)
        };

        download_tasks.push(DownloadTask {
            url: download_url,
            fallback_url,
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

    // Clean orphan mods (ONLY mods previously installed by Wex, never delete user mods!)
    let mut mods_deleted = 0;
    let mods_dir = instance_dir.join("mods");
    let state_path = instance_dir.join("pack.state.json");
    let previous_installed_mods: HashSet<String> = if state_path.exists() {
        fs::read_to_string(&state_path)
            .ok()
            .and_then(|content| serde_json::from_str::<PackState>(&content).ok())
            .map(|s| {
                s.installed_mods
                    .into_iter()
                    .map(|m| m.to_lowercase())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        HashSet::new()
    };

    if mods_dir.exists() {
        let manifest_mod_names: HashSet<String> = manifest
            .files
            .iter()
            .filter(|f| f.path.starts_with("mods/") && f.path.ends_with(".jar"))
            .filter_map(|f| {
                Path::new(&f.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_lowercase())
            })
            .collect();

        if let Ok(entries) = fs::read_dir(&mods_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                    let lower = file_name.to_lowercase();
                    if lower.ends_with(".jar") {
                        // Only delete if Wex installed it previously AND it's removed from manifest!
                        if previous_installed_mods.contains(&lower)
                            && !manifest_mod_names.contains(&lower)
                        {
                            events.on_event(&Event::Log {
                                level: LogLevel::Info,
                                message: format!("Deleting removed pack mod: {}", file_name),
                            });
                            let _ = fs::remove_file(&p);
                            mods_deleted += 1;
                        }
                    } else if lower.ends_with(".jar.disabled") {
                        let active_name = lower.trim_end_matches(".disabled");
                        if previous_installed_mods.contains(active_name)
                            && !manifest_mod_names.contains(active_name)
                        {
                            events.on_event(&Event::Log {
                                level: LogLevel::Info,
                                message: format!(
                                    "Deleting removed disabled pack mod: {}",
                                    file_name
                                ),
                            });
                            let _ = fs::remove_file(&p);
                            mods_deleted += 1;
                        }
                    }
                }
            }
        }
    }

    // Save pack.state.json with new list of installed pack mods
    let current_installed_mods: Vec<String> = manifest
        .files
        .iter()
        .filter(|f| f.path.starts_with("mods/") && f.path.ends_with(".jar"))
        .filter_map(|f| {
            Path::new(&f.path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        })
        .collect();

    let state = PackState {
        pack_version: manifest.pack_version.clone(),
        minecraft: manifest.minecraft.clone(),
        fabric_loader: manifest.fabric_loader.clone(),
        installed_at_utc: chrono::Utc::now().to_rfc3339(),
        installed_mods: current_installed_mods,
    };
    let state_json = serde_json::to_string_pretty(&state)?;
    fs::write(&state_path, state_json)?;

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

        let pack_mod = mods_dir.join("valid-pack-mod.jar");
        let old_pack_mod = mods_dir.join("old-pack-mod.jar");
        let user_mod = mods_dir.join("my-custom-minimap.jar");

        fs::write(&pack_mod, "data1").unwrap();
        fs::write(&old_pack_mod, "data2").unwrap();
        fs::write(&user_mod, "custom user mod").unwrap();

        // Previous state recorded only pack mods
        let previous_installed_mods: HashSet<String> = [
            "valid-pack-mod.jar".to_string(),
            "old-pack-mod.jar".to_string(),
        ]
        .into_iter()
        .collect();

        // New manifest only has valid-pack-mod.jar
        let manifest_mod_names: HashSet<String> =
            ["valid-pack-mod.jar".to_string()].into_iter().collect();

        for entry in fs::read_dir(&mods_dir).unwrap().flatten() {
            let p = entry.path();
            let name = p.file_name().unwrap().to_str().unwrap().to_lowercase();
            // ONLY remove if it was installed by Wex AND is no longer in manifest!
            if previous_installed_mods.contains(&name) && !manifest_mod_names.contains(&name) {
                fs::remove_file(p).unwrap();
            }
        }

        // Pack mod in manifest is kept
        assert!(pack_mod.exists());
        // Old pack mod no longer in manifest is removed
        assert!(!old_pack_mod.exists());
        // User's custom mod was NEVER in previous_installed_mods, so it MUST BE PRESERVED!
        assert!(
            user_mod.exists(),
            "User custom mod must NEVER be deleted by pack sync!"
        );
    }

    #[test]
    fn test_disabled_mod_not_redownloaded() {
        let tmp = tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        let disabled_mod = mods_dir.join("iris.jar.disabled");
        let test_data = b"some iris shader data";
        fs::write(&disabled_mod, test_data).unwrap();

        let entry = ManifestFileEntry {
            path: "mods/iris.jar".to_string(),
            sha256: "dummy".to_string(),
            size: test_data.len() as u64,
            mode: "sync".to_string(),
            url: None,
        };

        let clean_path = entry.path.replace('\\', "/");
        let disabled_target = tmp.path().join(format!("{}.disabled", clean_path));
        assert!(disabled_target.exists());

        let mut should_download = true;
        if let Ok(meta) = fs::metadata(&disabled_target) {
            if meta.len() == entry.size {
                should_download = false;
            }
        }

        assert!(
            !should_download,
            "Disabled mod with matching size must NOT be queued for re-download!"
        );
    }

    #[test]
    fn test_sync_modes_default_preserve_and_force_override() {
        let tmp = tempdir().unwrap();
        let instance_dir = tmp.path();

        let user_config = instance_dir.join("config").join("user.json");
        fs::create_dir_all(user_config.parent().unwrap()).unwrap();
        fs::write(&user_config, "user custom preferences").unwrap();

        let forced_config = instance_dir.join("config").join("forced.json");
        fs::write(&forced_config, "old forced content").unwrap();

        let force_set: HashSet<String> = ["config/forced.json".to_string()]
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        // Entry 1: mode default, not forced -> should keep existing file
        let entry_default = ManifestFileEntry {
            path: "config/user.json".to_string(),
            sha256: "dummy_sha".to_string(),
            size: 50,
            mode: "default".to_string(),
            url: None,
        };
        let clean_path1 = entry_default.path.replace('\\', "/");
        let is_forced1 = force_set.contains(&clean_path1.to_lowercase());
        let is_default1 = entry_default.mode.eq_ignore_ascii_case("default") && !is_forced1;
        let should_skip1 = is_default1 && user_config.exists();
        assert!(
            should_skip1,
            "User config in default mode must not be overwritten!"
        );

        // Entry 2: mode default, but present in force_configs -> should NOT skip
        let entry_forced = ManifestFileEntry {
            path: "config/forced.json".to_string(),
            sha256: "new_forced_sha".to_string(),
            size: 100,
            mode: "default".to_string(),
            url: None,
        };
        let clean_path2 = entry_forced.path.replace('\\', "/");
        let is_forced2 = force_set.contains(&clean_path2.to_lowercase());
        let is_default2 = entry_forced.mode.eq_ignore_ascii_case("default") && !is_forced2;
        let should_skip2 = is_default2 && forced_config.exists();
        assert!(
            !should_skip2,
            "Forced config must not be skipped even in default mode!"
        );

        // Entry 3: mode sync, identical hash and size -> should skip
        let synced_mod = instance_dir.join("mods").join("synced.jar");
        fs::create_dir_all(synced_mod.parent().unwrap()).unwrap();
        let content = b"test jar data";
        fs::write(&synced_mod, content).unwrap();
        let expected_hash =
            crate::download::compute_file_hash(&synced_mod, HashType::Sha256).unwrap();

        let entry_synced = ManifestFileEntry {
            path: "mods/synced.jar".to_string(),
            sha256: expected_hash,
            size: content.len() as u64,
            mode: "sync".to_string(),
            url: None,
        };
        let mut download_needed = true;
        if synced_mod.exists() {
            if let Ok(meta) = fs::metadata(&synced_mod) {
                if meta.len() == entry_synced.size {
                    if let Ok(actual_hash) =
                        crate::download::compute_file_hash(&synced_mod, HashType::Sha256)
                    {
                        if actual_hash.eq_ignore_ascii_case(&entry_synced.sha256) {
                            download_needed = false;
                        }
                    }
                }
            }
        }
        assert!(
            !download_needed,
            "Synced file matching size and hash must not re-download (0 files to download)"
        );
    }

    #[tokio::test]
    async fn test_pack_download_fallback_url_404_raw_ok() {
        use sha2::Digest;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let payload = b"simulated mod jar binary content";
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let expected_hash = hex::encode(hasher.finalize());

        // Primary URL returns 404
        Mock::given(method("GET"))
            .and(path("/release/mod.jar"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        // Fallback raw URL returns 200 with payload
        Mock::given(method("GET"))
            .and(path("/raw/mods/mod.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.to_vec()))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let target = tmp.path().join("mods").join("mod.jar");
        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;

        let primary_url = format!("{}/release/mod.jar", mock_server.uri());
        let fallback_url = format!("{}/raw/mods/mod.jar", mock_server.uri());

        let task = DownloadTask {
            url: primary_url,
            fallback_url: Some(fallback_url),
            target_path: target.clone(),
            size: Some(payload.len() as u64),
            hash: Some((expected_hash.clone(), HashType::Sha256)),
        };

        let pool_res = download_pool(&client, vec![task], 2, &events, "Test Fallback").await;
        assert!(pool_res.is_ok(), "Download pool must succeed via fallback");
        assert!(target.exists(), "Target file must be written to disk");

        let disk_hash = crate::download::compute_file_hash(&target, HashType::Sha256).unwrap();
        assert_eq!(
            disk_hash, expected_hash,
            "Hash on disk must match expected sha256"
        );
    }

    #[tokio::test]
    async fn test_pack_download_fallback_both_404_returns_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary URL returns 404
        Mock::given(method("GET"))
            .and(path("/release/mod.jar"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        // Fallback raw URL also returns 404
        Mock::given(method("GET"))
            .and(path("/raw/mods/mod.jar"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let target = tmp.path().join("mods").join("mod.jar");
        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;

        let primary_url = format!("{}/release/mod.jar", mock_server.uri());
        let fallback_url = format!("{}/raw/mods/mod.jar", mock_server.uri());

        let task = DownloadTask {
            url: primary_url,
            fallback_url: Some(fallback_url),
            target_path: target,
            size: Some(1024),
            hash: Some(("abcdef".to_string(), HashType::Sha256)),
        };

        let pool_res = download_pool(&client, vec![task], 2, &events, "Test Fallback").await;
        assert!(
            pool_res.is_err(),
            "Download pool must fail when both primary and fallback return 404"
        );
    }
}
