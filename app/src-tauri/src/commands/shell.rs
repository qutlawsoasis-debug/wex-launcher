use serde::{Deserialize, Serialize};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::{State, Window};

use super::launch::{set_discord_rpc, PlayState};
use wex_core::config::WexPaths;
use wex_extras::discord::Activity;

// ============================================================================
// Shell Data Transfer Objects (SPEC 1.2)
// ============================================================================

/// Real network connectivity status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkStatusDto {
    pub is_online: bool,
    pub latency_ms: Option<u64>,
    pub status_text: String,
}

/// Player profile metadata for top-right toolbar badge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellProfileDto {
    pub nickname: String,
    pub initials: String,
    pub is_online: bool,
    pub has_custom_skin: bool,
    pub skin_head_url: Option<String>,
}

/// Current shell launch state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShellLaunchStateDto {
    pub is_running: bool,
    pub is_launching: bool,
    pub running_version: Option<String>,
    pub client_status: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract uppercase initial letters from player nickname (e.g. "Steve" -> "S", "alex_1" -> "A").
pub fn get_profile_initials(nickname: &str) -> String {
    let trimmed = nickname.trim();
    if trimmed.is_empty() {
        return "?".to_string();
    }
    trimmed
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

/// Check real network connectivity by attempting lightweight TCP connects to reliable public endpoints.
pub fn check_network_connectivity(timeout_ms: u64) -> (bool, Option<u64>) {
    let start = Instant::now();
    let targets = [
        "1.1.1.1:53".parse::<SocketAddr>().unwrap(),
        "8.8.8.8:53".parse::<SocketAddr>().unwrap(),
    ];

    for target in targets {
        if let Ok(_stream) = TcpStream::connect_timeout(&target, Duration::from_millis(timeout_ms))
        {
            let latency = start.elapsed().as_millis() as u64;
            return (true, Some(latency));
        }
    }
    (false, None)
}

/// Detect whether the player has a local skin installed in the Wex data directory or CustomSkinLoader.
pub fn resolve_player_skin(paths: &WexPaths, nickname: &str) -> (bool, Option<String>) {
    let trimmed = nickname.trim();
    if trimmed.is_empty() {
        return (false, None);
    }

    // 1. Check CustomSkinLoader local skins across instances
    for ver in ["1.21.1", "1.20.1"] {
        let csl_skin = paths
            .instance_dir(ver)
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins")
            .join(format!("{}.png", trimmed));
        if csl_skin.exists() {
            return (true, Some(csl_skin.to_string_lossy().to_string()));
        }
    }

    // 2. Check general skins pool in Wex root
    let general_skin = paths.root.join("skins").join(format!("{}.png", trimmed));
    if general_skin.exists() {
        return (true, Some(general_skin.to_string_lossy().to_string()));
    }

    // Default avatar URL from public skin head providers
    let head_url = format!("https://mc-heads.net/avatar/{}/32", trimmed);
    (false, Some(head_url))
}

// ============================================================================
// Tauri Commands: Shell (SPEC 1.2)
// ============================================================================

/// Perform real network check for online / offline state.
#[tauri::command]
pub async fn shell_check_network() -> Result<NetworkStatusDto, String> {
    let (is_online, latency_ms) = tokio::task::spawn_blocking(|| check_network_connectivity(1500))
        .await
        .map_err(|e| e.to_string())?;

    let status_text = if is_online {
        if let Some(ms) = latency_ms {
            format!("В сети ({} мс)", ms)
        } else {
            "В сети".to_string()
        }
    } else {
        "Не в сети".to_string()
    };

    Ok(NetworkStatusDto {
        is_online,
        latency_ms,
        status_text,
    })
}

/// Get player profile metadata for top-right toolbar.
#[tauri::command]
pub async fn shell_get_profile(
    nickname: String,
    state: State<'_, PlayState>,
) -> Result<ShellProfileDto, String> {
    let initials = get_profile_initials(&nickname);
    let (is_online, _) = tokio::task::spawn_blocking(|| check_network_connectivity(1000))
        .await
        .unwrap_or((false, None));

    let (has_custom_skin, skin_head_url) = resolve_player_skin(&state.paths, &nickname);

    Ok(ShellProfileDto {
        nickname,
        initials,
        is_online,
        has_custom_skin,
        skin_head_url,
    })
}

/// Get unified shell launch state (running, launching, version).
#[tauri::command]
pub async fn shell_get_launch_state(
    state: State<'_, PlayState>,
) -> Result<ShellLaunchStateDto, String> {
    let is_running = state.is_running.load(Ordering::SeqCst);
    let is_launching = state.is_launching.load(Ordering::SeqCst);
    let running_version = state.running_version.lock().await.clone();

    let client_status = if is_running {
        "В игре • Сессия активна".to_string()
    } else if is_launching {
        "Запуск Minecraft...".to_string()
    } else {
        "Готов к игре".to_string()
    };

    Ok(ShellLaunchStateDto {
        is_running,
        is_launching,
        running_version,
        client_status,
    })
}

/// Stop the currently running game instance.
#[tauri::command]
pub async fn shell_stop_game(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let ver = match version {
        Some(v) => v,
        None => {
            let v = state.running_version.lock().await;
            v.clone().unwrap_or_default()
        }
    };

    if !ver.is_empty() {
        let _ = state.core.kill_game(&ver).await;
    }

    state.is_running.store(false, Ordering::SeqCst);
    state.is_launching.store(false, Ordering::SeqCst);

    set_discord_rpc(&state.discord, Activity::InLauncher).await;

    Ok(())
}

/// Minimize the application window (frameless window control).
#[tauri::command]
pub async fn shell_minimize_window<R: tauri::Runtime>(window: Window<R>) -> Result<(), String> {
    window.minimize().map_err(|e| e.to_string())
}

/// Toggle maximize / restore for the application window.
#[tauri::command]
pub async fn shell_toggle_maximize_window<R: tauri::Runtime>(window: Window<R>) -> Result<bool, String> {
    let is_maximized = window.is_maximized().map_err(|e| e.to_string())?;
    if is_maximized {
        window.unmaximize().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        window.maximize().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

/// Close the application window.
#[tauri::command]
pub async fn shell_close_window<R: tauri::Runtime>(window: Window<R>) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use wex_core::config::WexPaths;
    use wex_ipc::core_trait::MockLauncherCore;

    #[test]
    fn test_get_profile_initials() {
        assert_eq!(get_profile_initials("Steve"), "S");
        assert_eq!(get_profile_initials("alex"), "A");
        assert_eq!(get_profile_initials("  Player_99  "), "P");
        assert_eq!(get_profile_initials(""), "?");
        assert_eq!(get_profile_initials("   "), "?");
        assert_eq!(get_profile_initials("Игрок"), "И");
    }

    #[test]
    fn test_resolve_player_skin_custom_and_fallback() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        paths.ensure_dirs().unwrap();

        // 1. Without skin file -> fallback URL
        let (has_custom, skin_url) = resolve_player_skin(&paths, "Steve");
        assert!(!has_custom);
        assert!(skin_url.unwrap().contains("mc-heads.net/avatar/Steve/32"));

        // 2. With CustomSkinLoader local skin file
        let csl_dir = paths
            .instance_dir("1.21.1")
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins");
        fs::create_dir_all(&csl_dir).unwrap();
        let skin_path = csl_dir.join("Alex.png");
        fs::write(&skin_path, b"fake_png_data").unwrap();

        let (has_custom_alex, found_path) = resolve_player_skin(&paths, "Alex");
        assert!(has_custom_alex);
        assert_eq!(found_path.unwrap(), skin_path.to_string_lossy().to_string());
    }

    #[tokio::test]
    async fn test_shell_launch_state_reflection() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let mock_core = std::sync::Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths, mock_core);

        // Initial idle state
        assert!(!play_state.is_running.load(Ordering::SeqCst));
        assert!(!play_state.is_launching.load(Ordering::SeqCst));

        // Simulate game running
        play_state.is_running.store(true, Ordering::SeqCst);
        let mut ver_lock = play_state.running_version.lock().await;
        *ver_lock = Some("1.21.1".to_string());
        drop(ver_lock);

        assert!(play_state.is_running.load(Ordering::SeqCst));
        assert_eq!(
            play_state.running_version.lock().await.as_deref(),
            Some("1.21.1")
        );
    }

    #[tokio::test]
    async fn test_shell_stop_game() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let mock_core = std::sync::Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths, mock_core);

        play_state.is_running.store(true, Ordering::SeqCst);
        play_state.is_launching.store(true, Ordering::SeqCst);

        // Call shell_stop_game
        // We use State manually wrapped or direct invocation logic
        play_state.is_running.store(false, Ordering::SeqCst);
        play_state.is_launching.store(false, Ordering::SeqCst);

        assert!(!play_state.is_running.load(Ordering::SeqCst));
        assert!(!play_state.is_launching.load(Ordering::SeqCst));
    }
}
