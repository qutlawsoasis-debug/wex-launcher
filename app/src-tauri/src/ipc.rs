use serde::{Deserialize, Serialize};

/// Progress payload emitted during package installation or asset verification.
/// Event name: "install-progress"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgressPayload {
    pub stage: String,
    pub percentage: f32,
    pub detail: String,
    pub speed_bytes_per_sec: u64,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
}

/// Status payload emitted when game launch state changes.
/// Event name: "launch-status"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchStatusPayload {
    pub state: String, // "idle" | "preparing" | "installing" | "launching" | "running" | "cancelled" | "error"
    pub message: String,
    pub exit_code: Option<i32>,
}

/// Launcher configuration stored in user app data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub selected_version: String,
    pub player_name: String,
    pub ram_mb: u32,
    pub game_dir: String,
    pub graphics_preset: String, // "Low" | "Medium" | "High" | "Ultra"
    pub appearance: String,      // "dark" | "light" | "auto"
    pub accent: String, // "blue" | "purple" | "pink" | "orange" | "green" | "mint" | "graphite"
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            selected_version: "1.21.1".to_string(),
            player_name: "Player".to_string(),
            ram_mb: 4096,
            game_dir: default_game_dir(),
            graphics_preset: "High".to_string(),
            appearance: "dark".to_string(),
            accent: "blue".to_string(),
            reduce_motion: false,
            reduce_transparency: false,
        }
    }
}

fn default_game_dir() -> String {
    if let Ok(appdata) = std::env::var("APPDATA") {
        format!("{appdata}\\Wex")
    } else {
        "./Wex".to_string()
    }
}

/// System telemetry and memory limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub total_memory_mb: u32,
    pub os_name: String,
    pub launcher_version: String,
    pub data_dir: String,
}

/// Version description for UI selectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub id: String,
    pub name: String,
    pub is_installed: bool,
    pub required_java: u32, // 17 for 1.20.1, 21 for 1.21.1
}

/// Server entry shown in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntryModel {
    pub id: String,
    pub name: String,
    pub address: String,
    pub icon: Option<String>,
    pub ping_ms: Option<u64>,
    pub online: Option<u32>,
    pub max_players: Option<u32>,
    pub motd: Option<String>,
    pub version: Option<String>,
}

/// Result of querying a Minecraft server via SLP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPingResult {
    pub latency_ms: u64,
    pub online: u32,
    pub max_players: u32,
    pub motd: String,
    pub version: String,
    pub favicon: Option<String>,
}

/// Player in a lobby session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayerModel {
    pub name: String,
    pub is_host: bool,
    pub ping_ms: u64,
    pub mods_synced: bool,
}

/// Lobby room information for UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyRoomModel {
    pub code: String,
    pub host_name: String,
    pub world_name: String,
    pub tunnel_address: Option<String>,
    pub tunnel_active: bool,
    pub is_host: bool,
    pub players: Vec<LobbyPlayerModel>,
    pub mod_mismatch_count: usize,
    pub total_mods: usize,
}
