//! wex-core library crate stub for launcher functionality.
//! Core networking, launch, and sync implementations are extended by agent a1.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressEvent {
    pub stage: String,
    pub percentage: f32,
    pub detail: String,
    pub speed_bytes_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub selected_version: String,
    pub player_name: String,
    pub ram_gb: u32,
    pub reduce_motion: bool,
    pub reduce_transparency: bool,
    pub appearance: String, // "dark", "light", "auto"
    pub accent: String,     // "blue", "mono", etc.
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
