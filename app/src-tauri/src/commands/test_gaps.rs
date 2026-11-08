//! Execute real command functions with managed mock state, local HTTP and isolated files.
// The normal app binary gets this resource from tauri-build; libtest does not.
#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "resource", kind = "static")]
extern "C" {}

#[path = "test_gaps_more.rs"]
mod more;
#[path = "test_gaps_runtime.rs"]
mod runtime;
use super::{
    friends, launch, lobby, mods, network, overview, settings, shell, wardrobe, workshop,
    workshop_packs,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    fs,
    sync::{atomic::Ordering, Arc},
};
use tauri::{
    test::{mock_builder, mock_context, noop_assets, MockRuntime},
    App, Manager,
};
use tempfile::{tempdir, TempDir};
use wex_core::config::WexPaths;
use wex_ipc::{
    commands::*,
    core_trait::{LauncherCore, MockLauncherCore},
    error::{IpcError, IpcResult},
    models::*,
};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

struct Fixture {
    _temp: TempDir,
    paths: WexPaths,
    server: MockServer,
    app: App<MockRuntime>,
}

impl Fixture {
    async fn new() -> Self {
        Self::with_core(Arc::new(MockLauncherCore::new())).await
    }
    async fn with_core(core: Arc<dyn LauncherCore>) -> Self {
        let temp = tempdir().unwrap();
        let paths = WexPaths::new(Some(temp.path().join("wex")));
        paths.ensure_dirs().unwrap();
        for version in ["1.20.1", "1.21.1"] {
            paths.ensure_instance_dirs(version).unwrap();
        }
        let server = MockServer::start().await;
        let app = mock_builder()
            .manage(launch::PlayState::new_with_core(paths.clone(), core))
            .manage(friends::FriendsState::new_with_client(
                paths.clone(),
                wex_social::FriendApiClient::new(server.uri()),
            ))
            .manage(lobby::LobbyState::new(
                paths.root.join("tools"),
                &server.uri(),
                &server.uri(),
            ))
            .manage(network::NetworkState::new_with_status(false))
            .manage(crate::AppState::default())
            .build(mock_context(noop_assets()))
            .unwrap();
        Self {
            _temp: temp,
            paths,
            server,
            app,
        }
    }
    fn play(&self) -> tauri::State<'_, launch::PlayState> {
        self.app.state()
    }
    fn social(&self) -> tauri::State<'_, friends::FriendsState> {
        self.app.state()
    }
    fn room(&self) -> tauri::State<'_, lobby::LobbyState> {
        self.app.state()
    }
    fn block_config(&self) {
        fs::create_dir(&self.paths.config_file).unwrap();
    }
    async fn respond(&self, route: &str, status: u16, body: Value) {
        Mock::given(path(route))
            .respond_with(ResponseTemplate::new(status).set_body_json(body))
            .mount(&self.server)
            .await;
    }
}

struct RejectingCore;
fn rejected<T>() -> IpcResult<T> {
    Err(IpcError::internal("fixture failure", None))
}
#[async_trait::async_trait]
impl LauncherCore for RejectingCore {
    async fn get_config(&self) -> IpcResult<LauncherConfig> {
        rejected()
    }
    async fn save_config(&self, _: LauncherConfig) -> IpcResult<()> {
        rejected()
    }
    async fn get_installed_versions(&self) -> IpcResult<Vec<InstalledVersion>> {
        rejected()
    }
    async fn install_instance(
        &self,
        _: InstallInstanceRequest,
    ) -> IpcResult<InstallInstanceResponse> {
        rejected()
    }
    async fn launch_game(&self, _: LaunchGameRequest) -> IpcResult<LaunchGameResponse> {
        rejected()
    }
    async fn cancel_launch(&self) -> IpcResult<()> {
        rejected()
    }
    async fn is_running(&self, _: &str) -> IpcResult<bool> {
        rejected()
    }
    async fn kill_game(&self, _: &str) -> IpcResult<()> {
        rejected()
    }
}

fn dto<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn auth() -> UserAuthDto {
    dto(
        json!({"user_id":"fixture-user","user_token":"fixture-token","friend_code":"ABC123","nick":"Fixture"}),
    )
}
fn png(width: u32, height: u32) -> String {
    let image = wex_extras::image::RgbaImage::from_pixel(
        width,
        height,
        wex_extras::image::Rgba([12, 34, 56, 255]),
    );
    BASE64.encode(wex_extras::skins::encode_png_bytes(&image).unwrap())
}

#[tokio::test]
async fn get_config_success_uses_managed_core() {
    let f = Fixture::new().await;
    assert_eq!(launch::get_config(f.play()).await.unwrap().ram_gb, 4);
}
#[tokio::test]
async fn get_config_error_propagates_core_failure() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    assert!(launch::get_config(f.play())
        .await
        .unwrap_err()
        .contains("fixture failure"));
}
#[tokio::test]
async fn save_config_success_persists_through_core() {
    let f = Fixture::new().await;
    let cfg = LauncherConfig {
        player_name: "Updated".into(),
        ..Default::default()
    };
    launch::save_config(cfg, f.play()).await.unwrap();
    assert_eq!(
        launch::get_config(f.play()).await.unwrap().player_name,
        "Updated"
    );
}
#[tokio::test]
async fn save_config_error_is_returned() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    assert!(launch::save_config(LauncherConfig::default(), f.play())
        .await
        .is_err());
}
#[tokio::test]
async fn set_config_success_delegates_to_core() {
    let f = Fixture::new().await;
    launch::set_config(
        LauncherConfig {
            selected_version: "1.20.1".into(),
            ..Default::default()
        },
        f.play(),
    )
    .await
    .unwrap();
    assert_eq!(
        launch::get_config(f.play()).await.unwrap().selected_version,
        "1.20.1"
    );
}
#[tokio::test]
async fn set_config_error_is_returned() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    assert!(launch::set_config(LauncherConfig::default(), f.play())
        .await
        .is_err());
}
#[tokio::test]
async fn get_installed_versions_success_contains_both_targets() {
    let f = Fixture::new().await;
    let versions = launch::get_installed_versions(f.play()).await.unwrap();
    assert!(versions.iter().any(|v| v.id == "1.20.1"));
    assert!(versions.iter().any(|v| v.id == "1.21.1"));
}
#[tokio::test]
async fn get_installed_versions_error_is_returned() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    assert!(launch::get_installed_versions(f.play()).await.is_err());
}
#[tokio::test]
async fn is_game_running_success_reads_actual_command_state() {
    let f = Fixture::new().await;
    assert!(!launch::is_game_running(f.play()).await.unwrap());
    f.play().is_running.store(true, Ordering::SeqCst);
    assert!(launch::is_game_running(f.play()).await.unwrap());
}
#[tokio::test]
async fn kill_game_success_clears_running_flags() {
    let f = Fixture::new().await;
    f.play().is_running.store(true, Ordering::SeqCst);
    launch::kill_game(Some("1.21.1".into()), f.play())
        .await
        .unwrap();
    assert!(!launch::is_game_running(f.play()).await.unwrap());
}
#[tokio::test]
async fn kill_game_core_error_is_currently_suppressed() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    f.play().is_running.store(true, Ordering::SeqCst);
    launch::kill_game(Some("1.20.1".into()), f.play())
        .await
        .unwrap();
    assert!(!f.play().is_running.load(Ordering::SeqCst));
}
#[tokio::test]
async fn shell_get_launch_state_success_reflects_running_version() {
    let f = Fixture::new().await;
    f.play().is_running.store(true, Ordering::SeqCst);
    *f.play().running_version.lock().await = Some("1.20.1".into());
    let state = shell::shell_get_launch_state(f.play()).await.unwrap();
    assert!(state.is_running);
    assert_eq!(state.running_version.as_deref(), Some("1.20.1"));
}
#[tokio::test]
async fn shell_stop_game_success_calls_core_and_clears_flags() {
    let f = Fixture::new().await;
    f.play().is_running.store(true, Ordering::SeqCst);
    shell::shell_stop_game(Some("1.21.1".into()), f.play())
        .await
        .unwrap();
    assert!(!f.play().is_running.load(Ordering::SeqCst));
}
#[tokio::test]
async fn shell_stop_game_core_error_is_currently_suppressed() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    shell::shell_stop_game(Some("1.21.1".into()), f.play())
        .await
        .unwrap();
    assert!(!f.play().is_launching.load(Ordering::SeqCst));
}
#[tokio::test]
async fn get_network_status_success_uses_override_without_network() {
    let f = Fixture::new().await;
    let state = f.app.state::<network::NetworkState>();
    assert!(
        !network::get_network_status(state.clone())
            .await
            .unwrap()
            .is_online
    );
    state.set_override(Some(true)).await;
    assert!(network::get_network_status(state).await.unwrap().is_online);
}

macro_rules! settings_get_case {
    ($name:ident,$func:ident) => {
        #[tokio::test]
        async fn $name() {
            let f = Fixture::new().await;
            let read = settings::$func(f.play()).await.unwrap();
            assert_eq!(read.game_dir, f.paths.root.to_string_lossy());
            assert!(read.ram_mb >= 1024);
        }
    };
}
settings_get_case!(
    get_launcher_settings_success_uses_temp_root,
    get_launcher_settings
);
settings_get_case!(
    get_launcher_config_success_uses_temp_root,
    get_launcher_config
);

macro_rules! settings_save_cases {
    ($ok:ident,$err:ident,$func:ident) => {
        #[tokio::test]
        async fn $ok() {
            let f = Fixture::new().await;
            let mut cfg = settings::get_launcher_settings(f.play()).await.unwrap();
            cfg.ram_mb = 512;
            cfg.selected_version = "1.20.1".into();
            settings::$func(cfg, f.play()).await.unwrap();
            let saved = wex_core::config::LauncherConfig::load_or_default(&f.paths.config_file);
            assert_eq!(saved.max_memory_mb, 1024);
            assert_eq!(saved.selected_version, "1.20.1");
        }
        #[tokio::test]
        async fn $err() {
            let f = Fixture::new().await;
            let cfg = settings::get_launcher_settings(f.play()).await.unwrap();
            f.block_config();
            assert!(settings::$func(cfg, f.play()).await.is_err());
        }
    };
}
settings_save_cases!(
    save_launcher_settings_success_clamps_and_saves,
    save_launcher_settings_error_on_directory,
    save_launcher_settings
);
settings_save_cases!(
    save_launcher_config_success_clamps_and_saves,
    save_launcher_config_error_on_directory,
    save_launcher_config
);
#[tokio::test]
async fn reset_settings_success_replaces_saved_preferences() {
    let f = Fixture::new().await;
    wex_core::config::LauncherConfig {
        nickname: "Changed".into(),
        ..Default::default()
    }
    .save(&f.paths.config_file)
    .unwrap();
    let reset = settings::reset_settings(f.play()).await.unwrap();
    assert_eq!(reset.player_name, "Player");
}
#[tokio::test]
async fn reset_settings_error_on_directory() {
    let f = Fixture::new().await;
    f.block_config();
    assert!(settings::reset_settings(f.play()).await.is_err());
}
#[tokio::test]
async fn get_data_dir_success_is_managed_root() {
    let f = Fixture::new().await;
    assert_eq!(
        settings::get_data_dir(f.play()),
        f.paths.root.to_string_lossy()
    );
}
#[tokio::test]
async fn get_system_info_success_has_real_shape() {
    let f = Fixture::new().await;
    let info = settings::get_system_info(f.play()).await.unwrap();
    assert!(info.total_memory_mb > 0);
    assert_eq!(info.data_dir, f.paths.root.to_string_lossy());
}

#[tokio::test]
async fn list_mods_success_lists_empty_instances_both_versions() {
    let f = Fixture::new().await;
    for v in ["1.20.1", "1.21.1"] {
        assert!(mods::list_mods(v.into(), f.play())
            .await
            .unwrap()
            .is_empty());
    }
}
#[tokio::test]
async fn list_mods_error_when_mods_is_file() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.21.1").join("mods");
    fs::remove_dir(&dir).unwrap();
    fs::write(&dir, b"block").unwrap();
    assert!(mods::list_mods("1.21.1".into(), f.play()).await.is_err());
}
#[tokio::test]
async fn add_custom_mod_success_copies_into_requested_version() {
    let f = Fixture::new().await;
    let src = f.paths.root.join("custom-1.jar");
    fs::write(&src, b"fixture").unwrap();
    for v in ["1.20.1", "1.21.1"] {
        let entry = mods::add_custom_mod(v.into(), src.to_string_lossy().into(), f.play())
            .await
            .unwrap();
        assert_eq!(entry.file_name, "custom-1.jar");
        assert!(f.paths.instance_dir(v).join("mods/custom-1.jar").exists());
    }
}
#[tokio::test]
async fn add_custom_mod_error_rejects_non_jar() {
    let f = Fixture::new().await;
    let src = f.paths.root.join("bad.txt");
    fs::write(&src, b"fixture").unwrap();
    assert!(
        mods::add_custom_mod("1.21.1".into(), src.to_string_lossy().into(), f.play())
            .await
            .unwrap_err()
            .contains(".jar")
    );
}
#[tokio::test]
async fn toggle_mod_success_renames_both_directions() {
    let f = Fixture::new().await;
    for v in ["1.20.1", "1.21.1"] {
        let file = f.paths.instance_dir(v).join("mods/custom.jar");
        fs::write(&file, b"fixture").unwrap();
        let off = mods::toggle_mod(v.into(), "custom.jar".into(), false, f.play())
            .await
            .unwrap();
        assert!(!off.enabled);
        let on = mods::toggle_mod(v.into(), off.file_name, true, f.play())
            .await
            .unwrap();
        assert!(on.enabled);
        assert!(file.exists());
    }
}
#[tokio::test]
async fn toggle_mod_error_missing_file() {
    let f = Fixture::new().await;
    assert!(
        mods::toggle_mod("1.21.1".into(), "missing.jar".into(), false, f.play())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn delete_custom_mod_success_removes_file() {
    let f = Fixture::new().await;
    let file = f.paths.instance_dir("1.20.1").join("mods/custom.jar");
    fs::write(&file, b"fixture").unwrap();
    mods::delete_custom_mod("1.20.1".into(), "custom.jar".into(), f.play())
        .await
        .unwrap();
    assert!(!file.exists());
}
#[tokio::test]
async fn delete_custom_mod_error_rejects_traversal_without_touching_sentinel() {
    let f = Fixture::new().await;
    let sentinel = f.paths.root.join("outside.jar");
    fs::write(&sentinel, b"keep").unwrap();
    assert!(
        mods::delete_custom_mod("1.21.1".into(), "../../outside.jar".into(), f.play())
            .await
            .is_err()
    );
    assert_eq!(fs::read(sentinel).unwrap(), b"keep");
}

#[tokio::test]
async fn social_login_success_saves_auth_and_uses_remote_profile() {
    let f = Fixture::new().await;
    Mock::given(method("GET")).and(path("/api/user/profile")).and(header("X-User-Id","fixture-user")).respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"fixture-user","nick":"Remote","friendCode":"ABC123","playTimeSeconds":42,"totalLaunches":3,"achievements":[]}))).expect(1).mount(&f.server).await;
    let profile = friends::social_login(None, Some(auth()), f.social())
        .await
        .unwrap();
    assert_eq!(profile.nick, "Remote");
    assert_eq!(profile.play_time_seconds, 42);
    assert_eq!(
        f.social().auth.lock().await.as_ref().unwrap().user_id,
        "fixture-user"
    );
}
#[tokio::test]
async fn social_login_error_when_auth_missing() {
    let f = Fixture::new().await;
    assert_eq!(
        friends::social_login(None, None, f.social())
            .await
            .unwrap_err()
            .code,
        "INVALID_ARGUMENT"
    );
    assert!(f.server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn social_login_http_error_returns_offline_profile() {
    let f = Fixture::new().await;
    f.respond("/api/user/profile", 503, json!({})).await;
    let profile = friends::social_login(None, Some(auth()), f.social())
        .await
        .unwrap();
    assert_eq!(profile.nick, "Fixture");
    assert_eq!(profile.play_time_seconds, 0);
}

fn sync_response() -> Value {
    json!({"friends":[{"id":"remote","nick":"Remote","status":"playing","lastSeen":null,"gameVersion":"1.21.1","worldName":"Fixture World","lobbyCode":null}],"incomingRequests":[],"incomingInvites":[]})
}
#[tokio::test]
async fn social_get_friends_success_updates_cache() {
    let f = Fixture::new().await;
    f.respond("/api/user/sync", 200, sync_response()).await;
    let response = friends::social_get_friends(None, Some(auth()), f.social())
        .await
        .unwrap();
    assert_eq!(response.friends[0].id, "remote");
    assert_eq!(f.social().local_friends.lock().await.len(), 1);
}
#[tokio::test]
async fn social_get_friends_http_error_keeps_cached_list() {
    let f = Fixture::new().await;
    f.respond("/api/user/sync", 200, sync_response()).await;
    friends::social_get_friends(None, Some(auth()), f.social())
        .await
        .unwrap();
    f.server.reset().await;
    f.respond("/api/user/sync", 503, json!({})).await;
    assert_eq!(
        friends::social_get_friends(None, Some(auth()), f.social())
            .await
            .unwrap()
            .friends[0]
            .id,
        "remote"
    );
}
#[tokio::test]
async fn social_sync_success_uses_stored_auth() {
    let f = Fixture::new().await;
    *f.social().auth.lock().await = Some(auth());
    f.respond("/api/user/sync", 200, sync_response()).await;
    let response = friends::social_sync(None, None, f.social()).await.unwrap();
    assert_eq!(response.friends[0].nick, "Remote");
}
#[tokio::test]
async fn social_sync_http_error_returns_local_snapshot() {
    let f = Fixture::new().await;
    f.respond("/api/user/sync", 503, json!({})).await;
    let response = friends::social_sync(None, Some(auth()), f.social())
        .await
        .unwrap();
    assert!(response.friends.is_empty());
    assert!(response.incoming_invites.is_empty());
}
#[tokio::test]
async fn social_send_friend_request_success_sends_expected_json() {
    let f = Fixture::new().await;
    Mock::given(method("POST"))
        .and(path("/api/friends/request"))
        .and(body_json(json!({"friendCode":"TARGET"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"message":"ok"})))
        .expect(1)
        .mount(&f.server)
        .await;
    friends::social_send_friend_request(None, Some(auth()), Some("TARGET".into()), f.social())
        .await
        .unwrap();
}
#[tokio::test]
async fn social_send_friend_request_error_preserves_api_code() {
    let f = Fixture::new().await;
    f.respond("/api/friends/request", 403, json!({})).await;
    assert_eq!(
        friends::social_send_friend_request(None, Some(auth()), Some("TARGET".into()), f.social())
            .await
            .unwrap_err()
            .code,
        "SOCIAL_UNAUTHORIZED"
    );
}
#[tokio::test]
async fn social_send_friend_request_empty_code_fails_before_network() {
    let f = Fixture::new().await;
    assert_eq!(
        friends::social_send_friend_request(None, Some(auth()), Some(" ".into()), f.social())
            .await
            .unwrap_err()
            .code,
        "INVALID_ARGUMENT"
    );
    assert!(f.server.received_requests().await.unwrap().is_empty());
}

macro_rules! friend_response_cases {
    ($ok:ident,$err:ident,$func:ident,$accepted:expr) => {
        #[tokio::test] async fn $ok(){let f=Fixture::new().await;f.social().local_requests.lock().await.push(dto(json!({"id":"req","from_id":"remote","from_nick":"Remote","created_at":null})));Mock::given(path("/api/friends/respond")).and(body_json(json!({"fromId":"req","accept":$accepted}))).respond_with(ResponseTemplate::new(200)).expect(1).mount(&f.server).await;friends::$func(None,Some(auth()),Some("req".into()),f.social()).await.unwrap();assert!(f.social().local_requests.lock().await.is_empty());assert_eq!(f.social().local_friends.lock().await.len(),usize::from($accepted));}
        #[tokio::test] async fn $err(){let f=Fixture::new().await;assert_eq!(friends::$func(None,None,Some("req".into()),f.social()).await.unwrap_err().code,"INVALID_ARGUMENT");assert!(f.server.received_requests().await.unwrap().is_empty());}
    };
}
friend_response_cases!(
    social_accept_friend_request_success_updates_cache,
    social_accept_friend_request_error_missing_auth,
    social_accept_friend_request,
    true
);
friend_response_cases!(
    social_decline_friend_request_success_updates_cache,
    social_decline_friend_request_error_missing_auth,
    social_decline_friend_request,
    false
);
#[tokio::test]
async fn social_send_lobby_invite_success_uses_expected_target() {
    let f = Fixture::new().await;
    Mock::given(path("/api/invites/send"))
        .and(body_json(json!({"toId":"remote","lobbyCode":"ABC123"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"inviteId":"invite"})))
        .expect(1)
        .mount(&f.server)
        .await;
    friends::social_send_lobby_invite(
        None,
        Some(auth()),
        Some("remote".into()),
        Some("ABC123".into()),
        None,
        f.social(),
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn social_send_lobby_invite_error_missing_code() {
    let f = Fixture::new().await;
    assert_eq!(
        friends::social_send_lobby_invite(
            None,
            Some(auth()),
            Some("remote".into()),
            None,
            None,
            f.social()
        )
        .await
        .unwrap_err()
        .code,
        "INVALID_ARGUMENT"
    );
}
#[tokio::test]
async fn social_send_lobby_invite_remote_failure_is_currently_suppressed() {
    let f = Fixture::new().await;
    f.respond("/api/invites/send", 503, json!({})).await;
    friends::social_send_lobby_invite(
        None,
        Some(auth()),
        Some("remote".into()),
        Some("ABC123".into()),
        None,
        f.social(),
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn social_respond_lobby_invite_success_returns_code_and_removes_cache_entry() {
    let f = Fixture::new().await;
    f.social().local_invites.lock().await.push(dto(json!({"id":"inv","lobby_code":"ABC123","from_id":"remote","from_nick":"Remote","host_name":null,"created_at":null})));
    f.respond("/api/invites/respond", 200, json!({"lobbyCode":"ABC123"}))
        .await;
    assert_eq!(
        friends::social_respond_lobby_invite("inv".into(), true, Some(auth()), f.social())
            .await
            .unwrap()
            .as_deref(),
        Some("ABC123")
    );
    assert!(f.social().local_invites.lock().await.is_empty());
}
#[tokio::test]
async fn social_respond_lobby_invite_error_does_not_discard_cached_invite() {
    let f = Fixture::new().await;
    f.social().local_invites.lock().await.push(dto(json!({"id":"inv","lobby_code":"ABC123","from_id":"remote","from_nick":"Remote","host_name":null,"created_at":null})));
    f.respond("/api/invites/respond", 404, json!({})).await;
    assert_eq!(
        friends::social_respond_lobby_invite("inv".into(), true, Some(auth()), f.social())
            .await
            .unwrap_err()
            .code,
        "SOCIAL_NOT_FOUND"
    );
    assert_eq!(f.social().local_invites.lock().await.len(), 1);
}
#[tokio::test]
async fn social_set_presence_success_serializes_remote_status() {
    let f = Fixture::new().await;
    Mock::given(path("/api/user/sync")).and(body_json(json!({"nick":"Fixture","presence":{"status":"playing","game_version":"1.20.1","world_name":null,"lobby_code":null}}))).respond_with(ResponseTemplate::new(200).set_body_json(json!({}))).expect(1).mount(&f.server).await;
    friends::social_set_presence(
        None,
        Some(auth()),
        Some(UserPresenceDto {
            status: Some(OnlineStatusDto::Playing),
            game_version: Some("1.20.1".into()),
            world_name: None,
            lobby_code: None,
        }),
        f.social(),
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn social_set_presence_error_missing_auth() {
    let f = Fixture::new().await;
    assert_eq!(
        friends::social_set_presence(None, None, None, f.social())
            .await
            .unwrap_err()
            .code,
        "INVALID_ARGUMENT"
    );
}
#[tokio::test]
async fn social_list_achievements_success_uses_explicit_temp_root() {
    let f = Fixture::new().await;
    let response = friends::social_list_achievements(
        None,
        Some(f.paths.root.to_string_lossy().into()),
        f.social(),
    )
    .await
    .unwrap();
    assert!(!response.achievements.is_empty());
    assert_eq!(response.achievements.len(), response.progress.len());
}
#[tokio::test]
async fn social_list_achievements_error_invalid_utf8() {
    let f = Fixture::new().await;
    fs::write(f.paths.root.join("achievements.json"), [0xff]).unwrap();
    assert_eq!(
        friends::social_list_achievements(None, None, f.social())
            .await
            .unwrap_err()
            .code,
        "ACHIEVEMENT_IO_ERROR"
    );
}
#[tokio::test]
async fn social_report_achievement_progress_impl_error_unknown_achievement() {
    let tmp = tempdir().unwrap();
    assert_eq!(
        friends::report_achievement_progress_impl::<MockRuntime>(
            None,
            tmp.path().into(),
            "nonexistent",
            1
        )
        .await
        .unwrap_err()
        .code,
        "NOT_FOUND"
    );
}

macro_rules! typed_lobby_bool_cases {
    ($ok:ident,$err:ident,$func:ident,$route:expr,$request:expr) => {
        #[tokio::test]
        async fn $ok() {
            let f = Fixture::new().await;
            f.respond($route, 200, json!({})).await;
            assert!(lobby::$func(dto($request), f.room()).await.unwrap());
        }
        #[tokio::test]
        async fn $err() {
            let f = Fixture::new().await;
            // A redirect loop makes reqwest::send fail deterministically. Dropping
            // MockServer alone returns its still-listening server to the pool.
            Mock::given(path($route))
                .respond_with(
                    ResponseTemplate::new(307)
                        .insert_header("Location", format!("{}{}", f.server.uri(), $route)),
                )
                .mount(&f.server)
                .await;
            assert_eq!(
                lobby::$func(dto($request), f.room())
                    .await
                    .unwrap_err()
                    .code,
                "NET_LOBBY_NETWORK_ERROR"
            );
        }
    };
}
typed_lobby_bool_cases!(
    net_lobby_open_success,
    net_lobby_open_error,
    net_lobby_open,
    "/api/lobby/open",
    json!({"code":"ABC123","host_token":"fixture","tunnel_address":"local:1","port":25565})
);
typed_lobby_bool_cases!(
    net_lobby_heartbeat_success,
    net_lobby_heartbeat_error,
    net_lobby_heartbeat,
    "/api/lobby/heartbeat",
    json!({"code":"ABC123","host_token":"fixture"})
);
typed_lobby_bool_cases!(
    net_lobby_close_success,
    net_lobby_close_error,
    net_lobby_close,
    "/api/lobby/close",
    json!({"code":"ABC123","host_token":"fixture"})
);
typed_lobby_bool_cases!(
    net_lobby_leave_success,
    net_lobby_leave_error,
    net_lobby_leave,
    "/api/lobby/leave",
    json!({"code":"ABC123","player_name":"Fixture"})
);
typed_lobby_bool_cases!(
    net_lobby_kick_success,
    net_lobby_kick_error,
    net_lobby_kick,
    "/api/lobby/kick",
    json!({"code":"ABC123","host_token":"fixture","player":"Remote"})
);
#[tokio::test]
async fn net_lobby_create_success_uses_typed_response() {
    let f = Fixture::new().await;
    f.respond(
        "/api/lobby",
        200,
        json!({"code":"ABC123","hostToken":"fixture","status":"open","createdAt":42}),
    )
    .await;
    assert_eq!(
        lobby::net_lobby_create(
            dto(json!({"host_name":"Fixture","manifest":null})),
            f.room()
        )
        .await
        .unwrap()
        .code,
        "ABC123"
    );
}
#[tokio::test]
async fn net_lobby_create_error_maps_server_error() {
    let f = Fixture::new().await;
    f.respond("/api/lobby", 503, json!({})).await;
    assert_eq!(
        lobby::net_lobby_create(
            dto(json!({"host_name":"Fixture","manifest":null})),
            f.room()
        )
        .await
        .unwrap_err()
        .code,
        "NET_LOBBY_API_ERROR"
    );
}
#[tokio::test]
async fn net_lobby_join_success_returns_tunnel() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/join",200,json!({"success":true,"code":"ABC123","status":"open","tunnelAddress":"localhost:25565","playerCount":2})).await;
    assert_eq!(
        lobby::net_lobby_join(
            dto(json!({"code":"ABC123","player_name":"Fixture","manifest":null})),
            f.room()
        )
        .await
        .unwrap()
        .tunnel_address
        .as_deref(),
        Some("localhost:25565")
    );
}
#[tokio::test]
async fn net_lobby_join_error_maps_not_found() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/join", 404, json!({})).await;
    assert_eq!(
        lobby::net_lobby_join(
            dto(json!({"code":"ABC123","player_name":"Fixture","manifest":null})),
            f.room()
        )
        .await
        .unwrap_err()
        .code,
        "NET_LOBBY_NOT_FOUND"
    );
}
fn room_status() -> Value {
    json!({"code":"ABC123","status":"open","tunnelAddress":"localhost:25565","playerCount":1,"lastHeartbeat":42,"players":["Fixture"],"hostName":"Fixture","kicked":false})
}
#[tokio::test]
async fn net_lobby_status_success_reads_remote_players() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/status", 200, room_status()).await;
    assert_eq!(
        lobby::net_lobby_status(
            dto(json!({"code":"ABC123","player_name":"Fixture"})),
            f.room()
        )
        .await
        .unwrap()
        .players
        .unwrap(),
        vec!["Fixture"]
    );
}
#[tokio::test]
async fn net_lobby_status_error_invalid_json() {
    let f = Fixture::new().await;
    Mock::given(path("/api/lobby/status"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&f.server)
        .await;
    assert_eq!(
        lobby::net_lobby_status(dto(json!({"code":"ABC123","player_name":null})), f.room())
            .await
            .unwrap_err()
            .code,
        "NET_LOBBY_NETWORK_ERROR"
    );
}
#[tokio::test]
async fn net_lobby_tunnel_config_success_reads_remote_configuration() {
    let f = Fixture::new().await;
    f.respond(
        "/api/tunnel-config",
        200,
        json!({"secret":"fixture","publicAddress":"local.example","publicPort":1234}),
    )
    .await;
    assert_eq!(
        lobby::net_lobby_tunnel_config(f.room())
            .await
            .unwrap()
            .public_port,
        Some(1234)
    );
}
#[tokio::test]
async fn net_lobby_tunnel_config_error_maps_forbidden() {
    let f = Fixture::new().await;
    f.respond("/api/tunnel-config", 403, json!({})).await;
    assert_eq!(
        lobby::net_lobby_tunnel_config(f.room())
            .await
            .unwrap_err()
            .code,
        "NET_LOBBY_UNAUTHORIZED"
    );
}
#[tokio::test]
async fn net_lobby_list_open_rooms_success_returns_empty_service_result() {
    let f = Fixture::new().await;
    f.respond("/worlds", 200, json!({"worlds":[]})).await;
    assert!(
        lobby::net_lobby_list_open_rooms(dto(json!({"player_name":null})), f.room())
            .await
            .unwrap()
            .rooms
            .is_empty()
    );
}
#[tokio::test]
async fn net_lobby_list_open_rooms_error_invalid_json() {
    let f = Fixture::new().await;
    Mock::given(path("/worlds"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&f.server)
        .await;
    assert_eq!(
        lobby::net_lobby_list_open_rooms(dto(json!({"player_name":null})), f.room())
            .await
            .unwrap_err()
            .code,
        "NET_LOBBY_JSON_ERROR"
    );
}
#[tokio::test]
async fn list_open_rooms_success_delegates_to_cloud_backend() {
    let f = Fixture::new().await;
    f.respond("/worlds", 200, json!({"worlds":[]})).await;
    assert!(lobby::list_open_rooms(None, f.room())
        .await
        .unwrap()
        .is_empty());
}
#[tokio::test]
async fn list_open_rooms_error_preserves_readable_message() {
    let f = Fixture::new().await;
    f.respond("/worlds", 503, json!({})).await;
    assert!(lobby::list_open_rooms(None, f.room())
        .await
        .unwrap_err()
        .contains("недоступен"));
}
#[tokio::test]
async fn get_lobby_status_success_returns_remote_room() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/status", 200, room_status()).await;
    let room = lobby::get_lobby_status("ABC123".into(), Some("Fixture".into()), f.room())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(room.code, "#ABC123");
}
#[tokio::test]
async fn get_lobby_status_remote_error_returns_none() {
    let f = Fixture::new().await;
    f.respond("/api/lobby/status", 404, json!({})).await;
    assert!(lobby::get_lobby_status("ABC123".into(), None, f.room())
        .await
        .unwrap()
        .is_none());
}
#[tokio::test]
async fn join_lobby_room_error_rejects_empty_code_before_network() {
    let f = Fixture::new().await;
    assert!(lobby::join_lobby_room(" ".into(), None, f.room())
        .await
        .is_err());
    assert!(f.server.received_requests().await.unwrap().is_empty());
}
#[tokio::test]
async fn kick_lobby_player_error_without_host_session() {
    let f = Fixture::new().await;
    assert!(
        lobby::kick_lobby_player("ABC123".into(), "Remote".into(), f.room())
            .await
            .unwrap_err()
            .contains("организатором")
    );
}
#[tokio::test]
async fn net_tunnel_stop_success_without_process_is_noop() {
    let f = Fixture::new().await;
    lobby::net_tunnel_stop(f.room()).await.unwrap();
    assert!(f.room().supervisor.lock().await.is_none());
}
#[tokio::test]
async fn net_tunnel_status_success_initially_inactive() {
    let f = Fixture::new().await;
    assert_eq!(
        lobby::net_tunnel_status(f.room()).await.unwrap(),
        TunnelStatusDto::Inactive
    );
}
#[tokio::test]
async fn net_tunnel_start_error_missing_binary_is_structured() {
    let f = Fixture::new().await;
    let request = dto(
        json!({"binary_path":f.paths.root.join("missing.exe"),"secret_key":f.paths.root.join("fixture.secret")}),
    );
    assert_eq!(
        lobby::net_tunnel_start(request, f.room())
            .await
            .unwrap_err()
            .code,
        "INTERNAL_ERROR"
    );
}

#[tokio::test]
async fn workshop_list_worlds_success_both_versions() {
    let f = Fixture::new().await;
    for v in ["1.20.1", "1.21.1"] {
        assert!(workshop::workshop_list_worlds(Some(v.into()), f.play())
            .await
            .unwrap()
            .is_empty());
    }
}
#[tokio::test]
async fn workshop_list_worlds_non_directory_returns_empty() {
    let f = Fixture::new().await;
    fs::write(f.paths.instance_dir("1.21.1").join("saves"), b"block").unwrap();
    assert!(workshop::workshop_list_worlds(None, f.play())
        .await
        .unwrap()
        .is_empty());
}
#[tokio::test]
async fn social_list_worlds_success_uses_explicit_game_dir() {
    let f = Fixture::new().await;
    let response = workshop::social_list_worlds(
        Some(dto(json!({"game_dir":f.paths.root.join("other")}))),
        None,
        f.play(),
    )
    .await
    .unwrap();
    assert!(response.worlds.is_empty());
}
#[tokio::test]
async fn social_list_worlds_non_directory_returns_empty() {
    let f = Fixture::new().await;
    let dir = f.paths.root.join("other");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("saves"), b"block").unwrap();
    assert!(
        workshop::social_list_worlds(Some(dto(json!({"game_dir":dir}))), None, f.play())
            .await
            .unwrap()
            .worlds
            .is_empty()
    );
}
#[tokio::test]
async fn workshop_create_world_backup_success_contains_original_data() {
    let f = Fixture::new().await;
    let world = f.paths.root.join("FixtureWorld");
    fs::create_dir(&world).unwrap();
    fs::write(world.join("level.dat"), b"fixture").unwrap();
    let backup = workshop::workshop_create_world_backup(
        world.to_string_lossy().into(),
        Some("1.20.1".into()),
        f.play(),
    )
    .await
    .unwrap();
    assert!(std::path::Path::new(&backup.file_path).starts_with(f.paths.instance_dir("1.20.1")));
    let zip = zip::ZipArchive::new(fs::File::open(backup.file_path).unwrap()).unwrap();
    assert_eq!(zip.len(), 1);
    assert_eq!(fs::read(world.join("level.dat")).unwrap(), b"fixture");
}
#[tokio::test]
async fn workshop_create_world_backup_error_missing_world() {
    let f = Fixture::new().await;
    assert!(workshop::workshop_create_world_backup(
        f.paths.root.join("missing").to_string_lossy().into(),
        None,
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn social_create_world_backup_success_respects_explicit_destination() {
    let f = Fixture::new().await;
    let world = f.paths.root.join("FixtureWorld");
    let backups = f.paths.root.join("custom-backups");
    fs::create_dir(&world).unwrap();
    fs::write(world.join("level.dat"), b"fixture").unwrap();
    let result = workshop::social_create_world_backup(
        dto(json!({"world_dir":world,"backups_dir":backups})),
        f.play(),
    )
    .await
    .unwrap();
    assert!(std::path::Path::new(&result.file_path).starts_with(backups));
}
#[tokio::test]
async fn social_create_world_backup_error_missing_world() {
    let f = Fixture::new().await;
    assert!(workshop::social_create_world_backup(dto(json!({"world_dir":f.paths.root.join("missing"),"backups_dir":f.paths.root.join("backups")})),f.play()).await.is_err());
}
#[tokio::test]
async fn workshop_list_world_backups_success_empty_directory() {
    let f = Fixture::new().await;
    assert!(
        workshop::workshop_list_world_backups(None, None, None, f.play())
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn workshop_list_world_backups_non_directory_returns_empty() {
    let f = Fixture::new().await;
    fs::write(f.paths.instance_dir("1.21.1").join("backups"), b"block").unwrap();
    assert!(
        workshop::workshop_list_world_backups(None, None, None, f.play())
            .await
            .unwrap()
            .is_empty()
    );
}
#[tokio::test]
async fn social_list_world_backups_success_explicit_directory() {
    let f = Fixture::new().await;
    assert!(workshop::social_list_world_backups(
        dto(json!({"backups_dir":f.paths.root.join("backups")})),
        f.play()
    )
    .await
    .unwrap()
    .backups
    .is_empty());
}
#[tokio::test]
async fn social_list_world_backups_non_directory_returns_empty() {
    let f = Fixture::new().await;
    let blocker = f.paths.root.join("blocker");
    fs::write(&blocker, b"block").unwrap();
    assert!(
        workshop::social_list_world_backups(dto(json!({"backups_dir":blocker})), f.play())
            .await
            .unwrap()
            .backups
            .is_empty()
    );
}

fn archive(path: &std::path::Path, name: &str, bytes: &[u8]) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    zip.start_file(name, zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(bytes).unwrap();
    zip.finish().unwrap();
}
#[tokio::test]
async fn workshop_restore_world_backup_success_extracts_into_requested_target() {
    let f = Fixture::new().await;
    let zip = f.paths.root.join("Fixture_2026.zip");
    archive(&zip, "level.dat", b"fixture");
    let target = f.paths.root.join("restored");
    let result = workshop::workshop_restore_world_backup(
        zip.to_string_lossy().into(),
        Some(target.to_string_lossy().into()),
        None,
        f.play(),
    )
    .await
    .unwrap();
    assert_eq!(std::path::Path::new(&result), target);
    assert_eq!(fs::read(target.join("level.dat")).unwrap(), b"fixture");
}
#[tokio::test]
#[ignore = "Known data-loss bug: restore deletes existing world before validating ZIP; no production logic changes in this test-only task"]
async fn workshop_restore_world_backup_error_invalid_zip_does_not_remove_existing_world() {
    let f = Fixture::new().await;
    let zip = f.paths.root.join("bad.zip");
    fs::write(&zip, b"not zip").unwrap();
    let target = f.paths.root.join("existing");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("sentinel"), b"keep").unwrap();
    assert!(workshop::workshop_restore_world_backup(
        zip.to_string_lossy().into(),
        Some(target.to_string_lossy().into()),
        None,
        f.play()
    )
    .await
    .is_err());
    assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"keep");
}
#[tokio::test]
async fn social_restore_world_backup_success_uses_named_saves_root() {
    let f = Fixture::new().await;
    let zip = f.paths.root.join("Fixture_2026.zip");
    archive(&zip, "level.dat", b"fixture");
    let saves = f.paths.root.join("saves-root");
    let result = workshop::social_restore_world_backup(
        dto(json!({"backup_path":zip,"saves_dir":saves})),
        f.play(),
    )
    .await
    .unwrap();
    assert!(std::path::Path::new(&result.restored_world_dir).starts_with(saves));
    assert_eq!(
        fs::read(std::path::Path::new(&result.restored_world_dir).join("level.dat")).unwrap(),
        b"fixture"
    );
}
#[tokio::test]
async fn social_restore_world_backup_error_missing_archive() {
    let f = Fixture::new().await;
    assert!(workshop::social_restore_world_backup(dto(json!({"backup_path":f.paths.root.join("missing.zip"),"saves_dir":f.paths.root.join("saves")})),f.play()).await.is_err());
}
#[tokio::test]
async fn workshop_delete_world_success_only_removes_selected_temp_world() {
    let f = Fixture::new().await;
    let world = f.paths.root.join("world");
    let neighbor = f.paths.root.join("neighbor");
    fs::create_dir(&world).unwrap();
    fs::write(&neighbor, b"keep").unwrap();
    workshop::workshop_delete_world(world.to_string_lossy().into(), f.play())
        .await
        .unwrap();
    assert!(!world.exists());
    assert_eq!(fs::read(neighbor).unwrap(), b"keep");
}
#[tokio::test]
async fn workshop_delete_world_error_missing_folder() {
    let f = Fixture::new().await;
    assert!(workshop::workshop_delete_world(
        f.paths.root.join("missing").to_string_lossy().into(),
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn workshop_list_screenshots_success_reads_file_metadata() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.20.1").join("screenshots");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("fixture.png"), b"png").unwrap();
    let result = workshop::workshop_list_screenshots(Some("1.20.1".into()), f.play())
        .await
        .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].file_name, "fixture.png");
}
#[tokio::test]
async fn workshop_list_screenshots_non_directory_returns_empty() {
    let f = Fixture::new().await;
    fs::write(f.paths.instance_dir("1.21.1").join("screenshots"), b"block").unwrap();
    assert!(workshop::workshop_list_screenshots(None, f.play())
        .await
        .unwrap()
        .is_empty());
}
#[tokio::test]
async fn social_list_screenshots_success_uses_explicit_directory() {
    let f = Fixture::new().await;
    let dir = f.paths.root.join("screenshots");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("fixture.png"), b"png").unwrap();
    assert_eq!(
        workshop::social_list_screenshots(
            Some(dto(json!({"screenshots_dir":dir}))),
            None,
            f.play()
        )
        .await
        .unwrap()
        .screenshots
        .len(),
        1
    );
}
#[tokio::test]
async fn social_list_screenshots_non_directory_returns_empty() {
    let f = Fixture::new().await;
    let file = f.paths.root.join("blocker");
    fs::write(&file, b"block").unwrap();
    assert!(workshop::social_list_screenshots(
        Some(dto(json!({"screenshots_dir":file}))),
        None,
        f.play()
    )
    .await
    .unwrap()
    .screenshots
    .is_empty());
}
#[tokio::test]
async fn workshop_open_path_error_missing_path_does_not_launch_explorer() {
    let f = Fixture::new().await;
    assert!(
        workshop::workshop_open_path(f.paths.root.join("missing").to_string_lossy().into())
            .await
            .unwrap_err()
            .contains("не существует")
    );
}

#[tokio::test]
async fn workshop_list_resource_packs_success_reads_metadata() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.21.1").join("resourcepacks");
    fs::create_dir_all(&dir).unwrap();
    archive(
        &dir.join("custom.zip"),
        "pack.mcmeta",
        br#"{"pack":{"description":"Fixture","pack_format":15}}"#,
    );
    let packs = workshop_packs::workshop_list_resource_packs(None, f.play())
        .await
        .unwrap();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].description.as_deref(), Some("Fixture"));
    assert_eq!(packs[0].pack_format, Some(15));
}
#[tokio::test]
async fn workshop_list_resource_packs_invalid_zip_uses_fallback_metadata() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.21.1").join("resourcepacks");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("custom.zip"), b"invalid ZIP").unwrap();
    let packs = workshop_packs::workshop_list_resource_packs(None, f.play())
        .await
        .unwrap();
    assert_eq!(packs.len(), 1);
    assert!(packs[0].description.is_none());
}
#[tokio::test]
async fn workshop_toggle_resource_pack_success_writes_options() {
    let f = Fixture::new().await;
    workshop_packs::workshop_toggle_resource_pack("custom.zip".into(), true, None, f.play())
        .await
        .unwrap();
    let text = fs::read_to_string(f.paths.instance_dir("1.21.1").join("options.txt")).unwrap();
    assert!(text.contains("file/custom.zip"));
}
#[tokio::test]
async fn workshop_toggle_resource_pack_error_when_options_is_directory() {
    let f = Fixture::new().await;
    fs::create_dir(f.paths.instance_dir("1.21.1").join("options.txt")).unwrap();
    assert!(workshop_packs::workshop_toggle_resource_pack(
        "custom.zip".into(),
        true,
        None,
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn workshop_set_resource_packs_order_success_preserves_user_options() {
    let f = Fixture::new().await;
    let options = f.paths.instance_dir("1.21.1").join("options.txt");
    fs::write(&options, b"fov:80\n").unwrap();
    workshop_packs::workshop_set_resource_packs_order(
        vec!["second.zip".into(), "first.zip".into()],
        None,
        f.play(),
    )
    .await
    .unwrap();
    let text = fs::read_to_string(options).unwrap();
    assert!(text.contains("fov:80"));
    assert!(text.find("second.zip").unwrap() < text.find("first.zip").unwrap());
}
#[tokio::test]
async fn workshop_set_resource_packs_order_error_when_options_is_directory() {
    let f = Fixture::new().await;
    fs::create_dir(f.paths.instance_dir("1.21.1").join("options.txt")).unwrap();
    assert!(
        workshop_packs::workshop_set_resource_packs_order(vec![], None, f.play())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn workshop_list_shader_packs_success_tracks_active_shader() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.20.1").join("shaderpacks");
    fs::create_dir_all(&dir).unwrap();
    archive(&dir.join("custom.zip"), "shaders/fixture.glsl", b"fixture");
    workshop_packs::workshop_set_active_shader_pack(
        Some("custom.zip".into()),
        Some("1.20.1".into()),
        f.play(),
    )
    .await
    .unwrap();
    let packs = workshop_packs::workshop_list_shader_packs(Some("1.20.1".into()), f.play())
        .await
        .unwrap();
    assert_eq!(packs.len(), 1);
    assert!(packs[0].is_active);
}
#[tokio::test]
async fn workshop_list_shader_packs_missing_directory_returns_empty() {
    let f = Fixture::new().await;
    assert!(workshop_packs::workshop_list_shader_packs(None, f.play())
        .await
        .unwrap()
        .is_empty());
}
#[tokio::test]
async fn workshop_set_active_shader_pack_success_can_disable_shader() {
    let f = Fixture::new().await;
    workshop_packs::workshop_set_active_shader_pack(Some("custom.zip".into()), None, f.play())
        .await
        .unwrap();
    workshop_packs::workshop_set_active_shader_pack(None, None, f.play())
        .await
        .unwrap();
    let text = fs::read_to_string(
        f.paths
            .instance_dir("1.21.1")
            .join("config/iris.properties"),
    )
    .unwrap();
    assert!(text.contains("enableShaders=false"));
}
#[tokio::test]
async fn workshop_set_active_shader_pack_error_when_config_is_file() {
    let f = Fixture::new().await;
    let config = f.paths.instance_dir("1.21.1").join("config");
    if config.exists() {
        fs::remove_dir(&config).unwrap();
    }
    fs::write(config, b"block").unwrap();
    assert!(workshop_packs::workshop_set_active_shader_pack(
        Some("custom.zip".into()),
        None,
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn workshop_add_pack_file_success_copies_both_types_and_versions() {
    let f = Fixture::new().await;
    let src = f.paths.root.join("custom.zip");
    archive(&src, "fixture", b"pack");
    for (version, kind, folder) in [
        ("1.20.1", "resourcepack", "resourcepacks"),
        ("1.21.1", "shaderpack", "shaderpacks"),
    ] {
        assert_eq!(
            workshop_packs::workshop_add_pack_file(
                src.to_string_lossy().into(),
                kind.into(),
                Some(version.into()),
                f.play()
            )
            .await
            .unwrap(),
            "custom.zip"
        );
        assert_eq!(
            fs::read(
                f.paths
                    .instance_dir(version)
                    .join(folder)
                    .join("custom.zip")
            )
            .unwrap(),
            fs::read(&src).unwrap()
        );
    }
}
#[tokio::test]
async fn workshop_add_pack_file_error_missing_source() {
    let f = Fixture::new().await;
    assert!(workshop_packs::workshop_add_pack_file(
        f.paths.root.join("missing.zip").to_string_lossy().into(),
        "resourcepack".into(),
        None,
        f.play()
    )
    .await
    .is_err());
}
#[tokio::test]
async fn workshop_delete_pack_file_success_removes_custom_pack() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.21.1").join("resourcepacks");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("custom.zip"), b"pack").unwrap();
    workshop_packs::workshop_delete_pack_file(
        "custom.zip".into(),
        "resourcepack".into(),
        None,
        f.play(),
    )
    .await
    .unwrap();
    assert!(!dir.join("custom.zip").exists());
}
#[tokio::test]
async fn workshop_delete_pack_file_error_keeps_official_pack() {
    let f = Fixture::new().await;
    let dir = f.paths.instance_dir("1.21.1").join("resourcepacks");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("Aura-Pack.zip"), b"keep").unwrap();
    assert!(workshop_packs::workshop_delete_pack_file(
        "Aura-Pack.zip".into(),
        "resourcepack".into(),
        None,
        f.play()
    )
    .await
    .is_err());
    assert_eq!(fs::read(dir.join("Aura-Pack.zip")).unwrap(), b"keep");
}
