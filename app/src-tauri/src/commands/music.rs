use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::path::Path;
use tauri::State;
use wex_extras::music::{MusicManager, MusicSettings};
use wex_ipc::commands::{
    MusicGetTrackDataRequest, MusicGetTrackDataResponse, MusicPlaylistResponse, MusicSettingsDto,
    MusicTrackDto,
};

use super::launch::PlayState;

pub fn music_get_playlist_impl(root: &Path) -> Result<MusicPlaylistResponse, String> {
    let music_dir = root.join("music");
    let manager = MusicManager::new(&music_dir);
    let tracks = manager.scan_tracks().map_err(|e| e.to_string())?;

    let mut dtos = Vec::with_capacity(tracks.len());
    for t in tracks {
        let cover_data_url = if let Some(ref cover_name) = t.cover_file {
            match manager.get_cover_bytes(cover_name) {
                Ok((bytes, mime)) => {
                    Some(format!("data:{};base64,{}", mime, BASE64.encode(&bytes)))
                }
                Err(_) => None,
            }
        } else {
            None
        };

        dtos.push(MusicTrackDto {
            id: t.id,
            title: t.title,
            artist: t.artist,
            file_name: t.file_name,
            cover_data_url,
            duration_seconds: t.duration_seconds,
        });
    }

    Ok(MusicPlaylistResponse {
        tracks: dtos,
        music_dir: music_dir.to_string_lossy().to_string(),
    })
}

pub fn music_get_track_data_impl(
    root: &Path,
    file_name: &str,
) -> Result<MusicGetTrackDataResponse, String> {
    let music_dir = root.join("music");
    let manager = MusicManager::new(&music_dir);
    let (bytes, mime) = manager
        .get_track_bytes(file_name)
        .map_err(|e| e.to_string())?;

    Ok(MusicGetTrackDataResponse {
        file_name: file_name.to_string(),
        mime_type: mime.to_string(),
        audio_base64: BASE64.encode(&bytes),
    })
}

pub fn music_open_folder_impl(root: &Path) -> Result<(), String> {
    let music_dir = root.join("music");
    let manager = MusicManager::new(&music_dir);
    manager.ensure_music_dir().map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&music_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&music_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&music_dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn music_get_settings_impl(root: &Path) -> Result<MusicSettingsDto, String> {
    let music_dir = root.join("music");
    let manager = MusicManager::new(&music_dir);
    let settings = manager.load_settings();

    Ok(MusicSettingsDto {
        enabled: settings.enabled,
        volume: settings.volume,
        auto_pause_on_launch: settings.auto_pause_on_launch,
    })
}

pub fn music_set_settings_impl(root: &Path, settings: MusicSettingsDto) -> Result<(), String> {
    let music_dir = root.join("music");
    let manager = MusicManager::new(&music_dir);
    let s = MusicSettings {
        enabled: settings.enabled,
        volume: settings.volume,
        auto_pause_on_launch: settings.auto_pause_on_launch,
    };
    manager.save_settings(&s).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn music_get_playlist(
    state: State<'_, PlayState>,
) -> Result<MusicPlaylistResponse, String> {
    music_get_playlist_impl(&state.paths.root)
}

#[tauri::command]
pub async fn music_get_track_data(
    request: MusicGetTrackDataRequest,
    state: State<'_, PlayState>,
) -> Result<MusicGetTrackDataResponse, String> {
    music_get_track_data_impl(&state.paths.root, &request.file_name)
}

#[tauri::command]
pub async fn music_open_folder(state: State<'_, PlayState>) -> Result<(), String> {
    music_open_folder_impl(&state.paths.root)
}

#[tauri::command]
pub async fn music_get_settings(state: State<'_, PlayState>) -> Result<MusicSettingsDto, String> {
    music_get_settings_impl(&state.paths.root)
}

#[tauri::command]
pub async fn music_set_settings(
    settings: MusicSettingsDto,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    music_set_settings_impl(&state.paths.root, settings)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_music_empty_playlist() {
        let tmp = tempdir().unwrap();
        let res = music_get_playlist_impl(tmp.path()).unwrap();
        assert!(res.tracks.is_empty());
        assert!(res.music_dir.ends_with("music"));
    }

    #[test]
    fn test_music_playlist_with_tracks_and_cover() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        fs::create_dir_all(&music_dir).unwrap();

        fs::write(music_dir.join("Artist - Track.mp3"), b"mock mp3 data").unwrap();
        fs::write(music_dir.join("Artist - Track.png"), b"mock png data").unwrap();

        let res = music_get_playlist_impl(tmp.path()).unwrap();
        assert_eq!(res.tracks.len(), 1);
        let track = &res.tracks[0];
        assert_eq!(track.artist, "Artist");
        assert_eq!(track.title, "Track");
        assert_eq!(track.file_name, "Artist - Track.mp3");
        assert!(track.cover_data_url.is_some());
        assert!(track
            .cover_data_url
            .as_ref()
            .unwrap()
            .starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_music_get_track_data() {
        let tmp = tempdir().unwrap();
        let music_dir = tmp.path().join("music");
        fs::create_dir_all(&music_dir).unwrap();

        let audio_content = b"my audio stream bytes";
        fs::write(music_dir.join("theme.ogg"), audio_content).unwrap();

        let res = music_get_track_data_impl(tmp.path(), "theme.ogg").unwrap();
        assert_eq!(res.file_name, "theme.ogg");
        assert_eq!(res.mime_type, "audio/ogg");

        let decoded = BASE64.decode(&res.audio_base64).unwrap();
        assert_eq!(decoded, audio_content);
    }

    #[test]
    fn test_music_get_track_data_security() {
        let tmp = tempdir().unwrap();
        let err = music_get_track_data_impl(tmp.path(), "../something.mp3").unwrap_err();
        assert!(err.contains("traversal"));
    }

    #[test]
    fn test_music_settings_roundtrip() {
        let tmp = tempdir().unwrap();
        let default_s = music_get_settings_impl(tmp.path()).unwrap();
        assert!(default_s.enabled);
        assert_eq!(default_s.volume, 75);

        let new_s = MusicSettingsDto {
            enabled: false,
            volume: 30,
            auto_pause_on_launch: true,
        };
        music_set_settings_impl(tmp.path(), new_s.clone()).unwrap();

        let loaded = music_get_settings_impl(tmp.path()).unwrap();
        assert_eq!(loaded, new_s);
    }
}
