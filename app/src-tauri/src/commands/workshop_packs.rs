use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;
use wex_core::config::WexPaths;

use super::launch::PlayState;

// ============================================================================
// DTOs for Resource Packs and Shaders (SPEC 1.7)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourcePackItemDto {
    pub name: String,
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub description: Option<String>,
    pub pack_format: Option<u32>,
    pub icon_data_url: Option<String>,
    pub is_enabled: bool,
    pub is_from_pack: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShaderPackItemDto {
    pub name: String,
    pub file_name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub description: Option<String>,
    pub icon_data_url: Option<String>,
    pub is_active: bool,
    pub is_from_pack: bool,
}

fn resolve_game_dir(paths: &WexPaths, version: Option<&str>) -> PathBuf {
    let ver = version.unwrap_or("1.21.1");
    paths.instance_dir(ver)
}

// Pure Rust standard RFC 4648 Base64 encoder (zero extra dependencies)
fn base64_encode(bytes: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARSET[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn parse_pack_mcmeta(bytes: &[u8]) -> (Option<String>, Option<u32>) {
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(bytes) {
        let pack = val.get("pack");
        let format = pack
            .and_then(|p| p.get("pack_format"))
            .and_then(|f| f.as_u64())
            .map(|n| n as u32);
        let desc = pack.and_then(|p| p.get("description")).and_then(|d| {
            if let Some(s) = d.as_str() {
                Some(s.to_string())
            } else if let Some(obj) = d.as_object() {
                obj.get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            } else if let Some(arr) = d.as_array() {
                let texts: Vec<&str> = arr
                    .iter()
                    .filter_map(|item| {
                        item.as_str()
                            .or_else(|| item.get("text").and_then(|t| t.as_str()))
                    })
                    .collect();
                if !texts.is_empty() {
                    Some(texts.join(" "))
                } else {
                    None
                }
            } else {
                None
            }
        });
        (desc, format)
    } else {
        (None, None)
    }
}

fn read_pack_meta_from_dir(dir: &Path) -> (Option<String>, Option<u32>, Option<String>) {
    let mcmeta_path = dir.join("pack.mcmeta");
    let (desc, format) = if mcmeta_path.is_file() {
        parse_pack_mcmeta(&fs::read(&mcmeta_path).unwrap_or_default())
    } else {
        (None, None)
    };

    let png_path = dir.join("pack.png");
    let icon_url = if png_path.is_file() {
        fs::read(&png_path)
            .ok()
            .map(|b| format!("data:image/png;base64,{}", base64_encode(&b)))
    } else {
        None
    };

    (desc, format, icon_url)
}

fn read_pack_meta_from_zip(zip_path: &Path) -> (Option<String>, Option<u32>, Option<String>) {
    let file = match fs::File::open(zip_path) {
        Ok(f) => f,
        Err(_) => return (None, None, None),
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return (None, None, None),
    };

    let mut desc = None;
    let mut format = None;
    let mut icon_url = None;

    if let Ok(mut mcmeta_file) = archive.by_name("pack.mcmeta") {
        let mut buf = Vec::new();
        if mcmeta_file.read_to_end(&mut buf).is_ok() {
            let (d, f) = parse_pack_mcmeta(&buf);
            desc = d;
            format = f;
        }
    }

    if let Ok(mut png_file) = archive.by_name("pack.png") {
        let mut buf = Vec::new();
        if png_file.read_to_end(&mut buf).is_ok() {
            icon_url = Some(format!("data:image/png;base64,{}", base64_encode(&buf)));
        }
    }

    (desc, format, icon_url)
}

fn is_file_from_pack(file_name: &str, folder_type: &str, instance_dir: &Path) -> bool {
    let lower_name = file_name.to_lowercase();
    if lower_name.starts_with("aura") || lower_name.contains("aura-pack") {
        return true;
    }

    let manifest_paths = [
        instance_dir.join("manifest.json"),
        instance_dir.join("manifest-1.21.1.json"),
        instance_dir.join("pack.state.json"),
    ];

    for mp in &manifest_paths {
        if mp.is_file() {
            if let Ok(text) = fs::read_to_string(mp) {
                let target = format!("{}/{}", folder_type, file_name);
                if text.contains(&target) || text.contains(file_name) {
                    return true;
                }
            }
        }
    }

    false
}

fn get_enabled_resource_packs(options_txt_path: &Path) -> Vec<String> {
    if !options_txt_path.is_file() {
        return Vec::new();
    }
    let content = fs::read_to_string(options_txt_path).unwrap_or_default();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("resourcePacks:") {
            if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str(rest) {
                return arr
                    .into_iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }
    }
    Vec::new()
}

fn set_enabled_resource_pack(
    options_txt_path: &Path,
    file_name: &str,
    enable: bool,
) -> std::io::Result<()> {
    let mut lines: Vec<String> = if options_txt_path.is_file() {
        fs::read_to_string(options_txt_path)?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let target_entry = format!("file/{}", file_name);
    let mut found = false;

    for line in &mut lines {
        if line.starts_with("resourcePacks:") {
            found = true;
            let rest = &line["resourcePacks:".len()..];
            let mut list: Vec<String> =
                serde_json::from_str(rest).unwrap_or_else(|_| vec!["vanilla".to_string()]);

            if enable {
                if !list
                    .iter()
                    .any(|item| item == &target_entry || item == file_name)
                {
                    list.push(target_entry.clone());
                }
            } else {
                list.retain(|item| item != &target_entry && item != file_name);
            }

            *line = format!(
                "resourcePacks:{}",
                serde_json::to_string(&list).unwrap_or_default()
            );
            break;
        }
    }

    if !found {
        let list = if enable {
            vec!["vanilla".to_string(), target_entry]
        } else {
            vec!["vanilla".to_string()]
        };
        lines.push(format!(
            "resourcePacks:{}",
            serde_json::to_string(&list).unwrap_or_default()
        ));
    }

    if let Some(p) = options_txt_path.parent() {
        let _ = fs::create_dir_all(p);
    }
    fs::write(options_txt_path, lines.join("\n"))?;
    Ok(())
}

fn get_active_shader(instance_dir: &Path) -> Option<String> {
    let iris_file = instance_dir.join("config").join("iris.properties");
    if iris_file.is_file() {
        if let Ok(content) = fs::read_to_string(&iris_file) {
            let mut enabled = true;
            let mut pack_name: Option<String> = None;
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(val) = trimmed.strip_prefix("enableShaders=") {
                    enabled = val.trim().eq_ignore_ascii_case("true");
                } else if let Some(val) = trimmed.strip_prefix("shaderPack=") {
                    let v = val.trim();
                    if !v.is_empty()
                        && !v.eq_ignore_ascii_case("OFF")
                        && !v.eq_ignore_ascii_case("(internal)")
                    {
                        pack_name = Some(v.to_string());
                    }
                }
            }
            if enabled {
                return pack_name;
            } else {
                return None;
            }
        }
    }
    None
}

fn set_active_shader(instance_dir: &Path, shader_name: Option<&str>) -> std::io::Result<()> {
    let config_dir = instance_dir.join("config");
    fs::create_dir_all(&config_dir)?;
    let iris_file = config_dir.join("iris.properties");

    let (enable, pack_val) = match shader_name {
        Some(s) if !s.is_empty() && !s.eq_ignore_ascii_case("OFF") => (true, s),
        _ => (false, "OFF"),
    };

    let iris_content = format!("enableShaders={}\nshaderPack={}\n", enable, pack_val);
    fs::write(&iris_file, iris_content)?;

    let opt_shaders = instance_dir.join("optionsshaders.txt");
    let opt_content = format!("currentShaderPacks={}\nshaderPack={}\n", pack_val, pack_val);
    let _ = fs::write(opt_shaders, opt_content);

    Ok(())
}

fn calculate_path_size(p: &Path) -> u64 {
    if p.is_file() {
        p.metadata().map(|m| m.len()).unwrap_or(0)
    } else if p.is_dir() {
        let mut total = 0;
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries.flatten() {
                total += calculate_path_size(&entry.path());
            }
        }
        total
    } else {
        0
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// List resource packs from the instance directory.
#[tauri::command]
pub async fn workshop_list_resource_packs(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<ResourcePackItemDto>, String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let packs_dir = game_dir.join("resourcepacks");
    if !packs_dir.is_dir() {
        return Ok(Vec::new());
    }

    let options_txt = game_dir.join("options.txt");
    let enabled_list = get_enabled_resource_packs(&options_txt);

    let mut items = Vec::new();
    let entries = fs::read_dir(&packs_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        let fname = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if fname.starts_with('.') {
            continue;
        }

        let is_zip = path.is_file()
            && path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false);
        let is_dir = path.is_dir();

        if !is_zip && !is_dir {
            continue;
        }

        let size_bytes = calculate_path_size(&path);
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let (description, pack_format, icon_data_url) = if is_zip {
            read_pack_meta_from_zip(&path)
        } else {
            read_pack_meta_from_dir(&path)
        };

        let target_prefix = format!("file/{}", fname);
        let is_enabled = enabled_list
            .iter()
            .any(|e| e == &target_prefix || e == &fname);
        let is_from_pack = is_file_from_pack(&fname, "resourcepacks", &game_dir);

        items.push(ResourcePackItemDto {
            name,
            file_name: fname,
            file_path: path.to_string_lossy().to_string(),
            size_bytes,
            description,
            pack_format,
            icon_data_url,
            is_enabled,
            is_from_pack,
        });
    }

    // Sort: enabled first, then alphabetically
    items.sort_by(|a, b| match (a.is_enabled, b.is_enabled) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(items)
}

/// Enable or disable a resource pack in options.txt.
#[tauri::command]
pub async fn workshop_toggle_resource_pack(
    file_name: String,
    enabled: bool,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let options_txt = game_dir.join("options.txt");
    set_enabled_resource_pack(&options_txt, &file_name, enabled).map_err(|e| e.to_string())
}

/// Set complete ordered list of enabled resource packs in options.txt.
#[tauri::command]
pub async fn workshop_set_resource_packs_order(
    pack_names: Vec<String>,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let options_txt = game_dir.join("options.txt");

    let mut lines: Vec<String> = if options_txt.is_file() {
        fs::read_to_string(&options_txt)
            .map_err(|e| e.to_string())?
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    };

    let mut new_list = vec!["vanilla".to_string()];
    for f in &pack_names {
        if f != "vanilla" {
            let entry = if f.starts_with("file/") {
                f.clone()
            } else {
                format!("file/{}", f)
            };
            if !new_list.contains(&entry) {
                new_list.push(entry);
            }
        }
    }

    let mut found = false;
    for line in &mut lines {
        if line.starts_with("resourcePacks:") {
            found = true;
            *line = format!(
                "resourcePacks:{}",
                serde_json::to_string(&new_list).unwrap_or_default()
            );
            break;
        }
    }

    if !found {
        lines.push(format!(
            "resourcePacks:{}",
            serde_json::to_string(&new_list).unwrap_or_default()
        ));
    }

    if let Some(p) = options_txt.parent() {
        let _ = fs::create_dir_all(p);
    }
    fs::write(&options_txt, lines.join("\n")).map_err(|e| e.to_string())?;

    Ok(())
}

/// List shader packs from the instance directory.
#[tauri::command]
pub async fn workshop_list_shader_packs(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<ShaderPackItemDto>, String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let shaders_dir = game_dir.join("shaderpacks");
    if !shaders_dir.is_dir() {
        return Ok(Vec::new());
    }

    let active_shader = get_active_shader(&game_dir);
    let mut items = Vec::new();
    let entries = fs::read_dir(&shaders_dir).map_err(|e| e.to_string())?;

    for entry in entries.flatten() {
        let path = entry.path();
        let fname = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if fname.starts_with('.') || fname.ends_with(".txt") {
            continue;
        }

        let is_zip = path.is_file()
            && path
                .extension()
                .map(|e| e.eq_ignore_ascii_case("zip"))
                .unwrap_or(false);
        let is_dir = path.is_dir();

        if !is_zip && !is_dir {
            continue;
        }

        let size_bytes = calculate_path_size(&path);
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let (description, _, icon_data_url) = if is_zip {
            read_pack_meta_from_zip(&path)
        } else {
            read_pack_meta_from_dir(&path)
        };

        let is_active =
            active_shader.as_deref() == Some(&fname) || active_shader.as_deref() == Some(&name);
        let is_from_pack = is_file_from_pack(&fname, "shaderpacks", &game_dir);

        items.push(ShaderPackItemDto {
            name,
            file_name: fname,
            file_path: path.to_string_lossy().to_string(),
            size_bytes,
            description,
            icon_data_url,
            is_active,
            is_from_pack,
        });
    }

    items.sort_by(|a, b| match (a.is_active, b.is_active) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(items)
}

/// Set active shader in config/iris.properties (None or "OFF" disables shaders).
#[tauri::command]
pub async fn workshop_set_active_shader_pack(
    shader_file_name: Option<String>,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    set_active_shader(&game_dir, shader_file_name.as_deref()).map_err(|e| e.to_string())
}

pub fn add_pack_file_impl(
    paths: &WexPaths,
    source_path: &str,
    pack_type: &str,
    version: Option<&str>,
) -> Result<String, String> {
    let src = PathBuf::from(source_path);
    if !src.is_file() {
        return Err("Выбранный файл не найден на диске. Выберите файл заново.".to_string());
    }

    let file_name = src
        .file_name()
        .ok_or_else(|| "Недопустимое имя файла".to_string())?
        .to_string_lossy()
        .to_string();

    let game_dir = resolve_game_dir(paths, version);
    let dest_dir = if pack_type == "shaderpack" {
        game_dir.join("shaderpacks")
    } else {
        game_dir.join("resourcepacks")
    };

    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let dest_file = dest_dir.join(&file_name);
    fs::copy(&src, &dest_file).map_err(|e| e.to_string())?;

    Ok(file_name)
}

/// Add a new resource pack or shader pack ZIP file into the instance directory.
#[tauri::command]
pub async fn workshop_add_pack_file(
    source_path: String,
    pack_type: String,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    add_pack_file_impl(&state.paths, &source_path, &pack_type, version.as_deref())
}

pub fn delete_pack_file_impl(
    paths: &WexPaths,
    file_name: &str,
    pack_type: &str,
    version: Option<&str>,
) -> Result<(), String> {
    let game_dir = resolve_game_dir(paths, version);
    let folder_name = if pack_type == "shaderpack" {
        "shaderpacks"
    } else {
        "resourcepacks"
    };

    if is_file_from_pack(file_name, folder_name, &game_dir) {
        return Err("Файлы из официальной сборки защищены от удаления. Вы можете отключить их в настройках.".to_string());
    }

    let target = game_dir.join(folder_name).join(file_name);
    if !target.exists() {
        return Err("Файл не найден. Обновите список файлов.".to_string());
    }

    if target.is_dir() {
        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(&target).map_err(|e| e.to_string())?;
    }

    // Disable if it was enabled
    if pack_type == "shaderpack" {
        let active = get_active_shader(&game_dir);
        if active.as_deref() == Some(file_name) {
            let _ = set_active_shader(&game_dir, None);
        }
    } else {
        let options_txt = game_dir.join("options.txt");
        let _ = set_enabled_resource_pack(&options_txt, file_name, false);
    }

    Ok(())
}

/// Delete a user-added resource pack or shader pack (packs from official pack are protected).
#[tauri::command]
pub async fn workshop_delete_pack_file(
    file_name: String,
    pack_type: String,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    delete_pack_file_impl(&state.paths, &file_name, &pack_type, version.as_deref())
}

/// Reveal resourcepacks or shaderpacks folder in Explorer/Finder.
#[tauri::command]
pub async fn workshop_open_packs_folder(
    pack_type: String,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let folder = if pack_type == "shaderpack" {
        game_dir.join("shaderpacks")
    } else {
        game_dir.join("resourcepacks")
    };

    if !folder.exists() {
        let _ = fs::create_dir_all(&folder);
    }

    super::workshop::workshop_open_path(folder.to_string_lossy().to_string()).await
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::tempdir;
    use wex_ipc::core_trait::MockLauncherCore;
    use zip::write::SimpleFileOptions;

    fn create_dummy_zip_pack(zip_path: &Path, desc: &str, format: u32) {
        let file = fs::File::create(zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("pack.mcmeta", options).unwrap();
        let mcmeta_json = format!(
            r#"{{"pack":{{"description":"{}","pack_format":{}}}}}"#,
            desc, format
        );
        use std::io::Write;
        zip.write_all(mcmeta_json.as_bytes()).unwrap();

        zip.start_file("pack.png", options).unwrap();
        zip.write_all(b"fake_png_bytes").unwrap();

        zip.finish().unwrap();
    }

    #[tokio::test]
    async fn test_resource_packs_listing_and_toggle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths.clone(), mock_core);

        let inst_dir = paths.instance_dir("1.21.1");
        let rp_dir = inst_dir.join("resourcepacks");
        fs::create_dir_all(&rp_dir).unwrap();

        // 1. Initially empty
        let options_txt = inst_dir.join("options.txt");
        assert!(get_enabled_resource_packs(&options_txt).is_empty());

        // 2. Add packs: one custom, one Aura-Pack file
        let custom_pack = rp_dir.join("Faithful.zip");
        create_dummy_zip_pack(&custom_pack, "Faithful 64x HD Pack", 15);

        let aura_pack = rp_dir.join("Aura-ResourcePack.zip");
        create_dummy_zip_pack(&aura_pack, "Официальные текстуры Aura", 15);

        // Enable Faithful in options.txt
        set_enabled_resource_pack(&options_txt, "Faithful.zip", true).unwrap();
        let enabled = get_enabled_resource_packs(&options_txt);
        assert!(enabled.contains(&"file/Faithful.zip".to_string()));

        // Check is_from_pack detection
        assert!(!is_file_from_pack(
            "Faithful.zip",
            "resourcepacks",
            &inst_dir
        ));
        assert!(is_file_from_pack(
            "Aura-ResourcePack.zip",
            "resourcepacks",
            &inst_dir
        ));

        // Disable Faithful
        set_enabled_resource_pack(&options_txt, "Faithful.zip", false).unwrap();
        let enabled_after = get_enabled_resource_packs(&options_txt);
        assert!(!enabled_after.contains(&"file/Faithful.zip".to_string()));
    }

    #[tokio::test]
    async fn test_shader_packs_lifecycle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths.clone(), mock_core);

        let inst_dir = paths.instance_dir("1.21.1");
        let shaders_dir = inst_dir.join("shaderpacks");
        fs::create_dir_all(&shaders_dir).unwrap();

        // Initially no active shader
        assert_eq!(get_active_shader(&inst_dir), None);

        // Create a dummy shader pack
        let shader_file = shaders_dir.join("Complementary.zip");
        create_dummy_zip_pack(&shader_file, "Complementary Shaders", 15);

        // Activate Complementary
        set_active_shader(&inst_dir, Some("Complementary.zip")).unwrap();
        assert_eq!(
            get_active_shader(&inst_dir),
            Some("Complementary.zip".to_string())
        );

        let iris = fs::read_to_string(inst_dir.join("config").join("iris.properties")).unwrap();
        assert!(iris.contains("enableShaders=true"));
        assert!(iris.contains("shaderPack=Complementary.zip"));

        // Disable shaders
        set_active_shader(&inst_dir, None).unwrap();
        assert_eq!(get_active_shader(&inst_dir), None);
        let iris_off = fs::read_to_string(inst_dir.join("config").join("iris.properties")).unwrap();
        assert!(iris_off.contains("enableShaders=false"));
        assert!(iris_off.contains("shaderPack=OFF"));
    }

    #[tokio::test]
    async fn test_add_and_delete_protection() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths.clone(), mock_core);

        let inst_dir = paths.instance_dir("1.21.1");
        let rp_dir = inst_dir.join("resourcepacks");
        fs::create_dir_all(&rp_dir).unwrap();

        // Create an outside zip file to import
        let src_zip = tmp.path().join("ImportMe.zip");
        create_dummy_zip_pack(&src_zip, "Imported Pack", 15);

        // Add pack
        let added_name = add_pack_file_impl(
            &paths,
            &src_zip.to_string_lossy(),
            "resourcepack",
            Some("1.21.1"),
        )
        .unwrap();

        assert_eq!(added_name, "ImportMe.zip");
        assert!(rp_dir.join("ImportMe.zip").is_file());

        // Attempting to delete an Aura-Pack file must fail
        let aura_file = rp_dir.join("Aura-Texture.zip");
        fs::write(&aura_file, b"aura").unwrap();

        let del_aura_res =
            delete_pack_file_impl(&paths, "Aura-Texture.zip", "resourcepack", Some("1.21.1"));
        assert!(del_aura_res.is_err());
        assert!(aura_file.is_file());

        // Deleting user pack succeeds
        let del_user_res =
            delete_pack_file_impl(&paths, "ImportMe.zip", "resourcepack", Some("1.21.1"));
        assert!(del_user_res.is_ok());
        assert!(!rp_dir.join("ImportMe.zip").exists());
    }
}
