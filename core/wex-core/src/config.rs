use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::error::Result;

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
}

impl WexPaths {
    pub fn new(custom_root: Option<PathBuf>) -> Self {
        let root = custom_root.unwrap_or_else(|| {
            if let Ok(app_data) = std::env::var("APPDATA") {
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

        Self {
            root,
            instances_dir,
            libraries_dir,
            assets_dir,
            versions_dir,
            runtime_dir,
            logs_dir,
            config_file,
        }
    }

    pub fn instance_dir(&self, version: &str) -> PathBuf {
        self.instances_dir.join(version)
    }

    pub fn instance_natives_dir(&self, version: &str) -> PathBuf {
        self.instance_dir(version).join("natives")
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

    #[serde(default = "default_nickname")]
    pub nickname: String,

    #[serde(default = "default_memory")]
    pub max_memory_mb: u32,

    #[serde(default)]
    pub custom_java_path: Option<PathBuf>,

    #[serde(default)]
    pub jvm_args: Vec<String>,
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

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            selected_version: default_version(),
            nickname: default_nickname(),
            max_memory_mb: default_memory(),
            custom_java_path: None,
            jvm_args: Vec::new(),
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
    fn test_config_serialization() {
        let tmp = tempdir().unwrap();
        let config_path = tmp.path().join("config.json");

        let mut cfg = LauncherConfig::default();
        cfg.nickname = "TestUser".to_string();
        cfg.max_memory_mb = 8192;
        cfg.save(&config_path).unwrap();

        let loaded = LauncherConfig::load_or_default(&config_path);
        assert_eq!(loaded.nickname, "TestUser");
        assert_eq!(loaded.max_memory_mb, 8192);
        assert_eq!(loaded.selected_version, "1.20.1");
    }
}
