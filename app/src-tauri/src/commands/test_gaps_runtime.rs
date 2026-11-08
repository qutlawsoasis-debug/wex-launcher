use super::*;

fn host_session() -> lobby::ActiveLobbySession {
    lobby::ActiveLobbySession {
        code: "ABC123".into(),
        host_token: Some("fixture".into()),
        host_name: "Host".into(),
        world_name: "Fixture".into(),
        player_name: "Host".into(),
        is_host: true,
        tunnel_address: None,
    }
}
#[tokio::test]
async fn join_lobby_room_success_normalizes_code_and_saves_guest_session() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/join", 200, json!({"success":true,"code":"ABC123","status":"open","tunnelAddress":"localhost:25565","playerCount":2})).await;
    f.respond("/api/lobby/status", 200, room_status()).await;
    let room = lobby::join_lobby_room("#ABC123".into(), Some("Guest".into()), f.room())
        .await
        .unwrap();
    assert!(!room.is_host);
    assert_eq!(room.code, "#ABC123");
    assert_eq!(
        f.room()
            .active_session
            .lock()
            .await
            .as_ref()
            .unwrap()
            .player_name,
        "Guest"
    );
    f.respond("/api/lobby/leave", 200, json!({})).await;
    lobby::close_lobby_room(room.code, f.room()).await.unwrap();
}
#[tokio::test]
async fn kick_lobby_player_success_uses_host_token() {
    let f = Fixture::new().await;
    *f.room().active_session.lock().await = Some(host_session());
    Mock::given(path("/api/lobby/kick"))
        .and(body_json(
            json!({"code":"ABC123","hostToken":"fixture","player":"Remote"}),
        ))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&f.server)
        .await;
    assert!(
        lobby::kick_lobby_player("#ABC123".into(), "Remote".into(), f.room())
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn get_lobby_status_error_when_kicked_discards_stale_session() {
    let f = Fixture::new().await;
    *f.room().active_session.lock().await = Some(host_session());
    let mut response = room_status();
    response["kicked"] = json!(true);
    f.respond("/api/lobby/status", 200, response).await;
    assert!(
        lobby::get_lobby_status("ABC123".into(), Some("Host".into()), f.room())
            .await
            .is_err()
    );
    assert!(f.room().active_session.lock().await.is_none());
}
#[tokio::test]
async fn close_lobby_room_remote_error_is_currently_suppressed() {
    let f = Fixture::new().await;
    *f.room().active_session.lock().await = Some(host_session());
    f.respond("/api/lobby/close", 401, json!({})).await;
    lobby::close_lobby_room("ABC123".into(), f.room())
        .await
        .unwrap();
    assert!(f.room().active_session.lock().await.is_none());
}

#[tokio::test]
async fn net_tunnel_start_success_uses_disposable_test_process_and_stop_releases_it() {
    let f = Fixture::new().await;
    // libtest rejects --secret_path before running tests. This exercises child
    // supervision without a tunnel executable, network, or a game process.
    let response = lobby::net_tunnel_start(
        NetTunnelStartRequest {
            binary_path: std::env::current_exe().unwrap().to_string_lossy().into(),
            secret_key: f.paths.root.join("fixture-secret").to_string_lossy().into(),
        },
        f.room(),
    )
    .await
    .unwrap();
    assert_eq!(response.pid, 0);
    assert!(f.room().supervisor.lock().await.is_some());
    lobby::net_tunnel_stop(f.room()).await.unwrap();
    assert!(f.room().supervisor.lock().await.is_none());
}

// Commands that use APPDATA run in a separate copy of this test executable.
// Never mutate the environment of the parallel parent harness or use real data.
fn isolated_appdata(name: &str) -> bool {
    if std::env::var("WEX_TEST_ISOLATED_APPDATA").as_deref() == Ok(name) {
        return true;
    }
    let temp = tempdir().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .arg(format!("commands::test_gaps::runtime::{name}"))
        .args(["--exact", "--test-threads=1"])
        .env("APPDATA", temp.path())
        .env("WEX_TEST_ISOLATED_APPDATA", name)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    false
}
fn poison_servers(f: &Fixture) {
    let state = f.app.state::<crate::AppState>();
    std::thread::scope(|s| {
        let _ = s
            .spawn(|| {
                let _guard = state.servers.lock().unwrap();
                panic!("fixture poison");
            })
            .join();
    });
}
#[tokio::test]
async fn list_servers_success_clones_managed_state() {
    let f = Fixture::new().await;
    let result = crate::list_servers(f.app.state()).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].address, "mc.auramc.net");
}
#[tokio::test]
async fn list_servers_error_poisoned_mutex() {
    let f = Fixture::new().await;
    poison_servers(&f);
    assert!(crate::list_servers(f.app.state()).is_err());
}
#[tokio::test]
async fn add_server_success_persists_in_isolated_appdata() {
    if !isolated_appdata("add_server_success_persists_in_isolated_appdata") {
        return;
    }
    let f = Fixture::new().await;
    let entry = crate::add_server("Fixture".into(), "127.0.0.1:25565".into(), f.app.state())
        .await
        .unwrap();
    assert_eq!(entry.name, "Fixture");
    assert!(crate::list_servers(f.app.state())
        .unwrap()
        .iter()
        .any(|e| e.address == entry.address));
    let file =
        std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap()).join("Wex/servers.dat");
    let nbt = wex_net::load_servers_dat(&file).unwrap();
    assert!(nbt.get("servers").is_some());
}
#[tokio::test]
async fn add_server_error_poisoned_mutex_has_no_file_writes() {
    let f = Fixture::new().await;
    poison_servers(&f);
    assert!(
        crate::add_server("Fixture".into(), "127.0.0.1".into(), f.app.state())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn remove_server_success_persists_and_preserves_neighbors() {
    if !isolated_appdata("remove_server_success_persists_and_preserves_neighbors") {
        return;
    }
    let f = Fixture::new().await;
    crate::add_server("Fixture".into(), "127.0.0.1:25565".into(), f.app.state())
        .await
        .unwrap();
    crate::remove_server("127.0.0.1:25565".into(), f.app.state()).unwrap();
    let entries = crate::list_servers(f.app.state()).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.address != "127.0.0.1:25565"));
    let file =
        std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap()).join("Wex/servers.dat");
    assert!(wex_net::load_servers_dat(&file)
        .unwrap()
        .get("servers")
        .is_some());
}
#[tokio::test]
async fn remove_server_error_poisoned_mutex_has_no_file_writes() {
    let f = Fixture::new().await;
    poison_servers(&f);
    assert!(crate::remove_server("127.0.0.1".into(), f.app.state()).is_err());
}
#[cfg(windows)]
#[tokio::test]
async fn extras_detect_old_launcher_success_uses_only_isolated_profile() {
    if !isolated_appdata("extras_detect_old_launcher_success_uses_only_isolated_profile") {
        return;
    }
    let dir = std::path::PathBuf::from(std::env::var_os("APPDATA").unwrap()).join(".aura");
    fs::create_dir(&dir).unwrap();
    assert_eq!(
        overview::extras_detect_old_launcher()
            .await
            .unwrap()
            .unwrap(),
        dir.to_string_lossy()
    );
}
#[cfg(windows)]
#[tokio::test]
async fn extras_detect_old_launcher_missing_returns_none_in_isolated_profile() {
    if !isolated_appdata("extras_detect_old_launcher_missing_returns_none_in_isolated_profile") {
        return;
    }
    assert!(overview::extras_detect_old_launcher()
        .await
        .unwrap()
        .is_none());
}
#[tokio::test]
async fn ping_server_success_reads_only_loopback_fixture() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use wex_net::ping::{read_varint, write_varint};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let len = read_varint(&mut socket).await.unwrap();
        let mut handshake = vec![0; len as usize];
        socket.read_exact(&mut handshake).await.unwrap();
        assert_eq!(handshake[0], 0);
        assert_eq!(read_varint(&mut socket).await.unwrap(), 1);
        assert_eq!(read_varint(&mut socket).await.unwrap(), 0);
        let json = br#"{"version":{"name":"1.20.1 / 1.21.1","protocol":763},"players":{"max":8,"online":2},"description":"Fixture"}"#;
        let mut packet = Vec::new();
        write_varint(&mut packet, 0).await.unwrap();
        write_varint(&mut packet, json.len() as i32).await.unwrap();
        packet.extend_from_slice(json);
        write_varint(&mut socket, packet.len() as i32)
            .await
            .unwrap();
        socket.write_all(&packet).await.unwrap();
    });
    let result = crate::ping_server(address.to_string()).await.unwrap();
    server.await.unwrap();
    assert_eq!(result.online, 2);
    assert_eq!(result.max_players, 8);
    assert_eq!(result.motd, "Fixture");
}
#[tokio::test]
async fn ping_server_error_on_closed_loopback_port() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    assert!(crate::ping_server(address.to_string()).await.is_err());
}

#[tokio::test]
async fn start_game_success_and_supervisor_cleanup_both_versions() {
    for version in ["1.20.1", "1.21.1"] {
        let f = Fixture::new().await;
        let response = launch::start_game(
            f.app.handle().clone(),
            version.into(),
            "Fixture".into(),
            None,
            f.play(),
        )
        .await
        .unwrap();
        assert!(response.success);
        assert!(f.play().is_running.load(Ordering::SeqCst));
        assert!(!f.play().is_launching.load(Ordering::SeqCst));
        assert_eq!(
            f.play().running_version.lock().await.as_deref(),
            Some(version)
        );
        launch::kill_game(Some(version.into()), f.play())
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        assert!(f.play().running_version.lock().await.is_none());
    }
}
#[tokio::test]
async fn start_game_error_duplicate_does_not_launch_core() {
    let f = Fixture::new().await;
    f.play().is_launching.store(true, Ordering::SeqCst);
    assert!(launch::start_game(
        f.app.handle().clone(),
        "1.21.1".into(),
        "Fixture".into(),
        None,
        f.play()
    )
    .await
    .is_err());
    assert!(!f.play().is_running.load(Ordering::SeqCst));
}
#[tokio::test]
async fn start_game_error_core_failure_clears_launching_flag() {
    let core = Arc::new(MockLauncherCore::new());
    *core.fail_next_launch.lock().unwrap() = true;
    let f = Fixture::with_core(core).await;
    assert!(launch::start_game(
        f.app.handle().clone(),
        "1.20.1".into(),
        "Fixture".into(),
        None,
        f.play()
    )
    .await
    .is_err());
    assert!(!f.play().is_launching.load(Ordering::SeqCst));
    assert!(!f.play().is_running.load(Ordering::SeqCst));
}
#[tokio::test]
async fn launch_game_success_alias_returns_message() {
    let f = Fixture::new().await;
    assert!(!launch::launch_game(
        f.app.handle().clone(),
        "1.21.1".into(),
        "Fixture".into(),
        f.play()
    )
    .await
    .unwrap()
    .is_empty());
    launch::kill_game(None, f.play()).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(600)).await;
}
#[tokio::test]
async fn launch_game_error_alias_propagates_duplicate_guard() {
    let f = Fixture::new().await;
    f.play().is_launching.store(true, Ordering::SeqCst);
    assert!(launch::launch_game(
        f.app.handle().clone(),
        "1.20.1".into(),
        "Fixture".into(),
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn cancel_launch_success_resets_pending_state() {
    let f = Fixture::new().await;
    f.play().is_launching.store(true, Ordering::SeqCst);
    launch::cancel_launch(f.app.handle().clone(), f.play())
        .await
        .unwrap();
    assert!(!f.play().is_launching.load(Ordering::SeqCst));
}
#[tokio::test]
async fn cancel_launch_core_error_is_currently_suppressed() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    f.play().is_launching.store(true, Ordering::SeqCst);
    launch::cancel_launch(f.app.handle().clone(), f.play())
        .await
        .unwrap();
    assert!(!f.play().is_launching.load(Ordering::SeqCst));
}
#[tokio::test]
async fn shell_window_commands_success_use_only_mock_runtime() {
    let f = Fixture::new().await;
    let view = tauri::WebviewWindowBuilder::new(
        &f.app,
        "fixture",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .build()
    .unwrap();
    let webview: &tauri::Webview<MockRuntime> = view.as_ref();
    let window = webview.window().clone();
    shell::shell_minimize_window(window.clone()).await.unwrap();
    assert!(shell::shell_toggle_maximize_window(window.clone())
        .await
        .unwrap());
    shell::shell_close_window(window).await.unwrap();
}
#[tokio::test]
async fn social_report_achievement_progress_success_unlocks_and_emits_event() {
    use tauri::Listener;
    let f = Fixture::new().await;
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = events.clone();
    f.app
        .listen(wex_ipc::events::EVT_SOCIAL_ACHIEVEMENT_TOAST, move |e| {
            captured.lock().unwrap().push(e.payload().to_owned());
        });
    let result = friends::social_report_achievement_progress(
        f.app.handle().clone(),
        None,
        None,
        Some("first_launch".into()),
        Some(1),
        f.social(),
    )
    .await
    .unwrap();
    assert!(result.unlocked);
    assert_eq!(result.current, 1.0);
    assert_eq!(events.lock().unwrap().len(), 1);
    let listed = friends::social_list_achievements(None, None, f.social())
        .await
        .unwrap();
    assert!(
        listed
            .progress
            .iter()
            .find(|p| p.id == "first_launch")
            .unwrap()
            .unlocked
    );
}
#[tokio::test]
async fn social_report_achievement_progress_error_missing_id_has_no_disk_writes() {
    let f = Fixture::new().await;
    let error = friends::social_report_achievement_progress(
        f.app.handle().clone(),
        None,
        None,
        None,
        None,
        f.social(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, "INVALID_ARGUMENT");
    assert!(!f.paths.root.join("achievements.json").exists());
}
#[tokio::test]
async fn create_lobby_room_success_initializes_host_and_stops_heartbeat() {
    let f = Fixture::new().await;
    f.respond(
        "/api/lobby",
        200,
        json!({"code":"ABC123","hostToken":"fixture","status":"open","createdAt":42}),
    )
    .await;
    f.respond(
        "/api/tunnel-config",
        200,
        json!({"publicAddress":"localhost","publicPort":25565}),
    )
    .await;
    f.respond("/api/lobby/open", 200, json!({})).await;
    f.respond("/api/lobby/heartbeat", 200, json!({})).await;
    f.respond("/api/lobby/close", 200, json!({})).await;
    let room = lobby::create_lobby_room("Fixture world".into(), Some("Fixture".into()), f.room())
        .await
        .unwrap();
    assert_eq!(room.code, "#ABC123");
    assert!(room.is_host);
    assert_eq!(room.tunnel_address.as_deref(), Some("localhost:25565"));
    assert!(f.room().heartbeat_handle.lock().await.is_some());
    lobby::close_lobby_room(room.code, f.room()).await.unwrap();
    assert!(f.room().heartbeat_handle.lock().await.is_none());
    assert!(f.room().active_session.lock().await.is_none());
}
#[tokio::test]
async fn create_lobby_room_error_does_not_create_session_or_heartbeat() {
    let f = Fixture::new().await;
    f.respond("/api/lobby", 503, json!({})).await;
    assert!(lobby::create_lobby_room("Fixture".into(), None, f.room())
        .await
        .is_err());
    assert!(f.room().heartbeat_handle.lock().await.is_none());
    assert!(f.room().active_session.lock().await.is_none());
}
#[tokio::test]
async fn close_lobby_room_success_without_session_is_idempotent() {
    let f = Fixture::new().await;
    lobby::close_lobby_room("ABC123".into(), f.room())
        .await
        .unwrap();
    lobby::close_lobby_room("ABC123".into(), f.room())
        .await
        .unwrap();
    assert!(f.server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn net_tunnel_ensure_binary_error_when_destination_is_file_avoids_download() {
    let f = Fixture::new().await;
    let destination = f.paths.root.join("blocked-tools");
    fs::write(&destination, b"keep").unwrap();
    let error =
        lobby::net_tunnel_ensure_binary(dto(json!({"destination_path":destination})), f.room())
            .await
            .unwrap_err();
    assert_eq!(error.code, "INTERNAL_ERROR");
    assert_eq!(fs::read(&destination).unwrap(), b"keep");
}
#[tokio::test]
async fn workshop_open_packs_folder_error_missing_folder_avoids_explorer() {
    let f = Fixture::new().await;
    let blocker = f.paths.instances_dir.join("fixture-blocked");
    fs::write(&blocker, b"keep").unwrap();
    assert!(workshop_packs::workshop_open_packs_folder(
        "shaderpack".into(),
        Some("fixture-blocked".into()),
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn workshop_restore_world_backup_error_missing_archive_preserves_existing_world() {
    let f = Fixture::new().await;
    let target = f.paths.root.join("existing");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), b"keep").unwrap();
    assert!(workshop::workshop_restore_world_backup(
        f.paths.root.join("missing.zip").to_string_lossy().into(),
        Some(target.to_string_lossy().into()),
        None,
        f.play()
    )
    .await
    .is_err());
    assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"keep");
}
