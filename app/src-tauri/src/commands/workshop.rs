use std::path::{Path, PathBuf};
use tauri::State;
use wex_core::config::WexPaths;
use wex_ipc::commands::{
    SocialCreateWorldBackupRequest, SocialListScreenshotsRequest, SocialListScreenshotsResponse,
    SocialListWorldBackupsRequest, SocialListWorldBackupsResponse, SocialListWorldsRequest,
    SocialListWorldsResponse, SocialRestoreWorldBackupRequest, SocialRestoreWorldBackupResponse,
};
use wex_ipc::models::{ScreenshotItemDto, WorldBackupItemDto, WorldItemDto};

use super::launch::PlayState;

fn resolve_game_dir(paths: &WexPaths, version: Option<&str>) -> PathBuf {
    let ver = version.unwrap_or("1.21.1");
    paths.instance_dir(ver)
}

// ============================================================================
// Tauri Commands: Workshop (SPEC 1.7)
// ============================================================================

/// List single-player worlds for the selected Minecraft instance.
#[tauri::command]
pub async fn workshop_list_worlds(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<WorldItemDto>, String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let worlds = wex_social::list_worlds(&game_dir).map_err(|e| e.to_string())?;
    Ok(worlds.into_iter().map(Into::into).collect())
}

/// Compatibility IPC command per docs/IPC.md
#[tauri::command]
pub async fn social_list_worlds(
    request: Option<SocialListWorldsRequest>,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<SocialListWorldsResponse, String> {
    let game_dir = match request {
        Some(req) if !req.game_dir.is_empty() => PathBuf::from(req.game_dir),
        _ => resolve_game_dir(&state.paths, version.as_deref()),
    };
    let worlds = wex_social::list_worlds(&game_dir).map_err(|e| e.to_string())?;
    Ok(SocialListWorldsResponse {
        worlds: worlds.into_iter().map(Into::into).collect(),
    })
}

/// Create a compressed ZIP backup of the specified world folder.
#[tauri::command]
pub async fn workshop_create_world_backup(
    world_folder: String,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<WorldBackupItemDto, String> {
    let world_path = PathBuf::from(&world_folder);
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let backup_base_dir = game_dir.join("backups");
    let zip_path = wex_social::create_world_backup(&world_path, &backup_base_dir)
        .map_err(|e| e.to_string())?;

    let file_name = zip_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let size_bytes = zip_path.metadata().map(|m| m.len()).unwrap_or(0);
    let created_at = zip_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

    Ok(WorldBackupItemDto {
        file_name,
        file_path: zip_path.to_string_lossy().to_string(),
        created_at,
        size_bytes,
    })
}

/// Compatibility IPC command per docs/IPC.md
#[tauri::command]
pub async fn social_create_world_backup(
    request: SocialCreateWorldBackupRequest,
    _state: State<'_, PlayState>,
) -> Result<WorldBackupItemDto, String> {
    let world_path = PathBuf::from(&request.world_dir);
    let backup_base = PathBuf::from(&request.backups_dir);
    let zip_path =
        wex_social::create_world_backup(&world_path, &backup_base).map_err(|e| e.to_string())?;

    let file_name = zip_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let size_bytes = zip_path.metadata().map(|m| m.len()).unwrap_or(0);
    let created_at = zip_path
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());

    Ok(WorldBackupItemDto {
        file_name,
        file_path: zip_path.to_string_lossy().to_string(),
        created_at,
        size_bytes,
    })
}

/// List available world backups for the instance or specific world.
#[tauri::command]
pub async fn workshop_list_world_backups(
    version: Option<String>,
    world_folder_name: Option<String>,
    display_name: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<WorldBackupItemDto>, String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let backup_base_dir = game_dir.join("backups");
    let folder_name = world_folder_name.as_deref().unwrap_or("");
    let d_name = display_name.as_deref().unwrap_or(folder_name);
    let backups = wex_social::list_world_backups(&backup_base_dir, folder_name, d_name)
        .map_err(|e| e.to_string())?;
    Ok(backups.into_iter().map(Into::into).collect())
}

/// Compatibility IPC command per docs/IPC.md
#[tauri::command]
pub async fn social_list_world_backups(
    request: SocialListWorldBackupsRequest,
    _state: State<'_, PlayState>,
) -> Result<SocialListWorldBackupsResponse, String> {
    let backup_base = PathBuf::from(&request.backups_dir);
    let backups =
        wex_social::list_world_backups(&backup_base, "", "").map_err(|e| e.to_string())?;
    Ok(SocialListWorldBackupsResponse {
        backups: backups.into_iter().map(Into::into).collect(),
    })
}

/// Restore a world from a ZIP backup archive.
#[tauri::command]
pub async fn workshop_restore_world_backup(
    backup_path: String,
    target_world_folder: Option<String>,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    let zip_file = PathBuf::from(&backup_path);
    let target_dir = match target_world_folder {
        Some(t) if !t.is_empty() => PathBuf::from(t),
        _ => {
            let stem = zip_file
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let safe_name = stem.split('_').next().unwrap_or("RestoredWorld");
            let game_dir = resolve_game_dir(&state.paths, version.as_deref());
            game_dir.join("saves").join(safe_name)
        }
    };
    wex_social::restore_world_backup(&zip_file, &target_dir).map_err(|e| e.to_string())?;
    Ok(target_dir.to_string_lossy().to_string())
}

/// Compatibility IPC command per docs/IPC.md
#[tauri::command]
pub async fn social_restore_world_backup(
    request: SocialRestoreWorldBackupRequest,
    _state: State<'_, PlayState>,
) -> Result<SocialRestoreWorldBackupResponse, String> {
    let zip_file = PathBuf::from(&request.backup_path);
    let saves_dir = PathBuf::from(&request.saves_dir);
    let stem = zip_file
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let safe_name = stem.split('_').next().unwrap_or("RestoredWorld");
    let target_dir = saves_dir.join(safe_name);
    wex_social::restore_world_backup(&zip_file, &target_dir).map_err(|e| e.to_string())?;
    Ok(SocialRestoreWorldBackupResponse {
        restored_world_dir: target_dir.to_string_lossy().to_string(),
    })
}

/// Permanently delete a world folder.
#[tauri::command]
pub async fn workshop_delete_world(
    world_folder: String,
    _state: State<'_, PlayState>,
) -> Result<(), String> {
    let path = PathBuf::from(&world_folder);
    wex_social::delete_world(&path).map_err(|e| e.to_string())
}

/// List screenshot PNG files in the instance screenshot folder.
#[tauri::command]
pub async fn workshop_list_screenshots(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<Vec<ScreenshotItemDto>, String> {
    let game_dir = resolve_game_dir(&state.paths, version.as_deref());
    let screenshots_dir = game_dir.join("screenshots");
    let list = wex_social::list_screenshots(&screenshots_dir).map_err(|e| e.to_string())?;
    Ok(list.into_iter().map(Into::into).collect())
}

/// Compatibility IPC command per docs/IPC.md
#[tauri::command]
pub async fn social_list_screenshots(
    request: Option<SocialListScreenshotsRequest>,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<SocialListScreenshotsResponse, String> {
    let screenshots_dir = match request {
        Some(req) if !req.screenshots_dir.is_empty() => PathBuf::from(req.screenshots_dir),
        _ => {
            let game_dir = resolve_game_dir(&state.paths, version.as_deref());
            game_dir.join("screenshots")
        }
    };
    let list = wex_social::list_screenshots(&screenshots_dir).map_err(|e| e.to_string())?;
    Ok(SocialListScreenshotsResponse {
        screenshots: list.into_iter().map(Into::into).collect(),
    })
}

/// Reveal a file or folder in system file manager (Explorer / Finder / xdg-open).
#[tauri::command]
pub async fn workshop_open_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(format!("Путь не существует: {}", path));
    }

    #[cfg(target_os = "windows")]
    {
        if p.is_file() {
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", p.display()))
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            std::process::Command::new("explorer")
                .arg(&p)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if p.is_file() {
            std::process::Command::new("open")
                .arg("-R")
                .arg(&p)
                .spawn()
                .map_err(|e| e.to_string())?;
        } else {
            std::process::Command::new("open")
                .arg(&p)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    #[cfg(target_os = "linux")]
    {
        let target = if p.is_file() {
            p.parent().unwrap_or(&p)
        } else {
            &p
        };
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Copies the image at the given path directly into the system clipboard as bitmap data.
pub fn copy_screenshot_image_internal<F>(path: &str, set_clipboard: F) -> Result<(), String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let p = PathBuf::from(path);
    if !p.exists() || !p.is_file() {
        return Err(format!("Файл скриншота не найден: {}", path));
    }
    set_clipboard(&p)
}

fn system_set_clipboard_image(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $img = [System.Drawing.Image]::FromFile('{}'); [System.Windows.Forms.Clipboard]::SetImage($img); $img.Dispose()",
            path.to_string_lossy().replace('\'', "''")
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .map_err(|e| format!("Ошибка копирования изображения: {}", e))?;
        if !status.success() {
            return Err("Не удалось скопировать изображение в буфер обмена".to_string());
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

#[tauri::command]
pub async fn workshop_copy_screenshot_image(path: String) -> Result<(), String> {
    copy_screenshot_image_internal(&path, system_set_clipboard_image)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;
    use wex_ipc::core_trait::MockLauncherCore;

    fn make_test_level_dat(name: &str, mode: i32, last_played: u64) -> Vec<u8> {
        let mut nbt = Vec::new();
        // LevelName
        nbt.push(0x08);
        nbt.extend_from_slice(&(b"LevelName".len() as u16).to_be_bytes());
        nbt.extend_from_slice(b"LevelName");
        nbt.extend_from_slice(&(name.len() as u16).to_be_bytes());
        nbt.extend_from_slice(name.as_bytes());
        // GameType
        nbt.push(0x03);
        nbt.extend_from_slice(&(b"GameType".len() as u16).to_be_bytes());
        nbt.extend_from_slice(b"GameType");
        nbt.extend_from_slice(&mode.to_be_bytes());
        // LastPlayed
        nbt.push(0x04);
        nbt.extend_from_slice(&(b"LastPlayed".len() as u16).to_be_bytes());
        nbt.extend_from_slice(b"LastPlayed");
        nbt.extend_from_slice(&last_played.to_be_bytes());
        nbt
    }

    #[tokio::test]
    async fn test_workshop_list_worlds_and_empty() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths.clone(), mock_core);

        let inst_dir = paths.instance_dir("1.21.1");
        fs::create_dir_all(&inst_dir).unwrap();

        // Empty state when saves folder is missing or empty
        let worlds = wex_social::list_worlds(&inst_dir).unwrap();
        assert!(worlds.is_empty());

        // Create a test world
        let world_dir = inst_dir.join("saves").join("SurvivalWorld");
        fs::create_dir_all(&world_dir).unwrap();
        let nbt = make_test_level_dat("Alpha Base", 0, 1731000000);
        fs::write(world_dir.join("level.dat"), &nbt).unwrap();

        let worlds = wex_social::list_worlds(&inst_dir).unwrap();
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].display_name, "Alpha Base");
        assert_eq!(worlds[0].folder_name, "SurvivalWorld");
    }

    #[tokio::test]
    async fn test_workshop_backup_lifecycle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths.clone(), mock_core);

        let inst_dir = paths.instance_dir("1.21.1");
        let world_dir = inst_dir.join("saves").join("BaseWorld");
        fs::create_dir_all(&world_dir).unwrap();
        let nbt = make_test_level_dat("Base World", 1, 100);
        fs::write(world_dir.join("level.dat"), &nbt).unwrap();
        fs::write(world_dir.join("world_data.bin"), b"block_data").unwrap();

        let backups_dir = inst_dir.join("backups");
        let zip_path = wex_social::create_world_backup(&world_dir, &backups_dir).unwrap();
        assert!(zip_path.is_file());

        let backups =
            wex_social::list_world_backups(&backups_dir, "BaseWorld", "Base World").unwrap();
        assert_eq!(backups.len(), 1);

        // Delete original and restore from backup
        wex_social::delete_world(&world_dir).unwrap();
        assert!(!world_dir.exists());

        wex_social::restore_world_backup(&zip_path, &world_dir).unwrap();
        assert!(world_dir.join("level.dat").is_file());
        assert!(world_dir.join("world_data.bin").is_file());
    }

    #[tokio::test]
    async fn test_workshop_screenshots_listing() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let inst_dir = paths.instance_dir("1.21.1");
        let screens_dir = inst_dir.join("screenshots");
        fs::create_dir_all(&screens_dir).unwrap();

        fs::write(screens_dir.join("screen1.png"), b"fake_png_1").unwrap();
        fs::write(screens_dir.join("screen2.png"), b"fake_png_2").unwrap();
        fs::write(screens_dir.join("log.txt"), b"not_png").unwrap();

        let list = wex_social::list_screenshots(&screens_dir).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_workshop_open_nonexistent_path() {
        let res = workshop_open_path("C:\\nonexistent_dir_12345_67890".to_string()).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_workshop_copy_screenshot_nonexistent() {
        let res = copy_screenshot_image_internal("C:\\nonexistent_screen_123.png", |_| Ok(()));
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Файл скриншота не найден"));
    }

    #[tokio::test]
    async fn test_workshop_copy_screenshot_valid_png_isolated() {
        let tmp = tempdir().unwrap();
        let shot_file = tmp.path().join("shot.png");
        let png_bytes = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        fs::write(&shot_file, &png_bytes).unwrap();
        let mut clipboard_invoked = false;
        let res = copy_screenshot_image_internal(&shot_file.to_string_lossy(), |p| {
            assert_eq!(p, shot_file.as_path());
            clipboard_invoked = true;
            Ok(())
        });
        assert!(res.is_ok());
        assert!(
            clipboard_invoked,
            "Isolated test verified clipboard callback without mutating OS clipboard"
        );
    }

    #[tokio::test]
    #[ignore = "Touches real host clipboard; run explicitly via cargo test -- --ignored"]
    async fn test_workshop_copy_screenshot_real_clipboard_opt_in() {
        let tmp = tempdir().unwrap();
        let shot_file = tmp.path().join("shot.png");
        let png_bytes = [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        fs::write(&shot_file, &png_bytes).unwrap();
        let res = workshop_copy_screenshot_image(shot_file.to_string_lossy().to_string()).await;
        assert!(res.is_ok());
    }
}
