use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::State;

use crate::commands::launch::PlayState;
use crate::ipc::{ServerEntryModel, ServerPingResult};
use wex_ipc::commands::{
    NetLoadServersRequest, NetLoadServersResponse, NetMergeServersRequest, NetMergeServersResponse,
    NetPingRequest, NetResolveRequest, NetResolveResponse, NetSaveServersRequest,
};
use wex_ipc::models::{PackServerEntryDto, PlayerSampleDto, ServerPingResponseDto};
use wex_net::servers::{
    load_servers_dat, normalize_address, save_servers_dat, NbtTag, PackServerEntry,
};
use wex_net::ServerPinger;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PingFailureReason {
    Offline,
    InvalidAddress,
    NoInternet,
}

impl PingFailureReason {
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::Offline => "Сервер выключен или недоступен",
            Self::InvalidAddress => "Неверный адрес сервера",
            Self::NoInternet => "Отсутствует подключение к интернету",
        }
    }
}

pub fn classify_ping_error(err: &wex_net::PingError) -> (PingFailureReason, &'static str) {
    match err {
        wex_net::PingError::Dns(_) => (
            PingFailureReason::InvalidAddress,
            PingFailureReason::InvalidAddress.user_message(),
        ),
        wex_net::PingError::Timeout => (
            PingFailureReason::Offline,
            PingFailureReason::Offline.user_message(),
        ),
        wex_net::PingError::Io(io_err) => {
            use std::io::ErrorKind;
            match io_err.kind() {
                ErrorKind::ConnectionRefused | ErrorKind::TimedOut | ErrorKind::ConnectionReset => {
                    (
                        PingFailureReason::Offline,
                        PingFailureReason::Offline.user_message(),
                    )
                }
                ErrorKind::NotFound | ErrorKind::AddrNotAvailable => (
                    PingFailureReason::InvalidAddress,
                    PingFailureReason::InvalidAddress.user_message(),
                ),
                ErrorKind::NetworkUnreachable | ErrorKind::NotConnected => (
                    PingFailureReason::NoInternet,
                    PingFailureReason::NoInternet.user_message(),
                ),
                _ => {
                    let s = io_err.to_string().to_lowercase();
                    if s.contains("network unreachable")
                        || s.contains("enetunreach")
                        || s.contains("enetdown")
                    {
                        (
                            PingFailureReason::NoInternet,
                            PingFailureReason::NoInternet.user_message(),
                        )
                    } else if s.contains("no such host")
                        || s.contains("failed to lookup")
                        || s.contains("invalid")
                        || s.contains("cannot assign")
                    {
                        (
                            PingFailureReason::InvalidAddress,
                            PingFailureReason::InvalidAddress.user_message(),
                        )
                    } else {
                        (
                            PingFailureReason::Offline,
                            PingFailureReason::Offline.user_message(),
                        )
                    }
                }
            }
        }
        wex_net::PingError::Protocol(_) | wex_net::PingError::Json(_) => (
            PingFailureReason::Offline,
            PingFailureReason::Offline.user_message(),
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerPingDetails {
    pub success: bool,
    pub latency_ms: Option<u64>,
    pub online: Option<u32>,
    pub max_players: Option<u32>,
    pub motd: Option<String>,
    pub version: Option<String>,
    pub favicon: Option<String>,
    pub failure_reason: Option<PingFailureReason>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FriendHostedServerModel {
    pub id: String,
    pub friend_nick: String,
    pub friend_id: Option<String>,
    pub world_name: String,
    pub address: String,
    pub lobby_code: Option<String>,
    pub game_version: Option<String>,
    pub is_lan: bool,
}

/// Returns the path to `servers.dat` for the specified version.
pub fn get_servers_dat_path(paths: &wex_core::config::WexPaths, version: Option<&str>) -> PathBuf {
    let ver = version.unwrap_or("1.21.1");
    let inst_dir = paths.instance_dir(ver);
    let _ = std::fs::create_dir_all(&inst_dir);
    inst_dir.join("servers.dat")
}

/// Official default Aura / Wex servers to seed when `servers.dat` is newly initialized.
pub fn default_official_servers() -> Vec<PackServerEntry> {
    vec![
        PackServerEntry {
            id: Some("aura-main".to_string()),
            name: "Aura Official — Survival & PvP".to_string(),
            ip: "mc.auramc.net".to_string(),
            icon: None,
        },
        PackServerEntry {
            id: Some("aura-modded".to_string()),
            name: "Aura Pack Community Server".to_string(),
            ip: "pack.auramc.net".to_string(),
            icon: None,
        },
    ]
}

/// Helper to parse all servers out of a root NBT compound.
pub fn parse_servers_from_nbt(root: &NbtTag) -> Vec<ServerEntryModel> {
    let mut result = Vec::new();

    let elements = match root.get("servers") {
        Some(NbtTag::List { elements, .. }) => elements,
        _ => return result,
    };

    for (idx, elem) in elements.iter().enumerate() {
        if let NbtTag::Compound(fields) = elem {
            let mut name = String::new();
            let mut ip = String::new();
            let mut icon = None;

            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", NbtTag::String(s)) => name = s.clone(),
                    ("ip", NbtTag::String(s)) => ip = s.clone(),
                    ("icon", NbtTag::String(s)) => icon = Some(s.clone()),
                    _ => {}
                }
            }

            if !ip.is_empty() {
                let id = format!("srv-{}-{}", idx, ip.replace([':', '.'], "-"));
                result.push(ServerEntryModel {
                    id,
                    name: if name.is_empty() { ip.clone() } else { name },
                    address: ip,
                    icon,
                    ping_ms: None,
                    online: None,
                    max_players: None,
                    motd: None,
                    version: None,
                });
            }
        }
    }

    result
}

// ============================================================================
// Tauri Commands: Servers Management
// ============================================================================

/// List Minecraft servers from `servers.dat` of the specified or default instance.
#[tauri::command]
pub async fn list_servers(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<ServerEntryModel>, String> {
    let servers_dat_path = get_servers_dat_path(&state.paths, version.as_deref());

    let mut root = load_servers_dat(&servers_dat_path).map_err(|e| e.to_string())?;
    let mut servers = parse_servers_from_nbt(&root);

    // If servers list is completely empty, merge the official Aura servers
    if servers.is_empty() {
        let defaults = default_official_servers();
        wex_net::merge_pack_servers(&mut root, &defaults);
        let _ = save_servers_dat(&servers_dat_path, &root);
        servers = parse_servers_from_nbt(&root);
    }

    Ok(servers)
}

/// Add a new server entry to `servers.dat` for the chosen instance.
#[tauri::command]
pub async fn add_server(
    version: Option<String>,
    name: String,
    address: String,
    state: State<'_, PlayState>,
) -> Result<ServerEntryModel, String> {
    let name_trimmed = name.trim().to_string();
    let addr_trimmed = address.trim().to_string();

    if name_trimmed.is_empty() || addr_trimmed.is_empty() {
        return Err("Название и адрес сервера не могут быть пустыми".to_string());
    }

    let servers_dat_path = get_servers_dat_path(&state.paths, version.as_deref());
    let mut root = load_servers_dat(&servers_dat_path).unwrap_or_else(|_| {
        NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: Vec::new(),
            },
        )])
    });

    let norm = normalize_address(&addr_trimmed);

    // Check if server with this normalized address already exists
    let elements = match root.get_mut("servers") {
        Some(NbtTag::List { elements, tag_id }) => {
            *tag_id = 10;
            elements
        }
        _ => {
            root.set(
                "servers",
                NbtTag::List {
                    tag_id: 10,
                    elements: Vec::new(),
                },
            );
            match root.get_mut("servers") {
                Some(NbtTag::List { elements, .. }) => elements,
                _ => unreachable!(),
            }
        }
    };

    let already_exists = elements.iter().any(|elem| {
        if let NbtTag::Compound(_) = elem {
            if let Some(ip) = elem.get_string("ip") {
                return normalize_address(ip) == norm;
            }
        }
        false
    });

    if already_exists {
        return Err("Сервер с таким адресом уже добавлен".to_string());
    }

    let new_entry = NbtTag::Compound(vec![
        ("name".to_string(), NbtTag::String(name_trimmed.clone())),
        ("ip".to_string(), NbtTag::String(addr_trimmed.clone())),
    ]);
    elements.push(new_entry);

    save_servers_dat(&servers_dat_path, &root).map_err(|e| e.to_string())?;

    Ok(ServerEntryModel {
        id: format!(
            "srv-{}",
            std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_millis()
        ),
        name: name_trimmed,
        address: addr_trimmed,
        icon: None,
        ping_ms: None,
        online: None,
        max_players: None,
        motd: None,
        version: None,
    })
}

/// Edit an existing server's name and/or address in `servers.dat`.
#[tauri::command]
pub async fn edit_server(
    version: Option<String>,
    old_address: String,
    name: String,
    new_address: String,
    state: State<'_, PlayState>,
) -> Result<ServerEntryModel, String> {
    let name_trimmed = name.trim().to_string();
    let new_addr_trimmed = new_address.trim().to_string();

    if name_trimmed.is_empty() || new_addr_trimmed.is_empty() {
        return Err("Название и адрес сервера не могут быть пустыми".to_string());
    }

    let servers_dat_path = get_servers_dat_path(&state.paths, version.as_deref());
    let mut root = load_servers_dat(&servers_dat_path).map_err(|e| e.to_string())?;

    let old_norm = normalize_address(&old_address);
    let mut updated = false;
    let mut existing_icon = None;

    if let Some(NbtTag::List { elements, .. }) = root.get_mut("servers") {
        for elem in elements.iter_mut() {
            if let NbtTag::Compound(_) = elem {
                if let Some(ip) = elem.get_string("ip") {
                    if normalize_address(ip) == old_norm {
                        elem.set("name", NbtTag::String(name_trimmed.clone()));
                        elem.set("ip", NbtTag::String(new_addr_trimmed.clone()));
                        existing_icon = elem.get_string("icon").map(|s| s.to_string());
                        updated = true;
                        break;
                    }
                }
            }
        }
    }

    if !updated {
        return Err("Сервер для изменения не найден".to_string());
    }

    save_servers_dat(&servers_dat_path, &root).map_err(|e| e.to_string())?;

    Ok(ServerEntryModel {
        id: format!(
            "srv-{}",
            std::time::SystemTime::now()
                .elapsed()
                .unwrap_or_default()
                .as_millis()
        ),
        name: name_trimmed,
        address: new_addr_trimmed,
        icon: existing_icon,
        ping_ms: None,
        online: None,
        max_players: None,
        motd: None,
        version: None,
    })
}

/// Remove a server entry from `servers.dat`.
#[tauri::command]
pub async fn remove_server(
    version: Option<String>,
    address: String,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let servers_dat_path = get_servers_dat_path(&state.paths, version.as_deref());
    let mut root = load_servers_dat(&servers_dat_path).map_err(|e| e.to_string())?;

    let norm = normalize_address(&address);

    if let Some(NbtTag::List { elements, .. }) = root.get_mut("servers") {
        elements.retain(|elem| {
            if let NbtTag::Compound(_) = elem {
                if let Some(ip) = elem.get_string("ip") {
                    return normalize_address(ip) != norm;
                }
            }
            true
        });
    }

    save_servers_dat(&servers_dat_path, &root).map_err(|e| e.to_string())
}

/// Ping a Minecraft server using SLP protocol with timeout.
#[tauri::command]
pub async fn ping_server(
    address: String,
    timeout_ms: Option<u64>,
) -> Result<ServerPingResult, String> {
    let timeout_val = timeout_ms.unwrap_or(2500);
    let pinger = ServerPinger::new(timeout_val, timeout_val, 763);

    match pinger.ping(&address).await {
        Ok(res) => Ok(ServerPingResult {
            latency_ms: res.latency_ms,
            online: res.players.online,
            max_players: res.players.max,
            motd: res.description,
            version: res.version.name,
            favicon: res.favicon,
        }),
        Err(e) => {
            let (reason, msg) = classify_ping_error(&e);
            let code = match reason {
                PingFailureReason::Offline => "offline",
                PingFailureReason::InvalidAddress => "invalid_address",
                PingFailureReason::NoInternet => "no_internet",
            };
            Err(format!("{}: {}", code, msg))
        }
    }
}

/// Ping a Minecraft server returning structured details and categorized failure reason without fake counts.
#[tauri::command]
pub async fn ping_server_details(
    address: String,
    timeout_ms: Option<u64>,
) -> Result<ServerPingDetails, String> {
    let timeout_val = timeout_ms.unwrap_or(2500);
    let pinger = ServerPinger::new(timeout_val, timeout_val, 763);

    match pinger.ping(&address).await {
        Ok(res) => Ok(ServerPingDetails {
            success: true,
            latency_ms: Some(res.latency_ms),
            online: Some(res.players.online),
            max_players: Some(res.players.max),
            motd: Some(res.description),
            version: Some(res.version.name),
            favicon: res.favicon,
            failure_reason: None,
            error_message: None,
        }),
        Err(e) => {
            let (reason, msg) = classify_ping_error(&e);
            Ok(ServerPingDetails {
                success: false,
                latency_ms: None,
                online: None,
                max_players: None,
                motd: None,
                version: None,
                favicon: None,
                failure_reason: Some(reason),
                error_message: Some(msg.to_string()),
            })
        }
    }
}

/// Helper function to import and merge servers from servers.dat of both versions (1.20.1 and 1.21.1)
/// without duplicate entries (matching by normalize_address).
pub fn import_servers_from_instances_internal(
    paths: &wex_core::config::WexPaths,
) -> Result<Vec<ServerEntryModel>, String> {
    let mut combined_servers: Vec<PackServerEntry> = Vec::new();
    let mut seen_addresses = HashSet::new();

    let paths_to_check = [
        paths.instance_dir("1.21.1").join("servers.dat"),
        paths.instance_dir("1.20.1").join("servers.dat"),
        paths.root.join("servers.dat"),
    ];

    for s_path in &paths_to_check {
        if s_path.exists() {
            if let Ok(root) = load_servers_dat(s_path) {
                let entries = parse_servers_from_nbt(&root);
                for entry in entries {
                    let norm = normalize_address(&entry.address);
                    if seen_addresses.insert(norm) {
                        combined_servers.push(PackServerEntry {
                            id: Some(entry.id),
                            name: entry.name,
                            ip: entry.address,
                            icon: entry.icon,
                        });
                    }
                }
            }
        }
    }

    // If completely empty across all files, seed with default official servers
    if combined_servers.is_empty() {
        for s in default_official_servers() {
            let norm = normalize_address(&s.ip);
            if seen_addresses.insert(norm) {
                combined_servers.push(s);
            }
        }
    }

    // Build unified NbtTag compound
    let mut unified_root = NbtTag::Compound(vec![(
        "servers".to_string(),
        NbtTag::List {
            tag_id: 10,
            elements: Vec::new(),
        },
    )]);
    wex_net::merge_pack_servers(&mut unified_root, &combined_servers);

    // Save unified deduplicated list back to both 1.20.1 and 1.21.1 instances
    for ver in &["1.20.1", "1.21.1"] {
        let p = get_servers_dat_path(paths, Some(ver));
        let _ = save_servers_dat(&p, &unified_root);
    }

    Ok(parse_servers_from_nbt(&unified_root))
}

/// Import servers from servers.dat of both versions (1.20.1 and 1.21.1) without duplicates.
#[tauri::command]
pub async fn import_servers_both_versions(
    state: State<'_, PlayState>,
) -> Result<Vec<ServerEntryModel>, String> {
    import_servers_from_instances_internal(&state.paths)
}

/// Retrieves friend hosted servers from friends presence and lobby rooms.
#[tauri::command]
pub async fn get_friend_hosted_servers(
    state: State<'_, PlayState>,
) -> Result<Vec<FriendHostedServerModel>, String> {
    let mut result = Vec::new();
    let mut seen_ids = HashSet::new();

    // 1. Check friends state file
    let friends_file = state.paths.root.join("friends.json");
    if let Ok(content) = std::fs::read_to_string(&friends_file) {
        if let Ok(friends) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
            for f in friends {
                let nick = f
                    .get("nick")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let status = f.get("status").and_then(|v| v.as_str()).unwrap_or_default();
                let world_name = f
                    .get("world_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let lobby_code = f
                    .get("lobby_code")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let game_version = f
                    .get("game_version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let id = f.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());

                if (status == "playing" || status == "in_lobby")
                    && (world_name.is_some() || lobby_code.is_some())
                {
                    let w_name = world_name.unwrap_or_else(|| "Кооперативный мир".to_string());
                    let addr = if let Some(ref code) = lobby_code {
                        format!("#{code}")
                    } else if w_name.contains(':') {
                        w_name.clone()
                    } else {
                        "127.0.0.1:25565".to_string()
                    };

                    let srv_id = format!("friend-srv-{}", id.as_deref().unwrap_or(&nick));
                    if seen_ids.insert(srv_id.clone()) {
                        result.push(FriendHostedServerModel {
                            id: srv_id,
                            friend_nick: nick,
                            friend_id: id,
                            world_name: w_name,
                            address: addr,
                            lobby_code,
                            game_version,
                            is_lan: false,
                        });
                    }
                }
            }
        }
    }

    Ok(result)
}

// ============================================================================
// Standard IPC DTO Commands (wex-ipc compliance)
// ============================================================================

#[tauri::command]
pub async fn net_load_servers(
    request: NetLoadServersRequest,
) -> Result<NetLoadServersResponse, String> {
    let root = load_servers_dat(&request.path).map_err(|e| e.to_string())?;
    let mut dtos = Vec::new();

    if let Some(NbtTag::List { elements, .. }) = root.get("servers") {
        for elem in elements {
            if let NbtTag::Compound(_) = elem {
                let name = elem.get_string("name").unwrap_or_default().to_string();
                let ip = elem.get_string("ip").unwrap_or_default().to_string();
                let icon = elem.get_string("icon").map(|s| s.to_string());
                if !ip.is_empty() {
                    dtos.push(PackServerEntryDto {
                        id: None,
                        name,
                        ip,
                        icon,
                    });
                }
            }
        }
    }

    Ok(NetLoadServersResponse { servers: dtos })
}

#[tauri::command]
pub async fn net_save_servers(request: NetSaveServersRequest) -> Result<(), String> {
    let pack_servers: Vec<PackServerEntry> = request
        .servers
        .into_iter()
        .map(|s| PackServerEntry {
            id: s.id,
            name: s.name,
            ip: s.ip,
            icon: s.icon,
        })
        .collect();

    let mut root = NbtTag::Compound(vec![(
        "servers".to_string(),
        NbtTag::List {
            tag_id: 10,
            elements: Vec::new(),
        },
    )]);

    wex_net::merge_pack_servers(&mut root, &pack_servers);
    save_servers_dat(&request.path, &root).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn net_merge_servers(
    request: NetMergeServersRequest,
) -> Result<NetMergeServersResponse, String> {
    let mut root = load_servers_dat(&request.user_servers_path).map_err(|e| e.to_string())?;
    let pack_servers: Vec<PackServerEntry> = request
        .pack_servers
        .into_iter()
        .map(|s| PackServerEntry {
            id: s.id,
            name: s.name,
            ip: s.ip,
            icon: s.icon,
        })
        .collect();

    let before_count = match root.get("servers") {
        Some(NbtTag::List { elements, .. }) => elements.len(),
        _ => 0,
    };

    wex_net::merge_pack_servers(&mut root, &pack_servers);

    let after_count = match root.get("servers") {
        Some(NbtTag::List { elements, .. }) => elements.len(),
        _ => 0,
    };

    save_servers_dat(&request.user_servers_path, &root).map_err(|e| e.to_string())?;

    Ok(NetMergeServersResponse {
        merged_count: after_count.saturating_sub(before_count),
    })
}

#[tauri::command]
pub async fn net_ping_server(request: NetPingRequest) -> Result<ServerPingResponseDto, String> {
    let timeout = request.timeout_ms.unwrap_or(2500);
    let pinger = ServerPinger::new(timeout, timeout, 763);

    let address = if let Some(port) = request.port {
        format!("{}:{}", request.address, port)
    } else {
        request.address
    };

    let res = pinger.ping(&address).await.map_err(|e| e.to_string())?;

    Ok(ServerPingResponseDto {
        host: res.host,
        port: res.port,
        latency_ms: res.latency_ms,
        version_name: res.version.name,
        protocol: res.version.protocol,
        players_online: res.players.online,
        players_max: res.players.max,
        sample_players: res
            .players
            .sample
            .unwrap_or_default()
            .into_iter()
            .map(|p| PlayerSampleDto {
                name: p.name,
                id: p.id,
            })
            .collect(),
        description: res.description,
        favicon: res.favicon,
    })
}

#[tauri::command]
pub async fn net_resolve_address(request: NetResolveRequest) -> Result<NetResolveResponse, String> {
    let (host, port) = wex_net::resolve_minecraft_address(&request.host).await;

    Ok(NetResolveResponse { host, port })
}

// ============================================================================
// Unit Tests (Mock core / local files)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_core::config::WexPaths;

    #[test]
    fn test_nbt_parsing_and_defaults() {
        let root = NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: vec![
                    NbtTag::Compound(vec![
                        ("name".to_string(), NbtTag::String("Server 1".to_string())),
                        (
                            "ip".to_string(),
                            NbtTag::String("play.example.com".to_string()),
                        ),
                    ]),
                    NbtTag::Compound(vec![
                        ("name".to_string(), NbtTag::String("Server 2".to_string())),
                        (
                            "ip".to_string(),
                            NbtTag::String("mc.test.org:25565".to_string()),
                        ),
                        (
                            "icon".to_string(),
                            NbtTag::String("data:image/png;base64,...".to_string()),
                        ),
                    ]),
                ],
            },
        )]);

        let parsed = parse_servers_from_nbt(&root);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "Server 1");
        assert_eq!(parsed[0].address, "play.example.com");
        assert!(parsed[0].icon.is_none());

        assert_eq!(parsed[1].name, "Server 2");
        assert_eq!(parsed[1].address, "mc.test.org:25565");
        assert_eq!(parsed[1].icon.as_deref(), Some("data:image/png;base64,..."));
    }

    #[test]
    fn test_servers_dat_add_edit_remove_cycle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let servers_dat = get_servers_dat_path(&paths, Some("1.21.1"));

        // 1. Initial save of default servers
        let mut root = NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: Vec::new(),
            },
        )]);
        save_servers_dat(&servers_dat, &root).unwrap();

        // 2. Add server
        let elem = NbtTag::Compound(vec![
            (
                "name".to_string(),
                NbtTag::String("Survival Realm".to_string()),
            ),
            ("ip".to_string(), NbtTag::String("realm.mc.net".to_string())),
        ]);
        if let Some(NbtTag::List { elements, .. }) = root.get_mut("servers") {
            elements.push(elem);
        }
        save_servers_dat(&servers_dat, &root).unwrap();

        let loaded = load_servers_dat(&servers_dat).unwrap();
        let servers = parse_servers_from_nbt(&loaded);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "Survival Realm");
        assert_eq!(servers[0].address, "realm.mc.net");

        // 3. Edit server
        if let Some(NbtTag::List { elements, .. }) = root.get_mut("servers") {
            for s in elements.iter_mut() {
                if s.get_string("ip") == Some("realm.mc.net") {
                    s.set("name", NbtTag::String("Updated Realm".to_string()));
                    s.set("ip", NbtTag::String("realm.mc.net:25577".to_string()));
                }
            }
        }
        save_servers_dat(&servers_dat, &root).unwrap();

        let loaded2 = load_servers_dat(&servers_dat).unwrap();
        let servers2 = parse_servers_from_nbt(&loaded2);
        assert_eq!(servers2.len(), 1);
        assert_eq!(servers2[0].name, "Updated Realm");
        assert_eq!(servers2[0].address, "realm.mc.net:25577");

        // 4. Remove server
        if let Some(NbtTag::List { elements, .. }) = root.get_mut("servers") {
            elements.retain(|s| s.get_string("ip") != Some("realm.mc.net:25577"));
        }
        save_servers_dat(&servers_dat, &root).unwrap();

        let loaded3 = load_servers_dat(&servers_dat).unwrap();
        let servers3 = parse_servers_from_nbt(&loaded3);
        assert_eq!(servers3.len(), 0);
    }

    #[test]
    fn test_classify_ping_error_categories() {
        use std::io;

        // 1. Offline (ConnectionRefused, TimedOut, etc.)
        let err_timeout = wex_net::PingError::Timeout;
        let (reason1, msg1) = classify_ping_error(&err_timeout);
        assert_eq!(reason1, PingFailureReason::Offline);
        assert_eq!(msg1, "Сервер выключен или недоступен");

        let err_refused = wex_net::PingError::Io(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "connection refused",
        ));
        let (reason2, msg2) = classify_ping_error(&err_refused);
        assert_eq!(reason2, PingFailureReason::Offline);
        assert_eq!(msg2, "Сервер выключен или недоступен");

        // 2. Invalid address (DNS, NotFound, AddrNotAvailable)
        let err_dns = wex_net::PingError::Dns("failed to lookup address information".to_string());
        let (reason3, msg3) = classify_ping_error(&err_dns);
        assert_eq!(reason3, PingFailureReason::InvalidAddress);
        assert_eq!(msg3, "Неверный адрес сервера");

        let err_not_found = wex_net::PingError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "No such host is known",
        ));
        let (reason4, msg4) = classify_ping_error(&err_not_found);
        assert_eq!(reason4, PingFailureReason::InvalidAddress);
        assert_eq!(msg4, "Неверный адрес сервера");

        // 3. No internet (NetworkUnreachable, NotConnected)
        let err_net = wex_net::PingError::Io(io::Error::new(
            io::ErrorKind::NetworkUnreachable,
            "A socket operation was attempted to an unreachable network",
        ));
        let (reason5, msg5) = classify_ping_error(&err_net);
        assert_eq!(reason5, PingFailureReason::NoInternet);
        assert_eq!(msg5, "Отсутствует подключение к интернету");
    }

    #[test]
    fn test_import_servers_both_versions_deduplication() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));

        // 1. Setup 1.20.1 servers.dat with Server 1 and Duplicate Server
        let v120_dat = get_servers_dat_path(&paths, Some("1.20.1"));
        let root120 = NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: vec![
                    NbtTag::Compound(vec![
                        ("name".to_string(), NbtTag::String("SMP 1.20".to_string())),
                        (
                            "ip".to_string(),
                            NbtTag::String("smp.auramc.net:25565".to_string()),
                        ),
                    ]),
                    NbtTag::Compound(vec![
                        ("name".to_string(), NbtTag::String("Aura Hub".to_string())),
                        (
                            "ip".to_string(),
                            NbtTag::String("hub.auramc.net:25565".to_string()),
                        ),
                    ]),
                ],
            },
        )]);
        save_servers_dat(&v120_dat, &root120).unwrap();

        // 2. Setup 1.21.1 servers.dat with Server 2 and Duplicate Server (with normalized alias "hub.auramc.net")
        let v121_dat = get_servers_dat_path(&paths, Some("1.21.1"));
        let root121 = NbtTag::Compound(vec![(
            "servers".to_string(),
            NbtTag::List {
                tag_id: 10,
                elements: vec![
                    NbtTag::Compound(vec![
                        (
                            "name".to_string(),
                            NbtTag::String("Minigames 1.21".to_string()),
                        ),
                        (
                            "ip".to_string(),
                            NbtTag::String("mini.auramc.net".to_string()),
                        ),
                    ]),
                    NbtTag::Compound(vec![
                        ("name".to_string(), NbtTag::String("Hub Lobby".to_string())),
                        (
                            "ip".to_string(),
                            NbtTag::String("hub.auramc.net".to_string()),
                        ),
                    ]),
                ],
            },
        )]);
        save_servers_dat(&v121_dat, &root121).unwrap();

        // 3. Run import_servers_from_instances_internal
        let merged = import_servers_from_instances_internal(&paths).unwrap();

        // Verify exactly 3 servers are returned (hub.auramc.net deduplicated)
        assert_eq!(
            merged.len(),
            3,
            "Expected exactly 3 servers after deduplication, got: {:?}",
            merged
        );

        let addrs: Vec<String> = merged.iter().map(|s| s.address.clone()).collect();
        assert!(addrs.contains(&"smp.auramc.net:25565".to_string()));
        assert!(addrs.contains(&"mini.auramc.net".to_string()));
        assert!(
            addrs.contains(&"hub.auramc.net".to_string())
                || addrs.contains(&"hub.auramc.net:25565".to_string())
        );

        // Verify both 1.20.1 and 1.21.1 servers.dat files now have all 3 deduplicated servers
        let loaded_v120 = load_servers_dat(&v120_dat).unwrap();
        let servers_v120 = parse_servers_from_nbt(&loaded_v120);
        assert_eq!(servers_v120.len(), 3);

        let loaded_v121 = load_servers_dat(&v121_dat).unwrap();
        let servers_v121 = parse_servers_from_nbt(&loaded_v121);
        assert_eq!(servers_v121.len(), 3);
    }

    #[tokio::test]
    async fn test_ping_offline_server_on_loopback_returns_offline_diagnostic() {
        // Ping unused local port on 127.0.0.1 (server turned off)
        let pinger = ServerPinger::new(300, 300, 763);
        let res = pinger.ping("127.0.0.1:49151").await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        let (reason, msg) = classify_ping_error(&err);
        assert_eq!(reason, PingFailureReason::Offline);
        assert_eq!(msg, "Сервер выключен или недоступен");
    }
}
