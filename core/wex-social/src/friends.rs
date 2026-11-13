//! Client for the friends service (friend list, presence, friend requests and
//! room invites).
//!
//! The contract is the one the live service at `lobby-api.vercel.app` speaks
//! (the same service the old Aura launcher used; checked with read-only probes
//! on 2026-11-13). All calls are JSON over HTTPS:
//!
//! | Call | Request | Answer |
//! |---|---|---|
//! | `POST /api/user/register` | `{nick}` | `{userId, userToken, friendCode}` |
//! | `POST /api/sync` (auth) | `{nick, status, lobbyCode}` | `{friends, incomingRequests, outgoingRequests, invites, sentInvites}` |
//! | `POST /api/friends/request` (auth) | `{friendCode}` | `{ok, message?}` |
//! | `POST /api/friends/respond` (auth) | `{fromId, accept}` | `{ok}` |
//! | `POST /api/friends/remove` (auth) | `{friendId}` | `{ok}` |
//! | `POST /api/invite` (auth) | `{friendId, lobbyCode, hostToken}` | `{inviteId}` |
//! | `POST /api/invite/respond` (auth) | `{inviteId, accept}` | `{lobbyCode?, ok}` |
//! | `GET /api/user/profile?friendId=` (auth) | | profile |
//!
//! "auth" means the headers `X-User-Id` and `X-User-Token` that register handed
//! out. `status` is `online`, `lobby`, `playing` or `offline`. Errors come back
//! as `{"error": "..."}` with an HTTP status.
//!
//! Security note: the service identifies a player only by the nickname sent to
//! `register`; it does not check a Microsoft account or UUID. Any client can
//! register any nickname. The friend code is the only thing that ties two
//! players together.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Environment variable that points Wex at a friends service (`https://…`).
/// `off`, `none`, `disabled` or `0` switch friends off.
pub const FRIENDS_API_URL_ENV: &str = "WEX_FRIENDS_API_URL";

#[derive(Debug, Error)]
pub enum FriendError {
    #[error("Сетевая ошибка HTTP: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Ошибка API ({status}): {message}")]
    Api { status: StatusCode, message: String },
    #[error("Ошибка сериализации/десериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Неавторизован: отсутствует UserId или UserToken")]
    Unauthorized,
}

/// What a failure means for the player, independent of the transport details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// No connection to the service (DNS, refused, TLS, reset).
    Network,
    /// The service did not answer in time.
    Timeout,
    /// The saved user id/token are no longer valid (HTTP 401).
    Unauthorized,
    /// The service refused this request (other 4xx), e.g. the nickname.
    Rejected,
    /// The host answers but has no friends API at this path (404/405).
    NoFriendsApi,
    /// The service failed (5xx or 429).
    ServerError,
    /// The answer was not what the contract describes.
    BadResponse,
}

impl FriendError {
    pub fn failure_kind(&self) -> FailureKind {
        match self {
            FriendError::Http(e) if e.is_timeout() => FailureKind::Timeout,
            FriendError::Http(e) if e.is_decode() => FailureKind::BadResponse,
            FriendError::Http(_) => FailureKind::Network,
            FriendError::Api { status, .. } => match status.as_u16() {
                401 => FailureKind::Unauthorized,
                404 | 405 => FailureKind::NoFriendsApi,
                429 => FailureKind::ServerError,
                400..=499 => FailureKind::Rejected,
                500..=599 => FailureKind::ServerError,
                _ => FailureKind::BadResponse,
            },
            FriendError::Json(_) => FailureKind::BadResponse,
            FriendError::Unauthorized => FailureKind::Unauthorized,
        }
    }

    /// The service's own message (`{"error": "..."}`), when there is one.
    pub fn server_message(&self) -> Option<&str> {
        match self {
            FriendError::Api { message, .. } if !message.trim().is_empty() => Some(message),
            _ => None,
        }
    }
}

/// The friends service URL: the environment variable when set, otherwise the
/// default. `None` when friends are switched off or the value is not http(s).
pub fn resolve_friends_api_url(env_value: Option<&str>, default_url: &str) -> Option<String> {
    let raw = match env_value.map(str::trim) {
        Some(v) if !v.is_empty() => v,
        _ => default_url.trim(),
    };
    if matches!(
        raw.to_ascii_lowercase().as_str(),
        "off" | "none" | "disabled" | "0"
    ) {
        return None;
    }
    if raw.starts_with("https://") || raw.starts_with("http://") {
        Some(raw.trim_end_matches('/').to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAuth {
    pub user_id: String,
    pub user_token: String,
    pub friend_code: String,
    pub nick: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnlineStatus {
    Online,
    Playing,
    InLobby,
    Offline,
}

impl OnlineStatus {
    /// The value the service expects in `status`.
    pub fn wire(&self) -> &'static str {
        match self {
            OnlineStatus::Online => "online",
            OnlineStatus::Playing => "playing",
            OnlineStatus::InLobby => "lobby",
            OnlineStatus::Offline => "offline",
        }
    }

    fn parse(status: Option<&str>, online: Option<bool>) -> Self {
        if online == Some(false) {
            return OnlineStatus::Offline;
        }
        let s = status.unwrap_or("").to_ascii_lowercase();
        match s.as_str() {
            "playing" => OnlineStatus::Playing,
            "lobby" | "in_lobby" | "inlobby" => OnlineStatus::InLobby,
            "online" => OnlineStatus::Online,
            "offline" => OnlineStatus::Offline,
            _ if online == Some(true) => OnlineStatus::Online,
            _ => OnlineStatus::Offline,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserPresence {
    pub status: Option<OnlineStatus>,
    pub game_version: Option<String>,
    pub world_name: Option<String>,
    pub lobby_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendItem {
    pub id: String,
    pub nick: String,
    pub status: OnlineStatus,
    pub last_seen: Option<i64>,
    pub game_version: Option<String>,
    pub world_name: Option<String>,
    pub lobby_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendRequestItem {
    pub id: String,
    pub from_id: String,
    pub from_nick: String,
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyInviteItem {
    pub id: String,
    /// Empty when the service hands the code out only on accept (`/api/invite/respond`).
    pub lobby_code: String,
    pub from_id: String,
    pub from_nick: String,
    pub host_name: Option<String>,
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncResponse {
    pub friends: Vec<FriendItem>,
    pub incoming_requests: Vec<FriendRequestItem>,
    pub incoming_invites: Vec<LobbyInviteItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendProfile {
    pub id: String,
    pub nick: String,
    pub friend_code: Option<String>,
    pub play_time_seconds: u64,
    pub total_launches: u32,
    pub achievements_count: usize,
    pub skin_hash: Option<String>,
    pub skin_model: Option<String>,
}

/// Result of [`FriendApiClient::sign_in`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignIn {
    pub auth: UserAuth,
    /// The first sync. `None` when register worked but the first sync did not;
    /// the credentials are still valid and must be kept.
    pub sync: Option<SyncResponse>,
    /// True when the service handed out new credentials (first sign-in or the
    /// saved ones were rejected with 401).
    pub registered: bool,
}

#[derive(Clone)]
pub struct FriendApiClient {
    client: Client,
    base_url: String,
}

// ---------------------------------------------------------------------------
// Wire formats (tolerant: the live service and the earlier draft both parse)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RawAuth {
    #[serde(rename = "userId")]
    user_id: Option<String>,
    #[serde(rename = "userToken")]
    user_token: Option<String>,
    #[serde(rename = "friendCode")]
    friend_code: Option<String>,
    nick: Option<String>,
}

#[derive(Deserialize)]
struct RawSync {
    #[serde(default)]
    friends: Vec<RawFriend>,
    #[serde(default, rename = "incomingRequests")]
    incoming_requests: Vec<RawRequest>,
    #[serde(default)]
    invites: Vec<RawInvite>,
    #[serde(default, rename = "incomingInvites")]
    incoming_invites: Vec<RawInvite>,
}

#[derive(Deserialize)]
struct RawFriend {
    id: String,
    #[serde(default)]
    nick: String,
    status: Option<String>,
    online: Option<bool>,
    #[serde(rename = "lastSeen")]
    last_seen: Option<i64>,
    #[serde(rename = "gameVersion")]
    game_version: Option<String>,
    #[serde(rename = "worldName")]
    world_name: Option<String>,
    #[serde(rename = "lobbyCode")]
    lobby_code: Option<String>,
}

#[derive(Deserialize)]
struct RawRequest {
    id: String,
    #[serde(rename = "fromId")]
    from_id: Option<String>,
    #[serde(rename = "fromNick")]
    from_nick: Option<String>,
    nick: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<i64>,
}

#[derive(Deserialize)]
struct RawInvite {
    #[serde(rename = "inviteId")]
    invite_id: Option<String>,
    id: Option<String>,
    #[serde(rename = "lobbyCode")]
    lobby_code: Option<String>,
    #[serde(rename = "fromId", default)]
    from_id: String,
    #[serde(rename = "fromNick", default)]
    from_nick: String,
    #[serde(rename = "hostName")]
    host_name: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<i64>,
    ts: Option<i64>,
}

fn non_empty(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

impl RawSync {
    fn into_sync(self) -> SyncResponse {
        let friends = self
            .friends
            .into_iter()
            .map(|f| FriendItem {
                status: OnlineStatus::parse(f.status.as_deref(), f.online),
                id: f.id,
                nick: f.nick,
                last_seen: f.last_seen,
                game_version: non_empty(f.game_version),
                world_name: non_empty(f.world_name),
                lobby_code: non_empty(f.lobby_code),
            })
            .collect();
        let incoming_requests = self
            .incoming_requests
            .into_iter()
            .map(|r| FriendRequestItem {
                from_id: non_empty(r.from_id).unwrap_or_else(|| r.id.clone()),
                from_nick: non_empty(r.from_nick).or(r.nick).unwrap_or_default(),
                id: r.id,
                created_at: r.created_at,
            })
            .collect();
        let incoming_invites = self
            .invites
            .into_iter()
            .chain(self.incoming_invites)
            .filter_map(|i| {
                let id = non_empty(i.invite_id).or(non_empty(i.id))?;
                Some(LobbyInviteItem {
                    id,
                    lobby_code: non_empty(i.lobby_code).unwrap_or_default(),
                    from_id: i.from_id,
                    from_nick: i.from_nick,
                    host_name: non_empty(i.host_name),
                    created_at: i.created_at.or(i.ts),
                })
            })
            .collect();
        SyncResponse {
            friends,
            incoming_requests,
            incoming_invites,
        }
    }
}

/// `{"error": "..."}` or `{"message": "..."}` from an error answer, else the text.
fn error_message(text: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        for key in ["error", "message"] {
            if let Some(s) = v.get(key).and_then(|m| m.as_str()) {
                return s.to_string();
            }
        }
    }
    text.chars().take(200).collect()
}

async fn check(res: reqwest::Response) -> Result<reqwest::Response, FriendError> {
    if res.status().is_success() {
        return Ok(res);
    }
    let status = res.status();
    let text = res.text().await.unwrap_or_default();
    Err(FriendError::Api {
        status,
        message: error_message(&text),
    })
}

impl FriendApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("WexLauncher")
                .connect_timeout(Duration::from_secs(8))
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Host of the service, for the UI («lobby-api.vercel.app»).
    pub fn host(&self) -> String {
        reqwest::Url::parse(&self.base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
            .unwrap_or_else(|| self.base_url.clone())
    }

    fn post(&self, path: &str, auth: Option<&UserAuth>) -> reqwest::RequestBuilder {
        let req = self.client.post(format!("{}{}", self.base_url, path));
        match auth {
            Some(a) => req
                .header("X-User-Id", &a.user_id)
                .header("X-User-Token", &a.user_token),
            None => req,
        }
    }

    pub async fn register(&self, nick: &str) -> Result<UserAuth, FriendError> {
        let res = self
            .post("/api/user/register", None)
            .json(&serde_json::json!({ "nick": nick }))
            .send()
            .await?;
        let res = check(res).await?;
        let raw: RawAuth = res.json().await?;
        match (non_empty(raw.user_id), non_empty(raw.user_token)) {
            (Some(user_id), Some(user_token)) => Ok(UserAuth {
                user_id,
                user_token,
                friend_code: non_empty(raw.friend_code).unwrap_or_default(),
                nick: non_empty(raw.nick).unwrap_or_else(|| nick.to_string()),
            }),
            _ => Err(FriendError::Api {
                status: StatusCode::BAD_GATEWAY,
                message: "register answered without userId/userToken".to_string(),
            }),
        }
    }

    /// Publish presence and fetch friends, requests and invites in one call.
    pub async fn sync(
        &self,
        auth: &UserAuth,
        presence: Option<&UserPresence>,
    ) -> Result<SyncResponse, FriendError> {
        let status = presence
            .and_then(|p| p.status.as_ref())
            .unwrap_or(&OnlineStatus::Online);
        let lobby_code = presence.and_then(|p| p.lobby_code.clone());
        let res = self
            .post("/api/sync", Some(auth))
            .json(&serde_json::json!({
                "nick": auth.nick,
                "status": status.wire(),
                "lobbyCode": lobby_code,
            }))
            .send()
            .await?;
        let res = check(res).await?;
        let text = res.text().await?;
        let raw: RawSync = if text.trim().is_empty() {
            serde_json::from_str("{}")?
        } else {
            serde_json::from_str(&text)?
        };
        Ok(raw.into_sync())
    }

    /// Sign in to friends as `nick`: reuse the saved credentials when the
    /// service still accepts them, otherwise register (first time, or after a
    /// 401). Any other failure is returned as is and the saved credentials stay.
    pub async fn sign_in(
        &self,
        nick: &str,
        saved: Option<UserAuth>,
        presence: &UserPresence,
    ) -> Result<SignIn, FriendError> {
        if let Some(mut auth) = saved {
            auth.nick = nick.to_string();
            match self.sync(&auth, Some(presence)).await {
                Ok(sync) => {
                    return Ok(SignIn {
                        auth,
                        sync: Some(sync),
                        registered: false,
                    })
                }
                Err(e) if e.failure_kind() == FailureKind::Unauthorized => {}
                Err(e) => return Err(e),
            }
        }
        let mut auth = self.register(nick).await?;
        auth.nick = nick.to_string();
        let sync = self.sync(&auth, Some(presence)).await.ok();
        Ok(SignIn {
            auth,
            sync,
            registered: true,
        })
    }

    pub async fn send_friend_request(
        &self,
        auth: &UserAuth,
        friend_code: &str,
    ) -> Result<String, FriendError> {
        let res = self
            .post("/api/friends/request", Some(auth))
            .json(&serde_json::json!({ "friendCode": friend_code }))
            .send()
            .await?;
        let res = check(res).await?;

        #[derive(Deserialize)]
        struct RawMsg {
            #[serde(default)]
            message: String,
        }
        let msg: RawMsg = res.json().await.unwrap_or(RawMsg {
            message: String::new(),
        });
        Ok(if msg.message.is_empty() {
            "Заявка отправлена".to_string()
        } else {
            msg.message
        })
    }

    pub async fn respond_friend_request(
        &self,
        auth: &UserAuth,
        from_id: &str,
        accept: bool,
    ) -> Result<(), FriendError> {
        let res = self
            .post("/api/friends/respond", Some(auth))
            .json(&serde_json::json!({ "fromId": from_id, "accept": accept }))
            .send()
            .await?;
        check(res).await?;
        Ok(())
    }

    pub async fn remove_friend(&self, auth: &UserAuth, friend_id: &str) -> Result<(), FriendError> {
        let res = self
            .post("/api/friends/remove", Some(auth))
            .json(&serde_json::json!({ "friendId": friend_id }))
            .send()
            .await?;
        check(res).await?;
        Ok(())
    }

    /// Invite a friend into a room. `host_token` proves the room is ours; the
    /// old launcher sent an empty string when it had none.
    pub async fn send_lobby_invite(
        &self,
        auth: &UserAuth,
        friend_id: &str,
        lobby_code: &str,
        host_token: Option<&str>,
    ) -> Result<String, FriendError> {
        let res = self
            .post("/api/invite", Some(auth))
            .json(&serde_json::json!({
                "friendId": friend_id,
                "lobbyCode": lobby_code,
                "hostToken": host_token.unwrap_or(""),
            }))
            .send()
            .await?;
        let res = check(res).await?;

        #[derive(Deserialize)]
        struct RawInvRes {
            #[serde(rename = "inviteId")]
            invite_id: Option<String>,
        }
        let raw: RawInvRes = res.json().await.unwrap_or(RawInvRes { invite_id: None });
        Ok(raw.invite_id.unwrap_or_default())
    }

    /// Accept or decline an invite; on accept the service returns the room code.
    pub async fn respond_lobby_invite(
        &self,
        auth: &UserAuth,
        invite_id: &str,
        accept: bool,
    ) -> Result<Option<String>, FriendError> {
        let res = self
            .post("/api/invite/respond", Some(auth))
            .json(&serde_json::json!({ "inviteId": invite_id, "accept": accept }))
            .send()
            .await?;
        let res = check(res).await?;

        #[derive(Deserialize)]
        struct RawLobbyResp {
            #[serde(rename = "lobbyCode")]
            lobby_code: Option<String>,
        }
        let raw: RawLobbyResp = res
            .json()
            .await
            .unwrap_or(RawLobbyResp { lobby_code: None });
        Ok(non_empty(raw.lobby_code))
    }

    pub async fn get_user_profile(
        &self,
        auth: &UserAuth,
        friend_id: &str,
    ) -> Result<FriendProfile, FriendError> {
        let url = reqwest::Url::parse_with_params(
            &format!("{}/api/user/profile", self.base_url),
            &[("friendId", friend_id)],
        )
        .map_err(|e| FriendError::Api {
            status: StatusCode::BAD_REQUEST,
            message: e.to_string(),
        })?;
        let res = self
            .client
            .get(url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .send()
            .await?;
        let res = check(res).await?;

        #[derive(Deserialize)]
        struct RawProf {
            #[serde(alias = "userId")]
            id: String,
            nick: String,
            #[serde(rename = "friendCode")]
            friend_code: Option<String>,
            #[serde(default, rename = "playTimeSeconds")]
            play_time_seconds: Option<u64>,
            #[serde(default, rename = "totalLaunches")]
            total_launches: Option<u32>,
            #[serde(default)]
            achievements: Vec<serde_json::Value>,
            #[serde(rename = "skinSha1")]
            skin_sha1: Option<String>,
            #[serde(rename = "skinModel")]
            skin_model: Option<String>,
        }

        let raw: RawProf = res.json().await?;
        Ok(FriendProfile {
            id: raw.id,
            nick: raw.nick,
            friend_code: raw.friend_code,
            play_time_seconds: raw.play_time_seconds.unwrap_or(0),
            total_launches: raw.total_launches.unwrap_or(0),
            achievements_count: raw.achievements.len(),
            skin_hash: raw.skin_sha1,
            skin_model: raw.skin_model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn auth() -> UserAuth {
        UserAuth {
            user_id: "usr_1".to_string(),
            user_token: "tok_1".to_string(),
            friend_code: "FC-1".to_string(),
            nick: "User1".to_string(),
        }
    }

    /// A sync answer in the live service's shape (old Aura launcher models).
    fn live_sync() -> serde_json::Value {
        serde_json::json!({
            "friends": [
                { "id": "usr_2", "nick": "StevePro", "online": true, "status": "playing", "lobbyCode": null, "lastSeen": 1730000000000_i64 },
                { "id": "usr_3", "nick": "Lobbyist", "online": true, "status": "lobby", "lobbyCode": "ROOM7", "lastSeen": 1730000000000_i64 },
                { "id": "usr_4", "nick": "Sleeper", "online": false, "status": "online", "lobbyCode": null, "lastSeen": 1720000000000_i64 }
            ],
            "incomingRequests": [ { "id": "usr_5", "nick": "CreeperFan" } ],
            "outgoingRequests": [ { "id": "usr_6", "nick": "Waiting" } ],
            "invites": [ { "inviteId": "inv_1", "fromId": "usr_3", "fromNick": "Lobbyist", "ts": 1730000050 } ],
            "sentInvites": []
        })
    }

    #[test]
    fn resolve_url_prefers_env_and_can_switch_off() {
        assert_eq!(
            resolve_friends_api_url(None, "https://lobby.example/"),
            Some("https://lobby.example".into())
        );
        assert_eq!(
            resolve_friends_api_url(Some("  "), "https://lobby.example"),
            Some("https://lobby.example".into())
        );
        assert_eq!(
            resolve_friends_api_url(Some("http://127.0.0.1:9/"), "https://lobby.example"),
            Some("http://127.0.0.1:9".into())
        );
        assert_eq!(
            resolve_friends_api_url(Some("off"), "https://lobby.example"),
            None
        );
        assert_eq!(
            resolve_friends_api_url(Some("ftp://x"), "https://lobby.example"),
            None
        );
    }

    #[test]
    fn failure_kinds_follow_http_status() {
        let api = |s: u16| FriendError::Api {
            status: StatusCode::from_u16(s).unwrap(),
            message: String::new(),
        };
        assert_eq!(api(401).failure_kind(), FailureKind::Unauthorized);
        assert_eq!(api(404).failure_kind(), FailureKind::NoFriendsApi);
        assert_eq!(api(405).failure_kind(), FailureKind::NoFriendsApi);
        assert_eq!(api(409).failure_kind(), FailureKind::Rejected);
        assert_eq!(api(429).failure_kind(), FailureKind::ServerError);
        assert_eq!(api(503).failure_kind(), FailureKind::ServerError);
        assert_eq!(
            FriendError::Unauthorized.failure_kind(),
            FailureKind::Unauthorized
        );
    }

    #[tokio::test]
    async fn network_failure_is_network() {
        // Port 9 (discard) on loopback is closed: the connection is refused.
        let client = FriendApiClient::new("http://127.0.0.1:9");
        let err = client.sync(&auth(), None).await.unwrap_err();
        assert!(matches!(
            err.failure_kind(),
            FailureKind::Network | FailureKind::Timeout
        ));
    }

    #[tokio::test]
    async fn register_reads_live_answer_without_nick() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/user/register"))
            .and(body_json(serde_json::json!({ "nick": "AlexAura" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "userId": "usr_1001", "userToken": "sec_token_999", "friendCode": "K7Q2"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let auth = FriendApiClient::new(server.uri())
            .register("AlexAura")
            .await
            .unwrap();
        assert_eq!(auth.user_id, "usr_1001");
        assert_eq!(auth.friend_code, "K7Q2");
        assert_eq!(auth.nick, "AlexAura");
    }

    #[tokio::test]
    async fn register_without_token_is_a_bad_answer_not_a_sign_in() {
        let server = MockServer::start().await;
        Mock::given(path("/api/user/register"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "error": "busy" })),
            )
            .mount(&server)
            .await;
        let err = FriendApiClient::new(server.uri())
            .register("AlexAura")
            .await
            .unwrap_err();
        assert_eq!(err.failure_kind(), FailureKind::ServerError);
    }

    #[tokio::test]
    async fn sync_sends_live_body_and_parses_live_answer() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/sync"))
            .and(header("X-User-Id", "usr_1"))
            .and(header("X-User-Token", "tok_1"))
            .and(body_json(serde_json::json!({
                "nick": "User1", "status": "lobby", "lobbyCode": "ROOM9"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(live_sync()))
            .expect(1)
            .mount(&server)
            .await;
        let client = FriendApiClient::new(server.uri());
        let res = client
            .sync(
                &auth(),
                Some(&UserPresence {
                    status: Some(OnlineStatus::InLobby),
                    lobby_code: Some("ROOM9".into()),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        assert_eq!(res.friends.len(), 3);
        assert_eq!(res.friends[0].status, OnlineStatus::Playing);
        assert_eq!(res.friends[1].status, OnlineStatus::InLobby);
        assert_eq!(res.friends[1].lobby_code.as_deref(), Some("ROOM7"));
        assert_eq!(res.friends[2].status, OnlineStatus::Offline);
        assert_eq!(res.incoming_requests.len(), 1);
        assert_eq!(res.incoming_requests[0].from_id, "usr_5");
        assert_eq!(res.incoming_requests[0].from_nick, "CreeperFan");
        assert_eq!(res.incoming_invites.len(), 1);
        assert_eq!(res.incoming_invites[0].id, "inv_1");
        assert_eq!(res.incoming_invites[0].lobby_code, "");
        assert_eq!(res.incoming_invites[0].created_at, Some(1730000050));
    }

    #[tokio::test]
    async fn sync_without_presence_says_online_and_accepts_draft_shape() {
        let server = MockServer::start().await;
        Mock::given(path("/api/sync"))
            .and(body_json(serde_json::json!({
                "nick": "User1", "status": "online", "lobbyCode": null
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "friends": [{ "id": "usr_2", "nick": "Bob", "status": "playing", "gameVersion": "1.21.1", "worldName": "SMP", "lobbyCode": "SMP1" }],
                "incomingRequests": [{ "id": "req_1", "fromId": "usr_3", "fromNick": "Ann", "createdAt": 5 }],
                "incomingInvites": [{ "id": "inv_2", "lobbyCode": "SMP1", "fromId": "usr_2", "fromNick": "Bob", "hostName": "Bob" }]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let res = FriendApiClient::new(server.uri())
            .sync(&auth(), None)
            .await
            .unwrap();
        assert_eq!(res.friends[0].game_version.as_deref(), Some("1.21.1"));
        assert_eq!(res.incoming_requests[0].from_id, "usr_3");
        assert_eq!(res.incoming_invites[0].lobby_code, "SMP1");
    }

    #[tokio::test]
    async fn sign_in_reuses_saved_credentials() {
        let server = MockServer::start().await;
        Mock::given(path("/api/sync"))
            .and(header("X-User-Token", "tok_1"))
            .and(body_json(serde_json::json!({
                "nick": "NewName", "status": "online", "lobbyCode": null
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(live_sync()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/api/user/register"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let s = FriendApiClient::new(server.uri())
            .sign_in("NewName", Some(auth()), &UserPresence::default())
            .await
            .unwrap();
        assert!(!s.registered);
        assert_eq!(s.auth.nick, "NewName");
        assert_eq!(s.auth.friend_code, "FC-1");
        assert_eq!(s.sync.unwrap().friends.len(), 3);
    }

    #[tokio::test]
    async fn sign_in_registers_when_saved_credentials_are_rejected() {
        let server = MockServer::start().await;
        Mock::given(path("/api/sync"))
            .and(header("X-User-Token", "tok_1"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({ "error": "bad token" })),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/api/user/register"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "userId": "usr_9", "userToken": "tok_9", "friendCode": "NEW9"
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(path("/api/sync"))
            .and(header("X-User-Token", "tok_9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .expect(1)
            .mount(&server)
            .await;
        let s = FriendApiClient::new(server.uri())
            .sign_in("User1", Some(auth()), &UserPresence::default())
            .await
            .unwrap();
        assert!(s.registered);
        assert_eq!(s.auth.user_id, "usr_9");
        assert_eq!(s.auth.friend_code, "NEW9");
        assert!(s.sync.unwrap().friends.is_empty());
    }

    #[tokio::test]
    async fn sign_in_keeps_saved_credentials_when_service_fails() {
        let server = MockServer::start().await;
        Mock::given(path("/api/sync"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        Mock::given(path("/api/user/register"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        let err = FriendApiClient::new(server.uri())
            .sign_in("User1", Some(auth()), &UserPresence::default())
            .await
            .unwrap_err();
        assert_eq!(err.failure_kind(), FailureKind::ServerError);
    }

    #[tokio::test]
    async fn sign_in_reports_rejected_nickname_with_server_message() {
        let server = MockServer::start().await;
        Mock::given(path("/api/user/register"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(serde_json::json!({ "error": "Nick must be 3-16 characters" })),
            )
            .mount(&server)
            .await;
        let err = FriendApiClient::new(server.uri())
            .sign_in("x", None, &UserPresence::default())
            .await
            .unwrap_err();
        assert_eq!(err.failure_kind(), FailureKind::Rejected);
        assert_eq!(err.server_message(), Some("Nick must be 3-16 characters"));
    }

    #[tokio::test]
    async fn sign_in_keeps_new_credentials_when_first_sync_fails() {
        let server = MockServer::start().await;
        Mock::given(path("/api/user/register"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "userId": "usr_9", "userToken": "tok_9", "friendCode": "NEW9"
            })))
            .mount(&server)
            .await;
        Mock::given(path("/api/sync"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let s = FriendApiClient::new(server.uri())
            .sign_in("User1", None, &UserPresence::default())
            .await
            .unwrap();
        assert!(s.registered);
        assert!(s.sync.is_none());
        assert_eq!(s.auth.user_token, "tok_9");
    }

    #[tokio::test]
    async fn friend_request_and_invites_use_live_paths() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/friends/request"))
            .and(body_json(serde_json::json!({ "friendCode": "FC-TARGET" })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "ok": true })),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/invite"))
            .and(body_json(serde_json::json!({
                "friendId": "usr_2", "lobbyCode": "ROOM88", "hostToken": "host-secret"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "inviteId": "inv_9988" })),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/invite/respond"))
            .and(body_json(
                serde_json::json!({ "inviteId": "inv_1", "accept": true }),
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "ok": true, "lobbyCode": "ROOM7" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let client = FriendApiClient::new(server.uri());
        assert_eq!(
            client
                .send_friend_request(&auth(), "FC-TARGET")
                .await
                .unwrap(),
            "Заявка отправлена"
        );
        assert_eq!(
            client
                .send_lobby_invite(&auth(), "usr_2", "ROOM88", Some("host-secret"))
                .await
                .unwrap(),
            "inv_9988"
        );
        assert_eq!(
            client
                .respond_lobby_invite(&auth(), "inv_1", true)
                .await
                .unwrap()
                .as_deref(),
            Some("ROOM7")
        );
    }

    #[tokio::test]
    async fn error_answer_keeps_service_message() {
        let server = MockServer::start().await;
        Mock::given(path("/api/friends/request"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_json(serde_json::json!({ "error": "Friend code not found" })),
            )
            .mount(&server)
            .await;
        let err = FriendApiClient::new(server.uri())
            .send_friend_request(&auth(), "NOPE")
            .await
            .unwrap_err();
        assert_eq!(err.server_message(), Some("Friend code not found"));
    }

    #[tokio::test]
    async fn profile_query_is_encoded() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/user/profile"))
            .and(wiremock::matchers::query_param("friendId", "a b&c"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "userId": "a b&c", "nick": "Bob"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let p = FriendApiClient::new(server.uri())
            .get_user_profile(&auth(), "a b&c")
            .await
            .unwrap();
        assert_eq!(p.nick, "Bob");
        assert_eq!(p.play_time_seconds, 0);
    }

    #[test]
    fn host_is_shown_without_scheme() {
        assert_eq!(
            FriendApiClient::new("https://lobby.example/").host(),
            "lobby.example"
        );
    }
}
