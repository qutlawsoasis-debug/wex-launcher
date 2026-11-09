use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WexPaths {
    pub root: PathBuf,
    pub instances_dir: PathBuf,
    pub libraries_dir: PathBuf,
    pub assets_dir: PathBuf,
    pub versions_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub config_file: PathBuf,
    pub playtime_file: PathBuf,
}

impl WexPaths {
    pub fn new(custom_root: Option<PathBuf>) -> Self {
        let root = custom_root.unwrap_or_else(|| {
            if let Ok(custom_data_dir) = std::env::var("WEX_DATA_DIR") {
                PathBuf::from(custom_data_dir)
            } else if let Ok(app_data) = std::env::var("APPDATA") {
                PathBuf::from(app_data).join("Wex")
            } else if let Some(base_dirs) = directories::BaseDirs::new() {
                base_dirs.data_dir().join("Wex")
            } else {
                PathBuf::from(".wex")
            }
        });

        let instances_dir = root.join("instances");
        let libraries_dir = root.join("libraries");
        let assets_dir = root.join("assets");
        let versions_dir = root.join("versions");
        let runtime_dir = root.join("runtime");
        let logs_dir = root.join("logs");
        let config_file = root.join("config.json");
        let playtime_file = root.join("playtime.json");

        Self {
            root,
            instances_dir,
            libraries_dir,
            assets_dir,
            versions_dir,
            runtime_dir,
            logs_dir,
            config_file,
            playtime_file,
        }
    }

    pub fn instance_dir(&self, version: &str) -> PathBuf {
        self.instances_dir.join(version)
    }

    pub fn instance_natives_dir(&self, version: &str) -> PathBuf {
        self.instance_dir(version).join("natives")
    }

    pub fn client_jar(&self, version: &str) -> PathBuf {
        self.versions_dir
            .join(version)
            .join(format!("{}.jar", version))
    }

    pub fn assets_indexes_dir(&self) -> PathBuf {
        self.assets_dir.join("indexes")
    }

    pub fn assets_objects_dir(&self) -> PathBuf {
        self.assets_dir.join("objects")
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.root)?;
        fs::create_dir_all(&self.instances_dir)?;
        fs::create_dir_all(&self.libraries_dir)?;
        fs::create_dir_all(&self.assets_dir)?;
        fs::create_dir_all(self.assets_indexes_dir())?;
        fs::create_dir_all(self.assets_objects_dir())?;
        fs::create_dir_all(&self.versions_dir)?;
        fs::create_dir_all(&self.runtime_dir)?;
        fs::create_dir_all(&self.logs_dir)?;
        Ok(())
    }

    pub fn ensure_instance_dirs(&self, version: &str) -> Result<()> {
        let inst = self.instance_dir(version);
        fs::create_dir_all(&inst)?;
        fs::create_dir_all(inst.join("mods"))?;
        fs::create_dir_all(inst.join("config"))?;
        fs::create_dir_all(inst.join("logs"))?;
        fs::create_dir_all(self.instance_natives_dir(version))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default = "default_version")]
    pub selected_version: String,

    #[serde(
        default = "default_nickname",
        alias = "player_name",
        alias = "playerName"
    )]
    pub nickname: String,

    #[serde(
        default = "default_memory",
        alias = "max_memory_mb",
        alias = "ramMb",
        alias = "ram_gb",
        deserialize_with = "deserialize_ram"
    )]
    pub max_memory_mb: u32,

    #[serde(default, alias = "custom_java_path", alias = "javaPath")]
    pub custom_java_path: Option<PathBuf>,

    #[serde(default, alias = "jvm_args")]
    pub jvm_args: Vec<String>,

    #[serde(
        default = "default_graphics_preset",
        alias = "graphicsPreset",
        alias = "graphics_preset"
    )]
    pub graphics_preset: String,

    #[serde(
        default = "default_discord_rpc",
        alias = "discordRpc",
        alias = "discord_rpc"
    )]
    pub discord_rpc: bool,

    #[serde(default = "default_language")]
    pub language: String,

    #[serde(default = "default_java_mode", alias = "javaMode", alias = "java_mode")]
    pub java_mode: String,

    #[serde(default, alias = "reduceMotion")]
    pub reduce_motion: bool,

    #[serde(default, alias = "reduceTransparency")]
    pub reduce_transparency: bool,

    #[serde(
        default,
        alias = "quickPlayMultiplayer",
        alias = "quick_play_multiplayer"
    )]
    pub quick_play_multiplayer: Option<String>,

    #[serde(default = "default_appearance")]
    pub appearance: String,

    #[serde(default = "default_accent")]
    pub accent: String,

    #[serde(
        default,
        alias = "fastStartJvm",
        alias = "fast_start_jvm",
        alias = "prewarmGame"
    )]
    pub fast_start_jvm: bool,

    #[serde(default)]
    pub window_state: WindowStateConfig,

    #[serde(
        default = "default_minimize_to_tray",
        alias = "minimize_to_tray",
        alias = "minimizeToTray",
        alias = "minimize_to_tray_on_game_start"
    )]
    pub minimize_to_tray: bool,

    #[serde(
        default = "default_true",
        alias = "friendNotifications",
        alias = "friend_notifications"
    )]
    pub friend_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowStateConfig {
    #[serde(default = "default_window_width")]
    pub width: u32,
    #[serde(default = "default_window_height")]
    pub height: u32,
    #[serde(default)]
    pub x: Option<i32>,
    #[serde(default)]
    pub y: Option<i32>,
    #[serde(default)]
    pub is_maximized: bool,
}

fn default_window_width() -> u32 {
    1080
}

fn default_window_height() -> u32 {
    720
}

fn default_minimize_to_tray() -> bool {
    true
}

impl Default for WindowStateConfig {
    fn default() -> Self {
        Self {
            width: default_window_width(),
            height: default_window_height(),
            x: None,
            y: None,
            is_maximized: false,
        }
    }
}

fn deserialize_ram<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let val = u32::deserialize(deserializer)?;
    if val <= 64 {
        Ok(val * 1024)
    } else {
        Ok(val)
    }
}

fn default_version() -> String {
    "1.20.1".to_string()
}

fn default_nickname() -> String {
    "Player".to_string()
}

fn default_memory() -> u32 {
    4096
}

fn default_appearance() -> String {
    "dark".to_string()
}

fn default_accent() -> String {
    "blue".to_string()
}

fn default_graphics_preset() -> String {
    "High".to_string()
}

fn default_discord_rpc() -> bool {
    true
}

fn default_language() -> String {
    "ru".to_string()
}

fn default_java_mode() -> String {
    "auto".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            selected_version: default_version(),
            nickname: default_nickname(),
            max_memory_mb: default_memory(),
            custom_java_path: None,
            jvm_args: Vec::new(),
            graphics_preset: default_graphics_preset(),
            discord_rpc: default_discord_rpc(),
            language: default_language(),
            java_mode: default_java_mode(),
            reduce_motion: false,
            reduce_transparency: false,
            quick_play_multiplayer: None,
            appearance: default_appearance(),
            accent: default_accent(),
            fast_start_jvm: false,
            window_state: WindowStateConfig::default(),
            minimize_to_tray: default_minimize_to_tray(),
            friend_notifications: true,
        }
    }
}

impl From<wex_ipc::models::LauncherConfig> for LauncherConfig {
    fn from(ipc: wex_ipc::models::LauncherConfig) -> Self {
        Self {
            selected_version: ipc.selected_version,
            nickname: ipc.player_name,
            max_memory_mb: if ipc.ram_gb <= 64 {
                ipc.ram_gb * 1024
            } else {
                ipc.ram_gb
            },
            custom_java_path: None,
            jvm_args: Vec::new(),
            graphics_preset: default_graphics_preset(),
            discord_rpc: default_discord_rpc(),
            language: default_language(),
            java_mode: default_java_mode(),
            reduce_motion: ipc.reduce_motion,
            reduce_transparency: ipc.reduce_transparency,
            quick_play_multiplayer: None,
            appearance: ipc.appearance,
            accent: ipc.accent,
            fast_start_jvm: ipc.fast_start_jvm,
            window_state: WindowStateConfig::default(),
            minimize_to_tray: default_minimize_to_tray(),
            friend_notifications: default_true(),
        }
    }
}

impl From<LauncherConfig> for wex_ipc::models::LauncherConfig {
    fn from(core: LauncherConfig) -> Self {
        Self {
            selected_version: core.selected_version,
            player_name: core.nickname,
            ram_gb: (core.max_memory_mb / 1024).max(1),
            reduce_motion: core.reduce_motion,
            reduce_transparency: core.reduce_transparency,
            appearance: core.appearance,
            accent: core.accent,
            fast_start_jvm: core.fast_start_jvm,
        }
    }
}

impl LauncherConfig {
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(cfg) = serde_json::from_str(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wex_paths() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        paths.ensure_dirs().unwrap();
        paths.ensure_instance_dirs("1.20.1").unwrap();
        paths.ensure_instance_dirs("1.21.1").unwrap();

        assert!(paths.root.exists());
        assert!(paths.instance_dir("1.20.1").exists());
        assert!(paths.instance_dir("1.21.1").exists());
        assert!(paths.instance_dir("1.20.1").join("mods").exists());
        assert!(paths.instance_dir("1.21.1").join("mods").exists());
        assert!(paths.assets_indexes_dir().exists());
    }

    #[test]
    fn test_wex_paths_env_override() {
        let tmp = tempdir().unwrap();
        let custom_dir = tmp.path().join("WexCustom");
        std::env::set_var("WEX_DATA_DIR", &custom_dir);

        let paths = WexPaths::new(None);
        assert_eq!(paths.root, custom_dir);
        std::env::remove_var("WEX_DATA_DIR");
    }

    #[test]
    fn test_config_serialization() {
        let tmp = tempdir().unwrap();
        let config_path = tmp.path().join("config.json");

        let cfg = LauncherConfig {
            nickname: "TestUser".to_string(),
            max_memory_mb: 8192,
            ..Default::default()
        };
        cfg.save(&config_path).unwrap();

        let loaded = LauncherConfig::load_or_default(&config_path);
        assert_eq!(loaded.nickname, "TestUser");
        assert_eq!(loaded.max_memory_mb, 8192);
        assert_eq!(loaded.selected_version, "1.20.1");
    }

    #[test]
    fn test_window_state_and_minimize_to_tray_config() {
        let tmp = tempdir().unwrap();
        let config_path = tmp.path().join("config.json");

        // Test defaults
        let default_cfg = LauncherConfig::default();
        assert!(default_cfg.minimize_to_tray);
        assert_eq!(default_cfg.window_state.width, 1080);
        assert_eq!(default_cfg.window_state.height, 720);
        assert_eq!(default_cfg.window_state.x, None);
        assert_eq!(default_cfg.window_state.y, None);
        assert!(!default_cfg.window_state.is_maximized);

        // Test custom state persistence
        let custom_cfg = LauncherConfig {
            minimize_to_tray: false,
            window_state: WindowStateConfig {
                width: 1440,
                height: 900,
                x: Some(250),
                y: Some(180),
                is_maximized: true,
            },
            ..Default::default()
        };
        custom_cfg.save(&config_path).unwrap();

        let loaded = LauncherConfig::load_or_default(&config_path);
        assert!(!loaded.minimize_to_tray);
        assert_eq!(loaded.window_state.width, 1440);
        assert_eq!(loaded.window_state.height, 900);
        assert_eq!(loaded.window_state.x, Some(250));
        assert_eq!(loaded.window_state.y, Some(180));
        assert!(loaded.window_state.is_maximized);
    }
}
