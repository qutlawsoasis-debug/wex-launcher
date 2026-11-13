//! Microsoft sign-in for Minecraft: OAuth 2.0 device code flow, then
//! Xbox Live → XSTS → Minecraft services.
//!
//! The device code flow suits a desktop launcher: Wex shows a short code, the
//! player enters it on microsoft.com/link in any browser, and no password ever
//! passes through the launcher.
//!
//! Endpoints and request bodies follow Microsoft identity platform
//! (`/consumers/oauth2/v2.0/devicecode`, `/token`) and the Xbox / Minecraft
//! services used by the official launcher.

use crate::error::{Result, WexError};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration as StdDuration;
use tokio_util::sync::CancellationToken;

pub const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone)]
pub struct MsEndpoints {
    /// `https://login.microsoftonline.com/consumers/oauth2/v2.0`
    pub oauth: String,
    /// `https://user.auth.xboxlive.com`
    pub xbl: String,
    /// `https://xsts.auth.xboxlive.com`
    pub xsts: String,
    /// `https://api.minecraftservices.com`
    pub minecraft: String,
}

impl Default for MsEndpoints {
    fn default() -> Self {
        Self {
            oauth: "https://login.microsoftonline.com/consumers/oauth2/v2.0".to_string(),
            xbl: "https://user.auth.xboxlive.com".to_string(),
            xsts: "https://xsts.auth.xboxlive.com".to_string(),
            minecraft: "https://api.minecraftservices.com".to_string(),
        }
    }
}

impl MsEndpoints {
    /// All four services on one base URL (tests).
    pub fn single(base: &str) -> Self {
        Self {
            oauth: format!("{}/oauth", base),
            xbl: format!("{}/xbl", base),
            xsts: format!("{}/xsts", base),
            minecraft: format!("{}/mc", base),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Clone, Deserialize)]
pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub expires_in: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollOutcome {
    Pending,
    SlowDown,
}

/// Result of the whole chain for one account.
#[derive(Clone)]
pub struct MinecraftLogin {
    pub uuid: String,
    pub name: String,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub xuid: Option<String>,
    pub refresh_token: String,
}

#[derive(Deserialize)]
struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct XboxResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxClaims,
}

#[derive(Deserialize)]
struct XboxClaims {
    xui: Vec<XboxUser>,
}

#[derive(Deserialize)]
struct XboxUser {
    uhs: String,
    #[serde(default)]
    xid: Option<String>,
}

#[derive(Deserialize)]
struct XstsError {
    #[serde(rename = "XErr", default)]
    xerr: u64,
}

#[derive(Deserialize)]
struct McLogin {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

#[derive(Deserialize)]
struct McProfile {
    id: String,
    name: String,
}

fn auth(code: &'static str, message: &str, details: Option<String>) -> WexError {
    WexError::Auth {
        code,
        message: message.to_string(),
        details,
    }
}

/// Network failures get one honest message; other HTTP errors keep their code.
fn net(e: reqwest::Error) -> WexError {
    if e.is_connect() || e.is_timeout() {
        auth(
            "AUTH_OFFLINE",
            "Нет связи с серверами Microsoft. Проверьте интернет и повторите вход.",
            Some(e.to_string()),
        )
    } else {
        WexError::Network(e)
    }
}

/// Explain an XSTS refusal (`XErr`) in words the player can act on.
pub fn explain_xsts_error(xerr: u64) -> (&'static str, &'static str) {
    match xerr {
        2148916233 => (
            "AUTH_NO_XBOX_ACCOUNT",
            "У этой учётной записи Microsoft нет профиля Xbox. Войдите один раз на xbox.com, создайте профиль и повторите вход.",
        ),
        2148916235 => (
            "AUTH_XBOX_REGION",
            "Xbox Live недоступен в стране этой учётной записи.",
        ),
        2148916236 | 2148916237 => (
            "AUTH_XBOX_ADULT_VERIFICATION",
            "Учётной записи нужно подтверждение возраста на xbox.com.",
        ),
        2148916238 => (
            "AUTH_XBOX_CHILD",
            "Это детская учётная запись. Взрослый должен добавить её в семью Microsoft, затем повторите вход.",
        ),
        _ => (
            "AUTH_XBOX_REFUSED",
            "Xbox Live отклонил вход. Повторите попытку позже.",
        ),
    }
}

pub struct MsAuth {
    client: reqwest::Client,
    client_id: String,
    ep: MsEndpoints,
}

impl MsAuth {
    pub fn new(client: reqwest::Client, client_id: impl Into<String>) -> Self {
        Self::with_endpoints(client, client_id, MsEndpoints::default())
    }

    pub fn with_endpoints(
        client: reqwest::Client,
        client_id: impl Into<String>,
        ep: MsEndpoints,
    ) -> Self {
        Self {
            client,
            client_id: client_id.into(),
            ep,
        }
    }

    /// Step 1: ask Microsoft for a code the player types on the verification page.
    pub async fn start_device_code(&self) -> Result<DeviceCode> {
        let resp = self
            .client
            .post(format!("{}/devicecode", self.ep.oauth))
            .form(&[("client_id", self.client_id.as_str()), ("scope", SCOPE)])
            .send()
            .await
            .map_err(net)?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let err: Option<OAuthError> = serde_json::from_str(&body).ok();
            let code = err.as_ref().map(|e| e.error.as_str()).unwrap_or("");
            if code == "unauthorized_client" || code == "invalid_client" {
                return Err(auth(
                    "AUTH_CLIENT_REJECTED",
                    "Microsoft не принимает ID приложения Wex для входа. Проверьте WEX_MS_CLIENT_ID.",
                    Some(body),
                ));
            }
            return Err(auth(
                "AUTH_MS_FAILED",
                "Microsoft не начал вход. Повторите попытку позже.",
                Some(format!("HTTP {}: {}", status, body)),
            ));
        }
        resp.json::<DeviceCode>().await.map_err(net)
    }

    /// Step 2 (one attempt): has the player finished on the verification page?
    pub async fn poll_device_code(
        &self,
        code: &DeviceCode,
    ) -> Result<std::result::Result<MsTokens, PollOutcome>> {
        let resp = self
            .client
            .post(format!("{}/token", self.ep.oauth))
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", self.client_id.as_str()),
                ("device_code", code.device_code.as_str()),
            ])
            .send()
            .await
            .map_err(net)?;
        if resp.status().is_success() {
            return Ok(Ok(resp.json::<MsTokens>().await.map_err(net)?));
        }
        let body = resp.text().await.unwrap_or_default();
        let err: OAuthError = serde_json::from_str(&body).map_err(|_| {
            auth(
                "AUTH_MS_FAILED",
                "Microsoft вернул непонятный ответ. Повторите вход.",
                Some(body.clone()),
            )
        })?;
        match err.error.as_str() {
            "authorization_pending" => Ok(Err(PollOutcome::Pending)),
            "slow_down" => Ok(Err(PollOutcome::SlowDown)),
            "authorization_declined" => Err(auth(
                "AUTH_DECLINED",
                "Вход отменён на странице Microsoft.",
                None,
            )),
            "expired_token" | "code_expired" => Err(auth(
                "AUTH_CODE_EXPIRED",
                "Код истёк. Начните вход заново, чтобы получить новый код.",
                None,
            )),
            _ => Err(auth(
                "AUTH_MS_FAILED",
                "Microsoft отклонил вход. Повторите попытку.",
                Some(format!(
                    "{}: {}",
                    err.error,
                    err.error_description.unwrap_or_default()
                )),
            )),
        }
    }

    /// Poll until the player finishes, the code expires or `cancel` fires.
    pub async fn wait_for_device_code(
        &self,
        code: &DeviceCode,
        cancel: &CancellationToken,
    ) -> Result<MsTokens> {
        let deadline = tokio::time::Instant::now() + StdDuration::from_secs(code.expires_in.max(1));
        let mut interval = code.interval;
        loop {
            match self.poll_device_code(code).await? {
                Ok(tokens) => return Ok(tokens),
                Err(PollOutcome::SlowDown) => interval += 5,
                Err(PollOutcome::Pending) => {}
            }
            let wait = StdDuration::from_millis(interval * 1000);
            if tokio::time::Instant::now() + wait > deadline {
                return Err(auth(
                    "AUTH_CODE_EXPIRED",
                    "Код истёк. Начните вход заново, чтобы получить новый код.",
                    None,
                ));
            }
            tokio::select! {
                _ = cancel.cancelled() => return Err(WexError::Cancelled),
                _ = tokio::time::sleep(wait) => {}
            }
        }
    }

    /// Exchange a refresh token for new tokens. Microsoft may rotate the refresh token.
    pub async fn refresh(&self, refresh_token: &str) -> Result<MsTokens> {
        let resp = self
            .client
            .post(format!("{}/token", self.ep.oauth))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", self.client_id.as_str()),
                ("refresh_token", refresh_token),
                ("scope", SCOPE),
            ])
            .send()
            .await
            .map_err(net)?;
        if resp.status().is_success() {
            let mut tokens = resp.json::<MsTokens>().await.map_err(net)?;
            if tokens.refresh_token.is_empty() {
                tokens.refresh_token = refresh_token.to_string();
            }
            return Ok(tokens);
        }
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_client_error() {
            return Err(auth(
                "AUTH_SIGN_IN_AGAIN",
                "Сохранённый вход Microsoft больше не действует. Войдите в аккаунт ещё раз.",
                Some(body),
            ));
        }
        Err(auth(
            "AUTH_MS_FAILED",
            "Microsoft не обновил вход. Повторите попытку позже.",
            Some(format!("HTTP {}: {}", status, body)),
        ))
    }

    /// Xbox Live user token from the Microsoft access token.
    async fn xbox_live(&self, ms_access_token: &str) -> Result<(String, String)> {
        let body = json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={}", ms_access_token),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        });
        let resp = self
            .client
            .post(format!("{}/user/authenticate", self.ep.xbl))
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(net)?;
        if !resp.status().is_success() {
            let status = resp.status();
            return Err(auth(
                "AUTH_XBOX_REFUSED",
                "Xbox Live отклонил вход. Повторите попытку позже.",
                Some(format!("XBL HTTP {}", status)),
            ));
        }
        let x: XboxResponse = resp.json().await.map_err(net)?;
        let uhs = x
            .display_claims
            .xui
            .first()
            .map(|u| u.uhs.clone())
            .ok_or_else(|| {
                auth(
                    "AUTH_XBOX_REFUSED",
                    "Xbox Live вернул неполный ответ.",
                    None,
                )
            })?;
        Ok((x.token, uhs))
    }

    /// XSTS token for Minecraft services.
    async fn xsts(&self, xbl_token: &str) -> Result<(String, String, Option<String>)> {
        let body = json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        });
        let resp = self
            .client
            .post(format!("{}/xsts/authorize", self.ep.xsts))
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(net)?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            let text = resp.text().await.unwrap_or_default();
            let xerr = serde_json::from_str::<XstsError>(&text)
                .map(|e| e.xerr)
                .unwrap_or(0);
            let (code, msg) = explain_xsts_error(xerr);
            return Err(auth(code, msg, Some(format!("XErr {}", xerr))));
        }
        if !resp.status().is_success() {
            return Err(auth(
                "AUTH_XBOX_REFUSED",
                "Xbox Live отклонил вход. Повторите попытку позже.",
                Some(format!("XSTS HTTP {}", resp.status())),
            ));
        }
        let x: XboxResponse = resp.json().await.map_err(net)?;
        let user = x.display_claims.xui.first().ok_or_else(|| {
            auth(
                "AUTH_XBOX_REFUSED",
                "Xbox Live вернул неполный ответ.",
                None,
            )
        })?;
        Ok((x.token, user.uhs.clone(), user.xid.clone()))
    }

    async fn minecraft_login(&self, uhs: &str, xsts_token: &str) -> Result<McLogin> {
        let resp = self
            .client
            .post(format!(
                "{}/authentication/login_with_xbox",
                self.ep.minecraft
            ))
            .json(&json!({ "identityToken": format!("XBL3.0 x={};{}", uhs, xsts_token) }))
            .send()
            .await
            .map_err(net)?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::FORBIDDEN && body.contains("Invalid app registration")
            {
                return Err(auth(
                    "AUTH_APP_NOT_APPROVED",
                    "Minecraft пока не разрешил вход через это приложение Microsoft. Нужна проверка ID приложения у Mojang.",
                    Some(body),
                ));
            }
            return Err(auth(
                "AUTH_MINECRAFT_FAILED",
                "Сервис Minecraft не принял вход. Повторите попытку позже.",
                Some(format!("HTTP {}: {}", status, body)),
            ));
        }
        resp.json().await.map_err(net)
    }

    async fn profile(&self, mc_access_token: &str) -> Result<McProfile> {
        let resp = self
            .client
            .get(format!("{}/minecraft/profile", self.ep.minecraft))
            .bearer_auth(mc_access_token)
            .send()
            .await
            .map_err(net)?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(auth(
                "AUTH_NO_MINECRAFT",
                "На этой учётной записи нет Minecraft: Java Edition. Купите игру или войдите без лицензии.",
                None,
            ));
        }
        if !resp.status().is_success() {
            return Err(auth(
                "AUTH_MINECRAFT_FAILED",
                "Сервис Minecraft не отдал профиль игрока. Повторите попытку позже.",
                Some(format!("profile HTTP {}", resp.status())),
            ));
        }
        resp.json().await.map_err(net)
    }

    /// Steps 3–6: Xbox Live → XSTS → Minecraft token → profile.
    pub async fn complete(&self, ms: MsTokens) -> Result<MinecraftLogin> {
        let (xbl_token, _) = self.xbox_live(&ms.access_token).await?;
        let (xsts_token, uhs, xuid) = self.xsts(&xbl_token).await?;
        let mc = self.minecraft_login(&uhs, &xsts_token).await?;
        let profile = self.profile(&mc.access_token).await?;
        let lifetime = if mc.expires_in == 0 {
            86_400
        } else {
            mc.expires_in
        };
        Ok(MinecraftLogin {
            uuid: profile.id.replace('-', "").to_ascii_lowercase(),
            name: profile.name,
            access_token: mc.access_token,
            expires_at: Utc::now() + Duration::seconds(lifetime as i64),
            xuid,
            refresh_token: ms.refresh_token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mount_chain(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/xbl/user/authenticate"))
            .and(body_string_contains("d=ms-access"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "IssueInstant": "2026-11-13T00:00:00Z",
                "Token": "xbl-token",
                "DisplayClaims": { "xui": [ { "uhs": "user-hash" } ] }
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/xsts/xsts/authorize"))
            .and(body_string_contains("xbl-token"))
            .and(body_string_contains("rp://api.minecraftservices.com/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Token": "xsts-token",
                "DisplayClaims": { "xui": [ { "uhs": "user-hash", "xid": "2535412345" } ] }
            })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/mc/authentication/login_with_xbox"))
            .and(body_string_contains("XBL3.0 x=user-hash;xsts-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "username": "some-guid",
                "access_token": "mc-access",
                "token_type": "Bearer",
                "expires_in": 86400
            })))
            .mount(server)
            .await;
    }

    fn tokens() -> MsTokens {
        MsTokens {
            access_token: "ms-access".into(),
            refresh_token: "ms-refresh".into(),
            expires_in: 3600,
        }
    }

    #[tokio::test]
    async fn full_chain_returns_profile_and_tokens() {
        let server = MockServer::start().await;
        mount_chain(&server).await;
        Mock::given(method("GET"))
            .and(path("/mc/minecraft/profile"))
            .and(header("authorization", "Bearer mc-access"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "069a79f444e94726a5befca90e38aaf5",
                "name": "Notch",
                "skins": [], "capes": []
            })))
            .mount(&server)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        let login = a.complete(tokens()).await.unwrap();
        assert_eq!(login.name, "Notch");
        assert_eq!(login.uuid, "069a79f444e94726a5befca90e38aaf5");
        assert_eq!(login.access_token, "mc-access");
        assert_eq!(login.xuid.as_deref(), Some("2535412345"));
        assert_eq!(login.refresh_token, "ms-refresh");
        assert!(login.expires_at > Utc::now() + Duration::hours(23));
    }

    #[tokio::test]
    async fn no_game_on_account_is_explained() {
        let server = MockServer::start().await;
        mount_chain(&server).await;
        Mock::given(method("GET"))
            .and(path("/mc/minecraft/profile"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "error": "NOT_FOUND", "errorMessage": "The server has not found anything matching the request URI"
            })))
            .mount(&server)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        match a.complete(tokens()).await {
            Err(WexError::Auth { code, .. }) => assert_eq!(code, "AUTH_NO_MINECRAFT"),
            other => panic!("unexpected {:?}", other.map(|l| l.name)),
        }
    }

    #[tokio::test]
    async fn xsts_child_account_is_explained() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/xbl/user/authenticate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "Token": "xbl-token", "DisplayClaims": { "xui": [ { "uhs": "h" } ] }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/xsts/xsts/authorize"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "Identity": "0", "XErr": 2148916238u64, "Message": "", "Redirect": "https://start.ui.xboxlive.com/AddChildToFamily"
            })))
            .mount(&server)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        match a.complete(tokens()).await {
            Err(WexError::Auth { code, .. }) => assert_eq!(code, "AUTH_XBOX_CHILD"),
            other => panic!("unexpected {:?}", other.map(|l| l.name)),
        }
        assert_eq!(explain_xsts_error(2148916233).0, "AUTH_NO_XBOX_ACCOUNT");
    }

    #[tokio::test]
    async fn device_code_flow_waits_then_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/devicecode"))
            .and(body_string_contains("client_id=cid"))
            .and(body_string_contains("offline_access"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "device_code": "dev-123",
                "user_code": "ABCD-EFGH",
                "verification_uri": "https://www.microsoft.com/link",
                "expires_in": 900,
                "interval": 0,
                "message": "To sign in, use a web browser..."
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("device_code=dev-123"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "authorization_pending", "error_description": "pending"
            })))
            .up_to_n_times(2)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("device_code=dev-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "token_type": "Bearer", "scope": SCOPE, "expires_in": 3600,
                "access_token": "ms-access", "refresh_token": "ms-refresh"
            })))
            .mount(&server)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        let code = a.start_device_code().await.unwrap();
        assert_eq!(code.user_code, "ABCD-EFGH");
        let tokens = a
            .wait_for_device_code(&code, &CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(tokens.refresh_token, "ms-refresh");
    }

    #[tokio::test]
    async fn device_code_declined_and_cancel() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(json!({ "error": "authorization_declined" })),
            )
            .mount(&server)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        let code = DeviceCode {
            device_code: "d".into(),
            user_code: "U".into(),
            verification_uri: "https://www.microsoft.com/link".into(),
            expires_in: 60,
            interval: 1,
        };
        assert!(matches!(
            a.wait_for_device_code(&code, &CancellationToken::new())
                .await,
            Err(WexError::Auth {
                code: "AUTH_DECLINED",
                ..
            })
        ));

        let pending = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .respond_with(
                ResponseTemplate::new(400)
                    .set_body_json(json!({ "error": "authorization_pending" })),
            )
            .mount(&pending)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&pending.uri()),
        );
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            a.wait_for_device_code(&code, &cancel).await,
            Err(WexError::Cancelled)
        ));
    }

    #[tokio::test]
    async fn refresh_rotates_token_and_rejection_asks_to_sign_in() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .and(body_string_contains("refresh_token=good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "ms-access-2", "refresh_token": "rotated", "expires_in": 3600
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/oauth/token"))
            .and(body_string_contains("refresh_token=revoked"))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })),
            )
            .mount(&server)
            .await;
        let a = MsAuth::with_endpoints(
            reqwest::Client::new(),
            "cid",
            MsEndpoints::single(&server.uri()),
        );
        assert_eq!(a.refresh("good").await.unwrap().refresh_token, "rotated");
        assert!(matches!(
            a.refresh("revoked").await,
            Err(WexError::Auth {
                code: "AUTH_SIGN_IN_AGAIN",
                ..
            })
        ));
    }
}
