//! Lobby and Tunnel command handlers for Wex launcher.
//!
//! Provides typed Tauri commands for:
//! - Creating and joining P2P lobbies via `wex_net::LobbyApiClient`
//! - Automatic heartbeat loop
//! - Managing tunnel state with `PlayitSupervisor`
//! - Kicking players and closing rooms
//! - Fetching open rooms from cloud backend with honest "service unavailable" error handling

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::State;
use tokio::sync::Mutex;

use wex_ipc::commands::{
    NetLanListDiscoveredRequest, NetLanListDiscoveredResponse, NetListOpenRoomsRequest,
    NetListOpenRoomsResponse, NetLobbyCloseRequest, NetLobbyCreateRequest,
    NetLobbyHeartbeatRequest, NetLobbyJoinRequest, NetLobbyKickRequest, NetLobbyLeaveRequest,
    NetLobbyOpenRequest, NetLobbyStatusRequest, NetTunnelEnsureBinaryRequest,
    NetTunnelEnsureBinaryResponse, NetTunnelStartRequest, NetTunnelStartResponse,
};
use wex_ipc::error::{IpcError, IpcResult};
use wex_ipc::models::{
    LanWorldDto, LobbyCreateResponseDto, LobbyJoinResponseDto, LobbyStatusResponseDto,
    NetOpenRoomDto, TunnelConfigResponseDto, TunnelStatusDto,
};
use wex_net::events::{NetEventBus, TunnelStatus};
use wex_net::lan::LanDiscoveryService;
use wex_net::lobby::{HeartbeatHandle, LobbyApiClient, LobbyError};
use wex_net::tunnel::{ensure_binary_downloaded, PlayitSupervisor};

use crate::ipc::{LobbyPlayerModel, LobbyRoomModel};

pub const DEFAULT_LOBBY_API_URL: &str = "https://lobby-api.vercel.app";
pub const DEFAULT_OLD_BACKEND_URL: &str = "http://89.168.82.200:8080";

#[derive(Debug, Clone)]
pub struct ActiveLobbySession {
    pub code: String,
    pub host_token: Option<String>,
    pub host_name: String,
    pub world_name: String,
    pub player_name: String,
    pub is_host: bool,
    pub tunnel_address: Option<String>,
}

pub struct LobbyState {
    pub client: LobbyApiClient,
    pub event_bus: Arc<NetEventBus>,
    pub active_session: Arc<Mutex<Option<ActiveLobbySession>>>,
    pub heartbeat_handle: Arc<Mutex<Option<HeartbeatHandle>>>,
    pub supervisor: Arc<Mutex<Option<PlayitSupervisor>>>,
    pub current_tunnel_status: Arc<Mutex<TunnelStatus>>,
    pub tools_dir: PathBuf,
    pub old_backend_url: String,
    pub lan_discovery: Arc<LanDiscoveryService>,
}

unsafe impl Send for LobbyState {}
unsafe impl Sync for LobbyState {}

impl LobbyState {
    pub fn new(tools_dir: PathBuf, lobby_api_url: &str, old_backend_url: &str) -> Self {
        let event_bus = Arc::new(NetEventBus::new(256));
        let client =
            LobbyApiClient::new(lobby_api_url, Some(event_bus.clone())).unwrap_or_else(|_| {
                LobbyApiClient::new(DEFAULT_LOBBY_API_URL, Some(event_bus.clone()))
                    .expect("Failed to initialize default LobbyApiClient")
            });

        let lan_discovery = Arc::new(LanDiscoveryService::default());
        std::mem::drop(lan_discovery.clone().spawn_listener());

        Self {
            client,
            event_bus,
            active_session: Arc::new(Mutex::new(None)),
            heartbeat_handle: Arc::new(Mutex::new(None)),
            supervisor: Arc::new(Mutex::new(None)),
            current_tunnel_status: Arc::new(Mutex::new(TunnelStatus::Inactive)),
            tools_dir,
            old_backend_url: old_backend_url.to_string(),
            lan_discovery,
        }
    }
}

fn format_lobby_error(err: &LobbyError) -> String {
    match err {
        LobbyError::Reqwest(e) => {
            if e.is_timeout() {
                "Время ожидания ответа от сервиса лобби истекло. Проверьте интернет-соединение."
                    .to_string()
            } else {
                "Не удалось связаться с сервисом лобби. Проверьте сетевое подключение.".to_string()
            }
        }
        LobbyError::Api { status, message } => {
            if *status == 404 {
                "Комната не найдена или уже была закрыта хостом.".to_string()
            } else if *status == 401 || *status == 403 {
                "Неверный токен доступа к комнате лобби.".to_string()
            } else if *status >= 500 {
                "Сервис лобби временно недоступен. Попробуйте позже.".to_string()
            } else if !message.is_empty() {
                message.clone()
            } else {
                format!("Ошибка сервиса лобби (HTTP {status})")
            }
        }
        LobbyError::Json(_) => "Некорректный формат ответа от сервиса лобби.".to_string(),
        LobbyError::Url(_) => "Недопустимый адрес сервиса лобби.".to_string(),
    }
}

async fn fetch_open_rooms(
    base_url: &str,
    player_name: Option<&str>,
) -> IpcResult<NetListOpenRoomsResponse> {
    let player = player_name.unwrap_or("Player");
    let mut encoded_player = String::new();
    for byte in player.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_' || byte == b'.' {
            encoded_player.push(byte as char);
        } else {
            encoded_player.push_str(&format!("%{:02X}", byte));
        }
    }

    let url_str = format!(
        "{}/worlds?player={}",
        base_url.trim_end_matches('/'),
        encoded_player
    );

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| IpcError::internal("Не удалось создать HTTP-клиент", Some(e.to_string())))?;

    let resp = http
        .get(&url_str)
        .header("X-Aura-Secret", "wex-launcher-secret")
        .send()
        .await
        .map_err(|e| {
            IpcError::new(
                "NET_LOBBY_API_ERROR",
                "Сервис недоступен. Не удалось связаться с сервером комнат",
                Some(e.to_string()),
            )
        })?;

    if !resp.status().is_success() {
        return Err(IpcError::new(
            "NET_LOBBY_API_ERROR",
            "Сервис недоступен. Сервер комнат вернул ошибку",
            Some(format!("HTTP {}", resp.status())),
        ));
    }

    #[derive(serde::Deserialize)]
    struct RawWorldsResponse {
        worlds: Option<Vec<RawWorldItem>>,
    }

    #[derive(serde::Deserialize)]
    struct RawWorldItem {
        id: String,
        name: String,
        owner: String,
        #[serde(default)]
        status: String,
        #[serde(default)]
        port: Option<u16>,
        #[serde(default)]
        online_players: Option<Vec<String>>,
        #[serde(default = "default_max_players")]
        max_players: u32,
        #[serde(default = "default_game_version")]
        game_version: Option<String>,
    }

    fn default_max_players() -> u32 {
        8
    }

    fn default_game_version() -> Option<String> {
        Some("1.21.1".to_string())
    }

    let parsed: RawWorldsResponse = resp.json().await.map_err(|e| {
        IpcError::new(
            "NET_LOBBY_JSON_ERROR",
            "Некорректный формат ответа от сервера комнат",
            Some(e.to_string()),
        )
    })?;

    let rooms = parsed
        .worlds
        .unwrap_or_default()
        .into_iter()
        .map(|w| NetOpenRoomDto {
            id: w.id,
            name: w.name,
            host_name: w.owner,
            game_version: w.game_version.unwrap_or_else(|| "1.21.1".to_string()),
            player_count: w
                .online_players
                .as_ref()
                .map(|p| p.len() as u32)
                .unwrap_or(0),
            max_players: w.max_players,
            address: w.port.map(|p| format!("89.168.82.200:{}", p)),
            status: if w.status.is_empty() {
                "open".to_string()
            } else {
                w.status
            },
        })
        .collect();

    Ok(NetListOpenRoomsResponse { rooms })
}

// ============================================================================
// Standard wex-ipc Commands (Section 3.2)
// ============================================================================

#[tauri::command]
pub async fn net_lobby_create(
    request: NetLobbyCreateRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<LobbyCreateResponseDto> {
    let manifest: Option<Vec<wex_net::ModManifestEntry>> = request.manifest.map(|entries| {
        entries
            .into_iter()
            .map(|e| wex_net::ModManifestEntry {
                id: e.id,
                name: e.name,
                version: e.version,
                enabled: e.enabled,
            })
            .collect()
    });
    let res = state
        .client
        .create_lobby(&request.host_name, manifest.as_deref())
        .await
        .map_err(IpcError::from)?;
    Ok(res.into())
}

#[tauri::command]
pub async fn net_lobby_join(
    request: NetLobbyJoinRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<LobbyJoinResponseDto> {
    let manifest: Option<Vec<wex_net::ModManifestEntry>> = request.manifest.map(|entries| {
        entries
            .into_iter()
            .map(|e| wex_net::ModManifestEntry {
                id: e.id,
                name: e.name,
                version: e.version,
                enabled: e.enabled,
            })
            .collect()
    });
    let res = state
        .client
        .join_lobby(&request.code, &request.player_name, manifest.as_deref())
        .await
        .map_err(IpcError::from)?;
    Ok(res.into())
}

#[tauri::command]
pub async fn net_lobby_open(
    request: NetLobbyOpenRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<bool> {
    let res = state
        .client
        .open_lobby(
            &request.code,
            &request.host_token,
            &request.tunnel_address,
            request.port,
        )
        .await
        .map_err(IpcError::from)?;
    Ok(res)
}

#[tauri::command]
pub async fn net_lobby_status(
    request: NetLobbyStatusRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<LobbyStatusResponseDto> {
    let res = state
        .client
        .get_status(&request.code, request.player_name.as_deref())
        .await
        .map_err(IpcError::from)?;
    Ok(res.into())
}

#[tauri::command]
pub async fn net_lobby_heartbeat(
    request: NetLobbyHeartbeatRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<bool> {
    let res = state
        .client
        .heartbeat(&request.code, &request.host_token)
        .await
        .map_err(IpcError::from)?;
    Ok(res)
}

#[tauri::command]
pub async fn net_lobby_close(
    request: NetLobbyCloseRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<bool> {
    let mut hb_lock = state.heartbeat_handle.lock().await;
    if let Some(hb) = hb_lock.take() {
        hb.stop();
    }
    let mut sup_lock = state.supervisor.lock().await;
    if let Some(mut sup) = sup_lock.take() {
        sup.stop().await;
    }
    let res = state
        .client
        .close_lobby(&request.code, &request.host_token)
        .await
        .map_err(IpcError::from)?;
    let mut session_lock = state.active_session.lock().await;
    *session_lock = None;
    Ok(res)
}

#[tauri::command]
pub async fn net_lobby_leave(
    request: NetLobbyLeaveRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<bool> {
    let res = state
        .client
        .leave_lobby(&request.code, &request.player_name)
        .await
        .map_err(IpcError::from)?;
    let mut session_lock = state.active_session.lock().await;
    *session_lock = None;
    Ok(res)
}

#[tauri::command]
pub async fn net_lobby_kick(
    request: NetLobbyKickRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<bool> {
    let res = state
        .client
        .kick_player(&request.code, &request.host_token, &request.player)
        .await
        .map_err(IpcError::from)?;
    Ok(res)
}

#[tauri::command]
pub async fn net_lobby_tunnel_config(
    state: State<'_, LobbyState>,
) -> IpcResult<TunnelConfigResponseDto> {
    let res = state
        .client
        .get_tunnel_config()
        .await
        .map_err(IpcError::from)?;
    Ok(res.into())
}

#[tauri::command]
pub async fn net_lobby_list_open_rooms(
    request: NetListOpenRoomsRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<NetListOpenRoomsResponse> {
    fetch_open_rooms(&state.old_backend_url, request.player_name.as_deref()).await
}

#[tauri::command]
pub async fn net_tunnel_ensure_binary(
    request: NetTunnelEnsureBinaryRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<NetTunnelEnsureBinaryResponse> {
    let dest = PathBuf::from(&request.destination_path);
    let path = ensure_binary_downloaded(&dest, Some(&state.event_bus))
        .await
        .map_err(|e| {
            IpcError::internal("Не удалось загрузить бинарник Playit", Some(e.to_string()))
        })?;
    Ok(NetTunnelEnsureBinaryResponse {
        path: path.to_string_lossy().to_string(),
        verified: true,
    })
}

#[tauri::command]
pub async fn net_tunnel_start(
    request: NetTunnelStartRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<NetTunnelStartResponse> {
    let mut sup_lock = state.supervisor.lock().await;
    let mut supervisor = PlayitSupervisor::new(Some(state.event_bus.clone()));
    let bin = PathBuf::from(&request.binary_path);
    let secret = PathBuf::from(&request.secret_key);
    supervisor.start(&bin, &secret).await.map_err(|e| {
        IpcError::internal("Не удалось запустить playit-agent", Some(e.to_string()))
    })?;
    *sup_lock = Some(supervisor);
    Ok(NetTunnelStartResponse { pid: 0 })
}

#[tauri::command]
pub async fn net_tunnel_stop(state: State<'_, LobbyState>) -> IpcResult<()> {
    let mut sup_lock = state.supervisor.lock().await;
    if let Some(mut sup) = sup_lock.take() {
        sup.stop().await;
    }
    Ok(())
}

#[tauri::command]
pub async fn net_tunnel_status(state: State<'_, LobbyState>) -> IpcResult<TunnelStatusDto> {
    let status = state.current_tunnel_status.lock().await.clone();
    Ok(status.into())
}

// ============================================================================
// High-Level UI Convenience Handlers
// ============================================================================

#[tauri::command]
pub async fn create_lobby_room(
    world_name: String,
    player_name: Option<String>,
    state: State<'_, LobbyState>,
) -> Result<LobbyRoomModel, String> {
    let host = player_name.unwrap_or_else(|| "Player".to_string());

    let created = state
        .client
        .create_lobby(&host, None)
        .await
        .map_err(|e| format_lobby_error(&e))?;

    let tunnel_cfg = state.client.get_tunnel_config().await.ok();
    let tunnel_addr = tunnel_cfg
        .as_ref()
        .and_then(|c| {
            c.public_address.as_ref().map(|a| {
                if let Some(p) = c.public_port {
                    format!("{a}:{p}")
                } else {
                    a.clone()
                }
            })
        })
        .unwrap_or_else(|| "tunnel.playit.gg:25565".to_string());

    let _ = state
        .client
        .open_lobby(&created.code, &created.host_token, &tunnel_addr, 25565)
        .await;

    let hb = state.client.start_heartbeat_loop(
        created.code.clone(),
        created.host_token.clone(),
        Duration::from_secs(15),
    );
    {
        let mut hb_lock = state.heartbeat_handle.lock().await;
        if let Some(old_hb) = hb_lock.replace(hb) {
            old_hb.stop();
        }
    }

    let display_code = if created.code.starts_with('#') {
        created.code.clone()
    } else {
        format!("#{}", created.code)
    };

    let room = LobbyRoomModel {
        code: display_code,
        host_name: host.clone(),
        world_name: world_name.clone(),
        tunnel_address: Some(tunnel_addr.clone()),
        tunnel_active: true,
        is_host: true,
        players: vec![LobbyPlayerModel {
            name: host.clone(),
            is_host: true,
            ping_ms: 0,
            mods_synced: true,
        }],
        mod_mismatch_count: 0,
        total_mods: 142,
    };

    let session = ActiveLobbySession {
        code: created.code,
        host_token: Some(created.host_token),
        host_name: host.clone(),
        world_name,
        player_name: host,
        is_host: true,
        tunnel_address: Some(tunnel_addr),
    };

    let mut session_lock = state.active_session.lock().await;
    *session_lock = Some(session);

    Ok(room)
}

#[tauri::command]
pub async fn join_lobby_room(
    code: String,
    player_name: Option<String>,
    state: State<'_, LobbyState>,
) -> Result<LobbyRoomModel, String> {
    let clean_code = code.trim().trim_start_matches('#').to_string();
    if clean_code.is_empty() {
        return Err("Введите корректный код комнаты".to_string());
    }
    let guest = player_name.unwrap_or_else(|| "Player".to_string());

    let joined = state
        .client
        .join_lobby(&clean_code, &guest, None)
        .await
        .map_err(|e| format_lobby_error(&e))?;

    if !joined.success {
        return Err("Не удалось подключиться к комнате лобби".to_string());
    }

    let status = state
        .client
        .get_status(&clean_code, Some(&guest))
        .await
        .map_err(|e| format_lobby_error(&e))?;

    let host_name = status.host_name.unwrap_or_else(|| "Хост".to_string());
    let players_list = status
        .players
        .unwrap_or_else(|| vec![host_name.clone(), guest.clone()]);

    let players = players_list
        .into_iter()
        .map(|name| {
            let is_host = name == host_name;
            LobbyPlayerModel {
                name,
                is_host,
                ping_ms: if is_host { 0 } else { 24 },
                mods_synced: true,
            }
        })
        .collect();

    let tunnel_addr = joined.tunnel_address.or(status.tunnel_address);
    let tunnel_active = tunnel_addr.is_some();

    let display_code = if code.starts_with('#') {
        code.clone()
    } else {
        format!("#{clean_code}")
    };

    let room = LobbyRoomModel {
        code: display_code,
        host_name: host_name.clone(),
        world_name: "Кооперативный мир Wex".to_string(),
        tunnel_address: tunnel_addr.clone(),
        tunnel_active,
        is_host: false,
        players,
        mod_mismatch_count: 0,
        total_mods: 142,
    };

    let session = ActiveLobbySession {
        code: clean_code,
        host_token: None,
        host_name,
        world_name: "Кооперативный мир Wex".to_string(),
        player_name: guest,
        is_host: false,
        tunnel_address: tunnel_addr,
    };

    let mut session_lock = state.active_session.lock().await;
    *session_lock = Some(session);

    Ok(room)
}

#[tauri::command]
pub async fn get_lobby_status(
    code: String,
    player_name: Option<String>,
    state: State<'_, LobbyState>,
) -> Result<Option<LobbyRoomModel>, String> {
    let clean_code = code.trim().trim_start_matches('#');
    let session = {
        let lock = state.active_session.lock().await;
        lock.clone()
    };

    let status = match state
        .client
        .get_status(clean_code, player_name.as_deref())
        .await
    {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };

    if status.kicked {
        let mut lock = state.active_session.lock().await;
        *lock = None;
        return Err("Вы были исключены из комнаты хостом".to_string());
    }

    let host_name = status
        .host_name
        .or_else(|| session.as_ref().map(|s| s.host_name.clone()))
        .unwrap_or_else(|| "Хост".to_string());

    let is_host = session.as_ref().map(|s| s.is_host).unwrap_or(false);
    let world_name = session
        .as_ref()
        .map(|s| s.world_name.clone())
        .unwrap_or_else(|| "Кооперативный мир Wex".to_string());

    let players_list = status.players.unwrap_or_else(|| vec![host_name.clone()]);
    let players = players_list
        .into_iter()
        .map(|name| {
            let is_h = name == host_name;
            let sync_info = status.mod_sync.as_ref().and_then(|m| m.get(&name));
            let mods_synced = sync_info.map(|i| i.status == "synced").unwrap_or(true);
            LobbyPlayerModel {
                name,
                is_host: is_h,
                ping_ms: if is_h { 0 } else { 20 },
                mods_synced,
            }
        })
        .collect();

    let tunnel_addr = status
        .tunnel_address
        .or_else(|| session.as_ref().and_then(|s| s.tunnel_address.clone()));
    let tunnel_active = tunnel_addr.is_some();

    Ok(Some(LobbyRoomModel {
        code: if code.starts_with('#') {
            code
        } else {
            format!("#{clean_code}")
        },
        host_name,
        world_name,
        tunnel_address: tunnel_addr,
        tunnel_active,
        is_host,
        players,
        mod_mismatch_count: 0,
        total_mods: 142,
    }))
}

#[tauri::command]
pub async fn close_lobby_room(code: String, state: State<'_, LobbyState>) -> Result<(), String> {
    let clean_code = code.trim().trim_start_matches('#');

    let mut hb_lock = state.heartbeat_handle.lock().await;
    if let Some(hb) = hb_lock.take() {
        hb.stop();
    }

    let mut sup_lock = state.supervisor.lock().await;
    if let Some(mut sup) = sup_lock.take() {
        sup.stop().await;
    }

    let session = {
        let mut lock = state.active_session.lock().await;
        lock.take()
    };

    if let Some(s) = session {
        if s.is_host {
            if let Some(token) = s.host_token {
                let _ = state.client.close_lobby(clean_code, &token).await;
            }
        } else {
            let _ = state.client.leave_lobby(clean_code, &s.player_name).await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn kick_lobby_player(
    code: String,
    player_name: String,
    state: State<'_, LobbyState>,
) -> Result<bool, String> {
    let clean_code = code.trim().trim_start_matches('#');
    let session = {
        let lock = state.active_session.lock().await;
        lock.clone()
    };

    if let Some(s) = session {
        if let Some(token) = s.host_token {
            let res = state
                .client
                .kick_player(clean_code, &token, &player_name)
                .await
                .map_err(|e| format_lobby_error(&e))?;
            return Ok(res);
        }
    }
    Err("Вы не являетесь организатором этой комнаты".to_string())
}

#[tauri::command]
pub async fn list_open_rooms(
    player_name: Option<String>,
    state: State<'_, LobbyState>,
) -> Result<Vec<NetOpenRoomDto>, String> {
    match fetch_open_rooms(&state.old_backend_url, player_name.as_deref()).await {
        Ok(res) => Ok(res.rooms),
        Err(e) => Err(e.message),
    }
}

fn lan_world_to_dto(w: wex_net::DiscoveredLanWorld) -> LanWorldDto {
    LanWorldDto {
        motd: w.clean_motd,
        address: w.address,
        port: w.port,
    }
}

#[tauri::command]
pub async fn net_lan_list_discovered(
    _request: NetLanListDiscoveredRequest,
    state: State<'_, LobbyState>,
) -> IpcResult<NetLanListDiscoveredResponse> {
    let worlds = state.lan_discovery.get_discovered_worlds().await;
    let list = worlds.into_iter().map(lan_world_to_dto).collect();
    Ok(NetLanListDiscoveredResponse { worlds: list })
}

#[tauri::command]
pub async fn list_lan_worlds(state: State<'_, LobbyState>) -> Result<Vec<LanWorldDto>, String> {
    let worlds = state.lan_discovery.get_discovered_worlds().await;
    Ok(worlds.into_iter().map(lan_world_to_dto).collect())
}

#[tauri::command]
pub async fn record_lan_world(
    packet: String,
    sender_ip: String,
    state: State<'_, LobbyState>,
) -> Result<Option<LanWorldDto>, String> {
    let world = state
        .lan_discovery
        .record_announcement(&packet, &sender_ip)
        .await;
    Ok(world.map(lan_world_to_dto))
}

#[tauri::command]
pub async fn connect_lan_world(
    address: String,
    version: Option<String>,
    nickname: Option<String>,
    play_state: State<'_, crate::commands::launch::PlayState>,
) -> Result<String, String> {
    let cfg = wex_core::config::LauncherConfig::load_or_default(&play_state.paths.config_file);
    let selected_ver = version.unwrap_or(cfg.selected_version);
    let player_nick = nickname.unwrap_or(cfg.nickname);

    let launch_req = wex_ipc::commands::LaunchGameRequest {
        version: selected_ver.clone(),
        nickname: player_nick.clone(),
        jvm_args: None,
        quick_play_multiplayer: Some(address.clone()),
    };

    match play_state.core.launch_game(launch_req).await {
        Ok(resp) => {
            play_state
                .is_running
                .store(true, std::sync::atomic::Ordering::SeqCst);
            {
                let mut v = play_state.running_version.lock().await;
                *v = Some(selected_ver);
            }
            Ok(format!(
                "Подключение к {} (PID {})",
                address,
                resp.pid.unwrap_or(0)
            ))
        }
        Err(e) => Err(format!("Ошибка автоподключения к LAN-миру: {}", e.message)),
    }
}

// ============================================================================
// Unit Tests (Mock Core/Backend)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_create_lobby_and_heartbeat_flow() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/lobby"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "A7X9",
                "hostToken": "secret_token_123",
                "status": "open",
                "createdAt": 1731000000
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/tunnel-config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "secret": "tunnel_sec",
                "publicAddress": "pgsql-jill.tun.ply.gg",
                "publicPort": 38062
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/lobby/open"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/lobby/heartbeat"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let state = LobbyState::new(
            tmp.path().to_path_buf(),
            &mock_server.uri(),
            &mock_server.uri(),
        );

        // Test net_lobby_create directly
        let res = net_lobby_create_direct(&state, "Steve").await.unwrap();
        assert_eq!(res.code, "A7X9");
        assert_eq!(res.host_token, "secret_token_123");

        // Test heartbeat
        let hb_res = net_lobby_heartbeat_direct(&state, "A7X9", "secret_token_123")
            .await
            .unwrap();
        assert!(hb_res);
    }

    #[tokio::test]
    async fn test_join_status_and_kick_flow() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/lobby/join"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true,
                "code": "A7X9",
                "status": "open",
                "tunnelAddress": "pgsql-jill.tun.ply.gg:38062",
                "playerCount": 2
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/api/lobby/status"))
            .and(query_param("code", "A7X9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "A7X9",
                "status": "open",
                "tunnelAddress": "pgsql-jill.tun.ply.gg:38062",
                "playerCount": 2,
                "lastHeartbeat": 1731000000,
                "players": ["Steve", "Alex"],
                "hostName": "Steve",
                "kicked": false
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/lobby/kick"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let state = LobbyState::new(
            tmp.path().to_path_buf(),
            &mock_server.uri(),
            &mock_server.uri(),
        );

        let join_res = net_lobby_join_direct(&state, "A7X9", "Alex").await.unwrap();
        assert!(join_res.success);
        assert_eq!(
            join_res.tunnel_address.as_deref(),
            Some("pgsql-jill.tun.ply.gg:38062")
        );

        let status_res = net_lobby_status_direct(&state, "A7X9", Some("Alex"))
            .await
            .unwrap();
        assert_eq!(status_res.players.unwrap().len(), 2);

        let kick_res = net_lobby_kick_direct(&state, "A7X9", "tok", "Alex")
            .await
            .unwrap();
        assert!(kick_res);
    }

    #[tokio::test]
    async fn test_open_rooms_service_unavailable_when_offline() {
        // Query an unreachable port to guarantee connection error
        let err = fetch_open_rooms("http://127.0.0.1:59999", Some("Player"))
            .await
            .unwrap_err();
        assert_eq!(err.code, "NET_LOBBY_API_ERROR");
        assert!(err.message.contains("Сервис недоступен"));
    }

    #[tokio::test]
    async fn test_open_rooms_success_when_online() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/worlds"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "max_active": 5,
                "active_count": 1,
                "worlds": [
                    {
                        "id": "world-abc",
                        "name": "Аура Выживание",
                        "owner": "ProHost",
                        "port": 25565,
                        "status": "running",
                        "online_players": ["ProHost", "Friend1"],
                        "max_players": 8,
                        "game_version": "1.21.1"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let res = fetch_open_rooms(&mock_server.uri(), Some("Steve"))
            .await
            .unwrap();
        assert_eq!(res.rooms.len(), 1);
        assert_eq!(res.rooms[0].name, "Аура Выживание");
        assert_eq!(res.rooms[0].player_count, 2);
        assert_eq!(res.rooms[0].address.as_deref(), Some("89.168.82.200:25565"));
    }

    // Direct helper wrappers for unit testing without State wrapper
    async fn net_lobby_create_direct(
        state: &LobbyState,
        host_name: &str,
    ) -> IpcResult<LobbyCreateResponseDto> {
        let res = state
            .client
            .create_lobby(host_name, None)
            .await
            .map_err(IpcError::from)?;
        Ok(res.into())
    }

    async fn net_lobby_join_direct(
        state: &LobbyState,
        code: &str,
        player_name: &str,
    ) -> IpcResult<LobbyJoinResponseDto> {
        let res = state
            .client
            .join_lobby(code, player_name, None)
            .await
            .map_err(IpcError::from)?;
        Ok(res.into())
    }

    async fn net_lobby_status_direct(
        state: &LobbyState,
        code: &str,
        player_name: Option<&str>,
    ) -> IpcResult<LobbyStatusResponseDto> {
        let res = state
            .client
            .get_status(code, player_name)
            .await
            .map_err(IpcError::from)?;
        Ok(res.into())
    }

    async fn net_lobby_heartbeat_direct(
        state: &LobbyState,
        code: &str,
        host_token: &str,
    ) -> IpcResult<bool> {
        let res = state
            .client
            .heartbeat(code, host_token)
            .await
            .map_err(IpcError::from)?;
        Ok(res)
    }

    async fn net_lobby_kick_direct(
        state: &LobbyState,
        code: &str,
        host_token: &str,
        player: &str,
    ) -> IpcResult<bool> {
        let res = state
            .client
            .kick_player(code, host_token, player)
            .await
            .map_err(IpcError::from)?;
        Ok(res)
    }

    #[tokio::test]
    async fn test_lan_world_discovery_service_integration() {
        let tmp = tempdir().unwrap();
        let state = LobbyState::new(
            tmp.path().to_path_buf(),
            DEFAULT_LOBBY_API_URL,
            DEFAULT_OLD_BACKEND_URL,
        );

        let packet = "[MOTD]Steve's Underground Base[/MOTD][AD]51234[/AD]";
        let discovered = state
            .lan_discovery
            .record_announcement(packet, "192.168.1.77")
            .await
            .expect("should record announcement");

        assert_eq!(discovered.clean_motd, "Steve's Underground Base");
        assert_eq!(discovered.address, "192.168.1.77:51234");
        assert_eq!(discovered.port, 51234);

        let list = state.lan_discovery.get_discovered_worlds().await;
        assert_eq!(list.len(), 1);
        let dto = lan_world_to_dto(list[0].clone());
        assert_eq!(dto.motd, "Steve's Underground Base");
        assert_eq!(dto.address, "192.168.1.77:51234");
        assert_eq!(dto.port, 51234);
    }
}
