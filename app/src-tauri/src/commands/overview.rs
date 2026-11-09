use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

use wex_core::config::WexPaths;
use wex_ipc::commands::{
    ExtrasImportOldLauncherRequest, SocialCheckLauncherUpdateRequest, SocialGetNewsRequest,
    SocialGetNewsResponse,
};
use wex_ipc::core_trait::LauncherCore;
use wex_ipc::models::{
    ImportedCapeDto, ImportedServerDto, ImportedSkinDto, ImportedWorldDto, OldLauncherDataDto,
    ReleaseNoteVersionDto, UpdateCheckResultDto,
};
use wex_social::launcher_update::{check_update_from_url, DEFAULT_TARGET_PLATFORM};
use wex_social::NewsManager;

use super::launch::PlayState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyImportRequest {
    pub old_launcher_dir: String,
    #[serde(default)]
    pub import_nickname: bool,
    pub nickname: Option<String>,
    #[serde(default)]
    pub import_ram: bool,
    pub ram_mb: Option<u32>,
    #[serde(default)]
    pub import_version: bool,
    pub selected_version: Option<String>,
    #[serde(default)]
    pub graphics_preset: Option<String>,
    #[serde(default)]
    pub import_servers: bool,
    #[serde(default)]
    pub servers: Vec<ImportedServerDto>,
    #[serde(default)]
    pub import_skins: bool,
    #[serde(default)]
    pub skins: Vec<ImportedSkinDto>,
    #[serde(default)]
    pub capes: Vec<ImportedCapeDto>,
    #[serde(default)]
    pub import_worlds: bool,
    #[serde(default)]
    pub worlds: Vec<ImportedWorldDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyImportResponse {
    pub success: bool,
    pub imported_nickname: Option<String>,
    pub imported_ram_gb: Option<u32>,
    pub imported_servers_count: usize,
    pub imported_skins_count: usize,
    pub imported_worlds_count: usize,
    pub message: String,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlayTimeDto {
    pub total_seconds: u64,
    pub total_launches: u32,
    pub last_played_utc: Option<String>,
    pub formatted_play_time: String,
    pub summary_text: String,
}

pub fn format_play_time(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours} ч. {minutes} мин.")
    } else {
        format!("{minutes} мин.")
    }
}

pub fn get_playtime_internal(paths: &WexPaths) -> PlayTimeDto {
    let stats_file = paths.root.join("stats.json");
    if let Ok(bytes) = fs::read(&stats_file) {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            let total_seconds = val
                .get("total_play_time_seconds")
                .or_else(|| val.get("total_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let total_launches = val
                .get("total_game_launches")
                .or_else(|| val.get("total_launches"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32;
            let last_played_utc = val
                .get("last_played_utc")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let formatted = format_play_time(total_seconds);
            let summary = format!("В игре: {formatted}  •  Запусков: {total_launches}");
            return PlayTimeDto {
                total_seconds,
                total_launches,
                last_played_utc,
                formatted_play_time: formatted,
                summary_text: summary,
            };
        }
    }
    PlayTimeDto {
        total_seconds: 0,
        total_launches: 0,
        last_played_utc: None,
        formatted_play_time: "0 мин.".to_string(),
        summary_text: "Время в игре: 0 мин. • 0 запусков".to_string(),
    }
}

pub fn record_playtime_internal(
    paths: &WexPaths,
    added_seconds: u64,
    inc_launch: bool,
) -> PlayTimeDto {
    let stats_file = paths.root.join("stats.json");
    let mut current = get_playtime_internal(paths);
    current.total_seconds += added_seconds;
    if inc_launch {
        current.total_launches += 1;
    }
    current.last_played_utc = Some(chrono::Utc::now().to_rfc3339());
    current.formatted_play_time = format_play_time(current.total_seconds);
    current.summary_text = format!(
        "В игре: {}  •  Запусков: {}",
        current.formatted_play_time, current.total_launches
    );

    let val = serde_json::json!({
        "total_play_time_seconds": current.total_seconds,
        "total_game_launches": current.total_launches,
        "last_played_utc": current.last_played_utc,
    });
    if let Some(parent) = stats_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json_str) = serde_json::to_string_pretty(&val) {
        if let Some(parent) = stats_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&stats_file, json_str);
    }
    current
}

// ---------------------------------------------------------------------------
// Business Logic Functions (Pure, easily testable with mock core)
// ---------------------------------------------------------------------------

pub fn get_news_internal(
    paths: &WexPaths,
    req: Option<SocialGetNewsRequest>,
) -> Vec<ReleaseNoteVersionDto> {
    let cache_dir = req
        .as_ref()
        .map(|r| PathBuf::from(&r.cache_dir))
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| paths.root.join("news"));

    let mgr = NewsManager::new(cache_dir);
    let versions = mgr.load_cached_or_default();
    versions
        .into_iter()
        .map(ReleaseNoteVersionDto::from)
        .collect()
}

pub async fn check_update_internal(
    current_version: &str,
    manifest_url: Option<&str>,
    target_platform: Option<&str>,
) -> Result<UpdateCheckResultDto, String> {
    let cur = if current_version.trim().is_empty() {
        "1.0.0"
    } else {
        current_version.trim()
    };
    let plat = target_platform.unwrap_or(DEFAULT_TARGET_PLATFORM);

    match manifest_url {
        Some(url) if !url.trim().is_empty() => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .map_err(|e| e.to_string())?;

            match check_update_from_url(&client, cur, plat, url).await {
                Ok(res) => Ok(res.into()),
                Err(err) => Err(err.to_string()),
            }
        }
        _ => {
            // Unobtrusive default check: if no remote url configured, report current version is up to date
            Ok(UpdateCheckResultDto {
                update_available: false,
                current_version: cur.to_string(),
                latest_version: Some(cur.to_string()),
                notes: None,
                download_url: None,
                signature: None,
                pub_date: None,
            })
        }
    }
}

pub fn detect_old_launcher_internal() -> Option<String> {
    wex_extras::import::default_aura_dir()
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().to_string())
}

pub fn import_old_launcher_internal(old_dir: &str) -> Result<OldLauncherDataDto, String> {
    let p = Path::new(old_dir);
    if !p.is_dir() {
        return Err(format!(
            "Каталог старого лаунчера не найден: {}",
            p.display()
        ));
    }

    match wex_extras::import::import_from_dir(p) {
        Ok(data) => Ok(OldLauncherDataDto::from(data)),
        Err(err) => Err(err.to_string()),
    }
}

fn validate_safe_component(name: &str, field_label: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(format!("{}: имя не может быть пустым", field_label));
    }
    if trimmed.contains('\0') || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!(
            "{}: обнаружена попытка path traversal или недопустимые разделители пути в '{}'",
            field_label, name
        ));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(format!(
            "{}: недопустимое относительное имя '{}'",
            field_label, name
        ));
    }
    let p = Path::new(trimmed);
    let mut comps = p.components();
    match (comps.next(), comps.next()) {
        (Some(std::path::Component::Normal(c)), None) => {
            let s = c
                .to_str()
                .ok_or_else(|| format!("{}: имя содержит невалидный UTF-8", field_label))?;
            Ok(s.to_string())
        }
        _ => Err(format!(
            "{}: недопустимый компонент пути '{}'",
            field_label, name
        )),
    }
}

fn validate_supported_version(version: &str) -> Result<String, String> {
    let safe = validate_safe_component(version, "Версия игры")?;
    if !safe
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(format!("Неподдерживаемая версия игры: '{}'", version));
    }
    Ok(safe)
}

fn generate_unique_world_dest(base_dir: &Path, folder_name: &str) -> PathBuf {
    let initial = base_dir.join(folder_name);
    if !initial.exists() {
        return initial;
    }
    let mut index = 1;
    loop {
        let candidate_name = format!("{}-imported-{}", folder_name, index);
        let candidate_path = base_dir.join(&candidate_name);
        if !candidate_path.exists() {
            return candidate_path;
        }
        index += 1;
    }
}

fn generate_unique_file_dest(base_dir: &Path, file_name: &str) -> PathBuf {
    let initial = base_dir.join(file_name);
    if !initial.exists() {
        return initial;
    }
    let p = Path::new(file_name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut index = 1;
    loop {
        let candidate_name = if ext.is_empty() {
            format!("{}-imported-{}", stem, index)
        } else {
            format!("{}-imported-{}.{}", stem, index, ext)
        };
        let candidate_path = base_dir.join(&candidate_name);
        if !candidate_path.exists() {
            return candidate_path;
        }
        index += 1;
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

pub async fn apply_import_internal(
    paths: &WexPaths,
    core: &dyn LauncherCore,
    req: ApplyImportRequest,
) -> Result<ApplyImportResponse, String> {
    let mut imported_nickname = None;
    let mut imported_ram_gb = None;
    let mut imported_servers_count = 0;
    let mut imported_skins_count = 0;
    let mut imported_worlds_count = 0;
    let mut errors: Vec<String> = Vec::new();

    // 1. Update Wex launcher configuration if requested
    let mut cfg = core.get_config().await.map_err(|e| e.to_string())?;
    let mut cfg_modified = false;

    if req.import_nickname {
        if let Some(nick) = req.nickname.as_ref().filter(|n| !n.trim().is_empty()) {
            cfg.player_name = nick.trim().to_string();
            imported_nickname = Some(cfg.player_name.clone());
            cfg_modified = true;
        }
    }

    if req.import_ram {
        if let Some(ram_mb) = req.ram_mb.filter(|&m| m > 0) {
            let gb = (ram_mb + 512) / 1024;
            let final_gb = gb.clamp(2, 64);
            cfg.ram_gb = final_gb;
            imported_ram_gb = Some(final_gb);
            cfg_modified = true;
        }
    }

    if req.import_version {
        if let Some(ver) = req
            .selected_version
            .as_ref()
            .filter(|v| !v.trim().is_empty())
        {
            match validate_supported_version(ver) {
                Ok(safe_ver) => {
                    cfg.selected_version = safe_ver;
                    cfg_modified = true;
                }
                Err(err) => {
                    errors.push(err);
                }
            }
        }
    }

    if cfg_modified {
        core.save_config(cfg).await.map_err(|e| e.to_string())?;
    }

    // 1b. Update core config file directly to preserve graphics_preset and exact RAM
    let mut core_cfg = wex_core::config::LauncherConfig::load_or_default(&paths.config_file);
    let mut core_modified = false;
    if req.import_nickname {
        if let Some(nick) = req.nickname.as_ref().filter(|n| !n.trim().is_empty()) {
            core_cfg.nickname = nick.trim().to_string();
            core_modified = true;
        }
    }
    if req.import_ram {
        if let Some(ram_mb) = req.ram_mb.filter(|&m| m > 0) {
            core_cfg.max_memory_mb = ram_mb;
            core_modified = true;
        }
    }
    if req.import_version {
        if let Some(ver) = req
            .selected_version
            .as_ref()
            .filter(|v| !v.trim().is_empty())
        {
            if let Ok(safe_ver) = validate_supported_version(ver) {
                core_cfg.selected_version = safe_ver;
                core_modified = true;
            }
        }
    }
    if let Some(preset) = req
        .graphics_preset
        .as_ref()
        .filter(|p| !p.trim().is_empty())
    {
        core_cfg.graphics_preset = preset.trim().to_string();
        core_modified = true;
    }
    if core_modified {
        let _ = core_cfg.save(&paths.config_file);
    }

    // 2. Safe non-destructive servers import: append to Wex root servers.dat AND instance servers.dat
    if req.import_servers && !req.servers.is_empty() {
        let root_servers_dat = paths.root.join("servers.dat");
        let v120_servers_dat = paths.instance_dir("1.20.1").join("servers.dat");
        let v121_servers_dat = paths.instance_dir("1.21.1").join("servers.dat");

        let pack_entries: Vec<wex_net::PackServerEntry> = req
            .servers
            .iter()
            .map(|s| wex_net::PackServerEntry {
                id: None,
                name: s.name.clone(),
                ip: s.ip.clone(),
                icon: s.icon.clone(),
            })
            .collect();

        for s_path in [&root_servers_dat, &v120_servers_dat, &v121_servers_dat] {
            if let Some(parent) = s_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let mut root = wex_net::load_servers_dat(s_path).unwrap_or_else(|_| {
                wex_net::NbtTag::Compound(vec![(
                    "servers".to_string(),
                    wex_net::NbtTag::List {
                        tag_id: 10,
                        elements: Vec::new(),
                    },
                )])
            });

            wex_net::merge_pack_servers(&mut root, &pack_entries);
            let _ = wex_net::save_servers_dat(s_path, &root);
        }
        imported_servers_count = req.servers.len();
    }

    // 3. Safe non-destructive skins import: copy PNGs into Wex skins/
    if req.import_skins && !req.skins.is_empty() {
        let target_skins_dir = paths.root.join("skins");
        if let Err(e) = fs::create_dir_all(&target_skins_dir) {
            errors.push(format!("Не удалось создать папку скинов: {}", e));
        } else {
            for skin in &req.skins {
                let file_name = match validate_safe_component(&skin.file_name, "Имя файла скина")
                {
                    Ok(f) => f,
                    Err(err) => {
                        errors.push(err);
                        continue;
                    }
                };
                let src = PathBuf::from(&skin.path);
                if !src.is_file() {
                    errors.push(format!("Файл скина не найден: '{}'", skin.path));
                    continue;
                }

                let initial_dest = target_skins_dir.join(&file_name);
                let dest = if initial_dest.exists() {
                    if let (Ok(s_bytes), Ok(d_bytes)) = (fs::read(&src), fs::read(&initial_dest)) {
                        if s_bytes == d_bytes {
                            imported_skins_count += 1;
                            continue;
                        }
                    }
                    generate_unique_file_dest(&target_skins_dir, &file_name)
                } else {
                    initial_dest
                };

                if !dest.starts_with(&target_skins_dir) {
                    errors.push(format!(
                        "Целевой путь скина выходит за пределы каталога: {:?}",
                        dest
                    ));
                    continue;
                }

                match fs::copy(&src, &dest) {
                    Ok(_) => imported_skins_count += 1,
                    Err(e) => errors.push(format!(
                        "Ошибка копирования скина '{}': {}",
                        skin.file_name, e
                    )),
                }
            }
        }
    }

    // 3b. Safe non-destructive capes import: copy PNGs into Wex capes/
    if req.import_skins && !req.capes.is_empty() {
        let target_capes_dir = paths.root.join("capes");
        if let Err(e) = fs::create_dir_all(&target_capes_dir) {
            errors.push(format!("Не удалось создать папку плащей: {}", e));
        } else {
            for cape in &req.capes {
                let file_name = match validate_safe_component(&cape.file_name, "Имя файла плаща")
                {
                    Ok(f) => f,
                    Err(err) => {
                        errors.push(err);
                        continue;
                    }
                };
                let src = PathBuf::from(&cape.path);
                if !src.is_file() {
                    errors.push(format!("Файл плаща не найден: '{}'", cape.path));
                    continue;
                }

                let initial_dest = target_capes_dir.join(&file_name);
                let dest = if initial_dest.exists() {
                    if let (Ok(s_bytes), Ok(d_bytes)) = (fs::read(&src), fs::read(&initial_dest)) {
                        if s_bytes == d_bytes {
                            continue;
                        }
                    }
                    generate_unique_file_dest(&target_capes_dir, &file_name)
                } else {
                    initial_dest
                };

                if !dest.starts_with(&target_capes_dir) {
                    errors.push(format!(
                        "Целевой путь плаща выходит за пределы каталога: {:?}",
                        dest
                    ));
                    continue;
                }

                if let Err(e) = fs::copy(&src, &dest) {
                    errors.push(format!(
                        "Ошибка копирования плаща '{}': {}",
                        cape.file_name, e
                    ));
                }
            }
        }
    }

    // 3c. Import playtime statistics if present in old launcher
    if let Ok(old_data) = import_old_launcher_internal(&req.old_launcher_dir) {
        if old_data.total_play_time_seconds > 0 || old_data.total_game_launches > 0 {
            let stats_file = paths.root.join("stats.json");
            let mut current = get_playtime_internal(paths);
            if current.total_seconds == 0 {
                current.total_seconds = old_data.total_play_time_seconds;
            }
            if current.total_launches == 0 {
                current.total_launches = old_data.total_game_launches;
            }
            current.formatted_play_time = format_play_time(current.total_seconds);
            current.summary_text = format!(
                "В игре: {}  •  Запусков: {}",
                current.formatted_play_time, current.total_launches
            );
            let val = serde_json::json!({
                "total_play_time_seconds": current.total_seconds,
                "total_game_launches": current.total_launches,
                "last_played_utc": current.last_played_utc,
            });
            if let Ok(json_str) = serde_json::to_string_pretty(&val) {
                let _ = fs::write(&stats_file, json_str);
            }
        }
    }

    // 4. Safe non-destructive worlds import: copy world saves into Wex instance saves
    if req.import_worlds && !req.worlds.is_empty() {
        for world in &req.worlds {
            let folder_name = match validate_safe_component(&world.folder_name, "Папка мира")
            {
                Ok(f) => f,
                Err(err) => {
                    errors.push(err);
                    continue;
                }
            };

            let raw_ver = world
                .version
                .as_deref()
                .or(req.selected_version.as_deref())
                .unwrap_or("1.21.1");
            let target_ver = match validate_supported_version(raw_ver) {
                Ok(v) => v,
                Err(err) => {
                    errors.push(err);
                    continue;
                }
            };

            let src = PathBuf::from(&world.source_path);
            if !src.is_dir() {
                errors.push(format!("Каталог мира не найден: '{}'", world.source_path));
                continue;
            }

            let target_saves_dir = paths.instance_dir(&target_ver).join("saves");
            if let Err(e) = fs::create_dir_all(&target_saves_dir) {
                errors.push(format!("Не удалось создать каталог сохранений: {}", e));
                continue;
            }

            let dest = generate_unique_world_dest(&target_saves_dir, &folder_name);
            if !dest.starts_with(&target_saves_dir) {
                errors.push(format!(
                    "Целевой путь сохранения мира выходит за пределы каталога: {:?}",
                    dest
                ));
                continue;
            }

            match copy_dir_all(&src, &dest) {
                Ok(_) => imported_worlds_count += 1,
                Err(err) => errors.push(format!(
                    "Ошибка копирования файлов мира '{}': {}",
                    world.folder_name, err
                )),
            }
        }
    }

    let success = errors.is_empty();
    let message = if success {
        "Данные успешно перенесены в Wex без изменения файлов старого лаунчера".to_string()
    } else {
        format!(
            "Импорт завершён с ошибками ({}/{} операций не выполнено): {}",
            errors.len(),
            req.worlds.len() + req.skins.len() + req.capes.len(),
            errors.join("; ")
        )
    };
    Ok(ApplyImportResponse {
        success,
        imported_nickname,
        imported_ram_gb,
        imported_servers_count,
        imported_skins_count,
        imported_worlds_count,
        message,
        errors,
    })
}

// ---------------------------------------------------------------------------
// Tauri Command Handlers
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn social_get_news(
    state: State<'_, PlayState>,
    request: Option<SocialGetNewsRequest>,
) -> Result<SocialGetNewsResponse, String> {
    let versions = get_news_internal(&state.paths, request);
    Ok(SocialGetNewsResponse { versions })
}

#[tauri::command]
pub async fn overview_get_news(
    state: State<'_, PlayState>,
) -> Result<Vec<ReleaseNoteVersionDto>, String> {
    Ok(get_news_internal(&state.paths, None))
}

#[tauri::command]
pub async fn social_check_launcher_update(
    _state: State<'_, PlayState>,
    request: Option<SocialCheckLauncherUpdateRequest>,
) -> Result<UpdateCheckResultDto, String> {
    let cur_v = request
        .as_ref()
        .map(|r| r.current_version.as_str())
        .unwrap_or("1.0.0");
    let manifest_url = request.as_ref().map(|r| r.manifest_url.as_str());
    let target_platform = request.as_ref().and_then(|r| r.target_platform.as_deref());

    check_update_internal(cur_v, manifest_url, target_platform).await
}

#[tauri::command]
pub async fn overview_check_update(
    _state: State<'_, PlayState>,
    manifest_url: Option<String>,
) -> Result<UpdateCheckResultDto, String> {
    check_update_internal(
        "1.0.0",
        manifest_url.as_deref(),
        Some(DEFAULT_TARGET_PLATFORM),
    )
    .await
}

#[tauri::command]
pub async fn extras_detect_old_launcher() -> Result<Option<String>, String> {
    Ok(detect_old_launcher_internal())
}

#[tauri::command]
pub async fn extras_import_old_launcher(
    request: ExtrasImportOldLauncherRequest,
) -> Result<OldLauncherDataDto, String> {
    import_old_launcher_internal(&request.old_launcher_dir)
}

#[tauri::command]
pub async fn extras_apply_import(
    request: ApplyImportRequest,
    state: State<'_, PlayState>,
) -> Result<ApplyImportResponse, String> {
    apply_import_internal(&state.paths, state.core.as_ref(), request).await
}

#[tauri::command]
pub async fn overview_get_playtime(state: State<'_, PlayState>) -> Result<PlayTimeDto, String> {
    Ok(get_playtime_internal(&state.paths))
}

#[tauri::command]
pub async fn overview_record_playtime(
    state: State<'_, PlayState>,
    added_seconds: u64,
    inc_launch: Option<bool>,
) -> Result<PlayTimeDto, String> {
    Ok(record_playtime_internal(
        &state.paths,
        added_seconds,
        inc_launch.unwrap_or(false),
    ))
}

// ---------------------------------------------------------------------------
// Unit Tests (Mock Core & temporary isolated directories)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_ipc::core_trait::MockLauncherCore;

    #[tokio::test]
    async fn test_news_retrieval_and_empty_handling() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));

        // 1. Initial retrieval returns bundled release notes
        let news = get_news_internal(&paths, None);
        assert!(!news.is_empty(), "Bundled release notes must not be empty");
        assert_eq!(news[0].version, "v1.0.0");
        assert!(news[0].is_current);

        // 2. Custom news cache file persistence
        let news_dir = tmp.path().join("custom_news");
        fs::create_dir_all(&news_dir).unwrap();
        let mgr = NewsManager::new(&news_dir);

        let custom_json = r#"{
            "userFacingVersion": "v1.2.0",
            "internalVersion": "1.2.0",
            "title": "Релиз 1.2.0",
            "dateText": "8 ноября 2026",
            "changelog": [
                "[Новое] Добавлен мастер импорта",
                "[Улучшено] Плавность интерфейса"
            ]
        }"#;

        mgr.update_from_version_json(custom_json, "v1.2.0").unwrap();

        let req = SocialGetNewsRequest {
            cache_dir: news_dir.to_string_lossy().to_string(),
            remote_url: None,
            force_refresh: false,
        };

        let updated = get_news_internal(&paths, Some(req));
        assert_eq!(updated[0].version, "v1.2.0");
        assert_eq!(updated[0].items.len(), 2);
    }

    #[tokio::test]
    async fn test_check_update_logic() {
        // Without remote URL -> reports current version is up to date
        let res = check_update_internal("1.0.0", None, None).await.unwrap();
        assert!(!res.update_available);
        assert_eq!(res.current_version, "1.0.0");

        // With invalid remote URL -> reports network error
        let err_res = check_update_internal(
            "1.0.0",
            Some("http://invalid.local.test:9999/manifest.json"),
            None,
        )
        .await;
        assert!(err_res.is_err());
    }

    #[tokio::test]
    async fn test_import_old_launcher_from_fixture() {
        let tmp = tempdir().unwrap();
        let aura_dir = tmp.path().join(".aura");
        fs::create_dir_all(&aura_dir).unwrap();

        // 1. Write config.json
        let config_json = serde_json::json!({
            "Nickname": "OldGamer",
            "RamMb": 6144,
            "SelectedVersion": "1.21.1",
            "GraphicsPreset": "Medium",
            "TotalPlayTimeSeconds": 12000,
            "TotalGameLaunches": 25,
            "WardrobeHistory": [
                {
                    "Id": "skin-1",
                    "Type": "skin",
                    "Name": "Warrior",
                    "Hash": "abc",
                    "Model": "classic"
                }
            ]
        });
        fs::write(aura_dir.join("config.json"), config_json.to_string()).unwrap();

        // 2. Skins directory
        let skins_dir = aura_dir.join("skins");
        fs::create_dir_all(&skins_dir).unwrap();
        fs::write(skins_dir.join("skin_steve.png"), b"FAKE_PNG_BYTES").unwrap();

        // 3. Import
        let imported = import_old_launcher_internal(&aura_dir.to_string_lossy()).unwrap();
        assert_eq!(imported.nickname, "OldGamer");
        assert_eq!(imported.ram_mb, 6144);
        assert_eq!(imported.selected_version, "1.21.1");
        assert_eq!(imported.skins.len(), 1);
        assert_eq!(imported.skins[0].file_name, "skin_steve.png");
    }

    #[tokio::test]
    async fn test_apply_import_copies_data_without_altering_old_launcher() {
        let tmp = tempdir().unwrap();
        let wex_root = tmp.path().join("Wex");
        let paths = WexPaths::new(Some(wex_root.clone()));
        let core = MockLauncherCore::new();

        // Source old launcher folder
        let old_aura_dir = tmp.path().join(".aura");
        let old_skins_dir = old_aura_dir.join("skins");
        fs::create_dir_all(&old_skins_dir).unwrap();

        let test_skin_path = old_skins_dir.join("player_skin.png");
        fs::write(&test_skin_path, b"TEST_SKIN_DATA_ORIGINAL").unwrap();

        let req = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: true,
            nickname: Some("ImportedChampion".to_string()),
            import_ram: true,
            ram_mb: Some(8192),
            import_version: true,
            selected_version: Some("1.21.1".to_string()),
            import_servers: true,
            servers: vec![ImportedServerDto {
                name: "Imported Server".to_string(),
                ip: "imported.auramc.net".to_string(),
                icon: None,
            }],
            import_skins: true,
            skins: vec![ImportedSkinDto {
                file_name: "player_skin.png".to_string(),
                path: test_skin_path.to_string_lossy().to_string(),
                hash: "dummyhash".to_string(),
                is_slim: false,
            }],
            capes: Vec::new(),
            graphics_preset: Some("High".to_string()),
            import_worlds: false,
            worlds: Vec::new(),
        };

        let res = apply_import_internal(&paths, &core, req).await.unwrap();
        assert!(res.success);
        assert_eq!(res.imported_nickname.as_deref(), Some("ImportedChampion"));
        assert_eq!(res.imported_ram_gb, Some(8));
        assert_eq!(res.imported_servers_count, 1);
        assert_eq!(res.imported_skins_count, 1);

        // Verify Wex config was updated in core
        let updated_cfg = core.get_config().await.unwrap();
        assert_eq!(updated_cfg.player_name, "ImportedChampion");
        assert_eq!(updated_cfg.ram_gb, 8);
        assert_eq!(updated_cfg.selected_version, "1.21.1");

        // Verify skin was copied to Wex
        let wex_skin_file = wex_root.join("skins").join("player_skin.png");
        assert!(wex_skin_file.is_file());
        assert_eq!(
            fs::read(&wex_skin_file).unwrap(),
            b"TEST_SKIN_DATA_ORIGINAL"
        );

        // CRITICAL: verify original old launcher skin file is untouched and still exists
        assert!(test_skin_path.is_file());
        assert_eq!(
            fs::read(&test_skin_path).unwrap(),
            b"TEST_SKIN_DATA_ORIGINAL"
        );
    }

    #[tokio::test]
    async fn test_import_real_scenario_with_worlds_and_settings() {
        let tmp = tempdir().unwrap();
        let wex_root = tmp.path().join("Wex");
        let paths = WexPaths::new(Some(wex_root.clone()));
        let core = MockLauncherCore::new();

        // 1. Setup realistic old launcher directory structure
        let old_aura_dir = tmp.path().join(".aura");
        fs::create_dir_all(&old_aura_dir).unwrap();

        // Old config.json
        let config_json = serde_json::json!({
            "Nickname": "AuraVeteran",
            "RamMb": 8192,
            "SelectedVersion": "1.20.1",
            "GraphicsPreset": "Ultra",
            "JvmArgs": "-XX:+UseG1GC",
            "DiscordRpcEnabled": true,
            "TotalPlayTimeSeconds": 86400,
            "TotalGameLaunches": 50,
            "SkinPath": "skins/hero.png",
            "SkinModel": "default"
        });
        fs::write(old_aura_dir.join("config.json"), config_json.to_string()).unwrap();

        // Old skins & capes
        let old_skins = old_aura_dir.join("skins");
        fs::create_dir_all(&old_skins).unwrap();
        let hero_skin = old_skins.join("hero.png");
        fs::write(&hero_skin, b"PNG_HERO_SKIN_DATA").unwrap();

        let old_capes = old_aura_dir.join("capes");
        fs::create_dir_all(&old_capes).unwrap();
        let aura_cape = old_capes.join("cape_cyan.png");
        fs::write(&aura_cape, b"PNG_AURA_CAPE_DATA").unwrap();

        // Old worlds in instances
        let v120_saves = old_aura_dir.join("instances").join("1.20.1").join("saves");
        let world_a = v120_saves.join("SurvivalWorld");
        fs::create_dir_all(&world_a).unwrap();
        fs::write(world_a.join("level.dat"), b"MOCK_LEVEL_DAT_A").unwrap();
        fs::write(world_a.join("icon.png"), b"MOCK_ICON_A").unwrap();

        let v121_saves = old_aura_dir.join("instances").join("1.21.1").join("saves");
        let world_b = v121_saves.join("CoopRealm");
        fs::create_dir_all(&world_b).unwrap();
        fs::write(world_b.join("level.dat"), b"MOCK_LEVEL_DAT_B").unwrap();

        // 2. Run scan
        let scanned = import_old_launcher_internal(&old_aura_dir.to_string_lossy()).unwrap();
        assert_eq!(scanned.nickname, "AuraVeteran");
        assert_eq!(scanned.ram_mb, 8192);
        assert_eq!(scanned.selected_version, "1.20.1");
        assert_eq!(scanned.graphics_preset.as_deref(), Some("Ultra"));
        assert_eq!(scanned.skins.len(), 1);
        assert_eq!(scanned.capes.len(), 1);
        assert_eq!(scanned.worlds.len(), 2);

        // 3. Apply import
        let req = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: true,
            nickname: Some(scanned.nickname),
            import_ram: true,
            ram_mb: Some(scanned.ram_mb),
            import_version: true,
            selected_version: Some(scanned.selected_version),
            graphics_preset: scanned.graphics_preset,
            import_servers: true,
            servers: scanned.servers,
            import_skins: true,
            skins: scanned.skins,
            capes: scanned.capes,
            import_worlds: true,
            worlds: scanned.worlds,
        };

        let res = apply_import_internal(&paths, &core, req).await.unwrap();
        assert!(res.success);
        assert_eq!(res.imported_nickname.as_deref(), Some("AuraVeteran"));
        assert_eq!(res.imported_ram_gb, Some(8));
        assert_eq!(res.imported_skins_count, 1);
        assert_eq!(res.imported_worlds_count, 2);

        // 4. Verify Wex directories received all files
        assert!(wex_root.join("skins").join("hero.png").is_file());
        assert!(wex_root.join("capes").join("cape_cyan.png").is_file());

        let target_world_a = paths
            .instance_dir("1.20.1")
            .join("saves")
            .join("SurvivalWorld");
        assert!(target_world_a.join("level.dat").is_file());
        assert!(target_world_a.join("icon.png").is_file());
        assert_eq!(
            fs::read(target_world_a.join("level.dat")).unwrap(),
            b"MOCK_LEVEL_DAT_A"
        );

        let target_world_b = paths.instance_dir("1.21.1").join("saves").join("CoopRealm");
        assert!(target_world_b.join("level.dat").is_file());
        assert_eq!(
            fs::read(target_world_b.join("level.dat")).unwrap(),
            b"MOCK_LEVEL_DAT_B"
        );

        // Verify core config has graphics preset and exact memory
        let core_cfg = wex_core::config::LauncherConfig::load_or_default(&paths.config_file);
        assert_eq!(core_cfg.graphics_preset, "Ultra");
        assert_eq!(core_cfg.max_memory_mb, 8192);

        // 5. CRITICAL NON-DESTRUCTIVE CHECK:
        // Ensure old launcher files still exist completely untouched
        assert!(hero_skin.is_file());
        assert!(aura_cape.is_file());
        assert!(world_a.join("level.dat").is_file());
        assert!(world_b.join("level.dat").is_file());
    }

    #[tokio::test]
    async fn test_apply_import_repeat_is_idempotent_and_safe() {
        let tmp = tempdir().unwrap();
        let wex_root = tmp.path().join("Wex");
        let paths = WexPaths::new(Some(wex_root.clone()));
        let core = MockLauncherCore::new();

        let old_aura_dir = tmp.path().join(".aura");
        let old_skins = old_aura_dir.join("skins");
        fs::create_dir_all(&old_skins).unwrap();
        let skin_path = old_skins.join("player.png");
        fs::write(&skin_path, b"ORIGINAL_SKIN").unwrap();

        let req = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: true,
            nickname: Some("Player1".to_string()),
            import_ram: true,
            ram_mb: Some(4096),
            import_version: true,
            selected_version: Some("1.21.1".to_string()),
            graphics_preset: None,
            import_servers: false,
            servers: Vec::new(),
            import_skins: true,
            skins: vec![ImportedSkinDto {
                file_name: "player.png".to_string(),
                path: skin_path.to_string_lossy().to_string(),
                hash: "h1".to_string(),
                is_slim: false,
            }],
            capes: Vec::new(),
            import_worlds: false,
            worlds: Vec::new(),
        };

        // First import
        let res1 = apply_import_internal(&paths, &core, req.clone())
            .await
            .unwrap();
        assert!(res1.success);
        assert_eq!(res1.imported_skins_count, 1);
        assert!(res1.errors.is_empty());

        // Second (repeat) import of identical content
        let res2 = apply_import_internal(&paths, &core, req).await.unwrap();
        assert!(res2.success);
        assert_eq!(res2.imported_skins_count, 1);
        assert!(res2.errors.is_empty());
        assert_eq!(
            fs::read(wex_root.join("skins").join("player.png")).unwrap(),
            b"ORIGINAL_SKIN"
        );
    }

    #[tokio::test]
    async fn test_apply_import_world_collision_preserves_existing_world() {
        let tmp = tempdir().unwrap();
        let wex_root = tmp.path().join("Wex");
        let paths = WexPaths::new(Some(wex_root.clone()));
        let core = MockLauncherCore::new();

        // Pre-create existing world in Wex instance
        let target_saves_dir = paths.instance_dir("1.21.1").join("saves");
        let existing_world = target_saves_dir.join("MyWorld");
        fs::create_dir_all(&existing_world).unwrap();
        fs::write(existing_world.join("level.dat"), b"EXISTING_ORIGINAL_DATA").unwrap();

        // Source world with same folder_name but different data
        let old_aura_dir = tmp.path().join(".aura");
        let src_world = old_aura_dir.join("saves").join("MyWorld");
        fs::create_dir_all(&src_world).unwrap();
        fs::write(src_world.join("level.dat"), b"IMPORTED_NEW_DATA").unwrap();

        let req = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: false,
            nickname: None,
            import_ram: false,
            ram_mb: None,
            import_version: false,
            selected_version: Some("1.21.1".to_string()),
            graphics_preset: None,
            import_servers: false,
            servers: Vec::new(),
            import_skins: false,
            skins: Vec::new(),
            capes: Vec::new(),
            import_worlds: true,
            worlds: vec![ImportedWorldDto {
                folder_name: "MyWorld".to_string(),
                display_name: "My World".to_string(),
                version: Some("1.21.1".to_string()),
                size_bytes: 100,
                source_path: src_world.to_string_lossy().to_string(),
            }],
        };

        let res = apply_import_internal(&paths, &core, req).await.unwrap();
        assert!(res.success);
        assert_eq!(res.imported_worlds_count, 1);
        assert!(res.errors.is_empty());

        // Verify existing world is 100% UNTOUCHED
        assert_eq!(
            fs::read(existing_world.join("level.dat")).unwrap(),
            b"EXISTING_ORIGINAL_DATA",
            "Existing world MUST NOT be overwritten on collision"
        );

        // Verify imported world was written to unique imported path
        let imported_dest = target_saves_dir.join("MyWorld-imported-1");
        assert!(imported_dest.is_dir());
        assert_eq!(
            fs::read(imported_dest.join("level.dat")).unwrap(),
            b"IMPORTED_NEW_DATA"
        );
    }

    #[tokio::test]
    async fn test_apply_import_path_traversal_prevention() {
        let tmp = tempdir().unwrap();
        let wex_root = tmp.path().join("Wex");
        let paths = WexPaths::new(Some(wex_root.clone()));
        let core = MockLauncherCore::new();

        let old_aura_dir = tmp.path().join(".aura");
        let dummy_src = old_aura_dir.join("dummy");
        fs::create_dir_all(&dummy_src).unwrap();
        fs::write(dummy_src.join("level.dat"), b"EVIL").unwrap();

        // 1. World folder traversal
        let req1 = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: false,
            nickname: None,
            import_ram: false,
            ram_mb: None,
            import_version: false,
            selected_version: Some("1.21.1".to_string()),
            graphics_preset: None,
            import_servers: false,
            servers: Vec::new(),
            import_skins: false,
            skins: Vec::new(),
            capes: Vec::new(),
            import_worlds: true,
            worlds: vec![ImportedWorldDto {
                folder_name: "../../escape".to_string(),
                display_name: "Evil World".to_string(),
                version: Some("1.21.1".to_string()),
                size_bytes: 10,
                source_path: dummy_src.to_string_lossy().to_string(),
            }],
        };

        let res1 = apply_import_internal(&paths, &core, req1).await.unwrap();
        assert!(!res1.success);
        assert_eq!(res1.imported_worlds_count, 0);
        assert!(res1.errors.iter().any(|e| e.contains("path traversal")));
        assert!(!wex_root.parent().unwrap().join("escape").exists());

        // 2. Skin file traversal
        let req2 = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: false,
            nickname: None,
            import_ram: false,
            ram_mb: None,
            import_version: false,
            selected_version: Some("1.21.1".to_string()),
            graphics_preset: None,
            import_servers: false,
            servers: Vec::new(),
            import_skins: true,
            skins: vec![ImportedSkinDto {
                file_name: "../evil_skin.png".to_string(),
                path: dummy_src.join("level.dat").to_string_lossy().to_string(),
                hash: "h".to_string(),
                is_slim: false,
            }],
            capes: Vec::new(),
            import_worlds: false,
            worlds: Vec::new(),
        };

        let res2 = apply_import_internal(&paths, &core, req2).await.unwrap();
        assert!(!res2.success);
        assert_eq!(res2.imported_skins_count, 0);
        assert!(res2.errors.iter().any(|e| e.contains("path traversal")));

        // 3. Version traversal
        let req3 = ApplyImportRequest {
            old_launcher_dir: old_aura_dir.to_string_lossy().to_string(),
            import_nickname: false,
            nickname: None,
            import_ram: false,
            ram_mb: None,
            import_version: false,
            selected_version: Some("../../evil_ver".to_string()),
            graphics_preset: None,
            import_servers: false,
            servers: Vec::new(),
            import_skins: false,
            skins: Vec::new(),
            capes: Vec::new(),
            import_worlds: true,
            worlds: vec![ImportedWorldDto {
                folder_name: "World1".to_string(),
                display_name: "World".to_string(),
                version: Some("../../evil_ver".to_string()),
                size_bytes: 10,
                source_path: dummy_src.to_string_lossy().to_string(),
            }],
        };

        let res3 = apply_import_internal(&paths, &core, req3).await.unwrap();
        assert!(!res3.success);
        assert_eq!(res3.imported_worlds_count, 0);
        assert!(res3
            .errors
            .iter()
            .any(|e| e.contains("path traversal") || e.contains("Неподдерживаемая версия")));
    }

    #[tokio::test]
    async fn test_apply_import_copy_error_handling() {
        let tmp = tempdir().unwrap();
        let wex_root = tmp.path().join("Wex");
        let paths = WexPaths::new(Some(wex_root.clone()));
        let core = MockLauncherCore::new();

        let non_existent = tmp.path().join("does_not_exist");

        let req = ApplyImportRequest {
            old_launcher_dir: tmp.path().to_string_lossy().to_string(),
            import_nickname: false,
            nickname: None,
            import_ram: false,
            ram_mb: None,
            import_version: false,
            selected_version: Some("1.21.1".to_string()),
            graphics_preset: None,
            import_servers: false,
            servers: Vec::new(),
            import_skins: true,
            skins: vec![ImportedSkinDto {
                file_name: "missing.png".to_string(),
                path: non_existent
                    .join("missing.png")
                    .to_string_lossy()
                    .to_string(),
                hash: "h".to_string(),
                is_slim: false,
            }],
            capes: Vec::new(),
            import_worlds: true,
            worlds: vec![ImportedWorldDto {
                folder_name: "MissingWorld".to_string(),
                display_name: "Missing World".to_string(),
                version: Some("1.21.1".to_string()),
                size_bytes: 10,
                source_path: non_existent
                    .join("MissingWorld")
                    .to_string_lossy()
                    .to_string(),
            }],
        };

        let res = apply_import_internal(&paths, &core, req).await.unwrap();
        assert!(
            !res.success,
            "Must report success=false when copy errors occur"
        );
        assert_eq!(res.imported_skins_count, 0);
        assert_eq!(res.imported_worlds_count, 0);
        assert_eq!(
            res.errors.len(),
            2,
            "Both skin and world copy failures must be recorded explicitly"
        );
        assert!(res.errors.iter().any(|e| e.contains("не найден")));
        assert!(res.message.contains("ошибками"));
    }

    #[test]
    fn test_playtime_initial_and_recording() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));

        // 1. Initially empty
        let initial = get_playtime_internal(&paths);
        assert_eq!(initial.total_seconds, 0);
        assert_eq!(initial.total_launches, 0);
        assert_eq!(initial.formatted_play_time, "0 мин.");

        // 2. Record 3700 seconds (~1 hr 1 min) with launch
        let rec = record_playtime_internal(&paths, 3700, true);
        assert_eq!(rec.total_seconds, 3700);
        assert_eq!(rec.total_launches, 1);
        assert_eq!(rec.formatted_play_time, "1 ч. 1 мин.");
        assert_eq!(rec.summary_text, "В игре: 1 ч. 1 мин.  •  Запусков: 1");

        // 3. Persistent across reloads
        let loaded = get_playtime_internal(&paths);
        assert_eq!(loaded.total_seconds, 3700);
        assert_eq!(loaded.total_launches, 1);
    }

    #[test]
    fn test_playtime_formatting() {
        assert_eq!(format_play_time(0), "0 мин.");
        assert_eq!(format_play_time(59), "0 мин.");
        assert_eq!(format_play_time(125), "2 мин.");
        assert_eq!(format_play_time(3600), "1 ч. 0 мин.");
        assert_eq!(format_play_time(3665), "1 ч. 1 мин.");
        assert_eq!(format_play_time(7500), "2 ч. 5 мин.");
    }
}
