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
    save_launcher_settings_impl(new_settings, &state.paths, state.core.as_ref()).await
}

#[tauri::command]
pub async fn reset_settings(state: State<'_, PlayState>) -> Result<LauncherSettingsDto, String> {
    reset_settings_impl(&state.paths, state.core.as_ref()).await
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
        let settings = get_launcher_settings_impl(&paths);
        assert_eq!(settings.player_name, "Player");
        assert_eq!(settings.ram_mb, 4096);
        assert_eq!(settings.graphics_preset, "High");
        assert_eq!(settings.java_mode, "auto");
        assert!(settings.discord_rpc);
        assert_eq!(settings.language, "ru");
        assert!(!settings.fast_start_jvm);

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
        let reset = reset_settings_impl(&paths, &mock_core).await.unwrap();
        assert_eq!(reset.player_name, "Player");
        assert_eq!(reset.ram_mb, 4096);
        assert_eq!(reset.graphics_preset, "High");
        assert_eq!(reset.java_mode, "auto");
        assert!(reset.discord_rpc);
    }
}
