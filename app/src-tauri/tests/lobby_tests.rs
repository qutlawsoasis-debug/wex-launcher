#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "resource", kind = "static")]
extern "C" {}

use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};
use tauri::{App, Manager};

use wex_app_lib::commands::{friends, launch, lobby, network};
use wex_app_lib::AppState;
use wex_core::config::WexPaths;
use wex_ipc::commands::{
    NetListOpenRoomsRequest, NetLobbyCloseRequest, NetLobbyCreateRequest, NetLobbyJoinRequest,
    NetLobbyKickRequest,
};
use wex_ipc::core_trait::MockLauncherCore;
use wex_ipc::models::UserAuthDto;
use wex_net::modsync::{
    compare_manifests, compute_manifest_hash, ModManifestEntry, ModMismatchType,
};

struct FixtureTunnel;
#[async_trait::async_trait]
impl wex_app_lib::commands::lobby_tunnel::TunnelRuntime for FixtureTunnel {
    async fn start(
        &self,
        _state: &lobby::LobbyState,
        config: &wex_net::lobby::TunnelConfigResponse,
        port: u16,
    ) -> Result<String, String> {
        assert!(port > 0);
        let address = config
            .public_address
            .as_deref()
            .ok_or("fixture has no tunnel address")?;
        let public_port = config.public_port.ok_or("fixture has no tunnel port")?;
        Ok(format!("{address}:{public_port}"))
    }
}
struct TestFixture {
    _temp: TempDir,
    _paths: WexPaths,
    server: MockServer,
    app: App<MockRuntime>,
}

impl TestFixture {
    async fn new() -> Self {
        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().join("wex")));
        paths.ensure_dirs().unwrap();
        for version in ["1.20.1", "1.21.1"] {
            paths.ensure_instance_dirs(version).unwrap();
        }

        // MockServer binds strictly to 127.0.0.1 (Rule 23:10 compliance)
        let server = MockServer::start().await;
        let core = Arc::new(MockLauncherCore::new());

        let app = mock_builder()
            .manage(launch::PlayState::new_with_core(paths.clone(), core))
            .manage(friends::FriendsState::new_with_client(
                paths.clone(),
                wex_social::FriendApiClient::new(server.uri()),
            ))
            .manage(
                lobby::LobbyState::new(paths.root.join("tools"), &server.uri(), &server.uri())
                    .with_tunnel_runtime(std::sync::Arc::new(FixtureTunnel)),
            )
            .manage(network::NetworkState::new_with_status(false))
            .manage(AppState::default())
            .build(mock_context(noop_assets()))
            .unwrap();

        Self {
            _temp: temp,
            _paths: paths,
            server,
            app,
        }
    }

    fn room(&self) -> tauri::State<'_, lobby::LobbyState> {
        self.app.state()
    }

    fn social(&self) -> tauri::State<'_, friends::FriendsState> {
        self.app.state()
    }
}

// ============================================================================
// 1. Create Room Flow
// ============================================================================
#[tokio::test]
async fn test_create_lobby_room_flow() {
    let f = TestFixture::new().await;

    Mock::given(method("POST"))
        .and(path("/api/lobby"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "ROOM123",
            "hostToken": "tok_host_alice",
            "status": "open",
            "createdAt": 1731100000
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/tunnel-config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "secret": "tunnel_sec_123",
            "publicAddress": "ply.gg",
            "publicPort": 38062
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/open"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&f.server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/heartbeat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&f.server)
        .await;

    let room = lobby::create_lobby_room("Alice Survival".into(), Some("Alice".into()), f.room())
        .await
        .unwrap();

    assert_eq!(room.code, "#ROOM123");
    assert!(room.is_host);
    assert_eq!(room.tunnel_address.as_deref(), Some("ply.gg:38062"));
    assert!(f.room().heartbeat_handle.lock().await.is_some());
    assert!(f.room().active_session.lock().await.is_some());

    // Also verify IPC wrapper net_lobby_create directly
    let create_req = NetLobbyCreateRequest {
        host_name: "Alice".into(),
        manifest: None,
    };
    let ipc_res = lobby::net_lobby_create(create_req, f.room()).await.unwrap();
    assert_eq!(ipc_res.code, "ROOM123");
    assert_eq!(ipc_res.host_token, "tok_host_alice");
}

// ============================================================================
// 2. Join Room Flow (by code and with hash prefix)
// ============================================================================
#[tokio::test]
async fn test_join_lobby_room_by_code_flow() {
    let f = TestFixture::new().await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/join"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": true,
            "code": "ROOM123",
            "status": "open",
            "tunnelAddress": "ply.gg:38062",
            "playerCount": 2
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/lobby/status"))
        .and(query_param("code", "ROOM123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "ROOM123",
            "status": "open",
            "tunnelAddress": "ply.gg:38062",
            "playerCount": 2,
            "lastHeartbeat": 1731100000,
            "players": ["Alice", "Bob"],
            "hostName": "Alice",
            "kicked": false
        })))
        .mount(&f.server)
        .await;

    // 1. Join using leading hash #ROOM123
    let room = lobby::join_lobby_room("#ROOM123".into(), Some("Bob".into()), f.room())
        .await
        .unwrap();

    assert_eq!(room.code, "#ROOM123");
    assert!(!room.is_host);
    assert_eq!(room.tunnel_address.as_deref(), Some("ply.gg:38062"));

    // 2. Verify status endpoint returns both players
    let status = lobby::get_lobby_status("#ROOM123".into(), Some("Bob".into()), f.room())
        .await
        .unwrap()
        .expect("room status should be present");

    assert_eq!(status.players.len(), 2);
    assert!(status.players.iter().any(|p| p.name == "Alice"));
    assert!(status.players.iter().any(|p| p.name == "Bob"));
    assert_eq!(status.host_name, "Alice");

    // 3. Test raw IPC DTO wrapper
    let join_dto = NetLobbyJoinRequest {
        code: "ROOM123".into(),
        player_name: "Bob".into(),
        manifest: None,
    };
    let ipc_res = lobby::net_lobby_join(join_dto, f.room()).await.unwrap();
    assert!(ipc_res.success);
    assert_eq!(ipc_res.tunnel_address.as_deref(), Some("ply.gg:38062"));
}

// ============================================================================
// 3. Friend Invitation and Delivery Flow
// ============================================================================
#[tokio::test]
async fn test_friend_invitation_delivery_and_response() {
    let f = TestFixture::new().await;

    let auth_alice = UserAuthDto {
        user_id: "usr_alice".into(),
        user_token: "tok_alice".into(),
        nick: "Alice".into(),
        friend_code: "AUR-ALICE-01".into(),
    };

    let auth_bob = UserAuthDto {
        user_id: "usr_bob".into(),
        user_token: "tok_bob".into(),
        nick: "Bob".into(),
        friend_code: "AUR-BOB-02".into(),
    };

    // 1. Mock sending invite to Bob
    Mock::given(method("POST"))
        .and(path("/api/invites/send"))
        .and(header("X-User-Id", "usr_alice"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "inviteId": "inv_999"
        })))
        .mount(&f.server)
        .await;

    // 2. Mock Bob sync receiving incoming invite
    Mock::given(method("POST"))
        .and(path("/api/user/sync"))
        .and(header("X-User-Id", "usr_bob"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "friends": [
                {
                    "id": "usr_alice",
                    "nick": "Alice",
                    "status": "online",
                    "lastSeen": 1731100000,
                    "gameVersion": "1.20.1",
                    "worldName": "Survival",
                    "lobbyCode": "ROOM123"
                }
            ],
            "incomingRequests": [],
            "incomingInvites": [
                {
                    "id": "inv_999",
                    "lobbyCode": "ROOM123",
                    "fromId": "usr_alice",
                    "fromNick": "Alice",
                    "hostName": "Alice",
                    "createdAt": 1731100000
                }
            ]
        })))
        .mount(&f.server)
        .await;

    // 3. Mock Bob responding to invite
    Mock::given(method("POST"))
        .and(path("/api/invites/respond"))
        .and(header("X-User-Id", "usr_bob"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "lobbyCode": "ROOM123"
        })))
        .mount(&f.server)
        .await;

    // Step A: Alice sends invite
    friends::social_send_lobby_invite(
        None,
        Some(auth_alice),
        Some("usr_bob".into()),
        Some("ROOM123".into()),
        Some("Alice".into()),
        f.social(),
    )
    .await
    .unwrap();

    // Step B: Bob performs sync and receives invite
    let sync_res = friends::social_sync(None, Some(auth_bob.clone()), f.social())
        .await
        .unwrap();

    assert_eq!(sync_res.incoming_invites.len(), 1);
    let inv = &sync_res.incoming_invites[0];
    assert_eq!(inv.id, "inv_999");
    assert_eq!(inv.lobby_code, "ROOM123");
    assert_eq!(inv.from_nick, "Alice");

    // Step C: Bob accepts invite
    let resp =
        friends::social_respond_lobby_invite("inv_999".into(), true, Some(auth_bob), f.social())
            .await
            .unwrap();
    assert_eq!(resp.as_deref(), Some("ROOM123"));
}

// ============================================================================
// 4. Mod Manifest Exchange and Comparison
// ============================================================================
#[test]
fn test_mod_manifest_comparison_and_hash_consistency() {
    let manifest_alice = vec![
        ModManifestEntry {
            id: "fabric-api".into(),
            name: "fabric-api".into(),
            version: "0.92.2".into(),
            enabled: true,
        },
        ModManifestEntry {
            id: "sodium".into(),
            name: "sodium".into(),
            version: "0.5.8".into(),
            enabled: true,
        },
        ModManifestEntry {
            id: "iris".into(),
            name: "iris".into(),
            version: "1.7.0".into(),
            enabled: true,
        },
    ];

    let manifest_bob_identical = manifest_alice.clone();

    // 1. Hashes match when manifests are identical
    let hash_alice = compute_manifest_hash(&manifest_alice);
    let hash_bob = compute_manifest_hash(&manifest_bob_identical);
    assert_eq!(hash_alice, hash_bob);
    assert!(!hash_alice.is_empty());

    // 2. Comparison returns zero mismatches
    let mismatches_identical = compare_manifests(&manifest_bob_identical, &manifest_alice);
    assert!(mismatches_identical.is_empty());

    // 3. Bob missing sodium and having mismatched iris version
    let manifest_bob_diff = vec![
        ModManifestEntry {
            id: "fabric-api".into(),
            name: "fabric-api".into(),
            version: "0.92.2".into(),
            enabled: true,
        },
        ModManifestEntry {
            id: "iris".into(),
            name: "iris".into(),
            version: "1.6.0-outdated".into(),
            enabled: true,
        },
    ];

    let mismatches = compare_manifests(&manifest_bob_diff, &manifest_alice);
    assert_eq!(mismatches.len(), 2);

    let sodium_mismatch = mismatches.iter().find(|m| m.mod_name == "sodium").unwrap();
    assert!(matches!(
        sodium_mismatch.mismatch_type,
        ModMismatchType::Missing
    ));

    let iris_mismatch = mismatches.iter().find(|m| m.mod_name == "iris").unwrap();
    assert!(matches!(
        iris_mismatch.mismatch_type,
        ModMismatchType::Version { .. }
    ));
}

// ============================================================================
// 5. Close Room and Cleanup Flow
// ============================================================================
#[tokio::test]
async fn test_close_lobby_room_and_cleanup() {
    let f = TestFixture::new().await;

    // Setup room
    Mock::given(method("POST"))
        .and(path("/api/lobby"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "ROOMCLOSE",
            "hostToken": "tok_close",
            "status": "open",
            "createdAt": 1731100000
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/tunnel-config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "secret": "tunnel_sec",
            "publicAddress": "ply.gg",
            "publicPort": 38062
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/open"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&f.server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/close"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&f.server)
        .await;

    let _ = lobby::create_lobby_room("Close Test".into(), None, f.room())
        .await
        .unwrap();
    assert!(f.room().active_session.lock().await.is_some());

    // Close room
    lobby::close_lobby_room("ROOMCLOSE".into(), f.room())
        .await
        .unwrap();

    // Verify session and heartbeat are cleared
    assert!(f.room().active_session.lock().await.is_none());
    assert!(f.room().heartbeat_handle.lock().await.is_none());

    // Idempotent repeated closure succeeds
    lobby::close_lobby_room("ROOMCLOSE".into(), f.room())
        .await
        .unwrap();

    // Raw IPC DTO close command
    let close_req = NetLobbyCloseRequest {
        code: "ROOMCLOSE".into(),
        host_token: "tok_close".into(),
    };
    let ipc_res = lobby::net_lobby_close(close_req, f.room()).await.unwrap();
    assert!(ipc_res);
}

// ============================================================================
// 6. Kick Player Flow
// ============================================================================
#[tokio::test]
async fn test_kick_lobby_player_flow() {
    let f = TestFixture::new().await;

    Mock::given(method("POST"))
        .and(path("/api/lobby"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "code": "ROOMKICK",
            "hostToken": "tok_kick_host",
            "status": "open",
            "createdAt": 1731100000
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/tunnel-config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "secret": "tunnel_sec",
            "publicAddress": "ply.gg",
            "publicPort": 38062
        })))
        .mount(&f.server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/open"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&f.server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/kick"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&f.server)
        .await;

    let _ = lobby::create_lobby_room("Kick World".into(), None, f.room())
        .await
        .unwrap();

    // Host kicks Griefer
    let kick_res = lobby::kick_lobby_player("ROOMKICK".into(), "Griefer".into(), f.room())
        .await
        .unwrap();
    assert!(kick_res);

    // Direct IPC DTO wrapper
    let kick_dto = NetLobbyKickRequest {
        code: "ROOMKICK".into(),
        host_token: "tok_kick_host".into(),
        player: "Griefer2".into(),
    };
    let ipc_res = lobby::net_lobby_kick(kick_dto, f.room()).await.unwrap();
    assert!(ipc_res);
}

// ============================================================================
// 7. Network Error: 404 Room Not Found
// ============================================================================
#[tokio::test]
async fn test_network_error_404_room_not_found() {
    let f = TestFixture::new().await;

    Mock::given(method("POST"))
        .and(path("/api/lobby/join"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": "Room not found",
            "code": "ROOM_NOT_FOUND"
        })))
        .mount(&f.server)
        .await;

    let res = lobby::join_lobby_room("NONEXIST".into(), Some("Player".into()), f.room()).await;
    assert!(res.is_err());
    assert!(f.room().active_session.lock().await.is_none());
}

// ============================================================================
// 8. Network Error: 503 Server Failure
// ============================================================================
#[tokio::test]
async fn test_network_error_503_service_unavailable() {
    let f = TestFixture::new().await;

    Mock::given(method("POST"))
        .and(path("/api/lobby"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&f.server)
        .await;

    let res = lobby::create_lobby_room("Fail World".into(), None, f.room()).await;
    assert!(res.is_err());
    assert!(f.room().heartbeat_handle.lock().await.is_none());
    assert!(f.room().active_session.lock().await.is_none());
}

// ============================================================================
// 9. Network Error: Unreachable Host (Offline)
// ============================================================================
#[tokio::test]
async fn test_network_error_unreachable_service_offline() {
    let temp = tempdir().unwrap();
    let unreachable_url = "http://127.0.0.1:59998";

    let state = lobby::LobbyState::new(temp.path().to_path_buf(), unreachable_url, unreachable_url);

    let app = tauri::test::mock_builder()
        .manage(state)
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let state_ref: tauri::State<'_, lobby::LobbyState> = app.state();

    let err = lobby::net_lobby_list_open_rooms(
        NetListOpenRoomsRequest {
            player_name: Some("Steve".into()),
        },
        state_ref,
    )
    .await
    .unwrap_err();

    assert_eq!(err.code, "NET_LOBBY_API_ERROR");
    assert!(err.message.contains("Сервис недоступен"));
}

// ============================================================================
// 10. Code Validation: Rejects Empty or Malformed Codes
// ============================================================================
#[tokio::test]
async fn test_validation_rejects_empty_code() {
    let f = TestFixture::new().await;

    let res_empty = lobby::join_lobby_room("   ".into(), Some("Steve".into()), f.room()).await;
    assert!(res_empty.is_err());
    assert!(res_empty.unwrap_err().contains("Введите корректный код"));

    let res_hash_only = lobby::join_lobby_room("#".into(), Some("Steve".into()), f.room()).await;
    assert!(res_hash_only.is_err());
    assert!(res_hash_only
        .unwrap_err()
        .contains("Введите корректный код"));
}
