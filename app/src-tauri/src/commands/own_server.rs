//! «Свой сервер»: Tauri commands over `wex_core::server`.
//!
//! A friends' server runs while Wex runs on this PC (Wex can stay in the tray).
//! Friends reach it by the LAN address, or over the internet by the address the
//! host typed after forwarding the port. The lobby tunnel is not used: it is a
//! single shared Playit address for all Wex rooms (see the report).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, State};
use tokio::sync::Mutex;
use wex_core::config::WexPaths;
use wex_core::event::{Event, EventHandler};
use wex_core::server::{
    self, net, InstallInfo, InstallOptions, ServerProcess, ServerRecord, ServerSettings,
    ServerSnapshot, ServerStatus, ServerStore,
};

pub const EVT_OWN_SERVER_PROGRESS: &str = "own-server-progress";
const STOP_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnServerView {
    pub id: String,
    pub version: String,
    pub created_at: String,
    pub settings: ServerSettings,
    pub eula_accepted: bool,
    pub installed: Option<InstallInfo>,
    pub installing: bool,
    pub status: ServerStatus,
    pub players: Vec<String>,
    pub whitelist: Vec<String>,
    /// For friends in the same network, `None` if no LAN interface was found.
    pub lan_address: Option<String>,
    /// For this PC (the host's own server list).
    pub local_address: String,
    /// What the host typed for internet friends; never detected or invented.
    pub public_address: Option<String>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnServerProgress {
    pub id: String,
    pub stage: String,
    pub current: u64,
    pub total: u64,
}

struct ProgressEmitter<F: Fn(OwnServerProgress) + Send + Sync> {
    id: String,
    emit: F,
}

impl<F: Fn(OwnServerProgress) + Send + Sync> EventHandler for ProgressEmitter<F> {
    fn on_event(&self, event: &Event) {
        let p = match event {
            Event::StageStarted {
                stage,
                step,
                total_steps,
            } => OwnServerProgress {
                id: self.id.clone(),
                stage: stage_label(stage),
                current: step.saturating_sub(1) as u64,
                total: *total_steps as u64,
            },
            Event::Progress {
                stage,
                current,
                total,
                ..
            } => OwnServerProgress {
                id: self.id.clone(),
                stage: stage_label(stage),
                current: *current,
                total: *total,
            },
            _ => return,
        };
        (self.emit)(p);
    }
}

fn stage_label(stage: &str) -> String {
    let s = stage.to_ascii_lowercase();
    if s.contains("minecraft server") {
        "Серверный jar Minecraft"
    } else if s.contains("fabric") {
        "Fabric"
    } else if s.contains("java") {
        "Java"
    } else if s.contains("mods") {
        "Моды сборки для сервера"
    } else if s.contains("properties") {
        "Настройки сервера"
    } else {
        "Установка сервера"
    }
    .to_string()
}

pub struct OwnServerState {
    pub paths: WexPaths,
    pub store: ServerStore,
    running: Mutex<HashMap<String, Arc<ServerProcess>>>,
    installing: std::sync::Mutex<HashSet<String>>,
}

struct InstallGuard<'a>(&'a std::sync::Mutex<HashSet<String>>, String);
impl Drop for InstallGuard<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap().remove(&self.1);
    }
}

impl OwnServerState {
    pub fn new(paths: WexPaths) -> Self {
        Self {
            store: ServerStore::new(&paths.root),
            paths,
            running: Mutex::new(HashMap::new()),
            installing: std::sync::Mutex::new(HashSet::new()),
        }
    }

    async fn process(&self, id: &str) -> Option<Arc<ServerProcess>> {
        self.running.lock().await.get(id).cloned()
    }

    pub async fn view(&self, rec: ServerRecord) -> OwnServerView {
        let process = self.process(&rec.id).await;
        let (status, players, problem) = match &process {
            Some(p) => {
                let snap = p.snapshot(u64::MAX);
                (snap.status, snap.players, snap.problem)
            }
            None => (ServerStatus::Stopped, Vec::new(), None),
        };
        let port = rec.settings.port;
        OwnServerView {
            eula_accepted: self.store.eula_accepted(&rec),
            installing: self.installing.lock().unwrap().contains(&rec.id),
            whitelist: self.store.whitelist(&rec.id),
            lan_address: net::lan_ipv4().map(|ip| net::join_address(&ip.to_string(), port)),
            local_address: net::join_address("localhost", port),
            public_address: rec
                .settings
                .public_address
                .as_deref()
                .map(|a| net::join_address(a, port)),
            id: rec.id,
            version: rec.version,
            created_at: rec.created_at,
            settings: rec.settings,
            installed: rec.installed,
            status,
            players,
            problem,
        }
    }

    pub async fn view_id(&self, id: &str) -> Result<OwnServerView, String> {
        let rec = self.store.load(id).map_err(|e| e.to_string())?;
        Ok(self.view(rec).await)
    }

    pub async fn list(&self) -> Vec<OwnServerView> {
        let mut out = Vec::new();
        for rec in self.store.list() {
            out.push(self.view(rec).await);
        }
        out
    }

    async fn is_live(&self, id: &str) -> bool {
        self.process(id).await.is_some_and(|p| !p.has_exited())
    }

    pub async fn create(
        &self,
        version: &str,
        settings: ServerSettings,
    ) -> Result<OwnServerView, String> {
        let rec = self
            .store
            .create(version, settings)
            .map_err(|e| e.to_string())?;
        Ok(self.view(rec).await)
    }

    pub async fn update(
        &self,
        id: &str,
        settings: ServerSettings,
    ) -> Result<OwnServerView, String> {
        let rec = self
            .store
            .update_settings(id, settings)
            .map_err(|e| e.to_string())?;
        Ok(self.view(rec).await)
    }

    pub async fn accept_eula(&self, id: &str) -> Result<OwnServerView, String> {
        let rec = self.store.accept_eula(id).map_err(|e| e.to_string())?;
        Ok(self.view(rec).await)
    }

    pub async fn install(
        &self,
        id: &str,
        events: &dyn EventHandler,
        install_lock: &Mutex<()>,
    ) -> Result<OwnServerView, String> {
        server::validate_id(id).map_err(|e| e.to_string())?;
        if self.is_live(id).await {
            return Err("Остановите сервер перед установкой или обновлением".into());
        }
        if !self.installing.lock().unwrap().insert(id.to_string()) {
            return Err("Установка этого сервера уже идёт".into());
        }
        let _guard = InstallGuard(&self.installing, id.to_string());
        // Libraries, Java and version jars are shared with the client install.
        let _shared = install_lock.lock().await;
        let client = wex_core::reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| e.to_string())?;
        let rec = server::install_server(
            &self.paths,
            &self.store,
            id,
            &client,
            events,
            &InstallOptions::default(),
        )
        .await
        .map_err(|e| wex_ipc::error::IpcError::from(e).message)?;
        Ok(self.view(rec).await)
    }

    pub async fn start(&self, id: &str) -> Result<OwnServerView, String> {
        if self.installing.lock().unwrap().contains(id) {
            return Err("Дождитесь окончания установки".into());
        }
        let mut running = self.running.lock().await;
        if running.get(id).is_some_and(|p| !p.has_exited()) {
            return Err("Сервер уже запущен".into());
        }
        let p = server::start_server(&self.paths, &self.store, id)
            .await
            .map_err(|e| e.to_string())?;
        running.insert(id.to_string(), Arc::new(p));
        drop(running);
        self.view_id(id).await
    }

    pub async fn stop(&self, id: &str) -> Result<OwnServerView, String> {
        if let Some(p) = self.process(id).await {
            p.stop(STOP_TIMEOUT).await.map_err(|e| e.to_string())?;
        }
        self.view_id(id).await
    }

    pub async fn console(&self, id: &str, since: u64) -> Result<Option<ServerSnapshot>, String> {
        server::validate_id(id).map_err(|e| e.to_string())?;
        Ok(self.process(id).await.map(|p| p.snapshot(since)))
    }

    pub async fn command(&self, id: &str, command: &str) -> Result<(), String> {
        let p = self
            .process(id)
            .await
            .filter(|p| !p.has_exited())
            .ok_or("Сервер не запущен")?;
        p.send_command(command).await.map_err(|e| e.to_string())
    }

    pub async fn whitelist(&self, id: &str, name: &str, add: bool) -> Result<(), String> {
        let name = name.trim();
        if !server::valid_player_name(name) {
            return Err("Ник: от 3 до 16 латинских букв, цифр или _".into());
        }
        let cmd = if add { "add" } else { "remove" };
        self.command(id, &format!("whitelist {cmd} {name}")).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        if self.is_live(id).await {
            return Err("Сначала остановите сервер".into());
        }
        if self.installing.lock().unwrap().contains(id) {
            return Err("Дождитесь окончания установки".into());
        }
        self.running.lock().await.remove(id);
        self.store.delete(id).map_err(|e| e.to_string())
    }

    /// Graceful stop of every server (the launcher is closing).
    pub async fn stop_all(&self) {
        let all: Vec<_> = self.running.lock().await.values().cloned().collect();
        for p in all {
            let _ = p.stop(Duration::from_secs(30)).await;
        }
    }
}

#[tauri::command]
pub async fn own_server_list(
    state: State<'_, OwnServerState>,
) -> Result<Vec<OwnServerView>, String> {
    Ok(state.list().await)
}

#[tauri::command]
pub async fn own_server_create(
    version: String,
    settings: ServerSettings,
    state: State<'_, OwnServerState>,
) -> Result<OwnServerView, String> {
    state.create(&version, settings).await
}

#[tauri::command]
pub async fn own_server_update(
    id: String,
    settings: ServerSettings,
    state: State<'_, OwnServerState>,
) -> Result<OwnServerView, String> {
    state.update(&id, settings).await
}

/// Called only from the explicit «Принимаю EULA» control in the UI.
#[tauri::command]
pub async fn own_server_accept_eula(
    id: String,
    state: State<'_, OwnServerState>,
) -> Result<OwnServerView, String> {
    state.accept_eula(&id).await
}

#[tauri::command]
pub async fn own_server_install<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    id: String,
    state: State<'_, OwnServerState>,
    play_state: State<'_, super::launch::PlayState>,
) -> Result<OwnServerView, String> {
    let events = ProgressEmitter {
        id: id.clone(),
        emit: move |p: OwnServerProgress| {
            let _ = app.emit(EVT_OWN_SERVER_PROGRESS, p);
        },
    };
    state.install(&id, &events, &play_state.install_lock).await
}

#[tauri::command]
pub async fn own_server_start(
    id: String,
    state: State<'_, OwnServerState>,
) -> Result<OwnServerView, String> {
    state.start(&id).await
}

#[tauri::command]
pub async fn own_server_stop(
    id: String,
    state: State<'_, OwnServerState>,
) -> Result<OwnServerView, String> {
    state.stop(&id).await
}

#[tauri::command]
pub async fn own_server_console(
    id: String,
    since: u64,
    state: State<'_, OwnServerState>,
) -> Result<Option<ServerSnapshot>, String> {
    state.console(&id, since).await
}

#[tauri::command]
pub async fn own_server_command(
    id: String,
    command: String,
    state: State<'_, OwnServerState>,
) -> Result<(), String> {
    state.command(&id, &command).await
}

#[tauri::command]
pub async fn own_server_whitelist(
    id: String,
    name: String,
    add: bool,
    state: State<'_, OwnServerState>,
) -> Result<(), String> {
    state.whitelist(&id, &name, add).await
}

/// Permanently deletes the server and its world; the UI confirms first.
#[tauri::command]
pub async fn own_server_delete(id: String, state: State<'_, OwnServerState>) -> Result<(), String> {
    state.delete(&id).await
}

/// Opens the Minecraft EULA page. The address is fixed here, never taken from the UI.
#[tauri::command]
pub fn own_server_open_eula() -> Result<(), String> {
    tauri_plugin_opener::open_url(server::EULA_URL, None::<&str>).map_err(|e| e.to_string())
}

/// «Добавить себе в Серверы»: the host's own `servers.dat` for that version.
#[tauri::command]
pub async fn own_server_add_to_servers(
    id: String,
    state: State<'_, OwnServerState>,
) -> Result<crate::ipc::ServerEntryModel, String> {
    let view = state.view_id(&id).await?;
    super::servers::add_server_impl(
        &state.paths,
        Some(&view.version),
        &view.settings.name,
        &view.local_address,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use wex_core::event::NoopEventHandler;

    fn state() -> (tempfile::TempDir, OwnServerState) {
        let tmp = tempfile::tempdir().unwrap();
        let s = OwnServerState::new(WexPaths::new(Some(tmp.path().to_path_buf())));
        (tmp, s)
    }

    #[tokio::test]
    async fn create_view_has_addresses_and_no_eula() {
        let (_t, s) = state();
        let v = s
            .create(
                "1.21.1",
                ServerSettings {
                    port: 25570,
                    public_address: Some("play.example.com".into()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert!(!v.eula_accepted);
        assert_eq!(v.status, ServerStatus::Stopped);
        assert_eq!(v.local_address, "localhost:25570");
        assert_eq!(v.public_address.as_deref(), Some("play.example.com:25570"));
        if let Some(lan) = &v.lan_address {
            assert!(lan.ends_with(":25570"));
        }
        assert_eq!(s.list().await.len(), 1);
        let v = s.accept_eula(&v.id).await.unwrap();
        assert!(v.eula_accepted);
    }

    #[tokio::test]
    async fn start_requires_install_and_commands_require_running() {
        let (_t, s) = state();
        let v = s.create("1.20.1", ServerSettings::default()).await.unwrap();
        assert!(s.start(&v.id).await.unwrap_err().contains("установите"));
        assert!(s.command(&v.id, "say hi").await.is_err());
        assert!(s
            .whitelist(&v.id, "bad name", true)
            .await
            .unwrap_err()
            .contains("Ник"));
        assert!(s.console(&v.id, 0).await.unwrap().is_none());
        assert!(s.console("../x", 0).await.is_err());
        let v = s.stop(&v.id).await.unwrap();
        assert_eq!(v.status, ServerStatus::Stopped);
    }

    #[tokio::test]
    async fn invalid_settings_and_versions_are_rejected() {
        let (_t, s) = state();
        assert!(s.create("1.19.2", ServerSettings::default()).await.is_err());
        let v = s.create("1.21.1", ServerSettings::default()).await.unwrap();
        let bad = ServerSettings {
            max_players: 0,
            ..Default::default()
        };
        assert!(s.update(&v.id, bad).await.is_err());
    }

    #[tokio::test]
    async fn install_is_single_flight_and_delete_removes() {
        let (_t, s) = state();
        let v = s.create("1.21.1", ServerSettings::default()).await.unwrap();
        s.installing.lock().unwrap().insert(v.id.clone());
        let lock = Mutex::new(());
        let err = s
            .install(&v.id, &NoopEventHandler, &lock)
            .await
            .unwrap_err();
        assert!(err.contains("уже идёт"));
        assert!(s.delete(&v.id).await.is_err());
        assert!(s.view_id(&v.id).await.unwrap().installing);
        s.installing.lock().unwrap().clear();
        s.delete(&v.id).await.unwrap();
        assert!(s.list().await.is_empty());
    }

    #[test]
    fn progress_labels_are_russian() {
        assert_eq!(stage_label("Minecraft server"), "Серверный jar Minecraft");
        assert_eq!(stage_label("Server mods"), "Моды сборки для сервера");
        let got = std::sync::Mutex::new(Vec::new());
        let e = ProgressEmitter {
            id: "srv-00000000".into(),
            emit: |p: OwnServerProgress| got.lock().unwrap().push(p),
        };
        e.on_event(&Event::StageStarted {
            stage: "Java".into(),
            step: 3,
            total_steps: 5,
        });
        e.on_event(&Event::Log {
            level: wex_core::LogLevel::Info,
            message: "x".into(),
        });
        let got = got.into_inner().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].stage, "Java");
        assert_eq!((got[0].current, got[0].total), (2, 5));
    }

    #[tokio::test]
    async fn add_to_servers_writes_host_list() {
        let (_t, s) = state();
        let v = s
            .create(
                "1.21.1",
                ServerSettings {
                    name: "Дача".into(),
                    port: 25570,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        let entry = crate::commands::servers::add_server_impl(
            &s.paths,
            Some(&v.version),
            &v.settings.name,
            &v.local_address,
        )
        .unwrap();
        assert_eq!(entry.address, "localhost:25570");
        assert!(crate::commands::servers::add_server_impl(
            &s.paths,
            Some(&v.version),
            "x",
            &v.local_address
        )
        .is_err());
    }
}
