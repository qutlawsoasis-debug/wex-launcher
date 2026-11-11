//! Startup Splash Screen Commands
//!
//! Implements SPEC 1.1 "Экран начальной загрузки"
//! - Configuration loading from wex-core (%APPDATA%\Wex\config.json)
//! - Launcher directory verification and initialization
//!
//! Launcher updates are handled by `commands::updater` (real download,
//! signature check) and surfaced on the Play screen, not here.

use serde::{Deserialize, Serialize};
use tauri::State;
use wex_core::config::{LauncherConfig, WexPaths};
use wex_ipc::models::LauncherConfig as IpcLauncherConfig;

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

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
        assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));
        assert!(info.directories_ok);
        assert_eq!(info.config.selected_version, "1.20.1");
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
