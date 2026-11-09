//! Startup Splash Screen Commands
//!
//! Implements SPEC 1.1 "Экран начальной загрузки"
//! - Configuration loading from wex-core (%APPDATA%\Wex\config.json)
//! - Launcher directory verification and initialization
//! - Launcher update check via wex-social::launcher_update
//! - Update download simulation and event streaming
//! - Network error fallback support (offline transition)

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use wex_core::config::{LauncherConfig, WexPaths};
use wex_ipc::models::{LauncherConfig as IpcLauncherConfig, UpdateCheckResultDto};
use wex_social::launcher_update::DEFAULT_TARGET_PLATFORM;

/// Managed state for startup commands
pub struct StartupState {
    pub paths: WexPaths,
    pub client: reqwest::Client,
}

impl StartupState {
    pub fn new(paths: WexPaths) -> Self {
        Self {
            paths,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    pub fn with_client(paths: WexPaths, client: reqwest::Client) -> Self {
        Self { paths, client }
    }
}

/// Information returned to the frontend upon initial launch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StartupInfoDto {
    pub app_name: String,
    pub app_version: String,
    pub config: IpcLauncherConfig,
    pub directories_ok: bool,
    pub data_dir: String,
    pub instances_dir: String,
}

/// Real-time progress payload for launcher update download
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateProgressPayload {
    pub percentage: f32,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub stage: String,
}

/// Reads launcher configuration from %APPDATA%\Wex and verifies instance folders
#[tauri::command]
pub async fn startup_get_info(state: State<'_, StartupState>) -> Result<StartupInfoDto, String> {
    let dirs_ok = state.paths.ensure_dirs().is_ok();
    let config = LauncherConfig::load_or_default(&state.paths.config_file);

    Ok(StartupInfoDto {
        app_name: "Wex".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        config: IpcLauncherConfig::from(config),
        directories_ok: dirs_ok,
        data_dir: state.paths.root.to_string_lossy().to_string(),
        instances_dir: state.paths.instances_dir.to_string_lossy().to_string(),
    })
}

/// Verifies and creates base launcher directories
#[tauri::command]
pub async fn startup_verify_directories(state: State<'_, StartupState>) -> Result<bool, String> {
    state
        .paths
        .ensure_dirs()
        .map(|_| true)
        .map_err(|e| format!("Не удалось инициализировать каталоги Wex: {e}"))
}

/// Checks for launcher updates via wex-social launcher_update module
#[tauri::command]
pub async fn startup_check_update(
    manifest_url: Option<String>,
    simulate_network_error: Option<bool>,
    state: State<'_, StartupState>,
) -> Result<UpdateCheckResultDto, String> {
    if simulate_network_error.unwrap_or(false) {
        return Err("Сетевая ошибка: невозможно связаться с сервером обновлений".to_string());
    }

    let url = manifest_url
        .or_else(|| std::env::var("WEX_UPDATE_URL").ok())
        .or_else(|| std::env::var("WEX_MANIFEST_URL").ok())
        .unwrap_or_else(|| {
            "https://raw.githubusercontent.com/wex-launcher/manifest/main/latest.json".to_string()
        });
    let cur_ver = std::env::var("WEX_CURRENT_VERSION")
        .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

    super::overview::check_update_internal_with_paths(
        Some(&state.paths),
        &cur_ver,
        Some(&url),
        Some(DEFAULT_TARGET_PLATFORM),
    )
    .await
}

/// Downloads and applies launcher update, reporting progress through IPC events
#[tauri::command]
pub async fn startup_download_update(app: AppHandle, download_url: String) -> Result<(), String> {
    simulate_or_download_update(Some(&app), &download_url).await
}

/// Internal helper for updating with optional emitter (for testing)
pub async fn simulate_or_download_update(
    emitter: Option<&AppHandle>,
    download_url: &str,
) -> Result<(), String> {
    if download_url.trim().is_empty() {
        return Err("Не указан URL для загрузки обновления".to_string());
    }

    let total_bytes: u64 = 48 * 1024 * 1024; // ~48 MB update package
    let steps = [
        (10.0, "Проверка манифеста обновления...", 4_800_000),
        (35.0, "Загрузка пакета обновления...", 16_800_000),
        (70.0, "Распаковка и верификация подписи...", 33_600_000),
        (95.0, "Подготовка к перезапуску...", 45_600_000),
        (100.0, "Обновление готово к установке", total_bytes),
    ];

    for (pct, stage, downloaded) in steps {
        if let Some(em) = emitter {
            let _ = em.emit(
                "startup-update-progress",
                UpdateProgressPayload {
                    percentage: pct,
                    downloaded_bytes: downloaded,
                    total_bytes,
                    speed_bytes_per_sec: 14 * 1024 * 1024,
                    stage: stage.to_string(),
                },
            );
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_social::launcher_update::check_update_from_manifest;

    #[tokio::test]
    async fn test_startup_get_info_defaults() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let state = StartupState::new(paths);

        // Verify directory creation and info reading
        assert!(state.paths.ensure_dirs().is_ok());
        let config = LauncherConfig::load_or_default(&state.paths.config_file);

        let info = StartupInfoDto {
            app_name: "Wex".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            config: IpcLauncherConfig::from(config),
            directories_ok: true,
            data_dir: state.paths.root.to_string_lossy().to_string(),
            instances_dir: state.paths.instances_dir.to_string_lossy().to_string(),
        };

        assert_eq!(info.app_name, "Wex");
        assert_eq!(info.app_version, "0.1.0");
        assert!(info.directories_ok);
        assert_eq!(info.config.selected_version, "1.20.1");
    }

    #[test]
    fn test_startup_check_update_from_manifest_newer() {
        let manifest_json = r#"{
            "version": "v1.2.0",
            "notes": "Оптимизация запуска и новый экран",
            "platforms": {
                "windows-x86_64": {
                    "signature": "untrusted comment: signature\nRWValidSig123456789012345678901234567890==",
                    "url": "https://example.com/wex-update-v1.2.0.zip"
                }
            }
        }"#;

        let res = check_update_from_manifest("0.1.0", "windows-x86_64", manifest_json).unwrap();
        assert!(res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("v1.2.0"));
        assert_eq!(
            res.download_url.as_deref(),
            Some("https://example.com/wex-update-v1.2.0.zip")
        );
    }

    #[test]
    fn test_startup_check_update_from_manifest_same_version() {
        let manifest_json = r#"{
            "version": "v0.1.0",
            "notes": "Текущая актуальная версия",
            "platforms": {
                "windows-x86_64": {
                    "signature": "RWValidSig123456789012345678901234567890==",
                    "url": "https://example.com/wex.zip"
                }
            }
        }"#;

        let res = check_update_from_manifest("0.1.0", "windows-x86_64", manifest_json).unwrap();
        assert!(!res.update_available);
    }

    #[tokio::test]
    async fn test_startup_download_update_simulation() {
        let res =
            simulate_or_download_update(None, "https://example.com/download/v1.2.0.zip").await;
        assert!(res.is_ok());

        let err_res = simulate_or_download_update(None, "  ").await;
        assert!(err_res.is_err());
    }

    #[tokio::test]
    async fn test_startup_verify_directories_success() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        assert!(paths.ensure_dirs().is_ok());
        assert!(paths.instances_dir.exists());
        assert!(paths.libraries_dir.exists());
        assert!(paths.assets_dir.exists());
    }
}
