//! «Свой сервер»: a dedicated Fabric server for a group of friends.
//!
//! Every server lives in its own folder `<data root>/servers/<id>/`:
//!
//! ```text
//! servers/<id>/
//!   wex-server.json        Wex record: settings, EULA consent, what was installed
//!   server.properties      written from the settings, other keys are kept
//!   eula.txt               only after the player accepted the EULA in the UI
//!   fabric-server.json     Fabric server profile used to build the classpath
//!   mods/                  Aura-Pack mods meant for the server (+ the player's own)
//!   world/                 the world; installs and updates never touch it
//! ```
//!
//! The vanilla server jar is shared per version in `versions/<v>/<v>-server.jar`
//! and Fabric libraries share `libraries/` with the client.

pub mod install;
pub mod net;
pub mod process;
pub mod properties;

use crate::error::{Result, WexError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub use install::{install_server, InstallOptions};
pub use process::{
    build_server_args, parse_player_event, start_server, PlayerEvent, ServerLogLine, ServerProcess,
    ServerSnapshot, ServerStatus,
};

pub const SUPPORTED_VERSIONS: [&str; 2] = ["1.20.1", "1.21.1"];
pub const RECORD_FILE: &str = "wex-server.json";
pub const EULA_URL: &str = "https://aka.ms/MinecraftEULA";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Peaceful,
    Easy,
    #[default]
    Normal,
    Hard,
}

/// What the player sets in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerSettings {
    /// Shown to friends in the server list (`motd`).
    pub name: String,
    pub max_players: u32,
    pub gamemode: GameMode,
    pub difficulty: Difficulty,
    pub whitelist: bool,
    /// Only licensed Microsoft accounts may join. Off lets offline accounts in,
    /// which also lets anyone use any nickname.
    pub online_mode: bool,
    pub pvp: bool,
    pub port: u16,
    pub memory_mb: u32,
    /// Address friends use over the internet (the router's public IP or a
    /// domain), typed by the player. Wex does not invent or detect one.
    pub public_address: Option<String>,
    /// `server-ip`: empty listens on every interface. Not exposed in the UI;
    /// tests bind to 127.0.0.1.
    pub bind_ip: Option<String>,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            name: "Сервер друзей".into(),
            max_players: 8,
            gamemode: GameMode::Survival,
            difficulty: Difficulty::Normal,
            whitelist: false,
            online_mode: true,
            pvp: true,
            port: 25565,
            memory_mb: 3072,
            public_address: None,
            bind_ip: None,
        }
    }
}

fn bad(msg: &str) -> WexError {
    WexError::Server(msg.to_string())
}

impl ServerSettings {
    pub fn validate(&self) -> Result<()> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > 59 {
            return Err(bad("Название сервера: от 1 до 59 символов"));
        }
        if name.chars().any(|c| c.is_control()) {
            return Err(bad("В названии сервера есть недопустимые символы"));
        }
        if !(1..=100).contains(&self.max_players) {
            return Err(bad("Число мест: от 1 до 100"));
        }
        if self.port < 1024 {
            return Err(bad("Порт: от 1024 до 65535"));
        }
        if !(1024..=32768).contains(&self.memory_mb) {
            return Err(bad("Память сервера: от 1024 до 32768 МБ"));
        }
        if let Some(addr) = self.public_address.as_deref() {
            let ok = !addr.is_empty()
                && addr.len() <= 253
                && addr
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
            if !ok {
                return Err(bad(
                    "Внешний адрес: домен или IP, например play.example.com",
                ));
            }
        }
        if let Some(ip) = self.bind_ip.as_deref() {
            if !ip.is_empty() && ip.parse::<std::net::IpAddr>().is_err() {
                return Err(bad("Некорректный адрес привязки сервера"));
            }
        }
        Ok(())
    }

    fn normalized(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.public_address = self
            .public_address
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty());
        self
    }
}

/// How the server-side mod set was decided.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModSource {
    /// The manifest marks every mod with `side`.
    Manifest,
    /// No `side` in the manifest: each jar's own `fabric.mod.json`
    /// `environment` decided (mods declaring `client` stay off the server).
    FabricModJson,
    /// Pack mods were not installed (vanilla Fabric server).
    NoPackMods,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallInfo {
    pub minecraft: String,
    pub fabric_loader: String,
    pub pack_version: Option<String>,
    pub mod_source: ModSource,
    /// File names in `mods/` that Wex placed there. Other files are the player's.
    pub mods: Vec<String>,
    /// SHA-256 of pack jars found to be client-only, so they are not fetched again.
    #[serde(default)]
    pub client_only: Vec<String>,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerRecord {
    pub id: String,
    pub version: String,
    pub created_at: String,
    pub settings: ServerSettings,
    /// Set only by [`ServerStore::accept_eula`], i.e. by the player in the UI.
    pub eula_accepted_at: Option<String>,
    pub installed: Option<InstallInfo>,
}

/// Ids are generated by Wex; anything else is rejected before it reaches a path.
pub fn validate_id(id: &str) -> Result<()> {
    let ok = id.len() == 12
        && id.starts_with("srv-")
        && id[4..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    if ok {
        Ok(())
    } else {
        Err(bad("Неизвестный сервер"))
    }
}

pub fn validate_version(version: &str) -> Result<()> {
    if SUPPORTED_VERSIONS.contains(&version) {
        Ok(())
    } else {
        Err(WexError::UnsupportedVersion(version.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ServerStore {
    dir: PathBuf,
}

impl ServerStore {
    pub fn new(data_root: &Path) -> Self {
        Self {
            dir: data_root.join("servers"),
        }
    }

    pub fn servers_dir(&self) -> &Path {
        &self.dir
    }

    pub fn server_dir(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        Ok(self.dir.join(id))
    }

    pub fn list(&self) -> Vec<ServerRecord> {
        let mut out: Vec<ServerRecord> = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                self.load(&name).ok()
            })
            .collect();
        out.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        out
    }

    pub fn load(&self, id: &str) -> Result<ServerRecord> {
        let path = self.server_dir(id)?.join(RECORD_FILE);
        let text = fs::read_to_string(&path).map_err(|_| bad("Сервер не найден"))?;
        let rec: ServerRecord = serde_json::from_str(&text)?;
        if rec.id != id {
            return Err(bad("Файл сервера повреждён"));
        }
        Ok(rec)
    }

    pub fn save(&self, rec: &ServerRecord) -> Result<()> {
        let dir = self.server_dir(&rec.id)?;
        fs::create_dir_all(&dir)?;
        let tmp = dir.join(format!("{RECORD_FILE}.tmp"));
        fs::write(&tmp, serde_json::to_vec_pretty(rec)?)?;
        fs::rename(&tmp, dir.join(RECORD_FILE))?;
        Ok(())
    }

    pub fn create(&self, version: &str, settings: ServerSettings) -> Result<ServerRecord> {
        validate_version(version)?;
        let settings = settings.normalized();
        settings.validate()?;
        let id = loop {
            let id = format!("srv-{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
            if !self.dir.join(&id).exists() {
                break id;
            }
        };
        let rec = ServerRecord {
            id,
            version: version.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            settings,
            eula_accepted_at: None,
            installed: None,
        };
        self.save(&rec)?;
        Ok(rec)
    }

    /// Saves new settings and rewrites `server.properties` if the server was installed.
    pub fn update_settings(&self, id: &str, settings: ServerSettings) -> Result<ServerRecord> {
        let settings = settings.normalized();
        settings.validate()?;
        let mut rec = self.load(id)?;
        rec.settings = settings;
        self.save(&rec)?;
        if rec.installed.is_some() {
            self.write_properties(&rec)?;
        }
        Ok(rec)
    }

    pub fn write_properties(&self, rec: &ServerRecord) -> Result<()> {
        let path = self.server_dir(&rec.id)?.join("server.properties");
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let merged =
            properties::merge_properties(&existing, &properties::managed_properties(&rec.settings));
        fs::write(path, merged)?;
        Ok(())
    }

    /// The player accepted the Minecraft EULA in the UI. Nothing else writes `eula.txt`.
    pub fn accept_eula(&self, id: &str) -> Result<ServerRecord> {
        let mut rec = self.load(id)?;
        let now = chrono::Utc::now().to_rfc3339();
        let dir = self.server_dir(id)?;
        fs::write(
            dir.join("eula.txt"),
            format!(
                "#Accepted by the player in Wex at {now}.\n#By changing the setting below to TRUE you are indicating your agreement to our EULA ({EULA_URL}).\neula=true\n"
            ),
        )?;
        rec.eula_accepted_at = Some(now);
        self.save(&rec)?;
        Ok(rec)
    }

    pub fn eula_accepted(&self, rec: &ServerRecord) -> bool {
        rec.eula_accepted_at.is_some()
            && self
                .server_dir(&rec.id)
                .ok()
                .and_then(|d| fs::read_to_string(d.join("eula.txt")).ok())
                .and_then(|t| properties::read_property(&t, "eula"))
                .is_some_and(|v| v.trim().eq_ignore_ascii_case("true"))
    }

    /// Permanently removes the server folder, world included. The UI asks first.
    pub fn delete(&self, id: &str) -> Result<()> {
        let dir = self.server_dir(id)?;
        self.load(id)?;
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    /// Names in `whitelist.json`, as the server keeps it.
    pub fn whitelist(&self, id: &str) -> Vec<String> {
        #[derive(Deserialize)]
        struct Entry {
            name: String,
        }
        self.server_dir(id)
            .ok()
            .and_then(|d| fs::read_to_string(d.join("whitelist.json")).ok())
            .and_then(|t| serde_json::from_str::<Vec<Entry>>(&t).ok())
            .map(|v| v.into_iter().map(|e| e.name).collect())
            .unwrap_or_default()
    }
}

/// A Minecraft player name as the server accepts it in `whitelist add`.
pub fn valid_player_name(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, ServerStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = ServerStore::new(tmp.path());
        (tmp, store)
    }

    #[test]
    fn create_list_load_both_versions() {
        let (_t, store) = store();
        let a = store.create("1.20.1", ServerSettings::default()).unwrap();
        let b = store
            .create(
                "1.21.1",
                ServerSettings {
                    name: "  Дача  ".into(),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_ne!(a.id, b.id);
        validate_id(&a.id).unwrap();
        assert_eq!(b.settings.name, "Дача");
        let list = store.list();
        assert_eq!(list.len(), 2);
        assert_eq!(store.load(&b.id).unwrap(), b);
        assert!(store
            .server_dir(&a.id)
            .unwrap()
            .starts_with(store.servers_dir()));
        assert!(store.create("1.19.4", ServerSettings::default()).is_err());
    }

    #[test]
    fn ids_cannot_escape_the_servers_folder() {
        let (_t, store) = store();
        for id in [
            "../x",
            "srv-../../a",
            "srv-ABCDEF12",
            "srv-1234567",
            "",
            "C:\\x",
        ] {
            assert!(store.server_dir(id).is_err(), "{id}");
        }
    }

    #[test]
    fn settings_validation() {
        let ok = ServerSettings::default();
        ok.validate().unwrap();
        let cases = [
            ServerSettings {
                name: " ".into(),
                ..ok.clone()
            },
            ServerSettings {
                name: "a\u{7}".into(),
                ..ok.clone()
            },
            ServerSettings {
                max_players: 0,
                ..ok.clone()
            },
            ServerSettings {
                port: 80,
                ..ok.clone()
            },
            ServerSettings {
                memory_mb: 512,
                ..ok.clone()
            },
            ServerSettings {
                public_address: Some("a b".into()),
                ..ok.clone()
            },
            ServerSettings {
                bind_ip: Some("nope".into()),
                ..ok.clone()
            },
        ];
        for c in cases {
            assert!(c.validate().is_err(), "{c:?}");
        }
        ServerSettings {
            public_address: Some("play.example.com:25570".into()),
            ..ok
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn eula_is_written_only_on_accept() {
        let (_t, store) = store();
        let rec = store.create("1.21.1", ServerSettings::default()).unwrap();
        let dir = store.server_dir(&rec.id).unwrap();
        assert!(!dir.join("eula.txt").exists());
        assert!(!store.eula_accepted(&rec));
        let rec = store.accept_eula(&rec.id).unwrap();
        assert!(store.eula_accepted(&rec));
        let text = fs::read_to_string(dir.join("eula.txt")).unwrap();
        assert!(text.contains("eula=true"));
        // A record that claims consent but whose eula.txt says false is not accepted.
        fs::write(dir.join("eula.txt"), "eula=false\n").unwrap();
        assert!(!store.eula_accepted(&rec));
    }

    #[test]
    fn settings_update_keeps_world_and_foreign_properties() {
        let (_t, store) = store();
        let mut rec = store.create("1.20.1", ServerSettings::default()).unwrap();
        let dir = store.server_dir(&rec.id).unwrap();
        fs::create_dir_all(dir.join("world/region")).unwrap();
        fs::write(dir.join("world/level.dat"), b"lvl").unwrap();
        fs::write(dir.join("server.properties"), "level-seed=42\nmotd=old\n").unwrap();
        rec.installed = Some(InstallInfo {
            minecraft: "1.20.1".into(),
            fabric_loader: "0.19.5".into(),
            pack_version: None,
            mod_source: ModSource::NoPackMods,
            mods: vec![],
            client_only: vec![],
            installed_at: "now".into(),
        });
        store.save(&rec).unwrap();
        let new = ServerSettings {
            name: "Новый мир".into(),
            max_players: 4,
            ..rec.settings.clone()
        };
        store.update_settings(&rec.id, new).unwrap();
        let props = fs::read_to_string(dir.join("server.properties")).unwrap();
        assert_eq!(
            properties::read_property(&props, "level-seed").as_deref(),
            Some("42")
        );
        assert_eq!(
            properties::read_property(&props, "motd").as_deref(),
            Some("Новый мир")
        );
        assert_eq!(
            properties::read_property(&props, "max-players").as_deref(),
            Some("4")
        );
        assert_eq!(fs::read(dir.join("world/level.dat")).unwrap(), b"lvl");
    }

    #[test]
    fn delete_removes_only_that_server() {
        let (_t, store) = store();
        let a = store.create("1.20.1", ServerSettings::default()).unwrap();
        let b = store.create("1.21.1", ServerSettings::default()).unwrap();
        store.delete(&a.id).unwrap();
        assert!(store.load(&a.id).is_err());
        assert!(store.load(&b.id).is_ok());
    }

    #[test]
    fn whitelist_and_names() {
        let (_t, store) = store();
        let rec = store.create("1.21.1", ServerSettings::default()).unwrap();
        let dir = store.server_dir(&rec.id).unwrap();
        fs::write(
            dir.join("whitelist.json"),
            r#"[{"uuid":"00000000-0000-0000-0000-000000000001","name":"Steve"}]"#,
        )
        .unwrap();
        assert_eq!(store.whitelist(&rec.id), vec!["Steve".to_string()]);
        assert!(valid_player_name("Alex_01"));
        assert!(!valid_player_name("ab"));
        assert!(!valid_player_name("bad name"));
        assert!(!valid_player_name("x;stop"));
    }
}

/// Real install and run, opt-in only (network, ~0.5 GB, a Java process):
/// `WEX_SERVER_E2E_ROOT=<temp dir> WEX_SERVER_E2E_VERSION=1.21.1 [WEX_SERVER_E2E_MODS=1]
///  cargo test -p wex-core --lib server::e2e -- --ignored --nocapture`
/// The server binds to 127.0.0.1 only, on a free port, and is always stopped.
#[cfg(test)]
mod e2e {
    use super::*;
    use std::time::{Duration, Instant};

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn install_start_join_log_stop() {
        let Ok(root) = std::env::var("WEX_SERVER_E2E_ROOT") else {
            return;
        };
        let version = std::env::var("WEX_SERVER_E2E_VERSION").unwrap_or_else(|_| "1.21.1".into());
        let with_mods = std::env::var("WEX_SERVER_E2E_MODS").is_ok();
        let paths = crate::config::WexPaths::new(Some(PathBuf::from(root)));
        let store = ServerStore::new(&paths.root);
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let rec = store
            .create(
                &version,
                ServerSettings {
                    name: "Тест Wex".into(),
                    port,
                    online_mode: false,
                    bind_ip: Some("127.0.0.1".into()),
                    memory_mb: 2048,
                    ..Default::default()
                },
            )
            .unwrap();
        let client = reqwest::Client::new();
        let t = Instant::now();
        let rec = install_server(
            &paths,
            &store,
            &rec.id,
            &client,
            &crate::event::ConsoleEventHandler,
            &InstallOptions {
                concurrency: 8,
                include_pack_mods: with_mods,
            },
        )
        .await
        .unwrap();
        let info = rec.installed.clone().unwrap();
        println!(
            "installed in {:?}: {:?}, {} mods, {} client-only",
            t.elapsed(),
            info.mod_source,
            info.mods.len(),
            info.client_only.len()
        );
        assert!(
            start_server(&paths, &store, &rec.id).await.is_err(),
            "starts without EULA"
        );
        store.accept_eula(&rec.id).unwrap();
        let p = start_server(&paths, &store, &rec.id).await.unwrap();
        let deadline = Instant::now() + Duration::from_secs(240);
        let mut seq = 0;
        while p.status() == ServerStatus::Starting && Instant::now() < deadline {
            let snap = p.snapshot(seq);
            for l in &snap.lines {
                println!("| {}", l.text);
            }
            seq = snap.next_seq;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let ready = p.status() == ServerStatus::Running;
        if ready {
            p.send_command("say hello from wex").await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        p.stop(Duration::from_secs(60)).await.unwrap();
        for l in p.snapshot(seq).lines {
            println!("| {}", l.text);
        }
        println!(
            "status after stop: {:?}, problem: {:?}",
            p.status(),
            p.snapshot(0).problem
        );
        assert!(ready, "server did not reach Done");
        assert!(p.has_exited());
        let dir = store.server_dir(&rec.id).unwrap();
        assert!(dir.join("world").join("level.dat").exists(), "world saved");
        let props = std::fs::read_to_string(dir.join("server.properties")).unwrap();
        assert_eq!(
            properties::read_property(&props, "motd").as_deref(),
            Some("Тест Wex")
        );
    }
}
