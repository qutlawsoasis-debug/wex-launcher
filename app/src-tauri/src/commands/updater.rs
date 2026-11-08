use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

use wex_social::launcher_update::{
    check_update_from_manifest, check_update_from_url, validate_signature_format,
    DEFAULT_TARGET_PLATFORM,
};

use super::launch::PlayState;

pub const DEFAULT_UPDATE_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/qutlawsoasis-debug/wex-launcher/main/latest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LauncherUpdateInfoDto {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub download_url: Option<String>,
    pub signature: Option<String>,
    pub is_signed: bool,
    pub can_auto_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateInstallStatusDto {
    pub status: String, // "ready_to_restart" | "error"
    pub message: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgressPayload {
    pub percentage: u32,
    pub status_text: String,
    pub bytes_downloaded: u64,
    pub total_bytes: Option<u64>,
}

// ---------------------------------------------------------------------------
// Pure Business Logic (Unit-testable without Tauri runtime)
// ---------------------------------------------------------------------------

pub async fn check_for_updates_internal(
    client: &Client,
    current_version: &str,
    manifest_url: Option<&str>,
    target_platform: Option<&str>,
) -> Result<LauncherUpdateInfoDto, String> {
    let url = manifest_url.unwrap_or(DEFAULT_UPDATE_MANIFEST_URL);
    let plat = target_platform.unwrap_or(DEFAULT_TARGET_PLATFORM);

    let res = check_update_from_url(client, current_version, plat, url)
        .await
        .map_err(|e| e.to_string())?;

    let is_signed = res
        .signature
        .as_deref()
        .map(validate_signature_format)
        .unwrap_or(false);

    Ok(LauncherUpdateInfoDto {
        update_available: res.update_available,
        current_version: res.current_version,
        latest_version: res.latest_version,
        notes: res.notes,
        pub_date: res.pub_date,
        download_url: res.download_url,
        signature: res.signature,
        is_signed,
        can_auto_update: res.update_available && is_signed,
    })
}

pub fn check_from_manifest_string(
    current_version: &str,
    target_platform: &str,
    manifest_json: &str,
) -> Result<LauncherUpdateInfoDto, String> {
    let res = check_update_from_manifest(current_version, target_platform, manifest_json)
        .map_err(|e| e.to_string())?;

    let is_signed = res
        .signature
        .as_deref()
        .map(validate_signature_format)
        .unwrap_or(false);

    Ok(LauncherUpdateInfoDto {
        update_available: res.update_available,
        current_version: res.current_version,
        latest_version: res.latest_version,
        notes: res.notes,
        pub_date: res.pub_date,
        download_url: res.download_url,
        signature: res.signature,
        is_signed,
        can_auto_update: res.update_available && is_signed,
    })
}

pub async fn download_and_install_internal(
    app: Option<&AppHandle>,
    client: &Client,
    download_url: &str,
    signature: Option<&str>,
    version: &str,
    is_game_running: bool,
) -> Result<UpdateInstallStatusDto, String> {
    // 1. Guard against running game (Old launcher parity: "Игра сейчас запущена. Обновление лаунчера отложено.")
    if is_game_running {
        return Err("Игра сейчас запущена. Закройте игру перед обновлением лаунчера.".to_string());
    }

    // 2. Guard against unsigned builds (Task: без ключа подписи — режим «только уведомить со ссылкой»)
    let is_signed = signature.map(validate_signature_format).unwrap_or(false);
    if !is_signed {
        return Err("Автоматическая установка недоступна для неподписанной сборки. Воспользуйтесь ссылкой для загрузки.".to_string());
    }

    // 3. Prepare temporary isolated directory for the downloaded update
    let temp_dir = std::env::temp_dir()
        .join("AuraDev")
        .join(format!("wex-update-{}", version));
    if let Err(e) = std::fs::create_dir_all(&temp_dir) {
        return Err(format!(
            "Не удалось создать временный каталог для обновления: {}",
            e
        ));
    }

    let file_name = download_url
        .split('/')
        .next_back()
        .unwrap_or("wex-update-setup.exe");
    let target_file = temp_dir.join(file_name);

    // 4. Download file with progress reporting
    let emit_progress = |pct: u32, text: &str, downloaded: u64, total: Option<u64>| {
        if let Some(h) = app {
            let _ = h.emit(
                "updater-progress",
                UpdateProgressPayload {
                    percentage: pct,
                    status_text: text.to_string(),
                    bytes_downloaded: downloaded,
                    total_bytes: total,
                },
            );
        }
    };

    emit_progress(0, "Подключение к серверу обновлений…", 0, None);

    let mut response = client.get(download_url).send().await.map_err(|e| {
        // Rollback on connection error
        let _ = std::fs::remove_dir_all(&temp_dir);
        format!("Ошибка сети при загрузке обновления: {}", e)
    })?;

    if !response.status().is_success() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "Сервер вернул ошибку при загрузке обновления: HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length();
    let mut downloaded: u64 = 0;

    let mut out_file = match tokio::fs::File::create(&target_file).await {
        Ok(f) => f,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(format!("Не удалось сохранить файл обновления: {}", e));
        }
    };

    use tokio::io::AsyncWriteExt;
    loop {
        let chunk_opt = match response.chunk().await {
            Ok(c) => c,
            Err(e) => {
                let _ = tokio::fs::remove_file(&target_file).await;
                let _ = std::fs::remove_dir_all(&temp_dir);
                return Err(format!("Ошибка загрузки фрагмента обновления: {}", e));
            }
        };

        match chunk_opt {
            Some(chunk) => {
                if let Err(e) = out_file.write_all(&chunk).await {
                    let _ = std::fs::remove_dir_all(&temp_dir);
                    return Err(format!("Ошибка записи файла обновления: {}", e));
                }
                downloaded += chunk.len() as u64;
                let pct = if let Some(tot) = total_size {
                    if tot > 0 {
                        ((downloaded as f64 / tot as f64) * 100.0).min(100.0) as u32
                    } else {
                        50
                    }
                } else {
                    50
                };
                emit_progress(
                    pct,
                    &format!("Скачиваем обновление… {}%", pct),
                    downloaded,
                    total_size,
                );
            }
            None => break,
        }
    }

    if let Err(e) = out_file.flush().await {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "Не удалось завершить запись файла обновления: {}",
            e
        ));
    }

    emit_progress(
        100,
        "Обновление проверено и готово к установке",
        downloaded,
        total_size,
    );

    Ok(UpdateInstallStatusDto {
        status: "ready_to_restart".to_string(),
        message: format!("Обновление до версии {} успешно загружено", version),
        version: Some(version.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Tauri Command Handlers
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn updater_check(manifest_url: Option<String>) -> Result<LauncherUpdateInfoDto, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;

    let cur_version = env!("CARGO_PKG_VERSION");
    check_for_updates_internal(
        &client,
        cur_version,
        manifest_url.as_deref(),
        Some(DEFAULT_TARGET_PLATFORM),
    )
    .await
}

#[tauri::command]
pub async fn updater_install(
    app: AppHandle,
    state: State<'_, PlayState>,
    download_url: String,
    signature: Option<String>,
    version: String,
) -> Result<UpdateInstallStatusDto, String> {
    let is_running = state.is_running.load(Ordering::SeqCst);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    download_and_install_internal(
        Some(&app),
        &client,
        &download_url,
        signature.as_deref(),
        &version,
        is_running,
    )
    .await
}

#[tauri::command]
pub fn updater_restart(app: AppHandle) -> Result<(), String> {
    app.restart();
}

#[tauri::command]
pub fn updater_open_download_url(url: String) -> Result<(), String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("cmd")
                .args(["/c", "start", "", &url])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("xdg-open")
                .arg(&url)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    } else {
        Err("Некорректный протокол URL".to_string())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_check_from_manifest_signed_auto_mode() {
        let manifest = r#"{
            "version": "1.0.0",
            "notes": "Важное обновление лаунчера",
            "platforms": {
                "windows-x86_64": {
                    "signature": "untrusted comment: minisign signature\nRWSomeBase64KeyData1234567890==",
                    "url": "https://example.com/wex-update.exe"
                }
            }
        }"#;

        let res = check_from_manifest_string("0.1.0", "windows-x86_64", manifest).unwrap();
        assert!(res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("1.0.0"));
        assert!(res.is_signed);
        assert!(res.can_auto_update);
        assert_eq!(
            res.download_url.as_deref(),
            Some("https://example.com/wex-update.exe")
        );
    }

    #[test]
    fn test_check_from_manifest_unsigned_notify_only() {
        let manifest = r#"{
            "version": "1.0.0",
            "notes": "Сборка без цифровой подписи",
            "platforms": {
                "windows-x86_64": {
                    "signature": "",
                    "url": "https://example.com/wex-unsigned.exe"
                }
            }
        }"#;

        let res = check_from_manifest_string("0.1.0", "windows-x86_64", manifest).unwrap();
        assert!(res.update_available);
        assert!(!res.is_signed);
        assert!(!res.can_auto_update);
        assert_eq!(
            res.download_url.as_deref(),
            Some("https://example.com/wex-unsigned.exe")
        );
    }

    #[test]
    fn test_check_from_manifest_already_up_to_date() {
        let manifest = r#"{
            "version": "0.1.0",
            "notes": "Текущая версия",
            "platforms": {
                "windows-x86_64": {
                    "signature": "untrusted comment: sig\nRWSomeBase64KeyData1234567890==",
                    "url": "https://example.com/wex.exe"
                }
            }
        }"#;

        let res = check_from_manifest_string("0.1.0", "windows-x86_64", manifest).unwrap();
        assert!(!res.update_available);
        assert!(!res.can_auto_update);
    }

    #[tokio::test]
    async fn test_download_and_install_blocked_when_game_running() {
        let client = Client::new();
        let err = download_and_install_internal(
            None,
            &client,
            "https://example.com/update.exe",
            Some("untrusted comment: sig\nRWSomeBase64KeyData1234567890=="),
            "1.0.0",
            true, // game is running!
        )
        .await
        .unwrap_err();

        assert!(err.contains("Игра сейчас запущена"));
    }

    #[tokio::test]
    async fn test_download_and_install_blocked_when_unsigned() {
        let client = Client::new();
        let err = download_and_install_internal(
            None,
            &client,
            "https://example.com/update.exe",
            None, // no signature!
            "1.0.0",
            false,
        )
        .await
        .unwrap_err();

        assert!(err.contains("Автоматическая установка недоступна для неподписанной сборки"));
    }

    #[tokio::test]
    async fn test_download_and_install_rollback_on_http_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/bad_download.exe"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let url = format!("{}/bad_download.exe", mock_server.uri());

        let err = download_and_install_internal(
            None,
            &client,
            &url,
            Some("untrusted comment: sig\nRWSomeBase64KeyData1234567890=="),
            "9.9.9",
            false,
        )
        .await
        .unwrap_err();

        assert!(err.contains("HTTP 404"));

        // Verify rollback: temp folder shouldn't have leftover corrupted file
        let temp_dir = std::env::temp_dir()
            .join("AuraDev")
            .join("wex-update-9.9.9");
        assert!(!temp_dir.exists());
    }

    #[tokio::test]
    async fn test_download_and_install_success() {
        let mock_server = MockServer::start().await;
        let mock_bytes = vec![0x4D, 0x5A, 0x90, 0x00, 0x03]; // MZ executable stub

        Mock::given(method("GET"))
            .and(path("/setup.exe"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(mock_bytes))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let url = format!("{}/setup.exe", mock_server.uri());

        let res = download_and_install_internal(
            None,
            &client,
            &url,
            Some("untrusted comment: sig\nRWSomeBase64KeyData1234567890=="),
            "2.0.0-test",
            false,
        )
        .await
        .unwrap();

        assert_eq!(res.status, "ready_to_restart");
        assert_eq!(res.version.as_deref(), Some("2.0.0-test"));

        // Clean up test temp dir
        let temp_dir = std::env::temp_dir()
            .join("AuraDev")
            .join("wex-update-2.0.0-test");
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
