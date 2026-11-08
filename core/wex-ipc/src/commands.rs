use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::*;

// ============================================================================
// Core Commands
// ============================================================================

pub const CMD_CORE_GET_CONFIG: &str = "core_get_config";
pub const CMD_CORE_SAVE_CONFIG: &str = "core_save_config";
pub const CMD_CORE_GET_INSTALLED_VERSIONS: &str = "core_get_installed_versions";
pub const CMD_CORE_INSTALL_INSTANCE: &str = "core_install_instance";
pub const CMD_CORE_LAUNCH_GAME: &str = "core_launch_game";
pub const CMD_CORE_CANCEL_LAUNCH: &str = "core_cancel_launch";
pub const CMD_CORE_IS_RUNNING: &str = "core_is_running";
pub const CMD_CORE_KILL_GAME: &str = "core_kill_game";

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct InstallInstanceRequest {
    pub version: String,
    pub force_reinstall: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct InstallInstanceResponse {
    pub success: bool,
    pub version: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LaunchGameRequest {
    pub version: String,
    pub nickname: String,
    pub jvm_args: Option<Vec<String>>,
    #[serde(default)]
    pub quick_play_multiplayer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LaunchGameResponse {
    pub success: bool,
    pub pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct IsRunningRequest {
    pub instance_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct IsRunningResponse {
    pub is_running: bool,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct KillGameRequest {
    pub instance_id: String,
}

// ============================================================================
// Net Commands
// ============================================================================

pub const CMD_NET_PING_SERVER: &str = "net_ping_server";
pub const CMD_NET_RESOLVE_ADDRESS: &str = "net_resolve_address";
pub const CMD_NET_LOAD_SERVERS: &str = "net_load_servers";
pub const CMD_NET_SAVE_SERVERS: &str = "net_save_servers";
pub const CMD_NET_MERGE_SERVERS: &str = "net_merge_servers";
pub const CMD_NET_LOBBY_CREATE: &str = "net_lobby_create";
pub const CMD_NET_LOBBY_JOIN: &str = "net_lobby_join";
pub const CMD_NET_LOBBY_OPEN: &str = "net_lobby_open";
pub const CMD_NET_LOBBY_STATUS: &str = "net_lobby_status";
pub const CMD_NET_LOBBY_HEARTBEAT: &str = "net_lobby_heartbeat";
pub const CMD_NET_LOBBY_CLOSE: &str = "net_lobby_close";
pub const CMD_NET_LOBBY_LEAVE: &str = "net_lobby_leave";
pub const CMD_NET_LOBBY_KICK: &str = "net_lobby_kick";
pub const CMD_NET_LOBBY_TUNNEL_CONFIG: &str = "net_lobby_tunnel_config";
pub const CMD_NET_LOBBY_LIST_OPEN_ROOMS: &str = "net_lobby_list_open_rooms";
pub const CMD_NET_LAN_LIST_DISCOVERED: &str = "net_lan_list_discovered";
pub const CMD_NET_LAN_CONNECT: &str = "net_lan_connect";
pub const CMD_NET_COMPARE_MANIFESTS: &str = "net_compare_manifests";
pub const CMD_NET_COMPUTE_MANIFEST_HASH: &str = "net_compute_manifest_hash";
pub const CMD_NET_TUNNEL_START: &str = "net_tunnel_start";
pub const CMD_NET_TUNNEL_STOP: &str = "net_tunnel_stop";
pub const CMD_NET_TUNNEL_STATUS: &str = "net_tunnel_status";
pub const CMD_NET_TUNNEL_ENSURE_BINARY: &str = "net_tunnel_ensure_binary";

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetPingRequest {
    pub address: String,
    pub port: Option<u16>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetResolveRequest {
    pub host: String,
    pub default_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetResolveResponse {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLoadServersRequest {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLoadServersResponse {
    pub servers: Vec<PackServerEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetSaveServersRequest {
    pub path: String,
    pub servers: Vec<PackServerEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetMergeServersRequest {
    pub user_servers_path: String,
    pub pack_servers: Vec<PackServerEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetMergeServersResponse {
    pub merged_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyCreateRequest {
    pub host_name: String,
    pub manifest: Option<Vec<ModManifestEntryDto>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyJoinRequest {
    pub code: String,
    pub player_name: String,
    pub manifest: Option<Vec<ModManifestEntryDto>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyOpenRequest {
    pub code: String,
    pub host_token: String,
    pub tunnel_address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyStatusRequest {
    pub code: String,
    pub player_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyHeartbeatRequest {
    pub code: String,
    pub host_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyCloseRequest {
    pub code: String,
    pub host_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyLeaveRequest {
    pub code: String,
    pub player_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLobbyKickRequest {
    pub code: String,
    pub host_token: String,
    pub player: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetListOpenRoomsRequest {
    pub player_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetListOpenRoomsResponse {
    pub rooms: Vec<NetOpenRoomDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLanListDiscoveredRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLanListDiscoveredResponse {
    pub worlds: Vec<LanWorldDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLanConnectRequest {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetLanConnectResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetCompareManifestsRequest {
    pub client_manifest: Vec<ModManifestEntryDto>,
    pub host_manifest: Vec<ModManifestEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetCompareManifestsResponse {
    pub mismatches: Vec<ModMismatchItemDto>,
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetComputeManifestHashRequest {
    pub manifest: Vec<ModManifestEntryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetComputeManifestHashResponse {
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetTunnelStartRequest {
    pub binary_path: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetTunnelStartResponse {
    pub pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetTunnelEnsureBinaryRequest {
    pub destination_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct NetTunnelEnsureBinaryResponse {
    pub path: String,
    pub verified: bool,
}

// ============================================================================
// Extras Commands
// ============================================================================

pub const CMD_EXTRAS_ANALYZE_CRASH: &str = "extras_analyze_crash";
pub const CMD_EXTRAS_VALIDATE_SKIN: &str = "extras_validate_skin";
pub const CMD_EXTRAS_CONVERT_SKIN: &str = "extras_convert_skin";
pub const CMD_EXTRAS_SAVE_SKIN: &str = "extras_save_skin";
pub const CMD_EXTRAS_LIST_SKIN_HISTORY: &str = "extras_list_skin_history";
pub const CMD_EXTRAS_GENERATE_CSL_CONFIG: &str = "extras_generate_csl_config";
pub const CMD_EXTRAS_DISCORD_SET_ACTIVITY: &str = "extras_discord_set_activity";
pub const CMD_EXTRAS_DISCORD_CLEAR_ACTIVITY: &str = "extras_discord_clear_activity";
pub const CMD_EXTRAS_IMPORT_OLD_LAUNCHER: &str = "extras_import_old_launcher";
pub const CMD_EXTRAS_GET_PRESETS: &str = "extras_get_presets";
pub const CMD_EXTRAS_APPLY_PRESET: &str = "extras_apply_preset";
pub const CMD_WARDROBE_GET_STATE: &str = "wardrobe_get_state";
pub const CMD_WARDROBE_APPLY: &str = "wardrobe_apply";
pub const CMD_WARDROBE_DELETE_HISTORY: &str = "wardrobe_delete_history";

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasAnalyzeCrashRequest {
    pub exit_code: Option<i32>,
    pub log_text: Option<String>,
    pub crash_report_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasValidateSkinRequest {
    pub skin_bytes_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasConvertSkinRequest {
    pub skin_bytes_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasConvertSkinResponse {
    pub converted_bytes_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasSaveSkinRequest {
    pub history_dir: String,
    pub skin_name: String,
    pub skin_bytes_base64: String,
    pub model: SkinModelDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasListSkinHistoryRequest {
    pub history_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasListSkinHistoryResponse {
    pub items: Vec<WardrobeItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasGenerateCslConfigRequest {
    pub output_path: String,
    pub csl_api_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct WardrobeApplyRequest {
    pub nickname: String,
    pub skin_name: String,
    pub skin_base64: Option<String>,
    pub model: SkinModelDto,
    pub cape_id: String,
    pub cape_name: String,
    pub cape_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct WardrobeApplyResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct WardrobeDeleteHistoryRequest {
    pub history_dir: String,
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct WardrobeDeleteHistoryResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasDiscordSetActivityRequest {
    pub activity: DiscordActivityDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasImportOldLauncherRequest {
    pub old_launcher_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct ExtrasGetPresetsResponse {
    pub presets: Vec<GraphicsPresetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ExtrasApplyPresetRequest {
    pub options_txt_path: String,
    pub shaderpacks_dir: String,
    pub tier: PresetTierDto,
}

// ============================================================================
// Social Commands
// ============================================================================

pub const CMD_SOCIAL_LOGIN: &str = "social_login";
pub const CMD_SOCIAL_GET_FRIENDS: &str = "social_get_friends";
pub const CMD_SOCIAL_SEND_FRIEND_REQUEST: &str = "social_send_friend_request";
pub const CMD_SOCIAL_ACCEPT_FRIEND_REQUEST: &str = "social_accept_friend_request";
pub const CMD_SOCIAL_DECLINE_FRIEND_REQUEST: &str = "social_decline_friend_request";
pub const CMD_SOCIAL_SYNC: &str = "social_sync";
pub const CMD_SOCIAL_SEND_LOBBY_INVITE: &str = "social_send_lobby_invite";
pub const CMD_SOCIAL_SET_PRESENCE: &str = "social_set_presence";
pub const CMD_SOCIAL_LIST_ACHIEVEMENTS: &str = "social_list_achievements";
pub const CMD_SOCIAL_REPORT_ACHIEVEMENT_PROGRESS: &str = "social_report_achievement_progress";
pub const CMD_SOCIAL_LIST_WORLDS: &str = "social_list_worlds";
pub const CMD_SOCIAL_CREATE_WORLD_BACKUP: &str = "social_create_world_backup";
pub const CMD_SOCIAL_LIST_WORLD_BACKUPS: &str = "social_list_world_backups";
pub const CMD_SOCIAL_RESTORE_WORLD_BACKUP: &str = "social_restore_world_backup";
pub const CMD_SOCIAL_LIST_SCREENSHOTS: &str = "social_list_screenshots";
pub const CMD_SOCIAL_GET_NEWS: &str = "social_get_news";
pub const CMD_SOCIAL_CHECK_LAUNCHER_UPDATE: &str = "social_check_launcher_update";

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialLoginRequest {
    pub auth: UserAuthDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialGetFriendsRequest {
    pub auth: UserAuthDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialGetFriendsResponse {
    pub friends: Vec<FriendItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialSendFriendRequestRequest {
    pub auth: UserAuthDto,
    pub target_friend_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialAcceptFriendRequestRequest {
    pub auth: UserAuthDto,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialDeclineFriendRequestRequest {
    pub auth: UserAuthDto,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialSyncRequest {
    pub auth: UserAuthDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialSendLobbyInviteRequest {
    pub auth: UserAuthDto,
    pub target_user_id: String,
    pub lobby_code: String,
    pub host_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialSetPresenceRequest {
    pub auth: UserAuthDto,
    pub presence: UserPresenceDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListAchievementsRequest {
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct SocialListAchievementsResponse {
    pub achievements: Vec<AchievementDefinitionDto>,
    pub progress: Vec<AchievementProgressDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialReportAchievementProgressRequest {
    pub data_dir: String,
    pub achievement_id: String,
    pub increment: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct SocialReportAchievementProgressResponse {
    pub unlocked: bool,
    pub current: f64,
    pub target: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListWorldsRequest {
    pub game_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListWorldsResponse {
    pub worlds: Vec<WorldItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialCreateWorldBackupRequest {
    pub world_dir: String,
    pub backups_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListWorldBackupsRequest {
    pub backups_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListWorldBackupsResponse {
    pub backups: Vec<WorldBackupItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialRestoreWorldBackupRequest {
    pub backup_path: String,
    pub saves_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialRestoreWorldBackupResponse {
    pub restored_world_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListScreenshotsRequest {
    pub screenshots_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialListScreenshotsResponse {
    pub screenshots: Vec<ScreenshotItemDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialGetNewsRequest {
    pub cache_dir: String,
    pub remote_url: Option<String>,
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialGetNewsResponse {
    pub versions: Vec<ReleaseNoteVersionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SocialCheckLauncherUpdateRequest {
    pub manifest_url: String,
    pub current_version: String,
    pub target_platform: Option<String>,
}

// ============================================================================
// Music Commands
// ============================================================================

pub const CMD_MUSIC_GET_PLAYLIST: &str = "music_get_playlist";
pub const CMD_MUSIC_GET_TRACK_DATA: &str = "music_get_track_data";
pub const CMD_MUSIC_OPEN_FOLDER: &str = "music_open_folder";
pub const CMD_MUSIC_GET_SETTINGS: &str = "music_get_settings";
pub const CMD_MUSIC_SET_SETTINGS: &str = "music_set_settings";
pub const CMD_MUSIC_SYNC_SMTC: &str = "music_sync_smtc";
pub const CMD_MUSIC_GET_SMTC_STATE: &str = "music_get_smtc_state";

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct MusicTrackDto {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub file_name: String,
    pub cover_data_url: Option<String>,
    pub duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct MusicPlaylistResponse {
    pub tracks: Vec<MusicTrackDto>,
    pub music_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct MusicGetTrackDataRequest {
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct MusicGetTrackDataResponse {
    pub file_name: String,
    pub mime_type: String,
    pub audio_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct MusicSettingsDto {
    pub enabled: bool,
    pub volume: u8,
    pub auto_pause_on_launch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct MusicSmtcStateDto {
    pub is_playing: bool,
    pub track_title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub has_cover: bool,
    pub position_seconds: u64,
    pub duration_seconds: Option<u64>,
}
