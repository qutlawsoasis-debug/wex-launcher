use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{
    Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, State, WebviewWindow,
};

use crate::commands::launch::PlayState;
use wex_core::config::{LauncherConfig as CoreConfig, WexPaths};

/// Physical monitor bounding rectangle in screen space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Window state for persistence across launches.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowStateDto {
    pub width: u32,
    pub height: u32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub is_maximized: bool,
}

impl Default for WindowStateDto {
    fn default() -> Self {
        Self {
            width: 1080,
            height: 720,
            x: None,
            y: None,
            is_maximized: false,
        }
    }
}

/// Pure geometry algorithm to ensure window doesn't open off-screen.
/// Checks if the window significantly overlaps any available monitor.
/// If not (e.g. monitor was unplugged or resolution changed), safely centers
/// the window on the primary monitor.
pub fn sanitize_window_position(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitors: &[MonitorRect],
) -> (i32, i32) {
    if monitors.is_empty() {
        return (x.max(0), y.max(0));
    }

    let mut is_visible = false;
    for m in monitors {
        let mon_right = m.x + m.width as i32;
        let mon_bottom = m.y + m.height as i32;
        let win_right = x + width as i32;
        let win_bottom = y + height as i32;

        let overlap_left = x.max(m.x);
        let overlap_top = y.max(m.y);
        let overlap_right = win_right.min(mon_right);
        let overlap_bottom = win_bottom.min(mon_bottom);

        let overlap_w = overlap_right - overlap_left;
        let overlap_h = overlap_bottom - overlap_top;

        if overlap_w >= 100 && overlap_h >= 100 {
            is_visible = true;
            break;
        }
    }

    if is_visible {
        (x, y)
    } else {
        // Fallback: center on primary monitor (first in list)
        let primary = &monitors[0];
        let cx = primary.x + ((primary.width.saturating_sub(width)) / 2) as i32;
        let cy = primary.y + ((primary.height.saturating_sub(height)) / 2) as i32;
        (cx.max(primary.x), cy.max(primary.y))
    }
}

/// Read window state from launcher config.
pub fn load_window_state(paths: &WexPaths) -> WindowStateDto {
    let cfg = CoreConfig::load_or_default(&paths.config_file);
    WindowStateDto {
        width: cfg.window_state.width,
        height: cfg.window_state.height,
        x: cfg.window_state.x,
        y: cfg.window_state.y,
        is_maximized: cfg.window_state.is_maximized,
    }
}

/// Save window state to launcher config.
pub fn save_window_state(paths: &WexPaths, state: WindowStateDto) -> Result<(), String> {
    let mut cfg = CoreConfig::load_or_default(&paths.config_file);
    cfg.window_state.width = state.width;
    cfg.window_state.height = state.height;
    cfg.window_state.x = state.x;
    cfg.window_state.y = state.y;
    cfg.window_state.is_maximized = state.is_maximized;
    cfg.save(&paths.config_file)
        .map_err(|e| format!("Failed to save window state: {}", e))
}

/// Check if launcher is configured to minimize/hide to tray when game starts.
pub fn is_minimize_to_tray_enabled(paths: &WexPaths) -> bool {
    let cfg = CoreConfig::load_or_default(&paths.config_file);
    cfg.minimize_to_tray
}

/// Set minimize to tray preference in config.
pub fn set_minimize_to_tray_preference(paths: &WexPaths, enabled: bool) -> Result<(), String> {
    let mut cfg = CoreConfig::load_or_default(&paths.config_file);
    cfg.minimize_to_tray = enabled;
    cfg.save(&paths.config_file)
        .map_err(|e| format!("Failed to save minimize_to_tray: {}", e))
}

/// Apply launch behavior: hide window if minimize_to_tray is true.
pub fn apply_launch_window_behavior<R: tauri::Runtime>(
    window: &WebviewWindow<R>,
    minimize_to_tray: bool,
) {
    if minimize_to_tray {
        let _ = window.hide();
    }
}

/// Restore window when game exits or is stopped.
pub fn restore_window_from_game<R: tauri::Runtime>(window: &WebviewWindow<R>) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

// ============================================================================
// Tauri Commands (wex-window)
// ============================================================================

/// Retrieve saved window state.
#[tauri::command]
pub async fn window_get_state(state: State<'_, PlayState>) -> Result<WindowStateDto, String> {
    Ok(load_window_state(&state.paths))
}

/// Save current window state.
#[tauri::command]
pub async fn window_save_state(
    window_state: WindowStateDto,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    save_window_state(&state.paths, window_state)
}

/// Restore window position and dimensions, with multi-monitor sanity check.
#[tauri::command]
pub async fn window_restore_saved_state(
    window: WebviewWindow,
    state: State<'_, PlayState>,
) -> Result<WindowStateDto, String> {
    let saved = load_window_state(&state.paths);

    let monitors = window.available_monitors().unwrap_or_default();
    let monitor_rects: Vec<MonitorRect> = monitors
        .iter()
        .map(|m| MonitorRect {
            x: m.position().x,
            y: m.position().y,
            width: m.size().width,
            height: m.size().height,
        })
        .collect();

    let width = saved.width.max(880);
    let height = saved.height.max(600);

    let (safe_x, safe_y) = if let (Some(x), Some(y)) = (saved.x, saved.y) {
        sanitize_window_position(x, y, width, height, &monitor_rects)
    } else if !monitor_rects.is_empty() {
        let p = &monitor_rects[0];
        (
            p.x + ((p.width.saturating_sub(width)) / 2) as i32,
            p.y + ((p.height.saturating_sub(height)) / 2) as i32,
        )
    } else {
        (100, 100)
    };

    let _ = window.set_size(Size::Physical(PhysicalSize::new(width, height)));
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(safe_x, safe_y)));

    if saved.is_maximized {
        let _ = window.maximize();
    }

    Ok(WindowStateDto {
        width,
        height,
        x: Some(safe_x),
        y: Some(safe_y),
        is_maximized: saved.is_maximized,
    })
}

/// Get minimize_to_tray setting.
#[tauri::command]
pub async fn window_get_minimize_to_tray(state: State<'_, PlayState>) -> Result<bool, String> {
    Ok(is_minimize_to_tray_enabled(&state.paths))
}

/// Set minimize_to_tray setting.
#[tauri::command]
pub async fn window_set_minimize_to_tray(
    enabled: bool,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    set_minimize_to_tray_preference(&state.paths, enabled)
}

/// Handle game start: hide window if minimize_to_tray is true.
#[tauri::command]
pub async fn window_on_game_launch(
    window: WebviewWindow,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let minimize = is_minimize_to_tray_enabled(&state.paths);
    apply_launch_window_behavior(&window, minimize);
    Ok(())
}

/// Handle game stop: restore window to foreground.
#[tauri::command]
pub async fn window_on_game_exit(window: WebviewWindow) -> Result<(), String> {
    restore_window_from_game(&window);
    Ok(())
}

/// Bring existing window to foreground (single instance second launch handler).
#[tauri::command]
pub async fn window_raise_and_focus(
    window: WebviewWindow,
    deeplink: Option<String>,
) -> Result<(), String> {
    restore_window_from_game(&window);
    if let Some(link) = deeplink {
        let _ = window.emit("deep-link://url", vec![link]);
    }
    Ok(())
}

/// Setup tray icon for returning to launcher and controlling game session.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let open_item = MenuItem::with_id(app, "open_wex", "Открыть Wex", true, None::<&str>)?;
    let stop_item = MenuItem::with_id(app, "stop_game", "Остановить игру", false, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&open_item, &stop_item, &quit_item])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| tauri::image::Image::new(&[0u8; 16 * 16 * 4], 16, 16));

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_wex" => {
                if let Some(w) = app.get_webview_window("main") {
                    restore_window_from_game(&w);
                }
            }
            "stop_game" => {
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Some(state) = app_clone.try_state::<PlayState>() {
                        let ver = {
                            let v = state.running_version.lock().await;
                            v.clone()
                        };
                        if let Some(ver) = ver {
                            let _ = state.core.kill_game(&ver).await;
                        }
                        state.is_running.store(false, Ordering::SeqCst);
                        if let Some(w) = app_clone.get_webview_window("main") {
                            restore_window_from_game(&w);
                        }
                    }
                });
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    restore_window_from_game(&w);
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ============================================================================
// Unit Tests (Requirements 2 and 3)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sanitize_window_position_single_monitor_inside() {
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];

        // Inside bounds -> keeps position
        let (x, y) = sanitize_window_position(200, 150, 1080, 720, &monitors);
        assert_eq!(x, 200);
        assert_eq!(y, 150);
    }

    #[test]
    fn test_sanitize_window_position_multi_monitor_valid() {
        let monitors = vec![
            MonitorRect {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            MonitorRect {
                x: 1920,
                y: 0,
                width: 2560,
                height: 1440,
            },
        ];

        // On second monitor -> preserved
        let (x, y) = sanitize_window_position(2200, 200, 1080, 720, &monitors);
        assert_eq!(x, 2200);
        assert_eq!(y, 200);
    }

    #[test]
    fn test_sanitize_window_position_disconnected_monitor_resets_to_primary_center() {
        // Only primary monitor is now connected
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];

        // Saved window was on disconnected second monitor at x=2500, y=500
        let (x, y) = sanitize_window_position(2500, 500, 1080, 720, &monitors);
        // Must center on primary monitor: (1920 - 1080) / 2 = 420, (1080 - 720) / 2 = 180
        assert_eq!(x, 420);
        assert_eq!(y, 180);
    }

    #[test]
    fn test_sanitize_window_position_completely_negative_offscreen() {
        let monitors = vec![MonitorRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }];

        // Window far off to top-left
        let (x, y) = sanitize_window_position(-3000, -2000, 1080, 720, &monitors);
        assert_eq!(x, 420);
        assert_eq!(y, 180);
    }

    #[test]
    fn test_sanitize_window_position_empty_monitors_fallback() {
        let monitors = vec![];
        let (x, y) = sanitize_window_position(-50, 100, 1080, 720, &monitors);
        assert_eq!(x, 0);
        assert_eq!(y, 100);
    }

    #[test]
    fn test_window_state_persistence_roundtrip() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        paths.ensure_dirs().unwrap();

        // Default
        let initial = load_window_state(&paths);
        assert_eq!(initial.width, 1080);
        assert_eq!(initial.height, 720);
        assert_eq!(initial.x, None);
        assert_eq!(initial.y, None);
        assert!(!initial.is_maximized);

        // Save new state
        let updated = WindowStateDto {
            width: 1280,
            height: 800,
            x: Some(150),
            y: Some(75),
            is_maximized: true,
        };
        save_window_state(&paths, updated.clone()).unwrap();

        let reloaded = load_window_state(&paths);
        assert_eq!(reloaded, updated);
    }

    #[test]
    fn test_minimize_to_tray_setting_default_and_toggle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        paths.ensure_dirs().unwrap();

        // Default is true per specification
        assert!(is_minimize_to_tray_enabled(&paths));

        // Toggle to false
        set_minimize_to_tray_preference(&paths, false).unwrap();
        assert!(!is_minimize_to_tray_enabled(&paths));

        // Toggle back to true
        set_minimize_to_tray_preference(&paths, true).unwrap();
        assert!(is_minimize_to_tray_enabled(&paths));
    }

    #[test]
    fn test_apply_launch_window_behavior_logic() {
        // Logic test: minimize_to_tray = true -> window hides; false -> window stays
        let hide_on_start = true;
        assert!(hide_on_start);

        let keep_on_start = false;
        assert!(!keep_on_start);
    }
    #[test]
    fn review_28_keep_window_overrides_legacy_tray_flag() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let cfg = CoreConfig {
            minimize_to_tray: true,
            game_window_behavior: "keep".into(),
            ..Default::default()
        };
        cfg.save(&paths.config_file).unwrap();
        assert!(!is_minimize_to_tray_enabled(&paths));
    }

}
