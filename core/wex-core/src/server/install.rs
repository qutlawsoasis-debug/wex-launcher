//! Installing a dedicated Fabric server.
//!
//! 1. Mojang `server.jar` from the version package, checked by SHA-1.
//! 2. Fabric server profile (`/v2/versions/loader/<mc>/<loader>/server/json`),
//!    its libraries checked by the SHA-1/SHA-256 that Fabric meta publishes.
//! 3. Java from the existing Mojang runtime logic (`java::ensure_mojang_java`).
//! 4. Aura-Pack mods for the server (see [`plan_server_mods`]).
//! 5. `server.properties` from the settings. `eula.txt` is never written here.
//!
//! The world folder and files the player added to `mods/` are never touched.

use super::{InstallInfo, ModSource, ServerRecord, ServerStore};
use crate::config::WexPaths;
use crate::download::{
    compute_file_hash, download_file_resumable, download_pool, DownloadTask, HashType,
};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use crate::fabric::{self, FabricProfile, DEFAULT_FABRIC_LOADER};
use crate::pack::{self, Manifest, ManifestFileEntry};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const FABRIC_PROFILE_FILE: &str = "fabric-server.json";

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub concurrency: usize,
    /// Install the Aura-Pack server mods. Off gives a vanilla Fabric server.
    pub include_pack_mods: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            concurrency: 6,
            include_pack_mods: true,
        }
    }
}

pub fn server_jar_path(paths: &WexPaths, version: &str) -> PathBuf {
    paths
        .versions_dir
        .join(version)
        .join(format!("{version}-server.jar"))
}

/// What a mod jar declares in `fabric.mod.json` → `environment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModEnvironment {
    Client,
    Server,
    Both,
    /// No readable `fabric.mod.json`.
    Unknown,
}

pub fn read_mod_environment(jar: &Path) -> ModEnvironment {
    let Ok(file) = fs::File::open(jar) else {
        return ModEnvironment::Unknown;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return ModEnvironment::Unknown;
    };
    let Ok(mut entry) = zip.by_name("fabric.mod.json") else {
        return ModEnvironment::Unknown;
    };
    let mut text = String::new();
    if entry.read_to_string(&mut text).is_err() {
        return ModEnvironment::Unknown;
    }
    // Some mods put raw newlines inside strings; strip control characters first.
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&cleaned) else {
        return ModEnvironment::Unknown;
    };
    match json.get("environment").and_then(|v| v.as_str()) {
        Some("client") => ModEnvironment::Client,
        Some("server") => ModEnvironment::Server,
        _ => ModEnvironment::Both, // "*" or missing: the Fabric default is both sides
    }
}

/// A pack mod that may go on the server.
#[derive(Debug, Clone)]
pub struct PlannedMod {
    pub file_name: String,
    pub entry: ManifestFileEntry,
    /// Decided by the manifest; otherwise the jar is inspected after download.
    pub needs_inspection: bool,
}

/// Which pack files go to the server.
///
/// * The manifest `side` field decides when present (`client` stays off;
///   `server`, `both` or `*` go on).
/// * Without it, the jar's own `fabric.mod.json` decides after the download.
/// * Jars shipped disabled (`*.jar.disabled`) and non-mod files are skipped.
/// * Jars already found client-only (by SHA-256) are not fetched again.
pub fn plan_server_mods(
    manifest: &Manifest,
    known_client_only: &[String],
) -> (Vec<PlannedMod>, ModSource) {
    let mut out = Vec::new();
    let mut all_marked = true;
    let mut any = false;
    for file in &manifest.files {
        let path = file.path.replace('\\', "/");
        let Some(name) = path.strip_prefix("mods/") else {
            continue;
        };
        if name.contains('/') || !name.ends_with(".jar") {
            continue;
        }
        any = true;
        let side = file.side.as_deref().map(str::to_ascii_lowercase);
        match side.as_deref() {
            Some("client") => continue,
            Some("server") | Some("both") | Some("*") => out.push(PlannedMod {
                file_name: name.to_string(),
                entry: file.clone(),
                needs_inspection: false,
            }),
            _ => {
                all_marked = false;
                if known_client_only
                    .iter()
                    .any(|h| h.eq_ignore_ascii_case(&file.sha256))
                {
                    continue;
                }
                out.push(PlannedMod {
                    file_name: name.to_string(),
                    entry: file.clone(),
                    needs_inspection: true,
                });
            }
        }
    }
    let source = if any && all_marked {
        ModSource::Manifest
    } else {
        ModSource::FabricModJson
    };
    (out, source)
}

fn file_matches(path: &Path, size: u64, sha256: &str) -> bool {
    fs::metadata(path).is_ok_and(|m| m.len() == size)
        && compute_file_hash(path, HashType::Sha256).is_ok_and(|h| h.eq_ignore_ascii_case(sha256))
}

fn log(events: &dyn EventHandler, level: LogLevel, message: String) {
    events.on_event(&Event::Log { level, message });
}

fn stage(events: &dyn EventHandler, name: &str, step: usize) {
    events.on_event(&Event::StageStarted {
        stage: name.to_string(),
        step,
        total_steps: 5,
    });
}

/// Puts the planned mods into `mods/`, reusing verified client copies, and
/// returns (installed file names, newly found client-only hashes).
async fn install_pack_mods(
    paths: &WexPaths,
    version: &str,
    mods_dir: &Path,
    planned: &[PlannedMod],
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<(Vec<String>, Vec<String>)> {
    fs::create_dir_all(mods_dir)?;
    let client_mods = paths.instance_dir(version).join("mods");
    let mut tasks = Vec::new();
    for m in planned {
        let target = mods_dir.join(&m.file_name);
        if file_matches(&target, m.entry.size, &m.entry.sha256) {
            continue;
        }
        let local = client_mods.join(&m.file_name);
        if file_matches(&local, m.entry.size, &m.entry.sha256) {
            fs::copy(&local, &target)?;
            continue;
        }
        let (url, fallback_url) = pack::pack_file_urls(version, &m.entry);
        tasks.push(DownloadTask {
            url,
            fallback_url,
            target_path: target,
            size: Some(m.entry.size),
            hash: Some((m.entry.sha256.clone(), HashType::Sha256)),
        });
    }
    log(
        events,
        LogLevel::Info,
        format!(
            "Server mods: {} to download, {} planned",
            tasks.len(),
            planned.len()
        ),
    );
    download_pool(client, tasks, concurrency, events, "Server mods").await?;

    let mut installed = Vec::new();
    let mut client_only = Vec::new();
    for m in planned {
        let target = mods_dir.join(&m.file_name);
        if !file_matches(&target, m.entry.size, &m.entry.sha256) {
            return Err(WexError::HashMismatch {
                path: target,
                expected: m.entry.sha256.clone(),
                actual: "unverified".into(),
            });
        }
        if m.needs_inspection && read_mod_environment(&target) == ModEnvironment::Client {
            fs::remove_file(&target)?;
            client_only.push(m.entry.sha256.to_lowercase());
            continue;
        }
        installed.push(m.file_name.clone());
    }
    Ok((installed, client_only))
}

/// Removes pack mods Wex installed earlier that the new plan no longer has.
/// Files Wex never placed (the player's own mods) stay.
pub fn remove_stale_managed_mods(
    mods_dir: &Path,
    previous: &[String],
    current: &[String],
) -> Result<usize> {
    let keep: HashSet<&str> = current.iter().map(String::as_str).collect();
    let mut removed = 0;
    for name in previous {
        if keep.contains(name.as_str()) || name.contains(['/', '\\']) || name.contains("..") {
            continue;
        }
        let path = mods_dir.join(name);
        if path.is_file() {
            fs::remove_file(path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// Installs or updates the server. Safe to run again: verified files are kept.
pub async fn install_server(
    paths: &WexPaths,
    store: &ServerStore,
    id: &str,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    opts: &InstallOptions,
) -> Result<ServerRecord> {
    let mut rec = store.load(id)?;
    super::validate_version(&rec.version)?;
    let version = rec.version.clone();
    let dir = store.server_dir(id)?;
    fs::create_dir_all(&dir)?;
    paths.ensure_dirs()?;

    stage(events, "Minecraft server", 1);
    let pkg = crate::minecraft::fetch_version_package(client, &version).await?;
    let server = pkg.downloads.server.clone().ok_or_else(|| {
        WexError::Server(format!("Mojang не публикует серверный jar для {version}"))
    })?;
    let jar = server_jar_path(paths, &version);
    if let Some(parent) = jar.parent() {
        fs::create_dir_all(parent)?;
    }
    download_file_resumable(
        client,
        &server.url,
        &jar,
        Some(server.size),
        Some((&server.sha1, HashType::Sha1)),
        events,
    )
    .await?;

    stage(events, "Fabric server", 2);
    let profile: FabricProfile =
        fabric::fetch_fabric_server_profile(client, &version, None).await?;
    if !profile.main_class.contains("Server") {
        return Err(WexError::ManifestError(format!(
            "Unexpected Fabric server main class {}",
            profile.main_class
        )));
    }
    fabric::install_fabric_libraries(paths, &profile, client, events, opts.concurrency).await?;
    fs::write(
        dir.join(FABRIC_PROFILE_FILE),
        serde_json::to_vec_pretty(&profile)?,
    )?;

    stage(events, "Java", 3);
    crate::java::ensure_mojang_java(paths, &version, client, events, opts.concurrency).await?;

    stage(events, "Server mods", 4);
    let previous = rec.installed.clone();
    let (mods, mod_source, pack_version, client_only) = if opts.include_pack_mods {
        let manifest = pack::fetch_manifest(client, &version, None, None).await?;
        let known = previous
            .as_ref()
            .map(|p| p.client_only.clone())
            .unwrap_or_default();
        let (planned, source) = plan_server_mods(&manifest, &known);
        let (installed, mut found) = install_pack_mods(
            paths,
            &version,
            &dir.join("mods"),
            &planned,
            client,
            events,
            opts.concurrency,
        )
        .await?;
        found.extend(known);
        found.sort();
        found.dedup();
        (installed, source, Some(manifest.pack_version), found)
    } else {
        (Vec::new(), ModSource::NoPackMods, None, Vec::new())
    };
    if let Some(prev) = &previous {
        remove_stale_managed_mods(&dir.join("mods"), &prev.mods, &mods)?;
    }

    stage(events, "server.properties", 5);
    rec.installed = Some(InstallInfo {
        minecraft: version.clone(),
        fabric_loader: profile
            .id
            .strip_prefix("fabric-loader-")
            .and_then(|s| s.split('-').next())
            .unwrap_or(DEFAULT_FABRIC_LOADER)
            .to_string(),
        pack_version,
        mod_source,
        mods,
        client_only,
        installed_at: chrono::Utc::now().to_rfc3339(),
    });
    store.write_properties(&rec)?;
    store.save(&rec)?;
    log(
        events,
        LogLevel::Info,
        format!("Server {id} installed for Minecraft {version}"),
    );
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn entry(path: &str, sha: &str, side: Option<&str>) -> ManifestFileEntry {
        ManifestFileEntry {
            path: path.into(),
            sha256: sha.into(),
            size: 1,
            mode: "sync".into(),
            url: None,
            side: side.map(Into::into),
        }
    }

    fn manifest(files: Vec<ManifestFileEntry>) -> Manifest {
        Manifest {
            name: "Wex".into(),
            pack_version: "1".into(),
            minecraft: "1.21.1".into(),
            fabric_loader: "0.19.5".into(),
            servers: None,
            files,
        }
    }

    fn jar(dir: &Path, name: &str, fabric_mod_json: Option<&str>) -> PathBuf {
        let path = dir.join(name);
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        if let Some(json) = fabric_mod_json {
            zip.start_file("fabric.mod.json", opts).unwrap();
            zip.write_all(json.as_bytes()).unwrap();
        }
        zip.start_file("a.txt", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        path
    }

    #[test]
    fn plan_without_side_inspects_every_mod_jar() {
        let m = manifest(vec![
            entry("mods/a.jar", "aa", None),
            entry("mods/off.jar.disabled", "bb", None),
            entry("config/a.json", "cc", None),
            entry("mods/sub/x.jar", "dd", None),
            entry("mods/known-client.jar", "EE", None),
        ]);
        let (plan, source) = plan_server_mods(&m, &["ee".into()]);
        assert_eq!(source, ModSource::FabricModJson);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].file_name, "a.jar");
        assert!(plan[0].needs_inspection);
    }

    #[test]
    fn plan_with_side_follows_manifest() {
        let m = manifest(vec![
            entry("mods/client.jar", "1", Some("client")),
            entry("mods/server.jar", "2", Some("server")),
            entry("mods/both.jar", "3", Some("Both")),
        ]);
        let (plan, source) = plan_server_mods(&m, &[]);
        assert_eq!(source, ModSource::Manifest);
        let names: Vec<_> = plan.iter().map(|p| p.file_name.as_str()).collect();
        assert_eq!(names, ["server.jar", "both.jar"]);
        assert!(plan.iter().all(|p| !p.needs_inspection));
    }

    #[test]
    fn mod_environment_from_fabric_mod_json() {
        let tmp = tempfile::tempdir().unwrap();
        let d = tmp.path();
        assert_eq!(
            read_mod_environment(&jar(
                d,
                "c.jar",
                Some(r#"{"id":"c","environment":"client"}"#)
            )),
            ModEnvironment::Client
        );
        assert_eq!(
            read_mod_environment(&jar(
                d,
                "s.jar",
                Some(r#"{"id":"s","environment":"server"}"#)
            )),
            ModEnvironment::Server
        );
        assert_eq!(
            read_mod_environment(&jar(d, "b.jar", Some(r#"{"id":"b","environment":"*"}"#))),
            ModEnvironment::Both
        );
        assert_eq!(
            read_mod_environment(&jar(d, "m.jar", Some(r#"{"id":"m"}"#))),
            ModEnvironment::Both
        );
        assert_eq!(
            read_mod_environment(&jar(
                d,
                "nl.jar",
                Some("{\"id\":\"x\",\"description\":\"a\nb\",\"environment\":\"client\"}")
            )),
            ModEnvironment::Client
        );
        assert_eq!(
            read_mod_environment(&jar(d, "n.jar", None)),
            ModEnvironment::Unknown
        );
        fs::write(d.join("bad.jar"), b"not a zip").unwrap();
        assert_eq!(
            read_mod_environment(&d.join("bad.jar")),
            ModEnvironment::Unknown
        );
    }

    #[test]
    fn stale_managed_mods_go_and_player_mods_stay() {
        let tmp = tempfile::tempdir().unwrap();
        let mods = tmp.path();
        for n in ["old.jar", "kept.jar", "mine.jar"] {
            fs::write(mods.join(n), b"x").unwrap();
        }
        let removed = remove_stale_managed_mods(
            mods,
            &["old.jar".into(), "kept.jar".into(), "../escape.jar".into()],
            &["kept.jar".into()],
        )
        .unwrap();
        assert_eq!(removed, 1);
        assert!(!mods.join("old.jar").exists());
        assert!(mods.join("kept.jar").exists());
        assert!(mods.join("mine.jar").exists());
    }

    #[tokio::test]
    async fn pack_mods_reuse_client_copy_and_drop_client_only() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("data")));
        let client_mods = paths.instance_dir("1.21.1").join("mods");
        fs::create_dir_all(&client_mods).unwrap();
        let both = jar(
            &client_mods,
            "both.jar",
            Some(r#"{"id":"b","environment":"*"}"#),
        );
        let cli = jar(
            &client_mods,
            "cli.jar",
            Some(r#"{"id":"c","environment":"client"}"#),
        );
        let mk = |p: &Path, name: &str| PlannedMod {
            file_name: name.into(),
            entry: ManifestFileEntry {
                path: format!("mods/{name}"),
                sha256: compute_file_hash(p, HashType::Sha256).unwrap(),
                size: fs::metadata(p).unwrap().len(),
                mode: "sync".into(),
                // Not reachable: the test fails if Wex tries to download.
                url: Some("http://127.0.0.1:9/never".into()),
                side: None,
            },
            needs_inspection: true,
        };
        let planned = vec![mk(&both, "both.jar"), mk(&cli, "cli.jar")];
        let server_mods = tmp.path().join("server/mods");
        let (installed, client_only) = install_pack_mods(
            &paths,
            "1.21.1",
            &server_mods,
            &planned,
            &reqwest::Client::new(),
            &crate::event::NoopEventHandler,
            2,
        )
        .await
        .unwrap();
        assert_eq!(installed, vec!["both.jar".to_string()]);
        assert_eq!(client_only, vec![planned[1].entry.sha256.to_lowercase()]);
        assert!(server_mods.join("both.jar").exists());
        assert!(!server_mods.join("cli.jar").exists());
    }

    #[test]
    fn server_jar_is_shared_per_version() {
        let paths = WexPaths::new(Some(PathBuf::from("root")));
        assert_eq!(
            server_jar_path(&paths, "1.20.1"),
            PathBuf::from("root/versions/1.20.1/1.20.1-server.jar")
        );
    }
}
