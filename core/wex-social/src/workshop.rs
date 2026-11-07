use chrono::Utc;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[derive(Debug, Error)]
pub enum WorkshopError {
    #[error("Каталог не найден: {0}")]
    NotFound(PathBuf),
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка работы с ZIP-архивом: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Некорректный формат level.dat: {0}")]
    InvalidLevelDat(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 3,
    Unknown = -1,
}

impl GameMode {
    pub fn from_i32(val: i32) -> Self {
        match val {
            0 => GameMode::Survival,
            1 => GameMode::Creative,
            2 => GameMode::Adventure,
            3 => GameMode::Spectator,
            _ => GameMode::Unknown,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            GameMode::Survival => "Выживание",
            GameMode::Creative => "Творческий",
            GameMode::Adventure => "Приключение",
            GameMode::Spectator => "Наблюдатель",
            GameMode::Unknown => "Неизвестно",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldItem {
    pub folder_name: String,
    pub folder_path: PathBuf,
    pub display_name: String,
    pub game_mode: GameMode,
    pub last_played: u64,
    pub day_time: u64,
    pub size_bytes: u64,
    pub icon_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBackupItem {
    pub file_name: String,
    pub file_path: PathBuf,
    pub created_at: Option<String>,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenshotItem {
    pub file_name: String,
    pub file_path: PathBuf,
    pub created_at: Option<String>,
    pub size_bytes: u64,
}

pub fn list_worlds(game_dir: &Path) -> Result<Vec<WorldItem>, WorkshopError> {
    let saves_dir = game_dir.join("saves");
    if !saves_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut worlds = Vec::new();
    let entries = fs::read_dir(&saves_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let level_dat = path.join("level.dat");
            if level_dat.is_file() {
                if let Ok(item) = parse_world_folder(&path) {
                    worlds.push(item);
                }
            }
        }
    }

    // Sort descending by last_played
    worlds.sort_by_key(|a| std::cmp::Reverse(a.last_played));
    Ok(worlds)
}

pub fn parse_world_folder(world_dir: &Path) -> Result<WorldItem, WorkshopError> {
    let folder_name = world_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let level_dat = world_dir.join("level.dat");
    let bytes = fs::read(&level_dat)?;

    // Decompress if gzip (magic bytes 0x1f, 0x8b)
    let nbt_bytes = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        bytes
    };

    let (display_name, game_mode, last_played, day_time) = parse_level_dat_nbt(&nbt_bytes);
    let size_bytes = calculate_dir_size(world_dir);

    let icon_candidate = world_dir.join("icon.png");
    let icon_path = if icon_candidate.is_file() {
        Some(icon_candidate)
    } else {
        None
    };

    Ok(WorldItem {
        folder_name: folder_name.clone(),
        folder_path: world_dir.to_path_buf(),
        display_name: if display_name.is_empty() {
            folder_name
        } else {
            display_name
        },
        game_mode,
        last_played,
        day_time,
        size_bytes,
        icon_path,
    })
}

// Scans uncompressed Big-Endian NBT bytes for LevelName, GameType, LastPlayed, DayTime
pub fn parse_level_dat_nbt(bytes: &[u8]) -> (String, GameMode, u64, u64) {
    let mut name = String::new();
    let mut mode = GameMode::Survival;
    let mut last_played = 0;
    let mut day_time = 0;

    let mut i = 0;
    while i < bytes.len() {
        // Tag String (0x08): [tag_type=0x08, name_len: u16, name: bytes, str_len: u16, str_bytes]
        if bytes[i] == 0x08 && i + 3 <= bytes.len() {
            let n_len = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]) as usize;
            let n_start = i + 3;
            if n_start + n_len + 2 <= bytes.len() {
                let tag_name = &bytes[n_start..n_start + n_len];
                let val_len_pos = n_start + n_len;
                let val_len =
                    u16::from_be_bytes([bytes[val_len_pos], bytes[val_len_pos + 1]]) as usize;
                let val_start = val_len_pos + 2;

                if val_start + val_len <= bytes.len() {
                    if tag_name == b"LevelName" {
                        name = String::from_utf8_lossy(&bytes[val_start..val_start + val_len])
                            .to_string();
                    }
                    i = val_start + val_len;
                    continue;
                }
            }
        }

        // Tag Int (0x03): [tag_type=0x03, name_len: u16, name: bytes, value: i32]
        if bytes[i] == 0x03 && i + 3 <= bytes.len() {
            let n_len = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]) as usize;
            let n_start = i + 3;
            if n_start + n_len + 4 <= bytes.len() {
                let tag_name = &bytes[n_start..n_start + n_len];
                let val_pos = n_start + n_len;
                let val = i32::from_be_bytes([
                    bytes[val_pos],
                    bytes[val_pos + 1],
                    bytes[val_pos + 2],
                    bytes[val_pos + 3],
                ]);

                if tag_name == b"GameType" {
                    mode = GameMode::from_i32(val);
                }
                i = val_pos + 4;
                continue;
            }
        }

        // Tag Long (0x04): [tag_type=0x04, name_len: u16, name: bytes, value: i64]
        if bytes[i] == 0x04 && i + 3 <= bytes.len() {
            let n_len = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]) as usize;
            let n_start = i + 3;
            if n_start + n_len + 8 <= bytes.len() {
                let tag_name = &bytes[n_start..n_start + n_len];
                let val_pos = n_start + n_len;
                let val = u64::from_be_bytes([
                    bytes[val_pos],
                    bytes[val_pos + 1],
                    bytes[val_pos + 2],
                    bytes[val_pos + 3],
                    bytes[val_pos + 4],
                    bytes[val_pos + 5],
                    bytes[val_pos + 6],
                    bytes[val_pos + 7],
                ]);

                if tag_name == b"LastPlayed" {
                    last_played = val;
                } else if tag_name == b"DayTime" || tag_name == b"Time" {
                    day_time = val;
                }
                i = val_pos + 8;
                continue;
            }
        }

        i += 1;
    }

    (name, mode, last_played, day_time)
}

pub fn create_world_backup(
    world_folder: &Path,
    backup_base_dir: &Path,
) -> Result<PathBuf, WorkshopError> {
    if !world_folder.is_dir() {
        return Err(WorkshopError::NotFound(world_folder.to_path_buf()));
    }

    let world_item = parse_world_folder(world_folder)?;
    let safe_name: String = world_item
        .display_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();

    let aura_backups_dir = backup_base_dir.join("AuraBackups");
    fs::create_dir_all(&aura_backups_dir)?;

    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S");
    let zip_filename = format!("{}_{}.zip", safe_name, timestamp);
    let zip_path = aura_backups_dir.join(&zip_filename);

    let file = File::create(&zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut zip, world_folder, world_folder, options)?;
    zip.finish()?;

    Ok(zip_path)
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    base_dir: &Path,
    current_dir: &Path,
    options: SimpleFileOptions,
) -> Result<(), WorkshopError> {
    let entries = fs::read_dir(current_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let rel_path = path
            .strip_prefix(base_dir)
            .map_err(std::io::Error::other)?;
        let name_str = rel_path.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(format!("{}/", name_str), options)?;
            add_dir_to_zip(zip, base_dir, &path, options)?;
        } else if path.is_file() {
            // Skip temporary session locks or zip archives inside the world
            if let Some(ext) = path.extension() {
                if ext == "lock" || ext == "zip" {
                    continue;
                }
            }
            zip.start_file(name_str, options)?;
            let mut f = File::open(&path)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}

pub fn restore_world_backup(
    zip_path: &Path,
    target_world_folder: &Path,
) -> Result<(), WorkshopError> {
    if !zip_path.is_file() {
        return Err(WorkshopError::NotFound(zip_path.to_path_buf()));
    }

    if target_world_folder.exists() {
        fs::remove_dir_all(target_world_folder)?;
    }
    fs::create_dir_all(target_world_folder)?;

    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let enclosed_name = file
            .enclosed_name()
            .ok_or_else(|| std::io::Error::other("Invalid path in ZIP"))?
            .to_owned();

        let outpath = target_world_folder.join(enclosed_name);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

pub fn list_world_backups(
    backup_base_dir: &Path,
    world_folder_name: &str,
    display_name: &str,
) -> Result<Vec<WorldBackupItem>, WorkshopError> {
    let aura_backups = backup_base_dir.join("AuraBackups");
    if !aura_backups.is_dir() {
        return Ok(Vec::new());
    }

    let safe_name: String = display_name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();

    let mut backups = Vec::new();
    let entries = fs::read_dir(&aura_backups)?;

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().map(|e| e == "zip").unwrap_or(false) {
            let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            if fname.starts_with(&safe_name) || fname.starts_with(world_folder_name) {
                let meta = p.metadata().ok();
                let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let created_at = meta
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339());

                backups.push(WorldBackupItem {
                    file_name: fname,
                    file_path: p,
                    created_at,
                    size_bytes,
                });
            }
        }
    }

    backups.sort_by_key(|a| std::cmp::Reverse(a.file_name.clone()));
    Ok(backups)
}

pub fn list_screenshots(screenshots_dir: &Path) -> Result<Vec<ScreenshotItem>, WorkshopError> {
    if !screenshots_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    let entries = fs::read_dir(screenshots_dir)?;

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_file()
            && p.extension()
                .map(|e| e.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        {
            let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
            let meta = p.metadata().ok();
            let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let created_at = meta
                .and_then(|m| m.modified().ok())
                .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339());

            list.push(ScreenshotItem {
                file_name: fname,
                file_path: p,
                created_at,
                size_bytes,
            });
        }
    }

    list.sort_by_key(|a| std::cmp::Reverse(a.file_name.clone()));
    Ok(list)
}

fn calculate_dir_size(dir: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            } else if p.is_dir() {
                total += calculate_dir_size(&p);
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_level_dat_bytes(name: &str, mode: i32, last_played: u64) -> Vec<u8> {
        let mut nbt = Vec::new();

        // LevelName string
        nbt.push(0x08);
        nbt.extend_from_slice(&(b"LevelName".len() as u16).to_be_bytes());
        nbt.extend_from_slice(b"LevelName");
        nbt.extend_from_slice(&(name.len() as u16).to_be_bytes());
        nbt.extend_from_slice(name.as_bytes());

        // GameType int
        nbt.push(0x03);
        nbt.extend_from_slice(&(b"GameType".len() as u16).to_be_bytes());
        nbt.extend_from_slice(b"GameType");
        nbt.extend_from_slice(&mode.to_be_bytes());

        // LastPlayed long
        nbt.push(0x04);
        nbt.extend_from_slice(&(b"LastPlayed".len() as u16).to_be_bytes());
        nbt.extend_from_slice(b"LastPlayed");
        nbt.extend_from_slice(&last_played.to_be_bytes());

        nbt
    }

    #[test]
    fn test_parse_worlds_and_nbt() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path();
        let world_dir = game_dir.join("saves").join("MySurvivalWorld");
        fs::create_dir_all(&world_dir).unwrap();

        let nbt = make_test_level_dat_bytes("Awesome Land", 0, 1730000000123);
        fs::write(world_dir.join("level.dat"), &nbt).unwrap();
        fs::write(world_dir.join("icon.png"), b"fake_png").unwrap();

        let worlds = list_worlds(game_dir).unwrap();
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].display_name, "Awesome Land");
        assert_eq!(worlds[0].game_mode, GameMode::Survival);
        assert_eq!(worlds[0].last_played, 1730000000123);
        assert!(worlds[0].icon_path.is_some());
    }

    #[test]
    fn test_backup_create_and_restore() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path();
        let world_dir = game_dir.join("saves").join("TestWorld");
        fs::create_dir_all(&world_dir).unwrap();

        let nbt = make_test_level_dat_bytes("Test World", 1, 100);
        fs::write(world_dir.join("level.dat"), &nbt).unwrap();
        fs::write(world_dir.join("chunk_0_0.mca"), b"data_chunk").unwrap();

        let backups_dir = game_dir.join("backups");
        let zip_path = create_world_backup(&world_dir, &backups_dir).unwrap();
        assert!(zip_path.is_file());

        let backups = list_world_backups(&backups_dir, "TestWorld", "Test World").unwrap();
        assert_eq!(backups.len(), 1);

        // Delete world and restore
        let restore_dir = game_dir.join("saves").join("RestoredWorld");
        restore_world_backup(&zip_path, &restore_dir).unwrap();

        assert!(restore_dir.join("level.dat").is_file());
        assert!(restore_dir.join("chunk_0_0.mca").is_file());
    }

    #[test]
    fn test_list_screenshots() {
        let dir = tempdir().unwrap();
        let screens_dir = dir.path().join("screenshots");
        fs::create_dir_all(&screens_dir).unwrap();

        fs::write(screens_dir.join("2026-11-01_12.00.00.png"), b"png1").unwrap();
        fs::write(screens_dir.join("2026-11-02_14.00.00.png"), b"png2").unwrap();
        fs::write(screens_dir.join("not_a_screenshot.txt"), b"txt").unwrap();

        let list = list_screenshots(&screens_dir).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].file_name, "2026-11-02_14.00.00.png");
    }
}
