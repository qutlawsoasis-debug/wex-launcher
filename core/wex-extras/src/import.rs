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
    })
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
        let config_str = format!(
            "\u{feff}{{\"Nickname\": \"BomPlayer\", \"RamMb\": 4096, \"SelectedVersion\": \"1.20.1\"}}"
        );
        fs::write(aura_path.join("config.json"), config_str.as_bytes()).unwrap();

        let imported = import_from_dir(aura_path).unwrap();
        assert_eq!(imported.nickname, "BomPlayer");
        assert_eq!(imported.ram_mb, 4096);
        assert_eq!(imported.selected_version, "1.20.1");
    }
}
