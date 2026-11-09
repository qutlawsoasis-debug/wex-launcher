use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MusicError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid track path or traversal attempted: {0}")]
    InvalidPath(String),
    #[error("Track not found: {0}")]
    NotFound(String),
    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub file_name: String,
    pub cover_file: Option<String>,
    pub duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MusicSettings {
    pub enabled: bool,
    pub volume: u8,
    pub auto_pause_on_launch: bool,
}

impl Default for MusicSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 75,
            auto_pause_on_launch: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MediaSessionState {
    pub has_active_session: bool,
    pub source_app: String,
    pub app_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub thumbnail_base64: Option<String>,
    pub timestamp_ms: i64,
}

impl Default for MediaSessionState {
    fn default() -> Self {
        Self {
            has_active_session: false,
            source_app: "Wex".to_string(),
            app_id: "WexLauncher".to_string(),
            title: "".to_string(),
            artist: "".to_string(),
            album: "Wex Music".to_string(),
            is_playing: false,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            thumbnail_base64: None,
            timestamp_ms: 0,
        }
    }
}

pub struct MusicManager {
    music_dir: PathBuf,
}

impl MusicManager {
    pub const SUPPORTED_EXTENSIONS: &'static [&'static str] =
        &["mp3", "wav", "ogg", "flac", "m4a", "aac"];

    pub fn new(music_dir: impl Into<PathBuf>) -> Self {
        Self {
            music_dir: music_dir.into(),
        }
    }

    pub fn music_dir(&self) -> &Path {
        &self.music_dir
    }

    pub fn ensure_music_dir(&self) -> Result<(), std::io::Error> {
        if !self.music_dir.exists() {
            fs::create_dir_all(&self.music_dir)?;
        }
        Ok(())
    }

    pub fn scan_tracks(&self) -> Result<Vec<AudioTrack>, MusicError> {
        self.ensure_music_dir()?;

        let mut tracks = Vec::new();
        let entries = match fs::read_dir(&self.music_dir) {
            Ok(e) => e,
            Err(e) => return Err(MusicError::Io(e)),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = match path.extension().and_then(|s| s.to_str()) {
                Some(e) => e.to_ascii_lowercase(),
                None => continue,
            };

            if !Self::SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
                continue;
            }

            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(f) => f.to_string(),
                None => continue,
            };

            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => file_name.clone(),
            };

            let (artist, title) = Self::parse_track_metadata(&stem);

            // Find cover image in same directory
            let cover_file = Self::find_cover_for_track(&self.music_dir, &stem);

            // Generate stable id from file_name
            let id = hex::encode(Sha256::digest(file_name.as_bytes()));

            tracks.push(AudioTrack {
                id,
                title,
                artist,
                file_name,
                cover_file,
                duration_seconds: None,
            });
        }

        // Sort alphabetically by title
        tracks.sort_by(|a, b| a.title.cmp(&b.title));

        Ok(tracks)
    }

    pub fn parse_track_metadata(stem: &str) -> (String, String) {
        if let Some((artist, title)) = stem.split_once(" - ") {
            let a = artist.trim();
            let t = title.trim();
            if !a.is_empty() && !t.is_empty() {
                return (a.to_string(), t.to_string());
            }
        }

        if let Some((artist, title)) = stem.split_once(" · ") {
            let a = artist.trim();
            let t = title.trim();
            if !a.is_empty() && !t.is_empty() {
                return (a.to_string(), t.to_string());
            }
        }

        ("Wex".to_string(), stem.trim().to_string())
    }

    fn find_cover_for_track(dir: &Path, stem: &str) -> Option<String> {
        let candidates = [
            format!("{}.jpg", stem),
            format!("{}.png", stem),
            format!("{}.jpeg", stem),
            "cover.jpg".to_string(),
            "cover.png".to_string(),
            "cover.jpeg".to_string(),
            "folder.jpg".to_string(),
            "folder.png".to_string(),
        ];

        for c in candidates {
            let p = dir.join(&c);
            if p.is_file() {
                return Some(c);
            }
        }

        None
    }

    pub fn get_track_bytes(&self, file_name: &str) -> Result<(Vec<u8>, &'static str), MusicError> {
        let clean_name = Path::new(file_name);
        if clean_name.components().count() != 1
            || file_name.contains("..")
            || file_name.contains('/')
            || file_name.contains('\\')
        {
            return Err(MusicError::InvalidPath(file_name.to_string()));
        }

        let full_path = self.music_dir.join(file_name);
        if !full_path.is_file() {
            return Err(MusicError::NotFound(file_name.to_string()));
        }

        let ext = full_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let mime = match ext.as_str() {
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "flac" => "audio/flac",
            "m4a" => "audio/mp4",
            "aac" => "audio/aac",
            _ => return Err(MusicError::UnsupportedFormat(ext)),
        };

        let bytes = fs::read(&full_path)?;
        Ok((bytes, mime))
    }

    pub fn get_cover_bytes(&self, file_name: &str) -> Result<(Vec<u8>, &'static str), MusicError> {
        let clean_name = Path::new(file_name);
        if clean_name.components().count() != 1
            || file_name.contains("..")
            || file_name.contains('/')
            || file_name.contains('\\')
        {
            return Err(MusicError::InvalidPath(file_name.to_string()));
        }

        let full_path = self.music_dir.join(file_name);
        if !full_path.is_file() {
            return Err(MusicError::NotFound(file_name.to_string()));
        }

        let ext = full_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            _ => return Err(MusicError::UnsupportedFormat(ext)),
        };

        let bytes = fs::read(&full_path)?;
        Ok((bytes, mime))
    }

    pub fn load_settings(&self) -> MusicSettings {
        let path = self.music_dir.join("music_settings.json");
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str(&content) {
                return settings;
            }
        }
        MusicSettings::default()
    }

    pub fn save_settings(&self, settings: &MusicSettings) -> Result<(), std::io::Error> {
        self.ensure_music_dir()?;
        let path = self.music_dir.join("music_settings.json");
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn write_session_state(&self, state: &MediaSessionState) -> Result<(), std::io::Error> {
        self.ensure_music_dir()?;
        let path = self.music_dir.join("music_session.json");
        let tmp_path = self.music_dir.join("music_session.json.tmp");
        let content = serde_json::to_string_pretty(state)?;
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    pub fn read_session_state(&self) -> Option<MediaSessionState> {
        let path = self.music_dir.join("music_session.json");
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn check_incoming_command(&self) -> Option<String> {
        let cmd_path = self.music_dir.join("music_command.json");
        if !cmd_path.is_file() {
            return None;
        }
        let content = fs::read_to_string(&cmd_path).ok()?;
        let _ = fs::remove_file(&cmd_path);
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            val.get("command")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_track_metadata() {
        assert_eq!(
            MusicManager::parse_track_metadata("C418 - Subwoofer Lullaby"),
            ("C418".to_string(), "Subwoofer Lullaby".to_string())
        );

        assert_eq!(
            MusicManager::parse_track_metadata("Lena Raine · Pigstep"),
            ("Lena Raine".to_string(), "Pigstep".to_string())
        );

        assert_eq!(
            MusicManager::parse_track_metadata("AmbientTrack"),
            ("Wex".to_string(), "AmbientTrack".to_string())
        );
    }

    #[test]
    fn test_scan_tracks_empty_dir() {
        let tmp = tempdir().unwrap();
        let manager = MusicManager::new(tmp.path().join("music"));
        let tracks = manager.scan_tracks().unwrap();
        assert!(tracks.is_empty());
    }

    #[test]
    fn test_scan_tracks_with_files_and_cover() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        fs::create_dir_all(&music_dir).unwrap();

        fs::write(music_dir.join("C418 - Sweden.mp3"), b"mock mp3 data").unwrap();
        fs::write(music_dir.join("C418 - Sweden.jpg"), b"mock jpg data").unwrap();
        fs::write(music_dir.join("Cat.ogg"), b"mock ogg data").unwrap();
        fs::write(music_dir.join("ignore.txt"), b"mock text data").unwrap();

        let manager = MusicManager::new(&music_dir);
        let tracks = manager.scan_tracks().unwrap();

        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Cat");
        assert_eq!(tracks[0].artist, "Wex");
        assert_eq!(tracks[0].file_name, "Cat.ogg");

        assert_eq!(tracks[1].title, "Sweden");
        assert_eq!(tracks[1].artist, "C418");
        assert_eq!(tracks[1].file_name, "C418 - Sweden.mp3");
        assert_eq!(tracks[1].cover_file, Some("C418 - Sweden.jpg".to_string()));
    }

    #[test]
    fn test_get_track_bytes_security_traversal() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        fs::create_dir_all(&music_dir).unwrap();

        let manager = MusicManager::new(&music_dir);

        assert!(matches!(
            manager.get_track_bytes("../secret.txt"),
            Err(MusicError::InvalidPath(_))
        ));

        assert!(matches!(
            manager.get_track_bytes("subdir/secret.mp3"),
            Err(MusicError::InvalidPath(_))
        ));

        assert!(matches!(
            manager.get_track_bytes("nonexistent.mp3"),
            Err(MusicError::NotFound(_))
        ));
    }

    #[test]
    fn test_get_track_bytes_success() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        fs::create_dir_all(&music_dir).unwrap();

        fs::write(music_dir.join("tune.mp3"), b"test audio bytes").unwrap();

        let manager = MusicManager::new(&music_dir);
        let (bytes, mime) = manager.get_track_bytes("tune.mp3").unwrap();
        assert_eq!(bytes, b"test audio bytes");
        assert_eq!(mime, "audio/mpeg");
    }

    #[test]
    fn test_settings_save_and_load() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        let manager = MusicManager::new(&music_dir);

        let default_settings = manager.load_settings();
        assert!(default_settings.enabled);
        assert_eq!(default_settings.volume, 75);

        let custom_settings = MusicSettings {
            enabled: false,
            volume: 42,
            auto_pause_on_launch: true,
        };

        manager.save_settings(&custom_settings).unwrap();

        let loaded = manager.load_settings();
        assert_eq!(loaded, custom_settings);
    }

    #[test]
    fn test_session_state_write_and_read() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        let manager = MusicManager::new(&music_dir);

        let state = MediaSessionState {
            has_active_session: true,
            source_app: "Wex".to_string(),
            app_id: "WexLauncher".to_string(),
            title: "Subwoofer Lullaby".to_string(),
            artist: "C418".to_string(),
            album: "Minecraft Volume Alpha".to_string(),
            is_playing: true,
            position_seconds: 42.0,
            duration_seconds: 210.0,
            thumbnail_base64: Some("data:image/png;base64,mock".to_string()),
            timestamp_ms: 1731000000000,
        };

        manager.write_session_state(&state).unwrap();

        let read = manager.read_session_state().unwrap();
        assert_eq!(read, state);
    }

    #[test]
    fn test_check_incoming_command() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        fs::create_dir_all(&music_dir).unwrap();
        let manager = MusicManager::new(&music_dir);

        assert_eq!(manager.check_incoming_command(), None);

        let cmd_file = music_dir.join("music_command.json");
        fs::write(&cmd_file, r#"{"command":"pause","seek":null}"#).unwrap();

        assert_eq!(manager.check_incoming_command(), Some("pause".to_string()));
        // File must be deleted after consumption
        assert!(!cmd_file.exists());
        assert_eq!(manager.check_incoming_command(), None);
    }
}
