pub mod commands;
pub mod ipc;

use commands::launch::PlayState;
use tauri::Manager;
use wex_core::config::WexPaths;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let paths = WexPaths::new(None);
            let _ = paths.ensure_dirs();
            let play_state = PlayState::new_real(paths, app.handle().clone());
            app.manage(play_state);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
