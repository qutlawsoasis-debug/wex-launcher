//! Player accounts: Microsoft (licensed Minecraft) and offline (nickname only).
//!
//! - `store`: saved accounts in the data root, tokens sealed with DPAPI on Windows.
//! - `microsoft`: device code sign-in and the Xbox Live → XSTS → Minecraft chain.
//! - `resolve_session`: the identity the game is launched with; refreshes
//!   Microsoft tokens on its own when they are about to expire.

pub mod microsoft;
pub mod secret;
pub mod store;

pub use microsoft::{DeviceCode, MinecraftLogin, MsAuth, MsEndpoints, MsTokens};
pub use store::{is_valid_offline_name, Account, AccountKind, AccountStore, MsSecrets};

use crate::auth::offline_uuid;
use crate::config::LauncherConfig;
use crate::error::{Result, WexError};
use chrono::{Duration, Utc};
use std::path::Path;

/// Environment variable with the Azure application (client) ID for Microsoft sign-in.
pub const CLIENT_ID_ENV: &str = "WEX_MS_CLIENT_ID";

/// Tokens closer than this to expiry are refreshed before launch.
const REFRESH_MARGIN_MINUTES: i64 = 10;

/// Serialises token refresh so two launches never rotate the refresh token at once.
static REFRESH_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The identity passed to Minecraft on the command line.
#[derive(Clone, PartialEq, Eq)]
pub struct GameSession {
    pub username: String,
    /// UUID as Minecraft expects it (with or without dashes).
    pub uuid: String,
    pub access_token: String,
    /// `msa` for Microsoft accounts, `legacy` for offline.
    pub user_type: String,
    pub xuid: Option<String>,
}

impl std::fmt::Debug for GameSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameSession")
            .field("username", &self.username)
            .field("uuid", &self.uuid)
            .field("access_token", &"***")
            .field("user_type", &self.user_type)
            .finish()
    }
}

impl GameSession {
    /// Vanilla offline identity: UUID from `OfflinePlayer:<name>`, token `0`.
    pub fn offline(name: &str) -> Self {
        Self {
            username: name.to_string(),
            uuid: offline_uuid(name).to_string(),
            access_token: "0".to_string(),
            user_type: "legacy".to_string(),
            xuid: None,
        }
    }

    pub fn is_microsoft(&self) -> bool {
        self.user_type == "msa"
    }
}

/// Client ID from the environment, then the user's config, then the build
/// environment (a release build may bake it in from CI; it is never in the repo).
pub fn microsoft_client_id(cfg: &LauncherConfig) -> Option<String> {
    let pick = |v: Option<String>| v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    pick(std::env::var(CLIENT_ID_ENV).ok())
        .or_else(|| pick(cfg.microsoft_client_id.clone()))
        .or_else(|| pick(option_env!("WEX_MS_CLIENT_ID").map(str::to_string)))
}

/// Save a finished Microsoft sign-in and make it the active account.
pub fn save_microsoft_login(store: &mut AccountStore, login: &MinecraftLogin) -> Result<Account> {
    store.upsert_microsoft(
        &login.uuid,
        &login.name,
        &MsSecrets {
            refresh_token: login.refresh_token.clone(),
            mc_access_token: login.access_token.clone(),
            mc_expires_at: login.expires_at,
            xuid: login.xuid.clone(),
        },
    )
}

/// Keep `config.nickname` equal to the active account's name, so every part of
/// the launcher that shows or compares the nickname sees the same player.
pub fn sync_config_nickname(config_file: &Path, account: Option<&Account>) -> Result<()> {
    if let Some(acc) = account {
        let mut cfg = LauncherConfig::load_or_default(config_file);
        if cfg.nickname != acc.name {
            cfg.nickname = acc.name.clone();
            cfg.save(config_file)?;
        }
    }
    Ok(())
}

/// Build the launch identity from the active account.
///
/// - No accounts yet (older installs): an offline account is created from
///   `config.nickname`.
/// - Active offline account: `config.nickname` is the name (the player edits it in
///   the wardrobe); the account follows it.
/// - Active Microsoft account: the Minecraft token is refreshed when it is about to
///   expire. A refresh rejected by Microsoft marks the account and asks to sign in again.
pub async fn resolve_session(
    root: &Path,
    client: &reqwest::Client,
    cfg: &LauncherConfig,
) -> Result<GameSession> {
    let auth = microsoft_client_id(cfg).map(|id| MsAuth::new(client.clone(), id));
    let mut store = AccountStore::open(root)?;
    resolve_session_with(&mut store, auth.as_ref(), cfg).await
}

pub async fn resolve_session_with(
    store: &mut AccountStore,
    auth: Option<&MsAuth>,
    cfg: &LauncherConfig,
) -> Result<GameSession> {
    let active = match store.active().cloned() {
        Some(a) => a,
        None => {
            let name = if is_valid_offline_name(cfg.nickname.trim()) {
                cfg.nickname.trim().to_string()
            } else {
                return Ok(GameSession::offline(&cfg.nickname));
            };
            store.add_offline(&name)?
        }
    };

    match active.kind {
        AccountKind::Offline => {
            let nick = cfg.nickname.trim();
            let acc = if nick != active.name && is_valid_offline_name(nick) {
                store.rename_offline(&active.id, nick)?
            } else {
                active
            };
            let _ = store.mark_used(&acc.id);
            Ok(GameSession::offline(&acc.name))
        }
        AccountKind::Microsoft => {
            let _guard = REFRESH_LOCK.lock().await;
            let mut secrets = store.read_secrets(&active.id)?;
            let mut name = active.name.clone();
            if secrets.mc_expires_at - Utc::now() < Duration::minutes(REFRESH_MARGIN_MINUTES) {
                let auth = auth.ok_or_else(|| WexError::Auth {
                    code: "AUTH_MS_UNAVAILABLE",
                    message:
                        "Вход Microsoft недоступен: не задан ID приложения (WEX_MS_CLIENT_ID)."
                            .to_string(),
                    details: None,
                })?;
                let login = match refresh_login(auth, &secrets.refresh_token).await {
                    Ok(l) => l,
                    Err(e) => {
                        if matches!(
                            e,
                            WexError::Auth {
                                code: "AUTH_SIGN_IN_AGAIN",
                                ..
                            }
                        ) {
                            let _ = store.mark_needs_sign_in(&active.id);
                        }
                        return Err(e);
                    }
                };
                if login.uuid != active.uuid {
                    return Err(WexError::Auth {
                        code: "AUTH_SIGN_IN_AGAIN",
                        message: "Сохранённый вход относится к другому профилю. Войдите в аккаунт ещё раз.".to_string(),
                        details: None,
                    });
                }
                save_microsoft_login(store, &login)?;
                secrets = store.read_secrets(&active.id)?;
                name = login.name;
            }
            let _ = store.mark_used(&active.id);
            Ok(GameSession {
                username: name,
                uuid: active.uuid.clone(),
                access_token: secrets.mc_access_token,
                user_type: "msa".to_string(),
                xuid: secrets.xuid,
            })
        }
    }
}

/// Refresh token → new Microsoft tokens → the whole Xbox / Minecraft chain.
pub async fn refresh_login(auth: &MsAuth, refresh_token: &str) -> Result<MinecraftLogin> {
    let tokens = auth.refresh(refresh_token).await?;
    auth.complete(tokens).await
}

/// Refresh the active Microsoft account in the background (launcher start), so
/// a rotated refresh token is saved even if the player does not launch the game.
/// Returns `Ok(false)` when there is nothing to refresh.
pub async fn refresh_active_if_needed(
    root: &Path,
    client: &reqwest::Client,
    cfg: &LauncherConfig,
) -> Result<bool> {
    let store = AccountStore::open(root)?;
    match store.active() {
        Some(a) if a.kind == AccountKind::Microsoft && !a.needs_sign_in => {}
        _ => return Ok(false),
    }
    drop(store);
    resolve_session(root, client, cfg).await.map(|_| true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn offline_session_matches_vanilla() {
        let s = GameSession::offline("Player");
        assert_eq!(s.uuid, "a01e3843-e521-3998-958a-f459800e4d11");
        assert_eq!(s.access_token, "0");
        assert_eq!(s.user_type, "legacy");
        assert!(!format!(
            "{:?}",
            GameSession {
                access_token: "secret".into(),
                ..s
            }
        )
        .contains("secret"));
    }

    #[test]
    fn client_id_comes_from_config_when_env_is_absent() {
        // The env var is read first; this test only runs the config branch when it is unset.
        if std::env::var(CLIENT_ID_ENV).is_ok() {
            return;
        }
        let mut cfg = LauncherConfig::default();
        cfg.microsoft_client_id = Some("  ".into());
        assert_eq!(
            microsoft_client_id(&cfg),
            option_env!("WEX_MS_CLIENT_ID").map(str::to_string)
        );
        cfg.microsoft_client_id = Some(" 0000-cid ".into());
        assert_eq!(microsoft_client_id(&cfg).as_deref(), Some("0000-cid"));
    }

    #[tokio::test]
    async fn legacy_install_gets_offline_account_from_nickname() {
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let cfg = LauncherConfig {
            nickname: "OldGamer".into(),
            ..Default::default()
        };
        let s = resolve_session_with(&mut store, None, &cfg).await.unwrap();
        assert_eq!(s.username, "OldGamer");
        assert_eq!(s.uuid, offline_uuid("OldGamer").to_string());
        assert_eq!(store.accounts().len(), 1);
        // Nickname edited in the wardrobe: the offline account follows.
        let cfg = LauncherConfig {
            nickname: "NewGamer".into(),
            ..Default::default()
        };
        let s = resolve_session_with(&mut store, None, &cfg).await.unwrap();
        assert_eq!(s.username, "NewGamer");
        assert_eq!(store.active().unwrap().name, "NewGamer");
    }

    #[tokio::test]
    async fn microsoft_session_uses_saved_token_while_fresh() {
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        store
            .upsert_microsoft(
                "069a79f444e94726a5befca90e38aaf5",
                "Notch",
                &MsSecrets {
                    refresh_token: "r".into(),
                    mc_access_token: "mc-fresh".into(),
                    mc_expires_at: Utc::now() + Duration::hours(20),
                    xuid: None,
                },
            )
            .unwrap();
        let s = resolve_session_with(&mut store, None, &LauncherConfig::default())
            .await
            .unwrap();
        assert_eq!(s.username, "Notch");
        assert_eq!(s.access_token, "mc-fresh");
        assert_eq!(s.user_type, "msa");
        assert_eq!(s.uuid, "069a79f444e94726a5befca90e38aaf5");
    }

    #[tokio::test]
    async fn expired_microsoft_token_is_refreshed_and_saved() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("refresh_token=old-refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "ms-access", "refresh_token": "new-refresh", "expires_in": 3600
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/xbl/user/authenticate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Token": "xbl", "DisplayClaims": { "xui": [ { "uhs": "h" } ] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/xsts/xsts/authorize"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Token": "xsts", "DisplayClaims": { "xui": [ { "uhs": "h" } ] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/mc/authentication/login_with_xbox"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "mc-new", "expires_in": 86400
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/mc/minecraft/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "069a79f444e94726a5befca90e38aaf5", "name": "NotchRenamed"
            })))
            .mount(&server)
            .await;

        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let acc = store
            .upsert_microsoft(
                "069a79f444e94726a5befca90e38aaf5",
                "Notch",
                &MsSecrets {
                    refresh_token: "old-refresh".into(),
                    mc_access_token: "mc-old".into(),
                    mc_expires_at: Utc::now() - Duration::minutes(1),
                    xuid: None,
                },
            )
            .unwrap();
        let auth = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        let s = resolve_session_with(&mut store, Some(&auth), &LauncherConfig::default())
            .await
            .unwrap();
        assert_eq!(s.access_token, "mc-new");
        assert_eq!(s.username, "NotchRenamed");
        let saved = store.read_secrets(&acc.id).unwrap();
        assert_eq!(saved.refresh_token, "new-refresh");
        // A fresh store from disk sees the same thing.
        let again = AccountStore::open(tmp.path()).unwrap();
        assert_eq!(
            again.read_secrets(&acc.id).unwrap().mc_access_token,
            "mc-new"
        );
    }

    #[tokio::test]
    async fn revoked_refresh_marks_account() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })),
            )
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let mut store = AccountStore::open(tmp.path()).unwrap();
        let acc = store
            .upsert_microsoft(
                "aa".repeat(16).as_str(),
                "Alex",
                &MsSecrets {
                    refresh_token: "revoked".into(),
                    mc_access_token: "mc-old".into(),
                    mc_expires_at: Utc::now(),
                    xuid: None,
                },
            )
            .unwrap();
        let auth = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        let err = resolve_session_with(&mut store, Some(&auth), &LauncherConfig::default())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            WexError::Auth {
                code: "AUTH_SIGN_IN_AGAIN",
                ..
            }
        ));
        assert!(store.get(&acc.id).unwrap().needs_sign_in);

        // Without a client ID an expired account cannot refresh: honest error, no stub.
        let err = resolve_session_with(&mut store, None, &LauncherConfig::default())
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            WexError::Auth {
                code: "AUTH_MS_UNAVAILABLE",
                ..
            }
        ));
    }
}
