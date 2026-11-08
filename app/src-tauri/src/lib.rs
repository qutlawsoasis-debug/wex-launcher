pub mod commands;
pub mod ipc;

use std::sync::Mutex;
use tauri::{Manager, State};

use commands::launch::PlayState;
use ipc::{LobbyPlayerModel, LobbyRoomModel, ServerEntryModel, SystemInfo};
use wex_core::config::WexPaths;

#[cfg(windows)]
fn get_total_ram_mb() -> u32 {
    use std::mem::zeroed;
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    unsafe {
        let mut status: MemoryStatusEx = zeroed();
        status.length = std::mem::size_of::<MemoryStatusEx>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            (status.total_phys / (1024 * 1024)) as u32
        } else {
            16384
        }
    }
}

#[cfg(not(windows))]
fn get_total_ram_mb() -> u32 {
    16384
}

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
// System Info & Telemetry
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_system_info(state: State<'_, PlayState>) -> SystemInfo {
    SystemInfo {
        total_memory_mb: get_total_ram_mb(),
        os_name: std::env::consts::OS.to_string(),
        launcher_version: "1.0.0".to_string(),
        data_dir: state.paths.root.to_string_lossy().to_string(),
    }
}

#[tauri::command]
fn get_data_dir(state: State<'_, PlayState>) -> String {
    state.paths.root.to_string_lossy().to_string()
}

#[tauri::command]
fn open_data_dir(state: State<'_, PlayState>) -> Result<(), String> {
    let path = &state.paths.root;
    if !path.exists() {
        let _ = std::fs::create_dir_all(path);
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn social_get_news(state: State<'_, PlayState>) -> Vec<wex_social::ReleaseNoteVersion> {
    let cache_dir = state.paths.root.join("news");
    let mgr = wex_social::NewsManager::new(cache_dir);
    mgr.load_cached_or_default()
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
            commands::launch::get_launcher_config,
            commands::launch::save_launcher_config,
            commands::launch::set_config,
            // System info
            get_system_info,
            get_data_dir,
            open_data_dir,
            social_get_news,
            // Servers commands
            commands::servers::list_servers,
            commands::servers::ping_server,
            commands::servers::add_server,
            commands::servers::edit_server,
            commands::servers::remove_server,
            commands::servers::net_load_servers,
            commands::servers::net_save_servers,
            commands::servers::net_merge_servers,
            commands::servers::net_ping_server,
            commands::servers::net_resolve_address,
            // Lobby commands
            create_lobby_room,
            join_lobby_room,
            get_lobby_status,
            close_lobby_room,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
