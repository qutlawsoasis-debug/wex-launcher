use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::*;

pub const EVT_INSTALL_PROGRESS: &str = "install-progress";
pub const EVT_LAUNCH_STATUS: &str = "launch-status";
pub const EVT_GAME_LOG: &str = "game-log";
pub const EVT_NET_TUNNEL_STATUS: &str = "net-tunnel-status";
pub const EVT_NET_PING_PROGRESS: &str = "net-ping-progress";
pub const EVT_NET_LOBBY_STATUS: &str = "net-lobby-status";
pub const EVT_SOCIAL_ACHIEVEMENT_TOAST: &str = "social-achievement-toast";
pub const EVT_SOCIAL_FRIEND_PRESENCE: &str = "social-friend-presence";
pub const EVT_SOCIAL_LOBBY_INVITE: &str = "social-lobby-invite";
pub const EVT_LAUNCHER_UPDATE_AVAILABLE: &str = "launcher-update-available";

/// Progress payload emitted during package installation or asset verification.
/// Event name: "install-progress"
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LaunchStatusPayload {
    /// "idle" | "preparing" | "installing" | "launching" | "running" | "cancelled" | "error"
    pub state: String,
    pub message: String,
    pub exit_code: Option<i32>,
}

/// Game stdout/stderr log stream line.
/// Event name: "game-log"
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct GameLogPayload {
    pub line: String,
    pub level: String, // "INFO" | "WARN" | "ERROR"
}

/// Tunnel status changes from playit-agent supervisor.
/// Event name: "net-tunnel-status"
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct NetTunnelStatusPayload {
    pub status: TunnelStatusDto,
    pub address: Option<String>,
}

/// Server ping progress or result update.
/// Event name: "net-ping-progress"
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetPingProgressPayload {
    pub host: String,
    pub port: u16,
    pub latency_ms: Option<u64>,
    pub status: String, // "resolving" | "connecting" | "pinging" | "done" | "failed"
}

/// Lobby room state broadcast.
/// Event name: "net-lobby-status"
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyStatusPayload {
    pub code: String,
    pub status: String,
    pub player_count: u32,
    pub tunnel_address: Option<String>,
}

/// Friend presence change notification.
/// Event name: "social-friend-presence"
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialFriendPresencePayload {
    pub user_id: String,
    pub presence: UserPresenceDto,
}

/// Unified tagged enum representing all asynchronous IPC events.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(tag = "event", content = "payload")]
pub enum IpcEvent {
    #[serde(rename = "install-progress")]
    InstallProgress(InstallProgressPayload),
    #[serde(rename = "launch-status")]
    LaunchStatus(LaunchStatusPayload),
    #[serde(rename = "game-log")]
    GameLog(GameLogPayload),
    #[serde(rename = "net-tunnel-status")]
    NetTunnelStatus(NetTunnelStatusPayload),
    #[serde(rename = "net-ping-progress")]
    NetPingProgress(NetPingProgressPayload),
    #[serde(rename = "net-lobby-status")]
    NetLobbyStatus(NetLobbyStatusPayload),
    #[serde(rename = "social-achievement-toast")]
    SocialAchievementToast(AchievementUnlockedEventDto),
    #[serde(rename = "social-friend-presence")]
    SocialFriendPresence(SocialFriendPresencePayload),
    #[serde(rename = "social-lobby-invite")]
    SocialLobbyInvite(LobbyInviteItemDto),
    #[serde(rename = "launcher-update-available")]
    LauncherUpdateAvailable(UpdateCheckResultDto),
}
