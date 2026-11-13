//! Friends, presence, friend requests and room invites (`wex_social::friends`),
//! signed in automatically with the launcher's active account.
//!
//! The friends service hands out its own user id, token and friend code on
//! `register` (by nickname). Wex keeps those per launcher account in
//! `<data root>/social/friends-auth.bin`, sealed with the same system cipher as
//! the account tokens (DPAPI on Windows), and reuses them on the next start.
//! The UI never sees the token: it asks `social_connect` to sign in and reads
//! the result as a [`FriendsConnectionDto`].

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use wex_core::accounts::secret::{system_cipher, SecretCipher};
use wex_core::accounts::{Account, AccountKind, AccountStore};
use wex_core::config::WexPaths;
use wex_ipc::commands::{
    SocialAcceptFriendRequestRequest, SocialDeclineFriendRequestRequest, SocialGetFriendsRequest,
    SocialGetFriendsResponse, SocialListAchievementsRequest, SocialListAchievementsResponse,
    SocialLoginRequest, SocialReportAchievementProgressRequest,
    SocialReportAchievementProgressResponse, SocialSendFriendRequestRequest,
    SocialSendLobbyInviteRequest, SocialSetPresenceRequest, SocialSyncRequest,
};
use wex_ipc::error::IpcError;
use wex_ipc::events::EVT_SOCIAL_ACHIEVEMENT_TOAST;
use wex_ipc::models::{
    AchievementDefinitionDto, AchievementProgressDto, AchievementUnlockedEventDto, FriendItemDto,
    FriendProfileDto, FriendRequestItemDto, LobbyInviteItemDto, OnlineStatusDto, SyncResponseDto,
    UserAuthDto, UserPresenceDto,
};
use wex_social::achievements::{get_standard_achievements, AchievementManager};
use wex_social::friends::{
    resolve_friends_api_url, FailureKind, FriendApiClient, FriendError, OnlineStatus, UserAuth,
    UserPresence, FRIENDS_API_URL_ENV,
};

use super::lobby::{ActiveLobbySession, DEFAULT_LOBBY_API_URL};

// ============================================================================
// Connection state shown to the UI
// ============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FriendsConnectionState {
    /// No active launcher account (`reason`: `no_account`), or the session ended
    /// (`session_expired`) and a new sign-in is due.
    SignedOut,
    Connecting,
    SignedIn,
    /// The service could not be used right now. `reason`: `network`, `timeout`,
    /// `server_error`, `no_friends_api`, `bad_response`.
    Unreachable,
    /// The service refused this account (`reason`: `nick_rejected`). `detail`
    /// carries the service's own words.
    Unusable,
    /// Friends are switched off (`WEX_FRIENDS_API_URL=off` or not http(s)).
    NotConfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FriendsConnectionDto {
    pub state: FriendsConnectionState,
    pub reason: Option<String>,
    pub detail: Option<String>,
    pub account_id: Option<String>,
    /// "microsoft" or "offline".
    pub account_kind: Option<String>,
    pub nick: Option<String>,
    pub friend_code: Option<String>,
    /// Host of the friends service, e.g. «lobby-api.vercel.app».
    pub server: Option<String>,
}

impl FriendsConnectionDto {
    fn new(state: FriendsConnectionState, server: Option<String>) -> Self {
        Self {
            state,
            reason: None,
            detail: None,
            account_id: None,
            account_kind: None,
            nick: None,
            friend_code: None,
            server,
        }
    }

    fn with_reason(mut self, reason: &str, detail: Option<String>) -> Self {
        self.reason = Some(reason.to_string());
        self.detail = detail;
        self
    }

    fn for_account(mut self, a: &Account) -> Self {
        self.account_id = Some(a.id.clone());
        self.account_kind = Some(kind_name(&a.kind).to_string());
        self.nick = Some(a.name.clone());
        self
    }
}

fn kind_name(kind: &AccountKind) -> &'static str {
    match kind {
        AccountKind::Microsoft => "microsoft",
        AccountKind::Offline => "offline",
    }
}

/// What a failed sign-in or sync means for the player.
fn classify(err: &FriendError) -> (FriendsConnectionState, &'static str) {
    use FriendsConnectionState::*;
    match err.failure_kind() {
        FailureKind::Network => (Unreachable, "network"),
        FailureKind::Timeout => (Unreachable, "timeout"),
        FailureKind::ServerError => (Unreachable, "server_error"),
        FailureKind::NoFriendsApi => (Unreachable, "no_friends_api"),
        FailureKind::BadResponse => (Unreachable, "bad_response"),
        FailureKind::Rejected => (Unusable, "nick_rejected"),
        FailureKind::Unauthorized => (SignedOut, "session_expired"),
    }
}

fn failure_detail(err: &FriendError) -> String {
    match err.server_message() {
        Some(m) => m.to_string(),
        None => err.to_string(),
    }
}

// ============================================================================
// Saved friends credentials, per launcher account
// ============================================================================

const AUTH_MAGIC: &[u8; 4] = b"WEXF";
const AUTH_FORMAT: u8 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct SavedAuthFile {
    entries: Vec<SavedAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct SavedAuth {
    account_id: String,
    /// Base URL of the service the credentials belong to.
    server: String,
    user_id: String,
    user_token: String,
    friend_code: String,
    nick: String,
}

impl SavedAuth {
    fn to_auth(&self) -> UserAuth {
        UserAuth {
            user_id: self.user_id.clone(),
            user_token: self.user_token.clone(),
            friend_code: self.friend_code.clone(),
            nick: self.nick.clone(),
        }
    }
}

fn auth_file_path(root: &Path) -> PathBuf {
    root.join("social").join("friends-auth.bin")
}

fn load_saved(root: &Path, cipher: &dyn SecretCipher) -> SavedAuthFile {
    let Ok(data) = std::fs::read(auth_file_path(root)) else {
        return SavedAuthFile::default();
    };
    let parsed = (|| {
        if data.len() < 6 || &data[..4] != AUTH_MAGIC || data[4] != AUTH_FORMAT {
            return None;
        }
        if data[5] != cipher.id() {
            return None;
        }
        let plain = cipher.open(&data[6..]).ok()?;
        serde_json::from_slice::<SavedAuthFile>(&plain).ok()
    })();
    parsed.unwrap_or_else(|| {
        tracing::warn!("friends: saved friends sign-in is unreadable, a new one will be made");
        SavedAuthFile::default()
    })
}

fn store_saved(root: &Path, cipher: &dyn SecretCipher, file: &SavedAuthFile) -> Result<(), String> {
    let path = auth_file_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let plain = serde_json::to_vec(file).map_err(|e| e.to_string())?;
    let sealed = cipher.seal(&plain).map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(sealed.len() + 6);
    out.extend_from_slice(AUTH_MAGIC);
    out.push(AUTH_FORMAT);
    out.push(cipher.id());
    out.extend_from_slice(&sealed);
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, &out).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

fn to_dto(a: &UserAuth) -> UserAuthDto {
    UserAuthDto {
        user_id: a.user_id.clone(),
        user_token: a.user_token.clone(),
        friend_code: a.friend_code.clone(),
        nick: a.nick.clone(),
    }
}

// ============================================================================
// State
// ============================================================================

/// Managed state for friends, lobby invitations, and social profiles.
pub struct FriendsState {
    pub paths: WexPaths,
    pub client: Arc<Mutex<FriendApiClient>>,
    pub auth: Arc<Mutex<Option<UserAuthDto>>>,
    pub local_friends: Arc<Mutex<Vec<FriendItemDto>>>,
    pub local_requests: Arc<Mutex<Vec<FriendRequestItemDto>>>,
    pub local_invites: Arc<Mutex<Vec<LobbyInviteItemDto>>>,
    pub connection: Arc<Mutex<FriendsConnectionDto>>,
    /// Last presence the UI published; every sync sends it again.
    pub presence: Arc<Mutex<UserPresence>>,
    configured: bool,
    connect_lock: Arc<Mutex<()>>,
    cipher: Arc<dyn SecretCipher>,
    lobby_session: Option<Arc<Mutex<Option<ActiveLobbySession>>>>,
}

impl FriendsState {
    /// The friends service is `WEX_FRIENDS_API_URL` when set, otherwise the
    /// lobby service the launcher already uses for rooms.
    pub fn new(paths: WexPaths) -> Self {
        let env = std::env::var(FRIENDS_API_URL_ENV).ok();
        match resolve_friends_api_url(env.as_deref(), DEFAULT_LOBBY_API_URL) {
            Some(url) => Self::build(paths, FriendApiClient::new(url), true),
            None => Self::build(paths, FriendApiClient::new(DEFAULT_LOBBY_API_URL), false),
        }
    }

    pub fn new_with_client(paths: WexPaths, client: FriendApiClient) -> Self {
        Self::build(paths, client, true)
    }

    fn build(paths: WexPaths, client: FriendApiClient, configured: bool) -> Self {
        let server = configured.then(|| client.host());
        let initial = if configured {
            FriendsConnectionDto::new(FriendsConnectionState::Connecting, server)
        } else {
            FriendsConnectionDto::new(FriendsConnectionState::NotConfigured, None)
                .with_reason("disabled", None)
        };
        Self {
            paths,
            client: Arc::new(Mutex::new(client)),
            auth: Arc::new(Mutex::new(None)),
            local_friends: Arc::new(Mutex::new(Vec::new())),
            local_requests: Arc::new(Mutex::new(Vec::new())),
            local_invites: Arc::new(Mutex::new(Vec::new())),
            connection: Arc::new(Mutex::new(initial)),
            presence: Arc::new(Mutex::new(UserPresence::default())),
            configured,
            connect_lock: Arc::new(Mutex::new(())),
            cipher: Arc::from(system_cipher()),
            lobby_session: None,
        }
    }

    /// Lets invites carry the room's host token (shared with `LobbyState`).
    pub fn with_lobby_session(mut self, session: Arc<Mutex<Option<ActiveLobbySession>>>) -> Self {
        self.lobby_session = Some(session);
        self
    }

    #[cfg(test)]
    fn with_cipher(mut self, cipher: Arc<dyn SecretCipher>) -> Self {
        self.cipher = cipher;
        self
    }

    #[cfg(test)]
    fn disabled(mut self) -> Self {
        self.configured = false;
        self
    }

    async fn stored_auth(&self) -> Option<UserAuthDto> {
        self.auth.lock().await.clone()
    }

    async fn clear_lists(&self) {
        self.local_friends.lock().await.clear();
        self.local_requests.lock().await.clear();
        self.local_invites.lock().await.clear();
    }

    async fn apply_sync(&self, dto: &SyncResponseDto) {
        *self.local_friends.lock().await = dto.friends.clone();
        *self.local_requests.lock().await = dto.incoming_requests.clone();
        *self.local_invites.lock().await = dto.incoming_invites.clone();
    }

    /// A sync with the stored sign-in failed: reflect it in the connection.
    async fn note_failure(&self, err: &FriendError) {
        let (state, reason) = classify(err);
        let mut c = self.connection.lock().await;
        if c.state != FriendsConnectionState::SignedIn
            && c.state != FriendsConnectionState::Unreachable
        {
            return;
        }
        if state == FriendsConnectionState::SignedOut {
            // The service no longer knows this token: sign in again on the next poll.
            *self.auth.lock().await = None;
        }
        c.state = state;
        c.reason = Some(reason.to_string());
        c.detail = Some(failure_detail(err));
    }

    async fn note_success(&self) {
        let mut c = self.connection.lock().await;
        if c.state == FriendsConnectionState::Unreachable {
            c.state = FriendsConnectionState::SignedIn;
            c.reason = None;
            c.detail = None;
        }
    }
}

fn active_account(root: &Path) -> Result<Option<Account>, String> {
    AccountStore::open(root)
        .map(|s| s.active().cloned())
        .map_err(|e| e.to_string())
}

/// Sign in to friends with the launcher's active account. Reuses the saved
/// friends credentials of that account; registers its name on first use.
pub async fn connect_impl(state: &FriendsState) -> FriendsConnectionDto {
    let _single = state.connect_lock.lock().await;
    let client = { state.client.lock().await.clone() };
    let server_host = client.host();

    if !state.configured {
        *state.auth.lock().await = None;
        state.clear_lists().await;
        let c = FriendsConnectionDto::new(FriendsConnectionState::NotConfigured, None)
            .with_reason("disabled", None);
        *state.connection.lock().await = c.clone();
        return c;
    }

    let account = match active_account(&state.paths.root) {
        Ok(Some(a)) => a,
        other => {
            let detail = other.err();
            let previous = state.auth.lock().await.take();
            state.clear_lists().await;
            if let Some(prev) = previous {
                publish_offline(&client, &prev).await;
            }
            let c = FriendsConnectionDto::new(FriendsConnectionState::SignedOut, Some(server_host))
                .with_reason("no_account", detail);
            *state.connection.lock().await = c.clone();
            return c;
        }
    };

    // Another account was signed in: it leaves, and its friends go with it.
    let previous_account = state.connection.lock().await.account_id.clone();
    if previous_account.as_deref() != Some(account.id.as_str()) {
        let previous = state.auth.lock().await.take();
        state.clear_lists().await;
        if let Some(prev) = previous {
            publish_offline(&client, &prev).await;
        }
    }

    *state.connection.lock().await = FriendsConnectionDto::new(
        FriendsConnectionState::Connecting,
        Some(server_host.clone()),
    )
    .for_account(&account);

    let root = state.paths.root.clone();
    let mut file = load_saved(&root, state.cipher.as_ref());
    let base = client.base_url().to_string();
    let saved = file
        .entries
        .iter()
        .find(|e| e.account_id == account.id && e.server == base)
        .map(SavedAuth::to_auth);
    let presence = state.presence.lock().await.clone();

    let result = client
        .sign_in(&account.name, saved.clone(), &presence)
        .await;
    let c = match result {
        Ok(signed) => {
            let changed = saved.as_ref() != Some(&signed.auth);
            if changed {
                file.entries
                    .retain(|e| !(e.account_id == account.id && e.server == base));
                file.entries.push(SavedAuth {
                    account_id: account.id.clone(),
                    server: base.clone(),
                    user_id: signed.auth.user_id.clone(),
                    user_token: signed.auth.user_token.clone(),
                    friend_code: signed.auth.friend_code.clone(),
                    nick: signed.auth.nick.clone(),
                });
                if let Err(e) = store_saved(&root, state.cipher.as_ref(), &file) {
                    tracing::warn!("friends: could not save the friends sign-in: {e}");
                }
            }
            if signed.registered {
                tracing::info!(
                    "friends: registered «{}» with {}",
                    account.name,
                    server_host
                );
            }
            *state.auth.lock().await = Some(to_dto(&signed.auth));
            if let Some(sync) = signed.sync {
                state.apply_sync(&SyncResponseDto::from(sync)).await;
            }
            let mut c = FriendsConnectionDto::new(
                FriendsConnectionState::SignedIn,
                Some(server_host.clone()),
            )
            .for_account(&account);
            c.friend_code = Some(signed.auth.friend_code.clone()).filter(|s| !s.is_empty());
            c
        }
        Err(err) => {
            let (st, reason) = classify(&err);
            tracing::warn!("friends: sign-in failed ({reason}): {err}");
            // Keep the saved sign-in for the next try; it is still ours.
            *state.auth.lock().await = match (st, &saved) {
                (FriendsConnectionState::Unreachable, Some(a)) => Some(to_dto(a)),
                _ => None,
            };
            let mut c = FriendsConnectionDto::new(st, Some(server_host.clone()))
                .for_account(&account)
                .with_reason(reason, Some(failure_detail(&err)));
            c.friend_code = saved.map(|a| a.friend_code).filter(|s| !s.is_empty());
            c
        }
    };
    *state.connection.lock().await = c.clone();
    c
}

/// Best effort: tell the service the previous account went offline.
async fn publish_offline(client: &FriendApiClient, auth: &UserAuthDto) {
    let auth: UserAuth = auth.clone().into();
    let offline = UserPresence {
        status: Some(OnlineStatus::Offline),
        ..Default::default()
    };
    let _ = tokio::time::timeout(Duration::from_secs(3), client.sync(&auth, Some(&offline))).await;
}

// ============================================================================
// Tauri Commands: Friends & Social API
// ============================================================================

/// Sign in to friends with the active launcher account (on start, after an
/// account switch, and from «Повторить»). Never shows a login screen.
#[tauri::command]
pub async fn social_connect(
    state: State<'_, FriendsState>,
) -> Result<FriendsConnectionDto, IpcError> {
    Ok(connect_impl(&state).await)
}

/// The current friends connection, without any network call.
#[tauri::command]
pub async fn social_connection_status(
    state: State<'_, FriendsState>,
) -> Result<FriendsConnectionDto, IpcError> {
    Ok(state.connection.lock().await.clone())
}

/// Social profile login with explicit friends credentials.
#[tauri::command]
pub async fn social_login(
    request: Option<SocialLoginRequest>,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<FriendProfileDto, IpcError> {
    let auth_dto = request
        .map(|r| r.auth)
        .or(auth)
        .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации для входа"))?;

    // Store auth in state
    {
        let mut cur = state.auth.lock().await;
        *cur = Some(auth_dto.clone());
    }

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.clone().into();
    match client.get_user_profile(&wex_auth, &wex_auth.user_id).await {
        Ok(prof) => Ok(FriendProfileDto::from(prof)),
        Err(_) => {
            // Offline fallback profile
            Ok(FriendProfileDto {
                id: auth_dto.user_id,
                nick: auth_dto.nick,
                friend_code: Some(auth_dto.friend_code),
                play_time_seconds: 0,
                total_launches: 0,
                achievements_count: 0,
                skin_hash: None,
                skin_model: None,
            })
        }
    }
}

/// Retrieve the active user's friends list with presence and activity.
#[tauri::command]
pub async fn social_get_friends(
    request: Option<SocialGetFriendsRequest>,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<SocialGetFriendsResponse, IpcError> {
    let auth_dto = match request.map(|r| r.auth).or(auth) {
        Some(a) => Some(a),
        None => state.stored_auth().await,
    };

    if let Some(a) = auth_dto {
        let client = { state.client.lock().await.clone() };
        let wex_auth: UserAuth = a.into();
        let presence = state.presence.lock().await.clone();
        if let Ok(sync_res) = client.sync(&wex_auth, Some(&presence)).await {
            let mut list = state.local_friends.lock().await;
            *list = sync_res.friends.into_iter().map(Into::into).collect();
            return Ok(SocialGetFriendsResponse {
                friends: list.clone(),
            });
        }
    }

    let list = state.local_friends.lock().await;
    Ok(SocialGetFriendsResponse {
        friends: list.clone(),
    })
}

/// Send a friend request by friend code.
#[tauri::command]
pub async fn social_send_friend_request(
    request: Option<SocialSendFriendRequestRequest>,
    auth: Option<UserAuthDto>,
    target_friend_code: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, code) = match request {
        Some(r) => (r.auth, r.target_friend_code),
        None => {
            let a = match auth {
                Some(a) => Some(a),
                None => state.stored_auth().await,
            }
            .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let c = target_friend_code
                .ok_or_else(|| IpcError::invalid_argument("Не указан код друга"))?;
            (a, c)
        }
    };

    let code = code.trim().trim_start_matches('#').to_string();
    if code.is_empty() {
        return Err(IpcError::invalid_argument("Код друга не может быть пустым"));
    }

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    client
        .send_friend_request(&wex_auth, &code)
        .await
        .map_err(IpcError::from)?;

    Ok(())
}

async fn answer_request_auth(
    request_auth: Option<UserAuthDto>,
    auth: Option<UserAuthDto>,
    state: &FriendsState,
) -> Result<UserAuthDto, IpcError> {
    match request_auth.or(auth) {
        Some(a) => Ok(a),
        None => state
            .stored_auth()
            .await
            .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации")),
    }
}

/// Accept an incoming friend request.
#[tauri::command]
pub async fn social_accept_friend_request(
    request: Option<SocialAcceptFriendRequestRequest>,
    auth: Option<UserAuthDto>,
    request_id: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (req_auth, req_id) = match request {
        Some(r) => (Some(r.auth), Some(r.request_id)),
        None => (None, request_id),
    };
    let auth_dto = answer_request_auth(req_auth, auth, &state).await?;
    let req_id =
        req_id.ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор заявки"))?;

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    client
        .respond_friend_request(&wex_auth, &req_id, true)
        .await
        .map_err(IpcError::from)?;

    // The next sync brings the real presence; until then the friend is listed as offline.
    let mut reqs = state.local_requests.lock().await;
    if let Some(pos) = reqs.iter().position(|r| r.id == req_id) {
        let removed = reqs.remove(pos);
        let mut friends = state.local_friends.lock().await;
        if !friends.iter().any(|f| f.id == removed.from_id) {
            friends.push(FriendItemDto {
                id: removed.from_id,
                nick: removed.from_nick,
                status: OnlineStatusDto::Offline,
                last_seen: None,
                game_version: None,
                world_name: None,
                lobby_code: None,
            });
        }
    }

    Ok(())
}

/// Decline an incoming friend request.
#[tauri::command]
pub async fn social_decline_friend_request(
    request: Option<SocialDeclineFriendRequestRequest>,
    auth: Option<UserAuthDto>,
    request_id: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (req_auth, req_id) = match request {
        Some(r) => (Some(r.auth), Some(r.request_id)),
        None => (None, request_id),
    };
    let auth_dto = answer_request_auth(req_auth, auth, &state).await?;
    let req_id =
        req_id.ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор заявки"))?;

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    client
        .respond_friend_request(&wex_auth, &req_id, false)
        .await
        .map_err(IpcError::from)?;

    let mut reqs = state.local_requests.lock().await;
    reqs.retain(|r| r.id != req_id);

    Ok(())
}

/// Full sync of friends, requests, and invites. Also publishes the last
/// presence. Without a friends sign-in, or when the service fails, it returns
/// the last known lists; `social_connection_status` says which.
#[tauri::command]
pub async fn social_sync(
    request: Option<SocialSyncRequest>,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<SyncResponseDto, IpcError> {
    sync_impl(&state, request.map(|r| r.auth).or(auth)).await
}

pub async fn sync_impl(
    state: &FriendsState,
    explicit: Option<UserAuthDto>,
) -> Result<SyncResponseDto, IpcError> {
    let from_state = explicit.is_none();
    let auth_dto = match explicit {
        Some(a) => Some(a),
        None => state.stored_auth().await,
    };

    if let Some(a) = auth_dto {
        let client = { state.client.lock().await.clone() };
        let wex_auth: UserAuth = a.into();
        let presence = state.presence.lock().await.clone();
        match client.sync(&wex_auth, Some(&presence)).await {
            Ok(res) => {
                let dto = SyncResponseDto::from(res);
                state.apply_sync(&dto).await;
                if from_state {
                    state.note_success().await;
                }
                return Ok(dto);
            }
            Err(e) => {
                tracing::debug!("social_sync failed: {e}");
                if from_state {
                    state.note_failure(&e).await;
                }
            }
        }
    }

    let f = state.local_friends.lock().await.clone();
    let reqs = state.local_requests.lock().await.clone();
    let invs = state.local_invites.lock().await.clone();
    Ok(SyncResponseDto {
        friends: f,
        incoming_requests: reqs,
        incoming_invites: invs,
    })
}

/// Send a room invite to a friend. The host token of our room goes with it.
#[tauri::command]
pub async fn social_send_lobby_invite(
    request: Option<SocialSendLobbyInviteRequest>,
    auth: Option<UserAuthDto>,
    target_user_id: Option<String>,
    lobby_code: Option<String>,
    _host_name: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (auth_dto, target_id, code) = match request {
        Some(r) => (r.auth, r.target_user_id, r.lobby_code),
        None => {
            let a = match auth {
                Some(a) => Some(a),
                None => state.stored_auth().await,
            }
            .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;
            let tid = target_user_id
                .ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор получателя"))?;
            let lc = lobby_code.ok_or_else(|| IpcError::invalid_argument("Не указан код лобби"))?;
            (a, tid, lc)
        }
    };

    let code = code.trim().trim_start_matches('#').to_string();
    if code.is_empty() {
        return Err(IpcError::invalid_argument(
            "Код комнаты не может быть пустым",
        ));
    }

    let host_token = match &state.lobby_session {
        Some(s) => s
            .lock()
            .await
            .as_ref()
            .filter(|sess| {
                sess.code
                    .trim_start_matches('#')
                    .eq_ignore_ascii_case(&code)
            })
            .and_then(|sess| sess.host_token.clone()),
        None => None,
    };

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    client
        .send_lobby_invite(&wex_auth, &target_id, &code, host_token.as_deref())
        .await
        .map_err(IpcError::from)?;
    Ok(())
}

/// Respond to an incoming room invite. On accept, returns the room code.
#[tauri::command]
pub async fn social_respond_lobby_invite(
    invite_id: String,
    accept: bool,
    auth: Option<UserAuthDto>,
    state: State<'_, FriendsState>,
) -> Result<Option<String>, IpcError> {
    let auth_dto = match auth {
        Some(a) => Some(a),
        None => state.stored_auth().await,
    }
    .ok_or_else(|| IpcError::invalid_argument("Не указаны данные авторизации"))?;

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    let code = client
        .respond_lobby_invite(&wex_auth, &invite_id, accept)
        .await
        .map_err(IpcError::from)?;

    let mut invs = state.local_invites.lock().await;
    let known = invs
        .iter()
        .find(|i| i.id == invite_id)
        .map(|i| i.lobby_code.clone())
        .filter(|c| !c.is_empty());
    invs.retain(|i| i.id != invite_id);

    Ok(code.or(known))
}

/// Publish the player's presence (online, playing, in a room, offline).
/// Without a friends sign-in it is only remembered, quietly: there is nobody to
/// tell yet (the player only pressed Play or closed the game).
#[tauri::command]
pub async fn social_set_presence(
    request: Option<SocialSetPresenceRequest>,
    auth: Option<UserAuthDto>,
    presence: Option<UserPresenceDto>,
    state: State<'_, FriendsState>,
) -> Result<(), IpcError> {
    let (explicit, pres_dto) = match request {
        Some(r) => (Some(r.auth), Some(r.presence)),
        None => (auth, presence),
    };
    set_presence_impl(&state, explicit, pres_dto).await
}

pub async fn set_presence_impl(
    state: &FriendsState,
    explicit: Option<UserAuthDto>,
    pres_dto: Option<UserPresenceDto>,
) -> Result<(), IpcError> {
    let pres_dto = pres_dto.unwrap_or(UserPresenceDto {
        status: None,
        game_version: None,
        world_name: None,
        lobby_code: None,
    });
    let wex_pres: UserPresence = pres_dto.into();
    *state.presence.lock().await = wex_pres.clone();

    let from_state = explicit.is_none();
    let auth_dto = match explicit {
        Some(a) => a,
        None => match state.stored_auth().await {
            Some(a) => a,
            None => {
                tracing::debug!(
                    "social_set_presence: not signed in to friends, nothing to publish"
                );
                return Ok(());
            }
        },
    };

    let client = { state.client.lock().await.clone() };
    let wex_auth: UserAuth = auth_dto.into();
    match client.sync(&wex_auth, Some(&wex_pres)).await {
        Ok(res) => {
            if from_state {
                state.apply_sync(&SyncResponseDto::from(res)).await;
                state.note_success().await;
            }
            Ok(())
        }
        // Signed in by the launcher: the connection state shows the failure; no toast on Play.
        Err(e) if from_state => {
            tracing::debug!("social_set_presence failed: {e}");
            state.note_failure(&e).await;
            Ok(())
        }
        Err(e) => Err(IpcError::from(e)),
    }
}

// ============================================================================
// Tauri Commands: Achievements
// ============================================================================

/// List standard achievements and local progress from achievements.json.
#[tauri::command]
pub async fn social_list_achievements(
    request: Option<SocialListAchievementsRequest>,
    data_dir: Option<String>,
    state: State<'_, FriendsState>,
) -> Result<SocialListAchievementsResponse, IpcError> {
    let dir_str = request
        .map(|r| r.data_dir)
        .or(data_dir)
        .unwrap_or_else(|| state.paths.root.to_string_lossy().to_string());

    list_achievements_impl(PathBuf::from(dir_str)).await
}

pub async fn list_achievements_impl(
    data_dir: PathBuf,
) -> Result<SocialListAchievementsResponse, IpcError> {
    let mgr = AchievementManager::new(&data_dir);
    let defs = get_standard_achievements();
    let progress_map = mgr.load_progress()?;

    let mut achievements = Vec::with_capacity(defs.len());
    let mut progress = Vec::with_capacity(defs.len());

    for def in defs {
        let p = progress_map
            .get(&def.id)
            .cloned()
            .unwrap_or(wex_social::AchievementProgress {
                id: def.id.clone(),
                current: 0.0,
                unlocked: false,
                unlocked_at: None,
            });

        achievements.push(AchievementDefinitionDto::from(def));
        progress.push(AchievementProgressDto::from(p));
    }

    Ok(SocialListAchievementsResponse {
        achievements,
        progress,
    })
}

/// Report progress increment on a specific launcher achievement.
/// Emits social-achievement-toast event when an achievement unlocks.
#[tauri::command]
pub async fn social_report_achievement_progress<R: tauri::Runtime>(
    app: AppHandle<R>,
    request: Option<SocialReportAchievementProgressRequest>,
    data_dir: Option<String>,
    achievement_id: Option<String>,
    increment: Option<u32>,
    state: State<'_, FriendsState>,
) -> Result<SocialReportAchievementProgressResponse, IpcError> {
    let (dir_str, ach_id, inc) = match request {
        Some(r) => (r.data_dir, r.achievement_id, r.increment),
        None => {
            let d = data_dir.unwrap_or_else(|| state.paths.root.to_string_lossy().to_string());
            let a = achievement_id
                .ok_or_else(|| IpcError::invalid_argument("Не указан идентификатор достижения"))?;
            let i = increment.unwrap_or(1);
            (d, a, i)
        }
    };

    report_achievement_progress_impl(Some(&app), PathBuf::from(dir_str), &ach_id, inc).await
}

pub async fn report_achievement_progress_impl<R: tauri::Runtime>(
    app: Option<&AppHandle<R>>,
    data_dir: PathBuf,
    achievement_id: &str,
    increment: u32,
) -> Result<SocialReportAchievementProgressResponse, IpcError> {
    let mgr = AchievementManager::new(&data_dir);
    let unlocked_event = mgr.increment_progress(achievement_id, increment as f64)?;

    let progress_map = mgr.load_progress()?;
    let defs = get_standard_achievements();
    let def = defs
        .iter()
        .find(|d| d.id == achievement_id)
        .ok_or_else(|| IpcError::not_found(format!("Достижение {achievement_id} не найдено")))?;

    let entry = progress_map.get(achievement_id).ok_or_else(|| {
        IpcError::not_found(format!("Прогресс достижения {achievement_id} не найден"))
    })?;

    if let Some(event) = unlocked_event {
        if let Some(a) = app {
            let dto = AchievementUnlockedEventDto::from(event);
            let _ = a.emit(EVT_SOCIAL_ACHIEVEMENT_TOAST, dto);
        }
    }

    Ok(SocialReportAchievementProgressResponse {
        unlocked: entry.unlocked,
        current: entry.current,
        target: def.target,
    })
}

// ============================================================================
// Unit Tests (Mock of Core & Services)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_social_list_achievements_empty() {
        let tmp = tempdir().unwrap();
        let res = list_achievements_impl(tmp.path().to_path_buf())
            .await
            .unwrap();

        assert_eq!(res.achievements.len(), 18);
        assert_eq!(res.progress.len(), 18);

        // Verify "Искра" first_launch achievement is present
        let first_launch = res.achievements.iter().find(|a| a.id == "first_launch");
        assert!(first_launch.is_some());
        assert_eq!(first_launch.unwrap().title, "Искра");

        // Verify progress is initially 0 and unlocked is false
        let p = res
            .progress
            .iter()
            .find(|p| p.id == "first_launch")
            .unwrap();
        assert_eq!(p.current, 0.0);
        assert!(!p.unlocked);
    }

    #[tokio::test]
    async fn test_social_report_achievement_progress_incremental() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().to_path_buf();

        // Increment 10_launches target=10
        let res1 =
            report_achievement_progress_impl::<tauri::Wry>(None, dir.clone(), "ten_launches", 1)
                .await
                .unwrap();

        assert_eq!(res1.current, 1.0);
        assert_eq!(res1.target, 10);
        assert!(!res1.unlocked);

        // Increment again
        let res2 =
            report_achievement_progress_impl::<tauri::Wry>(None, dir.clone(), "ten_launches", 2)
                .await
                .unwrap();

        assert_eq!(res2.current, 3.0);
        assert!(!res2.unlocked);
    }

    #[tokio::test]
    async fn test_social_report_achievement_progress_unlock() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().to_path_buf();

        // first_launch target is 1
        let res =
            report_achievement_progress_impl::<tauri::Wry>(None, dir.clone(), "first_launch", 1)
                .await
                .unwrap();

        assert!(res.unlocked);
        assert_eq!(res.current, 1.0);
        assert_eq!(res.target, 1);

        // Achievements list should now reflect unlocked
        let list = list_achievements_impl(dir).await.unwrap();
        let p = list
            .progress
            .iter()
            .find(|p| p.id == "first_launch")
            .unwrap();
        assert!(p.unlocked);
        assert!(p.unlocked_at.is_some());
    }

    #[tokio::test]
    async fn test_social_friends_state_empty_by_default() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let state = FriendsState::new(paths);

        let friends = state.local_friends.lock().await;
        assert!(friends.is_empty(), "Empty state without fabricated data");

        let reqs = state.local_requests.lock().await;
        assert!(reqs.is_empty());

        let invs = state.local_invites.lock().await;
        assert!(invs.is_empty());
    }

    #[tokio::test]
    async fn test_social_local_request_accept_flow() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let state = FriendsState::new(paths);

        // Add incoming request to local state
        {
            let mut reqs = state.local_requests.lock().await;
            reqs.push(FriendRequestItemDto {
                id: "req-101".to_string(),
                from_id: "user-42".to_string(),
                from_nick: "Steve".to_string(),
                created_at: Some(1731000000),
            });
        }

        // Verify request exists
        assert_eq!(state.local_requests.lock().await.len(), 1);

        // Simulate decline
        {
            let mut reqs = state.local_requests.lock().await;
            reqs.retain(|r| r.id != "req-101");
        }
        assert_eq!(state.local_requests.lock().await.len(), 0);
        assert_eq!(state.local_friends.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_social_local_request_accept_adds_friend() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let state = FriendsState::new(paths);

        {
            let mut reqs = state.local_requests.lock().await;
            reqs.push(FriendRequestItemDto {
                id: "req-202".to_string(),
                from_id: "user-77".to_string(),
                from_nick: "Alex".to_string(),
                created_at: Some(1731000000),
            });
        }

        // Accept request
        {
            let mut reqs = state.local_requests.lock().await;
            let removed = reqs.remove(0);
            let mut friends = state.local_friends.lock().await;
            friends.push(FriendItemDto {
                id: removed.from_id,
                nick: removed.from_nick,
                status: OnlineStatusDto::Online,
                last_seen: None,
                game_version: Some("1.21.1".to_string()),
                world_name: None,
                lobby_code: None,
            });
        }

        assert_eq!(state.local_requests.lock().await.len(), 0);
        let friends = state.local_friends.lock().await;
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].nick, "Alex");
        assert_eq!(friends[0].status, OnlineStatusDto::Online);
    }

    // ------------------------------------------------------------------------
    // Automatic friends sign-in with the launcher account (mock service)
    // ------------------------------------------------------------------------

    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Reversible stand-in for DPAPI so tests do not depend on the Windows user.
    struct XorCipher;
    impl SecretCipher for XorCipher {
        fn id(&self) -> u8 {
            77
        }
        fn name(&self) -> &'static str {
            "test-xor"
        }
        fn seal(&self, plain: &[u8]) -> wex_core::Result<Vec<u8>> {
            Ok(plain.iter().map(|b| b ^ 0x5a).collect())
        }
        fn open(&self, sealed: &[u8]) -> wex_core::Result<Vec<u8>> {
            Ok(sealed.iter().map(|b| b ^ 0x5a).collect())
        }
    }

    struct Env {
        _tmp: tempfile::TempDir,
        server: MockServer,
        state: FriendsState,
    }

    async fn env() -> Env {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        let server = MockServer::start().await;
        let state = FriendsState::new_with_client(paths, FriendApiClient::new(server.uri()))
            .with_cipher(Arc::new(XorCipher));
        Env {
            _tmp: tmp,
            server,
            state,
        }
    }

    /// Adds an offline account; the new account becomes the active one.
    fn add_offline(state: &FriendsState, name: &str) -> String {
        let mut store = AccountStore::open(&state.paths.root).unwrap();
        store.add_offline(name).unwrap().id
    }

    fn registered(user: &str, token: &str, code: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "userId": user, "userToken": token, "friendCode": code
        }))
    }

    fn empty_sync() -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({}))
    }

    fn live_sync() -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "friends": [
                { "id": "u2", "nick": "Steve", "online": true, "status": "playing", "lobbyCode": null, "lastSeen": 1 },
                { "id": "u3", "nick": "Alex", "online": false, "status": "online", "lobbyCode": null, "lastSeen": 1 }
            ],
            "incomingRequests": [ { "id": "u4", "nick": "Ann" } ],
            "invites": [ { "inviteId": "inv1", "fromId": "u2", "fromNick": "Steve", "ts": 5 } ]
        }))
    }

    fn presence(status: OnlineStatusDto, lobby: Option<&str>) -> Option<UserPresenceDto> {
        Some(UserPresenceDto {
            status: Some(status),
            game_version: None,
            world_name: None,
            lobby_code: lobby.map(str::to_string),
        })
    }

    #[tokio::test]
    async fn connect_without_account_is_signed_out_and_calls_nobody() {
        let e = env().await;
        let c = connect_impl(&e.state).await;
        assert_eq!(c.state, FriendsConnectionState::SignedOut);
        assert_eq!(c.reason.as_deref(), Some("no_account"));
        assert!(e.server.received_requests().await.unwrap().is_empty());
        assert!(e.state.auth.lock().await.is_none());
    }

    #[tokio::test]
    async fn connect_when_switched_off_is_not_configured() {
        let e = env().await;
        let state = e.state.disabled();
        add_offline(&state, "Steve_1");
        let c = connect_impl(&state).await;
        assert_eq!(c.state, FriendsConnectionState::NotConfigured);
        assert!(e.server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn first_connect_registers_the_account_name_and_saves_sealed_credentials() {
        let e = env().await;
        let id = add_offline(&e.state, "Snow_Fox");
        Mock::given(method("POST"))
            .and(path("/api/user/register"))
            .and(body_json(serde_json::json!({ "nick": "Snow_Fox" })))
            .respond_with(registered("u1", "secret-token-1", "K7Q2"))
            .expect(1)
            .mount(&e.server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/sync"))
            .and(header("X-User-Token", "secret-token-1"))
            .and(body_json(
                serde_json::json!({ "nick": "Snow_Fox", "status": "online", "lobbyCode": null }),
            ))
            .respond_with(live_sync())
            .expect(1)
            .mount(&e.server)
            .await;

        let c = connect_impl(&e.state).await;
        assert_eq!(c.state, FriendsConnectionState::SignedIn);
        assert_eq!(c.account_id.as_deref(), Some(id.as_str()));
        assert_eq!(c.account_kind.as_deref(), Some("offline"));
        assert_eq!(c.nick.as_deref(), Some("Snow_Fox"));
        assert_eq!(c.friend_code.as_deref(), Some("K7Q2"));
        assert_eq!(c.server.as_deref(), Some("127.0.0.1"));
        assert_eq!(e.state.local_friends.lock().await.len(), 2);
        assert_eq!(
            e.state.local_friends.lock().await[0].status,
            OnlineStatusDto::Playing
        );
        assert_eq!(e.state.local_requests.lock().await[0].from_nick, "Ann");
        assert_eq!(e.state.local_invites.lock().await[0].id, "inv1");

        // The token is on disk only sealed, and the UI never receives it.
        let raw = std::fs::read(auth_file_path(&e.state.paths.root)).unwrap();
        assert_eq!(&raw[..4], AUTH_MAGIC);
        assert!(!raw.windows(14).any(|w| w == b"secret-token-1"));
        assert!(!serde_json::to_string(&c)
            .unwrap()
            .contains("secret-token-1"));
    }

    #[tokio::test]
    async fn next_start_reuses_saved_credentials_without_registering() {
        let e = env().await;
        add_offline(&e.state, "Snow_Fox");
        Mock::given(path("/api/user/register"))
            .respond_with(registered("u1", "tok1", "K7Q2"))
            .expect(1)
            .mount(&e.server)
            .await;
        Mock::given(path("/api/sync"))
            .respond_with(live_sync())
            .mount(&e.server)
            .await;
        assert_eq!(
            connect_impl(&e.state).await.state,
            FriendsConnectionState::SignedIn
        );

        // A new launcher process: fresh state, same data root.
        let again = FriendsState::new_with_client(
            e.state.paths.clone(),
            FriendApiClient::new(e.server.uri()),
        )
        .with_cipher(Arc::new(XorCipher));
        let c = connect_impl(&again).await;
        assert_eq!(c.state, FriendsConnectionState::SignedIn);
        assert_eq!(c.friend_code.as_deref(), Some("K7Q2"));
        assert_eq!(again.auth.lock().await.as_ref().unwrap().user_id, "u1");
    }

    #[tokio::test]
    async fn switching_accounts_signs_in_the_new_one_and_drops_old_friends() {
        let e = env().await;
        let first = add_offline(&e.state, "First_1");
        Mock::given(path("/api/user/register"))
            .and(body_json(serde_json::json!({ "nick": "First_1" })))
            .respond_with(registered("u1", "tok1", "AAAA"))
            .mount(&e.server)
            .await;
        Mock::given(path("/api/user/register"))
            .and(body_json(serde_json::json!({ "nick": "Second_2" })))
            .respond_with(registered("u2", "tok2", "BBBB"))
            .mount(&e.server)
            .await;
        // The old account says goodbye.
        Mock::given(path("/api/sync"))
            .and(header("X-User-Token", "tok1"))
            .and(body_json(
                serde_json::json!({ "nick": "First_1", "status": "offline", "lobbyCode": null }),
            ))
            .respond_with(empty_sync())
            .expect(1)
            .mount(&e.server)
            .await;
        Mock::given(path("/api/sync"))
            .and(header("X-User-Token", "tok1"))
            .respond_with(live_sync())
            .mount(&e.server)
            .await;
        Mock::given(path("/api/sync"))
            .and(header("X-User-Token", "tok2"))
            .respond_with(empty_sync())
            .mount(&e.server)
            .await;

        let c1 = connect_impl(&e.state).await;
        assert_eq!(c1.account_id.as_deref(), Some(first.as_str()));
        assert_eq!(e.state.local_friends.lock().await.len(), 2);

        let second = add_offline(&e.state, "Second_2");
        let c2 = connect_impl(&e.state).await;
        assert_eq!(c2.state, FriendsConnectionState::SignedIn);
        assert_eq!(c2.account_id.as_deref(), Some(second.as_str()));
        assert_eq!(c2.friend_code.as_deref(), Some("BBBB"));
        assert!(e.state.local_friends.lock().await.is_empty());
        assert_eq!(e.state.auth.lock().await.as_ref().unwrap().user_id, "u2");
    }

    #[tokio::test]
    async fn unreachable_service_is_reported_with_reason_and_retry_recovers() {
        let e = env().await;
        add_offline(&e.state, "Snow_Fox");
        Mock::given(path("/api/user/register"))
            .respond_with(
                ResponseTemplate::new(503)
                    .set_body_json(serde_json::json!({ "error": "maintenance" })),
            )
            .up_to_n_times(1)
            .mount(&e.server)
            .await;
        let c = connect_impl(&e.state).await;
        assert_eq!(c.state, FriendsConnectionState::Unreachable);
        assert_eq!(c.reason.as_deref(), Some("server_error"));
        assert_eq!(c.detail.as_deref(), Some("maintenance"));
        assert!(e.state.auth.lock().await.is_none());

        Mock::given(path("/api/user/register"))
            .respond_with(registered("u1", "tok1", "K7Q2"))
            .mount(&e.server)
            .await;
        Mock::given(path("/api/sync"))
            .respond_with(live_sync())
            .mount(&e.server)
            .await;
        assert_eq!(
            connect_impl(&e.state).await.state,
            FriendsConnectionState::SignedIn
        );
    }

    #[tokio::test]
    async fn host_without_friends_api_is_unreachable_not_signed_in() {
        let e = env().await;
        add_offline(&e.state, "Snow_Fox");
        Mock::given(path("/api/user/register"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({ "error": "Not Found" })),
            )
            .mount(&e.server)
            .await;
        let c = connect_impl(&e.state).await;
        assert_eq!(c.state, FriendsConnectionState::Unreachable);
        assert_eq!(c.reason.as_deref(), Some("no_friends_api"));
    }

    #[tokio::test]
    async fn closed_port_is_a_network_failure() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        let state =
            FriendsState::new_with_client(paths, FriendApiClient::new("http://127.0.0.1:9"))
                .with_cipher(Arc::new(XorCipher));
        add_offline(&state, "Snow_Fox");
        let c = connect_impl(&state).await;
        assert_eq!(c.state, FriendsConnectionState::Unreachable);
        assert!(matches!(
            c.reason.as_deref(),
            Some("network") | Some("timeout")
        ));
    }

    #[tokio::test]
    async fn rejected_name_makes_the_account_unusable_with_the_service_reason() {
        let e = env().await;
        add_offline(&e.state, "Snow_Fox");
        Mock::given(path("/api/user/register"))
            .respond_with(
                ResponseTemplate::new(409)
                    .set_body_json(serde_json::json!({ "error": "Nick is taken" })),
            )
            .mount(&e.server)
            .await;
        let c = connect_impl(&e.state).await;
        assert_eq!(c.state, FriendsConnectionState::Unusable);
        assert_eq!(c.reason.as_deref(), Some("nick_rejected"));
        assert_eq!(c.detail.as_deref(), Some("Nick is taken"));
    }

    async fn signed_in(e: &Env) {
        add_offline(&e.state, "Snow_Fox");
        Mock::given(path("/api/user/register"))
            .respond_with(registered("u1", "tok1", "K7Q2"))
            .mount(&e.server)
            .await;
        Mock::given(path("/api/sync"))
            .respond_with(live_sync())
            .up_to_n_times(1)
            .mount(&e.server)
            .await;
        assert_eq!(
            connect_impl(&e.state).await.state,
            FriendsConnectionState::SignedIn
        );
        e.server.reset().await;
    }

    #[tokio::test]
    async fn sync_failure_marks_unreachable_and_success_recovers() {
        let e = env().await;
        signed_in(&e).await;
        Mock::given(path("/api/sync"))
            .respond_with(ResponseTemplate::new(502))
            .up_to_n_times(1)
            .mount(&e.server)
            .await;
        let snapshot = sync_impl(&e.state, None).await.unwrap();
        assert_eq!(snapshot.friends.len(), 2, "the last known list stays");
        assert_eq!(
            e.state.connection.lock().await.state,
            FriendsConnectionState::Unreachable
        );

        Mock::given(path("/api/sync"))
            .respond_with(live_sync())
            .mount(&e.server)
            .await;
        sync_impl(&e.state, None).await.unwrap();
        let c = e.state.connection.lock().await.clone();
        assert_eq!(c.state, FriendsConnectionState::SignedIn);
        assert_eq!(c.friend_code.as_deref(), Some("K7Q2"));
    }

    #[tokio::test]
    async fn expired_token_signs_out_so_the_next_connect_registers_again() {
        let e = env().await;
        signed_in(&e).await;
        Mock::given(path("/api/sync"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&e.server)
            .await;
        sync_impl(&e.state, None).await.unwrap();
        let c = e.state.connection.lock().await.clone();
        assert_eq!(c.state, FriendsConnectionState::SignedOut);
        assert_eq!(c.reason.as_deref(), Some("session_expired"));
        assert!(e.state.auth.lock().await.is_none());
    }

    #[tokio::test]
    async fn presence_is_remembered_quietly_and_sent_with_every_sync() {
        let e = env().await;
        // Before sign-in: remembered, no request, no error.
        set_presence_impl(&e.state, None, presence(OnlineStatusDto::Playing, None))
            .await
            .unwrap();
        assert!(e.server.received_requests().await.unwrap().is_empty());

        add_offline(&e.state, "Snow_Fox");
        Mock::given(path("/api/user/register"))
            .respond_with(registered("u1", "tok1", "K7Q2"))
            .mount(&e.server)
            .await;
        Mock::given(path("/api/sync"))
            .and(body_json(
                serde_json::json!({ "nick": "Snow_Fox", "status": "playing", "lobbyCode": null }),
            ))
            .respond_with(live_sync())
            .expect(2)
            .mount(&e.server)
            .await;
        connect_impl(&e.state).await;
        sync_impl(&e.state, None).await.unwrap();
        e.server.verify().await;

        // Signed in and the service fails: no error for the UI, the state says it.
        e.server.reset().await;
        Mock::given(path("/api/sync"))
            .and(body_json(
                serde_json::json!({ "nick": "Snow_Fox", "status": "lobby", "lobbyCode": "ROOM7" }),
            ))
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&e.server)
            .await;
        set_presence_impl(
            &e.state,
            None,
            presence(OnlineStatusDto::InLobby, Some("ROOM7")),
        )
        .await
        .unwrap();
        assert_eq!(
            e.state.connection.lock().await.state,
            FriendsConnectionState::Unreachable
        );
    }

    #[tokio::test]
    async fn saved_credentials_of_another_cipher_are_not_used() {
        let e = env().await;
        let root = e.state.paths.root.clone();
        store_saved(
            &root,
            &XorCipher,
            &SavedAuthFile {
                entries: vec![SavedAuth {
                    account_id: "a".into(),
                    server: "s".into(),
                    user_id: "u".into(),
                    user_token: "t".into(),
                    friend_code: "c".into(),
                    nick: "n".into(),
                }],
            },
        )
        .unwrap();
        assert_eq!(load_saved(&root, &XorCipher).entries.len(), 1);
        struct Other;
        impl SecretCipher for Other {
            fn id(&self) -> u8 {
                78
            }
            fn name(&self) -> &'static str {
                "other"
            }
            fn seal(&self, p: &[u8]) -> wex_core::Result<Vec<u8>> {
                Ok(p.to_vec())
            }
            fn open(&self, s: &[u8]) -> wex_core::Result<Vec<u8>> {
                Ok(s.to_vec())
            }
        }
        assert!(load_saved(&root, &Other).entries.is_empty());
    }
}
