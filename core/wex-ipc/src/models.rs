use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

// ============================================================================
// Core / Launcher Models
// ============================================================================

/// Launcher configuration stored in user app data.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct InstalledVersion {
    pub id: String,
    pub name: String,
    pub is_installed: bool,
}

// ============================================================================
// Network (wex-net) Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct PlayerSampleDto {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ServerPingResponseDto {
    pub host: String,
    pub port: u16,
    pub latency_ms: u64,
    pub version_name: String,
    pub protocol: i32,
    pub players_online: u32,
    pub players_max: u32,
    pub sample_players: Vec<PlayerSampleDto>,
    pub description: String,
    pub favicon: Option<String>,
}

impl From<wex_net::ServerPingResponse> for ServerPingResponseDto {
    fn from(r: wex_net::ServerPingResponse) -> Self {
        Self {
            host: r.host,
            port: r.port,
            latency_ms: r.latency_ms,
            version_name: r.version.name,
            protocol: r.version.protocol,
            players_online: r.players.online,
            players_max: r.players.max,
            sample_players: r
                .players
                .sample
                .unwrap_or_default()
                .into_iter()
                .map(|p| PlayerSampleDto {
                    name: p.name,
                    id: p.id,
                })
                .collect(),
            description: r.description,
            favicon: r.favicon,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct PackServerEntryDto {
    pub id: Option<String>,
    pub name: String,
    pub ip: String,
    pub icon: Option<String>,
}

impl From<wex_net::PackServerEntry> for PackServerEntryDto {
    fn from(s: wex_net::PackServerEntry) -> Self {
        Self {
            id: s.id,
            name: s.name,
            ip: s.ip,
            icon: s.icon,
        }
    }
}

impl From<PackServerEntryDto> for wex_net::PackServerEntry {
    fn from(s: PackServerEntryDto) -> Self {
        Self {
            id: s.id,
            name: s.name,
            ip: s.ip,
            icon: s.icon,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LobbyCreateResponseDto {
    pub code: String,
    pub host_token: String,
    pub status: String,
    pub created_at: i64,
}

impl From<wex_net::lobby::LobbyCreateResponse> for LobbyCreateResponseDto {
    fn from(r: wex_net::lobby::LobbyCreateResponse) -> Self {
        Self {
            code: r.code,
            host_token: r.host_token,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LobbyJoinResponseDto {
    pub success: bool,
    pub code: String,
    pub status: String,
    pub tunnel_address: Option<String>,
    pub player_count: u32,
}

impl From<wex_net::lobby::LobbyJoinResponse> for LobbyJoinResponseDto {
    fn from(r: wex_net::lobby::LobbyJoinResponse) -> Self {
        Self {
            success: r.success,
            code: r.code,
            status: r.status,
            tunnel_address: r.tunnel_address,
            player_count: r.player_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LobbyStatusResponseDto {
    pub code: String,
    pub status: String,
    pub tunnel_address: Option<String>,
    pub player_count: u32,
    pub last_heartbeat: i64,
    pub players: Option<Vec<String>>,
    pub host_name: Option<String>,
    pub kicked: bool,
}

impl From<wex_net::lobby::LobbyStatusResponse> for LobbyStatusResponseDto {
    fn from(r: wex_net::lobby::LobbyStatusResponse) -> Self {
        Self {
            code: r.code,
            status: r.status,
            tunnel_address: r.tunnel_address,
            player_count: r.player_count,
            last_heartbeat: r.last_heartbeat,
            players: r.players,
            host_name: r.host_name,
            kicked: r.kicked,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct TunnelConfigResponseDto {
    pub secret: Option<String>,
    pub public_address: Option<String>,
    pub public_port: Option<u16>,
}

impl From<wex_net::lobby::TunnelConfigResponse> for TunnelConfigResponseDto {
    fn from(r: wex_net::lobby::TunnelConfigResponse) -> Self {
        Self {
            secret: r.secret,
            public_address: r.public_address,
            public_port: r.public_port,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ModManifestEntryDto {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

impl From<wex_net::ModManifestEntry> for ModManifestEntryDto {
    fn from(m: wex_net::ModManifestEntry) -> Self {
        Self {
            id: m.id,
            name: m.name,
            version: m.version,
            enabled: m.enabled,
        }
    }
}

impl From<ModManifestEntryDto> for wex_net::ModManifestEntry {
    fn from(m: ModManifestEntryDto) -> Self {
        Self {
            id: m.id,
            name: m.name,
            version: m.version,
            enabled: m.enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub enum ModMismatchTypeDto {
    Disabled,
    Missing,
    Extra,
    Version {
        player_version: String,
        host_version: String,
    },
}

impl From<wex_net::ModMismatchType> for ModMismatchTypeDto {
    fn from(t: wex_net::ModMismatchType) -> Self {
        match t {
            wex_net::ModMismatchType::Disabled => Self::Disabled,
            wex_net::ModMismatchType::Missing => Self::Missing,
            wex_net::ModMismatchType::Extra => Self::Extra,
            wex_net::ModMismatchType::Version {
                player_version,
                host_version,
            } => Self::Version {
                player_version,
                host_version,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ModMismatchItemDto {
    pub mod_id: String,
    pub mod_name: String,
    pub mismatch_type: ModMismatchTypeDto,
    pub type_ru: String,
    pub is_fixable: bool,
    pub player_version: Option<String>,
    pub host_version: Option<String>,
    pub display_description: String,
}

impl From<wex_net::ModMismatchItem> for ModMismatchItemDto {
    fn from(i: wex_net::ModMismatchItem) -> Self {
        Self {
            mod_id: i.mod_id,
            mod_name: i.mod_name,
            mismatch_type: i.mismatch_type.into(),
            type_ru: i.type_ru,
            is_fixable: i.is_fixable,
            player_version: i.player_version,
            host_version: i.host_version,
            display_description: i.display_description,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub enum TunnelStatusDto {
    Inactive,
    Downloading { progress: f32 },
    Starting,
    Running { address: String, port: u16 },
    Stopped,
    Error(String),
}

impl From<wex_net::TunnelStatus> for TunnelStatusDto {
    fn from(s: wex_net::TunnelStatus) -> Self {
        match s {
            wex_net::TunnelStatus::Inactive => Self::Inactive,
            wex_net::TunnelStatus::Downloading { progress } => Self::Downloading { progress },
            wex_net::TunnelStatus::Starting => Self::Starting,
            wex_net::TunnelStatus::Running { address, port } => Self::Running { address, port },
            wex_net::TunnelStatus::Stopped => Self::Stopped,
            wex_net::TunnelStatus::Error(e) => Self::Error(e),
        }
    }
}

// ============================================================================
// Extras (wex-extras) Models
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum CrashKindDto {
    OutOfMemory,
    MissingFabricDependency {
        mod_name: String,
        required_dep: String,
    },
    IncompatibleMod {
        mod_name: String,
    },
    MixinConflict {
        mod_names: Vec<String>,
        target_class: Option<String>,
    },
    InvalidJavaVersion {
        required_java: String,
        current_java: Option<String>,
    },
    CorruptedFile {
        details: String,
    },
    GraphicsDriverOrShader {
        details: String,
    },
    Unknown {
        exit_code: Option<i32>,
    },
}

impl From<wex_extras::CrashKind> for CrashKindDto {
    fn from(k: wex_extras::CrashKind) -> Self {
        match k {
            wex_extras::CrashKind::OutOfMemory => Self::OutOfMemory,
            wex_extras::CrashKind::MissingFabricDependency {
                mod_name,
                required_dep,
            } => Self::MissingFabricDependency {
                mod_name,
                required_dep,
            },
            wex_extras::CrashKind::IncompatibleMod { mod_name } => {
                Self::IncompatibleMod { mod_name }
            }
            wex_extras::CrashKind::MixinConflict {
                mod_names,
                target_class,
            } => Self::MixinConflict {
                mod_names,
                target_class,
            },
            wex_extras::CrashKind::InvalidJavaVersion {
                required_java,
                current_java,
            } => Self::InvalidJavaVersion {
                required_java,
                current_java,
            },
            wex_extras::CrashKind::CorruptedFile { details } => Self::CorruptedFile { details },
            wex_extras::CrashKind::GraphicsDriverOrShader { details } => {
                Self::GraphicsDriverOrShader { details }
            }
            wex_extras::CrashKind::Unknown { exit_code } => Self::Unknown { exit_code },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CrashDiagnosisDto {
    pub kind: CrashKindDto,
    pub title: String,
    pub user_advice: String,
    pub technical_summary: String,
}

impl From<wex_extras::CrashDiagnosis> for CrashDiagnosisDto {
    fn from(d: wex_extras::CrashDiagnosis) -> Self {
        Self {
            kind: d.kind.into(),
            title: d.title,
            user_advice: d.user_advice,
            technical_summary: d.technical_summary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum SkinModelDto {
    Classic,
    Slim,
}

impl From<wex_extras::SkinModel> for SkinModelDto {
    fn from(m: wex_extras::SkinModel) -> Self {
        match m {
            wex_extras::SkinModel::Classic => Self::Classic,
            wex_extras::SkinModel::Slim => Self::Slim,
        }
    }
}

impl From<SkinModelDto> for wex_extras::SkinModel {
    fn from(m: SkinModelDto) -> Self {
        match m {
            SkinModelDto::Classic => Self::Classic,
            SkinModelDto::Slim => Self::Slim,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SkinValidationResultDto {
    pub is_valid: bool,
    pub error: Option<String>,
    pub width: u32,
    pub height: u32,
}

impl From<wex_extras::SkinValidationResult> for SkinValidationResultDto {
    fn from(v: wex_extras::SkinValidationResult) -> Self {
        Self {
            is_valid: v.is_valid,
            error: v.error,
            width: v.width,
            height: v.height,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WardrobeItemDto {
    pub id: String,
    pub item_type: String,
    pub name: String,
    pub hash: String,
    pub model: String,
    pub relative_path: String,
    pub created_utc: String,
}

impl From<wex_extras::WardrobeItem> for WardrobeItemDto {
    fn from(w: wex_extras::WardrobeItem) -> Self {
        Self {
            id: w.id,
            item_type: w.item_type,
            name: w.name,
            hash: w.hash,
            model: w.model,
            relative_path: w.relative_path,
            created_utc: w.created_utc,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub enum DiscordActivityDto {
    #[default]
    InLauncher,
    Playing {
        version: String,
        world_name: Option<String>,
        mod_count: Option<usize>,
        start_timestamp: Option<i64>,
    },
    InLobby {
        lobby_code: Option<String>,
        player_count: usize,
        host_name: Option<String>,
    },
}

impl From<wex_extras::Activity> for DiscordActivityDto {
    fn from(a: wex_extras::Activity) -> Self {
        match a {
            wex_extras::Activity::InLauncher => Self::InLauncher,
            wex_extras::Activity::Playing {
                version,
                world_name,
                mod_count,
                start_timestamp,
            } => Self::Playing {
                version,
                world_name,
                mod_count,
                start_timestamp,
            },
            wex_extras::Activity::InLobby {
                lobby_code,
                player_count,
                host_name,
            } => Self::InLobby {
                lobby_code,
                player_count,
                host_name,
            },
        }
    }
}

impl From<DiscordActivityDto> for wex_extras::Activity {
    fn from(a: DiscordActivityDto) -> Self {
        match a {
            DiscordActivityDto::InLauncher => Self::InLauncher,
            DiscordActivityDto::Playing {
                version,
                world_name,
                mod_count,
                start_timestamp,
            } => Self::Playing {
                version,
                world_name,
                mod_count,
                start_timestamp,
            },
            DiscordActivityDto::InLobby {
                lobby_code,
                player_count,
                host_name,
            } => Self::InLobby {
                lobby_code,
                player_count,
                host_name,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ImportedServerDto {
    pub name: String,
    pub ip: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ImportedSkinDto {
    pub file_name: String,
    pub path: String,
    pub hash: String,
    pub is_slim: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ImportedCapeDto {
    pub file_name: String,
    pub path: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ImportedWardrobeItemDto {
    pub id: String,
    pub item_type: String,
    pub name: String,
    pub hash: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct OldLauncherDataDto {
    pub nickname: String,
    pub ram_mb: u32,
    pub selected_version: String,
    pub graphics_preset: Option<String>,
    pub total_play_time_seconds: u64,
    pub total_game_launches: u32,
    pub servers: Vec<ImportedServerDto>,
    pub skins: Vec<ImportedSkinDto>,
    pub capes: Vec<ImportedCapeDto>,
    pub wardrobe_history: Vec<ImportedWardrobeItemDto>,
}

impl From<wex_extras::OldLauncherData> for OldLauncherDataDto {
    fn from(d: wex_extras::OldLauncherData) -> Self {
        Self {
            nickname: d.nickname,
            ram_mb: d.ram_mb,
            selected_version: d.selected_version,
            graphics_preset: d.graphics_preset,
            total_play_time_seconds: d.total_play_time_seconds,
            total_game_launches: d.total_game_launches,
            servers: d
                .servers
                .into_iter()
                .map(|s| ImportedServerDto {
                    name: s.name,
                    ip: s.ip,
                    icon: s.icon,
                })
                .collect(),
            skins: d
                .skins
                .into_iter()
                .map(|s| ImportedSkinDto {
                    file_name: s.file_name,
                    path: s.path.to_string_lossy().to_string(),
                    hash: s.hash,
                    is_slim: s.is_slim,
                })
                .collect(),
            capes: d
                .capes
                .into_iter()
                .map(|c| ImportedCapeDto {
                    file_name: c.file_name,
                    path: c.path.to_string_lossy().to_string(),
                    hash: c.hash,
                })
                .collect(),
            wardrobe_history: d
                .wardrobe_history
                .into_iter()
                .map(|w| ImportedWardrobeItemDto {
                    id: w.id,
                    item_type: w.item_type,
                    name: w.name,
                    hash: w.hash,
                    model: w.model,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
pub enum PresetTierDto {
    Low,
    Medium,
    High,
    Ultra,
}

impl From<wex_extras::PresetTier> for PresetTierDto {
    fn from(t: wex_extras::PresetTier) -> Self {
        match t {
            wex_extras::PresetTier::Low => Self::Low,
            wex_extras::PresetTier::Medium => Self::Medium,
            wex_extras::PresetTier::High => Self::High,
            wex_extras::PresetTier::Ultra => Self::Ultra,
        }
    }
}

impl From<PresetTierDto> for wex_extras::PresetTier {
    fn from(t: PresetTierDto) -> Self {
        match t {
            PresetTierDto::Low => Self::Low,
            PresetTierDto::Medium => Self::Medium,
            PresetTierDto::High => Self::High,
            PresetTierDto::Ultra => Self::Ultra,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct GraphicsPresetDto {
    pub key: String,
    pub name: String,
    pub hardware: String,
    pub render_distance: u32,
    pub simulation_distance: u32,
    pub graphics_mode: String,
    pub particles: String,
    pub clouds: String,
    pub entity_distance_scaling: f64,
    pub biome_blend_radius: u32,
    pub mipmap_levels: u32,
    pub ao: u32,
    pub ram_mb: u32,
    pub jvm_args: Vec<String>,
    pub shader_pack: String,
    pub shader_packs_fallback: Vec<String>,
    pub shader_options: HashMap<String, String>,
    pub resource_packs: Vec<String>,
}

impl From<wex_extras::GraphicsPreset> for GraphicsPresetDto {
    fn from(p: wex_extras::GraphicsPreset) -> Self {
        Self {
            key: p.key,
            name: p.name,
            hardware: p.hardware,
            render_distance: p.render_distance,
            simulation_distance: p.simulation_distance,
            graphics_mode: p.graphics_mode,
            particles: p.particles,
            clouds: p.clouds,
            entity_distance_scaling: p.entity_distance_scaling,
            biome_blend_radius: p.biome_blend_radius,
            mipmap_levels: p.mipmap_levels,
            ao: p.ao,
            ram_mb: p.ram_mb,
            jvm_args: p.jvm_args,
            shader_pack: p.shader_pack,
            shader_packs_fallback: p.shader_packs_fallback,
            shader_options: p.shader_options,
            resource_packs: p.resource_packs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct PresetApplyResultDto {
    pub preset_key: String,
    pub chosen_shader: String,
    pub final_ram_mb: u32,
    pub jvm_args: Vec<String>,
    pub written_options: HashMap<String, String>,
    pub warning: Option<String>,
}

impl From<wex_extras::PresetApplyResult> for PresetApplyResultDto {
    fn from(r: wex_extras::PresetApplyResult) -> Self {
        Self {
            preset_key: r.preset_key,
            chosen_shader: r.chosen_shader,
            final_ram_mb: r.final_ram_mb,
            jvm_args: r.jvm_args,
            written_options: r.written_options,
            warning: r.warning,
        }
    }
}

// ============================================================================
// Social (wex-social) Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct UserAuthDto {
    pub user_id: String,
    pub user_token: String,
    pub friend_code: String,
    pub nick: String,
}

impl From<UserAuthDto> for wex_social::UserAuth {
    fn from(a: UserAuthDto) -> Self {
        Self {
            user_id: a.user_id,
            user_token: a.user_token,
            friend_code: a.friend_code,
            nick: a.nick,
        }
    }
}

impl From<wex_social::UserAuth> for UserAuthDto {
    fn from(a: wex_social::UserAuth) -> Self {
        Self {
            user_id: a.user_id,
            user_token: a.user_token,
            friend_code: a.friend_code,
            nick: a.nick,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
pub enum OnlineStatusDto {
    Online,
    Playing,
    InLobby,
    Offline,
}

impl From<wex_social::OnlineStatus> for OnlineStatusDto {
    fn from(s: wex_social::OnlineStatus) -> Self {
        match s {
            wex_social::OnlineStatus::Online => Self::Online,
            wex_social::OnlineStatus::Playing => Self::Playing,
            wex_social::OnlineStatus::InLobby => Self::InLobby,
            wex_social::OnlineStatus::Offline => Self::Offline,
        }
    }
}

impl From<OnlineStatusDto> for wex_social::OnlineStatus {
    fn from(s: OnlineStatusDto) -> Self {
        match s {
            OnlineStatusDto::Online => Self::Online,
            OnlineStatusDto::Playing => Self::Playing,
            OnlineStatusDto::InLobby => Self::InLobby,
            OnlineStatusDto::Offline => Self::Offline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct FriendProfileDto {
    pub id: String,
    pub nick: String,
    pub friend_code: Option<String>,
    pub play_time_seconds: u64,
    pub total_launches: u32,
    pub achievements_count: usize,
    pub skin_hash: Option<String>,
    pub skin_model: Option<String>,
}

impl From<wex_social::FriendProfile> for FriendProfileDto {
    fn from(p: wex_social::FriendProfile) -> Self {
        Self {
            id: p.id,
            nick: p.nick,
            friend_code: p.friend_code,
            play_time_seconds: p.play_time_seconds,
            total_launches: p.total_launches,
            achievements_count: p.achievements_count,
            skin_hash: p.skin_hash,
            skin_model: p.skin_model,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct FriendItemDto {
    pub id: String,
    pub nick: String,
    pub status: OnlineStatusDto,
    pub last_seen: Option<i64>,
    pub game_version: Option<String>,
    pub world_name: Option<String>,
    pub lobby_code: Option<String>,
}

impl From<wex_social::FriendItem> for FriendItemDto {
    fn from(f: wex_social::FriendItem) -> Self {
        Self {
            id: f.id,
            nick: f.nick,
            status: f.status.into(),
            last_seen: f.last_seen,
            game_version: f.game_version,
            world_name: f.world_name,
            lobby_code: f.lobby_code,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct UserPresenceDto {
    pub status: Option<OnlineStatusDto>,
    pub game_version: Option<String>,
    pub world_name: Option<String>,
    pub lobby_code: Option<String>,
}

impl From<UserPresenceDto> for wex_social::UserPresence {
    fn from(p: UserPresenceDto) -> Self {
        Self {
            status: p.status.map(Into::into),
            game_version: p.game_version,
            world_name: p.world_name,
            lobby_code: p.lobby_code,
        }
    }
}

impl From<wex_social::UserPresence> for UserPresenceDto {
    fn from(p: wex_social::UserPresence) -> Self {
        Self {
            status: p.status.map(Into::into),
            game_version: p.game_version,
            world_name: p.world_name,
            lobby_code: p.lobby_code,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct FriendRequestItemDto {
    pub id: String,
    pub from_id: String,
    pub from_nick: String,
    pub created_at: Option<i64>,
}

impl From<wex_social::FriendRequestItem> for FriendRequestItemDto {
    fn from(r: wex_social::FriendRequestItem) -> Self {
        Self {
            id: r.id,
            from_id: r.from_id,
            from_nick: r.from_nick,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct LobbyInviteItemDto {
    pub id: String,
    pub lobby_code: String,
    pub from_id: String,
    pub from_nick: String,
    pub host_name: Option<String>,
    pub created_at: Option<i64>,
}

impl From<wex_social::LobbyInviteItem> for LobbyInviteItemDto {
    fn from(i: wex_social::LobbyInviteItem) -> Self {
        Self {
            id: i.id,
            lobby_code: i.lobby_code,
            from_id: i.from_id,
            from_nick: i.from_nick,
            host_name: i.host_name,
            created_at: i.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct SyncResponseDto {
    pub friends: Vec<FriendItemDto>,
    pub incoming_requests: Vec<FriendRequestItemDto>,
    pub incoming_invites: Vec<LobbyInviteItemDto>,
}

impl From<wex_social::SyncResponse> for SyncResponseDto {
    fn from(s: wex_social::SyncResponse) -> Self {
        Self {
            friends: s.friends.into_iter().map(Into::into).collect(),
            incoming_requests: s.incoming_requests.into_iter().map(Into::into).collect(),
            incoming_invites: s.incoming_invites.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct AchievementDefinitionDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub target: u32,
}

impl From<wex_social::AchievementDefinition> for AchievementDefinitionDto {
    fn from(a: wex_social::AchievementDefinition) -> Self {
        Self {
            id: a.id,
            title: a.title,
            description: a.description,
            target: a.target,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
pub struct AchievementProgressDto {
    pub id: String,
    pub current: f64,
    pub unlocked: bool,
    pub unlocked_at: Option<String>,
}

impl From<wex_social::AchievementProgress> for AchievementProgressDto {
    fn from(p: wex_social::AchievementProgress) -> Self {
        Self {
            id: p.id,
            current: p.current,
            unlocked: p.unlocked,
            unlocked_at: p.unlocked_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct AchievementUnlockedEventDto {
    pub id: String,
    pub title: String,
    pub description: String,
    pub unlocked_at: String,
}

impl From<wex_social::AchievementUnlockedEvent> for AchievementUnlockedEventDto {
    fn from(e: wex_social::AchievementUnlockedEvent) -> Self {
        Self {
            id: e.id,
            title: e.title,
            description: e.description,
            unlocked_at: e.unlocked_at,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
pub enum GameModeDto {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
    Unknown = -1,
}

impl From<wex_social::GameMode> for GameModeDto {
    fn from(m: wex_social::GameMode) -> Self {
        match m {
            wex_social::GameMode::Survival => Self::Survival,
            wex_social::GameMode::Creative => Self::Creative,
            wex_social::GameMode::Adventure => Self::Adventure,
            wex_social::GameMode::Spectator => Self::Spectator,
            wex_social::GameMode::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct WorldItemDto {
    pub folder_name: String,
    pub folder_path: String,
    pub display_name: String,
    pub game_mode: GameModeDto,
    pub last_played: u64,
    pub day_time: u64,
    pub size_bytes: u64,
    pub icon_path: Option<String>,
}

impl From<wex_social::WorldItem> for WorldItemDto {
    fn from(w: wex_social::WorldItem) -> Self {
        Self {
            folder_name: w.folder_name,
            folder_path: w.folder_path.to_string_lossy().to_string(),
            display_name: w.display_name,
            game_mode: w.game_mode.into(),
            last_played: w.last_played,
            day_time: w.day_time,
            size_bytes: w.size_bytes,
            icon_path: w.icon_path.map(|p| p.to_string_lossy().to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct WorldBackupItemDto {
    pub file_name: String,
    pub file_path: String,
    pub created_at: Option<String>,
    pub size_bytes: u64,
}

impl From<wex_social::WorldBackupItem> for WorldBackupItemDto {
    fn from(b: wex_social::WorldBackupItem) -> Self {
        Self {
            file_name: b.file_name,
            file_path: b.file_path.to_string_lossy().to_string(),
            created_at: b.created_at,
            size_bytes: b.size_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ScreenshotItemDto {
    pub file_name: String,
    pub file_path: String,
    pub created_at: Option<String>,
    pub size_bytes: u64,
}

impl From<wex_social::ScreenshotItem> for ScreenshotItemDto {
    fn from(s: wex_social::ScreenshotItem) -> Self {
        Self {
            file_name: s.file_name,
            file_path: s.file_path.to_string_lossy().to_string(),
            created_at: s.created_at,
            size_bytes: s.size_bytes,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
pub enum NoteCategoryDto {
    New,
    Improved,
    Fixed,
    General,
}

impl From<wex_social::NoteCategory> for NoteCategoryDto {
    fn from(c: wex_social::NoteCategory) -> Self {
        match c {
            wex_social::NoteCategory::New => Self::New,
            wex_social::NoteCategory::Improved => Self::Improved,
            wex_social::NoteCategory::Fixed => Self::Fixed,
            wex_social::NoteCategory::General => Self::General,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ReleaseNoteItemDto {
    pub category: NoteCategoryDto,
    pub text: String,
    pub raw: String,
}

impl From<wex_social::ReleaseNoteItem> for ReleaseNoteItemDto {
    fn from(i: wex_social::ReleaseNoteItem) -> Self {
        Self {
            category: i.category.into(),
            text: i.text,
            raw: i.raw,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct ReleaseNoteVersionDto {
    pub version: String,
    pub internal_version: String,
    pub date_text: String,
    pub short_title: String,
    pub is_current: bool,
    pub items: Vec<ReleaseNoteItemDto>,
}

impl From<wex_social::ReleaseNoteVersion> for ReleaseNoteVersionDto {
    fn from(v: wex_social::ReleaseNoteVersion) -> Self {
        Self {
            version: v.version,
            internal_version: v.internal_version,
            date_text: v.date_text,
            short_title: v.short_title,
            is_current: v.is_current,
            items: v.items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
pub struct UpdateCheckResultDto {
    pub update_available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub download_url: Option<String>,
    pub signature: Option<String>,
    pub pub_date: Option<String>,
}

impl From<wex_social::UpdateCheckResult> for UpdateCheckResultDto {
    fn from(u: wex_social::UpdateCheckResult) -> Self {
        Self {
            update_available: u.update_available,
            current_version: u.current_version,
            latest_version: u.latest_version,
            notes: u.notes,
            download_url: u.download_url,
            signature: u.signature,
            pub_date: u.pub_date,
        }
    }
}
