use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::commands::launch::PlayState;
use wex_core::config::WexPaths;

/// Metadata for a discovered log or crash report file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogFileInfo {
    pub file_name: String,
    pub relative_path: String,
    pub file_type: String, // "latest" | "crash_report" | "jvm_crash" | "archived"
    pub size_bytes: u64,
    pub modified_timestamp: i64,
    pub is_crash: bool,
}

/// A parsed entry / line from a log file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogEntry {
    pub line_number: usize,
    pub timestamp: Option<String>,
    pub level: String, // "ERROR" | "WARN" | "INFO" | "DEBUG" | "OTHER"
    pub thread: Option<String>,
    pub message: String,
}

/// Response returned when querying log contents.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LogReadResponse {
    pub file_name: String,
    pub total_lines: usize,
    pub returned_lines: usize,
    pub entries: Vec<LogEntry>,
    pub has_more: bool,
    pub error_count: usize,
    pub warn_count: usize,
}

pub const SUPPORTED_LOG_VERSIONS: &[&str] = &["1.20.1", "1.21.1"];

/// Resolves the instance directory for a specific supported Minecraft version.
pub fn validate_and_resolve_instance_dir(
    paths: &WexPaths,
    version: Option<&str>,
) -> Result<PathBuf, String> {
    let ver = match version {
        Some(v) => {
            let trimmed = v.trim();
            if !SUPPORTED_LOG_VERSIONS.contains(&trimmed) {
                return Err(format!(
                    "Неподдерживаемая версия игры: '{}'. Разрешённые версии: {:?}",
                    trimmed, SUPPORTED_LOG_VERSIONS
                ));
            }
            trimmed
        }
        None => "1.21.1",
    };
    Ok(paths.instance_dir(ver))
}

/// Validates and resolves the path to a requested log file inside the instance directory.
/// Strictly enforces that the file must reside in `inst_dir/logs/`, `inst_dir/crash-reports/`,
/// or be a JVM crash file `hs_err_pid*.log` in `inst_dir`.
/// Blocks any path traversal (`..`), null bytes, drive letters, absolute paths, or outside symlinks.
pub fn validate_and_resolve_log_path(inst_dir: &Path, file_name: &str) -> Result<PathBuf, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("Имя файла журнала не может быть пустым".to_string());
    }

    if trimmed.contains('\0') {
        return Err("Имя файла содержит недопустимый символ".to_string());
    }

    // Reject any relative traversal components
    let normalized = trimmed.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();
    for part in &parts {
        if *part == ".." || *part == "." {
            return Err("Попытка выхода за пределы каталога (обнаружен '..')".to_string());
        }
    }

    // Reject absolute paths and drive prefixes
    if trimmed.starts_with('/') || trimmed.starts_with('\\') || trimmed.contains(':') {
        return Err("Абсолютные пути к журналам запрещены".to_string());
    }

    // Determine target relative path based on allowed categories
    let clean_fname = normalized.trim_start_matches('/');

    let target_path = if let Some(sub) = clean_fname.strip_prefix("crash-reports/") {
        if sub.is_empty() || sub.contains('/') {
            return Err("Недопустимое имя файла в crash-reports".to_string());
        }
        if !sub.ends_with(".log") && !sub.ends_with(".txt") {
            return Err("Недопустимое расширение файла отчёта о сбое".to_string());
        }
        inst_dir.join("crash-reports").join(sub)
    } else if let Some(sub) = clean_fname.strip_prefix("logs/") {
        if sub.is_empty() || sub.contains('/') {
            return Err("Недопустимое имя файла в logs".to_string());
        }
        if !sub.ends_with(".log") && !sub.ends_with(".txt") {
            return Err("Недопустимое расширение файла журнала".to_string());
        }
        inst_dir.join("logs").join(sub)
    } else if clean_fname.starts_with("hs_err_pid") && clean_fname.ends_with(".log") {
        if clean_fname.contains('/') {
            return Err("Недопустимый путь к дампу JVM".to_string());
        }
        inst_dir.join(clean_fname)
    } else if !clean_fname.contains('/') {
        // Plain file name defaults to logs/
        if !clean_fname.ends_with(".log") && !clean_fname.ends_with(".txt") {
            return Err("Недопустимое расширение файла журнала".to_string());
        }
        inst_dir.join("logs").join(clean_fname)
    } else {
        return Err(format!(
            "Недопустимый путь к журналу: '{}'. Разрешены только файлы в logs/, crash-reports/ или hs_err_pid*.log",
            file_name
        ));
    };

    if !target_path.is_file() {
        return Err(format!("Файл журнала не найден: {}", file_name));
    }

    // Defense in depth: verify that canonical path starts with canonical inst_dir
    if let (Ok(canon_target), Ok(canon_inst)) =
        (target_path.canonicalize(), inst_dir.canonicalize())
    {
        if !canon_target.starts_with(&canon_inst) {
            return Err("Путь к файлу журнала выходит за пределы каталога экземпляра".to_string());
        }
    }

    Ok(target_path)
}

/// Fast parser for Minecraft log lines.
/// Handles standard format: `[HH:mm:ss] [Thread/LEVEL]: message`
/// Continuation lines (stack traces, etc.) retain the level of the previous line.
pub fn parse_log_line(raw_line: &str, line_number: usize, prev_level: &str) -> LogEntry {
    let trimmed = raw_line.trim_end();

    // Standard Minecraft format: [12:34:56] [Render thread/INFO]: Message
    if trimmed.starts_with('[') {
        if let Some(first_close) = trimmed.find(']') {
            let possible_time = &trimmed[1..first_close];
            let after_time = trimmed[first_close + 1..].trim_start();

            if after_time.starts_with('[') {
                if let Some(second_close) = after_time.find(']') {
                    let thread_level = &after_time[1..second_close];
                    let message = after_time[second_close + 1..].trim_start();
                    let message = message.strip_prefix(':').unwrap_or(message).trim_start();

                    let (thread, level) = if let Some(slash_idx) = thread_level.rfind('/') {
                        let th = &thread_level[..slash_idx];
                        let lvl = &thread_level[slash_idx + 1..];
                        (Some(th.to_string()), normalize_level(lvl))
                    } else {
                        (
                            Some(thread_level.to_string()),
                            normalize_level(thread_level),
                        )
                    };

                    return LogEntry {
                        line_number,
                        timestamp: Some(possible_time.to_string()),
                        level,
                        thread,
                        message: message.to_string(),
                    };
                }
            }
        }
    }

    // Check for explicit ERROR or WARN markers in raw text
    let left_trimmed = trimmed.trim_start();
    let level = if trimmed.contains("ERROR")
        || left_trimmed.starts_with("Exception in thread")
        || left_trimmed.starts_with("Caused by:")
        || trimmed.contains("FATAL")
    {
        "ERROR".to_string()
    } else if trimmed.contains("WARN") {
        "WARN".to_string()
    } else if left_trimmed.starts_with("at ") || left_trimmed.starts_with("...") {
        // Stack trace line inherits previous level if it was error or warn
        if prev_level == "ERROR" || prev_level == "WARN" {
            prev_level.to_string()
        } else {
            "ERROR".to_string()
        }
    } else {
        "INFO".to_string()
    };

    LogEntry {
        line_number,
        timestamp: None,
        thread: None,
        level,
        message: trimmed.to_string(),
    }
}

fn normalize_level(raw: &str) -> String {
    let upper = raw.trim().to_uppercase();
    match upper.as_str() {
        "ERROR" | "FATAL" | "SEVERE" => "ERROR".to_string(),
        "WARN" | "WARNING" => "WARN".to_string(),
        "INFO" => "INFO".to_string(),
        "DEBUG" | "TRACE" => "DEBUG".to_string(),
        _ => "OTHER".to_string(),
    }
}

/// Lists all log and crash-report files in the instance directory.
pub fn list_log_files_impl(paths: &WexPaths, version: Option<&str>) -> Vec<LogFileInfo> {
    let inst_dir = match validate_and_resolve_instance_dir(paths, version) {
        Ok(dir) => dir,
        Err(_) => return Vec::new(),
    };
    let mut files = Vec::new();

    // 1. logs/ directory (latest.log and others)
    let logs_dir = inst_dir.join("logs");
    if logs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !fname.ends_with(".log") && !fname.ends_with(".txt") {
                    continue;
                }

                let meta = entry.metadata().ok();
                let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified_timestamp = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let file_type = if fname == "latest.log" {
                    "latest"
                } else {
                    "archived"
                };

                files.push(LogFileInfo {
                    file_name: fname.to_string(),
                    relative_path: format!("logs/{}", fname),
                    file_type: file_type.to_string(),
                    size_bytes,
                    modified_timestamp,
                    is_crash: false,
                });
            }
        }
    }

    // 2. crash-reports/ directory
    let crash_dir = inst_dir.join("crash-reports");
    if crash_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !fname.ends_with(".txt") && !fname.ends_with(".log") {
                    continue;
                }

                let meta = entry.metadata().ok();
                let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified_timestamp = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                files.push(LogFileInfo {
                    file_name: fname.to_string(),
                    relative_path: format!("crash-reports/{}", fname),
                    file_type: "crash_report".to_string(),
                    size_bytes,
                    modified_timestamp,
                    is_crash: true,
                });
            }
        }
    }

    // 3. JVM crashes in instance root (hs_err_pid*.log)
    if let Ok(entries) = fs::read_dir(&inst_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.starts_with("hs_err_pid") && fname.ends_with(".log") {
                let meta = entry.metadata().ok();
                let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified_timestamp = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                files.push(LogFileInfo {
                    file_name: fname.to_string(),
                    relative_path: fname.to_string(),
                    file_type: "jvm_crash".to_string(),
                    size_bytes,
                    modified_timestamp,
                    is_crash: true,
                });
            }
        }
    }

    // Sorting:
    // 1. "latest.log" always first
    // 2. crash reports sorted by modification time descending
    // 3. other logs by modification time descending
    files.sort_by(|a, b| {
        if a.file_type == "latest" {
            return std::cmp::Ordering::Less;
        }
        if b.file_type == "latest" {
            return std::cmp::Ordering::Greater;
        }
        if a.is_crash != b.is_crash {
            return b.is_crash.cmp(&a.is_crash);
        }
        b.modified_timestamp.cmp(&a.modified_timestamp)
    });

    files
}

/// Reads a log file efficiently, parsing and applying search query and level filter.
/// Memory usage is strictly bounded by `max_lines` (default 2500, max 10000) using a ring buffer.
pub fn read_log_file_impl(
    paths: &WexPaths,
    version: Option<&str>,
    file_name: &str,
    max_lines: Option<usize>,
    filter_level: Option<&str>,
    search_query: Option<&str>,
) -> Result<LogReadResponse, String> {
    let inst_dir = validate_and_resolve_instance_dir(paths, version)?;
    let target_path = validate_and_resolve_log_path(&inst_dir, file_name)?;

    let file =
        File::open(&target_path).map_err(|e| format!("Не удалось открыть файл журнала: {}", e))?;
    let mut reader = BufReader::new(file);

    // Limit maximum lines in memory to keep UI responsive and bounded
    let line_limit = max_lines.unwrap_or(2500).min(10000);
    let filter_lvl = filter_level
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s != "ALL");
    let query_lower = search_query
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    let mut entries_ring: VecDeque<LogEntry> = VecDeque::with_capacity(line_limit);
    let mut total_matched = 0;
    let mut line_buf = String::new();
    let mut line_number = 0;
    let mut prev_level = "INFO".to_string();

    let mut error_count = 0;
    let mut warn_count = 0;

    while reader.read_line(&mut line_buf).map_err(|e| e.to_string())? > 0 {
        line_number += 1;
        let entry = parse_log_line(&line_buf, line_number, &prev_level);
        prev_level = entry.level.clone();

        if entry.level == "ERROR" {
            error_count += 1;
        } else if entry.level == "WARN" {
            warn_count += 1;
        }

        // Apply filters
        let matches_level = match &filter_lvl {
            Some(f) => entry.level == *f,
            None => true,
        };

        let matches_query = match &query_lower {
            Some(q) => {
                entry.message.to_lowercase().contains(q)
                    || entry
                        .thread
                        .as_ref()
                        .map(|t| t.to_lowercase().contains(q))
                        .unwrap_or(false)
            }
            None => true,
        };

        if matches_level && matches_query {
            total_matched += 1;
            if entries_ring.len() == line_limit {
                entries_ring.pop_front();
            }
            entries_ring.push_back(entry);
        }

        line_buf.clear();
    }

    let has_more = total_matched > line_limit;
    let returned_entries: Vec<LogEntry> = entries_ring.into();
    let returned_count = returned_entries.len();

    Ok(LogReadResponse {
        file_name: file_name.to_string(),
        total_lines: line_number,
        returned_lines: returned_count,
        entries: returned_entries,
        has_more,
        error_count,
        warn_count,
    })
}

/// Trait for opening directories in the system file manager.
pub trait FolderOpener: Send + Sync {
    fn open_folder(&self, dir: &Path) -> Result<(), String>;
}

/// Production folder opener for OS file explorer.
pub struct SystemFolderOpener;

impl FolderOpener for SystemFolderOpener {
    fn open_folder(&self, dir: &Path) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(dir)
                .spawn()
                .map_err(|e| format!("Не удалось открыть проводник: {}", e))?;
            Ok(())
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(dir)
                .spawn()
                .map_err(|e| format!("Не удалось открыть Finder: {}", e))?;
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            let dir = dir.to_path_buf();
            std::process::Command::new("xdg-open")
                .arg(&dir)
                .spawn()
                .map_err(|e| format!("Не удалось открыть файловый менеджер: {}", e))?;
            Ok(())
        }
    }
}

/// Mock folder opener for automated unit and integration tests (prevents popping explorer windows).
#[derive(Default, Clone)]
pub struct MockFolderOpener {
    pub opened_folders: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl FolderOpener for MockFolderOpener {
    fn open_folder(&self, dir: &Path) -> Result<(), String> {
        if let Ok(mut lock) = self.opened_folders.lock() {
            lock.push(dir.to_path_buf());
        }
        Ok(())
    }
}

/// Opens the specified folder using the injected opener.
pub fn open_logs_folder_with_opener<O: FolderOpener>(
    paths: &WexPaths,
    version: Option<&str>,
    folder_type: Option<&str>,
    opener: Option<&O>,
) -> Result<String, String> {
    let inst_dir = validate_and_resolve_instance_dir(paths, version)?;
    let target_dir = match folder_type {
        Some("crash_reports") | Some("crash-reports") => inst_dir.join("crash-reports"),
        _ => inst_dir.join("logs"),
    };

    let _ = fs::create_dir_all(&target_dir);

    if let Some(op) = opener {
        let _ = op.open_folder(&target_dir);
    }

    Ok(target_dir.to_string_lossy().to_string())
}

/// Opens the specified folder. In tests, does NOT spawn explorer.
pub fn open_logs_folder_impl(
    paths: &WexPaths,
    version: Option<&str>,
    folder_type: Option<&str>,
) -> Result<String, String> {
    open_logs_folder_with_opener::<MockFolderOpener>(paths, version, folder_type, None)
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Returns list of log files available for the specified version.
#[tauri::command]
pub async fn logs_list_files(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<LogFileInfo>, String> {
    Ok(list_log_files_impl(&state.paths, version.as_deref()))
}

/// Reads lines from the requested log file with streaming parsing and search filtering.
#[tauri::command]
pub async fn logs_read_file(
    file_name: String,
    version: Option<String>,
    max_lines: Option<usize>,
    filter_level: Option<String>,
    search_query: Option<String>,
    state: State<'_, PlayState>,
) -> Result<LogReadResponse, String> {
    read_log_file_impl(
        &state.paths,
        version.as_deref(),
        &file_name,
        max_lines,
        filter_level.as_deref(),
        search_query.as_deref(),
    )
}

/// Opens the logs or crash-reports directory in the system file manager.
#[tauri::command]
pub async fn logs_open_folder(
    version: Option<String>,
    folder_type: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    open_logs_folder_with_opener(
        &state.paths,
        version.as_deref(),
        folder_type.as_deref(),
        Some(&SystemFolderOpener),
    )
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_minecraft_log_lines() {
        let line1 = "[14:20:11] [Render thread/INFO]: [FabricLoader] Loading 142 mods...";
        let entry1 = parse_log_line(line1, 1, "INFO");
        assert_eq!(entry1.level, "INFO");
        assert_eq!(entry1.timestamp, Some("14:20:11".to_string()));
        assert_eq!(entry1.thread, Some("Render thread".to_string()));
        assert!(entry1.message.contains("Loading 142 mods"));

        let line2 = "[14:20:12] [main/WARN]: Mod 'optifine' is outdated";
        let entry2 = parse_log_line(line2, 2, "INFO");
        assert_eq!(entry2.level, "WARN");
        assert_eq!(entry2.thread, Some("main".to_string()));

        let line3 = "[14:20:15] [Render thread/ERROR]: Failed to load world chunk";
        let entry3 = parse_log_line(line3, 3, "INFO");
        assert_eq!(entry3.level, "ERROR");

        // Stack trace line inherits previous ERROR level
        let line4 = "    at net.minecraft.world.Chunk.load(Chunk.java:42)";
        let entry4 = parse_log_line(line4, 4, "ERROR");
        assert_eq!(entry4.level, "ERROR");
    }

    #[test]
    fn test_list_log_files_and_order() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let inst = paths.instance_dir("1.21.1");
        let logs_dir = inst.join("logs");
        let crash_dir = inst.join("crash-reports");
        fs::create_dir_all(&logs_dir).unwrap();
        fs::create_dir_all(&crash_dir).unwrap();

        fs::write(logs_dir.join("latest.log"), b"Latest log content").unwrap();
        fs::write(logs_dir.join("2026-11-07-1.log"), b"Old log content").unwrap();
        fs::write(
            crash_dir.join("crash-2026-11-08_14.00.00-client.txt"),
            b"Crash report",
        )
        .unwrap();

        let files = list_log_files_impl(&paths, Some("1.21.1"));
        assert_eq!(files.len(), 3);

        // latest.log must always be first
        assert_eq!(files[0].file_name, "latest.log");
        assert_eq!(files[0].file_type, "latest");
        assert!(!files[0].is_crash);

        // Crash report must have is_crash: true
        let crash_item = files.iter().find(|f| f.file_name.starts_with("crash-"));
        assert!(crash_item.is_some());
        assert!(crash_item.unwrap().is_crash);
    }

    #[test]
    fn test_read_log_file_with_search_and_level_filter() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let inst = paths.instance_dir("1.21.1");
        let logs_dir = inst.join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        let log_content = "\
[12:00:00] [main/INFO]: Initializing Minecraft
[12:00:01] [main/WARN]: Slow disk read detected
[12:00:02] [main/ERROR]: Failed to connect to telemetry server
[12:00:03] [main/INFO]: Game session ready
";
        fs::write(logs_dir.join("latest.log"), log_content.as_bytes()).unwrap();

        // 1. Read all lines
        let res_all =
            read_log_file_impl(&paths, Some("1.21.1"), "latest.log", None, None, None).unwrap();
        assert_eq!(res_all.total_lines, 4);
        assert_eq!(res_all.returned_lines, 4);
        assert_eq!(res_all.error_count, 1);
        assert_eq!(res_all.warn_count, 1);

        // 2. Filter by ERROR level
        let res_err = read_log_file_impl(
            &paths,
            Some("1.21.1"),
            "latest.log",
            None,
            Some("ERROR"),
            None,
        )
        .unwrap();
        assert_eq!(res_err.returned_lines, 1);
        assert!(res_err.entries[0].message.contains("telemetry"));

        // 3. Search query
        let res_search = read_log_file_impl(
            &paths,
            Some("1.21.1"),
            "latest.log",
            None,
            None,
            Some("disk"),
        )
        .unwrap();
        assert_eq!(res_search.returned_lines, 1);
        assert_eq!(res_search.entries[0].level, "WARN");
    }

    #[test]
    fn test_logs_read_file_path_traversal_rejected() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let inst = paths.instance_dir("1.21.1");
        let logs_dir = inst.join("logs");
        fs::create_dir_all(&logs_dir).unwrap();
        fs::write(logs_dir.join("latest.log"), b"Safe log content").unwrap();

        // 1. Relative traversal with ..
        let err1 = read_log_file_impl(&paths, Some("1.21.1"), "../../Cargo.toml", None, None, None);
        assert!(err1.is_err(), "Must reject path traversal with ..");
        assert!(err1.unwrap_err().contains(".."));

        // 2. Traversal embedded in logs/
        let err2 = read_log_file_impl(
            &paths,
            Some("1.21.1"),
            "logs/../../Cargo.toml",
            None,
            None,
            None,
        );
        assert!(err2.is_err(), "Must reject embedded traversal with ..");

        // 3. Absolute path / drive letter
        let err3 = read_log_file_impl(
            &paths,
            Some("1.21.1"),
            "C:\\Windows\\System32\\drivers\\etc\\hosts",
            None,
            None,
            None,
        );
        assert!(err3.is_err(), "Must reject absolute path");

        // 4. Accessing non-log file in instance dir (e.g. options.txt)
        let err4 = read_log_file_impl(&paths, Some("1.21.1"), "options.txt", None, None, None);
        assert!(
            err4.is_err(),
            "Must reject non-log files outside logs/crash-reports"
        );
    }

    #[test]
    fn test_logs_read_file_unsupported_version_rejected() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));

        // 1. Unsupported version
        let err1 = read_log_file_impl(&paths, Some("1.12.2"), "latest.log", None, None, None);
        assert!(err1.is_err(), "Must reject unsupported version");
        assert!(err1.unwrap_err().contains("Неподдерживаемая версия"));

        // 2. Traversal in version parameter
        let err2 = read_log_file_impl(&paths, Some("../../etc"), "latest.log", None, None, None);
        assert!(err2.is_err(), "Must reject traversal in version");

        // 3. list_log_files returns empty for unsupported version
        let list = list_log_files_impl(&paths, Some("1.16.5"));
        assert!(
            list.is_empty(),
            "Must return empty list for unsupported version"
        );
    }

    #[test]
    fn test_logs_read_file_max_lines_bounded_memory() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let inst = paths.instance_dir("1.21.1");
        let logs_dir = inst.join("logs");
        fs::create_dir_all(&logs_dir).unwrap();

        // Generate 5000 log lines
        let mut content = String::with_capacity(300_000);
        for i in 1..=5000 {
            content.push_str(&format!(
                "[12:00:{:02}] [Render thread/INFO]: Event step {}\n",
                i % 60,
                i
            ));
        }
        fs::write(logs_dir.join("latest.log"), content.as_bytes()).unwrap();

        // Read with max_lines = 20
        let res = read_log_file_impl(&paths, Some("1.21.1"), "latest.log", Some(20), None, None)
            .expect("read should succeed");

        assert_eq!(res.total_lines, 5000);
        assert_eq!(res.returned_lines, 20);
        assert_eq!(res.entries.len(), 20);
        assert!(res.has_more, "has_more must be true when 5000 > 20");

        // Verify it returned the tail (last 20 lines: 4981..5000)
        assert_eq!(res.entries[0].line_number, 4981);
        assert_eq!(res.entries[19].line_number, 5000);
        assert!(res.entries[19].message.contains("Event step 5000"));
    }

    #[test]
    fn test_open_logs_folder_with_mock_opener() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let mock = MockFolderOpener::default();

        let res = open_logs_folder_with_opener(
            &paths,
            Some("1.21.1"),
            Some("crash_reports"),
            Some(&mock),
        )
        .expect("open should succeed");

        assert!(res.contains("crash-reports"));
        let opened = mock.opened_folders.lock().unwrap();
        assert_eq!(opened.len(), 1);
        assert!(opened[0].ends_with("crash-reports"));
    }
}
