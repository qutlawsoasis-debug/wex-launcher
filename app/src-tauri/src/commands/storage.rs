use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Emitter, State};
use wex_core::config::WexPaths;
use wex_core::download::{compute_file_hash, download_pool, DownloadTask, HashType};
use wex_core::event::{Event, EventHandler, LogLevel};
use wex_core::pack::{
    fetch_manifest, get_manifest_filename, get_repo_prefix, Manifest, DEFAULT_PACK_BRANCH,
    DEFAULT_PACK_REPO,
};

use crate::commands::launch::PlayState;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct IntegrityCheckResultDto {
    pub total_files: u32,
    pub checked_files: u32,
    pub corrupted_files: u32,
    pub repaired_files: u32,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StorageCacheInfoDto {
    pub total_bytes: u64,
    pub formatted_size: String,
    pub logs_bytes: u64,
    pub crash_reports_bytes: u64,
    pub temp_bytes: u64,
    pub file_count: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ClearStorageCacheResultDto {
    pub bytes_freed: u64,
    pub formatted_freed: String,
    pub files_deleted: u32,
    pub status_text: String,
}

pub const SEVEN_DAYS_SECS: u64 = 7 * 24 * 60 * 60;
pub const EVT_STORAGE_VERIFY_PROGRESS: &str = "storage-verify-progress";

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Format bytes into human-readable Apple HIG string (e.g. "14.2 МБ", "520 КБ", "0 Б").
pub fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} ГБ", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} МБ", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} КБ", bytes as f64 / 1024.0)
    } else {
        format!("{} Б", bytes)
    }
}

/// Check if a path belongs to critical player data that MUST NEVER be deleted.
pub fn is_protected_path(path: &Path) -> bool {
    let s = path.to_string_lossy().replace('\\', "/").to_lowercase();
    s.contains("/saves/")
        || s.contains("/screenshots/")
        || s.contains("/skins/")
        || s.contains("/capes/")
        || s.contains("/resourcepacks/")
        || s.contains("/shaderpacks/")
        || s.ends_with("/options.txt")
        || s.ends_with("/config.json")
        || s.ends_with("/servers.dat")
        || s.ends_with("/pack.state.json")
}

#[derive(Debug, Default, Clone)]
pub struct ScannedCache {
    pub files: Vec<PathBuf>,
    pub total_bytes: u64,
    pub logs_bytes: u64,
    pub crash_reports_bytes: u64,
    pub temp_bytes: u64,
}

/// Recursively scans logs, old crash reports, and temp/download caches.
pub fn scan_cache(paths: &WexPaths) -> ScannedCache {
    let mut scanned = ScannedCache::default();
    let now = SystemTime::now();
    let seven_days_ago = now
        .checked_sub(Duration::from_secs(SEVEN_DAYS_SECS))
        .unwrap_or(now);

    let mut log_dirs = vec![paths.logs_dir.clone()];
    let mut crash_dirs = vec![paths.root.join("crash-reports")];
    let mut temp_dirs = vec![
        paths.root.join("temp"),
        paths.root.join(".cache"),
        paths.root.join("downloads"),
    ];

    // Gather instance-specific log, crash, and temp dirs
    if let Ok(entries) = fs::read_dir(&paths.instances_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                log_dirs.push(entry.path().join("logs"));
                crash_dirs.push(entry.path().join("crash-reports"));
                temp_dirs.push(entry.path().join(".temp"));
            }
        }
    }

    // 1. Scan logs
    for dir in log_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && !is_protected_path(&path) {
                    let file_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    // Never delete active latest.log
                    if file_name.eq_ignore_ascii_case("latest.log") {
                        continue;
                    }

                    let is_gz = file_name.ends_with(".log.gz");
                    let is_old_log = if let Ok(meta) = entry.metadata() {
                        let is_old = meta.modified().map(|m| m < seven_days_ago).unwrap_or(false);
                        file_name.ends_with(".log") && is_old
                    } else {
                        false
                    };

                    if is_gz || is_old_log {
                        if let Ok(meta) = entry.metadata() {
                            let len = meta.len();
                            scanned.logs_bytes += len;
                            scanned.total_bytes += len;
                            scanned.files.push(path);
                        }
                    }
                }
            }
        }
    }

    // 2. Scan crash reports older than 7 days
    for dir in crash_dirs {
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && !is_protected_path(&path) {
                    if let Ok(meta) = entry.metadata() {
                        let is_old = meta.modified().map(|m| m < seven_days_ago).unwrap_or(false);
                        if is_old {
                            let len = meta.len();
                            scanned.crash_reports_bytes += len;
                            scanned.total_bytes += len;
                            scanned.files.push(path);
                        }
                    }
                }
            }
        }
    }

    // 3. Scan temp and downloads
    for dir in temp_dirs {
        if !dir.exists() {
            continue;
        }
        collect_dir_files(&dir, &mut scanned, &seven_days_ago);
    }

    scanned
}

fn collect_dir_files(dir: &Path, scanned: &mut ScannedCache, _seven_days_ago: &SystemTime) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_protected_path(&path) {
                continue;
            }
            if path.is_file() {
                if let Ok(meta) = entry.metadata() {
                    let len = meta.len();
                    scanned.temp_bytes += len;
                    scanned.total_bytes += len;
                    scanned.files.push(path);
                }
            } else if path.is_dir() {
                collect_dir_files(&path, scanned, _seven_days_ago);
            }
        }
    }
}

/// Pure implementation of cache calculation.
pub fn get_storage_cache_info_impl(paths: &WexPaths) -> StorageCacheInfoDto {
    let scanned = scan_cache(paths);
    StorageCacheInfoDto {
        total_bytes: scanned.total_bytes,
        formatted_size: format_bytes(scanned.total_bytes),
        logs_bytes: scanned.logs_bytes,
        crash_reports_bytes: scanned.crash_reports_bytes,
        temp_bytes: scanned.temp_bytes,
        file_count: scanned.files.len() as u32,
    }
}

/// Pure implementation of cache cleaning.
pub fn clear_storage_cache_impl(
    paths: &WexPaths,
    is_game_running: bool,
) -> Result<ClearStorageCacheResultDto, String> {
    if is_game_running {
        return Err("Нельзя очищать кэш во время запущенной игры".to_string());
    }

    let scanned = scan_cache(paths);
    let mut bytes_freed: u64 = 0;
    let mut files_deleted: u32 = 0;

    for file in scanned.files {
        if is_protected_path(&file) {
            continue;
        }
        if let Ok(meta) = fs::metadata(&file) {
            let len = meta.len();
            if fs::remove_file(&file).is_ok() {
                bytes_freed += len;
                files_deleted += 1;
            }
        }
    }

    let formatted_freed = format_bytes(bytes_freed);
    let status_text = if files_deleted > 0 {
        format!("Освобождено {}", formatted_freed)
    } else {
        "Временные файлы не найдены".to_string()
    };

    Ok(ClearStorageCacheResultDto {
        bytes_freed,
        formatted_freed,
        files_deleted,
        status_text,
    })
}

/// Pure implementation of integrity verification and corrupted file repair.
pub async fn verify_game_files_impl<E: EventHandler>(
    paths: &WexPaths,
    version: &str,
    is_game_running: bool,
    client: &reqwest::Client,
    events: &E,
) -> Result<IntegrityCheckResultDto, String> {
    if is_game_running {
        return Err("Нельзя проверять файлы во время запущенной игры".to_string());
    }

    let clean_version = if version.trim().is_empty() {
        "1.21.1"
    } else {
        version.trim()
    };

    paths
        .ensure_instance_dirs(clean_version)
        .map_err(|e| format!("Ошибка создания каталогов инстанса: {}", e))?;

    let instance_dir = paths.instance_dir(clean_version);

    events.on_event(&Event::StageStarted {
        stage: format!("Проверка целостности файлов ({})", clean_version),
        step: 1,
        total_steps: 2,
    });

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Получение манифеста файлов для Minecraft {}...",
            clean_version
        ),
    });

    // Check local manifest first (the installed pack manifest); otherwise fetch from remote
    let manifest_filename = get_manifest_filename(clean_version);
    let local_candidates = [
        instance_dir.join(manifest_filename),
        instance_dir.join("manifest.json"),
        paths
            .versions_dir
            .join(clean_version)
            .join(manifest_filename),
    ];
    let mut local_manifest = None;
    for candidate in &local_candidates {
        if candidate.exists() {
            if let Ok(content) = fs::read_to_string(candidate) {
                if let Ok(parsed) = serde_json::from_str::<Manifest>(&content) {
                    local_manifest = Some(parsed);
                    break;
                }
            }
        }
    }

    let manifest = match local_manifest {
        Some(m) => m,
        None => fetch_manifest(client, clean_version, None, None)
            .await
            .map_err(|e| {
                format!(
                    "Не удалось получить манифест файлов для версии {} (сеть: {}) и локальный манифест не найден.",
                    clean_version, e
                )
            })?,
    };

    let total_files = manifest.files.len() as u32;
    let mut checked_files: u32 = 0;
    let mut corrupted_tasks: Vec<DownloadTask> = Vec::new();

    let repo = DEFAULT_PACK_REPO;
    let branch = DEFAULT_PACK_BRANCH;
    let repo_prefix = get_repo_prefix(clean_version);

    for (idx, file) in manifest.files.iter().enumerate() {
        checked_files += 1;
        let clean_path = file.path.replace('\\', "/");
        let target_path = instance_dir.join(&clean_path);

        // Check if user disabled this mod (.disabled)
        if clean_path.starts_with("mods/") && clean_path.ends_with(".jar") {
            let disabled_target_path = instance_dir.join(format!("{}.disabled", clean_path));
            if disabled_target_path.exists() {
                if let Ok(meta) = fs::metadata(&disabled_target_path) {
                    if meta.len() == file.size {
                        continue;
                    }
                }
            }
        }

        // If file mode is default and file already exists, don't overwrite user custom settings
        if file.mode.eq_ignore_ascii_case("default") && target_path.exists() {
            continue;
        }

        let mut needs_repair = true;
        if target_path.exists() {
            if let Ok(meta) = fs::metadata(&target_path) {
                if meta.len() == file.size {
                    if let Ok(actual_hash) = compute_file_hash(&target_path, HashType::Sha256) {
                        if actual_hash.eq_ignore_ascii_case(&file.sha256) {
                            needs_repair = false;
                        }
                    }
                }
            }
        }

        if needs_repair {
            let escaped_segments: Vec<String> =
                clean_path.split('/').map(urlencoding_encode).collect();
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

            corrupted_tasks.push(DownloadTask {
                url: download_url,
                fallback_url,
                target_path,
                size: Some(file.size),
                hash: Some((file.sha256.clone(), HashType::Sha256)),
            });
        }

        // Report progress every 5 files or on final
        if idx % 5 == 0 || idx + 1 == manifest.files.len() {
            events.on_event(&Event::Progress {
                stage: format!("Проверка файлов ({}/{})", idx + 1, total_files),
                current: (idx + 1) as u64,
                total: total_files as u64,
                message: Some(clean_path),
            });
        }
    }

    let corrupted_files = corrupted_tasks.len() as u32;
    let mut repaired_files: u32 = 0;

    if !corrupted_tasks.is_empty() {
        events.on_event(&Event::StageStarted {
            stage: format!("Восстановление повреждённых файлов ({})", corrupted_files),
            step: 2,
            total_steps: 2,
        });

        // Download and repair corrupted files
        let download_res =
            download_pool(client, corrupted_tasks, 4, events, "Восстановление файлов").await;
        if let Err(e) = download_res {
            return Err(format!("Ошибка перекачки повреждённых файлов: {}", e));
        }

        // The pool returns success only after every initially corrupted task has
        // passed its expected size/hash checks and been installed on disk.
        // Healthy, default-mode and disabled files were never repair tasks.
        repaired_files = corrupted_files;
    }

    let status = if corrupted_files == 0 {
        "Всё в порядке".to_string()
    } else {
        format!("Исправлено {} файлов", corrupted_files)
    };

    let message = if corrupted_files == 0 {
        format!(
            "Все файлы сборки Minecraft {} проверены и в порядке (всего файлов: {}).",
            clean_version, total_files
        )
    } else {
        format!(
            "Проверка завершена. Обнаружено повреждённых: {}. Успешно восстановлено: {} из {} проверенных.",
            corrupted_files, repaired_files, checked_files
        )
    };

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: "Проверка целостности файлов завершена".to_string(),
    });

    Ok(IntegrityCheckResultDto {
        total_files,
        checked_files,
        corrupted_files,
        repaired_files,
        status,
        message,
    })
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Tauri command to get real storage cache size.
#[tauri::command]
pub async fn get_storage_cache_info(
    state: State<'_, PlayState>,
) -> Result<StorageCacheInfoDto, String> {
    Ok(get_storage_cache_info_impl(&state.paths))
}

/// Tauri command to clear storage cache. Rejects if game is currently running.
#[tauri::command]
pub async fn clear_storage_cache(
    state: State<'_, PlayState>,
) -> Result<ClearStorageCacheResultDto, String> {
    let is_running = state.is_running.load(Ordering::SeqCst);
    clear_storage_cache_impl(&state.paths, is_running)
}

/// Tauri command to verify game files and repair corruptions. Rejects if game is running.
#[tauri::command]
pub async fn verify_game_files(
    version: Option<String>,
    state: State<'_, PlayState>,
    app: AppHandle,
) -> Result<IntegrityCheckResultDto, String> {
    let is_running = state.is_running.load(Ordering::SeqCst);
    let target_version = version.unwrap_or_else(|| "1.21.1".to_string());
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Ошибка создания HTTP клиента: {}", e))?;

    struct AppEventHandler {
        app: AppHandle,
    }

    impl EventHandler for AppEventHandler {
        fn on_event(&self, event: &Event) {
            match event {
                Event::Progress {
                    stage,
                    current,
                    total,
                    message,
                } => {
                    let pct = if *total > 0 {
                        (*current as f64 / *total as f64) * 100.0
                    } else {
                        0.0
                    };
                    let _ = self.app.emit(
                        EVT_STORAGE_VERIFY_PROGRESS,
                        serde_json::json!({
                            "stage": stage,
                            "downloaded": current,
                            "total": total,
                            "percentage": pct,
                            "message": message,
                        }),
                    );
                }
                Event::StageStarted { stage, .. } => {
                    let _ = self.app.emit(
                        EVT_STORAGE_VERIFY_PROGRESS,
                        serde_json::json!({
                            "stage": stage,
                        }),
                    );
                }
                _ => {}
            }
        }
    }

    let handler = AppEventHandler { app };
    // Repairs write instance files: never overlap a background install or Play.
    // A running game is rejected inside the impl, so only wait when it may repair.
    let _install = if is_running {
        None
    } else {
        Some(state.install_lock.lock().await)
    };
    verify_game_files_impl(&state.paths, &target_version, is_running, &client, &handler).await
}

// ============================================================================
// Unit Tests (Strict Parity & Apple HIG Constraints)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_core::event::NoopEventHandler;
    use wex_core::pack::{Manifest, ManifestFileEntry};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_format_bytes_accuracy() {
        assert_eq!(format_bytes(0), "0 Б");
        assert_eq!(format_bytes(512), "512 Б");
        assert_eq!(format_bytes(1024), "1.0 КБ");
        assert_eq!(format_bytes(15 * 1024 * 1024), "15.0 МБ");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 ГБ");
    }

    #[test]
    fn test_protected_paths_check() {
        assert!(is_protected_path(Path::new(
            "C:/Wex/instances/1.21.1/saves/World1/level.dat"
        )));
        assert!(is_protected_path(Path::new(
            "C:/Wex/instances/1.21.1/screenshots/shot.png"
        )));
        assert!(is_protected_path(Path::new(
            "C:/Wex/instances/1.21.1/options.txt"
        )));
        assert!(is_protected_path(Path::new("C:/Wex/skins/steve.png")));
        assert!(is_protected_path(Path::new("C:/Wex/resourcepacks/wex.zip")));
        assert!(!is_protected_path(Path::new(
            "C:/Wex/instances/1.21.1/logs/2026-10-01-1.log.gz"
        )));
        assert!(!is_protected_path(Path::new(
            "C:/Wex/crash-reports/crash-2026-10-01.txt"
        )));
    }

    #[test]
    fn test_rejection_when_game_running() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let client = reqwest::Client::new();
        let events = NoopEventHandler;

        // 1. Cache clearing when game running must fail
        let clear_res = clear_storage_cache_impl(&paths, true);
        assert!(clear_res.is_err());
        assert!(clear_res.unwrap_err().contains("запущенной игры"));

        // 2. Verify files when game running must fail
        let rt = tokio::runtime::Runtime::new().unwrap();
        let verify_res = rt.block_on(verify_game_files_impl(
            &paths, "1.21.1", true, &client, &events,
        ));
        assert!(verify_res.is_err());
        assert!(verify_res.unwrap_err().contains("запущенной игры"));
    }

    #[test]
    fn test_cache_cleaning_preserves_saves_and_screenshots() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let _ = paths.ensure_dirs();

        let inst_dir = paths.instance_dir("1.21.1");
        let logs_dir = inst_dir.join("logs");
        let crash_dir = inst_dir.join("crash-reports");
        let temp_dir = paths.root.join("temp");
        let saves_dir = inst_dir.join("saves").join("SurvivalWorld");
        let screens_dir = inst_dir.join("screenshots");
        let skins_dir = paths.root.join("skins");

        fs::create_dir_all(&logs_dir).unwrap();
        fs::create_dir_all(&crash_dir).unwrap();
        fs::create_dir_all(&temp_dir).unwrap();
        fs::create_dir_all(&saves_dir).unwrap();
        fs::create_dir_all(&screens_dir).unwrap();
        fs::create_dir_all(&skins_dir).unwrap();

        // Files to be deleted (gz logs, temp files)
        let log_gz = logs_dir.join("old-session.log.gz");
        fs::write(&log_gz, b"compressed log data").unwrap();

        let temp_file = temp_dir.join("download.tmp");
        fs::write(&temp_file, b"temporary download chunks").unwrap();

        // Protected files that MUST NOT be touched
        let active_log = logs_dir.join("latest.log");
        fs::write(&active_log, b"active game log").unwrap();

        let world_level = saves_dir.join("level.dat");
        fs::write(&world_level, b"precious world save file").unwrap();

        let screenshot = screens_dir.join("epic_build.png");
        fs::write(&screenshot, b"png screenshot data").unwrap();

        let user_options = inst_dir.join("options.txt");
        fs::write(&user_options, b"fov:90\ngamma:1.0").unwrap();

        let user_skin = skins_dir.join("alex.png");
        fs::write(&user_skin, b"skin binary data").unwrap();

        // Check cache calculation before cleaning
        let info = get_storage_cache_info_impl(&paths);
        assert!(info.total_bytes > 0);
        assert!(info.file_count >= 2);

        // Execute cache clearing
        let result = clear_storage_cache_impl(&paths, false).unwrap();
        assert!(result.bytes_freed > 0);
        assert!(result.files_deleted >= 2);

        // Assert deleted files
        assert!(!log_gz.exists(), "Compressed log file must be removed");
        assert!(
            !temp_file.exists(),
            "Temporary download file must be removed"
        );

        // Assert strictly preserved files
        assert!(active_log.exists(), "latest.log must remain preserved");
        assert!(
            world_level.exists(),
            "World saves level.dat must be preserved!"
        );
        assert!(screenshot.exists(), "Screenshots must be preserved!");
        assert!(user_options.exists(), "options.txt must be preserved!");
        assert!(user_skin.exists(), "User skins must be preserved!");
    }

    #[tokio::test]
    async fn integrity_repair_counts_only_initially_corrupted_both_versions() {
        for version in ["1.20.1", "1.21.1"] {
            let tmp = tempdir().unwrap();
            let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
            let instance = paths.instance_dir(version);
            fs::create_dir_all(instance.join("mods")).unwrap();
            let server = MockServer::start().await;
            let content = b"verified content";
            fs::write(instance.join("mods/healthy.jar"), content).unwrap();
            let hash =
                compute_file_hash(&instance.join("mods/healthy.jar"), HashType::Sha256).unwrap();
            fs::write(instance.join("mods/broken.jar"), b"broken").unwrap();
            fs::write(instance.join("mods/disabled.jar.disabled"), content).unwrap();
            fs::write(instance.join("options.txt"), content).unwrap();
            Mock::given(method("GET"))
                .and(path("/broken"))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(content.to_vec()))
                .expect(1)
                .mount(&server)
                .await;
            let files = [
                "mods/broken.jar",
                "mods/healthy.jar",
                "mods/disabled.jar",
                "options.txt",
            ]
            .into_iter()
            .map(|path| ManifestFileEntry {
                path: path.to_string(),
                sha256: hash.clone(),
                size: content.len() as u64,
                mode: if path == "options.txt" {
                    "default"
                } else {
                    "sync"
                }
                .to_string(),
                url: Some(format!("{}/broken", server.uri())),
                side: None,
            })
            .collect();
            let manifest = Manifest {
                name: "test".to_string(),
                pack_version: "test".to_string(),
                minecraft: version.to_string(),
                fabric_loader: "0.19.5".to_string(),
                servers: None,
                files,
            };
            fs::write(
                instance.join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            let client = reqwest::Client::builder().no_proxy().build().unwrap();
            let result = verify_game_files_impl(&paths, version, false, &client, &NoopEventHandler)
                .await
                .unwrap();
            assert_eq!(result.total_files, 4);
            assert_eq!(result.checked_files, 4);
            assert_eq!(result.corrupted_files, 1);
            assert_eq!(result.repaired_files, 1);
            assert!(
                result.message.contains("Успешно восстановлено: 1 из 4"),
                "{version}: {}",
                result.message
            );
            assert_eq!(fs::read(instance.join("mods/broken.jar")).unwrap(), content);
            assert!(!instance.join("mods/disabled.jar").exists());
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn test_corrupted_file_redownloaded_and_repaired() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let inst_dir = paths.instance_dir("1.21.1");
        let mods_dir = inst_dir.join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        let mock_server = MockServer::start().await;
        let genuine_content = b"GENUINE_FABRIC_MOD_CONTENT_OK";
        let sample_file = inst_dir.join("sample.bin");
        fs::write(&sample_file, genuine_content).unwrap();
        let genuine_hash = compute_file_hash(&sample_file, HashType::Sha256).unwrap();
        let _ = fs::remove_file(&sample_file);

        Mock::given(method("GET"))
            .and(path("/release/testmod.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(genuine_content.to_vec()))
            .mount(&mock_server)
            .await;

        let corrupted_mod_path = mods_dir.join("testmod.jar");
        // Write intentionally corrupted content
        fs::write(&corrupted_mod_path, b"CORRUPTED_OR_TRUNCATED").unwrap();

        // Create local manifest pointing to mock server URL
        let manifest = Manifest {
            name: "Aura-Pack".to_string(),
            pack_version: "1.0.0".to_string(),
            minecraft: "1.21.1".to_string(),
            fabric_loader: "0.16.5".to_string(),
            servers: None,
            files: vec![ManifestFileEntry {
                path: "mods/testmod.jar".to_string(),
                sha256: genuine_hash.clone(),
                size: genuine_content.len() as u64,
                mode: "sync".to_string(),
                url: Some(format!("{}/release/testmod.jar", mock_server.uri())),
                side: None,
            }],
        };

        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        let local_manifest_file = inst_dir.join("manifest.json");
        fs::write(&local_manifest_file, manifest_json).unwrap();

        let client = reqwest::Client::new();
        let events = NoopEventHandler;

        // Run verification
        let result = verify_game_files_impl(&paths, "1.21.1", false, &client, &events)
            .await
            .unwrap();

        assert_eq!(result.total_files, 1);
        assert_eq!(result.corrupted_files, 1);
        assert_eq!(result.repaired_files, 1);
        assert_eq!(result.status, "Исправлено 1 файлов");

        // Check that file on disk is now repaired and genuine
        let disk_hash = compute_file_hash(&corrupted_mod_path, HashType::Sha256).unwrap();
        assert_eq!(
            disk_hash, genuine_hash,
            "File hash on disk must now match genuine manifest hash"
        );
    }

    #[tokio::test]
    async fn test_intact_files_report_all_good() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let inst_dir = paths.instance_dir("1.21.1");
        let mods_dir = inst_dir.join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        let content = b"PERFECT_INTACT_MOD_FILE";
        let sample_file = inst_dir.join("sample2.bin");
        fs::write(&sample_file, content).unwrap();
        let genuine_hash = compute_file_hash(&sample_file, HashType::Sha256).unwrap();
        let _ = fs::remove_file(&sample_file);

        let mod_path = mods_dir.join("intact.jar");
        fs::write(&mod_path, content).unwrap();

        let manifest = Manifest {
            name: "Aura-Pack".to_string(),
            pack_version: "1.0.0".to_string(),
            minecraft: "1.21.1".to_string(),
            fabric_loader: "0.16.5".to_string(),
            servers: None,
            files: vec![ManifestFileEntry {
                path: "mods/intact.jar".to_string(),
                sha256: genuine_hash,
                size: content.len() as u64,
                mode: "sync".to_string(),
                url: None,
                side: None,
            }],
        };

        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        let local_manifest_file = inst_dir.join("manifest.json");
        fs::write(&local_manifest_file, manifest_json).unwrap();

        let client = reqwest::Client::new();
        let events = NoopEventHandler;

        let result = verify_game_files_impl(&paths, "1.21.1", false, &client, &events)
            .await
            .unwrap();

        assert_eq!(result.total_files, 1);
        assert_eq!(result.corrupted_files, 0);
        assert_eq!(result.repaired_files, 0);
        assert_eq!(result.status, "Всё в порядке");
    }
}
