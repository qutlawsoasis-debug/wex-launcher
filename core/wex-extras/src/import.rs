use crate::skins::{detect_skin_model, SkinModel};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("Каталог данных старого лаунчера не найден: {0}")]
    NotFound(PathBuf),
    #[error("Ошибка ввода-вывода при импорте: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка разбора JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedServer {
    pub name: String,
    pub ip: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSkin {
    pub file_name: String,
    pub path: PathBuf,
    pub hash: String,
    pub is_slim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedCape {
    pub file_name: String,
    pub path: PathBuf,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedWardrobeItem {
    pub id: String,
    pub item_type: String,
    pub name: String,
    pub hash: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedWorld {
    pub folder_name: String,
    pub display_name: String,
    pub version: Option<String>,
    pub source_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OldLauncherData {
    pub nickname: String,
    pub ram_mb: u32,
    pub selected_version: String,
    pub graphics_preset: Option<String>,
    pub total_play_time_seconds: u64,
    pub total_game_launches: u32,
    pub servers: Vec<ImportedServer>,
    pub skins: Vec<ImportedSkin>,
    pub capes: Vec<ImportedCape>,
    pub wardrobe_history: Vec<ImportedWardrobeItem>,
    pub worlds: Vec<ImportedWorld>,
    pub jvm_args: Option<String>,
    pub discord_rpc: Option<bool>,
    pub active_skin_path: Option<String>,
    pub active_skin_model: Option<String>,
}

pub fn default_aura_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|p| p.join(".aura"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join(".aura"))
    }
}

pub fn import_from_dir(aura_dir: &Path) -> Result<OldLauncherData, ImportError> {
    if !aura_dir.is_dir() {
        return Err(ImportError::NotFound(aura_dir.to_path_buf()));
    }

    let mut nickname = "Player".to_string();
    let mut ram_mb = 6144;
    let mut selected_version = "1.20.1".to_string();
    let mut graphics_preset = None;
    let mut total_play_time_seconds = 0;
    let mut total_game_launches = 0;
    let mut wardrobe_history = Vec::new();
    let mut jvm_args = None;
    let mut discord_rpc = None;
    let mut active_skin_path = None;
    let mut active_skin_model = None;

    // 1. Read config.json
    let config_path = aura_dir.join("config.json");
    if config_path.is_file() {
        if let Ok(text) = fs::read_to_string(&config_path) {
            let clean_text = text.trim_start_matches('\u{feff}');
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(clean_text) {
                if let Some(nick) = v.get("Nickname").and_then(|n| n.as_str()) {
                    nickname = nick.to_string();
                }
                if let Some(ram) = v.get("RamMb").and_then(|r| r.as_u64()) {
                    ram_mb = ram as u32;
                }
                if let Some(ver) = v.get("SelectedVersion").and_then(|s| s.as_str()) {
                    selected_version = ver.to_string();
                }
                if let Some(preset) = v.get("GraphicsPreset").and_then(|p| p.as_str()) {
                    graphics_preset = Some(preset.to_string());
                }
                if let Some(args) = v.get("JvmArgs").and_then(|a| a.as_str()) {
                    jvm_args = Some(args.to_string());
                }
                if let Some(rpc) = v.get("DiscordRpcEnabled").and_then(|d| d.as_bool()) {
                    discord_rpc = Some(rpc);
                }
                if let Some(sp) = v.get("SkinPath").and_then(|s| s.as_str()) {
                    if !sp.trim().is_empty() {
                        active_skin_path = Some(sp.to_string());
                    }
                }
                if let Some(sm) = v.get("SkinModel").and_then(|s| s.as_str()) {
                    active_skin_model = Some(sm.to_string());
                }
                if let Some(time) = v.get("TotalPlayTimeSeconds").and_then(|t| t.as_u64()) {
                    total_play_time_seconds = time;
                }
                if let Some(launches) = v.get("TotalGameLaunches").and_then(|l| l.as_u64()) {
                    total_game_launches = launches as u32;
                }
                if let Some(arr) = v.get("WardrobeHistory").and_then(|w| w.as_array()) {
                    for item_val in arr {
                        wardrobe_history.push(ImportedWardrobeItem {
                            id: item_val
                                .get("Id")
                                .and_then(|i| i.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            item_type: item_val
                                .get("Type")
                                .and_then(|t| t.as_str())
                                .unwrap_or("skin")
                                .to_string(),
                            name: item_val
                                .get("Name")
                                .and_then(|n| n.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            hash: item_val
                                .get("Hash")
                                .and_then(|h| h.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            model: item_val
                                .get("Model")
                                .and_then(|m| m.as_str())
                                .unwrap_or("default")
                                .to_string(),
                        });
                    }
                }
            }
        }
    }

    // 2. Read servers.dat
    let mut servers = Vec::new();
    let server_dat_candidates = [
        aura_dir
            .join("instances")
            .join("1.20.1")
            .join("servers.dat"),
        aura_dir
            .join("instances")
            .join("1.21.1")
            .join("servers.dat"),
        aura_dir.join("servers.dat"),
    ];

    for s_path in &server_dat_candidates {
        if s_path.is_file() {
            if let Ok(bytes) = fs::read(s_path) {
                let parsed = parse_servers_dat_bytes(&bytes);
                for s in parsed {
                    if !servers
                        .iter()
                        .any(|existing: &ImportedServer| existing.ip.eq_ignore_ascii_case(&s.ip))
                    {
                        servers.push(s);
                    }
                }
            }
        }
    }

    // 3. Scan skins/
    let mut skins = Vec::new();
    let skins_dir = aura_dir.join("skins");
    if skins_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&skins_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.extension()
                        .map(|ext| ext.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                {
                    if let Ok(bytes) = fs::read(&p) {
                        let mut hasher = Sha256::new();
                        hasher.update(&bytes);
                        let hash = hex::encode(hasher.finalize());

                        let is_slim = if let Ok(img) = image::load_from_memory(&bytes) {
                            detect_skin_model(&img.to_rgba8()) == SkinModel::Slim
                        } else {
                            false
                        };

                        skins.push(ImportedSkin {
                            file_name: p
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path: p,
                            hash,
                            is_slim,
                        });
                    }
                }
            }
        }
    }

    // 4. Scan capes/
    let mut capes = Vec::new();
    let capes_dir = aura_dir.join("capes");
    if capes_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&capes_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file()
                    && p.extension()
                        .map(|ext| ext.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                {
                    if let Ok(bytes) = fs::read(&p) {
                        let mut hasher = Sha256::new();
                        hasher.update(&bytes);
                        let hash = hex::encode(hasher.finalize());

                        capes.push(ImportedCape {
                            file_name: p
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            path: p,
                            hash,
                        });
                    }
                }
            }
        }
    }

    // 5. Check if active SkinPath exists and is not yet in skins list
    if let Some(ref sp) = active_skin_path {
        let p = PathBuf::from(sp);
        let resolved = if p.is_absolute() { p } else { aura_dir.join(p) };
        if resolved.is_file() {
            let fn_str = resolved
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !skins.iter().any(|s: &ImportedSkin| s.file_name == fn_str) {
                if let Ok(bytes) = fs::read(&resolved) {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    let hash = hex::encode(hasher.finalize());
                    let is_slim = active_skin_model
                        .as_deref()
                        .map(|m| m.eq_ignore_ascii_case("slim"))
                        .unwrap_or(false);

                    skins.push(ImportedSkin {
                        file_name: fn_str,
                        path: resolved,
                        hash,
                        is_slim,
                    });
                }
            }
        }
    }

    // 6. Scan saves/ (worlds from instances and legacy root)
    let mut worlds = Vec::new();
    let save_candidates: [(&str, Option<&str>); 3] = [
        ("instances/1.20.1/saves", Some("1.20.1")),
        ("instances/1.21.1/saves", Some("1.21.1")),
        ("saves", None),
    ];

    for (rel_path, ver) in save_candidates {
        let saves_path = aura_dir.join(rel_path);
        if saves_path.is_dir() {
            if let Ok(entries) = fs::read_dir(&saves_path) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let level_dat = p.join("level.dat");
                        if level_dat.is_file() {
                            let folder_name = p
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();

                            // Avoid duplicate worlds by folder_name
                            if !worlds
                                .iter()
                                .any(|w: &ImportedWorld| w.folder_name == folder_name)
                            {
                                let display_name = parse_level_name_fallback(&level_dat)
                                    .unwrap_or_else(|| folder_name.clone());
                                let size_bytes = dir_size_recursive(&p);

                                worlds.push(ImportedWorld {
                                    folder_name,
                                    display_name,
                                    version: ver.map(|s| s.to_string()),
                                    source_path: p,
                                    size_bytes,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(OldLauncherData {
        nickname,
        ram_mb,
        selected_version,
        graphics_preset,
        total_play_time_seconds,
        total_game_launches,
        servers,
        skins,
        capes,
        wardrobe_history,
        worlds,
        jvm_args,
        discord_rpc,
        active_skin_path,
        active_skin_model,
    })
}

fn parse_level_name_fallback(level_dat_path: &Path) -> Option<String> {
    let bytes = fs::read(level_dat_path).ok()?;
    let data = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
        let mut decomp = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decomp).ok()?;
        decomp
    } else {
        bytes
    };

    // Look for string tag with name "LevelName" (tag 8, name len 9, b"LevelName")
    for i in 0..data.len().saturating_sub(15) {
        if data[i] == 0x08
            && data[i + 1] == 0x00
            && data[i + 2] == 0x09
            && &data[i + 3..i + 12] == b"LevelName"
        {
            let len_pos = i + 12;
            if len_pos + 2 <= data.len() {
                let s_len = u16::from_be_bytes([data[len_pos], data[len_pos + 1]]) as usize;
                let s_start = len_pos + 2;
                if s_start + s_len <= data.len() {
                    let s = String::from_utf8_lossy(&data[s_start..s_start + s_len])
                        .trim()
                        .to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

fn dir_size_recursive(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += dir_size_recursive(&p);
            }
        }
    }
    total
}

// Simple binary NBT scanner for servers.dat to avoid complex NBT dependencies in wex-extras
fn parse_servers_dat_bytes(bytes: &[u8]) -> Vec<ImportedServer> {
    let mut servers = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // Look for string tag with name "ip" (TagType: 8, Name length: 2, Name: "ip")
        // In uncompressed NBT: byte 0x08, [0x00, 0x02], b"ip", [len_hi, len_lo], string_bytes
        if i + 5 < bytes.len()
            && bytes[i] == 0x08
            && bytes[i + 1] == 0x00
            && bytes[i + 2] == 0x02
            && &bytes[i + 3..i + 5] == b"ip"
        {
            let ip_len_pos = i + 5;
            if ip_len_pos + 2 <= bytes.len() {
                let ip_len =
                    u16::from_be_bytes([bytes[ip_len_pos], bytes[ip_len_pos + 1]]) as usize;
                let ip_start = ip_len_pos + 2;
                if ip_start + ip_len <= bytes.len() {
                    let ip =
                        String::from_utf8_lossy(&bytes[ip_start..ip_start + ip_len]).to_string();

                    // Search nearby for "name" tag within 256 bytes backwards or forwards
                    let mut name = ip.clone();
                    let search_start = i.saturating_sub(256);
                    let search_end = (i + ip_len + 256).min(bytes.len());

                    for j in search_start..search_end.saturating_sub(7) {
                        if bytes[j] == 0x08
                            && bytes[j + 1] == 0x00
                            && bytes[j + 2] == 0x04
                            && &bytes[j + 3..j + 7] == b"name"
                        {
                            let n_len_pos = j + 7;
                            if n_len_pos + 2 <= bytes.len() {
                                let n_len =
                                    u16::from_be_bytes([bytes[n_len_pos], bytes[n_len_pos + 1]])
                                        as usize;
                                let n_start = n_len_pos + 2;
                                if n_start + n_len <= bytes.len() {
                                    name =
                                        String::from_utf8_lossy(&bytes[n_start..n_start + n_len])
                                            .to_string();
                                    break;
                                }
                            }
                        }
                    }

                    if !ip.is_empty() {
                        servers.push(ImportedServer {
                            name,
                            ip,
                            icon: None,
                        });
                    }

                    i = ip_start + ip_len;
                    continue;
                }
            }
        }
        i += 1;
    }

    servers
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_import_from_fixture_dir() {
        let dir = tempdir().unwrap();
        let aura_path = dir.path();

        // 1. Write config.json
        let config_json = serde_json::json!({
            "Nickname": "MasterMiner",
            "RamMb": 8192,
            "SelectedVersion": "1.21.1",
            "GraphicsPreset": "High",
            "TotalPlayTimeSeconds": 36000,
            "TotalGameLaunches": 42,
            "WardrobeHistory": [
                {
                    "Id": "skin_abcdef",
                    "Type": "skin",
                    "Name": "Warrior",
                    "Hash": "abcdef123456",
                    "Model": "slim"
                }
            ]
        });
        fs::write(
            aura_path.join("config.json"),
            serde_json::to_string(&config_json).unwrap(),
        )
        .unwrap();

        // 2. Write servers.dat with NBT binary fragment
        let inst_dir = aura_path.join("instances").join("1.20.1");
        fs::create_dir_all(&inst_dir).unwrap();

        let mut nbt_dummy = Vec::new();
        // tag 8 "name": "Aura Official"
        nbt_dummy.push(0x08);
        nbt_dummy.extend_from_slice(&[0x00, 0x04]);
        nbt_dummy.extend_from_slice(b"name");
        nbt_dummy.extend_from_slice(&[0x00, 0x0D]);
        nbt_dummy.extend_from_slice(b"Aura Official");

        // tag 8 "ip": "play.auramc.net"
        nbt_dummy.push(0x08);
        nbt_dummy.extend_from_slice(&[0x00, 0x02]);
        nbt_dummy.extend_from_slice(b"ip");
        nbt_dummy.extend_from_slice(&[0x00, 0x0F]);
        nbt_dummy.extend_from_slice(b"play.auramc.net");

        fs::write(inst_dir.join("servers.dat"), &nbt_dummy).unwrap();

        // 3. Write a mock skin
        let skins_dir = aura_path.join("skins");
        fs::create_dir_all(&skins_dir).unwrap();
        let mock_skin = image::RgbaImage::from_pixel(64, 64, image::Rgba([1, 2, 3, 255]));
        let skin_bytes = crate::skins::encode_png_bytes(&mock_skin).unwrap();
        fs::write(skins_dir.join("custom_skin.png"), &skin_bytes).unwrap();

        // 4. Write mock world saves
        let world_dir = inst_dir.join("saves").join("AuraWorld");
        fs::create_dir_all(&world_dir).unwrap();
        fs::write(world_dir.join("level.dat"), b"MOCK_LEVEL_DAT").unwrap();
        fs::write(world_dir.join("icon.png"), b"MOCK_ICON").unwrap();

        // Perform import
        let imported = import_from_dir(aura_path).unwrap();

        assert_eq!(imported.nickname, "MasterMiner");
        assert_eq!(imported.ram_mb, 8192);
        assert_eq!(imported.selected_version, "1.21.1");
        assert_eq!(imported.graphics_preset.as_deref(), Some("High"));
        assert_eq!(imported.total_game_launches, 42);
        assert_eq!(imported.wardrobe_history.len(), 1);
        assert_eq!(imported.wardrobe_history[0].name, "Warrior");

        assert_eq!(imported.servers.len(), 1);
        assert_eq!(imported.servers[0].ip, "play.auramc.net");
        assert_eq!(imported.servers[0].name, "Aura Official");

        assert_eq!(imported.skins.len(), 1);
        assert_eq!(imported.skins[0].file_name, "custom_skin.png");

        assert_eq!(imported.worlds.len(), 1);
        assert_eq!(imported.worlds[0].folder_name, "AuraWorld");
        assert_eq!(imported.worlds[0].version.as_deref(), Some("1.20.1"));
    }

    #[test]
    fn test_import_missing_dir_returns_error() {
        let missing = PathBuf::from("C:\\definitely\\non_existent_aura_dir_12345");
        let res = import_from_dir(&missing);
        assert!(res.is_err());
    }

    #[test]
    fn test_import_from_fixture_with_bom() {
        let dir = tempdir().unwrap();
        let aura_path = dir.path();

        // Write config with UTF-8 BOM (\u{feff})
        let config_str =
            "\u{feff}{\"Nickname\": \"BomPlayer\", \"RamMb\": 4096, \"SelectedVersion\": \"1.20.1\"}";
        fs::write(aura_path.join("config.json"), config_str.as_bytes()).unwrap();

        let imported = import_from_dir(aura_path).unwrap();
        assert_eq!(imported.nickname, "BomPlayer");
        assert_eq!(imported.ram_mb, 4096);
        assert_eq!(imported.selected_version, "1.20.1");
    }
}
