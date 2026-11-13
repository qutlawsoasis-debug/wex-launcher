//! «Облачные миры»: Tauri commands over `wex_social::cloud`.
//!
//! Worlds live on the Aura world manager and run 24/7 there, not on this PC.
//! The owner is always the active Wex account, read here from the account
//! store: the UI never says who it is. The manager's address and secret come
//! from the environment or the user's config (never from the repository); with
//! either missing every command answers with the reason and the UI shows the
//! feature as unavailable.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;
use wex_core::accounts::AccountStore;
use wex_core::config::{LauncherConfig, WexPaths};
use wex_social::cloud::{
    is_valid_player, CloudClient, CloudConfig, CloudWorld, StartRequest, WorldState,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudStatusView {
    pub available: bool,
    /// Why the feature is unavailable (no address, no secret, no account).
    pub reason: Option<String>,
    /// The active account's name: the owner of new worlds.
    pub owner: Option<String>,
    /// The host friends connect to.
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudWorldView {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub seed: Option<String>,
    pub allowed_players: Vec<String>,
    pub state: WorldState,
    pub port: Option<u16>,
    /// `host:port` to join, when the manager has given the world a port.
    pub address: Option<String>,
    pub online_players: Vec<String>,
    pub max_players: u32,
    pub is_owner: bool,
    pub stage_text: Option<String>,
    pub progress: u8,
    pub game_version: String,
    pub last_played: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudListView {
    pub owner: String,
    pub max_active: u32,
    pub active_count: u32,
    pub worlds: Vec<CloudWorldView>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudStartView {
    pub world_id: Option<String>,
    pub state: WorldState,
    pub port: Option<u16>,
    pub address: Option<String>,
}

pub struct CloudState {
    root: PathBuf,
    config_file: PathBuf,
    /// Tests point the commands at a mock manager.
    fixed: Option<CloudConfig>,
}

fn view(cfg: &CloudConfig, w: &CloudWorld, player: &str) -> CloudWorldView {
    CloudWorldView {
        id: w.id.clone(),
        name: w.name.clone(),
        owner: w.owner.clone(),
        seed: w.seed.clone().filter(|s| !s.trim().is_empty()),
        allowed_players: w.allowed_players.clone(),
        state: w.state(),
        port: w.port,
        address: w.port.map(|p| cfg.address(p)),
        online_players: w.online_players.clone(),
        max_players: w.max_players,
        is_owner: w.owned_by(player),
        stage_text: w.stage_text.clone().filter(|s| !s.trim().is_empty()),
        progress: w.progress_percent(),
        game_version: w.version().to_string(),
        last_played: w.last_played,
    }
}

impl CloudState {
    pub fn new(paths: &WexPaths) -> Self {
        Self {
            root: paths.root.clone(),
            config_file: paths.config_file.clone(),
            fixed: None,
        }
    }

    #[cfg(test)]
    fn with_config(paths: &WexPaths, cfg: CloudConfig) -> Self {
        Self {
            fixed: Some(cfg),
            ..Self::new(paths)
        }
    }

    fn config(&self) -> Result<CloudConfig, String> {
        if let Some(c) = &self.fixed {
            return Ok(c.clone());
        }
        let cfg = LauncherConfig::load_or_default(&self.config_file);
        CloudConfig::from_env_and_config(cfg.cloud_url, cfg.cloud_secret)
            .map_err(|u| u.reason().to_string())
    }

    /// The active account's name, as Minecraft will see it in game.
    fn owner(&self) -> Result<String, String> {
        let store = AccountStore::open(&self.root).map_err(|e| e.to_string())?;
        let name = store
            .active()
            .map(|a| a.name.trim().to_string())
            .ok_or_else(|| "Войдите в аккаунт, чтобы пользоваться облачными мирами".to_string())?;
        if !is_valid_player(&name) {
            return Err(format!("Ник «{name}» не подходит для облачного мира"));
        }
        Ok(name)
    }

    fn client(&self) -> Result<(CloudClient, String), String> {
        let cfg = self.config()?;
        let owner = self.owner()?;
        Ok((CloudClient::new(cfg), owner))
    }

    pub fn status(&self) -> CloudStatusView {
        let cfg = self.config();
        let owner = self.owner();
        let reason = match (&cfg, &owner) {
            (Err(r), _) | (Ok(_), Err(r)) => Some(r.clone()),
            _ => None,
        };
        CloudStatusView {
            available: reason.is_none(),
            reason,
            owner: owner.ok(),
            host: cfg.ok().map(|c| c.host()),
        }
    }

    pub async fn list(&self) -> Result<CloudListView, String> {
        let (client, owner) = self.client()?;
        let res = client.worlds(&owner).await.map_err(|e| e.to_string())?;
        let mut worlds: Vec<CloudWorldView> = res
            .worlds
            .iter()
            .map(|w| view(client.config(), w, &owner))
            .collect();
        // My worlds first, then shared ones; most recently played first.
        worlds.sort_by(|a, b| {
            b.is_owner
                .cmp(&a.is_owner)
                .then(b.last_played.total_cmp(&a.last_played))
        });
        Ok(CloudListView {
            owner,
            max_active: res.max_active,
            active_count: res.active_count,
            worlds,
        })
    }

    async fn find(
        &self,
        client: &CloudClient,
        owner: &str,
        id: &str,
    ) -> Result<CloudWorld, String> {
        client
            .worlds(owner)
            .await
            .map_err(|e| e.to_string())?
            .worlds
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| "Мир не найден или к нему больше нет доступа".to_string())
    }

    async fn run(&self, client: &CloudClient, req: StartRequest) -> Result<CloudStartView, String> {
        let r = client.start(&req).await.map_err(|e| e.to_string())?;
        let state = WorldState::from_status(&r.status);
        Ok(CloudStartView {
            world_id: r.world_id.or(req.world_id),
            // A fresh world is not «sleeping»: the manager just accepted it.
            state: if state == WorldState::Sleeping {
                WorldState::Starting
            } else {
                state
            },
            port: r.port,
            address: r.port.map(|p| client.config().address(p)),
        })
    }

    pub async fn create(
        &self,
        name: &str,
        version: &str,
        seed: Option<String>,
    ) -> Result<CloudStartView, String> {
        let (client, owner) = self.client()?;
        let req = StartRequest {
            owner,
            world_name: name.to_string(),
            seed,
            world_id: None,
            version: version.to_string(),
        };
        self.run(&client, req).await
    }

    /// Wakes a saved world (mine or one shared with me).
    pub async fn start(&self, id: &str) -> Result<CloudStartView, String> {
        let (client, me) = self.client()?;
        let w = self.find(&client, &me, id).await?;
        let req = StartRequest {
            owner: if w.owner.trim().is_empty() {
                me
            } else {
                w.owner.clone()
            },
            world_name: w.name.clone(),
            seed: w.seed.clone(),
            world_id: Some(w.id.clone()),
            version: w.version().to_string(),
        };
        self.run(&client, req).await
    }

    pub async fn stop(&self, id: &str) -> Result<(), String> {
        let (client, me) = self.client()?;
        let w = self.find(&client, &me, id).await?;
        client.stop(&w.id, w.port).await.map_err(|e| e.to_string())
    }

    pub async fn share(&self, id: &str, player: &str, add: bool) -> Result<Vec<String>, String> {
        let (client, me) = self.client()?;
        let player = player.trim();
        if player.eq_ignore_ascii_case(&me) {
            return Err("Это ваш ник: у владельца доступ есть всегда".into());
        }
        let w = self.find(&client, &me, id).await?;
        if !w.owned_by(&me) {
            return Err("Делиться миром может только владелец".into());
        }
        client
            .share(&w.owner, &w.id, player, add)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let (client, me) = self.client()?;
        let w = self.find(&client, &me, id).await?;
        if !w.owned_by(&me) {
            return Err("Удалить мир может только владелец".into());
        }
        if w.state() != WorldState::Sleeping {
            return Err("Сначала остановите мир".into());
        }
        client
            .delete(&w.owner, &w.id)
            .await
            .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn cloud_status(state: State<'_, CloudState>) -> CloudStatusView {
    state.status()
}

#[tauri::command]
pub async fn cloud_list(state: State<'_, CloudState>) -> Result<CloudListView, String> {
    state.list().await
}

#[tauri::command]
pub async fn cloud_create(
    name: String,
    version: String,
    seed: Option<String>,
    state: State<'_, CloudState>,
) -> Result<CloudStartView, String> {
    state.create(&name, &version, seed).await
}

#[tauri::command]
pub async fn cloud_start(
    id: String,
    state: State<'_, CloudState>,
) -> Result<CloudStartView, String> {
    state.start(&id).await
}

#[tauri::command]
pub async fn cloud_stop(id: String, state: State<'_, CloudState>) -> Result<(), String> {
    state.stop(&id).await
}

#[tauri::command]
pub async fn cloud_share(
    id: String,
    player: String,
    add: bool,
    state: State<'_, CloudState>,
) -> Result<Vec<String>, String> {
    state.share(&id, &player, add).await
}

#[tauri::command]
pub async fn cloud_delete(id: String, state: State<'_, CloudState>) -> Result<(), String> {
    state.delete(&id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn paths_with_account(name: Option<&str>) -> (tempfile::TempDir, WexPaths) {
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        if let Some(n) = name {
            AccountStore::open(&paths.root)
                .unwrap()
                .add_offline(n)
                .unwrap();
        }
        (tmp, paths)
    }

    fn world(id: &str, owner: &str, status: &str, port: Option<u16>) -> serde_json::Value {
        serde_json::json!({
            "id": id, "name": format!("Мир {id}"), "owner": owner, "seed": "",
            "allowed_players": ["Alex"], "status": status, "port": port,
            "online_players": [], "max_players": 8, "is_owner": false,
            "progress": 0, "game_version": null, "last_played": 1.0
        })
    }

    async fn manager(worlds: Vec<serde_json::Value>) -> MockServer {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/worlds"))
            .and(query_param("player", "Steve"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "max_active": 3, "active_count": 1, "worlds": worlds
            })))
            .mount(&server)
            .await;
        server
    }

    fn state(paths: &WexPaths, server: &MockServer) -> CloudState {
        CloudState::with_config(
            paths,
            CloudConfig::new(&server.uri(), "test-secret").unwrap(),
        )
    }

    #[test]
    fn status_without_secret_or_account_says_why() {
        let (_t, paths) = paths_with_account(None);
        let s = CloudState::new(&paths);
        // The test environment may set the variables; only check the shape then.
        let st = s.status();
        assert!(!st.available);
        assert!(st.reason.is_some());
        let (_t2, paths2) = paths_with_account(None);
        let s2 =
            CloudState::with_config(&paths2, CloudConfig::new("http://10.1.2.3:9", "x").unwrap());
        let st2 = s2.status();
        assert!(!st2.available);
        assert!(st2.reason.unwrap().contains("Войдите"));
        assert_eq!(st2.host.as_deref(), Some("10.1.2.3"));
    }

    #[tokio::test]
    async fn status_is_available_with_config_and_account() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![]).await;
        let st = state(&paths, &server).status();
        assert!(st.available, "{st:?}");
        assert_eq!(st.owner.as_deref(), Some("Steve"));
    }

    #[tokio::test]
    async fn list_puts_my_worlds_first_with_addresses() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![
            world("shared", "Alex", "running", Some(25610)),
            world("mine", "steve", "sleeping", None),
        ])
        .await;
        let l = state(&paths, &server).list().await.unwrap();
        assert_eq!(l.owner, "Steve");
        assert_eq!(l.worlds[0].id, "mine");
        assert!(l.worlds[0].is_owner && !l.worlds[1].is_owner);
        assert_eq!(l.worlds[0].seed, None);
        assert_eq!(l.worlds[0].game_version, "1.20.1");
        assert_eq!(l.worlds[1].state, WorldState::Running);
        assert_eq!(l.worlds[1].address.as_deref(), Some("127.0.0.1:25610"));
    }

    #[tokio::test]
    async fn create_uses_the_active_account_as_owner() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![]).await;
        Mock::given(method("POST"))
            .and(path("/lobbies/create"))
            .and(body_json(serde_json::json!({
                "owner": "Steve", "worldName": "Дача", "seed": "7",
                "worldId": null, "version": "1.21.1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "port": 25620, "status": "created", "worldId": "new1"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let r = state(&paths, &server)
            .create("Дача", "1.21.1", Some("7".into()))
            .await
            .unwrap();
        assert_eq!(r.world_id.as_deref(), Some("new1"));
        assert_eq!(r.state, WorldState::Starting);
        assert_eq!(r.address.as_deref(), Some("127.0.0.1:25620"));
    }

    #[tokio::test]
    async fn create_without_account_sends_nothing() {
        let (_t, paths) = paths_with_account(None);
        let server = MockServer::start().await;
        assert!(state(&paths, &server)
            .create("Дача", "1.21.1", None)
            .await
            .is_err());
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn start_wakes_a_shared_world_under_its_owner() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![world("w1", "Alex", "sleeping", None)]).await;
        Mock::given(method("POST"))
            .and(path("/lobbies/create"))
            .and(body_json(serde_json::json!({
                "owner": "Alex", "worldName": "Мир w1", "seed": "",
                "worldId": "w1", "version": "1.20.1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "port": 25611, "status": "running", "worldId": "w1"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let r = state(&paths, &server).start("w1").await.unwrap();
        assert_eq!(r.state, WorldState::Running);
        assert!(state(&paths, &server).start("missing").await.is_err());
    }

    #[tokio::test]
    async fn stop_sends_the_known_port() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![world("w1", "Steve", "running", Some(25601))]).await;
        Mock::given(method("POST"))
            .and(path("/lobbies/stop"))
            .and(body_json(
                serde_json::json!({ "worldId": "w1", "port": 25601 }),
            ))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
        state(&paths, &server).stop("w1").await.unwrap();
    }

    #[tokio::test]
    async fn only_the_owner_shares_or_deletes() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![
            world("theirs", "Alex", "sleeping", None),
            world("mine", "Steve", "running", Some(25601)),
        ])
        .await;
        let s = state(&paths, &server);
        assert!(s
            .share("theirs", "Bob_1", true)
            .await
            .unwrap_err()
            .contains("владелец"));
        assert!(s.delete("theirs").await.unwrap_err().contains("владелец"));
        assert!(s.delete("mine").await.unwrap_err().contains("остановите"));
        assert!(s
            .share("mine", "steve", true)
            .await
            .unwrap_err()
            .contains("ваш ник"));
        // Nothing but the list was asked for.
        let reqs = server.received_requests().await.unwrap();
        assert!(reqs.iter().all(|r| r.url.path() == "/worlds"));
    }

    #[tokio::test]
    async fn share_and_delete_reach_the_manager_for_my_world() {
        let (_t, paths) = paths_with_account(Some("Steve"));
        let server = manager(vec![world("mine", "Steve", "sleeping", None)]).await;
        Mock::given(method("POST"))
            .and(path("/lobbies/share"))
            .and(body_json(serde_json::json!({
                "owner": "Steve", "worldId": "mine", "targetPlayer": "Bob_1", "action": "add"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true, "allowed_players": ["Alex", "Bob_1"]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/worlds/delete"))
            .and(body_json(
                serde_json::json!({ "owner": "Steve", "worldId": "mine" }),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "success": true })),
            )
            .expect(1)
            .mount(&server)
            .await;
        let s = state(&paths, &server);
        assert_eq!(
            s.share("mine", " Bob_1 ", true).await.unwrap(),
            vec!["Alex", "Bob_1"]
        );
        s.delete("mine").await.unwrap();
    }
}
