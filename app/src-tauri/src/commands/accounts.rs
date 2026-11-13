//! Player accounts for the UI: list, offline sign-in, Microsoft device-code
//! sign-in, switching and signing out. All storage and token handling lives in
//! `wex_core::accounts`; this module only adapts it to Tauri commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::Serialize;
use tauri::State;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use wex_core::accounts::{self, Account, AccountKind, AccountStore, DeviceCode, MsAuth};
use wex_core::config::LauncherConfig;
use wex_core::WexError;
use wex_ipc::error::IpcError;

use super::launch::PlayState;

/// Device-code sign-ins waiting for the player, by flow id.
#[derive(Default, Clone)]
pub struct AccountsState {
    flows: Arc<Mutex<HashMap<String, PendingFlow>>>,
}

#[derive(Clone)]
struct PendingFlow {
    code: DeviceCode,
    cancel: CancellationToken,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AccountDto {
    pub id: String,
    /// "offline" or "microsoft".
    pub kind: String,
    pub name: String,
    pub uuid: String,
    pub needs_sign_in: bool,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AccountsOverview {
    pub accounts: Vec<AccountDto>,
    pub active_id: Option<String>,
    /// False when no Microsoft client ID is configured: the UI shows the method as unavailable.
    pub microsoft_available: bool,
    /// How tokens are protected on this system ("dpapi-user" on Windows).
    pub token_storage: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeviceCodeDto {
    pub flow_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
}

fn dto(a: &Account) -> AccountDto {
    AccountDto {
        id: a.id.clone(),
        kind: match a.kind {
            AccountKind::Offline => "offline",
            AccountKind::Microsoft => "microsoft",
        }
        .to_string(),
        name: a.name.clone(),
        uuid: a.uuid.clone(),
        needs_sign_in: a.needs_sign_in,
        last_used_at: a.last_used_at.map(|t| t.to_rfc3339()),
    }
}

fn err(e: WexError) -> IpcError {
    e.into()
}

fn client_id(config_file: &std::path::Path) -> Option<String> {
    accounts::microsoft_client_id(&LauncherConfig::load_or_default(config_file))
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("WexLauncher")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

/// The overview for a data root; also migrates an old install's nickname into an offline account.
pub fn overview_at(
    root: &std::path::Path,
    config_file: &std::path::Path,
) -> Result<AccountsOverview, IpcError> {
    let mut store = AccountStore::open(root).map_err(err)?;
    if store.accounts().is_empty() {
        let cfg = LauncherConfig::load_or_default(config_file);
        if accounts::is_valid_offline_name(cfg.nickname.trim()) {
            store.add_offline(cfg.nickname.trim()).map_err(err)?;
        }
    }
    Ok(AccountsOverview {
        accounts: store.accounts().iter().map(dto).collect(),
        active_id: store.active_id().map(str::to_string),
        microsoft_available: client_id(config_file).is_some(),
        token_storage: store.cipher_name().to_string(),
    })
}

fn root_and_config(state: &PlayState) -> (PathBuf, PathBuf) {
    (state.paths.root.clone(), state.paths.config_file.clone())
}

/// After the active account changed: the config nickname follows it and a
/// prewarmed game (started for the previous player) is stopped.
async fn after_switch(state: &PlayState, active: Option<&Account>) -> Result<(), IpcError> {
    accounts::sync_config_nickname(&state.paths.config_file, active).map_err(err)?;
    state.cleanup_prewarm().await;
    Ok(())
}

#[tauri::command]
pub async fn accounts_list(state: State<'_, PlayState>) -> Result<AccountsOverview, IpcError> {
    let (root, cfg) = root_and_config(&state);
    overview_at(&root, &cfg)
}

#[tauri::command]
pub async fn accounts_add_offline(
    name: String,
    state: State<'_, PlayState>,
) -> Result<AccountsOverview, IpcError> {
    let (root, cfg) = root_and_config(&state);
    let acc = AccountStore::open(&root)
        .and_then(|mut s| s.add_offline(&name))
        .map_err(err)?;
    after_switch(&state, Some(&acc)).await?;
    overview_at(&root, &cfg)
}

#[tauri::command]
pub async fn accounts_switch(
    id: String,
    state: State<'_, PlayState>,
) -> Result<AccountsOverview, IpcError> {
    let (root, cfg) = root_and_config(&state);
    let acc = AccountStore::open(&root)
        .and_then(|mut s| s.switch(&id))
        .map_err(err)?;
    after_switch(&state, Some(&acc)).await?;
    overview_at(&root, &cfg)
}

/// Remove the account and delete its tokens from this computer.
#[tauri::command]
pub async fn accounts_sign_out(
    id: String,
    state: State<'_, PlayState>,
) -> Result<AccountsOverview, IpcError> {
    let (root, cfg) = root_and_config(&state);
    let mut store = AccountStore::open(&root).map_err(err)?;
    let was_active = store.active_id() == Some(id.as_str());
    let next = store.sign_out(&id).map_err(err)?;
    if was_active {
        after_switch(&state, next.as_ref()).await?;
    }
    Ok(AccountsOverview {
        accounts: store.accounts().iter().map(dto).collect(),
        active_id: store.active_id().map(str::to_string),
        microsoft_available: client_id(&cfg).is_some(),
        token_storage: store.cipher_name().to_string(),
    })
}

fn unavailable() -> IpcError {
    IpcError::new(
        "AUTH_MS_UNAVAILABLE",
        "Вход Microsoft недоступен в этой сборке: не задан ID приложения (WEX_MS_CLIENT_ID).",
        None,
    )
}

/// Start a Microsoft sign-in: returns the code the player enters on the Microsoft page.
#[tauri::command]
pub async fn accounts_ms_begin(
    state: State<'_, PlayState>,
    accounts_state: State<'_, AccountsState>,
) -> Result<DeviceCodeDto, IpcError> {
    let id = client_id(&state.paths.config_file).ok_or_else(unavailable)?;
    let auth = MsAuth::new(http_client(), id);
    let code = auth.start_device_code().await.map_err(err)?;
    let flow_id = uuid::Uuid::new_v4().to_string();
    let dto = DeviceCodeDto {
        flow_id: flow_id.clone(),
        user_code: code.user_code.clone(),
        verification_uri: code.verification_uri.clone(),
        expires_in: code.expires_in,
    };
    let mut flows = accounts_state.flows.lock().await;
    // One sign-in at a time: an older code is cancelled.
    for f in flows.values() {
        f.cancel.cancel();
    }
    flows.clear();
    flows.insert(
        flow_id,
        PendingFlow {
            code,
            cancel: CancellationToken::new(),
        },
    );
    Ok(dto)
}

/// Wait until the player finishes on the Microsoft page, then finish the chain and save the account.
#[tauri::command]
pub async fn accounts_ms_finish(
    flow_id: String,
    state: State<'_, PlayState>,
    accounts_state: State<'_, AccountsState>,
) -> Result<AccountsOverview, IpcError> {
    let flow = accounts_state
        .flows
        .lock()
        .await
        .get(&flow_id)
        .cloned()
        .ok_or_else(|| {
            IpcError::new(
                "AUTH_CODE_EXPIRED",
                "Этот вход уже завершён или отменён. Начните заново.",
                None,
            )
        })?;
    let (root, cfg) = root_and_config(&state);
    let id = client_id(&cfg).ok_or_else(unavailable)?;
    let auth = MsAuth::new(http_client(), id);

    let result = async {
        let tokens = auth.wait_for_device_code(&flow.code, &flow.cancel).await?;
        let login = auth.complete(tokens).await?;
        let mut store = AccountStore::open(&root)?;
        accounts::save_microsoft_login(&mut store, &login)
    }
    .await;
    accounts_state.flows.lock().await.remove(&flow_id);

    let acc = result.map_err(|e| match e {
        WexError::Cancelled => IpcError::new("AUTH_CANCELLED", "Вход отменён.", None),
        other => err(other),
    })?;
    after_switch(&state, Some(&acc)).await?;
    overview_at(&root, &cfg)
}

#[tauri::command]
pub async fn accounts_ms_cancel(
    flow_id: String,
    accounts_state: State<'_, AccountsState>,
) -> Result<(), IpcError> {
    if let Some(f) = accounts_state.flows.lock().await.remove(&flow_id) {
        f.cancel.cancel();
    }
    Ok(())
}

/// Only Microsoft's own pages may be opened from a sign-in flow.
pub fn is_microsoft_verification_url(url: &str) -> bool {
    match reqwest::Url::parse(url) {
        Ok(u) => {
            u.scheme() == "https"
                && u.username().is_empty()
                && u.password().is_none()
                && matches!(
                    u.host_str(),
                    Some("www.microsoft.com")
                        | Some("microsoft.com")
                        | Some("login.microsoftonline.com")
                        | Some("login.live.com")
                )
        }
        Err(_) => false,
    }
}

/// Open the Microsoft verification page of a pending sign-in in the default browser.
#[tauri::command]
pub async fn accounts_ms_open_page(
    flow_id: String,
    accounts_state: State<'_, AccountsState>,
) -> Result<(), IpcError> {
    let url = accounts_state
        .flows
        .lock()
        .await
        .get(&flow_id)
        .map(|f| f.code.verification_uri.clone())
        .ok_or_else(|| {
            IpcError::new(
                "AUTH_CODE_EXPIRED",
                "Этот вход уже завершён или отменён. Начните заново.",
                None,
            )
        })?;
    if !is_microsoft_verification_url(&url) {
        return Err(IpcError::new(
            "AUTH_MS_FAILED",
            "Microsoft вернул неожиданную ссылку для входа.",
            Some(url),
        ));
    }
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| {
        IpcError::new(
            "AUTH_MS_FAILED",
            "Не удалось открыть браузер. Откройте ссылку вручную.",
            Some(e.to_string()),
        )
    })
}

/// Launcher start: refresh the active Microsoft account in the background so a
/// rotated refresh token is saved. Failures only go to the log; launch reports them.
pub fn spawn_startup_refresh(root: PathBuf, config_file: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let cfg = LauncherConfig::load_or_default(&config_file);
        match accounts::refresh_active_if_needed(&root, &http_client(), &cfg).await {
            Ok(true) => tracing::info!("accounts: active Microsoft account is signed in"),
            Ok(false) => {}
            Err(e) => tracing::warn!("accounts: startup refresh failed: {}", e),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn overview_migrates_nickname_and_reports_storage() {
        let tmp = tempdir().unwrap();
        let cfg_file = tmp.path().join("config.json");
        LauncherConfig {
            nickname: "OldGamer".into(),
            ..Default::default()
        }
        .save(&cfg_file)
        .unwrap();
        let o = overview_at(tmp.path(), &cfg_file).unwrap();
        assert_eq!(o.accounts.len(), 1);
        assert_eq!(o.accounts[0].kind, "offline");
        assert_eq!(o.accounts[0].name, "OldGamer");
        assert_eq!(o.active_id.as_deref(), Some(o.accounts[0].id.as_str()));
        #[cfg(windows)]
        assert_eq!(o.token_storage, "dpapi-user");
        if std::env::var(accounts::CLIENT_ID_ENV).is_err()
            && option_env!("WEX_MS_CLIENT_ID").is_none()
        {
            assert!(
                !o.microsoft_available,
                "no client id: Microsoft sign-in is shown unavailable"
            );
        }
    }

    #[test]
    fn switching_updates_config_nickname() {
        let tmp = tempdir().unwrap();
        let cfg_file = tmp.path().join("config.json");
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let a = store.add_offline("FirstOne").unwrap();
        accounts::sync_config_nickname(&cfg_file, Some(&a)).unwrap();
        assert_eq!(
            LauncherConfig::load_or_default(&cfg_file).nickname,
            "FirstOne"
        );
    }

    #[test]
    fn only_microsoft_pages_can_be_opened() {
        assert!(is_microsoft_verification_url(
            "https://www.microsoft.com/link"
        ));
        assert!(is_microsoft_verification_url(
            "https://login.microsoftonline.com/common/oauth2/deviceauth"
        ));
        assert!(!is_microsoft_verification_url(
            "http://www.microsoft.com/link"
        ));
        assert!(!is_microsoft_verification_url(
            "https://microsoft.com.evil.example/link"
        ));
        assert!(!is_microsoft_verification_url(
            "https://user@www.microsoft.com/link"
        ));
        assert!(!is_microsoft_verification_url("javascript:alert(1)"));
    }
}
