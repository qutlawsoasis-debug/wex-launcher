pub mod commands;
pub mod ipc;

pub use commands::servers::*;

use std::sync::Mutex;
use tauri::{Emitter, Manager};

use commands::launch::PlayState;
use ipc::{LobbyRoomModel, ServerEntryModel};
use wex_core::config::WexPaths;

pub struct AppState {
    pub servers: Mutex<Vec<ServerEntryModel>>,
    pub active_lobby: Mutex<Option<LobbyRoomModel>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            servers: Mutex::new(Vec::new()),
            active_lobby: Mutex::new(None),
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
                for arg in argv {
                    if arg.starts_with("wex://") || arg.starts_with("aura://") {
                        let _ = w.emit("deep-link://url", vec![arg]);
                        break;
                    }
                }
            }
        }))
        .plugin(tauri_plugin_notification::init())
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
            let _ = commands::window::setup_tray(app);
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
            commands::settings::get_discord_rpc_preview,
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
            commands::overview::overview_get_playtime,
            commands::overview::overview_record_playtime,
            // Servers commands (commands/servers.rs)
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
            commands::lobby::net_lan_list_discovered,
            commands::lobby::net_lan_publish,
            commands::lobby::list_lan_worlds,
            commands::lobby::record_lan_world,
            commands::lobby::connect_lan_world,
            commands::lobby::net_lan_start_discovery,
            commands::lobby::net_lan_stop_discovery,
            commands::lobby::net_lan_is_discovering,
            commands::lobby::publish_lan_world,
            commands::lobby::retry_lobby_tunnel,
            commands::lobby::net_compare_manifests,
            // Wardrobe commands
            commands::wardrobe::extras_validate_skin,
            commands::wardrobe::extras_convert_skin,
            commands::wardrobe::extras_save_skin,
            commands::wardrobe::extras_list_skin_history,
            commands::wardrobe::extras_generate_csl_config,
            commands::wardrobe::wardrobe_delete_history,
            commands::wardrobe::wardrobe_get_skin,
            commands::wardrobe::wardrobe_get_state,
            commands::wardrobe::wardrobe_apply,
            // Music commands
            commands::music::music_get_playlist,
            commands::music::music_get_track_data,
            commands::music::music_open_folder,
            commands::music::music_get_settings,
            commands::music::music_set_settings,
            commands::music::music_sync_smtc,
            commands::music::music_get_smtc_state,
            commands::music::music_update_session,
            commands::music::music_check_command,
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
            commands::workshop::workshop_open_screenshots_folder,
            commands::workshop::workshop_copy_screenshot_image,
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
            // Report commands (SPEC 2.10)
            commands::report::report_get_preview,
            commands::report::report_save_zip,
            commands::report::report_copy_text,
            // Window & Tray commands (SPEC wex-window)
            commands::window::window_get_state,
            commands::window::window_save_state,
            commands::window::window_restore_saved_state,
            commands::window::window_get_minimize_to_tray,
            commands::window::window_set_minimize_to_tray,
            commands::window::window_on_game_launch,
            commands::window::window_on_game_exit,
            commands::window::window_raise_and_focus,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
