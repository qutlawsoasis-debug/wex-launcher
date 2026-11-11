use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tauri::State;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use wex_core::config::{LauncherConfig as CoreConfig, WexPaths};
use wex_extras::crash::analyze_crash;

/// Anonymizes personal user paths, player nicknames, secret tokens, and friends' IP addresses in text.
pub fn anonymize_text(text: &str, player_name: Option<&str>) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut result = text.to_string();

    // 1. Mask player nickname if provided and not generic
    if let Some(nick) = player_name {
        let trimmed_nick = nick.trim();
        if !trimmed_nick.is_empty()
            && trimmed_nick.to_lowercase() != "player"
            && trimmed_nick.to_lowercase() != "steve"
            && trimmed_nick.to_lowercase() != "alex"
        {
            result = result.replace(trimmed_nick, "<player>");
        }
    }

    // 2. Mask user paths (e.g. C:\Users\Username\... or /home/username/...)
    result = mask_user_directories(&result);

    // 3. Mask secret tokens / headers / CLI args
    result = mask_sensitive_tokens(&result);

    // 4. Mask friends' IPs, LAN endpoints, and playit tunnel hosts
    result = mask_ip_addresses(&result);

    result
}

/// Masks directory paths containing personal usernames without corrupting UTF-8 / non-ASCII characters.
fn mask_user_directories(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut remainder = text;

    // Support both single backslash and JSON-escaped double backslash, and forward slashes
    let markers = [
        "\\\\Users\\\\",
        "\\\\users\\\\",
        "\\Users\\",
        "\\users\\",
        "/Users/",
        "/users/",
        "/home/",
    ];

    while !remainder.is_empty() {
        let mut earliest: Option<(usize, &str)> = None;
        for &marker in &markers {
            if let Some(pos) = remainder.find(marker) {
                if earliest.is_none_or(|(min_pos, _)| pos < min_pos) {
                    earliest = Some((pos, marker));
                }
            }
        }

        match earliest {
            Some((pos, marker)) => {
                output.push_str(&remainder[..pos]);
                output.push_str(marker);
                let after_marker = &remainder[pos + marker.len()..];

                // If already masked (e.g. repeated anonymization passes), preserve *** and advance
                let username_end = if after_marker.starts_with("***") {
                    3
                } else {
                    // Look for path separator (\ or /) before quotes, newlines, or delimiters
                    let next_sep = after_marker.find(['\\', '/']);
                    let next_delim = after_marker.find(['"', '\'', '\n', '\r', '\t', ';', ':']);

                    match (next_sep, next_delim) {
                        (Some(sep), Some(delim)) if sep < delim && sep <= 64 => sep,
                        (Some(sep), None) if sep <= 64 => sep,
                        (_, Some(delim)) => delim,
                        _ => after_marker
                            .find([' ', '\n', '\r', '\t'])
                            .unwrap_or(after_marker.len()),
                    }
                };

                if username_end > 0 {
                    output.push_str("***");
                }
                remainder = &after_marker[username_end..];
            }
            None => {
                output.push_str(remainder);
                break;
            }
        }
    }

    // Additional check for active Windows/Unix user environment variables
    for env_key in &["USERNAME", "USER"] {
        if let Ok(uname) = std::env::var(env_key) {
            let trimmed = uname.trim();
            if trimmed.len() >= 3
                && trimmed.to_lowercase() != "root"
                && trimmed.to_lowercase() != "default"
            {
                output = output.replace(&format!("\\{}", trimmed), "\\***");
                output = output.replace(&format!("/{}", trimmed), "/***");
                output = output.replace(&format!("\\\\{}", trimmed), "\\\\***");
                output = output.replace(&format!("\"{}\"", trimmed), "\"***\"");
                output = output.replace(&format!("'{}'", trimmed), "'***'");
                output = output.replace(&format!(" {} ", trimmed), " *** ");
            }
        }
    }

    output
}

/// Masks sensitive tokens, passwords, and API keys.
fn mask_sensitive_tokens(text: &str) -> String {
    let mut res = text.to_string();

    // 1. CLI arguments: --accessToken <token>
    let cli_token_re =
        Regex::new(r"--(?:accessToken|userProperties)\s+[^\s]+").expect("valid regex");
    res = cli_token_re
        .replace_all(&res, "--accessToken ***")
        .to_string();

    // 2. Query parameters: ?token=xxx or &token=xxx, ?secret=xxx, ?key=xxx
    let query_re =
        Regex::new(r#"(?i)([?&](?:token|access_token|secret|key|api_key|password)=)[^&\s\r\n"']+"#)
            .expect("valid regex");
    res = query_re.replace_all(&res, "${1}***").to_string();

    // 3. JSON key-value pairs with double-quoted strings (handling spaces in secret values, and Bearer tokens):
    let json_token_re = Regex::new(
        r#"(?i)(["'](?:x-user-token|usertoken|ownertoken|hosttoken|playitsecret|discordappid|accesstoken|access_token|refreshtoken|refresh_token|authtoken|auth_token|sessiontoken|session_token|secret_key|client_secret|clientsecret|apikey|api_key|api-key|private_key|key|token|secret|password|passwd|credential|auth|authorization)["']\s*:\s*")((?:Bearer\s+)?[^"\r\n]+)(")"#,
    ).expect("valid regex");
    res = json_token_re
        .replace_all(&res, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let val = &caps[2];
            let suffix = &caps[3];
            if val.to_lowercase().starts_with("bearer ") {
                format!("{}Bearer ***{}", prefix, suffix)
            } else {
                format!("{}***{}", prefix, suffix)
            }
        })
        .to_string();

    // 4. Single-quoted values in JSON/JS:
    let json_sq_token_re = Regex::new(
        r#"(?i)(['](?:x-user-token|usertoken|ownertoken|hosttoken|playitsecret|discordappid|accesstoken|access_token|refreshtoken|refresh_token|authtoken|auth_token|sessiontoken|session_token|secret_key|client_secret|clientsecret|apikey|api_key|api-key|private_key|key|token|secret|password|passwd|credential|auth|authorization)[']\s*:\s*')((?:Bearer\s+)?[^'\r\n]+)(')"#,
    ).expect("valid regex");
    res = json_sq_token_re
        .replace_all(&res, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let val = &caps[2];
            let suffix = &caps[3];
            if val.to_lowercase().starts_with("bearer ") {
                format!("{}Bearer ***{}", prefix, suffix)
            } else {
                format!("{}***{}", prefix, suffix)
            }
        })
        .to_string();

    // 5. Key-value pairs anywhere in text or CLI/headers (unquoted values, yaml, config):
    let kv_token_re = Regex::new(
        r#"(?i)(\b(?:x-user-token|usertoken|ownertoken|hosttoken|playitsecret|discordappid|accesstoken|access_token|refreshtoken|refresh_token|authtoken|auth_token|sessiontoken|session_token|secret_key|client_secret|clientsecret|apikey|api_key|api-key|private_key|key|token|secret|password|passwd|credential|auth|authorization)\s*[:=]\s*["']?)((?:Bearer\s+)?[^"',\s\r\n\}]+)(["']?)"#,
    ).expect("valid regex");
    res = kv_token_re
        .replace_all(&res, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let val = &caps[2];
            let suffix = &caps[3];
            if val.to_lowercase().starts_with("bearer ") {
                format!("{}Bearer ***{}", prefix, suffix)
            } else {
                format!("{}***{}", prefix, suffix)
            }
        })
        .to_string();

    // 6. Bearer tokens anywhere in free text: Bearer <token>
    let bearer_re = Regex::new(r"(?i)\bBearer\s+[A-Za-z0-9_\-\.]+").expect("valid regex");
    res = bearer_re.replace_all(&res, "Bearer ***").to_string();

    res
}

/// Masks IP addresses, tunnel hosts, and friend connection endpoints.
fn mask_ip_addresses(text: &str) -> String {
    // 1. Mask playit tunnels: e.g. xxx.auto.playit.gg:12345 or xxx.playit.gg
    let tunnel_re =
        Regex::new(r"(?i)[a-z0-9-]+\.(?:auto\.)?playit\.gg(?::\d+)?").expect("valid regex");
    let res = tunnel_re.replace_all(text, "[tunnel]");

    // 2. Mask joinmc links: e.g. xxx.joinmc.link
    let joinmc_re = Regex::new(r"(?i)[a-z0-9-]+\.joinmc\.link(?::\d+)?").expect("valid regex");
    let res = joinmc_re.replace_all(&res, "[tunnel]");

    // 3. Mask IPv4 addresses with optional port: e.g. 192.168.1.50:25565 or 147.185.221.16
    // Use word boundaries and check that it's 4 octets. Note that Minecraft versions (e.g. 1.21.1) only have 3 octets.
    let ip_re = Regex::new(
        r"\b(?:(?:25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])(?::\d{1,5})?\b",
    ).expect("valid regex");

    let mut output = String::with_capacity(res.len());
    let mut last_end = 0;

    for m in ip_re.find_iter(&res) {
        let start = m.start();
        let end = m.end();

        // Check prefix in res before start: if preceded by "jdk-", "java-", "version ", skip
        let prefix = if start >= 8 {
            &res[start - 8..start]
        } else {
            &res[..start]
        };
        let lower_prefix = prefix.to_lowercase();
        if lower_prefix.ends_with("jdk-")
            || lower_prefix.ends_with("java-")
            || lower_prefix.ends_with("version ")
            || lower_prefix.ends_with("v")
        {
            output.push_str(&res[last_end..end]);
            last_end = end;
            continue;
        }

        output.push_str(&res[last_end..start]);
        output.push_str("[IP]");
        last_end = end;
    }

    output.push_str(&res[last_end..]);

    // 4. Mask IPv6 addresses (full or compressed): e.g. 2001:0db8:85a3:0000:0000:8a2e:0370:7334 or fe80::1ff
    let ipv6_re = Regex::new(
        r"(?i)\b(?:[0-9a-f]{1,4}:){3,7}[0-9a-f]{1,4}\b|\b(?:[0-9a-f]{1,4}:){1,6}:[0-9a-f]{1,4}\b|::1\b|\bfe80::[0-9a-f:]{1,16}\b",
    ).expect("valid regex");
    output = ipv6_re.replace_all(&output, "[IP]").to_string();

    output
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportModDto {
    pub filename: String,
    pub size_bytes: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReportPreviewDto {
    pub launcher_version: String,
    pub game_version: String,
    pub os_info: String,
    pub java_version: String,
    pub memory_info: String,
    pub diagnosis_title: Option<String>,
    pub diagnosis_advice: Option<String>,
    pub error_text: Option<String>,
    pub user_comment: Option<String>,
    pub mods: Vec<ReportModDto>,
    pub mods_count: usize,
    pub latest_log_tail: Option<String>,
    pub crash_report_name: Option<String>,
    pub crash_report_tail: Option<String>,
    pub full_json_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveReportResultDto {
    pub success: bool,
    pub zip_path: String,
    pub file_name: String,
    pub file_size_bytes: u64,
}

/// Resolves the instance directory for a specific Minecraft version.
fn resolve_instance_dir(paths: &WexPaths, version: Option<&str>) -> PathBuf {
    let ver = version.unwrap_or("1.21.1");
    paths.instance_dir(ver)
}

/// Scans installed mods in the instance mods directory.
pub fn scan_installed_mods(inst_dir: &Path) -> Vec<ReportModDto> {
    let mut mods = Vec::new();
    let mods_dir = inst_dir.join("mods");

    if mods_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(mods_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                    if fname.ends_with(".jar") || fname.ends_with(".disabled") {
                        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        let enabled = !fname.ends_with(".disabled");
                        mods.push(ReportModDto {
                            filename: fname.to_string(),
                            size_bytes,
                            enabled,
                        });
                    }
                }
            }
        }
    }

    mods.sort_by(|a, b| a.filename.cmp(&b.filename));
    mods
}

/// Reads the last N lines from a log file.
pub fn read_log_tail(file_path: &Path, max_lines: usize) -> Option<String> {
    if !file_path.is_file() {
        return None;
    }

    let file = File::open(file_path).ok()?;
    let reader = BufReader::new(file);

    let mut ring: Vec<String> = Vec::with_capacity(max_lines);
    for line in reader.lines().map_while(Result::ok) {
        if ring.len() == max_lines {
            ring.remove(0);
        }
        ring.push(line);
    }

    if ring.is_empty() {
        None
    } else {
        Some(ring.join("\n"))
    }
}

/// Finds the most recent crash report in the instance crash-reports directory.
pub fn find_latest_crash_report(inst_dir: &Path) -> Option<(String, String)> {
    let crash_dir = inst_dir.join("crash-reports");
    if !crash_dir.is_dir() {
        return None;
    }

    let entries = fs::read_dir(&crash_dir).ok()?;
    let mut reports: Vec<_> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("txt"))
                .unwrap_or(false)
        })
        .collect();

    reports.sort_by_key(|e| {
        e.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });

    if let Some(latest) = reports.last() {
        let fname = latest.file_name().to_string_lossy().to_string();
        let content = fs::read_to_string(latest.path()).unwrap_or_default();
        Some((fname, content))
    } else {
        None
    }
}

/// Helper to get basic OS string.
fn get_os_info() -> String {
    #[cfg(target_os = "windows")]
    {
        "Windows (64-bit)".to_string()
    }
    #[cfg(target_os = "macos")]
    {
        "macOS (Apple Silicon / Intel)".to_string()
    }
    #[cfg(target_os = "linux")]
    {
        "Linux (x86_64)".to_string()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "Unknown OS".to_string()
    }
}

/// Resolves Desktop folder on Windows or fallback.
fn resolve_desktop_dir(paths: &WexPaths) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let desktop = PathBuf::from(userprofile).join("Desktop");
            if desktop.is_dir() {
                return desktop;
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let desktop = PathBuf::from(home).join("Desktop");
            if desktop.is_dir() {
                return desktop;
            }
        }
    }

    let fallback = paths.root.join("reports");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// Prepares the full report preview DTO.
pub fn prepare_report_preview_impl(
    paths: &WexPaths,
    version: Option<&str>,
    error_text: Option<&str>,
    user_comment: Option<&str>,
    player_name: Option<&str>,
) -> BugReportPreviewDto {
    let ver = version.unwrap_or("1.21.1");
    let inst_dir = resolve_instance_dir(paths, Some(ver));

    // 1. Diagnosis
    let diagnosis = analyze_crash(&inst_dir, Some(-1));
    let diagnosis_title = Some(anonymize_text(&diagnosis.title, player_name));
    let diagnosis_advice = Some(anonymize_text(&diagnosis.user_advice, player_name));

    // 2. Logs
    let latest_log_path = inst_dir.join("logs").join("latest.log");
    let raw_log_tail = read_log_tail(&latest_log_path, 1500);
    let latest_log_tail = raw_log_tail.map(|l| anonymize_text(&l, player_name));

    // 3. Crash report
    let crash_report_pair = find_latest_crash_report(&inst_dir);
    let (crash_report_name, crash_report_tail) = match crash_report_pair {
        Some((name, content)) => (Some(name), Some(anonymize_text(&content, player_name))),
        None => (None, None),
    };

    // 4. Mods
    let mods = scan_installed_mods(&inst_dir);
    let mods_count = mods.len();

    // 5. System info
    let os_info = get_os_info();
    let cfg = CoreConfig::load_or_default(&paths.config_file);

    let memory_info = if cfg.max_memory_mb > 0 {
        format!("{} MB", cfg.max_memory_mb)
    } else {
        "Не указано".to_string()
    };

    let java_version = if cfg.java_mode == "custom" {
        if let Some(custom_path) = &cfg.custom_java_path {
            let path_str = custom_path.to_string_lossy();
            let masked = anonymize_text(&path_str, player_name);
            format!("Пользовательская ({})", masked)
        } else {
            "Пользовательская (не указана)".to_string()
        }
    } else {
        let is_17 = ver.starts_with("1.20");
        let exe_name = if cfg!(target_os = "windows") {
            "javaw.exe"
        } else {
            "java"
        };
        let mojang_dir_name = if is_17 {
            "java-runtime-gamma"
        } else {
            "java-runtime-delta"
        };
        let mojang_bin = paths
            .runtime_dir
            .join(mojang_dir_name)
            .join("bin")
            .join(exe_name);
        let java_tag = if is_17 { "Java 17" } else { "Java 21" };

        if mojang_bin.exists() {
            format!("{} (Mojang Runtime)", java_tag)
        } else {
            format!("{} (автозагрузка / не установлена)", java_tag)
        }
    };

    let anon_error = error_text.map(|e| anonymize_text(e, player_name));
    let anon_comment = user_comment.map(|c| anonymize_text(c, player_name));

    #[derive(Serialize)]
    struct FullExportJson<'a> {
        launcher: &'a str,
        launcher_version: &'a str,
        game_version: &'a str,
        os: &'a str,
        java: &'a str,
        memory: &'a str,
        diagnosis_title: Option<&'a str>,
        diagnosis_advice: Option<&'a str>,
        error_details: Option<&'a str>,
        user_comment: Option<&'a str>,
        mods_count: usize,
        mods: &'a [ReportModDto],
        has_crash_report: bool,
    }

    let export_obj = FullExportJson {
        launcher: "Wex Launcher",
        launcher_version: env!("CARGO_PKG_VERSION"),
        game_version: ver,
        os: &os_info,
        java: &java_version,
        memory: &memory_info,
        diagnosis_title: diagnosis_title.as_deref(),
        diagnosis_advice: diagnosis_advice.as_deref(),
        error_details: anon_error.as_deref(),
        user_comment: anon_comment.as_deref(),
        mods_count,
        mods: &mods,
        has_crash_report: crash_report_tail.is_some(),
    };

    let full_json_preview = serde_json::to_string_pretty(&export_obj).unwrap_or_default();

    BugReportPreviewDto {
        launcher_version: env!("CARGO_PKG_VERSION").to_string(),
        game_version: ver.to_string(),
        os_info,
        java_version,
        memory_info,
        diagnosis_title,
        diagnosis_advice,
        error_text: anon_error,
        user_comment: anon_comment,
        mods,
        mods_count,
        latest_log_tail,
        crash_report_name,
        crash_report_tail,
        full_json_preview,
    }
}

/// Trait for revealing exported report files in the system file explorer.
pub trait PathOpener: Send + Sync {
    fn open_path(&self, path: &Path) -> Result<(), String>;
}

/// Production opener that reveals the file in the OS explorer on Windows (only when invoked by user action).
pub struct SystemExplorerOpener;

impl PathOpener for SystemExplorerOpener {
    fn open_path(&self, path: &Path) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer.exe")
                .arg(format!("/select,\"{}\"", path.display()))
                .spawn()
                .map_err(|e| format!("Не удалось открыть проводник: {}", e))?;
            Ok(())
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Не удалось открыть Finder: {}", e))?;
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            let dir = path.parent().unwrap_or(path);
            std::process::Command::new("xdg-open")
                .arg(dir)
                .spawn()
                .map_err(|e| format!("Не удалось открыть файловый менеджер: {}", e))?;
            Ok(())
        }
    }
}

/// Mock opener for automated unit and integration tests (records opened paths without spawning explorer).
#[derive(Default, Clone)]
pub struct MockPathOpener {
    pub opened_paths: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl PathOpener for MockPathOpener {
    fn open_path(&self, path: &Path) -> Result<(), String> {
        if let Ok(mut lock) = self.opened_paths.lock() {
            lock.push(path.to_path_buf());
        }
        Ok(())
    }
}

/// Generates a ZIP archive with an optional injected opener (to keep tests strictly isolated from explorer).
pub fn save_report_zip_with_opener<O: PathOpener>(
    paths: &WexPaths,
    version: Option<&str>,
    error_text: Option<&str>,
    user_comment: Option<&str>,
    player_name: Option<&str>,
    custom_dest_dir: Option<&str>,
    opener: Option<&O>,
) -> Result<SaveReportResultDto, String> {
    let preview =
        prepare_report_preview_impl(paths, version, error_text, user_comment, player_name);

    let dest_dir = if let Some(custom) = custom_dest_dir {
        let p = PathBuf::from(custom);
        if p.is_dir() {
            p
        } else {
            resolve_desktop_dir(paths)
        }
    } else {
        resolve_desktop_dir(paths)
    };

    let now = chrono::Local::now();
    let zip_name = format!("Wex-report-{}.zip", now.format("%Y%m%d-%H%M%S"));
    let zip_path = dest_dir.join(&zip_name);

    let file = File::create(&zip_path)
        .map_err(|e| format!("Не удалось создать ZIP-архив отчёта: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. report.json
    let anon_json = anonymize_text(&preview.full_json_preview, player_name);
    zip.start_file("report.json", options)
        .map_err(|e| format!("Ошибка добавления report.json в архив: {}", e))?;
    zip.write_all(anon_json.as_bytes())
        .map_err(|e| format!("Ошибка записи report.json: {}", e))?;

    // 2. system-info.txt
    let mut sys_info = String::new();
    sys_info.push_str("=== Wex Launcher Bug Report ===\n");
    sys_info.push_str(&format!("Launcher Version: {}\n", preview.launcher_version));
    sys_info.push_str(&format!("Minecraft Version: {}\n", preview.game_version));
    sys_info.push_str(&format!("OS: {}\n", preview.os_info));
    sys_info.push_str(&format!("Java: {}\n", preview.java_version));
    sys_info.push_str(&format!("Memory: {}\n", preview.memory_info));
    sys_info.push_str(&format!("Installed Mods: {}\n", preview.mods_count));
    sys_info.push_str(&format!("Timestamp: {}\n", now.format("%Y-%m-%d %H:%M:%S")));
    if let Some(comment) = &preview.user_comment {
        sys_info.push_str(&format!("\nUser Comment:\n{}\n", comment));
    }
    if let Some(err) = &preview.error_text {
        sys_info.push_str(&format!("\nError Details:\n{}\n", err));
    }

    let anon_sys_info = anonymize_text(&sys_info, player_name);
    zip.start_file("system-info.txt", options)
        .map_err(|e| format!("Ошибка добавления system-info.txt в архив: {}", e))?;
    zip.write_all(anon_sys_info.as_bytes())
        .map_err(|e| format!("Ошибка записи system-info.txt: {}", e))?;

    // 3. latest.log
    if let Some(log_content) = &preview.latest_log_tail {
        let anon_log = anonymize_text(log_content, player_name);
        zip.start_file("latest.log", options)
            .map_err(|e| format!("Ошибка добавления latest.log в архив: {}", e))?;
        zip.write_all(anon_log.as_bytes())
            .map_err(|e| format!("Ошибка записи latest.log: {}", e))?;
    }

    // 4. crash-report.txt
    if let Some(crash_content) = &preview.crash_report_tail {
        let anon_crash = anonymize_text(crash_content, player_name);
        let entry_name = preview
            .crash_report_name
            .as_deref()
            .unwrap_or("crash-report.txt");
        zip.start_file(entry_name, options)
            .map_err(|e| format!("Ошибка добавления отчёта о сбое в архив: {}", e))?;
        zip.write_all(anon_crash.as_bytes())
            .map_err(|e| format!("Ошибка записи отчёта о сбое: {}", e))?;
    }

    // 5. mods.txt
    if !preview.mods.is_empty() {
        let mut mods_text = String::new();
        mods_text.push_str("Installed Mods List:\n");
        for m in &preview.mods {
            let status = if m.enabled { "ENABLED" } else { "DISABLED" };
            mods_text.push_str(&format!(
                "- {} ({} bytes, {})\n",
                m.filename, m.size_bytes, status
            ));
        }
        let anon_mods = anonymize_text(&mods_text, player_name);
        zip.start_file("mods.txt", options)
            .map_err(|e| format!("Ошибка добавления mods.txt в архив: {}", e))?;
        zip.write_all(anon_mods.as_bytes())
            .map_err(|e| format!("Ошибка записи mods.txt: {}", e))?;
    }

    zip.finish()
        .map_err(|e| format!("Ошибка завершения ZIP-архива: {}", e))?;

    let file_size_bytes = fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);

    // Call injected opener if provided (never opens in tests by default)
    if let Some(op) = opener {
        let _ = op.open_path(&zip_path);
    }
    reveal_saved_report(&zip_path);

    Ok(SaveReportResultDto {
        success: true,
        zip_path: zip_path.to_string_lossy().to_string(),
        file_name: zip_name,
        file_size_bytes,
    })
}

#[cfg(test)]
thread_local! {
    static REVEALED_REPORT: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn reveal_saved_report(zip_path: &std::path::Path) {
    #[cfg(test)]
    REVEALED_REPORT.with(|path| *path.borrow_mut() = Some(zip_path.to_path_buf()));
    #[cfg(not(test))]
    let _ = zip_path;
}

/// Generates a ZIP archive containing all report files. In tests, does NOT spawn explorer.
pub fn save_report_zip_impl(
    paths: &WexPaths,
    version: Option<&str>,
    error_text: Option<&str>,
    user_comment: Option<&str>,
    player_name: Option<&str>,
    custom_dest_dir: Option<&str>,
) -> Result<SaveReportResultDto, String> {
    save_report_zip_with_opener::<MockPathOpener>(
        paths,
        version,
        error_text,
        user_comment,
        player_name,
        custom_dest_dir,
        None,
    )
}

/// Formats the entire report into clipboard-friendly Markdown text.
pub fn format_clipboard_report_impl(
    paths: &WexPaths,
    version: Option<&str>,
    error_text: Option<&str>,
    user_comment: Option<&str>,
    player_name: Option<&str>,
) -> String {
    let preview =
        prepare_report_preview_impl(paths, version, error_text, user_comment, player_name);

    let mut md = String::new();
    md.push_str("### Отчёт об ошибке Wex Launcher\n\n");
    md.push_str(&format!(
        "- **Версия лаунчера:** `{}`\n",
        preview.launcher_version
    ));
    md.push_str(&format!("- **Версия игры:** `{}`\n", preview.game_version));
    md.push_str(&format!("- **ОС:** {}\n", preview.os_info));
    md.push_str(&format!("- **Среда Java:** {}\n", preview.java_version));
    md.push_str(&format!(
        "- **Модов установлено:** {}\n",
        preview.mods_count
    ));

    if let Some(title) = &preview.diagnosis_title {
        md.push_str(&format!("- **Диагноз:** {}\n", title));
    }
    if let Some(advice) = &preview.diagnosis_advice {
        md.push_str(&format!("- **Рекомендация:** {}\n", advice));
    }
    if let Some(comment) = &preview.user_comment {
        md.push_str(&format!(
            "\n**Комментарий игрока:**\n> {}\n",
            comment.replace('\n', "\n> ")
        ));
    }
    if let Some(err) = &preview.error_text {
        md.push_str(&format!("\n**Техническая ошибка:**\n```\n{}\n```\n", err));
    }
    if let Some(log_tail) = &preview.latest_log_tail {
        md.push_str("\n**Фрагмент журнала (обезличен):**\n```\n");
        // Take last 30 lines for clipboard readability
        let last_lines: Vec<&str> = log_tail.lines().collect();
        let start = last_lines.len().saturating_sub(30);
        md.push_str(&last_lines[start..].join("\n"));
        md.push_str("\n```\n");
    }

    md
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn report_get_preview(
    paths: State<'_, WexPaths>,
    version: Option<String>,
    error_text: Option<String>,
    user_comment: Option<String>,
    player_name: Option<String>,
) -> Result<BugReportPreviewDto, String> {
    Ok(prepare_report_preview_impl(
        &paths,
        version.as_deref(),
        error_text.as_deref(),
        user_comment.as_deref(),
        player_name.as_deref(),
    ))
}

#[tauri::command]
pub async fn report_save_zip(
    paths: State<'_, WexPaths>,
    version: Option<String>,
    error_text: Option<String>,
    user_comment: Option<String>,
    player_name: Option<String>,
    custom_dest_dir: Option<String>,
) -> Result<SaveReportResultDto, String> {
    save_report_zip_with_opener(
        &paths,
        version.as_deref(),
        error_text.as_deref(),
        user_comment.as_deref(),
        player_name.as_deref(),
        custom_dest_dir.as_deref(),
        Some(&SystemExplorerOpener),
    )
}

#[tauri::command]
pub async fn report_copy_text(
    paths: State<'_, WexPaths>,
    version: Option<String>,
    error_text: Option<String>,
    user_comment: Option<String>,
    player_name: Option<String>,
) -> Result<String, String> {
    Ok(format_clipboard_report_impl(
        &paths,
        version.as_deref(),
        error_text.as_deref(),
        user_comment.as_deref(),
        player_name.as_deref(),
    ))
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_anonymize_windows_paths() {
        let input = "Exception in thread \"main\" java.io.FileNotFoundException: C:\\Users\\Administrator\\AppData\\Roaming\\.wex\\instances\\1.21.1\\options.txt";
        let anonymized = anonymize_text(input, None);
        assert!(!anonymized.contains("Administrator"));
        assert!(anonymized.contains("C:\\Users\\***\\AppData"));
    }

    #[test]
    fn test_anonymize_forward_slash_and_linux_paths() {
        let input = "Failed to load /home/alexey/.local/share/wex/test.jar or C:/Users/Alexey/AppData/file.json";
        let anonymized = anonymize_text(input, None);
        assert!(!anonymized.contains("alexey"));
        assert!(!anonymized.contains("Alexey"));
        assert!(anonymized.contains("/home/***/.local"));
        assert!(anonymized.contains("C:/Users/***/AppData"));
    }

    #[test]
    fn test_anonymize_player_nickname() {
        let input = "[14:20:01] [Client/INFO]: Connecting to lobby with player secret_gamer_1337 on version 1.21.1";
        let anonymized = anonymize_text(input, Some("secret_gamer_1337"));
        assert!(!anonymized.contains("secret_gamer_1337"));
        assert!(anonymized.contains("<player>"));
    }

    #[test]
    fn test_anonymize_sensitive_token_headers() {
        let input = "POST /api/report HTTP/1.1\nX-User-Token: a1b2c3d4e5f607182930abcdef123456\nuserToken = secret_auth_hash_value\nNormal text line";
        let anonymized = anonymize_text(input, None);
        assert!(!anonymized.contains("a1b2c3d4e5f607182930abcdef123456"));
        assert!(!anonymized.contains("secret_auth_hash_value"));
        assert!(anonymized.contains("X-User-Token: ***"));
        assert!(anonymized.contains("userToken = ***"));
    }

    #[test]
    fn test_prepare_report_preview_structure() {
        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().to_path_buf()));

        let inst_dir = paths.instance_dir("1.21.1");
        fs::create_dir_all(inst_dir.join("logs")).unwrap();
        fs::write(
            inst_dir.join("logs").join("latest.log"),
            b"[12:00:00] [main/INFO]: Minecraft loaded\n[12:00:01] [main/ERROR]: OutOfMemoryError in C:\\Users\\Developer\\game",
        )
        .unwrap();

        let preview = prepare_report_preview_impl(
            &paths,
            Some("1.21.1"),
            Some("Fatal error occurred in C:\\Users\\Developer\\launch.bat"),
            Some("Game crashed when clicking settings"),
            Some("Developer"),
        );

        assert_eq!(preview.game_version, "1.21.1");
        assert_eq!(preview.launcher_version, env!("CARGO_PKG_VERSION"));
        assert!(preview
            .error_text
            .as_ref()
            .unwrap()
            .contains("C:\\Users\\***\\launch.bat"));
        assert!(preview
            .latest_log_tail
            .as_ref()
            .unwrap()
            .contains("OutOfMemoryError"));
        assert!(!preview
            .latest_log_tail
            .as_ref()
            .unwrap()
            .contains("Developer"));
    }

    #[test]
    fn test_save_report_zip_creates_valid_archive() {
        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().join("launcher_data")));

        let inst_dir = paths.instance_dir("1.21.1");
        fs::create_dir_all(inst_dir.join("mods")).unwrap();
        fs::write(
            inst_dir.join("mods").join("sodium.jar"),
            b"fake sodium content",
        )
        .unwrap();

        let dest_dir = temp.path().join("custom_desktop");
        fs::create_dir_all(&dest_dir).unwrap();

        let res = save_report_zip_impl(
            &paths,
            Some("1.21.1"),
            Some("Crash message"),
            Some("User comment"),
            Some("MyNick"),
            Some(dest_dir.to_str().unwrap()),
        )
        .unwrap();

        assert!(res.success);
        let zip_path = PathBuf::from(&res.zip_path);
        assert!(zip_path.is_file());
        REVEALED_REPORT.with(|path| assert_eq!(path.borrow().as_ref(), Some(&zip_path)));
        assert!(res.file_size_bytes > 0);

        // Read and verify zip contents
        let zip_file = File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(zip_file).unwrap();
        assert!(archive.by_name("report.json").is_ok());
        assert!(archive.by_name("system-info.txt").is_ok());
        assert!(archive.by_name("mods.txt").is_ok());
    }

    #[test]
    fn test_anonymize_ip_addresses_and_tunnels() {
        let input = "Player Alex connected from 192.168.1.50:25565, tunnel at friend.auto.playit.gg:12345 or 147.185.221.16\nLoading Minecraft 1.21.1 with Fabric 0.16.9 (jdk-21.0.12.1)";
        let anonymized = anonymize_text(input, None);

        // IPs and tunnels must be masked
        assert!(!anonymized.contains("192.168.1.50"));
        assert!(!anonymized.contains("147.185.221.16"));
        assert!(!anonymized.contains("friend.auto.playit.gg"));
        assert!(anonymized.contains("[IP]"));
        assert!(anonymized.contains("[tunnel]"));

        // Versions must NOT be broken
        assert!(anonymized.contains("1.21.1"));
        assert!(anonymized.contains("0.16.9"));
        assert!(anonymized.contains("jdk-21.0.12.1"));
    }

    #[test]
    fn test_anonymize_json_escaped_paths_and_tokens() {
        let input = r#"{"path":"C:\\Users\\aq08\\AppData\\Roaming\\Wex","auth":"Bearer secret_token_12345","args":"--accessToken token998877"}"#;
        let anonymized = anonymize_text(input, None);

        assert!(!anonymized.contains("aq08"));
        assert!(!anonymized.contains("secret_token_12345"));
        assert!(!anonymized.contains("token998877"));
        assert!(anonymized.contains("C:\\\\Users\\\\***\\\\AppData"));
        assert!(anonymized.contains("Bearer ***"));
        assert!(anonymized.contains("--accessToken ***"));
    }

    #[test]
    fn test_save_report_zip_leaks_audit() {
        use std::io::Read;

        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().join("launcher_data")));

        let inst_dir = paths.instance_dir("1.21.1");
        fs::create_dir_all(inst_dir.join("logs")).unwrap();
        fs::create_dir_all(inst_dir.join("mods")).unwrap();

        // Populate logs with sensitive tokens, paths, and friends' IPs
        fs::write(
            inst_dir.join("logs").join("latest.log"),
            b"[10:00:00] [Client/INFO]: Connecting to 192.168.1.100:25565\n[10:00:01] [Client/INFO]: UserToken: SuperSecretUserToken\n[10:00:02] [Client/INFO]: Loaded from C:\\Users\\aq08\\AppData\\Local\\Temp\n",
        )
        .unwrap();

        fs::write(
            inst_dir.join("mods").join("test_mod.jar"),
            b"fake jar content",
        )
        .unwrap();

        let dest_dir = temp.path().join("out_reports");
        fs::create_dir_all(&dest_dir).unwrap();

        let res = save_report_zip_impl(
            &paths,
            Some("1.21.1"),
            Some("Error in C:\\Users\\aq08\\file.rs with key: secret_api_key_42"),
            Some("Friend at 147.185.221.16 hosted server"),
            Some("SecretGamerNick"),
            Some(dest_dir.to_str().unwrap()),
        )
        .unwrap();

        assert!(res.success);

        // Open and audit all files in the ZIP
        let zip_file = File::open(&res.zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(zip_file).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().unwrap().to_string();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();

            // Leak checks on every file in the ZIP:
            assert!(
                !content.contains("aq08"),
                "Leaked Windows username in {}",
                name
            );
            assert!(
                !content.contains("192.168.1.100"),
                "Leaked friend IP in {}",
                name
            );
            assert!(
                !content.contains("147.185.221.16"),
                "Leaked external IP in {}",
                name
            );
            assert!(
                !content.contains("SuperSecretUserToken"),
                "Leaked user token in {}",
                name
            );
            assert!(
                !content.contains("secret_api_key_42"),
                "Leaked secret key in {}",
                name
            );
            assert!(
                !content.contains("SecretGamerNick"),
                "Leaked player nickname in {}",
                name
            );
        }
    }

    #[test]
    fn test_anonymize_preserves_non_ascii_utf8() {
        let input = "Ошибка в пути C:\\Users\\Developer\\Игры и моды: файл не найден. Причина: некорректная кодировка.";
        let anonymized = anonymize_text(input, None);

        // Username masked
        assert!(!anonymized.contains("Developer"));
        assert!(anonymized.contains("C:\\Users\\***\\Игры"));

        // Cyrillic characters are 100% preserved without byte corruption
        assert!(anonymized.contains("Ошибка в пути"));
        assert!(anonymized.contains("Игры и моды: файл не найден"));
        assert!(anonymized.contains("Причина: некорректная кодировка."));
    }

    #[test]
    fn test_anonymize_masks_json_secrets_with_spaces() {
        let input = r#"{"password": "my secret phrase with spaces 123!", "usertoken": "xyz-token-abc", "normal": "hello world"}"#;
        let anonymized = anonymize_text(input, None);

        // Entire secret string with spaces must be replaced by ***, not just the first word
        assert!(!anonymized.contains("my secret phrase"));
        assert!(!anonymized.contains("with spaces"));
        assert!(!anonymized.contains("xyz-token-abc"));
        assert!(anonymized.contains(r#""password": "***""#));
        assert!(anonymized.contains(r#""usertoken": "***""#));
        assert!(anonymized.contains(r#""normal": "hello world""#));
    }

    #[test]
    fn test_anonymize_ipv6_addresses() {
        let input = "Connected to [2001:0db8:85a3:0000:0000:8a2e:0370:7334]:25565 at 12:34:56";
        let anonymized = anonymize_text(input, None);

        assert!(!anonymized.contains("2001:0db8"));
        assert!(anonymized.contains("[IP]"));
        // Timestamp must NOT be masked as an IP
        assert!(anonymized.contains("12:34:56"));
    }

    #[test]
    fn test_save_report_zip_with_mock_opener_does_not_spawn_explorer() {
        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().join("launcher_data")));
        let mock = MockPathOpener::default();

        let res = save_report_zip_with_opener(
            &paths,
            Some("1.21.1"),
            Some("Test error"),
            Some("Test comment"),
            Some("Player1"),
            Some(temp.path().to_str().unwrap()),
            Some(&mock),
        )
        .expect("save should succeed");

        assert!(res.success);
        let opened = mock.opened_paths.lock().unwrap();
        assert_eq!(opened.len(), 1);
        assert!(opened[0].ends_with(&res.file_name));
    }

    #[test]
    fn test_anonymize_username_with_spaces_and_idempotence() {
        let input = "Exception in C:\\Users\\John Doe\\AppData\\Local\\Wex\\log.txt and C:\\Users\\Иван Иванов\\game.log. Привет мир! 🚀";
        let anonymized = anonymize_text(input, None);

        // Neither 'John' nor 'Doe' should be present
        assert!(!anonymized.contains("John Doe"));
        assert!(!anonymized.contains("John"));
        assert!(!anonymized.contains("Doe"));
        assert!(anonymized.contains("C:\\Users\\***\\AppData"));

        // Cyrillic username with space masked
        assert!(!anonymized.contains("Иван Иванов"));
        assert!(!anonymized.contains("Иван"));
        assert!(!anonymized.contains("Иванов"));
        assert!(anonymized.contains("C:\\Users\\***\\game.log"));

        // Russian text and emoji preserved
        assert!(anonymized.contains("Привет мир! 🚀"));

        // Idempotency: calling anonymize_text a second and third time must yield identical result
        let double_anonymized = anonymize_text(&anonymized, None);
        assert_eq!(anonymized, double_anonymized);
        let triple_anonymized = anonymize_text(&double_anonymized, None);
        assert_eq!(double_anonymized, triple_anonymized);
    }

    #[test]
    fn test_report_preview_reflects_actual_configured_memory_and_java() {
        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().to_path_buf()));

        // Save a custom config with 6144 MB RAM and custom Java path
        let cfg = CoreConfig {
            max_memory_mb: 6144,
            java_mode: "custom".to_string(),
            custom_java_path: Some(PathBuf::from(r"C:\Users\John Doe\jdk-21\bin\javaw.exe")),
            ..Default::default()
        };
        cfg.save(&paths.config_file).unwrap();

        let preview = prepare_report_preview_impl(
            &paths,
            Some("1.21.1"),
            Some("Crash log with error"),
            Some("My comment"),
            Some("SecretPlayer"),
        );

        // Configured RAM must be reflected in DTO and JSON preview, not hardcoded 4096 MB
        assert_eq!(preview.memory_info, "6144 MB");
        assert!(preview.full_json_preview.contains(r#""memory": "6144 MB""#));

        // Custom Java must be masked and indicated as custom, without leaking 'John Doe'
        assert!(preview.java_version.contains("Пользовательская"));
        assert!(!preview.java_version.contains("John Doe"));
        assert!(preview.java_version.contains("C:\\Users\\***\\jdk-21"));
        assert!(preview.full_json_preview.contains("Пользовательская"));
    }
}
