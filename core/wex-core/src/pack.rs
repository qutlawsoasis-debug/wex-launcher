use crate::config::WexPaths;
use crate::download::{download_pool_cancellable, DownloadTask, HashType};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

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
    /// Where the file is needed: `client`, `server` or `both`. Optional; the
    /// current Aura-Pack manifests do not set it yet (see `server::install`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}

/// Primary and fallback download URLs of a pack file.
pub fn pack_file_urls(version: &str, file: &ManifestFileEntry) -> (String, Option<String>) {
    let clean_path = file.path.replace('\\', "/");
    let escaped: Vec<String> = clean_path.split('/').map(urlencoding_encode).collect();
    let raw_pack_url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}{}",
        DEFAULT_PACK_REPO,
        DEFAULT_PACK_BRANCH,
        get_repo_prefix(version),
        escaped.join("/")
    );
    match &file.url {
        Some(custom) => (custom.clone(), Some(raw_pack_url)),
        None => (raw_pack_url, None),
    }
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
    /// [`manifest_digest`] of the manifest this state was synced from. Empty in
    /// states written by older Wex versions, which forces one re-sync.
    #[serde(rename = "manifestDigest", default)]
    pub manifest_digest: String,
}

/// SHA-256 over the manifest's file list (path, hash, size, mode), independent of
/// file order. Two manifests with the same digest install the same files.
pub fn manifest_digest(manifest: &Manifest) -> String {
    use sha2::Digest as _;
    let mut lines: Vec<String> = manifest
        .files
        .iter()
        .map(|f| {
            format!(
                "{}\t{}\t{}\t{}\n",
                f.path.replace('\\', "/"),
                f.sha256.to_ascii_lowercase(),
                f.size,
                f.mode.to_ascii_lowercase()
            )
        })
        .collect();
    lines.sort();
    let mut hasher = sha2::Sha256::new();
    for line in &lines {
        hasher.update(line.as_bytes());
    }
    hex::encode(hasher.finalize())
}

/// Whether the installed pack matches the published manifest.
#[derive(Debug)]
pub enum PackFreshness {
    /// pack.state.json records exactly this manifest.
    Current,
    /// The published manifest differs from what was installed (or nothing was).
    Outdated(Box<Manifest>),
    /// The manifest could not be fetched (offline, GitHub down).
    Unknown(WexError),
}

pub fn read_pack_state(paths: &WexPaths, version: &str) -> Option<PackState> {
    let content = fs::read_to_string(paths.instance_dir(version).join("pack.state.json")).ok()?;
    serde_json::from_str(&content).ok()
}

/// Overall deadline of the update check before a fast launch. Throttled
/// raw.githubusercontent.com must not hold Play for the full connect + read timeouts.
pub const PACK_CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(4);

/// Orders pack versions like `20261025.1406`: dot-separated numbers compared
/// numerically, anything non-numeric compared as text.
pub fn compare_pack_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let mut left = a.split('.');
    let mut right = b.split('.');
    loop {
        match (left.next(), right.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(x), Some(y)) => {
                let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                    (Ok(x), Ok(y)) => x.cmp(&y),
                    _ => x.cmp(y),
                };
                if order != std::cmp::Ordering::Equal {
                    return order;
                }
            }
        }
    }
}

/// Compares the published manifest at `url` with the installed pack.state.json.
/// Only a newer pack version, or the same version with a different file list,
/// counts as an update: a stale CDN edge serving an older manifest must not make
/// the launcher flip between two packs.
pub async fn check_pack_freshness_at(
    paths: &WexPaths,
    version: &str,
    client: &reqwest::Client,
    url: &str,
) -> PackFreshness {
    let manifest = match fetch_manifest_within(client, url, PACK_CHECK_TIMEOUT).await {
        Ok(m) => m,
        Err(e) => return PackFreshness::Unknown(e),
    };
    let Some(state) = read_pack_state(paths, version) else {
        return PackFreshness::Outdated(Box::new(manifest));
    };
    match compare_pack_versions(&manifest.pack_version, &state.pack_version) {
        std::cmp::Ordering::Greater => PackFreshness::Outdated(Box::new(manifest)),
        std::cmp::Ordering::Equal if state.manifest_digest != manifest_digest(&manifest) => {
            PackFreshness::Outdated(Box::new(manifest))
        }
        _ => PackFreshness::Current,
    }
}

/// [`check_pack_freshness_at`] against the default Aura-Pack repository.
pub async fn check_pack_freshness(
    paths: &WexPaths,
    version: &str,
    client: &reqwest::Client,
) -> PackFreshness {
    let url = manifest_url(version, None, None);
    check_pack_freshness_at(paths, version, client, &url).await
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

/// Validates a manifest-supplied relative path and resolves it inside `instance_dir`.
/// Returns the slash-normalized relative path and the absolute target.
pub fn resolve_pack_file(instance_dir: &Path, rel: &str) -> Result<(String, PathBuf)> {
    let target = crate::safe_path::safe_join(instance_dir, rel)?;
    Ok((rel.replace('\\', "/"), target))
}

pub fn get_repo_prefix(version: &str) -> &'static str {
    if version == "1.21.1" {
        "versions/1.21.1/"
    } else {
        ""
    }
}

/// Cache-busting URL of the published manifest for `version`.
pub fn manifest_url(version: &str, repo: Option<&str>, branch: Option<&str>) -> String {
    let repo = repo.unwrap_or(DEFAULT_PACK_REPO);
    let branch = branch.unwrap_or(DEFAULT_PACK_BRANCH);
    let filename = get_manifest_filename(version);
    format!(
        "https://raw.githubusercontent.com/{}/{}/{}?t={}",
        repo,
        branch,
        filename,
        chrono::Utc::now().timestamp_millis()
    )
}

pub async fn fetch_manifest_from(client: &reqwest::Client, url: &str) -> Result<Manifest> {
    let resp = client.get(url).send().await?.error_for_status()?;
    let manifest: Manifest = resp.json().await?;
    Ok(manifest)
}

/// [`fetch_manifest_from`] with an overall deadline for the whole request; running
/// out of time is a network timeout (`is_network_unavailable`).
pub async fn fetch_manifest_within(
    client: &reqwest::Client,
    url: &str,
    deadline: std::time::Duration,
) -> Result<Manifest> {
    let resp = client
        .get(url)
        .timeout(deadline)
        .send()
        .await?
        .error_for_status()?;
    let manifest: Manifest = resp.json().await?;
    Ok(manifest)
}

/// A failed pack sync that left the installed pack untouched and that a later
/// online start can repair: transport errors and bad downloads. Cancel, disk and
/// manifest errors are not in this group.
pub fn is_transient_sync_failure(err: &WexError) -> bool {
    matches!(
        err,
        WexError::Network(_)
            | WexError::DownloadError { .. }
            | WexError::HashMismatch { .. }
            | WexError::SizeMismatch { .. }
    )
}

pub async fn fetch_manifest(
    client: &reqwest::Client,
    version: &str,
    repo: Option<&str>,
    branch: Option<&str>,
) -> Result<Manifest> {
    fetch_manifest_from(client, &manifest_url(version, repo, branch)).await
}

/// Stores the manifest a sync is about to use, for offline starts.
pub fn cache_manifest(paths: &WexPaths, version: &str, manifest: &Manifest) {
    if let Ok(json) = serde_json::to_vec_pretty(manifest) {
        let _ = crate::fs_util::write_atomic(
            &paths.instance_dir(version).join("manifest.cache.json"),
            &json,
        );
    }
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
    sync_aura_pack_cancellable(paths, version, client, events, concurrency, None).await
}

pub async fn sync_aura_pack_cancellable(
    paths: &WexPaths,
    version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
    cancel_token: Option<&CancellationToken>,
) -> Result<PackSyncResult> {
    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            return Err(WexError::Cancelled);
        }
    }

    paths.ensure_instance_dirs(version)?;
    let instance_dir = paths.instance_dir(version);

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!("Fetching Aura-Pack manifest for Minecraft {}...", version),
    });

    let manifest_cache_file = instance_dir.join("manifest.cache.json");
    let manifest = match fetch_manifest(client, version, None, None).await {
        Ok(m) => {
            cache_manifest(paths, version, &m);
            m
        }
        Err(e) => {
            if manifest_cache_file.exists() {
                events.on_event(&Event::Log {
                    level: LogLevel::Warn,
                    message: format!(
                        "Network unavailable ({}); using cached Aura-Pack manifest for Minecraft {}",
                        e, version
                    ),
                });
                let content = fs::read_to_string(&manifest_cache_file)?;
                serde_json::from_str(&content)?
            } else {
                return Err(e);
            }
        }
    };
    sync_aura_pack_with_manifest(
        paths,
        version,
        client,
        events,
        concurrency,
        cancel_token,
        manifest,
    )
    .await
}

/// Folder inside the instance where a sync downloads new pack files. Files move
/// into place only after every download succeeded, so a dropped connection or
/// Cancel never leaves a mix of old and new mods.
pub const PACK_STAGING_DIR: &str = ".pack-staging";
/// Written when all staged files are verified, removed after the commit finished.
/// While it exists the instance may be half-committed and must not be launched.
pub const PACK_COMMIT_PLAN: &str = "pack.commit.json";

pub fn pack_staging_dir(paths: &WexPaths, version: &str) -> PathBuf {
    paths.instance_dir(version).join(PACK_STAGING_DIR)
}

fn pack_commit_plan_path(paths: &WexPaths, version: &str) -> PathBuf {
    paths.instance_dir(version).join(PACK_COMMIT_PLAN)
}

/// True when a commit was interrupted (crash, power loss) and still has to finish.
pub fn has_pending_pack_commit(paths: &WexPaths, version: &str) -> bool {
    pack_commit_plan_path(paths, version).exists()
}

#[derive(Debug, Serialize, Deserialize)]
struct PackCommitPlan {
    manifest: Manifest,
    /// Slash-separated paths, relative to the instance, waiting in the staging dir.
    staged: Vec<String>,
}

/// Finishes a commit that was interrupted after all files were staged. Needs no
/// network. Returns whether there was one.
pub fn finish_pending_pack_commit(
    paths: &WexPaths,
    version: &str,
    events: &dyn EventHandler,
) -> Result<bool> {
    let plan_path = pack_commit_plan_path(paths, version);
    let content = match fs::read(&plan_path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let plan: PackCommitPlan = serde_json::from_slice(&content)?;
    events.on_event(&Event::Log {
        level: LogLevel::Warn,
        message: format!(
            "Aura-Pack {}: finishing an interrupted update for Minecraft {}",
            plan.manifest.pack_version, version
        ),
    });
    commit_pack(paths, version, &plan, events)?;
    Ok(true)
}

/// Moves staged files into place, removes orphaned pack mods, writes the state
/// and drops the plan. Every step can be repeated after a crash.
fn commit_pack(
    paths: &WexPaths,
    version: &str,
    plan: &PackCommitPlan,
    events: &dyn EventHandler,
) -> Result<usize> {
    let instance_dir = paths.instance_dir(version);
    let staging_dir = pack_staging_dir(paths, version);
    for rel in &plan.staged {
        let src = crate::safe_path::safe_join(&staging_dir, rel)?;
        let (_, dst) = resolve_pack_file(&instance_dir, rel)?;
        if !src.is_file() {
            continue; // moved before the interruption
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| crate::download::map_io_err(e, parent))?;
        }
        crate::download::rename_with_retry_blocking(&src, &dst)
            .map_err(|e| crate::download::map_io_err(e, &dst))?;
    }

    let manifest = &plan.manifest;
    // Clean orphan mods (ONLY mods previously installed by Wex, never delete user mods!)
    let mut mods_deleted = 0;
    let mods_dir = instance_dir.join("mods");
    let state_path = instance_dir.join("pack.state.json");
    let previous_installed_mods: HashSet<String> = read_pack_state(paths, version)
        .map(|s| {
            s.installed_mods
                .into_iter()
                .map(|m| m.to_lowercase())
                .collect()
        })
        .unwrap_or_default();

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
        manifest_digest: manifest_digest(manifest),
    };
    let state_json = serde_json::to_string_pretty(&state)?;
    crate::fs_util::write_atomic(&state_path, state_json.as_bytes())?;

    match fs::remove_file(pack_commit_plan_path(paths, version)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    // Leftovers (stale .part files of other manifests) go with the folder.
    let _ = fs::remove_dir_all(&staging_dir);
    Ok(mods_deleted)
}

/// Syncs the instance to an already fetched manifest (no second manifest request).
///
/// Two phases: changed files are downloaded and verified in [`PACK_STAGING_DIR`]
/// (an interrupted run resumes from there), then committed into the instance in
/// one quick local step. A failed or cancelled download leaves the installed pack
/// exactly as it was.
pub async fn sync_aura_pack_with_manifest(
    paths: &WexPaths,
    version: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
    cancel_token: Option<&CancellationToken>,
    manifest: Manifest,
) -> Result<PackSyncResult> {
    if cancel_token.is_some_and(|t| t.is_cancelled()) {
        return Err(WexError::Cancelled);
    }
    paths.ensure_instance_dirs(version)?;
    let instance_dir = paths.instance_dir(version);
    let staging_dir = pack_staging_dir(paths, version);
    // A commit cut short by a crash is finished first, so planning sees a whole pack.
    finish_pending_pack_commit(paths, version, events)?;

    events.on_event(&Event::StageStarted {
        stage: format!("Aura-Pack sync ({})", version),
        step: 1,
        total_steps: 1,
    });

    let settings = fetch_pack_settings(client, None, None).await;

    let force_set: HashSet<String> = settings
        .force_configs
        .iter()
        .map(|s| s.replace('\\', "/").trim_start_matches('/').to_lowercase())
        .collect();

    let mut download_tasks = Vec::new();
    let mut staged = Vec::new();

    for file in &manifest.files {
        let (clean_path, target_path) = resolve_pack_file(&instance_dir, &file.path)?;

        // Check if mod was disabled by user (.disabled)
        if clean_path.starts_with("mods/") && clean_path.ends_with(".jar") {
            let disabled_target_path =
                crate::safe_path::safe_join(&instance_dir, &format!("{}.disabled", clean_path))?;
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
        let reason = match fs::metadata(&target_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => "missing".to_string(),
            Err(e) => format!("cannot read metadata: {}", e),
            Ok(meta) if meta.len() != file.size => {
                format!("size {} != {}", meta.len(), file.size)
            }
            // Hashing a mod jar is blocking IO: run it on the blocking pool.
            Ok(_) => match crate::download::compute_file_hash_async(
                target_path.clone(),
                HashType::Sha256,
            )
            .await
            {
                Ok(actual) if actual.eq_ignore_ascii_case(&file.sha256) => continue,
                Ok(actual) => format!("sha256 {} != {}", actual, file.sha256),
                Err(e) => format!("cannot hash: {}", e),
            },
        };
        events.on_event(&Event::Log {
            level: LogLevel::Info,
            message: format!("Aura-Pack: will download {} ({})", clean_path, reason),
        });

        let (download_url, fallback_url) = pack_file_urls(version, file);
        // Already verified staged files (an interrupted earlier sync) are not
        // downloaded again: the downloader checks the hash first.
        let staged_path = crate::safe_path::safe_join(&staging_dir, &clean_path)?;

        download_tasks.push(DownloadTask {
            url: download_url,
            fallback_url,
            target_path: staged_path,
            size: Some(file.size),
            hash: Some((file.sha256.clone(), HashType::Sha256)),
        });
        staged.push(clean_path);
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
        download_pool_cancellable(
            client,
            download_tasks,
            concurrency,
            events,
            &format!("Pack {}", version),
            cancel_token.cloned(),
        )
        .await?;
    }

    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            return Err(WexError::Cancelled);
        }
    }

    // Phase 2: every file is staged and verified. Record the plan, then commit.
    let pack_version = manifest.pack_version.clone();
    let plan = PackCommitPlan { manifest, staged };
    crate::fs_util::write_atomic(
        &pack_commit_plan_path(paths, version),
        &serde_json::to_vec(&plan)?,
    )?;
    let mods_deleted = commit_pack(paths, version, &plan, events)?;

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Aura-Pack sync complete: version {}, {} files updated, {} orphan mods deleted",
            pack_version, files_to_download_count, mods_deleted
        ),
    });

    Ok(PackSyncResult {
        pack_version,
        files_downloaded: files_to_download_count,
        mods_deleted,
    })
}

pub fn urlencoding_encode(s: &str) -> String {
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
    use crate::download::download_pool;
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
            side: None,
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
            side: None,
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
            side: None,
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
            side: None,
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

    #[tokio::test]
    async fn test_sync_aura_pack_cancellation_preserves_state() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let version = "1.20.1";
        paths.ensure_instance_dirs(version).unwrap();
        let instance_dir = paths.instance_dir(version);

        let mods_dir = instance_dir.join("mods");
        fs::create_dir_all(&mods_dir).unwrap();
        let mod_file = mods_dir.join("important_mod.jar");
        fs::write(&mod_file, b"vital mod data").unwrap();

        let state_path = instance_dir.join("pack.state.json");
        let initial_state = PackState {
            pack_version: "20261101.0000".to_string(),
            minecraft: "1.20.1".to_string(),
            fabric_loader: "0.19.5".to_string(),
            installed_at_utc: chrono::Utc::now().to_rfc3339(),
            installed_mods: vec!["important_mod.jar".to_string()],
            manifest_digest: String::new(),
        };
        fs::write(&state_path, serde_json::to_string(&initial_state).unwrap()).unwrap();

        let token = CancellationToken::new();
        token.cancel();

        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;

        let res =
            sync_aura_pack_cancellable(&paths, version, &client, &events, 2, Some(&token)).await;

        assert!(matches!(res, Err(WexError::Cancelled)));
        assert!(
            mod_file.exists(),
            "Mod file must not be deleted on cancellation"
        );
        let saved_state: PackState =
            serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
        assert_eq!(saved_state.pack_version, "20261101.0000");
    }

    #[tokio::test]
    async fn test_sync_aura_pack_offline_fallback_to_cached_manifest() {
        use sha2::Digest;

        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let version = "1.20.1";
        paths.ensure_instance_dirs(version).unwrap();
        let instance_dir = paths.instance_dir(version);

        let mod_content = b"local cached mod file content";
        let mut hasher = sha2::Sha256::new();
        hasher.update(mod_content);
        let mod_sha = hex::encode(hasher.finalize());

        let mods_dir = instance_dir.join("mods");
        fs::create_dir_all(&mods_dir).unwrap();
        let mod_file = mods_dir.join("offline_mod.jar");
        fs::write(&mod_file, mod_content).unwrap();

        let manifest = Manifest {
            name: "Aura".to_string(),
            pack_version: "20261108.offline".to_string(),
            minecraft: "1.20.1".to_string(),
            fabric_loader: "0.19.5".to_string(),
            servers: None,
            files: vec![ManifestFileEntry {
                path: "mods/offline_mod.jar".to_string(),
                sha256: mod_sha,
                size: mod_content.len() as u64,
                mode: "sync".to_string(),
                url: None,
                side: None,
            }],
        };

        // Cache manifest locally as manifest.cache.json
        let manifest_cache_file = instance_dir.join("manifest.cache.json");
        fs::write(
            &manifest_cache_file,
            serde_json::to_string_pretty(&manifest).unwrap(),
        )
        .unwrap();

        // Offline client (connection will fail immediately)
        let client = reqwest::Client::builder()
            // Route every request to a closed local port so the test is truly offline
            // (a short timeout alone still reaches GitHub on fast CI runners).
            .proxy(reqwest::Proxy::all("http://127.0.0.1:9").unwrap())
            .timeout(std::time::Duration::from_millis(500))
            .build()
            .unwrap();
        let events = crate::event::NoopEventHandler;

        let res = sync_aura_pack(&paths, version, &client, &events, 2).await;
        assert!(
            res.is_ok(),
            "Offline sync with cached manifest and installed files must succeed: {:?}",
            res
        );
        let sync_result = res.unwrap();
        assert_eq!(sync_result.pack_version, "20261108.offline");
        assert_eq!(sync_result.files_downloaded, 0);
    }

    #[test]
    fn resolve_pack_file_rejects_traversal() {
        let tmp = tempdir().unwrap();
        for bad in [
            "../evil.jar",
            "mods/../../evil",
            r"C:\evil",
            "/abs",
            "a/b:ads",
        ] {
            assert!(resolve_pack_file(tmp.path(), bad).is_err(), "{}", bad);
        }
        let err = resolve_pack_file(tmp.path(), "mods/../../evil.jar")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("mods/../../evil.jar"),
            "error must name the path: {err}"
        );
        let (clean, target) = resolve_pack_file(tmp.path(), r"mods\ok.jar").unwrap();
        assert_eq!(clean, "mods/ok.jar");
        assert!(target.starts_with(tmp.path()));
    }

    fn sample_manifest(version: &str, pack_version: &str, sha: &str) -> Manifest {
        Manifest {
            name: "Aura".into(),
            pack_version: pack_version.into(),
            minecraft: version.into(),
            fabric_loader: "0.19.5".into(),
            servers: None,
            files: vec![ManifestFileEntry {
                path: "mods/a.jar".into(),
                sha256: sha.into(),
                size: 3,
                mode: "sync".into(),
                url: None,
                side: None,
            }],
        }
    }

    #[test]
    fn manifest_digest_tracks_file_set_and_hashes_not_order() {
        let a = sample_manifest("1.21.1", "1", &"a".repeat(64));
        let mut b = a.clone();
        b.files.push(ManifestFileEntry {
            path: "config/x.json".into(),
            ..a.files[0].clone()
        });
        let mut b_reordered = b.clone();
        b_reordered.files.reverse();
        assert_ne!(manifest_digest(&a), manifest_digest(&b));
        assert_eq!(manifest_digest(&b), manifest_digest(&b_reordered));
        let c = sample_manifest("1.21.1", "1", &"b".repeat(64));
        assert_ne!(manifest_digest(&a), manifest_digest(&c));
    }

    #[tokio::test]
    async fn pack_freshness_compares_published_manifest_with_state() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for version in ["1.20.1", "1.21.1"] {
            let tmp = tempdir().unwrap();
            let paths = WexPaths::new(Some(tmp.path().join("Wex")));
            paths.ensure_instance_dirs(version).unwrap();
            let installed = sample_manifest(version, "1", &"a".repeat(64));
            let state = PackState {
                pack_version: "1".into(),
                minecraft: version.into(),
                fabric_loader: "0.19.5".into(),
                installed_at_utc: "t".into(),
                installed_mods: vec!["a.jar".into()],
                manifest_digest: manifest_digest(&installed),
            };
            fs::write(
                paths.instance_dir(version).join("pack.state.json"),
                serde_json::to_vec(&state).unwrap(),
            )
            .unwrap();
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/same.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&installed))
                .mount(&server)
                .await;
            let bumped = sample_manifest(version, "2", &"a".repeat(64));
            Mock::given(method("GET"))
                .and(path("/bumped.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&bumped))
                .mount(&server)
                .await;
            let rehashed = sample_manifest(version, "1", &"c".repeat(64));
            Mock::given(method("GET"))
                .and(path("/rehashed.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&rehashed))
                .mount(&server)
                .await;
            let client = reqwest::Client::new();
            let check = |p: &'static str| {
                let url = format!("{}{}", server.uri(), p);
                let paths = paths.clone();
                let client = client.clone();
                async move { check_pack_freshness_at(&paths, version, &client, &url).await }
            };
            assert!(matches!(check("/same.json").await, PackFreshness::Current));
            assert!(matches!(
                check("/bumped.json").await,
                PackFreshness::Outdated(_)
            ));
            assert!(matches!(
                check("/rehashed.json").await,
                PackFreshness::Outdated(_)
            ));
            let offline = check_pack_freshness_at(
                &paths,
                version,
                &client,
                "http://127.0.0.1:1/manifest.json",
            )
            .await;
            assert!(matches!(offline, PackFreshness::Unknown(_)));
        }
    }

    #[test]
    fn pack_versions_compare_numerically() {
        use std::cmp::Ordering::*;
        assert_eq!(
            compare_pack_versions("20261025.1406", "20261025.1406"),
            Equal
        );
        assert_eq!(
            compare_pack_versions("20261025.1406", "20261025.959"),
            Greater
        );
        assert_eq!(
            compare_pack_versions("20261101.0000", "20261025.2359"),
            Greater
        );
        assert_eq!(compare_pack_versions("2", "10"), Less);
        assert_eq!(compare_pack_versions("1.2", "1.2.1"), Less);
        assert_eq!(compare_pack_versions("test", "test"), Equal);
    }

    #[tokio::test]
    async fn stale_older_manifest_does_not_downgrade_the_pack() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for version in ["1.20.1", "1.21.1"] {
            let tmp = tempdir().unwrap();
            let paths = WexPaths::new(Some(tmp.path().join("Wex")));
            paths.ensure_instance_dirs(version).unwrap();
            let installed = sample_manifest(version, "20261102.1200", &"a".repeat(64));
            let state = PackState {
                pack_version: installed.pack_version.clone(),
                minecraft: version.into(),
                fabric_loader: "0.19.5".into(),
                installed_at_utc: "t".into(),
                installed_mods: vec!["a.jar".into()],
                manifest_digest: manifest_digest(&installed),
            };
            fs::write(
                paths.instance_dir(version).join("pack.state.json"),
                serde_json::to_vec(&state).unwrap(),
            )
            .unwrap();
            let server = MockServer::start().await;
            let older = sample_manifest(version, "20261101.0900", &"b".repeat(64));
            Mock::given(method("GET"))
                .and(path("/older.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&older))
                .mount(&server)
                .await;
            let newer = sample_manifest(version, "20261102.1300", &"a".repeat(64));
            Mock::given(method("GET"))
                .and(path("/newer.json"))
                .respond_with(ResponseTemplate::new(200).set_body_json(&newer))
                .mount(&server)
                .await;
            let client = reqwest::Client::new();
            let older_url = format!("{}/older.json", server.uri());
            let newer_url = format!("{}/newer.json", server.uri());
            assert!(matches!(
                check_pack_freshness_at(&paths, version, &client, &older_url).await,
                PackFreshness::Current
            ));
            assert!(matches!(
                check_pack_freshness_at(&paths, version, &client, &newer_url).await,
                PackFreshness::Outdated(_)
            ));
        }
    }

    #[tokio::test]
    async fn slow_manifest_check_gives_up_within_the_deadline() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(sample_manifest("1.21.1", "2", &"a".repeat(64)))
                    .set_delay(std::time::Duration::from_secs(10)),
            )
            .mount(&server)
            .await;
        // The production client: 15 s connect + 30 s idle read, no total deadline.
        let client = crate::download::http_client_builder().build().unwrap();
        let start = std::time::Instant::now();
        let res = check_pack_freshness_at(
            &paths,
            "1.21.1",
            &client,
            &format!("{}/manifest.json", server.uri()),
        )
        .await;
        let PackFreshness::Unknown(e) = res else {
            panic!("expected Unknown, got {res:?}");
        };
        assert!(e.is_network_unavailable(), "{e}");
        assert!(start.elapsed() < PACK_CHECK_TIMEOUT + std::time::Duration::from_secs(2));
    }
}
