use super::*;
use tauri::Listener;

#[tokio::test]
async fn lan_session_finishes_and_records_playtime_for_both_versions() {
    for version in ["1.20.1", "1.21.1"] {
        let core = Arc::new(MockLauncherCore::new());
        let f = Fixture::with_core(core.clone()).await;
        let result = lobby::connect_lan_world(
            f.app.handle().clone(),
            "127.0.0.1:54321".into(),
            Some(version.into()),
            Some("Guest".into()),
            f.play(),
        )
        .await
        .unwrap();
        assert!(!result.is_empty());
        assert!(f.play().is_running.load(Ordering::SeqCst));
        // Stop only the mock session; no real Minecraft process is ever created.
        core.kill_game(version).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while f.play().is_running.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        assert!(f.paths.playtime_file.is_file());
        assert!(f.play().running_version.lock().await.is_none());
    }
}

#[tokio::test]
async fn verified_join_sends_local_manifest_and_blocks_unverified_host() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/join", 200, json!({"success":true,"code":"ABC123","status":"open","tunnelAddress":"test.ply.gg:12345","playerCount":2})).await;
    f.respond("/api/lobby/status", 200, json!({"code":"ABC123","status":"open","tunnelAddress":"test.ply.gg:12345","playerCount":2,"lastHeartbeat":0,"players":["Host","Guest"],"hostName":"Host"})).await;
    let result = lobby::prepare_lobby_join(
        f.app.handle().clone(),
        "ABC123".into(),
        "Guest".into(),
        "1.21.1".into(),
        false,
        f.play(),
        f.room(),
    )
    .await;
    assert!(result.unwrap_err().contains("проверки модов"));
    assert!(!f.play().is_launching.load(Ordering::SeqCst));
    let requests = f.server.received_requests().await.unwrap();
    let join = requests
        .iter()
        .find(|r| r.url.path() == "/api/lobby/join")
        .unwrap();
    assert_eq!(join.body_json::<Value>().unwrap()["manifest"], json!([]));
}

#[tokio::test]
async fn repair_toggles_real_extra_mod_and_rechecks_manifest_before_allowing_launch() {
    use std::io::Write;
    use std::sync::atomic::AtomicUsize;
    for version in ["1.20.1", "1.21.1"] {
        let f = Fixture::new().await;
        let jar = f.paths.instance_dir(version).join("mods/fixture-extra.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&jar).unwrap());
        zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"id":"fixture-extra","name":"Fixture","version":"1.0"}"#)
            .unwrap();
        zip.finish().unwrap();
        f.respond("/api/lobby/join", 200, json!({"success":true,"code":"ABC123","status":"open","tunnelAddress":"test.ply.gg:12345","playerCount":2})).await;
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let mismatch = wex_net::ModMismatchItem {
            mod_id: "fixture-extra".into(),
            mod_name: "Fixture".into(),
            mismatch_type: wex_net::ModMismatchType::Extra,
            type_ru: "лишний".into(),
            is_fixable: true,
            player_version: Some("1.0".into()),
            host_version: None,
            display_description: "fixture".into(),
        };
        Mock::given(path("/api/lobby/status")).respond_with(move |_: &wiremock::Request| {
            let repaired = count.fetch_add(1, Ordering::SeqCst) >= 2;
            ResponseTemplate::new(200).set_body_json(json!({
                "code":"ABC123","status":"open","tunnelAddress":"test.ply.gg:12345","playerCount":2,"lastHeartbeat":0,
                "players":["Host","Guest"],"hostName":"Host",
                "modSync":{"Guest":{"status":if repaired {"synced"} else {"mismatch"},
                    "mismatches":if repaired {json!([])} else {json!([mismatch.clone()])}}}
            }))
        }).mount(&f.server).await;
        let events = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
        let captured = events.clone();
        f.app.listen("lobby-join-progress", move |event| {
            captured
                .lock()
                .unwrap()
                .push(serde_json::from_str(event.payload()).unwrap());
        });
        let room = lobby::prepare_lobby_join(
            f.app.handle().clone(),
            "ABC123".into(),
            "Guest".into(),
            version.into(),
            true,
            f.play(),
            f.room(),
        )
        .await
        .unwrap();
        assert_eq!(room.mod_mismatch_count, 0);
        assert!(!jar.exists());
        assert!(jar.with_extension("jar.disabled").exists());
        let stages: Vec<String> = events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e["stage"].as_str().unwrap().to_owned())
            .collect();
        assert_eq!(
            stages,
            ["checking_mods", "downloading_mods", "checking_mods"]
        );
        assert!(
            !f.play().is_running.load(Ordering::SeqCst),
            "joining metadata must not claim game launch"
        );
        let requests = f.server.received_requests().await.unwrap();
        let joins: Vec<Value> = requests
            .iter()
            .filter(|r| r.url.path() == "/api/lobby/join")
            .map(|r| r.body_json().unwrap())
            .collect();
        assert_eq!(joins.len(), 2);
        assert_eq!(joins[0]["manifest"][0]["enabled"], true);
        assert_eq!(joins[1]["manifest"][0]["enabled"], false);
    }
}

struct RecordingTunnel(Arc<std::sync::Mutex<Vec<u16>>>);
#[async_trait::async_trait]
impl super::super::lobby_tunnel::TunnelRuntime for RecordingTunnel {
    async fn start(
        &self,
        _: &lobby::LobbyState,
        config: &wex_net::lobby::TunnelConfigResponse,
        port: u16,
    ) -> Result<String, String> {
        self.0.lock().unwrap().push(port);
        Ok(format!(
            "{}:{}",
            config.public_address.as_deref().unwrap(),
            config.public_port.unwrap()
        ))
    }
}

#[tokio::test]
async fn publish_and_retry_preserve_actual_lan_port_and_propagate_open_failure() {
    let temp = tempdir().unwrap();
    let paths = WexPaths::new(Some(temp.path().into()));
    paths.ensure_dirs().unwrap();
    paths.ensure_instance_dirs("1.21.1").unwrap();
    let server = MockServer::start().await;
    let ports = Arc::new(std::sync::Mutex::new(Vec::new()));
    let state = lobby::LobbyState::new(paths.root.join("tools"), &server.uri(), &server.uri())
        .with_tunnel_runtime(Arc::new(RecordingTunnel(ports.clone())));
    let app = mock_builder()
        .manage(state)
        .manage(launch::PlayState::new_with_core(
            paths.clone(),
            Arc::new(MockLauncherCore::new()),
        ))
        .build(mock_context(noop_assets()))
        .unwrap();
    Mock::given(path("/api/lobby"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"code":"ABC123","hostToken":"fixture","status":"created","createdAt":0}),
        ))
        .mount(&server)
        .await;
    Mock::given(path("/api/tunnel-config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"secret":"fixture","publicAddress":"test.ply.gg","publicPort":38062}),
        ))
        .mount(&server)
        .await;
    Mock::given(path("/api/lobby/open")).and(body_json(
        json!({"code":"ABC123","hostToken":"fixture","tunnelAddress":"test.ply.gg:38062","port":54321})))
        .respond_with(ResponseTemplate::new(200)).expect(1).mount(&server).await;
    let room = lobby::publish_lan_world(
        "World".into(),
        54321,
        Some("Host".into()),
        Some("1.21.1".into()),
        app.state(),
        app.state(),
    )
    .await
    .unwrap();
    assert!(room.tunnel_active);
    assert_eq!(*ports.lock().unwrap(), vec![54321]);
    server.reset().await;
    Mock::given(path("/api/tunnel-config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"secret":"fixture","publicAddress":"test.ply.gg","publicPort":38062}),
        ))
        .mount(&server)
        .await;
    Mock::given(path("/api/lobby/open"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    assert!(lobby::retry_lobby_tunnel("ABC123".into(), app.state())
        .await
        .is_err());
    assert_eq!(*ports.lock().unwrap(), vec![54321, 54321]);
    assert!(app
        .state::<lobby::LobbyState>()
        .active_session
        .lock()
        .await
        .as_ref()
        .unwrap()
        .tunnel_address
        .is_none());
    assert!(matches!(
        *app.state::<lobby::LobbyState>()
            .current_tunnel_status
            .lock()
            .await,
        wex_net::TunnelStatus::Error(_)
    ));
    lobby::close_lobby_room("ABC123".into(), app.state())
        .await
        .unwrap();
    assert!(matches!(
        *app.state::<lobby::LobbyState>()
            .current_tunnel_status
            .lock()
            .await,
        wex_net::TunnelStatus::Stopped
    ));
    assert!(app
        .state::<lobby::LobbyState>()
        .local_port
        .lock()
        .await
        .is_none());
}
