//! The main window's size: on the first start it fits the screen (not a small
//! fixed box); after that it opens the way the player left it (size and
//! maximized), kept in `<root>/window.json`.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const FILE: &str = "window.json";
/// The window's minimum (tauri.conf.json minWidth/minHeight).
pub const MIN: (f64, f64) = (880.0, 600.0);
/// A comfortable first size on big screens.
pub const MAX_FIRST: (f64, f64) = (1600.0, 1000.0);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    /// Logical px.
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub maximized: bool,
}

/// The size to open with, in logical px, for a work area of `screen` logical px.
/// A saved size is used when it still fits the screen; otherwise about four
/// fifths of the screen, within [MIN, MAX_FIRST].
pub fn initial(saved: Option<Saved>, screen: (f64, f64)) -> Saved {
    let (sw, sh) = screen;
    if let Some(s) = saved {
        if s.width >= MIN.0 && s.height >= MIN.1 && s.width <= sw && s.height <= sh {
            return s;
        }
        if s.maximized {
            return Saved {
                maximized: true,
                ..first(screen)
            };
        }
    }
    first((sw, sh))
}

fn first((sw, sh): (f64, f64)) -> Saved {
    let w = (sw * 0.8).min(MAX_FIRST.0).max(MIN.0).min(sw.max(MIN.0));
    let h = (sh * 0.82).min(MAX_FIRST.1).max(MIN.1).min(sh.max(MIN.1));
    Saved {
        width: w.round(),
        height: h.round(),
        maximized: false,
    }
}

pub fn load(root: &Path) -> Option<Saved> {
    let text = std::fs::read_to_string(root.join(FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save(root: &Path, s: Saved) {
    if let Ok(text) = serde_json::to_string(&s) {
        let _ = std::fs::write(root.join(FILE), text);
    }
}

/// Applies the size to the main window and keeps it up to date when it closes.
pub fn attach<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>, root: &Path) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let screen = window
        .current_monitor()
        .ok()
        .flatten()
        .map(|m| {
            let s = m.size();
            (s.width as f64 / scale, s.height as f64 / scale)
        })
        .unwrap_or((1920.0, 1080.0));
    // Leave room for the taskbar.
    let screen = (screen.0, (screen.1 - 48.0).max(MIN.1));
    let s = initial(load(root), screen);
    let _ = window.set_size(tauri::LogicalSize::new(s.width, s.height));
    let _ = window.center();
    if s.maximized {
        let _ = window.maximize();
    }
    tracing::info!(
        "window {}x{} (maximized: {})",
        s.width,
        s.height,
        s.maximized
    );

    let root = root.to_path_buf();
    let w = window.clone();
    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
        ) {
            let maximized = w.is_maximized().unwrap_or(false);
            let scale = w.scale_factor().unwrap_or(1.0);
            if let Ok(size) = w.inner_size() {
                let logical = size.to_logical::<f64>(scale);
                let prev = load(&root);
                // While maximized keep the last normal size to come back to.
                let (width, height) = match (maximized, prev) {
                    (true, Some(p)) => (p.width, p.height),
                    _ => (logical.width.round(), logical.height.round()),
                };
                if width >= MIN.0 && height >= MIN.1 {
                    save(
                        &root,
                        Saved {
                            width,
                            height,
                            maximized,
                        },
                    );
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_start_fits_the_screen_not_a_small_box() {
        let s = initial(None, (1920.0, 1032.0));
        assert_eq!((s.width, s.height), (1536.0, 846.0));
        let big = initial(None, (3840.0, 2112.0));
        assert_eq!((big.width, big.height), MAX_FIRST);
        let small = initial(None, (1000.0, 640.0));
        assert_eq!((small.width, small.height), MIN);
    }

    #[test]
    fn a_saved_size_comes_back_when_it_fits() {
        let saved = Saved {
            width: 1200.0,
            height: 800.0,
            maximized: true,
        };
        assert_eq!(initial(Some(saved), (1920.0, 1032.0)), saved);
        // Saved on a bigger screen: fit this one, keep maximized.
        let huge = Saved {
            width: 3000.0,
            height: 1800.0,
            maximized: true,
        };
        let s = initial(Some(huge), (1920.0, 1032.0));
        assert!(s.width <= 1920.0 && s.maximized);
        // Nonsense in the file: the first-start size.
        let tiny = Saved {
            width: 10.0,
            height: 10.0,
            maximized: false,
        };
        assert_eq!(initial(Some(tiny), (1920.0, 1032.0)).width, 1536.0);
    }

    #[test]
    fn saved_to_and_read_from_the_data_root() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(load(tmp.path()).is_none());
        let s = Saved {
            width: 1300.0,
            height: 820.0,
            maximized: false,
        };
        save(tmp.path(), s);
        assert_eq!(load(tmp.path()), Some(s));
    }
}
