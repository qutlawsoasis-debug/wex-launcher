use super::*;

#[tokio::test]
async fn extras_analyze_crash_success_classifies_input_for_both_versions() {
    let f = Fixture::new().await;
    for version in ["1.20.1", "1.21.1"] {
        let result = super::super::crash::extras_analyze_crash(
            ExtrasAnalyzeCrashRequest {
                exit_code: Some(1),
                log_text: Some("java.lang.OutOfMemoryError: Java heap space".into()),
                crash_report_text: None,
            },
            Some(version.into()),
            f.play(),
        )
        .await
        .unwrap();
        assert_eq!(result.kind, CrashKindDto::OutOfMemory);
    }
}
#[tokio::test]
async fn extras_analyze_crash_missing_logs_returns_unknown_fallback() {
    let f = Fixture::new().await;
    let result = super::super::crash::extras_analyze_crash(
        ExtrasAnalyzeCrashRequest {
            exit_code: Some(17),
            log_text: None,
            crash_report_text: None,
        },
        None,
        f.play(),
    )
    .await
    .unwrap();
    assert_eq!(
        result.kind,
        CrashKindDto::Unknown {
            exit_code: Some(17)
        }
    );
}
#[tokio::test]
async fn crash_get_last_diagnosis_success_uses_only_instance_logs() {
    let f = Fixture::new().await;
    for version in ["1.20.1", "1.21.1"] {
        fs::write(
            f.paths.instance_dir(version).join("logs/latest.log"),
            b"java.lang.OutOfMemoryError: Java heap space",
        )
        .unwrap();
        let result =
            super::super::crash::crash_get_last_diagnosis(Some(version.into()), Some(1), f.play())
                .await
                .unwrap();
        assert_eq!(result.kind, CrashKindDto::OutOfMemory);
    }
}
#[tokio::test]
async fn crash_get_last_diagnosis_missing_logs_returns_unknown_fallback() {
    let f = Fixture::new().await;
    let result = super::super::crash::crash_get_last_diagnosis(None, Some(2), f.play())
        .await
        .unwrap();
    assert_eq!(result.kind, CrashKindDto::Unknown { exit_code: Some(2) });
}
#[tokio::test]
async fn crash_disable_mod_success_preserves_jar_and_other_version() {
    let f = Fixture::new().await;
    for version in ["1.20.1", "1.21.1"] {
        fs::write(
            f.paths.instance_dir(version).join("mods/fixture-1.jar"),
            b"jar fixture",
        )
        .unwrap();
    }
    let result =
        super::super::crash::crash_disable_mod("fixture".into(), Some("1.20.1".into()), f.play())
            .await
            .unwrap();
    assert_eq!(result, "fixture-1.jar.disabled");
    assert_eq!(
        fs::read(
            f.paths
                .instance_dir("1.20.1")
                .join("mods/fixture-1.jar.disabled")
        )
        .unwrap(),
        b"jar fixture"
    );
    assert!(f
        .paths
        .instance_dir("1.21.1")
        .join("mods/fixture-1.jar")
        .exists());
}
#[tokio::test]
async fn crash_disable_mod_error_missing_mod_does_not_rename_neighbors() {
    let f = Fixture::new().await;
    let neighbor = f.paths.instance_dir("1.21.1").join("mods/neighbor.jar");
    fs::write(&neighbor, b"keep").unwrap();
    assert!(
        super::super::crash::crash_disable_mod("missing".into(), None, f.play())
            .await
            .is_err()
    );
    assert!(neighbor.exists());
}
#[tokio::test]
async fn crash_open_log_file_error_blocked_logs_directory_avoids_explorer() {
    let f = Fixture::new().await;
    let logs = f.paths.instance_dir("1.21.1").join("logs");
    fs::remove_dir(&logs).unwrap();
    fs::write(&logs, b"keep").unwrap();
    assert!(super::super::crash::crash_open_log_file(None, f.play())
        .await
        .is_err());
    assert_eq!(fs::read(logs).unwrap(), b"keep");
}

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
async fn security_regression_net_tunnel_start_rejects_tampered_fixed_binary() {
    let f = Fixture::new().await;
    let tools = f.room().tools_dir.clone();
    fs::create_dir_all(&tools).unwrap();
    let binary = wex_net::tunnel::playit_binary_path(&tools);
    fs::write(&binary, b"tampered executable").unwrap();
    let error = lobby::net_tunnel_start(NetTunnelStartRequest {}, f.room())
        .await
        .unwrap_err();
    assert_eq!(error.code, "INTERNAL_ERROR");
    assert!(f.room().supervisor.lock().await.is_none());
    assert_eq!(fs::read(binary).unwrap(), b"tampered executable");
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
#[allow(dead_code)]
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
    let result = crate::list_servers(None, f.play()).await.unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].address, "mc.auramc.net");
}
#[tokio::test]
async fn list_servers_error_corrupt_file() {
    let f = Fixture::new().await;
    let path = f.paths.instance_dir("1.21.1").join("servers.dat");
    fs::write(path, b"not valid nbt").unwrap();
    assert!(crate::list_servers(Some("1.21.1".into()), f.play())
        .await
        .is_err());
}
#[tokio::test]
async fn add_server_success_persists_in_isolated_appdata() {
    let f = Fixture::new().await;
    let entry = crate::add_server(None, "Fixture".into(), "127.0.0.1:25565".into(), f.play())
        .await
        .unwrap();
    assert_eq!(entry.name, "Fixture");
    assert!(crate::list_servers(None, f.play())
        .await
        .unwrap()
        .iter()
        .any(|e| e.address == entry.address));
}
#[tokio::test]
async fn add_server_error_validation_empty_address() {
    let f = Fixture::new().await;
    assert!(
        crate::add_server(None, "Fixture".into(), "".into(), f.play())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn remove_server_success_persists_and_preserves_neighbors() {
    let f = Fixture::new().await;
    crate::add_server(None, "Fixture".into(), "127.0.0.1:25565".into(), f.play())
        .await
        .unwrap();
    crate::remove_server(None, "127.0.0.1:25565".into(), f.play())
        .await
        .unwrap();
    let entries = crate::list_servers(None, f.play()).await.unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().all(|e| e.address != "127.0.0.1:25565"));
}
#[tokio::test]
async fn remove_server_error_corrupt_file() {
    let f = Fixture::new().await;
    let path = f.paths.instance_dir("1.21.1").join("servers.dat");
    fs::write(path, b"not valid nbt").unwrap();
    assert!(
        crate::remove_server(Some("1.21.1".into()), "127.0.0.1".into(), f.play())
            .await
            .is_err()
    );
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
    let result = crate::ping_server(address.to_string(), Some(1000))
        .await
        .unwrap();
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
    assert!(crate::ping_server(address.to_string(), Some(100))
        .await
        .is_err());
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
        let mut cleaned_up = false;
        for _ in 0..30 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if f.play().running_version.lock().await.is_none() {
                cleaned_up = true;
                break;
            }
        }
        assert!(cleaned_up);
    }
}
// Regression: the startup verification / prewarm installs in the background.
// Play must wait for it instead of installing the same version at the same time.
#[tokio::test]
async fn start_game_waits_for_background_install_both_versions() {
    for version in ["1.20.1", "1.21.1"] {
        let f = Fixture::new().await;
        let background_install = f.play().install_lock.clone().lock_owned().await;
        let play = f.play();
        let release = async {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            assert!(
                !play.is_running.load(Ordering::SeqCst),
                "Play started while the background install still held the lock"
            );
            assert!(play.is_launching.load(Ordering::SeqCst));
            drop(background_install);
        };
        let (response, ()) = tokio::join!(
            launch::start_game(
                f.app.handle().clone(),
                version.into(),
                "Fixture".into(),
                None,
                None,
                f.play(),
            ),
            release
        );
        assert!(response.unwrap().success);
        assert!(f.play().is_running.load(Ordering::SeqCst));
        assert!(
            f.play().install_lock.try_lock().is_ok(),
            "Play must release the install lock"
        );
        launch::kill_game(Some(version.into()), f.play())
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn start_game_cancelled_while_waiting_for_background_install_does_not_launch() {
    let f = Fixture::new().await;
    let background_install = f.play().install_lock.clone().lock_owned().await;
    let cancel = async {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        launch::cancel_launch(f.app.handle().clone(), f.play())
            .await
            .unwrap();
        drop(background_install);
    };
    let (response, ()) = tokio::join!(
        launch::start_game(
            f.app.handle().clone(),
            "1.21.1".into(),
            "Fixture".into(),
            None,
            None,
            f.play(),
        ),
        cancel
    );
    assert!(response.is_err());
    assert!(!f.play().is_running.load(Ordering::SeqCst));
    assert!(!f.play().is_launching.load(Ordering::SeqCst));
}
/// Starts a game whose instance log contains a GPU crash signature and records
/// the launch-status and game-crash events emitted after it ends.
async fn run_session_with_gpu_log(
    f: &Fixture,
    version: &str,
    user_stop: bool,
) -> (Vec<String>, usize) {
    use tauri::Listener;
    let logs = f.paths.instance_dir(version).join("logs");
    fs::create_dir_all(&logs).unwrap();
    fs::write(logs.join("latest.log"), "[main/ERROR]: GLFW error 65543\n").unwrap();
    let statuses = Arc::new(std::sync::Mutex::new(Vec::new()));
    let crashes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let captured = statuses.clone();
    f.app.listen(wex_ipc::events::EVT_LAUNCH_STATUS, move |e| {
        captured.lock().unwrap().push(e.payload().to_owned());
    });
    let crash_count = crashes.clone();
    f.app.listen("game-crash", move |_| {
        crash_count.fetch_add(1, Ordering::SeqCst);
    });
    launch::start_game(
        f.app.handle().clone(),
        version.into(),
        "Fixture".into(),
        None,
        None,
        f.play(),
    )
    .await
    .unwrap();
    if user_stop {
        launch::kill_game(Some(version.into()), f.play())
            .await
            .unwrap();
    } else {
        // The game process ends on its own (crash), without a stop request.
        f.play().core.kill_game(version).await.unwrap();
    }
    // The supervisor polls every 500 ms; wait for its final idle/error status.
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if statuses
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.contains("\"idle\"") || s.contains("\"error\""))
        {
            break;
        }
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let statuses = statuses.lock().unwrap().clone();
    (statuses, crashes.load(Ordering::SeqCst))
}
#[tokio::test]
async fn stop_button_is_not_reported_as_crash_both_versions() {
    for version in ["1.20.1", "1.21.1"] {
        let f = Fixture::new().await;
        let (statuses, crashes) = run_session_with_gpu_log(&f, version, true).await;
        assert_eq!(
            crashes, 0,
            "a user stop must not open the crash sheet: {statuses:?}"
        );
        let last = statuses.last().expect("a final launch status");
        assert!(last.contains("\"idle\""), "{last}");
        assert!(last.contains("Игра остановлена"), "{last}");
        assert!(!f.play().stop_requested.load(Ordering::SeqCst));
    }
}
#[tokio::test]
async fn game_exit_without_stop_still_reports_crash() {
    let f = Fixture::new().await;
    let (statuses, crashes) = run_session_with_gpu_log(&f, "1.21.1", false).await;
    assert_eq!(crashes, 1, "{statuses:?}");
    assert!(statuses.last().unwrap().contains("\"error\""));
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
        None,
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
        None,
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
async fn net_tunnel_ensure_binary_error_when_backend_tools_is_file_avoids_download() {
    let f = Fixture::new().await;
    let destination = f.room().tools_dir.clone();
    fs::write(&destination, b"keep").unwrap();
    let error = lobby::net_tunnel_ensure_binary(NetTunnelEnsureBinaryRequest {}, f.room())
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

#[tokio::test]
async fn review_09_lan_respects_shared_launch_guard() {
    let f = Fixture::new().await;
    f.play().is_launching.store(true, Ordering::SeqCst);
    assert!(lobby::connect_lan_world(
        f.app.handle().clone(),
        "127.0.0.1:54321".into(),
        Some("1.21.1".into()),
        Some("Guest".into()),
        f.play()
    )
    .await
    .is_err());
    assert!(!f.play().is_running.load(Ordering::SeqCst));
}

#[tokio::test]
async fn review_26_failed_lan_bind_does_not_publish_running_state() {
    let occupied = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let service = wex_net::LanDiscoveryService::new_with_bind(
        std::time::Duration::from_secs(1),
        occupied.local_addr().unwrap(),
        None,
    );
    let _ = service.start().await;
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    assert!(!service.is_running().await);
    drop(occupied);
    let _ = service.start().await;
    assert!(service.is_running().await);
    service.stop().await;
}
