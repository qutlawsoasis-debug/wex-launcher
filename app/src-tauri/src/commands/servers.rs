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
        Err(e) => Err(e.to_string()),
    }
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
}
