pub mod commands;
pub mod ipc;

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

use commands::launch::PlayState;
use ipc::{LobbyPlayerModel, LobbyRoomModel, ServerEntryModel, ServerPingResult};
use wex_core::config::WexPaths;

pub struct AppState {
    pub servers: Mutex<Vec<ServerEntryModel>>,
    pub active_lobby: Mutex<Option<LobbyRoomModel>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            servers: Mutex::new(vec![
                ServerEntryModel {
                    id: "aura-main".to_string(),
                    name: "Aura Official — Survival & PvP".to_string(),
                    address: "mc.auramc.net".to_string(),
                    icon: None,
                    ping_ms: None,
                    online: None,
                    max_players: None,
                    motd: Some("Официальный сервер Aura Client • 1.20.1 & 1.21.1".to_string()),
                    version: Some("1.21.1 / 1.20.1".to_string()),
                },
                ServerEntryModel {
                    id: "aura-modded".to_string(),
                    name: "Aura Pack Community Server".to_string(),
                    address: "pack.auramc.net".to_string(),
                    icon: None,
                    ping_ms: None,
                    online: None,
                    max_players: None,
                    motd: Some("Сервер сообщества Aura-Pack".to_string()),
                    version: Some("1.21.1".to_string()),
                },
            ]),
            active_lobby: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Social News (SPEC.md 1.3)
// ---------------------------------------------------------------------------

#[tauri::command]
fn social_get_news(state: State<'_, PlayState>) -> Vec<wex_social::ReleaseNoteVersion> {
    let cache_dir = state.paths.root.join("news");
    let mgr = wex_social::NewsManager::new(cache_dir);
    mgr.load_cached_or_default()
}

// ---------------------------------------------------------------------------
// Servers Management (powered by core/wex-net)
// ---------------------------------------------------------------------------

#[tauri::command]
fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerEntryModel>, String> {
    state
        .servers
        .lock()
        .map(|s| s.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn ping_server(address: String) -> Result<ServerPingResult, String> {
    let pinger = wex_net::ServerPinger::new(2500, 2500, 763);
    match pinger.ping(&address).await {
        Ok(res) => Ok(ServerPingResult {
            latency_ms: res.latency_ms,
            online: res.players.online,
            max_players: res.players.max,
            motd: res.description,
            version: res.version.name,
            favicon: res.favicon,
        }),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn add_server(
    name: String,
    address: String,
    state: State<'_, AppState>,
) -> Result<ServerEntryModel, String> {
    let new_server = ServerEntryModel {
        id: format!(
            "server-{}",
            std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_millis()
        ),
        name: name.clone(),
        address: address.clone(),
        icon: None,
        ping_ms: None,
        online: None,
        max_players: None,
        motd: None,
        version: None,
    };

    let mut list = state.servers.lock().map_err(|e| e.to_string())?;
    list.push(new_server.clone());

    if let Ok(appdata) = std::env::var("APPDATA") {
        let servers_dat_path = PathBuf::from(appdata).join("Wex").join("servers.dat");
        if let Some(parent) = servers_dat_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let pack_servers: Vec<wex_net::PackServerEntry> = list
            .iter()
            .map(|s| wex_net::PackServerEntry {
                id: Some(s.id.clone()),
                name: s.name.clone(),
                ip: s.address.clone(),
                icon: s.icon.clone(),
            })
            .collect();

        let mut root = wex_net::load_servers_dat(&servers_dat_path).unwrap_or_else(|_| {
            wex_net::NbtTag::Compound(vec![(
                "servers".to_string(),
                wex_net::NbtTag::List {
                    tag_id: 10,
                    elements: Vec::new(),
                },
            )])
        });
        wex_net::merge_pack_servers(&mut root, &pack_servers);
        let _ = wex_net::save_servers_dat(&servers_dat_path, &root);
    }

    Ok(new_server)
}

#[tauri::command]
fn remove_server(address: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut list = state.servers.lock().map_err(|e| e.to_string())?;
    list.retain(|s| s.address != address);

    if let Ok(appdata) = std::env::var("APPDATA") {
        let servers_dat_path = PathBuf::from(appdata).join("Wex").join("servers.dat");
        let mut root = wex_net::load_servers_dat(&servers_dat_path).unwrap_or_else(|_| {
            wex_net::NbtTag::Compound(vec![(
                "servers".to_string(),
                wex_net::NbtTag::List {
                    tag_id: 10,
                    elements: Vec::new(),
                },
            )])
        });
        if let Some(wex_net::NbtTag::List { elements, .. }) = root.get_mut("servers") {
            elements.retain(|elem| {
                if let wex_net::NbtTag::Compound(fields) = elem {
                    !fields.iter().any(|(k, v)| {
                        k == "ip"
                            && matches!(v, wex_net::NbtTag::String(ip_str) if ip_str == &address)
                    })
                } else {
                    true
                }
            });
        }
        let _ = wex_net::save_servers_dat(&servers_dat_path, &root);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Lobby Management (SPEC.md 1.4)
// ---------------------------------------------------------------------------

#[tauri::command]
async fn create_lobby_room(
    world_name: String,
    state: State<'_, AppState>,
) -> Result<LobbyRoomModel, String> {
    let code = format!(
        "#{:X}",
        std::time::SystemTime::now()
            .elapsed()
            .unwrap_or_default()
            .as_millis()
            % 0xFFFF
    );

    let room = LobbyRoomModel {
        code,
        host_name: "HostPlayer".to_string(),
        world_name,
        tunnel_address: Some("tunnel.playit.gg:25565".to_string()),
        tunnel_active: true,
        is_host: true,
        players: vec![LobbyPlayerModel {
            name: "HostPlayer".to_string(),
            is_host: true,
            ping_ms: 0,
            mods_synced: true,
        }],
        mod_mismatch_count: 0,
        total_mods: 142,
    };

    let mut active = state.active_lobby.lock().map_err(|e| e.to_string())?;
    *active = Some(room.clone());
    Ok(room)
}

#[tauri::command]
async fn join_lobby_room(
    code: String,
    state: State<'_, AppState>,
) -> Result<LobbyRoomModel, String> {
    let room = LobbyRoomModel {
        code: code.clone(),
        host_name: "HostPlayer".to_string(),
        world_name: "Survival World".to_string(),
        tunnel_address: Some("tunnel.playit.gg:25565".to_string()),
        tunnel_active: true,
        is_host: false,
        players: vec![
            LobbyPlayerModel {
                name: "HostPlayer".to_string(),
                is_host: true,
                ping_ms: 24,
                mods_synced: true,
            },
            LobbyPlayerModel {
                name: "Player".to_string(),
                is_host: false,
                ping_ms: 18,
                mods_synced: true,
            },
        ],
        mod_mismatch_count: 0,
        total_mods: 142,
    };

    let mut active = state.active_lobby.lock().map_err(|e| e.to_string())?;
    *active = Some(room.clone());
    Ok(room)
}

#[tauri::command]
fn get_lobby_status(
    code: String,
    state: State<'_, AppState>,
) -> Result<Option<LobbyRoomModel>, String> {
    let active = state.active_lobby.lock().map_err(|e| e.to_string())?;
    if let Some(ref r) = *active {
        if r.code == code {
            return Ok(Some(r.clone()));
        }
    }
    Ok(None)
}

#[tauri::command]
fn close_lobby_room(_code: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut active = state.active_lobby.lock().map_err(|e| e.to_string())?;
    *active = None;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri Application Runner
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let paths = WexPaths::new(None);
            let _ = paths.ensure_dirs();
            let play_state = PlayState::new_real(paths, app.handle().clone());
            app.manage(play_state);
            app.manage(AppState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Standard wex-ipc commands
            commands::launch::get_config,
            commands::launch::save_config,
            commands::launch::get_installed_versions,
            commands::launch::start_game,
            commands::launch::cancel_launch,
            commands::launch::is_game_running,
            commands::launch::kill_game,
            // UI compatibility aliases
            commands::launch::launch_game,
            commands::launch::set_config,
            // Settings commands (commands/settings.rs)
            commands::settings::get_launcher_settings,
            commands::settings::save_launcher_settings,
            commands::settings::reset_settings,
            commands::settings::open_game_data_dir,
            commands::settings::get_system_info,
            commands::settings::get_data_dir,
            commands::settings::get_launcher_config,
            commands::settings::save_launcher_config,
            commands::settings::open_data_dir,
            // Mods commands (commands/mods.rs)
            commands::mods::list_mods,
            commands::mods::toggle_mod,
            commands::mods::add_custom_mod,
            commands::mods::delete_custom_mod,
            commands::mods::open_mods_dir,
            // Social news
            social_get_news,
            // Servers commands
            list_servers,
            ping_server,
            add_server,
            remove_server,
            // Lobby commands
            create_lobby_room,
            join_lobby_room,
            get_lobby_status,
            close_lobby_room,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
