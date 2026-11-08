pub mod commands;
pub mod ipc;

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Manager, State};

use commands::launch::PlayState;
use ipc::{LobbyRoomModel, ServerEntryModel, ServerPingResult};
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
// Tauri Application Runner
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let paths = WexPaths::new(None);
            let _ = paths.ensure_dirs();
            let network_state = commands::network::NetworkState::new();
            network_state.start_supervisor(app.handle().clone());
            let play_state =
                PlayState::new_real(paths.clone(), app.handle().clone(), network_state.clone());
            app.manage(play_state);
            app.manage(network_state);
            app.manage(commands::friends::FriendsState::new(paths.clone()));
            app.manage(AppState::default());
            let lobby_state = commands::lobby::LobbyState::new(
                paths.root.join("tools"),
                commands::lobby::DEFAULT_LOBBY_API_URL,
                commands::lobby::DEFAULT_OLD_BACKEND_URL,
            );
            app.manage(lobby_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Network & Connectivity commands
            commands::network::get_network_status,
            // Standard wex-ipc commands
            commands::launch::get_config,
            commands::launch::save_config,
            commands::launch::get_installed_versions,
            commands::launch::start_game,
            commands::launch::cancel_launch,
            commands::launch::is_game_running,
            commands::launch::kill_game,
            commands::launch::get_playtime_summary,
            commands::launch::playtime_get_summary,
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
            commands::settings::verify_instance_files,
            commands::settings::settings_verify_files,
            commands::settings::cleanup_cache,
            commands::settings::settings_cleanup_cache,
            // Mods commands (commands/mods.rs)
            commands::mods::list_mods,
            commands::mods::toggle_mod,
            commands::mods::add_custom_mod,
            commands::mods::delete_custom_mod,
            commands::mods::open_mods_dir,
            // Overview & Social/Extras commands
            commands::overview::social_get_news,
            commands::overview::overview_get_news,
            commands::overview::social_check_launcher_update,
            commands::overview::overview_check_update,
            commands::overview::extras_detect_old_launcher,
            commands::overview::extras_import_old_launcher,
            commands::overview::extras_apply_import,
            // Servers commands
            list_servers,
            ping_server,
            add_server,
            remove_server,
            // Lobby commands (from commands::lobby)
            commands::lobby::net_lobby_create,
            commands::lobby::net_lobby_join,
            commands::lobby::net_lobby_open,
            commands::lobby::net_lobby_status,
            commands::lobby::net_lobby_heartbeat,
            commands::lobby::net_lobby_close,
            commands::lobby::net_lobby_leave,
            commands::lobby::net_lobby_kick,
            commands::lobby::net_lobby_tunnel_config,
            commands::lobby::net_lobby_list_open_rooms,
            commands::lobby::net_tunnel_ensure_binary,
            commands::lobby::net_tunnel_start,
            commands::lobby::net_tunnel_stop,
            commands::lobby::net_tunnel_status,
            commands::lobby::create_lobby_room,
            commands::lobby::join_lobby_room,
            commands::lobby::get_lobby_status,
            commands::lobby::close_lobby_room,
            commands::lobby::kick_lobby_player,
            commands::lobby::list_open_rooms,
            // Wardrobe commands
            commands::wardrobe::extras_validate_skin,
            commands::wardrobe::extras_convert_skin,
            commands::wardrobe::extras_save_skin,
            commands::wardrobe::extras_list_skin_history,
            commands::wardrobe::extras_generate_csl_config,
            commands::wardrobe::wardrobe_delete_history,
            commands::wardrobe::wardrobe_get_state,
            commands::wardrobe::wardrobe_apply,
            // Friends & Social commands
            commands::friends::social_login,
            commands::friends::social_get_friends,
            commands::friends::social_send_friend_request,
            commands::friends::social_accept_friend_request,
            commands::friends::social_decline_friend_request,
            commands::friends::social_sync,
            commands::friends::social_send_lobby_invite,
            commands::friends::social_respond_lobby_invite,
            commands::friends::social_set_presence,
            commands::friends::social_list_achievements,
            commands::friends::social_report_achievement_progress,
            // Workshop commands (SPEC 1.7)
            commands::workshop::workshop_list_worlds,
            commands::workshop::social_list_worlds,
            commands::workshop::workshop_create_world_backup,
            commands::workshop::social_create_world_backup,
            commands::workshop::workshop_list_world_backups,
            commands::workshop::social_list_world_backups,
            commands::workshop::workshop_restore_world_backup,
            commands::workshop::social_restore_world_backup,
            commands::workshop::workshop_delete_world,
            commands::workshop::workshop_list_screenshots,
            commands::workshop::social_list_screenshots,
            commands::workshop::workshop_open_path,
            // Workshop packs and shaders commands (SPEC 1.7)
            commands::workshop_packs::workshop_list_resource_packs,
            commands::workshop_packs::workshop_toggle_resource_pack,
            commands::workshop_packs::workshop_set_resource_packs_order,
            commands::workshop_packs::workshop_list_shader_packs,
            commands::workshop_packs::workshop_set_active_shader_pack,
            commands::workshop_packs::workshop_add_pack_file,
            commands::workshop_packs::workshop_delete_pack_file,
            commands::workshop_packs::workshop_open_packs_folder,
            // Crash diagnosis commands (SPEC 2.10)
            commands::crash::extras_analyze_crash,
            commands::crash::crash_get_last_diagnosis,
            commands::crash::crash_open_log_file,
            commands::crash::crash_disable_mod,
            // Shell commands (SPEC 1.2)
            commands::shell::shell_check_network,
            commands::shell::shell_get_profile,
            commands::shell::shell_get_launch_state,
            commands::shell::shell_stop_game,
            commands::shell::shell_minimize_window,
            commands::shell::shell_toggle_maximize_window,
            commands::shell::shell_close_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
