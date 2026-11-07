//! Network modules for Wex launcher.
//!
//! Provides:
//! - `ping`: Minecraft Server List Ping (SLP) protocol, VarInt, DNS SRV resolution.
//! - `servers`: `servers.dat` NBT parsing & serialization, merging with pack server lists.
//! - `lobby`: Async HTTP client for the Lobby API (rooms, heartbeats, status).
//! - `modsync`: Mod manifest comparison and deterministic SHA-256 hashing.
//! - `tunnel`: Supervisor for `playit-agent` process, SHA-256 verification and Windows Job Object lifecycle.
//! - `events`: Event bus and channel for real-time network and status progress.

pub mod events;
pub mod lobby;
pub mod modsync;
pub mod ping;
pub mod servers;
pub mod tunnel;

pub use events::{NetEvent, NetEventBus, TunnelStatus};
pub use lobby::{LobbyApiClient, LobbyError};
pub use modsync::{compare_manifests, compute_manifest_hash, ModManifestEntry, ModMismatchItem, ModMismatchType};
pub use ping::{resolve_minecraft_address, ServerPingResponse, ServerPinger};
pub use servers::{load_servers_dat, merge_pack_servers, save_servers_dat, NbtTag, PackServerEntry};
pub use tunnel::{ensure_binary_downloaded, parse_tunnel_address, PlayitSupervisor};
