use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};
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

/// Heartbeats in a row that may fail before the loop reports the room as unreachable.
pub const HEARTBEAT_FAILURE_LIMIT: u32 = 3;

impl LobbyError {
    /// The lobby service says the room does not exist (closed or expired).
    pub fn is_not_found(&self) -> bool {
        matches!(self, LobbyError::Api { status: 404, .. })
    }
}

// ---------------------------------------------------------------------------
// DTOs & Models
// ---------------------------------------------------------------------------

/// `null` and a missing field both become the type's default.
fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

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
    #[serde(default, deserialize_with = "null_as_default")]
    pub player_count: u32,
    #[serde(default, deserialize_with = "null_as_default")]
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

/// What the heartbeat loop learned about the room, readable while it runs.
#[derive(Debug, Default)]
pub struct HeartbeatHealth {
    consecutive_failures: AtomicU32,
    room_gone: AtomicBool,
}

impl HeartbeatHealth {
    /// Heartbeats in a row that did not reach the service or were refused.
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures.load(Ordering::SeqCst)
    }

    /// The service answered 404: the room no longer exists and the loop stopped.
    pub fn room_gone(&self) -> bool {
        self.room_gone.load(Ordering::SeqCst)
    }

    /// At least [`HEARTBEAT_FAILURE_LIMIT`] heartbeats in a row failed.
    pub fn failing(&self) -> bool {
        self.consecutive_failures() >= HEARTBEAT_FAILURE_LIMIT
    }
}

pub struct HeartbeatHandle {
    stop_tx: watch::Sender<bool>,
    task: JoinHandle<()>,
    health: Arc<HeartbeatHealth>,
}

impl HeartbeatHandle {
    pub fn stop(self) {
        let _ = self.stop_tx.send(true);
        self.task.abort();
    }

    pub fn health(&self) -> Arc<HeartbeatHealth> {
        self.health.clone()
    }

    /// The loop ended on its own (the room is gone).
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
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
        let http = Client::builder().timeout(Duration::from_secs(10)).build()?;

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
        let payload = CreateLobbyRequest {
            host_name,
            manifest,
        };

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
        Ok((200..300).contains(&self.heartbeat_status(code, host_token).await?))
    }

    /// Sends one heartbeat and returns the HTTP status.
    async fn heartbeat_status(&self, code: &str, host_token: &str) -> Result<u16, LobbyError> {
        let url = self.endpoint("api/lobby/heartbeat")?;
        let payload = HeartbeatRequest { code, host_token };

        let resp = match self.http.post(url).json(&payload).send().await {
            Ok(resp) => resp,
            Err(e) => {
                if let Some(bus) = &self.events {
                    bus.emit(NetEvent::LobbyHeartbeatFailed {
                        code: code.to_string(),
                        error: e.to_string(),
                    });
                }
                return Err(e.into());
            }
        };
        let status = resp.status();
        if let Some(bus) = &self.events {
            if status.is_success() {
                bus.emit(NetEvent::LobbyHeartbeatSent {
                    code: code.to_string(),
                });
            } else {
                bus.emit(NetEvent::LobbyHeartbeatFailed {
                    code: code.to_string(),
                    error: format!("HTTP {status}"),
                });
            }
        }
        Ok(status.as_u16())
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

    /// Spawns a background task that sends a heartbeat every `interval`.
    ///
    /// A 404 means the room is gone: the loop records it in [`HeartbeatHealth`],
    /// emits [`NetEvent::LobbyRoomGone`] and stops. Repeated failures are
    /// counted and reported once with [`NetEvent::LobbyHeartbeatLost`].
    pub fn start_heartbeat_loop(
        &self,
        code: String,
        host_token: String,
        interval: Duration,
    ) -> HeartbeatHandle {
        let (stop_tx, mut stop_rx) = watch::channel(false);
        let client = self.clone();
        let health = Arc::new(HeartbeatHealth::default());
        let loop_health = health.clone();

        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await; // Initial tick fires immediately

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        match client.heartbeat_status(&code, &host_token).await {
                            Ok(404) => {
                                loop_health.room_gone.store(true, Ordering::SeqCst);
                                if let Some(bus) = &client.events {
                                    bus.emit(NetEvent::LobbyRoomGone { code: code.clone() });
                                }
                                break;
                            }
                            Ok(status) if (200..300).contains(&status) => {
                                loop_health.consecutive_failures.store(0, Ordering::SeqCst);
                            }
                            _ => {
                                let failures =
                                    loop_health.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
                                if failures == HEARTBEAT_FAILURE_LIMIT {
                                    if let Some(bus) = &client.events {
                                        bus.emit(NetEvent::LobbyHeartbeatLost {
                                            code: code.clone(),
                                            failures,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                }
            }
        });

        HeartbeatHandle {
            stop_tx,
            task,
            health,
        }
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

        let join_res = client
            .join_lobby("TEST", "GuestPlayer", None)
            .await
            .unwrap();
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

        assert!(client
            .open_lobby("TEST", "token", "ply.gg:12345", 25565)
            .await
            .unwrap());

        let status = client.get_status("TEST", Some("Player1")).await.unwrap();
        assert_eq!(status.code, "TEST");
        assert_eq!(status.player_count, 1);

        assert!(client.heartbeat("TEST", "token").await.unwrap());
        assert!(client.close_lobby("TEST", "token").await.unwrap());
        assert!(client.leave_lobby("TEST", "Player1").await.unwrap());
        assert!(client
            .kick_player("TEST", "token", "BadPlayer")
            .await
            .unwrap());

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

    #[tokio::test]
    async fn heartbeat_loop_stops_when_the_room_is_gone() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&mock_server)
            .await;
        let bus = Arc::new(NetEventBus::new(16));
        let mut rx = bus.subscribe();
        let client = LobbyApiClient::new(&mock_server.uri(), Some(bus)).unwrap();
        let handle =
            client.start_heartbeat_loop("GONE".into(), "token".into(), Duration::from_millis(20));
        let health = handle.health();
        tokio::time::timeout(Duration::from_secs(5), async {
            while !handle.is_finished() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("heartbeat loop must stop on 404");
        assert!(health.room_gone());
        let mut gone = false;
        while let Ok(event) = rx.try_recv() {
            gone |= event == NetEvent::LobbyRoomGone { code: "GONE".into() };
        }
        assert!(gone);
        // Only the first heartbeat was sent (checked by `expect(1)` on drop).
        tokio::time::sleep(Duration::from_millis(80)).await;
        handle.stop();
    }

    #[tokio::test]
    async fn heartbeat_loop_reports_repeated_failures_and_recovers() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(503))
            .up_to_n_times(HEARTBEAT_FAILURE_LIMIT as u64)
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        let bus = Arc::new(NetEventBus::new(64));
        let mut rx = bus.subscribe();
        let client = LobbyApiClient::new(&mock_server.uri(), Some(bus)).unwrap();
        let handle =
            client.start_heartbeat_loop("FLAKY".into(), "token".into(), Duration::from_millis(20));
        let health = handle.health();
        let mut lost = false;
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match rx.recv().await.unwrap() {
                    NetEvent::LobbyHeartbeatLost { failures, .. } => {
                        assert_eq!(failures, HEARTBEAT_FAILURE_LIMIT);
                        lost = true;
                    }
                    NetEvent::LobbyHeartbeatSent { .. } if lost => break,
                    _ => {}
                }
            }
        })
        .await
        .expect("failures must be reported, then the loop recovers");
        assert!(!health.room_gone());
        assert_eq!(health.consecutive_failures(), 0);
        assert!(!handle.is_finished());
        handle.stop();
    }

    #[test]
    fn status_tolerates_null_heartbeat_and_player_count() {
        let parsed: LobbyStatusResponse = serde_json::from_value(serde_json::json!({
            "code": "A", "status": "open", "tunnelAddress": null,
            "playerCount": null, "lastHeartbeat": null,
            "players": null, "hostName": null, "modSync": null
        }))
        .unwrap();
        assert_eq!((parsed.player_count, parsed.last_heartbeat), (0, 0));
        let missing: LobbyStatusResponse =
            serde_json::from_value(serde_json::json!({"code": "A", "status": "open"})).unwrap();
        assert_eq!((missing.player_count, missing.last_heartbeat), (0, 0));
        assert!(LobbyError::Api {
            status: 404,
            message: String::new()
        }
        .is_not_found());
        assert!(!LobbyError::Api {
            status: 500,
            message: "closed".into()
        }
        .is_not_found());
    }

    #[tokio::test]
    async fn test_room_wex_test_mp_lifecycle() {
        let mock_server = MockServer::start().await;

        // 1. Create room wex-test-mp
        Mock::given(method("POST"))
            .and(path("/api/lobby"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "wex-test-mp",
                "hostToken": "host_token_12345",
                "status": "waiting",
                "createdAt": 1731100000
            })))
            .mount(&mock_server)
            .await;

        // 2. Join room wex-test-mp
        Mock::given(method("POST"))
            .and(path("/api/lobby/join"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "code": "wex-test-mp",
                "status": "waiting",
                "tunnelAddress": null,
                "playerCount": 2
            })))
            .mount(&mock_server)
            .await;

        // 3. Open room / publish tunnel address
        Mock::given(method("POST"))
            .and(path("/api/lobby/open"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // 4. Status query
        Mock::given(method("GET"))
            .and(path("/api/lobby/status"))
            .and(query_param("code", "wex-test-mp"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "wex-test-mp",
                "status": "open",
                "tunnelAddress": "127.0.0.1:25565",
                "playerCount": 2,
                "lastHeartbeat": 1731100050,
                "players": ["WexA", "WexB"],
                "hostName": "WexA",
                "kicked": false
            })))
            .mount(&mock_server)
            .await;

        // 5. Heartbeat
        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        // 6. Close room
        Mock::given(method("POST"))
            .and(path("/api/lobby/close"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let client = LobbyApiClient::new(&mock_server.uri(), None).unwrap();

        // Step 1: WexA creates room
        let create_res = client.create_lobby("WexA", None).await.unwrap();
        assert_eq!(create_res.code, "wex-test-mp");
        assert_eq!(create_res.host_token, "host_token_12345");

        // Step 2: WexB joins room
        let join_res = client
            .join_lobby("wex-test-mp", "WexB", None)
            .await
            .unwrap();
        assert!(join_res.success);
        assert_eq!(join_res.player_count, 2);

        // Step 3: WexA publishes tunnel address on 127.0.0.1
        let open_ok = client
            .open_lobby(
                "wex-test-mp",
                &create_res.host_token,
                "127.0.0.1:25565",
                25565,
            )
            .await
            .unwrap();
        assert!(open_ok);

        // Step 4: WexB verifies status & tunnel
        let status = client
            .get_status("wex-test-mp", Some("WexB"))
            .await
            .unwrap();
        assert_eq!(status.code, "wex-test-mp");
        assert_eq!(status.status, "open");
        assert_eq!(status.tunnel_address.as_deref(), Some("127.0.0.1:25565"));
        assert_eq!(status.player_count, 2);

        // Step 5: Heartbeat
        let hb_ok = client
            .heartbeat("wex-test-mp", &create_res.host_token)
            .await
            .unwrap();
        assert!(hb_ok);

        // Step 6: Close room (cleanup)
        let close_ok = client
            .close_lobby("wex-test-mp", &create_res.host_token)
            .await
            .unwrap();
        assert!(close_ok);
    }
}
