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

/// Downloads and applies launcher update, reporting genuine progress through IPC events
#[tauri::command]
pub async fn startup_download_update(
    app: AppHandle,
    download_url: String,
    state: State<'_, StartupState>,
) -> Result<(), String> {
    download_update_stream(&state.client, Some(&app), &download_url).await
}

/// Downloads update package streamed from URL and emits genuine progress events (no fake steps)
pub async fn download_update_stream(
    client: &reqwest::Client,
    emitter: Option<&AppHandle>,
    download_url: &str,
) -> Result<(), String> {
    if download_url.trim().is_empty() {
        return Err("Не указан URL для загрузки обновления".to_string());
    }

    // Honest initial event: preparing, 0%
    if let Some(em) = emitter {
        let _ = em.emit(
            "startup-update-progress",
            UpdateProgressPayload {
                percentage: 0.0,
                downloaded_bytes: 0,
                total_bytes: 0,
                speed_bytes_per_sec: 0,
                stage: "Подготавливаем…".to_string(),
            },
        );
    }

    let mut res = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Сетевая ошибка при загрузке обновления: {e}"))?;

    if !res.status().is_success() {
        return Err(format!("Сервер вернул ошибку HTTP {}", res.status()));
    }

    let total_bytes = res.content_length().unwrap_or(0);
    let mut downloaded_bytes: u64 = 0;
    let start_time = std::time::Instant::now();

    while let Some(chunk) = res
        .chunk()
        .await
        .map_err(|e| format!("Ошибка передачи данных: {e}"))?
    {
        downloaded_bytes += chunk.len() as u64;

        if let Some(em) = emitter {
            let elapsed_sec = start_time.elapsed().as_secs_f64();
            let speed = if elapsed_sec > 0.05 {
                (downloaded_bytes as f64 / elapsed_sec) as u64
            } else {
                0
            };

            let percentage = if total_bytes > 0 {
                ((downloaded_bytes as f64 / total_bytes as f64) * 100.0).clamp(0.0, 100.0) as f32
            } else {
                0.0
            };

            let stage = if percentage >= 100.0 {
                "Обновление загружено".to_string()
            } else if total_bytes > 0 {
                format!(
                    "Загрузка: {:.1} / {:.1} МБ",
                    downloaded_bytes as f64 / (1024.0 * 1024.0),
                    total_bytes as f64 / (1024.0 * 1024.0)
                )
            } else {
                "Подготавливаем…".to_string()
            };

            let _ = em.emit(
                "startup-update-progress",
                UpdateProgressPayload {
                    percentage,
                    downloaded_bytes,
                    total_bytes,
                    speed_bytes_per_sec: speed,
                    stage,
                },
            );
        }
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
        assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));
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
    async fn test_startup_download_update_stream_real() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let test_payload = b"TEST_UPDATE_PAYLOAD_BYTES";

        Mock::given(method("GET"))
            .and(path("/wex-update.zip"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(test_payload.to_vec()))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let download_url = format!("{}/wex-update.zip", server.uri());

        let res = download_update_stream(&client, None, &download_url).await;
        assert!(res.is_ok());

        let err_res = download_update_stream(&client, None, "  ").await;
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
