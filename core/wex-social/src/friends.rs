use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

#[derive(Clone)]
pub struct FriendApiClient {
    client: Client,
    base_url: String,
}

impl FriendApiClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn register(&self, nick: &str) -> Result<UserAuth, FriendError> {
        let url = format!("{}/api/user/register", self.base_url);
        let res = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "nick": nick }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }

        #[derive(Deserialize)]
        struct RawAuth {
            #[serde(rename = "userId")]
            user_id: String,
            #[serde(rename = "userToken")]
            user_token: String,
            #[serde(rename = "friendCode")]
            friend_code: String,
            nick: String,
        }

        let raw: RawAuth = res.json().await?;
        Ok(UserAuth {
            user_id: raw.user_id,
            user_token: raw.user_token,
            friend_code: raw.friend_code,
            nick: raw.nick,
        })
    }

    pub async fn sync(
        &self,
        auth: &UserAuth,
        presence: Option<&UserPresence>,
    ) -> Result<SyncResponse, FriendError> {
        let url = format!("{}/api/user/sync", self.base_url);
        let mut body = serde_json::json!({
            "nick": auth.nick,
        });

        if let Some(p) = presence {
            body["presence"] = serde_json::to_value(p)?;
        }

        let res = self
            .client
            .post(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }

        #[derive(Deserialize)]
        struct RawSync {
            #[serde(default)]
            friends: Vec<RawFriend>,
            #[serde(default, rename = "incomingRequests")]
            incoming_requests: Vec<RawRequest>,
            #[serde(default, rename = "incomingInvites")]
            incoming_invites: Vec<RawInvite>,
        }

        #[derive(Deserialize)]
        struct RawFriend {
            id: String,
            nick: String,
            #[serde(default = "default_status")]
            status: String,
            #[serde(rename = "lastSeen")]
            last_seen: Option<i64>,
            #[serde(rename = "gameVersion")]
            game_version: Option<String>,
            #[serde(rename = "worldName")]
            world_name: Option<String>,
            #[serde(rename = "lobbyCode")]
            lobby_code: Option<String>,
        }

        fn default_status() -> String {
            "offline".to_string()
        }

        #[derive(Deserialize)]
        struct RawRequest {
            id: String,
            #[serde(rename = "fromId")]
            from_id: String,
            #[serde(rename = "fromNick")]
            from_nick: String,
            #[serde(rename = "createdAt")]
            created_at: Option<i64>,
        }

        #[derive(Deserialize)]
        struct RawInvite {
            id: String,
            #[serde(rename = "lobbyCode")]
            lobby_code: String,
            #[serde(rename = "fromId")]
            from_id: String,
            #[serde(rename = "fromNick")]
            from_nick: String,
            #[serde(rename = "hostName")]
            host_name: Option<String>,
            #[serde(rename = "createdAt")]
            created_at: Option<i64>,
        }

        let raw: RawSync = res.json().await?;

        let friends = raw
            .friends
            .into_iter()
            .map(|f| {
                let status = match f.status.to_lowercase().as_str() {
                    "online" => OnlineStatus::Online,
                    "playing" => OnlineStatus::Playing,
                    "in_lobby" => OnlineStatus::InLobby,
                    _ => OnlineStatus::Offline,
                };
                FriendItem {
                    id: f.id,
                    nick: f.nick,
                    status,
                    last_seen: f.last_seen,
                    game_version: f.game_version,
                    world_name: f.world_name,
                    lobby_code: f.lobby_code,
                }
            })
            .collect();

        let incoming_requests = raw
            .incoming_requests
            .into_iter()
            .map(|r| FriendRequestItem {
                id: r.id,
                from_id: r.from_id,
                from_nick: r.from_nick,
                created_at: r.created_at,
            })
            .collect();

        let incoming_invites = raw
            .incoming_invites
            .into_iter()
            .map(|i| LobbyInviteItem {
                id: i.id,
                lobby_code: i.lobby_code,
                from_id: i.from_id,
                from_nick: i.from_nick,
                host_name: i.host_name,
                created_at: i.created_at,
            })
            .collect();

        Ok(SyncResponse {
            friends,
            incoming_requests,
            incoming_invites,
        })
    }

    pub async fn send_friend_request(
        &self,
        auth: &UserAuth,
        friend_code: &str,
    ) -> Result<String, FriendError> {
        let url = format!("{}/api/friends/request", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .json(&serde_json::json!({ "friendCode": friend_code }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }

        #[derive(Deserialize)]
        struct RawMsg {
            #[serde(default)]
            message: String,
        }
        let msg: RawMsg = res.json().await.unwrap_or(RawMsg {
            message: "Заявка отправлена".to_string(),
        });
        Ok(msg.message)
    }

    pub async fn respond_friend_request(
        &self,
        auth: &UserAuth,
        from_id: &str,
        accept: bool,
    ) -> Result<(), FriendError> {
        let url = format!("{}/api/friends/respond", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .json(&serde_json::json!({
                "fromId": from_id,
                "accept": accept
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }
        Ok(())
    }

    pub async fn remove_friend(&self, auth: &UserAuth, friend_id: &str) -> Result<(), FriendError> {
        let url = format!("{}/api/friends/remove", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .json(&serde_json::json!({ "friendId": friend_id }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }
        Ok(())
    }

    pub async fn send_lobby_invite(
        &self,
        auth: &UserAuth,
        to_friend_id: &str,
        lobby_code: &str,
    ) -> Result<String, FriendError> {
        let url = format!("{}/api/invites/send", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .json(&serde_json::json!({
                "toId": to_friend_id,
                "lobbyCode": lobby_code
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }

        #[derive(Deserialize)]
        struct RawInvRes {
            #[serde(rename = "inviteId")]
            invite_id: Option<String>,
        }
        let raw: RawInvRes = res.json().await?;
        Ok(raw.invite_id.unwrap_or_default())
    }

    pub async fn respond_lobby_invite(
        &self,
        auth: &UserAuth,
        invite_id: &str,
        accept: bool,
    ) -> Result<Option<String>, FriendError> {
        let url = format!("{}/api/invites/respond", self.base_url);
        let res = self
            .client
            .post(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .json(&serde_json::json!({
                "inviteId": invite_id,
                "accept": accept
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }

        #[derive(Deserialize)]
        struct RawLobbyResp {
            #[serde(rename = "lobbyCode")]
            lobby_code: Option<String>,
        }
        let raw: RawLobbyResp = res
            .json()
            .await
            .unwrap_or(RawLobbyResp { lobby_code: None });
        Ok(raw.lobby_code)
    }

    pub async fn get_user_profile(
        &self,
        auth: &UserAuth,
        friend_id: &str,
    ) -> Result<FriendProfile, FriendError> {
        let url = format!("{}/api/user/profile?friendId={}", self.base_url, friend_id);
        let res = self
            .client
            .get(&url)
            .header("X-User-Id", &auth.user_id)
            .header("X-User-Token", &auth.user_token)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(FriendError::Api {
                status,
                message: text,
            });
        }

        #[derive(Deserialize)]
        struct RawProf {
            id: String,
            nick: String,
            #[serde(rename = "friendCode")]
            friend_code: Option<String>,
            #[serde(default, rename = "playTimeSeconds")]
            play_time_seconds: u64,
            #[serde(default, rename = "totalLaunches")]
            total_launches: u32,
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
            play_time_seconds: raw.play_time_seconds,
            total_launches: raw.total_launches,
            achievements_count: raw.achievements.len(),
            skin_hash: raw.skin_sha1,
            skin_model: raw.skin_model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_friend_api_register_and_sync() {
        let mock_server = MockServer::start().await;

        // Mock Register
        Mock::given(method("POST"))
            .and(path("/api/user/register"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "userId": "usr_1001",
                "userToken": "sec_token_999",
                "friendCode": "FC-1234",
                "nick": "AlexAura"
            })))
            .mount(&mock_server)
            .await;

        // Mock Sync
        Mock::given(method("POST"))
            .and(path("/api/user/sync"))
            .and(header("X-User-Id", "usr_1001"))
            .and(header("X-User-Token", "sec_token_999"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "friends": [
                    {
                        "id": "usr_2002",
                        "nick": "StevePro",
                        "status": "playing",
                        "gameVersion": "1.20.1",
                        "worldName": "AuraSMP",
                        "lobbyCode": "LOBBY7"
                    }
                ],
                "incomingRequests": [
                    {
                        "id": "req_1",
                        "fromId": "usr_3003",
                        "fromNick": "CreeperFan",
                        "createdAt": 1730000000
                    }
                ],
                "incomingInvites": [
                    {
                        "id": "inv_1",
                        "lobbyCode": "LOBBY7",
                        "fromId": "usr_2002",
                        "fromNick": "StevePro",
                        "hostName": "StevePro",
                        "createdAt": 1730000050
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = FriendApiClient::new(mock_server.uri());

        let auth = client.register("AlexAura").await.unwrap();
        assert_eq!(auth.user_id, "usr_1001");
        assert_eq!(auth.friend_code, "FC-1234");

        let sync_res = client
            .sync(
                &auth,
                Some(&UserPresence {
                    status: Some(OnlineStatus::Online),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();

        assert_eq!(sync_res.friends.len(), 1);
        assert_eq!(sync_res.friends[0].nick, "StevePro");
        assert_eq!(sync_res.friends[0].status, OnlineStatus::Playing);
        assert_eq!(sync_res.friends[0].lobby_code.as_deref(), Some("LOBBY7"));

        assert_eq!(sync_res.incoming_requests.len(), 1);
        assert_eq!(sync_res.incoming_requests[0].from_nick, "CreeperFan");

        assert_eq!(sync_res.incoming_invites.len(), 1);
        assert_eq!(sync_res.incoming_invites[0].lobby_code, "LOBBY7");
    }

    #[tokio::test]
    async fn test_friend_api_friend_request_and_invite() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/friends/request"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "message": "Заявка отправлена"
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/invites/send"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "inviteId": "inv_9988"
            })))
            .mount(&mock_server)
            .await;

        let client = FriendApiClient::new(mock_server.uri());
        let auth = UserAuth {
            user_id: "usr_1".to_string(),
            user_token: "tok_1".to_string(),
            friend_code: "FC-1".to_string(),
            nick: "User1".to_string(),
        };

        let msg = client
            .send_friend_request(&auth, "FC-TARGET")
            .await
            .unwrap();
        assert_eq!(msg, "Заявка отправлена");

        let inv_id = client
            .send_lobby_invite(&auth, "usr_2", "ROOM88")
            .await
            .unwrap();
        assert_eq!(inv_id, "inv_9988");
    }
}
