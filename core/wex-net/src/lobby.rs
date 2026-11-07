use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use url::Url;

use crate::events::{NetEvent, NetEventBus};
use crate::modsync::ModManifestEntry;

#[derive(Error, Debug)]
pub enum LobbyError {
    #[error("HTTP transport error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("API request failed with status {status}: {message}")]
    Api { status: u16, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid URL format: {0}")]
    Url(#[from] url::ParseError),
}

// ---------------------------------------------------------------------------
// DTOs & Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LobbyCreateResponse {
    pub code: String,
    pub host_token: String,
    pub status: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LobbyJoinResponse {
    pub success: bool,
    pub code: String,
    pub status: String,
    pub tunnel_address: Option<String>,
    pub player_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LobbyStatusResponse {
    pub code: String,
    pub status: String,
    pub tunnel_address: Option<String>,
    pub player_count: u32,
    pub last_heartbeat: i64,
    pub players: Option<Vec<String>>,
    pub host_name: Option<String>,
    pub mod_sync: Option<HashMap<String, PlayerModSyncInfo>>,
    #[serde(default)]
    pub kicked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerModSyncInfo {
    pub hash: Option<String>,
    pub status: String, // "synced", "mismatch", "unverified"
    #[serde(default)]
    pub mismatches: Vec<crate::modsync::ModMismatchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TunnelConfigResponse {
    pub secret: Option<String>,
    pub public_address: Option<String>,
    pub public_port: Option<u16>,
}

// Request payloads
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateLobbyRequest<'a> {
    host_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<&'a [ModManifestEntry]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JoinLobbyRequest<'a> {
    code: &'a str,
    player_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<&'a [ModManifestEntry]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenLobbyRequest<'a> {
    code: &'a str,
    host_token: &'a str,
    tunnel_address: &'a str,
    port: u16,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HeartbeatRequest<'a> {
    code: &'a str,
    host_token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseLobbyRequest<'a> {
    code: &'a str,
    host_token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LeaveLobbyRequest<'a> {
    code: &'a str,
    player_name: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KickPlayerRequest<'a> {
    code: &'a str,
    host_token: &'a str,
    player: &'a str,
}

// ---------------------------------------------------------------------------
// Heartbeat Handle
// ---------------------------------------------------------------------------

pub struct HeartbeatHandle {
    stop_tx: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl HeartbeatHandle {
    pub fn stop(self) {
        let _ = self.stop_tx.send(true);
        self.task.abort();
    }
}

// ---------------------------------------------------------------------------
// LobbyApiClient
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct LobbyApiClient {
    http: Client,
    base_url: Url,
    events: Option<Arc<NetEventBus>>,
}

impl LobbyApiClient {
    pub fn new(base_url_str: &str, events: Option<Arc<NetEventBus>>) -> Result<Self, LobbyError> {
        let mut url_str = base_url_str.trim().to_string();
        if !url_str.ends_with('/') {
            url_str.push('/');
        }
        let base_url = Url::parse(&url_str)?;
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            http,
            base_url,
            events,
        })
    }

    fn endpoint(&self, path: &str) -> Result<Url, LobbyError> {
        let clean = path.trim_start_matches('/');
        Ok(self.base_url.join(clean)?)
    }

    // 1. POST /api/lobby
    pub async fn create_lobby(
        &self,
        host_name: &str,
        manifest: Option<&[ModManifestEntry]>,
    ) -> Result<LobbyCreateResponse, LobbyError> {
        let url = self.endpoint("api/lobby")?;
        let payload = CreateLobbyRequest { host_name, manifest };

        let resp = self.http.post(url).json(&payload).send().await?;
        Self::check_status_and_json(resp).await
    }

    // 2. POST /api/lobby/join
    pub async fn join_lobby(
        &self,
        code: &str,
        player_name: &str,
        manifest: Option<&[ModManifestEntry]>,
    ) -> Result<LobbyJoinResponse, LobbyError> {
        let url = self.endpoint("api/lobby/join")?;
        let payload = JoinLobbyRequest {
            code,
            player_name,
            manifest,
        };

        let resp = self.http.post(url).json(&payload).send().await?;
        Self::check_status_and_json(resp).await
    }

    // 3. POST /api/lobby/open
    pub async fn open_lobby(
        &self,
        code: &str,
        host_token: &str,
        tunnel_address: &str,
        port: u16,
    ) -> Result<bool, LobbyError> {
        let url = self.endpoint("api/lobby/open")?;
        let payload = OpenLobbyRequest {
            code,
            host_token,
            tunnel_address,
            port,
        };

        let resp = self.http.post(url).json(&payload).send().await?;
        if resp.status().is_success() {
            Ok(true)
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            Err(LobbyError::Api {
                status,
                message: body,
            })
        }
    }

    // 4. GET /api/lobby/status?code={code}&player={player}
    pub async fn get_status(
        &self,
        code: &str,
        player_name: Option<&str>,
    ) -> Result<LobbyStatusResponse, LobbyError> {
        let mut url = self.endpoint("api/lobby/status")?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("code", code);
            if let Some(player) = player_name {
                pairs.append_pair("player", player);
            }
        }

        let resp = self.http.get(url).send().await?;
        Self::check_status_and_json(resp).await
    }

    // 5. POST /api/lobby/heartbeat
    pub async fn heartbeat(&self, code: &str, host_token: &str) -> Result<bool, LobbyError> {
        let url = self.endpoint("api/lobby/heartbeat")?;
        let payload = HeartbeatRequest { code, host_token };

        let resp = self.http.post(url).json(&payload).send().await?;
        let success = resp.status().is_success();
        if let Some(bus) = &self.events {
            if success {
                bus.emit(NetEvent::LobbyHeartbeatSent {
                    code: code.to_string(),
                });
            } else {
                bus.emit(NetEvent::LobbyHeartbeatFailed {
                    code: code.to_string(),
                    error: format!("HTTP {}", resp.status()),
                });
            }
        }
        Ok(success)
    }

    // 6. POST /api/lobby/close
    pub async fn close_lobby(&self, code: &str, host_token: &str) -> Result<bool, LobbyError> {
        let url = self.endpoint("api/lobby/close")?;
        let payload = CloseLobbyRequest { code, host_token };

        let resp = self.http.post(url).json(&payload).send().await?;
        Ok(resp.status().is_success())
    }

    // 7. POST /api/lobby/leave
    pub async fn leave_lobby(&self, code: &str, player_name: &str) -> Result<bool, LobbyError> {
        let url = self.endpoint("api/lobby/leave")?;
        let payload = LeaveLobbyRequest { code, player_name };

        let resp = self.http.post(url).json(&payload).send().await?;
        Ok(resp.status().is_success())
    }

    // 8. POST /api/lobby/kick
    pub async fn kick_player(
        &self,
        code: &str,
        host_token: &str,
        player: &str,
    ) -> Result<bool, LobbyError> {
        let url = self.endpoint("api/lobby/kick")?;
        let payload = KickPlayerRequest {
            code,
            host_token,
            player,
        };

        let resp = self.http.post(url).json(&payload).send().await?;
        Ok(resp.status().is_success())
    }

    // 9. GET /api/tunnel-config
    pub async fn get_tunnel_config(&self) -> Result<TunnelConfigResponse, LobbyError> {
        let url = self.endpoint("api/tunnel-config")?;
        let resp = self
            .http
            .get(url)
            .header("X-Aura-Client", "launcher")
            .send()
            .await?;

        Self::check_status_and_json(resp).await
    }

    /// Spawns a background task that periodically sends a heartbeat every 15 seconds.
    pub fn start_heartbeat_loop(
        &self,
        code: String,
        host_token: String,
        interval: Duration,
    ) -> HeartbeatHandle {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let client = self.clone();

        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await; // Initial tick fires immediately

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let _ = client.heartbeat(&code, &host_token).await;
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });

        HeartbeatHandle { stop_tx, task }
    }

    async fn check_status_and_json<T: for<'de> Deserialize<'de>>(
        resp: reqwest::Response,
    ) -> Result<T, LobbyError> {
        let status = resp.status();
        if status.is_success() {
            let data = resp.json::<T>().await?;
            Ok(data)
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(LobbyError::Api {
                status: status.as_u16(),
                message: body,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_create_and_join_lobby_mock() {
        let mock_server = MockServer::start().await;

        // Mock 1. POST /api/lobby
        Mock::given(method("POST"))
            .and(path("/api/lobby"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "TEST",
                "hostToken": "token123",
                "status": "open",
                "createdAt": 1731000000
            })))
            .mount(&mock_server)
            .await;

        // Mock 2. POST /api/lobby/join
        Mock::given(method("POST"))
            .and(path("/api/lobby/join"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "code": "TEST",
                "status": "open",
                "tunnelAddress": "ply.gg:12345",
                "playerCount": 2
            })))
            .mount(&mock_server)
            .await;

        let client = LobbyApiClient::new(&mock_server.uri(), None).unwrap();

        let create_res = client.create_lobby("HostPlayer", None).await.unwrap();
        assert_eq!(create_res.code, "TEST");
        assert_eq!(create_res.host_token, "token123");

        let join_res = client.join_lobby("TEST", "GuestPlayer", None).await.unwrap();
        assert!(join_res.success);
        assert_eq!(join_res.tunnel_address.unwrap(), "ply.gg:12345");
        assert_eq!(join_res.player_count, 2);
    }

    #[tokio::test]
    async fn test_all_remaining_lobby_endpoints_mock() {
        let mock_server = MockServer::start().await;

        // Mock 3. POST /api/lobby/open
        Mock::given(method("POST"))
            .and(path("/api/lobby/open"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Mock 4. GET /api/lobby/status
        Mock::given(method("GET"))
            .and(path("/api/lobby/status"))
            .and(query_param("code", "TEST"))
            .and(query_param("player", "Player1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "TEST",
                "status": "open",
                "tunnelAddress": "ply.gg:12345",
                "playerCount": 1,
                "lastHeartbeat": 1731000100,
                "players": ["Player1"],
                "hostName": "Player1",
                "kicked": false
            })))
            .mount(&mock_server)
            .await;

        // Mock 5. POST /api/lobby/heartbeat
        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Mock 6. POST /api/lobby/close
        Mock::given(method("POST"))
            .and(path("/api/lobby/close"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Mock 7. POST /api/lobby/leave
        Mock::given(method("POST"))
            .and(path("/api/lobby/leave"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Mock 8. POST /api/lobby/kick
        Mock::given(method("POST"))
            .and(path("/api/lobby/kick"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // Mock 9. GET /api/tunnel-config
        Mock::given(method("GET"))
            .and(path("/api/tunnel-config"))
            .and(header("X-Aura-Client", "launcher"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "secret": "secret_abc",
                "publicAddress": "ply.gg",
                "publicPort": 25565
            })))
            .mount(&mock_server)
            .await;

        let client = LobbyApiClient::new(&mock_server.uri(), None).unwrap();

        assert!(client.open_lobby("TEST", "token", "ply.gg:12345", 25565).await.unwrap());

        let status = client.get_status("TEST", Some("Player1")).await.unwrap();
        assert_eq!(status.code, "TEST");
        assert_eq!(status.player_count, 1);

        assert!(client.heartbeat("TEST", "token").await.unwrap());
        assert!(client.close_lobby("TEST", "token").await.unwrap());
        assert!(client.leave_lobby("TEST", "Player1").await.unwrap());
        assert!(client.kick_player("TEST", "token", "BadPlayer").await.unwrap());

        let tunnel = client.get_tunnel_config().await.unwrap();
        assert_eq!(tunnel.secret.unwrap(), "secret_abc");
        assert_eq!(tunnel.public_port.unwrap(), 25565);
    }

    #[tokio::test]
    async fn test_heartbeat_loop_and_stop() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2..)
            .mount(&mock_server)
            .await;

        let client = LobbyApiClient::new(&mock_server.uri(), None).unwrap();
        let handle = client.start_heartbeat_loop(
            "TEST".to_string(),
            "token".to_string(),
            Duration::from_millis(50),
        );

        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.stop();
    }
}
