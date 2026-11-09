use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;

use crate::commands::launch::PlayState;
use crate::ipc::SystemInfo;
use wex_core::config::{LauncherConfig as CoreConfig, WexPaths};
use wex_ipc::core_trait::LauncherCore;

#[cfg(windows)]
pub fn get_total_ram_mb() -> u32 {
    use std::mem::zeroed;
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    unsafe {
        let mut status: MemoryStatusEx = zeroed();
        status.length = std::mem::size_of::<MemoryStatusEx>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            (status.total_phys / (1024 * 1024)) as u32
        } else {
            16384
        }
    }
}

#[cfg(not(windows))]
pub fn get_total_ram_mb() -> u32 {
    16384
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LauncherSettingsDto {
    pub selected_version: String,
    pub player_name: String,
    pub ram_mb: u32,
    pub total_memory_mb: u32,
    pub game_dir: String,
    pub graphics_preset: String, // "Low" | "Medium" | "High" | "Ultra"
    pub java_mode: String,       // "auto" | "custom"
    pub custom_java_path: Option<String>,
    pub detected_java_17: Option<String>,
    pub detected_java_21: Option<String>,
    pub discord_rpc: bool,
    pub language: String,   // "ru" | "en"
    pub appearance: String, // "dark" | "light" | "auto"
    pub accent: String,     // "blue" | "purple" | "pink" | "orange" | "green" | "mint" | "graphite"
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
    pub fast_start_jvm: bool,
    #[serde(default = "default_fast_launch_dto")]
    pub fast_launch: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_true")]
    pub friend_notifications: bool,
    #[serde(default = "default_window_behavior")]
    pub game_window_behavior: String, // "keep" | "minimize" | "tray"
}

pub fn default_fast_launch_dto() -> bool {
    get_total_ram_mb() >= 15360
}

fn default_true() -> bool {
    true
}

fn default_window_behavior() -> String {
    "minimize".to_string()
}

impl Default for LauncherSettingsDto {
    fn default() -> Self {
        Self {
            selected_version: "1.21.1".to_string(),
            player_name: "Player".to_string(),
            ram_mb: 4096,
            total_memory_mb: get_total_ram_mb(),
            game_dir: String::new(),
            graphics_preset: "High".to_string(),
            java_mode: "auto".to_string(),
            custom_java_path: None,
            detected_java_17: None,
            detected_java_21: None,
            discord_rpc: true,
            language: "ru".to_string(),
            appearance: "dark".to_string(),
            accent: "blue".to_string(),
            reduce_motion: false,
            reduce_transparency: false,
            fast_start_jvm: false,
            fast_launch: default_fast_launch_dto(),
            minimize_to_tray: true,
            friend_notifications: true,
            game_window_behavior: "minimize".to_string(),
        }
    }
}

/// Detects runtime paths for Mojang / standard Java 17 and Java 21 installations.
pub fn detect_java_paths(runtime_dir: &Path) -> (Option<String>, Option<String>) {
    let mut java_17 = None;
    let mut java_21 = None;

    // 1. Check Wex isolated Mojang runtimes
    let mojang_17 =
        runtime_dir
            .join("java-runtime-gamma")
            .join("bin")
            .join(if cfg!(target_os = "windows") {
                "javaw.exe"
            } else {
                "java"
            });
    if mojang_17.exists() {
        java_17 = Some(mojang_17.to_string_lossy().to_string());
    }

    let mojang_21 =
        runtime_dir
            .join("java-runtime-delta")
            .join("bin")
            .join(if cfg!(target_os = "windows") {
                "javaw.exe"
            } else {
                "java"
            });
    if mojang_21.exists() {
        java_21 = Some(mojang_21.to_string_lossy().to_string());
    }

    // 2. Check Windows Program Files standard installations if not found in runtime
    #[cfg(target_os = "windows")]
    {
        let standard_roots = [
            PathBuf::from(r"C:\Program Files\Java"),
            PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
            PathBuf::from(r"C:\Program Files\Microsoft"),
        ];

        for root in &standard_roots {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let javaw = path.join("bin").join("javaw.exe");
                    if javaw.exists() {
                        if java_17.is_none() && (name.contains("17") || name.contains("gamma")) {
                            java_17 = Some(javaw.to_string_lossy().to_string());
                        }
                        if java_21.is_none() && (name.contains("21") || name.contains("delta")) {
                            java_21 = Some(javaw.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }

    (java_17, java_21)
}

pub fn get_launcher_settings_impl(paths: &WexPaths) -> LauncherSettingsDto {
    let cfg = CoreConfig::load_or_default(&paths.config_file);
    let (j17, j21) = detect_java_paths(&paths.runtime_dir);

    LauncherSettingsDto {
        selected_version: cfg.selected_version,
        player_name: cfg.nickname,
        ram_mb: cfg.max_memory_mb,
        total_memory_mb: get_total_ram_mb(),
        game_dir: paths.root.to_string_lossy().to_string(),
        graphics_preset: cfg.graphics_preset,
        java_mode: cfg.java_mode,
        custom_java_path: cfg
            .custom_java_path
            .map(|p| p.to_string_lossy().to_string()),
        detected_java_17: j17,
        detected_java_21: j21,
        discord_rpc: cfg.discord_rpc,
        language: cfg.language,
        appearance: cfg.appearance,
        accent: cfg.accent,
        reduce_motion: cfg.reduce_motion,
        reduce_transparency: cfg.reduce_transparency,
        fast_start_jvm: cfg.fast_start_jvm,
        fast_launch: cfg.fast_launch,
        minimize_to_tray: cfg.minimize_to_tray,
        friend_notifications: cfg.friend_notifications,
        game_window_behavior: cfg.game_window_behavior,
    }
}

pub async fn save_launcher_settings_impl(
    new_settings: LauncherSettingsDto,
    paths: &WexPaths,
    core: &dyn LauncherCore,
) -> Result<(), String> {
    let mut cfg = CoreConfig::load_or_default(&paths.config_file);

    cfg.selected_version = new_settings.selected_version;
    cfg.nickname = new_settings.player_name;
    cfg.max_memory_mb = new_settings.ram_mb.max(1024);
    cfg.graphics_preset = new_settings.graphics_preset.clone();
    cfg.java_mode = new_settings.java_mode.clone();

    if new_settings.java_mode == "custom" {
        if let Some(ref path_str) = new_settings.custom_java_path {
            let trimmed = path_str.trim();
            if !trimmed.is_empty() {
                cfg.custom_java_path = Some(PathBuf::from(trimmed));
            } else {
                cfg.custom_java_path = None;
            }
        } else {
            cfg.custom_java_path = None;
        }
    } else {
        cfg.custom_java_path = None;
    }

    cfg.discord_rpc = new_settings.discord_rpc;
    cfg.language = new_settings.language;
    cfg.appearance = new_settings.appearance;
    cfg.accent = new_settings.accent;
    cfg.reduce_motion = new_settings.reduce_motion;
    cfg.reduce_transparency = new_settings.reduce_transparency;
    cfg.fast_start_jvm = new_settings.fast_start_jvm;
    cfg.fast_launch = new_settings.fast_launch;
    cfg.minimize_to_tray = new_settings.minimize_to_tray;
    cfg.friend_notifications = new_settings.friend_notifications;
    cfg.game_window_behavior = new_settings.game_window_behavior;

    cfg.save(&paths.config_file)
        .map_err(|e| format!("Не удалось сохранить конфигурацию: {}", e))?;

    // Apply graphics preset to instance dir (options.txt, shaderpacks) via wex-extras
    let pool_dirs = vec![paths.root.join("shaderpacks_pool")];
    let inst_dir = paths.instance_dir(&cfg.selected_version);
    let _ = wex_extras::presets::apply_preset(&inst_dir, &new_settings.graphics_preset, &pool_dirs);

    // Sync with LauncherCore
    let _ = core.save_config(cfg.into()).await;

    Ok(())
}

pub async fn reset_settings_impl(
    paths: &WexPaths,
    core: &dyn LauncherCore,
) -> Result<LauncherSettingsDto, String> {
    let default_cfg = CoreConfig::default();
    default_cfg
        .save(&paths.config_file)
        .map_err(|e| format!("Не удалось сбросить конфигурацию: {}", e))?;

    let _ = core.save_config(default_cfg.into()).await;

    Ok(get_launcher_settings_impl(paths))
}

#[tauri::command]
pub async fn get_launcher_settings(
    state: State<'_, PlayState>,
) -> Result<LauncherSettingsDto, String> {
    Ok(get_launcher_settings_impl(&state.paths))
}

#[tauri::command]
pub async fn save_launcher_settings(
    new_settings: LauncherSettingsDto,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let old_cfg = CoreConfig::load_or_default(&state.paths.config_file);
    let discord_rpc = new_settings.discord_rpc;
    save_launcher_settings_impl(new_settings.clone(), &state.paths, state.core.as_ref()).await?;

    let mut lock = state.discord.lock().await;
    if let Some(client) = lock.as_mut() {
        client.set_enabled(discord_rpc).await;
    }

    let need_restart = old_cfg.selected_version != new_settings.selected_version
        || old_cfg.nickname != new_settings.player_name
        || old_cfg.max_memory_mb != new_settings.ram_mb
        || old_cfg.fast_launch != new_settings.fast_launch;

    if need_restart {
        state.cleanup_prewarm().await;
        if new_settings.fast_launch {
            state
                .trigger_prewarm(
                    new_settings.selected_version,
                    new_settings.player_name,
                    new_settings.ram_mb,
                )
                .await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn reset_settings(state: State<'_, PlayState>) -> Result<LauncherSettingsDto, String> {
    state.cleanup_prewarm().await;
    let res = reset_settings_impl(&state.paths, state.core.as_ref()).await?;
    let mut lock = state.discord.lock().await;
    if let Some(client) = lock.as_mut() {
        client.set_enabled(true).await;
    }
    if res.fast_launch {
        state
            .trigger_prewarm(
                res.selected_version.clone(),
                res.player_name.clone(),
                res.ram_mb,
            )
            .await;
    }
    Ok(res)
}

#[tauri::command]
pub async fn get_discord_rpc_preview(
    state: State<'_, PlayState>,
) -> Result<wex_ipc::models::DiscordPreviewDto, String> {
    let cfg = CoreConfig::load_or_default(&state.paths.config_file);
    let lock = state.discord.lock().await;
    let preview = if let Some(client) = lock.as_ref() {
        client.preview()
    } else {
        wex_extras::discord::get_activity_preview(
            Some(&wex_extras::discord::Activity::InLauncher),
            cfg.discord_rpc,
            false,
        )
    };
    Ok(preview.into())
}

#[tauri::command]
pub async fn open_game_data_dir(state: State<'_, PlayState>) -> Result<(), String> {
    let path = &state.paths.root;
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_data_dir(state: State<'_, PlayState>) -> String {
    state.paths.root.to_string_lossy().to_string()
}

#[tauri::command]
pub async fn get_system_info(state: State<'_, PlayState>) -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        total_memory_mb: get_total_ram_mb(),
        os_name: std::env::consts::OS.to_string(),
        launcher_version: "1.0.0".to_string(),
        data_dir: state.paths.root.to_string_lossy().to_string(),
    })
}

// ---------------------------------------------------------------------------
// Backward-compatibility aliases for UI
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_launcher_config(
    state: State<'_, PlayState>,
) -> Result<LauncherSettingsDto, String> {
    get_launcher_settings(state).await
}

#[tauri::command]
pub async fn save_launcher_config(
    new_config: LauncherSettingsDto,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    save_launcher_settings(new_config, state).await
}

#[tauri::command]
pub async fn open_data_dir(state: State<'_, PlayState>) -> Result<(), String> {
    open_game_data_dir(state).await
}

// ---------------------------------------------------------------------------
// Maintenance: File Verification & Cache Cleanup (SPEC & Parity)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct IntegrityCheckResult {
    pub version: String,
    pub total_files: usize,
    pub valid_files: usize,
    pub repaired_files: usize,
    pub status_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CleanupCacheResult {
    pub bytes_freed: u64,
    pub formatted_freed: String,
    pub files_removed: usize,
    pub status_message: String,
}

pub fn format_freed_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 байт".to_string();
    }
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} ГБ", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} МБ", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} КБ", bytes as f64 / KB as f64)
    } else {
        format!("{} байт", bytes)
    }
}

pub async fn verify_instance_files_impl(
    paths: &WexPaths,
    version: &str,
    client: &wex_core::reqwest::Client,
) -> Result<IntegrityCheckResult, String> {
    let manifest = wex_core::pack::fetch_manifest(client, version, None, None)
        .await
        .map_err(|e| format!("Не удалось получить манифест для {}: {}", version, e))?;

    let instance_dir = paths.instance_dir(version);
    if !instance_dir.exists() {
        return Ok(IntegrityCheckResult {
            version: version.to_string(),
            total_files: manifest.files.len(),
            valid_files: 0,
            repaired_files: 0,
            status_message: format!("Инстанс {} ещё не установлен", version),
        });
    }

    let repo_prefix = wex_core::pack::get_repo_prefix(version);
    let repo = wex_core::pack::DEFAULT_PACK_REPO;
    let branch = wex_core::pack::DEFAULT_PACK_BRANCH;

    let total_files = manifest.files.len();
    let mut valid_files = 0;
    let mut repaired_files = 0;

    for file in &manifest.files {
        let clean_path = file.path.replace('\\', "/");
        let target_path = instance_dir.join(&clean_path);

        let is_mod = clean_path.starts_with("mods/") && clean_path.ends_with(".jar");
        let disabled_target_path = if is_mod {
            instance_dir.join(format!("{}.disabled", clean_path))
        } else {
            PathBuf::new()
        };

        let mut is_valid = false;
        if target_path.exists() {
            if let Ok(meta) = std::fs::metadata(&target_path) {
                if meta.len() == file.size {
                    if let Ok(hash) = wex_core::download::compute_file_hash(
                        &target_path,
                        wex_core::download::HashType::Sha256,
                    ) {
                        if hash.eq_ignore_ascii_case(&file.sha256) {
                            is_valid = true;
                        }
                    }
                }
            }
        } else if is_mod && disabled_target_path.exists() {
            if let Ok(meta) = std::fs::metadata(&disabled_target_path) {
                if meta.len() == file.size {
                    is_valid = true;
                }
            }
        }

        if is_valid {
            valid_files += 1;
        } else {
            if let Some(parent) = target_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let escaped_segments: Vec<String> = clean_path
                .split('/')
                .map(wex_core::pack::urlencoding_encode)
                .collect();
            let raw_pack_url = format!(
                "https://raw.githubusercontent.com/{}/{}/{}{}",
                repo,
                branch,
                repo_prefix,
                escaped_segments.join("/")
            );

            let (primary_url, fallback_url) = if let Some(custom_url) = &file.url {
                (custom_url.as_str(), Some(raw_pack_url.as_str()))
            } else {
                (raw_pack_url.as_str(), None)
            };

            let expected_hash = Some((file.sha256.as_str(), wex_core::download::HashType::Sha256));
            let noop = wex_core::event::NoopEventHandler;
            if wex_core::download::download_file_with_fallback(
                client,
                primary_url,
                fallback_url,
                &target_path,
                Some(file.size),
                expected_hash,
                &noop,
                None,
            )
            .await
            .is_ok()
            {
                repaired_files += 1;
            }
        }
    }

    let status_message = if repaired_files == 0 {
        format!("Все файлы проверены и в порядке ({})", total_files)
    } else {
        format!(
            "Проверка завершена: восстановлено файлов: {} из {}",
            repaired_files, total_files
        )
    };

    Ok(IntegrityCheckResult {
        version: version.to_string(),
        total_files,
        valid_files,
        repaired_files,
        status_message,
    })
}

pub fn cleanup_cache_impl(paths: &WexPaths) -> CleanupCacheResult {
    let mut bytes_freed: u64 = 0;
    let mut files_removed: usize = 0;

    let mut clean_file = |path: &Path| {
        if let Ok(meta) = std::fs::metadata(path) {
            let len = meta.len();
            if std::fs::remove_file(path).is_ok() {
                bytes_freed = bytes_freed.saturating_add(len);
                files_removed += 1;
            }
        }
    };

    // 1. Root logs: *.log.gz, *.tmp, *.part, *.old
    if paths.logs_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&paths.logs_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.ends_with(".log.gz")
                    || name.ends_with(".tmp")
                    || name.ends_with(".part")
                    || name.ends_with(".old")
                {
                    clean_file(&p);
                }
            }
        }
    }

    // 2. Instances logs, crash reports, and temp files
    for ver in &["1.20.1", "1.21.1"] {
        let inst = paths.instance_dir(ver);
        let logs = inst.join("logs");
        if logs.exists() {
            if let Ok(entries) = std::fs::read_dir(&logs) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.ends_with(".log.gz")
                        || name.ends_with(".tmp")
                        || name.ends_with(".part")
                        || name.ends_with(".old")
                    {
                        clean_file(&p);
                    }
                }
            }
        }
        let crashes = inst.join("crash-reports");
        if crashes.exists() {
            if let Ok(entries) = std::fs::read_dir(&crashes) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        clean_file(&p);
                    }
                }
            }
        }
    }

    let formatted_freed = format_freed_bytes(bytes_freed);
    let status_message = if files_removed == 0 {
        "Кэш уже чист (0 байт)".to_string()
    } else {
        format!(
            "Очищено {} кэша (удалено файлов: {})",
            formatted_freed, files_removed
        )
    };

    CleanupCacheResult {
        bytes_freed,
        formatted_freed,
        files_removed,
        status_message,
    }
}

/// Check and repair instance game files against pack manifest.
#[tauri::command]
pub async fn verify_instance_files(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<IntegrityCheckResult, String> {
    let ver = version.unwrap_or_else(|| {
        let cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);
        cfg.selected_version
    });
    let client = wex_core::reqwest::Client::builder()
        .user_agent("WexLauncher/0.1.0")
        .build()
        .map_err(|e| e.to_string())?;

    verify_instance_files_impl(&state.paths, &ver, &client).await
}

/// Compatibility alias for verify_instance_files.
#[tauri::command]
pub async fn settings_verify_files(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<IntegrityCheckResult, String> {
    verify_instance_files(version, state).await
}

/// Cleanup logs, crash reports, and temp downloads.
#[tauri::command]
pub async fn cleanup_cache(state: State<'_, PlayState>) -> Result<CleanupCacheResult, String> {
    Ok(cleanup_cache_impl(&state.paths))
}

/// Compatibility alias for cleanup_cache.
#[tauri::command]
pub async fn settings_cleanup_cache(
    state: State<'_, PlayState>,
) -> Result<CleanupCacheResult, String> {
    cleanup_cache(state).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_ipc::core_trait::MockLauncherCore;

    #[tokio::test]
    async fn test_settings_load_and_save() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        let mock_core = MockLauncherCore::new();

        // 1. Initial settings
        let expected_def = CoreConfig::default();
        let settings = get_launcher_settings_impl(&paths);
        assert_eq!(settings.player_name, "Player");
        assert_eq!(settings.ram_mb, expected_def.max_memory_mb);
        assert_eq!(settings.graphics_preset, expected_def.graphics_preset);
        assert_eq!(settings.java_mode, "auto");
        assert!(settings.discord_rpc);
        assert_eq!(settings.language, "ru");
        assert!(!settings.fast_start_jvm);
        assert!(settings.minimize_to_tray);

        // 2. Modify and save settings
        let mut updated = settings.clone();
        updated.player_name = "Alex".to_string();
        updated.ram_mb = 8192;
        updated.graphics_preset = "Ultra".to_string();
        updated.java_mode = "custom".to_string();
        updated.custom_java_path = Some(r"C:\Java\jdk-21\bin\javaw.exe".to_string());
        updated.discord_rpc = false;
        updated.language = "en".to_string();
        updated.fast_start_jvm = true;
        updated.minimize_to_tray = false;
        updated.game_window_behavior = "tray".to_string();

        save_launcher_settings_impl(updated, &paths, &mock_core)
            .await
            .unwrap();

        // 3. Reload and verify
        let reloaded = get_launcher_settings_impl(&paths);
        assert_eq!(reloaded.player_name, "Alex");
        assert_eq!(reloaded.ram_mb, 8192);
        assert_eq!(reloaded.graphics_preset, "Ultra");
        assert_eq!(reloaded.java_mode, "custom");
        assert_eq!(
            reloaded.custom_java_path,
            Some(r"C:\Java\jdk-21\bin\javaw.exe".to_string())
        );
        assert!(!reloaded.discord_rpc);
        assert_eq!(reloaded.language, "en");
        assert!(reloaded.fast_start_jvm);
        assert!(!reloaded.minimize_to_tray);
        assert_eq!(reloaded.game_window_behavior, "tray");
    }

    #[tokio::test]
    async fn test_reset_settings_to_defaults() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        let mock_core = MockLauncherCore::new();

        // Save customized settings
        let custom = LauncherSettingsDto {
            player_name: "CustomUser".to_string(),
            ram_mb: 6144,
            ..Default::default()
        };
        save_launcher_settings_impl(custom, &paths, &mock_core)
            .await
            .unwrap();

        // Reset
        let expected_def = CoreConfig::default();
        let reset = reset_settings_impl(&paths, &mock_core).await.unwrap();
        assert_eq!(reset.player_name, "Player");
        assert_eq!(reset.ram_mb, expected_def.max_memory_mb);
        assert_eq!(reset.graphics_preset, expected_def.graphics_preset);
        assert_eq!(reset.java_mode, "auto");
        assert!(reset.discord_rpc);
        assert!(reset.minimize_to_tray);
    }

    #[test]
    fn test_format_freed_bytes() {
        assert_eq!(format_freed_bytes(0), "0 байт");
        assert_eq!(format_freed_bytes(512), "512 байт");
        assert_eq!(format_freed_bytes(2048), "2 КБ");
        assert_eq!(format_freed_bytes(10 * 1024 * 1024), "10.0 МБ");
        assert_eq!(format_freed_bytes(2 * 1024 * 1024 * 1024), "2.00 ГБ");
    }

    #[test]
    fn test_cleanup_cache_impl() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        paths.ensure_instance_dirs("1.20.1").unwrap();

        // Populate dummy log and crash files
        let log_dir = paths.instance_dir("1.20.1").join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let log_file = log_dir.join("old.log.gz");
        std::fs::write(&log_file, vec![0u8; 1024]).unwrap();

        let crash_dir = paths.instance_dir("1.20.1").join("crash-reports");
        std::fs::create_dir_all(&crash_dir).unwrap();
        let crash_file = crash_dir.join("crash-2026.txt");
        std::fs::write(&crash_file, "error report details").unwrap();

        let result = cleanup_cache_impl(&paths);
        assert!(result.bytes_freed > 1024);
        assert_eq!(result.files_removed, 2);
        assert!(!log_file.exists());
        assert!(!crash_file.exists());
        assert!(result.status_message.contains("Очищено"));
    }
}
