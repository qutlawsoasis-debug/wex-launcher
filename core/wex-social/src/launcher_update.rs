use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub const DEFAULT_TARGET_PLATFORM: &str = "windows-x86_64";

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Сетевая ошибка HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Ошибка парсинга манифеста latest.json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Платформа «{0}» не найдена в манифесте обновления")]
    PlatformNotFound(String),
    #[error("Некорректная цифровая подпись обновления: {0}")]
    InvalidSignature(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformUpdateInfo {
    pub signature: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateManifest {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    #[serde(default)]
    pub platforms: HashMap<String, PlatformUpdateInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub download_url: Option<String>,
    pub signature: Option<String>,
    pub pub_date: Option<String>,
}

pub fn is_newer_version(current: &str, remote: &str) -> bool {
    let clean_cur = current.trim().trim_start_matches('v');
    let clean_rem = remote.trim().trim_start_matches('v');

    let cur_parts: Vec<u64> = clean_cur
        .split('.')
        .filter_map(|s| s.split('-').next().and_then(|num| num.parse().ok()))
        .collect();

    let rem_parts: Vec<u64> = clean_rem
        .split('.')
        .filter_map(|s| s.split('-').next().and_then(|num| num.parse().ok()))
        .collect();

    for i in 0..cur_parts.len().max(rem_parts.len()) {
        let c = cur_parts.get(i).copied().unwrap_or(0);
        let r = rem_parts.get(i).copied().unwrap_or(0);
        if r > c {
            return true;
        } else if r < c {
            return false;
        }
    }

    false
}

pub fn validate_signature_format(sig: &str) -> bool {
    let trimmed = sig.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Tauri updater signatures are typically Minisign format (starts with "untrusted comment:" or base64 > 32 chars)
    if trimmed.starts_with("untrusted comment:") || trimmed.len() >= 32 {
        return true;
    }
    false
}

pub fn check_update_from_manifest(
    current_version: &str,
    target_platform: &str,
    manifest_json: &str,
) -> Result<UpdateCheckResult, UpdateError> {
    let manifest: UpdateManifest = serde_json::from_str(manifest_json)?;

    let has_newer = is_newer_version(current_version, &manifest.version);
    if !has_newer {
        return Ok(UpdateCheckResult {
            update_available: false,
            current_version: current_version.to_string(),
            latest_version: Some(manifest.version),
            notes: manifest.notes,
            download_url: None,
            signature: None,
            pub_date: manifest.pub_date,
        });
    }

    let plat = manifest
        .platforms
        .get(target_platform)
        .ok_or_else(|| UpdateError::PlatformNotFound(target_platform.to_string()))?;

    if !validate_signature_format(&plat.signature) {
        return Err(UpdateError::InvalidSignature(
            "Некорректный формат сигнатуры minisign".to_string(),
        ));
    }

    Ok(UpdateCheckResult {
        update_available: true,
        current_version: current_version.to_string(),
        latest_version: Some(manifest.version),
        notes: manifest.notes,
        download_url: Some(plat.url.clone()),
        signature: Some(plat.signature.clone()),
        pub_date: manifest.pub_date,
    })
}

pub async fn check_update_from_url(
    client: &Client,
    current_version: &str,
    target_platform: &str,
    manifest_url: &str,
) -> Result<UpdateCheckResult, UpdateError> {
    let res = client.get(manifest_url).send().await?;
    let text = res.text().await?;
    check_update_from_manifest(current_version, target_platform, &text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_semver_comparisons() {
        assert!(is_newer_version("1.0.0", "1.0.1"));
        assert!(is_newer_version("v1.0.0", "v1.1.0"));
        assert!(is_newer_version("1.2.88", "1.3.0"));
        assert!(!is_newer_version("1.1.0", "1.1.0"));
        assert!(!is_newer_version("1.2.0", "1.1.9"));
    }

    #[test]
    fn test_check_update_manifest_success() {
        let manifest_json = r#"{
            "version": "v1.2.0",
            "notes": "Новые фичи и оптимизации",
            "pub_date": "2026-11-08T00:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "untrusted comment: signature from tauri secret key\nRWSomeBase64SignatureString1234567890==",
                    "url": "https://github.com/qutlawsoasis-debug/wex-launcher/releases/download/v1.2.0/wex_x64-setup.nsis.zip"
                }
            }
        }"#;

        let res = check_update_from_manifest("v1.0.0", "windows-x86_64", manifest_json).unwrap();
        assert!(res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("v1.2.0"));
        assert_eq!(res.notes.as_deref(), Some("Новые фичи и оптимизации"));
        assert!(res.download_url.unwrap().contains("wex_x64-setup.nsis.zip"));
        assert!(res.signature.unwrap().contains("untrusted comment"));
    }

    #[test]
    fn test_check_update_already_latest() {
        let manifest_json = r#"{
            "version": "v1.0.0",
            "notes": "Текущая версия",
            "platforms": {
                "windows-x86_64": {
                    "signature": "RWValidSignatureStringHere1234567890==",
                    "url": "https://example.com/app.zip"
                }
            }
        }"#;

        let res = check_update_from_manifest("v1.0.0", "windows-x86_64", manifest_json).unwrap();
        assert!(!res.update_available);
        assert!(res.download_url.is_none());
    }

    #[tokio::test]
    async fn test_check_update_from_mock_url() {
        let mock_server = MockServer::start().await;

        let manifest_json = r#"{
            "version": "1.5.0",
            "notes": "Remote update test",
            "platforms": {
                "windows-x86_64": {
                    "signature": "RWValidSignatureStringHere1234567890==",
                    "url": "https://download.wex.app/1.5.0.zip"
                }
            }
        }"#;

        Mock::given(method("GET"))
            .and(path("/latest.json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(manifest_json))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let url = format!("{}/latest.json", mock_server.uri());
        let res = check_update_from_url(&client, "1.4.0", "windows-x86_64", &url)
            .await
            .unwrap();

        assert!(res.update_available);
        assert_eq!(res.latest_version.as_deref(), Some("1.5.0"));
    }
}
