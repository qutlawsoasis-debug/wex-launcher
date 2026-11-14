//! «Облачные миры»: friends' worlds hosted 24/7 on the Aura world manager.
//!
//! The manager is a small HTTP service (behaviour taken from the old
//! Aura-Launcher, no code copied). Every request carries the shared secret in
//! `X-Aura-Secret`. Neither the address nor the secret is in this repository:
//! they come from the environment (`WEX_CLOUD_URL`, `WEX_CLOUD_SECRET`), then
//! the user's config, then the build environment (`option_env!`, for a
//! release built by CI). Without both the feature is unavailable and says why.
//!
//! Players join a running world at `<manager host>:<world port>`.

use std::fmt;
use std::time::Duration;

use reqwest::{Client, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

pub const CLOUD_URL_ENV: &str = "WEX_CLOUD_URL";
pub const CLOUD_SECRET_ENV: &str = "WEX_CLOUD_SECRET";
pub const SECRET_HEADER: &str = "X-Aura-Secret";

/// Versions the manager can host; Wex ships both (AGENTS.md rule 4).
pub const CLOUD_VERSIONS: [&str; 2] = ["1.20.1", "1.21.1"];
/// The manager's default when a world has no `game_version` (old servers).
pub const DEFAULT_WORLD_VERSION: &str = "1.20.1";
pub const MAX_WORLD_NAME_CHARS: usize = 48;
pub const MAX_SEED_CHARS: usize = 64;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const READ_TIMEOUT: Duration = Duration::from_secs(15);
/// Creating or waking a world may wait for the manager to allocate a port.
const START_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unavailable {
    NoUrl,
    NoSecret,
    BadUrl,
}

impl Unavailable {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NoUrl => "Не задан адрес облачного сервера (WEX_CLOUD_URL)",
            Self::NoSecret => "Не задан ключ облачного сервера (WEX_CLOUD_SECRET)",
            Self::BadUrl => "Адрес облачного сервера должен начинаться с http:// или https://",
        }
    }
}

#[derive(Debug, Error)]
pub enum CloudError {
    #[error("{}", .0.reason())]
    Unavailable(Unavailable),
    #[error("Облачный сервер не ответил вовремя")]
    Timeout,
    #[error("Нет связи с облачным сервером")]
    Connect,
    #[error("Облачный сервер отклонил ключ доступа")]
    Unauthorized,
    #[error("Ошибка облачного сервера ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("{0}")]
    Rejected(String),
    #[error("Непонятный ответ облачного сервера: {0}")]
    Decode(String),
    #[error("{0}")]
    Invalid(&'static str),
    #[error("Сетевая ошибка: {0}")]
    Http(String),
}

impl From<reqwest::Error> for CloudError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else if e.is_connect() {
            Self::Connect
        } else if e.is_decode() {
            Self::Decode(e.to_string())
        } else {
            // No URL in the message: it is fine, but keep errors short.
            Self::Http(e.without_url().to_string())
        }
    }
}

/// Where the manager is and the secret it expects. `Debug` never prints the secret.
#[derive(Clone, PartialEq, Eq)]
pub struct CloudConfig {
    base_url: Url,
    secret: String,
}

impl fmt::Debug for CloudConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudConfig")
            .field("base_url", &self.base_url.as_str())
            .field("secret", &"<hidden>")
            .finish()
    }
}

fn pick(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// One source of settings, in priority order: env, then config, then build.
#[derive(Debug, Default, Clone)]
pub struct CloudSource {
    pub url: Option<String>,
    pub secret: Option<String>,
}

impl CloudSource {
    pub fn env() -> Self {
        Self {
            url: std::env::var(CLOUD_URL_ENV).ok(),
            secret: std::env::var(CLOUD_SECRET_ENV).ok(),
        }
    }

    pub fn build() -> Self {
        Self {
            url: option_env!("WEX_CLOUD_URL").map(str::to_string),
            secret: option_env!("WEX_CLOUD_SECRET").map(str::to_string),
        }
    }
}

impl CloudConfig {
    pub fn new(base_url: &str, secret: &str) -> Result<Self, Unavailable> {
        let secret = secret.trim();
        if secret.is_empty() {
            return Err(Unavailable::NoSecret);
        }
        let mut raw = base_url.trim().to_string();
        if raw.is_empty() {
            return Err(Unavailable::NoUrl);
        }
        if !raw.ends_with('/') {
            raw.push('/');
        }
        let url = Url::parse(&raw).map_err(|_| Unavailable::BadUrl)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
            return Err(Unavailable::BadUrl);
        }
        Ok(Self {
            base_url: url,
            secret: secret.to_string(),
        })
    }

    /// Each value is taken from the first source that has it.
    pub fn resolve(sources: &[CloudSource]) -> Result<Self, Unavailable> {
        let url = sources.iter().find_map(|s| pick(s.url.clone()));
        let secret = sources.iter().find_map(|s| pick(s.secret.clone()));
        match (url, secret) {
            (None, _) => Err(Unavailable::NoUrl),
            (_, None) => Err(Unavailable::NoSecret),
            (Some(u), Some(s)) => Self::new(&u, &s),
        }
    }

    /// Environment, then the user's config values, then the build environment.
    pub fn from_env_and_config(
        cfg_url: Option<String>,
        cfg_secret: Option<String>,
    ) -> Result<Self, Unavailable> {
        Self::resolve(&[
            CloudSource::env(),
            CloudSource {
                url: cfg_url,
                secret: cfg_secret,
            },
            CloudSource::build(),
        ])
    }

    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    /// The host players connect to (the manager's host).
    pub fn host(&self) -> String {
        // IPv6 hosts come back in brackets, ready for `host:port`.
        self.base_url.host_str().unwrap_or_default().to_string()
    }

    pub fn address(&self, port: u16) -> String {
        format!("{}:{}", self.host(), port)
    }
}

fn null_as_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

fn default_max_players() -> u32 {
    8
}

/// What the world is doing, from the manager's free-form `status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldState {
    Running,
    Starting,
    Stopping,
    /// Saved and not running ("sleeping", "stopped", anything else).
    Sleeping,
}

impl WorldState {
    pub fn from_status(status: &str) -> Self {
        match status.trim().to_ascii_lowercase().as_str() {
            "running" => Self::Running,
            "starting" => Self::Starting,
            "stopping" => Self::Stopping,
            _ => Self::Sleeping,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudWorld {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub allowed_players: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub created_at: f64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub last_played: f64,
    #[serde(default, deserialize_with = "null_as_default")]
    pub status: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub online_players: Vec<String>,
    #[serde(default = "default_max_players")]
    pub max_players: u32,
    #[serde(default, deserialize_with = "null_as_default")]
    pub is_owner: bool,
    #[serde(default, rename = "stageText")]
    pub stage_text: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub progress: i32,
    #[serde(default)]
    pub game_version: Option<String>,
}

impl CloudWorld {
    pub fn state(&self) -> WorldState {
        WorldState::from_status(&self.status)
    }

    pub fn version(&self) -> &str {
        self.game_version
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or(DEFAULT_WORLD_VERSION)
    }

    pub fn progress_percent(&self) -> u8 {
        self.progress.clamp(0, 100) as u8
    }

    /// The manager says so, or the nickname matches (case-insensitive, like Minecraft).
    pub fn owned_by(&self, player: &str) -> bool {
        self.is_owner || (!player.is_empty() && self.owner.eq_ignore_ascii_case(player))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudWorlds {
    #[serde(default, deserialize_with = "null_as_default")]
    pub max_active: u32,
    #[serde(default, deserialize_with = "null_as_default")]
    pub active_count: u32,
    #[serde(default, deserialize_with = "null_as_default")]
    pub worlds: Vec<CloudWorld>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateResponse {
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub status: String,
    #[serde(default, rename = "worldId")]
    pub world_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub game_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShareResponse {
    #[serde(default)]
    success: bool,
    #[serde(default, deserialize_with = "null_as_default")]
    allowed_players: Vec<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeleteResponse {
    #[serde(default)]
    success: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// What to create, or which saved world to wake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRequest {
    pub owner: String,
    pub world_name: String,
    pub seed: Option<String>,
    /// `None` creates a new world; `Some` wakes an existing one.
    pub world_id: Option<String>,
    pub version: String,
}

/// Vanilla name rule (offline and Microsoft names alike): 3–16 of `A-Za-z0-9_`.
pub fn is_valid_player(name: &str) -> bool {
    (3..=16).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Trimmed world name, or why it is not acceptable.
pub fn clean_world_name(name: &str) -> Result<String, CloudError> {
    let n = name.trim();
    if n.is_empty() {
        return Err(CloudError::Invalid("Укажите название мира"));
    }
    if n.chars().count() > MAX_WORLD_NAME_CHARS {
        return Err(CloudError::Invalid("Название мира слишком длинное"));
    }
    if n.chars().any(char::is_control) {
        return Err(CloudError::Invalid(
            "В названии мира есть недопустимые символы",
        ));
    }
    Ok(n.to_string())
}

pub fn clean_seed(seed: Option<&str>) -> Result<Option<String>, CloudError> {
    match seed.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(s) if s.chars().count() > MAX_SEED_CHARS || s.chars().any(char::is_control) => Err(
            CloudError::Invalid("Сид слишком длинный или содержит недопустимые символы"),
        ),
        Some(s) => Ok(Some(s.to_string())),
    }
}

fn check_version(v: &str) -> Result<(), CloudError> {
    if CLOUD_VERSIONS.contains(&v) {
        Ok(())
    } else {
        Err(CloudError::Invalid(
            "Облачный мир поддерживает версии 1.20.1 и 1.21.1",
        ))
    }
}

fn check_player(name: &str) -> Result<(), CloudError> {
    if is_valid_player(name) {
        Ok(())
    } else {
        Err(CloudError::Invalid("Ник: 3–16 латинских букв, цифр или _"))
    }
}

fn check_id(id: &str) -> Result<(), CloudError> {
    if id.trim().is_empty() || id.len() > 128 {
        Err(CloudError::Invalid("Нет идентификатора мира"))
    } else {
        Ok(())
    }
}

#[derive(Clone)]
pub struct CloudClient {
    http: Client,
    cfg: CloudConfig,
}

impl fmt::Debug for CloudClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudClient")
            .field("cfg", &self.cfg)
            .finish()
    }
}

impl CloudClient {
    pub fn new(cfg: CloudConfig) -> Self {
        let http = Client::builder()
            .user_agent("WexLauncher")
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(START_TIMEOUT)
            .build()
            .unwrap_or_default();
        Self { http, cfg }
    }

    pub fn config(&self) -> &CloudConfig {
        &self.cfg
    }

    fn url(&self, path: &str) -> Url {
        // `path` is a fixed relative path; joining it to a URL that ends in '/' cannot fail.
        self.cfg.base_url.join(path).expect("fixed relative path")
    }

    async fn send<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
        timeout: Duration,
    ) -> Result<(StatusCode, Option<T>), CloudError> {
        let res = req
            .header(SECRET_HEADER, &self.cfg.secret)
            .timeout(timeout)
            .send()
            .await?;
        let status = res.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(CloudError::Unauthorized);
        }
        let text = res.text().await?;
        let body = serde_json::from_str::<T>(&text).ok();
        if !status.is_success() && body.is_none() {
            return Err(CloudError::Api {
                status: status.as_u16(),
                message: text.chars().take(200).collect(),
            });
        }
        Ok((status, body))
    }

    /// Worlds the player owns and worlds shared with them.
    pub async fn worlds(&self, player: &str) -> Result<CloudWorlds, CloudError> {
        check_player(player)?;
        let req = self
            .http
            .get(self.url("worlds"))
            .query(&[("player", player)]);
        match self.send::<CloudWorlds>(req, READ_TIMEOUT).await? {
            (s, Some(w)) if s.is_success() => Ok(w),
            (s, _) => Err(CloudError::Api {
                status: s.as_u16(),
                message: "Список миров не получен".into(),
            }),
        }
    }

    /// Creates a world (`world_id: None`) or wakes a saved one. The manager
    /// answers with the port at once; the world may still be `starting`.
    pub async fn start(&self, r: &StartRequest) -> Result<CreateResponse, CloudError> {
        check_player(&r.owner)?;
        let name = clean_world_name(&r.world_name)?;
        let seed = clean_seed(r.seed.as_deref())?;
        check_version(&r.version)?;
        if let Some(id) = &r.world_id {
            check_id(id)?;
        }
        let body = serde_json::json!({
            "owner": r.owner,
            "worldName": name,
            "seed": seed.unwrap_or_default(),
            "worldId": r.world_id.as_deref().map(str::trim),
            "version": r.version,
        });
        let req = self.http.post(self.url("lobbies/create")).json(&body);
        let (status, res) = self.send::<CreateResponse>(req, START_TIMEOUT).await?;
        let res = res.ok_or_else(|| CloudError::Api {
            status: status.as_u16(),
            message: "Пустой ответ".into(),
        })?;
        if let Some(e) = res
            .error
            .as_deref()
            .map(str::trim)
            .filter(|e| !e.is_empty())
        {
            return Err(CloudError::Rejected(e.to_string()));
        }
        if !status.is_success() || res.port.is_none() {
            return Err(CloudError::Rejected(
                "Сервер не выделил порт для мира".into(),
            ));
        }
        Ok(res)
    }

    /// Saves and stops a running world.
    pub async fn stop(&self, world_id: &str, port: Option<u16>) -> Result<(), CloudError> {
        check_id(world_id)?;
        let body = serde_json::json!({ "worldId": world_id.trim(), "port": port });
        let req = self.http.post(self.url("lobbies/stop")).json(&body);
        let (status, _) = self.send::<serde_json::Value>(req, START_TIMEOUT).await?;
        if status.is_success() {
            Ok(())
        } else {
            Err(CloudError::Api {
                status: status.as_u16(),
                message: "Мир не остановлен".into(),
            })
        }
    }

    /// Gives (`add`) or takes back access for a player. Returns who has access now.
    pub async fn share(
        &self,
        owner: &str,
        world_id: &str,
        target: &str,
        add: bool,
    ) -> Result<Vec<String>, CloudError> {
        check_player(owner)?;
        check_id(world_id)?;
        check_player(target)?;
        let body = serde_json::json!({
            "owner": owner,
            "worldId": world_id.trim(),
            "targetPlayer": target,
            "action": if add { "add" } else { "remove" },
        });
        let req = self.http.post(self.url("lobbies/share")).json(&body);
        let (status, res) = self.send::<ShareResponse>(req, READ_TIMEOUT).await?;
        match res {
            Some(r) if r.success && status.is_success() => Ok(r.allowed_players),
            Some(r) => Err(CloudError::Rejected(
                r.error.unwrap_or_else(|| "Доступ не изменён".into()),
            )),
            None => Err(CloudError::Api {
                status: status.as_u16(),
                message: "Доступ не изменён".into(),
            }),
        }
    }

    /// Deletes a world for good (owner only, checked by the manager).
    pub async fn delete(&self, owner: &str, world_id: &str) -> Result<(), CloudError> {
        check_player(owner)?;
        check_id(world_id)?;
        let body = serde_json::json!({ "owner": owner, "worldId": world_id.trim() });
        let req = self.http.post(self.url("worlds/delete")).json(&body);
        let (status, res) = self.send::<DeleteResponse>(req, READ_TIMEOUT).await?;
        match res {
            Some(r) if r.success && status.is_success() => Ok(()),
            Some(r) => Err(CloudError::Rejected(
                r.error
                    .or(r.message)
                    .unwrap_or_else(|| "Мир не удалён".into()),
            )),
            None => Err(CloudError::Api {
                status: status.as_u16(),
                message: "Мир не удалён".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const SECRET: &str = "test-secret-not-real";

    fn client(server: &MockServer) -> CloudClient {
        CloudClient::new(CloudConfig::new(&server.uri(), SECRET).unwrap())
    }

    fn world_json() -> serde_json::Value {
        serde_json::json!({
            "id": "w1", "name": "Дача", "owner": "Steve", "seed": "42",
            "allowed_players": ["Alex"], "created_at": 1.5, "last_played": 2.5,
            "status": "running", "port": 25601, "online_players": ["Steve"],
            "max_players": 10, "is_owner": true, "stageText": "В сети", "progress": 100,
            "game_version": "1.21.1"
        })
    }

    #[test]
    fn config_needs_url_and_secret_and_hides_the_secret() {
        assert_eq!(CloudConfig::new("", "s"), Err(Unavailable::NoUrl));
        assert_eq!(
            CloudConfig::new("http://h:1", "  "),
            Err(Unavailable::NoSecret)
        );
        assert_eq!(CloudConfig::new("ftp://h", "s"), Err(Unavailable::BadUrl));
        assert_eq!(CloudConfig::new("not a url", "s"), Err(Unavailable::BadUrl));
        let c = CloudConfig::new(" http://10.0.0.7:25571 ", " sec ").unwrap();
        assert_eq!(c.base_url(), "http://10.0.0.7:25571/");
        assert_eq!(c.address(25601), "10.0.0.7:25601");
        let dbg = format!("{:?}", CloudClient::new(c));
        assert!(!dbg.contains("sec\"") && dbg.contains("<hidden>"), "{dbg}");
        let v6 = CloudConfig::new("http://[::1]:9/", "s").unwrap();
        assert_eq!(v6.address(5), "[::1]:5");
    }

    #[test]
    fn resolve_takes_each_value_from_the_first_source_that_has_it() {
        let env = CloudSource {
            url: Some(" ".into()),
            secret: Some("env-secret".into()),
        };
        let cfg = CloudSource {
            url: Some("http://cfg.example:1".into()),
            secret: Some("cfg-secret".into()),
        };
        let c = CloudConfig::resolve(&[env, cfg.clone()]).unwrap();
        assert_eq!(c.base_url(), "http://cfg.example:1/");
        assert_eq!(c.secret, "env-secret");
        assert_eq!(
            CloudConfig::resolve(&[CloudSource {
                url: cfg.url.clone(),
                secret: None
            }]),
            Err(Unavailable::NoSecret)
        );
        assert_eq!(CloudConfig::resolve(&[]), Err(Unavailable::NoUrl));
        assert!(Unavailable::NoSecret.reason().contains(CLOUD_SECRET_ENV));
    }

    #[test]
    fn world_parses_nulls_and_old_records() {
        let w: CloudWorld = serde_json::from_value(serde_json::json!({
            "id": "w2", "name": "Old", "owner": "Steve", "status": "Sleeping",
            "allowed_players": null, "online_players": null, "port": null, "stageText": null
        }))
        .unwrap();
        assert_eq!(w.state(), WorldState::Sleeping);
        assert_eq!(w.version(), DEFAULT_WORLD_VERSION);
        assert_eq!(w.max_players, 8);
        assert!(w.allowed_players.is_empty() && w.online_players.is_empty());
        assert!(w.owned_by("steve") && !w.owned_by("Alex") && !w.owned_by(""));
        let mut w = w;
        w.progress = 250;
        assert_eq!(w.progress_percent(), 100);
        w.status = "STARTING".into();
        assert_eq!(w.state(), WorldState::Starting);
    }

    #[test]
    fn inputs_are_validated() {
        assert!(is_valid_player("Steve_1") && !is_valid_player("ab") && !is_valid_player("Стив"));
        assert!(!is_valid_player("a_very_long_name_17"));
        assert_eq!(clean_world_name("  Дача ").unwrap(), "Дача");
        assert!(clean_world_name("   ").is_err());
        assert!(clean_world_name(&"x".repeat(49)).is_err());
        assert!(clean_world_name("a\nb").is_err());
        assert_eq!(clean_seed(Some("  ")).unwrap(), None);
        assert_eq!(clean_seed(Some(" -12 ")).unwrap().as_deref(), Some("-12"));
        assert!(clean_seed(Some(&"9".repeat(65))).is_err());
        assert!(check_version("1.19.4").is_err() && check_version("1.20.1").is_ok());
    }

    #[tokio::test]
    async fn worlds_sends_secret_and_player() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/worlds"))
            .and(query_param("player", "Steve"))
            .and(header(SECRET_HEADER, SECRET))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "max_active": 3, "active_count": 1, "worlds": [world_json()]
            })))
            .expect(1)
            .mount(&server)
            .await;
        let w = client(&server).worlds("Steve").await.unwrap();
        assert_eq!((w.max_active, w.active_count), (3, 1));
        assert_eq!(w.worlds[0].port, Some(25601));
        assert_eq!(w.worlds[0].stage_text.as_deref(), Some("В сети"));
        assert_eq!(w.worlds[0].state(), WorldState::Running);
        assert_eq!(w.worlds[0].version(), "1.21.1");
    }

    #[tokio::test]
    async fn worlds_rejects_bad_player_without_a_request() {
        let server = MockServer::start().await;
        let e = client(&server).worlds("x").await.unwrap_err();
        assert!(matches!(e, CloudError::Invalid(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn unauthorized_is_reported_as_such() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(401).set_body_string("no"))
            .mount(&server)
            .await;
        let e = client(&server).worlds("Steve").await.unwrap_err();
        assert!(matches!(e, CloudError::Unauthorized), "{e:?}");
    }

    #[tokio::test]
    async fn server_error_without_json_is_an_api_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(502).set_body_string("Bad gateway"))
            .mount(&server)
            .await;
        match client(&server).worlds("Steve").await.unwrap_err() {
            CloudError::Api { status, message } => {
                assert_eq!(status, 502);
                assert_eq!(message, "Bad gateway");
            }
            e => panic!("{e:?}"),
        }
    }

    #[tokio::test]
    async fn slow_server_times_out() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/lobbies/stop"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(3)))
            .mount(&server)
            .await;
        let c = client(&server);
        let req = c
            .http
            .post(c.url("lobbies/stop"))
            .json(&serde_json::json!({}));
        let e = c
            .send::<serde_json::Value>(req, Duration::from_millis(200))
            .await
            .unwrap_err();
        assert!(matches!(e, CloudError::Timeout), "{e:?}");
    }

    #[tokio::test]
    async fn unreachable_server_is_a_connect_error() {
        // Port 9 on localhost (discard) is closed on test machines.
        let c = CloudClient::new(CloudConfig::new("http://127.0.0.1:9", SECRET).unwrap());
        let e = c.worlds("Steve").await.unwrap_err();
        assert!(
            matches!(e, CloudError::Connect | CloudError::Timeout),
            "{e:?}"
        );
    }

    #[tokio::test]
    async fn create_posts_the_world_and_returns_the_port() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/lobbies/create"))
            .and(header(SECRET_HEADER, SECRET))
            .and(body_json(serde_json::json!({
                "owner": "Steve", "worldName": "Дача", "seed": "",
                "worldId": null, "version": "1.21.1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ip": "10.0.0.7", "port": 25602, "status": "starting", "worldId": "w9",
                "name": "Дача", "owner": "Steve", "error": null, "game_version": "1.21.1"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let r = client(&server)
            .start(&StartRequest {
                owner: "Steve".into(),
                world_name: " Дача ".into(),
                seed: Some("  ".into()),
                world_id: None,
                version: "1.21.1".into(),
            })
            .await
            .unwrap();
        assert_eq!(r.port, Some(25602));
        assert_eq!(r.world_id.as_deref(), Some("w9"));
    }

    #[tokio::test]
    async fn wake_sends_the_world_id_and_seed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/lobbies/create"))
            .and(body_json(serde_json::json!({
                "owner": "Steve", "worldName": "Дача", "seed": "42",
                "worldId": "w1", "version": "1.20.1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "port": 25601, "status": "running", "worldId": "w1"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let r = client(&server)
            .start(&StartRequest {
                owner: "Steve".into(),
                world_name: "Дача".into(),
                seed: Some("42".into()),
                world_id: Some("w1".into()),
                version: "1.20.1".into(),
            })
            .await
            .unwrap();
        assert_eq!(r.status, "running");
    }

    #[tokio::test]
    async fn create_error_field_is_shown_to_the_player() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/lobbies/create"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "status": "error", "error": "Достигнут лимит активных миров (3)"
            })))
            .mount(&server)
            .await;
        let e = client(&server)
            .start(&StartRequest {
                owner: "Steve".into(),
                world_name: "Дача".into(),
                seed: None,
                world_id: None,
                version: "1.21.1".into(),
            })
            .await
            .unwrap_err();
        assert_eq!(e.to_string(), "Достигнут лимит активных миров (3)");
    }

    #[tokio::test]
    async fn create_without_port_is_rejected() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "status": "ok" })),
            )
            .mount(&server)
            .await;
        let e = client(&server)
            .start(&StartRequest {
                owner: "Steve".into(),
                world_name: "Дача".into(),
                seed: None,
                world_id: None,
                version: "1.21.1".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(e, CloudError::Rejected(_)));
    }

    #[tokio::test]
    async fn create_checks_version_locally() {
        let server = MockServer::start().await;
        let e = client(&server)
            .start(&StartRequest {
                owner: "Steve".into(),
                world_name: "Дача".into(),
                seed: None,
                world_id: None,
                version: "1.8.9".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(e, CloudError::Invalid(_)));
        assert!(server.received_requests().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn stop_sends_id_and_port() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/lobbies/stop"))
            .and(header(SECRET_HEADER, SECRET))
            .and(body_json(
                serde_json::json!({ "worldId": "w1", "port": 25601 }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&server)
            .await;
        client(&server).stop("w1", Some(25601)).await.unwrap();
    }

    #[tokio::test]
    async fn stop_failure_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(404).set_body_string("unknown"))
            .mount(&server)
            .await;
        assert!(client(&server).stop("w1", None).await.is_err());
    }

    #[tokio::test]
    async fn share_adds_and_removes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/lobbies/share"))
            .and(body_json(serde_json::json!({
                "owner": "Steve", "worldId": "w1", "targetPlayer": "Alex", "action": "add"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true, "allowed_players": ["Alex"]
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/lobbies/share"))
            .and(body_json(serde_json::json!({
                "owner": "Steve", "worldId": "w1", "targetPlayer": "Alex", "action": "remove"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true, "allowed_players": null
            })))
            .expect(1)
            .mount(&server)
            .await;
        let c = client(&server);
        assert_eq!(
            c.share("Steve", "w1", "Alex", true).await.unwrap(),
            vec!["Alex"]
        );
        assert!(c
            .share("Steve", "w1", "Alex", false)
            .await
            .unwrap()
            .is_empty());
        assert!(matches!(
            c.share("Steve", "w1", "Ал", true).await.unwrap_err(),
            CloudError::Invalid(_)
        ));
    }

    #[tokio::test]
    async fn share_refusal_carries_the_reason() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "success": false, "error": "not owner"
            })))
            .mount(&server)
            .await;
        // 403 means the secret was refused, before any body is read.
        assert!(matches!(
            client(&server)
                .share("Steve", "w1", "Alex", true)
                .await
                .unwrap_err(),
            CloudError::Unauthorized
        ));
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": false, "error": "Только владелец может делиться миром"
            })))
            .mount(&server)
            .await;
        let e = client(&server)
            .share("Alex", "w1", "Steve", true)
            .await
            .unwrap_err();
        assert_eq!(e.to_string(), "Только владелец может делиться миром");
    }

    #[tokio::test]
    async fn delete_posts_owner_and_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/worlds/delete"))
            .and(header(SECRET_HEADER, SECRET))
            .and(body_json(
                serde_json::json!({ "owner": "Steve", "worldId": "w1" }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": true, "message": "deleted"
            })))
            .expect(1)
            .mount(&server)
            .await;
        client(&server).delete("Steve", "w1").await.unwrap();
    }

    #[tokio::test]
    async fn delete_refusal_carries_the_message() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "success": false, "message": "Сначала остановите мир"
            })))
            .mount(&server)
            .await;
        let e = client(&server).delete("Steve", "w1").await.unwrap_err();
        assert_eq!(e.to_string(), "Сначала остановите мир");
        assert!(client(&server).delete("Steve", " ").await.is_err());
    }
}
