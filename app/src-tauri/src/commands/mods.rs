use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

use crate::commands::launch::PlayState;
use wex_core::config::WexPaths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub file_name: String,
    pub enabled: bool,
    pub is_pack_mod: bool,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FabricModJson {
    pub id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    #[allow(dead_code)]
    pub icon: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct PackStateJson {
    #[serde(default)]
    pub installed_mods: Vec<String>,
}

/// Fallback parser that extracts mod name and version from filename.
fn parse_filename_fallback(file_name: &str) -> (String, String) {
    let clean = file_name
        .strip_suffix(".disabled")
        .unwrap_or(file_name)
        .strip_suffix(".jar")
        .unwrap_or(file_name);

    if let Some(pos) = clean.find('-') {
        let name = &clean[..pos];
        let ver = &clean[pos + 1..];
        (name.replace('_', " "), ver.to_string())
    } else {
        (clean.replace('_', " "), "unknown".to_string())
    }
}

/// Extracts metadata from a Fabric mod JAR file.
pub fn extract_mod_metadata(file_path: &Path) -> (String, String, String, String, Option<String>) {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.jar");

    let (fb_name, fb_ver) = parse_filename_fallback(file_name);

    if let Ok(file) = File::open(file_path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            if let Ok(mut mod_json_file) = archive.by_name("fabric.mod.json") {
                let mut content = String::new();
                if mod_json_file.read_to_string(&mut content).is_ok() {
                    if let Ok(fabric_json) = serde_json::from_str::<FabricModJson>(&content) {
                        let id = fabric_json.id.unwrap_or_else(|| fb_name.to_lowercase());
                        let name = fabric_json.name.unwrap_or_else(|| fb_name.clone());
                        let version = fabric_json.version.unwrap_or(fb_ver);
                        let description = fabric_json
                            .description
                            .unwrap_or_else(|| "Мод Fabric для Wex Client".to_string());

                        return (id, name, version, description, None);
                    }
                }
            }
        }
    }

    (
        fb_name.to_lowercase().replace(' ', "-"),
        fb_name,
        fb_ver,
        "Мод Fabric для Wex Client".to_string(),
        None,
    )
}

/// Collects pack mod filenames from pack.state.json or manifest files.
pub fn get_pack_mod_names(instance_dir: &Path, paths: &WexPaths, version: &str) -> HashSet<String> {
    let mut names = HashSet::new();

    // 1. Check instance-level pack.state.json
    let state_file = instance_dir.join("pack.state.json");
    if let Ok(content) = fs::read_to_string(&state_file) {
        if let Ok(state) = serde_json::from_str::<PackStateJson>(&content) {
            for m in state.installed_mods {
                let clean = m.strip_suffix(".disabled").unwrap_or(&m).to_lowercase();
                names.insert(clean);
            }
        }
    }

    // 2. Check root or instance manifest files
    let manifest_candidates = [
        paths.root.join("manifest.json"),
        paths.root.join(format!("manifest-{}.json", version)),
        instance_dir.join("manifest.json"),
    ];

    for manifest_path in &manifest_candidates {
        if let Ok(content) = fs::read_to_string(manifest_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(files) = json.get("files").and_then(|f| f.as_array()) {
                    for f in files {
                        if let Some(path_str) = f.get("path").and_then(|p| p.as_str()) {
                            if path_str.starts_with("mods/") && path_str.ends_with(".jar") {
                                if let Some(file_name) =
                                    Path::new(path_str).file_name().and_then(|n| n.to_str())
                                {
                                    names.insert(file_name.to_lowercase());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Known core required mods for Wex Client and Aura Pack
    let default_core_mods = [
        "wex-client",
        "aura-client",
        "fabric-api",
        "sodium",
        "iris",
        "modmenu",
        "wex-defaults",
        "aura-defaults",
    ];
    for core in default_core_mods {
        names.insert(core.to_string());
    }

    names
}

pub fn list_mods_impl(paths: &WexPaths, version: &str) -> Result<Vec<ModEntry>, String> {
    let inst_dir = paths.instance_dir(version);
    let mods_dir = inst_dir.join("mods");
    let _ = fs::create_dir_all(&mods_dir);

    let pack_mods = get_pack_mod_names(&inst_dir, paths, version);
    let mut result = Vec::new();

    let entries = fs::read_dir(&mods_dir).map_err(|e| {
        format!(
            "Не удалось прочитать папку модов {}: {}",
            mods_dir.display(),
            e
        )
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let enabled = if file_name.ends_with(".jar") {
            true
        } else if file_name.ends_with(".jar.disabled") {
            false
        } else {
            continue;
        };

        let base_file_name = file_name
            .strip_suffix(".disabled")
            .unwrap_or(&file_name)
            .to_lowercase();

        let (id, name, mod_version, description, icon) = extract_mod_metadata(&path);

        let is_pack_mod = pack_mods.contains(&base_file_name)
            || pack_mods
                .iter()
                .any(|p| base_file_name.contains(p) || id.to_lowercase().contains(p));

        result.push(ModEntry {
            id,
            name,
            version: mod_version,
            description,
            file_name,
            enabled,
            is_pack_mod,
            icon,
        });
    }

    // Sort: pack/core mods first, then alphabetically by name
    result.sort_by(|a, b| {
        b.is_pack_mod
            .cmp(&a.is_pack_mod)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(result)
}

pub fn toggle_mod_impl(
    paths: &WexPaths,
    version: &str,
    file_name: &str,
    enabled: bool,
) -> Result<ModEntry, String> {
    let inst_dir = paths.instance_dir(version);
    let mods_dir = inst_dir.join("mods");
    let current_path = mods_dir.join(file_name);

    if !current_path.exists() {
        return Err(format!("Файл мода не найден: {}", file_name));
    }

    let target_name = if enabled {
        file_name
            .strip_suffix(".disabled")
            .ok_or_else(|| "Мод уже включен".to_string())?
            .to_string()
    } else {
        if file_name.ends_with(".disabled") {
            return Err("Мод уже выключен".to_string());
        }
        format!("{}.disabled", file_name)
    };

    let target_path = mods_dir.join(&target_name);

    fs::rename(&current_path, &target_path)
        .map_err(|e| format!("Не удалось переименовать файл мода: {}", e))?;

    let pack_mods = get_pack_mod_names(&inst_dir, paths, version);
    let base_file_name = target_name
        .strip_suffix(".disabled")
        .unwrap_or(&target_name)
        .to_lowercase();

    let (id, name, mod_version, description, icon) = extract_mod_metadata(&target_path);
    let is_pack_mod = pack_mods.contains(&base_file_name)
        || pack_mods
            .iter()
            .any(|p| base_file_name.contains(p) || id.to_lowercase().contains(p));

    Ok(ModEntry {
        id,
        name,
        version: mod_version,
        description,
        file_name: target_name,
        enabled,
        is_pack_mod,
        icon,
    })
}

pub fn add_custom_mod_impl(
    paths: &WexPaths,
    version: &str,
    source_path: &str,
) -> Result<ModEntry, String> {
    let src = PathBuf::from(source_path);
    if !src.is_file() {
        return Err(format!("Исходный файл не найден: {}", source_path));
    }

    let file_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Недопустимое имя файла".to_string())?;

    if !file_name.ends_with(".jar") {
        return Err("Поддерживаются только файлы модов с расширением .jar".to_string());
    }

    let mods_dir = paths.instance_dir(version).join("mods");
    let _ = fs::create_dir_all(&mods_dir);
    let target = mods_dir.join(file_name);

    fs::copy(&src, &target).map_err(|e| format!("Не удалось скопировать мод в инстанс: {}", e))?;

    let (id, name, mod_version, description, icon) = extract_mod_metadata(&target);

    Ok(ModEntry {
        id,
        name,
        version: mod_version,
        description,
        file_name: file_name.to_string(),
        enabled: true,
        is_pack_mod: false,
        icon,
    })
}

pub fn delete_custom_mod_impl(
    paths: &WexPaths,
    version: &str,
    file_name: &str,
) -> Result<(), String> {
    // Prevent directory traversal
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err("Недопустимое имя файла мода".to_string());
    }

    let inst_dir = paths.instance_dir(version);
    let mods_dir = inst_dir.join("mods");
    let target_path = mods_dir.join(file_name);

    if !target_path.exists() {
        return Err(format!("Файл мода не найден: {}", file_name));
    }

    let pack_mods = get_pack_mod_names(&inst_dir, paths, version);
    let base_file_name = file_name
        .strip_suffix(".disabled")
        .unwrap_or(file_name)
        .to_lowercase();

    let (id, _, _, _, _) = extract_mod_metadata(&target_path);
    let is_pack_mod = pack_mods.contains(&base_file_name)
        || pack_mods
            .iter()
            .any(|p| base_file_name.contains(p) || id.to_lowercase().contains(p));

    if is_pack_mod {
        return Err("Моды из официального пака Wex защищены от удаления".to_string());
    }

    fs::remove_file(&target_path).map_err(|e| format!("Не удалось удалить файл мода: {}", e))?;

    Ok(())
}

pub fn open_mods_dir_impl(paths: &WexPaths, version: &str) -> Result<(), String> {
    let mods_dir = paths.instance_dir(version).join("mods");
    if !mods_dir.exists() {
        let _ = fs::create_dir_all(&mods_dir);
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&mods_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&mods_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&mods_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn list_mods(
    version: String,
    state: State<'_, PlayState>,
) -> Result<Vec<ModEntry>, String> {
    list_mods_impl(&state.paths, &version)
}

#[tauri::command]
pub async fn toggle_mod(
    version: String,
    file_name: String,
    enabled: bool,
    state: State<'_, PlayState>,
) -> Result<ModEntry, String> {
    toggle_mod_impl(&state.paths, &version, &file_name, enabled)
}

#[tauri::command]
pub async fn add_custom_mod(
    version: String,
    source_path: String,
    state: State<'_, PlayState>,
) -> Result<ModEntry, String> {
    add_custom_mod_impl(&state.paths, &version, &source_path)
}

#[tauri::command]
pub async fn delete_custom_mod(
    version: String,
    file_name: String,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    delete_custom_mod_impl(&state.paths, &version, &file_name)
}

#[tauri::command]
pub async fn open_mods_dir(version: String, state: State<'_, PlayState>) -> Result<(), String> {
    open_mods_dir_impl(&state.paths, &version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_dummy_jar(path: &Path, mod_id: &str, mod_name: &str, mod_ver: &str) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file("fabric.mod.json", options).unwrap();

        let json = format!(
            r#"{{"id":"{}","name":"{}","version":"{}","description":"Test Fabric Mod"}}"#,
            mod_id, mod_name, mod_ver
        );
        zip.write_all(json.as_bytes()).unwrap();
        zip.finish().unwrap();
    }

    #[tokio::test]
    async fn test_mods_lifecycle_add_toggle_delete() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        paths.ensure_instance_dirs("1.21.1").unwrap();

        // 1. Initial list empty
        let mods = list_mods_impl(&paths, "1.21.1").unwrap();
        assert_eq!(mods.len(), 0);

        // 2. Create custom external jar and add it
        let custom_jar = tmp.path().join("my-custom-mod-1.0.jar");
        create_dummy_jar(&custom_jar, "custom_mod", "My Custom Mod", "1.0.0");

        let added = add_custom_mod_impl(&paths, "1.21.1", &custom_jar.to_string_lossy()).unwrap();

        assert_eq!(added.id, "custom_mod");
        assert_eq!(added.name, "My Custom Mod");
        assert!(added.enabled);
        assert!(!added.is_pack_mod);

        // 3. Verify in list
        let mods_after = list_mods_impl(&paths, "1.21.1").unwrap();
        assert_eq!(mods_after.len(), 1);
        assert_eq!(mods_after[0].name, "My Custom Mod");

        // 4. Toggle disable
        let disabled = toggle_mod_impl(&paths, "1.21.1", "my-custom-mod-1.0.jar", false).unwrap();
        assert!(!disabled.enabled);
        assert_eq!(disabled.file_name, "my-custom-mod-1.0.jar.disabled");

        // 5. Toggle re-enable
        let re_enabled =
            toggle_mod_impl(&paths, "1.21.1", "my-custom-mod-1.0.jar.disabled", true).unwrap();
        assert!(re_enabled.enabled);
        assert_eq!(re_enabled.file_name, "my-custom-mod-1.0.jar");

        // 6. Delete custom mod
        delete_custom_mod_impl(&paths, "1.21.1", "my-custom-mod-1.0.jar").unwrap();

        let mods_final = list_mods_impl(&paths, "1.21.1").unwrap();
        assert_eq!(mods_final.len(), 0);
    }

    #[tokio::test]
    async fn test_pack_mod_cannot_be_deleted() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();
        paths.ensure_instance_dirs("1.21.1").unwrap();

        let inst_dir = paths.instance_dir("1.21.1");
        let mods_dir = inst_dir.join("mods");

        // Place a core mod (e.g. sodium) into mods folder
        let sodium_jar = mods_dir.join("sodium-fabric-0.6.9.jar");
        create_dummy_jar(&sodium_jar, "sodium", "Sodium", "0.6.9");

        // Save pack.state.json designating sodium as pack mod
        let pack_state = r#"{"pack_version":"20261108.1","minecraft":"1.21.1","installed_mods":["sodium-fabric-0.6.9.jar"]}"#;
        fs::write(inst_dir.join("pack.state.json"), pack_state).unwrap();

        // Verify listed as is_pack_mod = true
        let mods = list_mods_impl(&paths, "1.21.1").unwrap();
        assert_eq!(mods.len(), 1);
        assert!(mods[0].is_pack_mod);

        // Attempt deletion must return Err
        let res = delete_custom_mod_impl(&paths, "1.21.1", "sodium-fabric-0.6.9.jar");

        assert!(res.is_err());
        assert!(res.unwrap_err().contains("защищены от удаления"));
    }
}
