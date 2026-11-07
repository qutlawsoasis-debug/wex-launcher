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
    pub ram_gb: u32,
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
    pub appearance: String, // "dark" | "light" | "auto"
    pub accent: String,     // "blue" | "mono" | etc.
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            selected_version: "1.21.1".to_string(),
            player_name: "Player".to_string(),
            ram_gb: 4,
            reduce_motion: false,
            reduce_transparency: false,
            appearance: "dark".to_string(),
            accent: "blue".to_string(),
        }
    }
}

/// Version description for UI selectors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersion {
    pub id: String,
    pub name: String,
    pub is_installed: bool,
}
