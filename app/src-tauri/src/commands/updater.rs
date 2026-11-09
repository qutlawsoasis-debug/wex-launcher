use base64::Engine;
use minisign_verify::{PublicKey, Signature};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

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

#[path = "updater_stage.rs"]
mod staging;
pub use staging::StagedUpdate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallerExecutionPlan {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub target_version: String,
    pub installer_path: PathBuf,
}

static STAGED_UPDATE: Mutex<Option<StagedUpdate>> = Mutex::new(None);
static UPDATE_BUSY: AtomicBool = AtomicBool::new(false);

struct UpdateOperation;

impl UpdateOperation {
    fn begin() -> Result<Self, String> {
        UPDATE_BUSY
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "Обновление уже выполняется".to_string())?;
        Ok(Self)
    }
}

impl Drop for UpdateOperation {
    fn drop(&mut self) {
        UPDATE_BUSY.store(false, Ordering::Release);
    }
}

pub fn validate_update_version(version: &str) -> Result<(), String> {
    let target = semver::Version::parse(version)
        .map_err(|_| "Некорректная версия обновления".to_string())?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
    if !target.cmp_precedence(&current).is_gt() {
        return Err("Версия обновления должна быть новее установленной".into());
    }
    Ok(())
}

pub fn set_staged_update(update: StagedUpdate) {
    if let Ok(mut lock) = STAGED_UPDATE.lock() {
        *lock = Some(update);
    }
}

pub fn get_staged_update() -> Option<StagedUpdate> {
    STAGED_UPDATE.lock().ok().and_then(|lock| lock.clone())
}

pub fn clear_staged_update() {
    if let Ok(mut lock) = STAGED_UPDATE.lock() {
        *lock = None;
    }
}

// ---------------------------------------------------------------------------
// Cryptographic Minisign Verification
// ---------------------------------------------------------------------------

pub fn parse_minisign_pubkey(pubkey_str: &str) -> Result<PublicKey, String> {
    let trimmed = pubkey_str.trim();
    if trimmed.is_empty() {
        return Err("Открытый ключ подписи пуст".to_string());
    }

    // 1. Direct from base64 string (e.g. "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3")
    if let Ok(pk) = PublicKey::from_base64(trimmed) {
        return Ok(pk);
    }

    // 2. Base64-decoded string (Tauri sometimes base64-encodes the whole minisign.pub text)
    if let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        if let Ok(decoded_str) = std::str::from_utf8(&decoded_bytes) {
            let inner_trimmed = decoded_str.trim();
            if let Ok(pk) = PublicKey::decode(inner_trimmed) {
                return Ok(pk);
            }
            if let Ok(pk) = PublicKey::from_base64(inner_trimmed) {
                return Ok(pk);
            }
        }
    }

    // 3. Multi-line minisign.pub format
    PublicKey::decode(trimmed).map_err(|e| format!("Некорректный формат открытого ключа: {}", e))
}

pub fn parse_minisign_signature(sig_str: &str) -> Result<Signature, String> {
    let trimmed = sig_str.trim();
    if trimmed.is_empty() {
        return Err("Подпись обновления пуста".to_string());
    }

    // 1. Direct multi-line minisign signature format
    if let Ok(sig) = Signature::decode(trimmed) {
        return Ok(sig);
    }

    // 2. Base64-encoded signature string (standard in Tauri latest.json updater manifests)
    if let Ok(decoded_bytes) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
        if let Ok(decoded_str) = std::str::from_utf8(&decoded_bytes) {
            let inner_trimmed = decoded_str.trim();
            if let Ok(sig) = Signature::decode(inner_trimmed) {
                return Ok(sig);
            }
        }
    }

    Err("Некорректный формат цифровой подписи minisign".to_string())
}

pub fn verify_artifact_signature(
    data: &[u8],
    signature_str: &str,
    trusted_pubkey_str: &str,
) -> Result<(), String> {
    let pub_key = parse_minisign_pubkey(trusted_pubkey_str)?;
    let signature = parse_minisign_signature(signature_str)?;

    pub_key
        .verify(data, &signature, true)
        .map_err(|e| format!("Неверная цифровая подпись или повреждённые байты: {}", e))
}

pub fn resolve_trusted_public_key(
    explicit_key: Option<&str>,
    app: Option<&AppHandle>,
) -> Option<String> {
    resolve_trusted_public_key_internal(explicit_key, app, cfg!(debug_assertions))
}

pub fn resolve_trusted_public_key_internal(
    explicit_key: Option<&str>,
    app: Option<&AppHandle>,
    allow_debug_env: bool,
) -> Option<String> {
    if let Some(key) = explicit_key {
        let trimmed = key.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // Environment variables are strictly allowed only under cfg(debug_assertions)
    if allow_debug_env {
        if let Ok(key) =
            std::env::var("WEX_UPDATER_PUBKEY").or_else(|_| std::env::var("TAURI_UPDATER_PUBKEY"))
        {
            let trimmed = key.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    // In release builds, public key is sourced strictly from embedded binary config in AppHandle
    if let Some(handle) = app {
        if let Some(plugin_val) = handle.config().plugins.0.get("updater") {
            if let Some(pubkey) = plugin_val.get("pubkey").and_then(|v| v.as_str()) {
                let trimmed = pubkey.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }

    // Security hardening: reading tauri.conf.json from disk is completely removed to prevent key injection
    None
}

// ---------------------------------------------------------------------------
// Installer Preparation & Execution Plan
// ---------------------------------------------------------------------------

pub fn extract_installer_archive_if_needed(
    target_file: &Path,
    extract_dir: &Path,
) -> Result<PathBuf, String> {
    let ext = target_file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let is_zip = ext.eq_ignore_ascii_case("zip");

    if is_zip {
        let file = std::fs::File::open(target_file)
            .map_err(|e| format!("Не удалось открыть архив обновления: {}", e))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Не удалось распаковать zip-архив обновления: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Ошибка чтения файла из архива: {}", e))?;
            let outpath = extract_dir.join(file.mangled_name());

            if file.is_dir() {
                std::fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
                    }
                }
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| format!("Не удалось создать извлечённый файл: {}", e))?;
                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Не удалось записать извлечённый файл: {}", e))?;
            }
        }

        // Find installer executable in extract_dir
        if let Ok(entries) = std::fs::read_dir(extract_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                    if ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("msi") {
                        return Ok(p);
                    }
                }
            }
        }

        Err("В архиве обновления не обнаружен установщик .exe или .msi".to_string())
    } else {
        Ok(target_file.to_path_buf())
    }
}

pub fn build_installer_plan(
    installer_path: &Path,
    target_version: &str,
) -> Result<InstallerExecutionPlan, String> {
    validate_update_version(target_version)?;
    if !installer_path.exists() {
        return Err(format!(
            "Файл установщика не существует: {:?}",
            installer_path
        ));
    }

    let ext = installer_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if ext.eq_ignore_ascii_case("msi") {
        Ok(InstallerExecutionPlan {
            executable: PathBuf::from("msiexec.exe"),
            args: vec![
                "/i".to_string(),
                installer_path.to_string_lossy().to_string(),
                "/passive".to_string(),
                "/norestart".to_string(),
            ],
            target_version: target_version.to_string(),
            installer_path: installer_path.to_path_buf(),
        })
    } else {
        // NSIS .exe installer
        Ok(InstallerExecutionPlan {
            executable: installer_path.to_path_buf(),
            args: vec!["/UPDATE".to_string(), "/P".to_string()],
            target_version: target_version.to_string(),
            installer_path: installer_path.to_path_buf(),
        })
    }
}

pub fn execute_installer_plan_with_handler<F>(
    plan: &InstallerExecutionPlan,
    handler: F,
) -> Result<(), String>
where
    F: FnOnce(&Path, &[String]) -> Result<(), String>,
{
    handler(&plan.executable, &plan.args)
}

pub fn check_update_restart_allowed(is_running: bool, is_launching: bool) -> Result<(), String> {
    if is_running || is_launching {
        Err("Нельзя применить обновление: игра запущена или запускается. Закройте игру перед перезапуском лаунчера.".to_string())
    } else {
        Ok(())
    }
}

pub fn apply_staged_update_and_restart(
    app: Option<&AppHandle>,
) -> Result<InstallerExecutionPlan, String> {
    let _operation = UpdateOperation::begin()?;

    // Reject update restart if user game is currently running or launching
    if let Some(h) = app {
        if let Some(play_state) = h.try_state::<PlayState>() {
            check_update_restart_allowed(
                play_state.is_running.load(Ordering::SeqCst),
                play_state.is_launching.load(Ordering::SeqCst),
            )?;
            // Safely stop background prewarm before restarting into the installer
            tauri::async_runtime::block_on(play_state.cleanup_prewarm());
        }
    }

    // Take once: two restart requests cannot execute the same staged package.
    let staged = STAGED_UPDATE
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .ok_or_else(|| "Нет подготовленного проверенного обновления для установки".to_string())?;
    let plan = staged.execute(|executable, args| {
        #[cfg(target_os = "windows")]
        std::process::Command::new(executable)
            .args(args)
            .spawn()
            .map_err(|e| format!("Не удалось запустить установщик обновления: {}", e))?;
        #[cfg(not(target_os = "windows"))]
        let _ = (executable, args);
        Ok(())
    })?;
    staged.preserve_for_installer();
    #[cfg(target_os = "windows")]
    {
        if let Some(h) = app {
            h.exit(0);
        } else {
            std::process::exit(0);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Some(h) = app {
            h.restart();
        }
    }

    Ok(plan)
}

// ---------------------------------------------------------------------------
// Pure Business Logic (Unit-testable without Tauri runtime)
// ---------------------------------------------------------------------------

pub async fn check_for_updates_internal(
    client: &Client,
    current_version: &str,
    manifest_url: Option<&str>,
    target_platform: Option<&str>,
    trusted_pubkey: Option<&str>,
) -> Result<LauncherUpdateInfoDto, String> {
    let url = manifest_url.unwrap_or(DEFAULT_UPDATE_MANIFEST_URL);
    let plat = target_platform.unwrap_or(DEFAULT_TARGET_PLATFORM);

    let res = check_update_from_url(client, current_version, plat, url)
        .await
        .map_err(|e| e.to_string())?;

    let has_key = trusted_pubkey
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    let has_sig = res
        .signature
        .as_deref()
        .map(validate_signature_format)
        .unwrap_or(false);

    let is_signed = has_sig && has_key;
    let can_auto_update = res.update_available && is_signed;

    Ok(LauncherUpdateInfoDto {
        update_available: res.update_available,
        current_version: res.current_version,
        latest_version: res.latest_version,
        notes: res.notes,
        pub_date: res.pub_date,
        download_url: res.download_url,
        signature: res.signature,
        is_signed,
        can_auto_update,
    })
}

pub fn check_from_manifest_string_with_key(
    current_version: &str,
    target_platform: &str,
    manifest_json: &str,
    trusted_pubkey: Option<&str>,
) -> Result<LauncherUpdateInfoDto, String> {
    let res = check_update_from_manifest(current_version, target_platform, manifest_json)
        .map_err(|e| e.to_string())?;

    let has_key = trusted_pubkey
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    let has_sig = res
        .signature
        .as_deref()
        .map(validate_signature_format)
        .unwrap_or(false);

    let is_signed = has_sig && has_key;
    let can_auto_update = res.update_available && is_signed;

    Ok(LauncherUpdateInfoDto {
        update_available: res.update_available,
        current_version: res.current_version,
        latest_version: res.latest_version,
        notes: res.notes,
        pub_date: res.pub_date,
        download_url: res.download_url,
        signature: res.signature,
        is_signed,
        can_auto_update,
    })
}

pub async fn download_and_install_internal(
    app: Option<&AppHandle>,
    client: &Client,
    download_url: &str,
    signature: Option<&str>,
    version: &str,
    trusted_pubkey: Option<&str>,
    is_game_running: bool,
) -> Result<UpdateInstallStatusDto, String> {
    // 1. Guard against running game
    if is_game_running {
        return Err("Игра сейчас запущена. Закройте игру перед обновлением лаунчера.".to_string());
    }
    validate_update_version(version)?;

    // 2. Guard against missing trusted public key (Architect requirement: без ключа только ссылка)
    let pubkey_str = match trusted_pubkey {
        Some(k) if !k.trim().is_empty() => k.trim(),
        _ => {
            return Err("Автоматическая установка недоступна: ключ проверки подписи не настроен. Доступен режим «только уведомить со ссылкой».".to_string());
        }
    };

    // 3. Guard against unsigned releases
    let sig_str = match signature {
        Some(s) if validate_signature_format(s) => s.trim(),
        _ => {
            return Err("Автоматическая установка недоступна для неподписанной сборки. Воспользуйтесь ссылкой для загрузки.".to_string());
        }
    };

    // Use only a supported extension from a parsed URL; never a raw basename.
    let url = reqwest::Url::parse(download_url).map_err(|e| e.to_string())?;
    let extension = Path::new(url.path())
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "exe" | "msi" | "zip") {
        return Err("Неподдерживаемый формат установщика".into());
    }

    // 5. Download file with progress reporting
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

    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Ошибка сети при загрузке обновления: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Сервер вернул ошибку при загрузке обновления: HTTP {}",
            response.status()
        ));
    }

    let total_size = response.content_length();
    let mut downloaded: u64 = 0;
    let mut downloaded_bytes = Vec::new();

    loop {
        let chunk_opt = match response.chunk().await {
            Ok(c) => c,
            Err(e) => {
                return Err(format!("Ошибка загрузки фрагмента обновления: {}", e));
            }
        };

        match chunk_opt {
            Some(chunk) => {
                if downloaded_bytes.len().saturating_add(chunk.len()) > 512 * 1024 * 1024 {
                    return Err("Файл обновления слишком большой".into());
                }
                downloaded_bytes.extend_from_slice(&chunk);
                downloaded += chunk.len() as u64;
                let pct = if let Some(tot) = total_size {
                    if tot > 0 {
                        ((downloaded as f64 / tot as f64) * 100.0).min(90.0) as u32
                    } else {
                        45
                    }
                } else {
                    45
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

    emit_progress(
        92,
        "Проверка криптографической подписи minisign…",
        downloaded,
        total_size,
    );

    // 6. CRYPTOGRAPHIC SIGNATURE VERIFICATION WITH TRUSTED PUBLIC KEY
    if let Err(err_msg) = verify_artifact_signature(&downloaded_bytes, sig_str, pubkey_str) {
        // No temporary filesystem state exists yet; network/signature failures delete nothing.
        return Err(format!(
            "Ошибка проверки цифровой подписи (подмена или повреждение файлов): {}. Прежняя версия сохранена.",
            err_msg
        ));
    }

    // Own a random TempDir and retain signed-package verification metadata until restart.
    set_staged_update(StagedUpdate::new(
        &downloaded_bytes,
        sig_str,
        pubkey_str,
        version,
        &extension,
    )?);

    emit_progress(
        100,
        "Обновление проверено цифровой подписью и готово к установке",
        downloaded,
        total_size,
    );

    Ok(UpdateInstallStatusDto {
        status: "ready_to_restart".to_string(),
        message: format!(
            "Обновление до версии {} проверено цифровой подписью и готово к установке",
            version
        ),
        version: Some(version.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Tauri Command Handlers
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn updater_check(
    app: AppHandle,
    manifest_url: Option<String>,
) -> Result<LauncherUpdateInfoDto, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;

    let cur_version = env!("CARGO_PKG_VERSION");
    let trusted_key = resolve_trusted_public_key(None, Some(&app));

    check_for_updates_internal(
        &client,
        cur_version,
        manifest_url.as_deref(),
        Some(DEFAULT_TARGET_PLATFORM),
        trusted_key.as_deref(),
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
    let _operation = UpdateOperation::begin()?;
    clear_staged_update();
    let is_running = state.is_running.load(Ordering::SeqCst);
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let trusted_key = resolve_trusted_public_key(None, Some(&app));

    download_and_install_internal(
        Some(&app),
        &client,
        &download_url,
        signature.as_deref(),
        &version,
        trusted_key.as_deref(),
        is_running,
    )
    .await
}

#[tauri::command]
pub async fn updater_restart(app: AppHandle, state: State<'_, PlayState>) -> Result<(), String> {
    check_update_restart_allowed(
        state.is_running.load(Ordering::SeqCst),
        state.is_launching.load(Ordering::SeqCst),
    )?;
    state.cleanup_prewarm().await;
    let _ = apply_staged_update_and_restart(Some(&app))?;
    Ok(())
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
    use std::io::Write;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Valid Minisign keypair and signature for data b"test"
    const TEST_PUBKEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const TEST_DATA: &[u8] = b"test";
    const TEST_VALID_SIG: &str = "untrusted comment: signature from minisign secret key\nRWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=\ntrusted comment: timestamp:1555779966\tfile:test\nQtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";

    #[test]
    fn security_rejects_invalid_update_versions() {
        let dir = tempfile::tempdir().unwrap();
        let installer = dir.path().join("setup.exe");
        std::fs::write(&installer, TEST_DATA).unwrap();
        for version in [
            "../outside",
            "1.0.0\\..\\outside",
            "C:\\outside",
            "not-semver",
            "1.0",
            "0.0.0",
        ] {
            assert!(
                build_installer_plan(&installer, version).is_err(),
                "accepted {version}"
            );
        }
    }

    #[test]
    fn security_rejects_replay_even_with_build_metadata() {
        let current = env!("CARGO_PKG_VERSION");
        assert!(validate_update_version(current).is_err());
        assert!(validate_update_version(&format!("{current}+replay")).is_err());
    }

    #[test]
    fn security_serializes_install_and_restart_operations() {
        let operation = UpdateOperation::begin().unwrap();
        assert!(UpdateOperation::begin().is_err());
        drop(operation);
        assert!(UpdateOperation::begin().is_ok());
    }

    #[tokio::test]
    async fn security_malformed_version_cannot_delete_external_sentinel() {
        let external = tempfile::Builder::new()
            .prefix("c3-update-sentinel-")
            .tempdir()
            .unwrap();
        let sentinel = external.path().join("sentinel");
        std::fs::write(&sentinel, b"keep this unrelated directory").unwrap();
        let name = external.path().file_name().unwrap().to_str().unwrap();
        let probe = format!("c3-probe-{name}");
        let version = format!("{probe}/../../{name}");
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/setup.exe"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"tampered".to_vec()))
            .mount(&server)
            .await;
        let result = download_and_install_internal(
            None,
            &Client::new(),
            &format!("{}/setup.exe", server.uri()),
            Some(TEST_VALID_SIG),
            &version,
            Some(TEST_PUBKEY),
            false,
        )
        .await;
        // The old code creates this empty probe directory while traversing up to
        // the *owned test TempDir*. Never address or delete a user's directory.
        let legacy_probe = std::env::temp_dir()
            .join("AuraDev")
            .join(format!("wex-update-{probe}"));
        let _ = std::fs::remove_dir(legacy_probe);
        assert!(result.is_err());
        assert_eq!(
            std::fs::read(&sentinel).ok().as_deref(),
            Some(b"keep this unrelated directory".as_slice())
        );
    }

    #[test]
    fn test_verify_signature_valid() {
        let res = verify_artifact_signature(TEST_DATA, TEST_VALID_SIG, TEST_PUBKEY);
        assert!(res.is_ok(), "Valid signature must verify successfully");
    }

    #[test]
    fn test_verify_signature_tampered_bytes_fails() {
        // Tampered bytes: 1 byte changed
        let tampered_data = b"Test";
        let res = verify_artifact_signature(tampered_data, TEST_VALID_SIG, TEST_PUBKEY);
        assert!(res.is_err(), "Verification must fail on tampered bytes");
        let err = res.unwrap_err();
        assert!(
            err.contains("Неверная цифровая подпись") || err.contains("повреждённые байты"),
            "Error message should mention signature or corrupted bytes: {}",
            err
        );
    }

    #[test]
    fn test_verify_invalid_signature_string_fails() {
        let invalid_sig = "untrusted comment: invalid\nnot-valid-base64-content";
        let res = verify_artifact_signature(TEST_DATA, invalid_sig, TEST_PUBKEY);
        assert!(
            res.is_err(),
            "Verification must fail on invalid signature string"
        );
    }

    #[test]
    fn test_missing_public_key_fallback_to_notify_only() {
        let manifest = r#"{
            "version": "1.0.0",
            "notes": "Важное обновление",
            "platforms": {
                "windows-x86_64": {
                    "signature": "untrusted comment: sig\nRWSomeBase64KeyData1234567890==",
                    "url": "https://example.com/wex-update.exe"
                }
            }
        }"#;

        // No trusted public key configured -> can_auto_update MUST be false, notify-only
        let res =
            check_from_manifest_string_with_key("0.1.0", "windows-x86_64", manifest, None).unwrap();
        assert!(res.update_available);
        assert!(
            !res.is_signed,
            "Without trusted pubkey, release is not treated as signed"
        );
        assert!(
            !res.can_auto_update,
            "Without trusted pubkey, auto-update must be disabled (notify-only)"
        );

        // Empty trusted public key -> also notify-only
        let res_empty =
            check_from_manifest_string_with_key("0.1.0", "windows-x86_64", manifest, Some("   "))
                .unwrap();
        assert!(!res_empty.can_auto_update);

        // When trusted public key is provided -> auto-update is enabled
        let res_with_key = check_from_manifest_string_with_key(
            "0.1.0",
            "windows-x86_64",
            manifest,
            Some(TEST_PUBKEY),
        )
        .unwrap();
        assert!(res_with_key.can_auto_update);
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

        let res = check_from_manifest_string_with_key(
            "0.1.0",
            "windows-x86_64",
            manifest,
            Some(TEST_PUBKEY),
        )
        .unwrap();
        assert!(res.update_available);
        assert!(!res.is_signed);
        assert!(!res.can_auto_update);
        assert_eq!(
            res.download_url.as_deref(),
            Some("https://example.com/wex-unsigned.exe")
        );
    }

    #[tokio::test]
    async fn test_download_and_install_blocked_when_game_running() {
        let client = Client::new();
        let err = download_and_install_internal(
            None,
            &client,
            "https://example.com/update.exe",
            Some(TEST_VALID_SIG),
            "1.0.0",
            Some(TEST_PUBKEY),
            true, // game is running!
        )
        .await
        .unwrap_err();

        assert!(err.contains("Игра сейчас запущена"));
    }

    #[test]
    fn test_updater_restart_blocked_when_game_running_or_launching() {
        let running_err = check_update_restart_allowed(true, false).unwrap_err();
        assert!(running_err.contains("игра запущена"));

        let launching_err = check_update_restart_allowed(false, true).unwrap_err();
        assert!(launching_err.contains("игра запущена"));

        let both_err = check_update_restart_allowed(true, true).unwrap_err();
        assert!(both_err.contains("игра запущена"));

        assert!(check_update_restart_allowed(false, false).is_ok());
    }

    #[tokio::test]
    async fn test_download_and_install_blocked_when_no_trusted_pubkey() {
        let client = Client::new();
        let err = download_and_install_internal(
            None,
            &client,
            "https://example.com/update.exe",
            Some(TEST_VALID_SIG),
            "1.0.0",
            None, // missing trusted public key!
            false,
        )
        .await
        .unwrap_err();

        assert!(err.contains("ключ проверки подписи не настроен"));
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
            Some(TEST_PUBKEY),
            false,
        )
        .await
        .unwrap_err();

        assert!(err.contains("Автоматическая установка недоступна для неподписанной сборки"));
    }

    #[tokio::test]
    async fn test_download_and_install_rollback_on_tampered_bytes() {
        let mock_server = MockServer::start().await;
        // Server sends tampered bytes instead of authentic TEST_DATA
        let tampered_bytes = b"TAMPERED_MALICIOUS_DATA";

        Mock::given(method("GET"))
            .and(path("/setup.exe"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(tampered_bytes.to_vec()))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let url = format!("{}/setup.exe", mock_server.uri());

        let err = download_and_install_internal(
            None,
            &client,
            &url,
            Some(TEST_VALID_SIG),
            "9.9.9-tampered",
            Some(TEST_PUBKEY),
            false,
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("Ошибка проверки цифровой подписи")
                && err.contains("Прежняя версия сохранена"),
            "Error should indicate signature mismatch and rollback: {}",
            err
        );

        // Verify rollback: temp directory cleaned up
        let temp_dir = std::env::temp_dir()
            .join("AuraDev")
            .join("wex-update-9.9.9-tampered");
        assert!(
            !temp_dir.exists(),
            "Temp directory must be cleaned up on verification failure"
        );
    }

    #[tokio::test]
    async fn test_download_and_install_success_and_stages_installer() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/setup.exe"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(TEST_DATA.to_vec()))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let url = format!("{}/setup.exe", mock_server.uri());

        let res = download_and_install_internal(
            None,
            &client,
            &url,
            Some(TEST_VALID_SIG),
            "2.0.0-verified",
            Some(TEST_PUBKEY),
            false,
        )
        .await
        .unwrap();

        assert_eq!(res.status, "ready_to_restart");
        assert_eq!(res.version.as_deref(), Some("2.0.0-verified"));

        // Verify staged installer exists
        let staged = get_staged_update().expect("Staged update must be present");
        assert_eq!(staged.version, "2.0.0-verified");
        assert!(staged.installer_path.exists());

        // Clean up
        let _ = std::fs::remove_dir_all(staged.temp_dir);
        clear_staged_update();
    }

    #[test]
    fn test_installer_execution_plan_and_version_replacement() {
        let temp_dir = std::env::temp_dir()
            .join("AuraDev")
            .join("test-installer-plan");
        let _ = std::fs::create_dir_all(&temp_dir);
        let exe_path = temp_dir.join("Wex_1.2.0_x64-setup.exe");
        std::fs::write(&exe_path, b"dummy nsis installer").unwrap();

        // 1. Plan creation
        let plan = build_installer_plan(&exe_path, "1.2.0").expect("Plan must be built");
        assert_eq!(plan.executable, exe_path);
        assert_eq!(plan.args, vec!["/UPDATE", "/P"]);
        assert_eq!(plan.target_version, "1.2.0");

        // 2. Execution through mock handler to verify command formation and version replacement
        let mut executed_cmd = None;
        let res = execute_installer_plan_with_handler(&plan, |exe, args| {
            executed_cmd = Some((exe.to_path_buf(), args.to_vec()));
            Ok(())
        });

        assert!(res.is_ok());
        let (executed_exe, executed_args) = executed_cmd.unwrap();
        assert_eq!(executed_exe, exe_path);
        assert!(executed_args.contains(&"/UPDATE".to_string()));
        assert!(executed_args.contains(&"/P".to_string()));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_zip_installer_extraction() {
        let temp_dir = std::env::temp_dir()
            .join("AuraDev")
            .join("test-zip-extract");
        let _ = std::fs::create_dir_all(&temp_dir);
        let zip_path = temp_dir.join("update.zip");

        // Create zip with Wex_1.5.0_x64-setup.exe inside
        {
            let zip_file = std::fs::File::create(&zip_path).unwrap();
            let mut zip = zip::ZipWriter::new(zip_file);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("Wex_1.5.0_x64-setup.exe", options).unwrap();
            zip.write_all(b"fake installer binary content").unwrap();
            zip.finish().unwrap();
        }

        let extract_dir = temp_dir.join("extracted");
        let extracted_installer = extract_installer_archive_if_needed(&zip_path, &extract_dir)
            .expect("Zip extraction must succeed and locate .exe");

        assert!(extracted_installer.exists());
        assert_eq!(
            extracted_installer.file_name().and_then(|n| n.to_str()),
            Some("Wex_1.5.0_x64-setup.exe")
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_disk_tauri_conf_with_foreign_key_is_not_used() {
        let fake_conf_path = std::env::current_dir()
            .unwrap()
            .join("tauri.conf.json.security-test");
        let fake_conf_content = r#"{
            "plugins": {
                "updater": {
                    "pubkey": "RWMaliciousAttackerInjectedKey1234567890=="
                }
            }
        }"#;
        std::fs::write(&fake_conf_path, fake_conf_content).unwrap();

        // Without AppHandle and without explicit key, resolve must return None
        // and MUST NOT read anything from disk!
        let key = resolve_trusted_public_key_internal(None, None, false);
        assert!(
            key.is_none(),
            "Key from disk file must never be used in resolution"
        );

        let _ = std::fs::remove_file(&fake_conf_path);
    }

    #[test]
    fn test_env_var_ignored_in_release_build() {
        let fake_env_key = "RWAttackerEnvKey123456789012345678901234==";
        std::env::set_var("WEX_UPDATER_PUBKEY", fake_env_key);

        // 1. In release mode (allow_debug_env = false), environment variables are strictly ignored
        let release_key = resolve_trusted_public_key_internal(None, None, false);
        assert!(
            release_key.is_none(),
            "Environment variable must be strictly ignored in release mode"
        );

        // 2. In debug assertions mode (allow_debug_env = true), environment variable is accepted
        let debug_key = resolve_trusted_public_key_internal(None, None, true);
        assert_eq!(
            debug_key.as_deref(),
            Some(fake_env_key),
            "Environment variable should be accepted when allow_debug_env is true"
        );

        std::env::remove_var("WEX_UPDATER_PUBKEY");
    }

    #[test]
    fn test_installer_hooks_nsis_script_safety_and_marker_verification() {
        let nsh_content = include_str!("../../installer-hooks.nsh");

        // Must display actual target directory ($2) in MessageBox prompt rather than hardcoded %APPDATA%\Wex
        assert!(
            nsh_content.contains(r#"MessageBox MB_YESNO|MB_DEFBUTTON2|MB_ICONQUESTION "Удалить пользовательские данные и настройки Wex ($2)?" IDNO skip_data_cleanup"#),
            "NSIS prompt must display the actual resolved target path ($2)"
        );

        // Must verify Wex ownership marker (config.json or instances directory) for custom paths
        assert!(
            nsh_content.contains(r#"IfFileExists "$1\config.json""#),
            "NSIS script must check for config.json marker in custom WEX_DATA_DIR"
        );
        assert!(
            nsh_content.contains(r#"IfFileExists "$1\instances\*.*""#),
            "NSIS script must check for instances marker in custom WEX_DATA_DIR"
        );

        // Must delete target $2 rather than arbitrary unverified $1
        assert!(
            nsh_content.contains("RMDir /r \"$2\""),
            "RMDir must only delete verified target $2"
        );
        assert!(
            !nsh_content.contains("RMDir /r \"$1\""),
            "RMDir must NEVER delete unverified raw WEX_DATA_DIR $1"
        );
    }
}
