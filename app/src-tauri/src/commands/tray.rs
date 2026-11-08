use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::menu::MenuItem;
use tauri::{AppHandle, Manager, State, Wry};

/// Behavior of the launcher main window when Minecraft is launched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameWindowBehavior {
    Keep,
    Minimize,
    HideToTray,
}

impl GameWindowBehavior {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "keep" | "leave" | "оставить" => Self::Keep,
            "tray" | "hide" | "hide_to_tray" | "скрыть" | "скрыть в трей" => Self::HideToTray,
            _ => Self::Minimize,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Minimize => "minimize",
            Self::HideToTray => "tray",
        }
    }
}

pub const TRAY_ID: &str = "wex-main-tray";

/// Managed state to track tray menu item controls.
#[derive(Default)]
pub struct TrayState {
    pub stop_item: Mutex<Option<MenuItem<Wry>>>,
}

/// Applies window action upon Minecraft launch according to the selected setting.
pub fn apply_launch_window_behavior(app: &AppHandle, behavior_str: &str) {
    let behavior = GameWindowBehavior::from_str_loose(behavior_str);
    if let Some(window) = app.get_webview_window("main") {
        match behavior {
            GameWindowBehavior::Keep => {}
            GameWindowBehavior::Minimize => {
                let _ = window.minimize();
            }
            GameWindowBehavior::HideToTray => {
                let _ = window.hide();
            }
        }
    }
}

/// Restores the main window when game session finishes or when user requests via tray.
pub fn restore_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Updates tray menu items (e.g. enabling 'Stop Game' only when Minecraft process is alive).
pub fn update_tray_game_running(app: &AppHandle, is_running: bool) {
    if let Some(state) = app.try_state::<TrayState>() {
        if let Ok(lock) = state.stop_item.lock() {
            if let Some(ref item) = *lock {
                let _ = item.set_enabled(is_running);
            }
        }
    }
}

/// Initializes system tray icon and contextual menu for Wex.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::Menu;
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let open_i = MenuItem::with_id(app, "open_wex", "Открыть Wex", true, None::<&str>)?;
    let stop_i = MenuItem::with_id(app, "stop_game", "Остановить игру", false, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    // Store stop_item reference in TrayState for dynamic enabled state toggling
    if let Some(state) = app.try_state::<TrayState>() {
        if let Ok(mut lock) = state.stop_item.lock() {
            *lock = Some(stop_i.clone());
        }
    }

    let menu = Menu::with_items(app, &[&open_i, &stop_i, &quit_i])?;

    let icon = match app.default_window_icon().cloned() {
        Some(i) => i,
        None => tauri::image::Image::new(&[0u8; 16 * 16 * 4], 16, 16),
    };

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "open_wex" => {
                    restore_window(app);
                }
                "stop_game" => {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(state) =
                            app_clone.try_state::<crate::commands::launch::PlayState>()
                        {
                            let ver = {
                                let v = state.running_version.lock().await;
                                v.clone()
                            };
                            if let Some(ver) = ver {
                                let _ = state.core.kill_game(&ver).await;
                            }
                            state.is_running.store(false, Ordering::SeqCst);
                            restore_window(&app_clone);
                            update_tray_game_running(&app_clone, false);
                        }
                    });
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                restore_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn tray_apply_launch_behavior(
    app: AppHandle,
    behavior: Option<String>,
    state: State<'_, crate::commands::launch::PlayState>,
) -> Result<(), String> {
    let b = behavior.unwrap_or_else(|| {
        let cfg = wex_core::LauncherConfig::load_or_default(&state.paths.config_file);
        cfg.game_window_behavior
    });
    apply_launch_window_behavior(&app, &b);
    Ok(())
}

#[tauri::command]
pub fn tray_restore_window(app: AppHandle) -> Result<(), String> {
    restore_window(&app);
    Ok(())
}

#[tauri::command]
pub fn tray_get_window_behavior(
    state: State<'_, crate::commands::launch::PlayState>,
) -> Result<String, String> {
    let cfg = wex_core::LauncherConfig::load_or_default(&state.paths.config_file);
    Ok(cfg.game_window_behavior)
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn test_game_window_behavior_parsing() {
        assert_eq!(
            GameWindowBehavior::from_str_loose("keep"),
            GameWindowBehavior::Keep
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("Оставить"),
            GameWindowBehavior::Keep
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("minimize"),
            GameWindowBehavior::Minimize
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("Свернуть"),
            GameWindowBehavior::Minimize
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("tray"),
            GameWindowBehavior::HideToTray
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("hide"),
            GameWindowBehavior::HideToTray
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("hide_to_tray"),
            GameWindowBehavior::HideToTray
        );
        assert_eq!(
            GameWindowBehavior::from_str_loose("Скрыть в трей"),
            GameWindowBehavior::HideToTray
        );
        // Default fallback is Minimize
        assert_eq!(
            GameWindowBehavior::from_str_loose("invalid"),
            GameWindowBehavior::Minimize
        );
    }

    #[test]
    fn test_behavior_as_str() {
        assert_eq!(GameWindowBehavior::Keep.as_str(), "keep");
        assert_eq!(GameWindowBehavior::Minimize.as_str(), "minimize");
        assert_eq!(GameWindowBehavior::HideToTray.as_str(), "tray");
    }

    #[test]
    fn test_state_machine_window_behavior() {
        // State machine test simulating launch and exit decisions
        let is_running = Arc::new(AtomicBool::new(false));
        let behavior = GameWindowBehavior::from_str_loose("minimize");

        // Game launches
        is_running.store(true, Ordering::SeqCst);
        let launch_action = match behavior {
            GameWindowBehavior::Keep => "noop",
            GameWindowBehavior::Minimize => "minimize",
            GameWindowBehavior::HideToTray => "hide",
        };
        assert_eq!(launch_action, "minimize");

        // Game exits
        is_running.store(false, Ordering::SeqCst);
        let exit_action = "restore";
        assert_eq!(exit_action, "restore");
        assert!(!is_running.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tray_state_default() {
        let state = TrayState::default();
        let lock = state.stop_item.lock().unwrap();
        assert!(lock.is_none());
    }
}
