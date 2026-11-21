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

    /// Instance directory for `version`. A version that is not a plain single path
    /// component (traversal, separators, drive letters) cannot escape `instances/`: it
    /// maps to the inert `_invalid` folder.
    pub fn instance_dir(&self, version: &str) -> PathBuf {
        let plain = !version.is_empty()
            && version != "."
            && version != ".."
            && version
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'));
        if !plain {
            tracing::warn!(version, "unsafe instance version, using instances/_invalid");
        }
        self.instances_dir
            .join(if plain { version } else { "_invalid" })
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

    /// Winter look in the season (snowfall, snow on the build tiles). On by default.
    #[serde(default = "default_winter_look", alias = "winterLook")]
    pub winter_look: bool,

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

    #[serde(
        default = "default_fast_launch",
        alias = "fastLaunch",
        alias = "fast_launch"
    )]
    pub fast_launch: bool,

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

    #[serde(
        default = "default_game_window_behavior",
        alias = "game_window_behavior",
        alias = "gameWindowBehavior"
    )]
    pub game_window_behavior: String,

    /// Azure application (client) ID for Microsoft sign-in, set by the user.
    /// The `WEX_MS_CLIENT_ID` environment variable takes precedence (accounts::microsoft_client_id).
    #[serde(
        default,
        alias = "microsoftClientId",
        skip_serializing_if = "Option::is_none"
    )]
    pub microsoft_client_id: Option<String>,

    /// «Облачные миры»: the Aura world manager's address, set by the user.
    /// `WEX_CLOUD_URL` takes precedence (wex_social::cloud::CloudConfig).
    #[serde(default, alias = "cloudUrl", skip_serializing_if = "Option::is_none")]
    pub cloud_url: Option<String>,

    /// The manager's shared secret, set by the user; never in the repository.
    /// `WEX_CLOUD_SECRET` takes precedence.
    #[serde(
        default,
        alias = "cloudSecret",
        skip_serializing_if = "Option::is_none"
    )]
    pub cloud_secret: Option<String>,

    /// Launch identity resolved from the active account just before launch.
    /// Runtime only: never written to config.json.
    #[serde(skip)]
    pub session: Option<crate::accounts::GameSession>,
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

fn default_winter_look() -> bool {
    true
}

fn default_nickname() -> String {
    "Player".to_string()
}

#[cfg(windows)]
pub fn get_total_system_ram_mb() -> Option<u64> {
    unsafe {
        use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..std::mem::zeroed()
        };
        if GlobalMemoryStatusEx(&mut status) != 0 {
            Some(status.ullTotalPhys / (1024 * 1024))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
pub fn get_total_system_ram_mb() -> Option<u64> {
    None
}

/// Detect whether the current machine qualifies as weak hardware:
/// e.g. <= 4 logical CPU cores or <= 8 GB of physical RAM.
pub fn is_weak_hardware() -> bool {
    if let Ok(val) = std::env::var("WEX_FORCE_WEAK_PC") {
        if val == "1" || val.eq_ignore_ascii_case("true") {
            return true;
        }
        if val == "0" || val.eq_ignore_ascii_case("false") {
            return false;
        }
    }

    let logical_cores = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4);
    if logical_cores <= 4 {
        return true;
    }

    if let Some(ram_mb) = get_total_system_ram_mb() {
        if ram_mb <= 8192 {
            return true;
        }
    }

    false
}

fn default_memory() -> u32 {
    if is_weak_hardware() {
        3072
    } else {
        4096
    }
}

fn default_appearance() -> String {
    "dark".to_string()
}

fn default_accent() -> String {
    "blue".to_string()
}

fn default_graphics_preset() -> String {
    if is_weak_hardware() {
        "Low".to_string()
    } else {
        "High".to_string()
    }
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

#[cfg(windows)]
pub fn get_system_total_memory_mb() -> u32 {
    use std::mem::zeroed;
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    unsafe {
        let mut status: MemoryStatusEx = zeroed();
        status.length = std::mem::size_of::<MemoryStatusEx>() as u32;
        if GlobalMemoryStatusEx(&mut status) != 0 {
            (status.total_phys / (1024 * 1024)) as u32
        } else {
            16384
        }
    }
}

#[cfg(not(windows))]
pub fn get_system_total_memory_mb() -> u32 {
    16384
}

pub fn default_fast_launch() -> bool {
    // Off by default: prewarm starts a real Minecraft window that can appear
    // over other fullscreen apps while it loads. The user turns it on in
    // Settings › Лаунчер › «Готовить игру заранее».
    false
}

fn default_true() -> bool {
    true
}

fn default_game_window_behavior() -> String {
    "minimize".to_string()
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
            winter_look: default_winter_look(),
            quick_play_multiplayer: None,
            appearance: default_appearance(),
            accent: default_accent(),
            fast_start_jvm: false,
            fast_launch: default_fast_launch(),
            window_state: WindowStateConfig::default(),
            minimize_to_tray: default_minimize_to_tray(),
            friend_notifications: true,
            game_window_behavior: default_game_window_behavior(),
            microsoft_client_id: None,
            cloud_url: None,
            cloud_secret: None,
            session: None,
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
            winter_look: default_winter_look(),
            quick_play_multiplayer: None,
            appearance: ipc.appearance,
            accent: ipc.accent,
            fast_start_jvm: ipc.fast_start_jvm,
            fast_launch: ipc.fast_launch,
            window_state: WindowStateConfig::default(),
            minimize_to_tray: default_minimize_to_tray(),
            friend_notifications: default_true(),
            game_window_behavior: default_game_window_behavior(),
            microsoft_client_id: None,
            cloud_url: None,
            cloud_secret: None,
            session: None,
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
            fast_launch: core.fast_launch,
        }
    }
}

impl LauncherConfig {
    /// Read the config, or defaults when there is none. An unreadable file is
    /// copied to `config.json.broken` (and a warning logged) before defaults are
    /// used, so a damaged file never silently costs the player their settings.
    pub fn load_or_default(path: &Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                tracing::warn!(
                    "config {} is unreadable ({}), using defaults",
                    path.display(),
                    e
                );
                return Self::default();
            }
        };
        match serde_json::from_str(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                let broken = broken_config_path(path);
                tracing::warn!(
                    "config {} is damaged ({}), kept as {} and defaults are used",
                    path.display(),
                    e,
                    broken.display()
                );
                let _ = fs::copy(path, &broken);
                Self::default()
            }
        }
    }

    /// Write the config atomically: a temporary file next to it, then a rename,
    /// so a crash mid-write leaves the previous file intact.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        write_file_atomic(path, json.as_bytes())?;
        Ok(())
    }
}

/// Serialises load-modify-save of `config.json` within the process.
static CONFIG_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Hold the config lock for a load-modify-save written out by hand. Prefer
/// [`update_config`]; never hold the guard across an `.await` or call
/// `update_config` while holding it (the lock is not re-entrant).
pub fn lock_config() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_LOCK.lock().unwrap_or_else(|p| p.into_inner())
}

/// Load the config, let `change` edit it and save it, all under one process-wide
/// lock, so two commands changing different settings never undo each other.
/// The file is written only when something actually changed.
pub fn update_config<R>(path: &Path, change: impl FnOnce(&mut LauncherConfig) -> R) -> Result<R> {
    let _guard = lock_config();
    let mut cfg = LauncherConfig::load_or_default(path);
    let before = serde_json::to_value(&cfg)?;
    let out = change(&mut cfg);
    if !path.exists() || serde_json::to_value(&cfg)? != before {
        cfg.save(path)?;
    }
    Ok(out)
}

fn broken_config_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".broken");
    path.with_file_name(name)
}

/// Write `data` to a unique temporary file beside `path`, flush it to disk and
/// rename it over `path` (the rename replaces the old file on Windows too).
pub fn write_file_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = path.with_file_name(name);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fast_launch_is_off_by_default() {
        // Prewarm opens a real game window; it must never start unless the user turned it on.
        assert!(!LauncherConfig::default().fast_launch);
        let cfg: LauncherConfig = serde_json::from_str("{}").unwrap();
        assert!(!cfg.fast_launch);
    }

    #[test]
    fn cloud_settings_are_kept_and_not_written_when_unset() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("config.json");
        LauncherConfig::default().save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("cloud_"), "{text}");
        std::fs::write(
            &path,
            r#"{"cloudUrl":"http://h.example:1/","cloudSecret":"s"}"#,
        )
        .unwrap();
        let mut cfg = LauncherConfig::load_or_default(&path);
        assert_eq!(cfg.cloud_url.as_deref(), Some("http://h.example:1/"));
        assert_eq!(cfg.cloud_secret.as_deref(), Some("s"));
        cfg.nickname = "Steve".into();
        cfg.save(&path).unwrap();
        let again = LauncherConfig::load_or_default(&path);
        assert_eq!(again.cloud_secret.as_deref(), Some("s"));
    }

    #[test]
    fn damaged_config_is_kept_as_broken_and_save_is_atomic() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("config.json");
        std::fs::write(&path, b"{\"nickname\": \"Ste").unwrap();
        let cfg = LauncherConfig::load_or_default(&path);
        assert_eq!(cfg.nickname, LauncherConfig::default().nickname);
        let broken = tmp.path().join("config.json.broken");
        assert_eq!(std::fs::read(&broken).unwrap(), b"{\"nickname\": \"Ste");

        let cfg = LauncherConfig {
            nickname: "Steve".into(),
            ..Default::default()
        };
        cfg.save(&path).unwrap();
        assert_eq!(LauncherConfig::load_or_default(&path).nickname, "Steve");
        let names: Vec<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().all(|n| !n.ends_with(".tmp")),
            "no temporary files left: {names:?}"
        );
    }

    #[test]
    fn update_config_never_loses_a_concurrent_change() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("config.json");
        update_config(&path, |c| c.max_memory_mb = 2048).unwrap();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for _ in 0..25 {
                        update_config(&path, |c| c.max_memory_mb += 1).unwrap();
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().unwrap();
        }
        assert_eq!(LauncherConfig::load_or_default(&path).max_memory_mb, 2248);
    }

    #[test]
    fn update_config_skips_unchanged_writes() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("config.json");
        update_config(&path, |_| ()).unwrap();
        assert!(path.exists(), "a missing config is created");
        std::fs::write(&path, b"{\"nickname\":\"Alex\"}").unwrap();
        let name = update_config(&path, |c| c.nickname.clone()).unwrap();
        assert_eq!(name, "Alex");
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"nickname\":\"Alex\"}");
    }

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
        assert_eq!(loaded.game_window_behavior, "minimize");
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

    #[test]
    fn test_weak_hardware_detection_and_defaults() {
        std::env::set_var("WEX_FORCE_WEAK_PC", "true");
        assert!(is_weak_hardware());
        assert_eq!(default_memory(), 3072);
        assert_eq!(default_graphics_preset(), "Low");

        let def_cfg = LauncherConfig::default();
        assert_eq!(def_cfg.max_memory_mb, 3072);
        assert_eq!(def_cfg.graphics_preset, "Low");

        std::env::set_var("WEX_FORCE_WEAK_PC", "false");
        assert!(!is_weak_hardware());
        assert_eq!(default_memory(), 4096);
        assert_eq!(default_graphics_preset(), "High");

        std::env::remove_var("WEX_FORCE_WEAK_PC");
    }

    #[test]
    fn instance_dir_cannot_escape_instances() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        for bad in ["..", "../x", r"..\x", r"C:\x", "a/b", "", "/abs"] {
            let d = paths.instance_dir(bad);
            assert_eq!(d, paths.instances_dir.join("_invalid"), "{bad:?}");
        }
        assert_eq!(
            paths.instance_dir("1.21.1"),
            paths.instances_dir.join("1.21.1")
        );
    }
}
