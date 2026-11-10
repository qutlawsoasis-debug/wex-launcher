//! Real Playit mapping and Minecraft reachability, replaceable at the process boundary in tests.
use super::lobby::LobbyState;
use std::time::Duration;
use wex_net::{lobby::TunnelConfigResponse, PlayitSupervisor, ServerPinger, TunnelStatus};

#[async_trait::async_trait]
pub trait TunnelRuntime: Send + Sync {
    async fn start(
        &self,
        state: &LobbyState,
        config: &TunnelConfigResponse,
        local_port: u16,
    ) -> Result<String, String>;
}

pub struct PlayitRuntime;

#[async_trait::async_trait]
impl TunnelRuntime for PlayitRuntime {
    async fn start(
        &self,
        state: &LobbyState,
        config: &TunnelConfigResponse,
        local_port: u16,
    ) -> Result<String, String> {
        if local_port == 0 {
            return Err("Не указан порт открытого LAN-мира".into());
        }
        let secret = config
            .secret
            .as_deref()
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or("Сервис туннелей не предоставил действующий ключ агента")?;
        let host = config
            .public_address
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .ok_or("Сервис туннелей не предоставил публичный адрес")?;
        let public_port = config
            .public_port
            .filter(|p| *p > 0)
            .ok_or("Сервис туннелей не предоставил публичный порт")?;
        let public_address = format!("{host}:{public_port}");
        let pinger = ServerPinger::new(1500, 1500, 763);
        pinger
            .ping(&format!("127.0.0.1:{local_port}"))
            .await
            .map_err(|_| {
                "Откройте свой Minecraft-мир для сети и повторите публикацию".to_string()
            })?;

        // Protocol and field names are from playit-agent v0.15.26 api_client/src/api.rs.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        let response: serde_json::Value = client
            .post("https://api.playit.gg/agents/rundata")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Agent-Key {secret}"),
            )
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|_| "Не удалось связаться с Playit")?
            .error_for_status()
            .map_err(|_| "Playit отклонил запрос агента")?
            .json()
            .await
            .map_err(|_| "Некорректный ответ Playit")?;
        let tunnel_id = select_tunnel(&response, host, public_port)?;
        let binary = tokio::time::timeout(
            Duration::from_secs(60),
            wex_net::ensure_binary_downloaded(&state.tools_dir, Some(&state.event_bus)),
        )
        .await
        .map_err(|_| "Истекло время загрузки агента Playit")?
        .map_err(|e| format!("Не удалось подготовить агент Playit: {e}"))?;
        let secret_path = state.tools_dir.join("lobby-agent.secret");
        tokio::fs::write(&secret_path, secret)
            .await
            .map_err(|_| "Не удалось сохранить ключ агента")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&secret_path, std::fs::Permissions::from_mode(0o600))
                .await
                .map_err(|e| e.to_string())?;
        }
        let mut supervisor = PlayitSupervisor::new(Some(state.event_bus.clone()));
        if let Err(e) = supervisor
            .start_mapped(&binary, &secret_path, &tunnel_id, local_port)
            .await
        {
            let _ = tokio::fs::remove_file(&secret_path).await;
            return Err(format!("Не удалось запустить Playit: {e}"));
        }
        // A public Minecraft SLP reply, not possession of a configured address,
        // confirms that the mapped LAN endpoint is actually reachable.
        let ready = tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if !supervisor.is_alive().await {
                    return Err("Агент Playit завершился до открытия туннеля");
                }
                if pinger.ping(&public_address).await.is_ok() {
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        })
        .await;
        let _ = tokio::fs::remove_file(&secret_path).await;
        match ready {
            Ok(Ok(())) => {
                *state.supervisor.lock().await = Some(supervisor);
                *state.current_tunnel_status.lock().await = TunnelStatus::Running {
                    address: host.into(),
                    port: public_port,
                };
                Ok(public_address)
            }
            other => {
                supervisor.stop().await;
                Err(match other {
                    Ok(Err(message)) => message.to_string(),
                    _ => "Туннель не подтвердил доступность Minecraft-мира. Попробуйте ещё раз."
                        .into(),
                })
            }
        }
    }
}

fn select_tunnel(response: &serde_json::Value, host: &str, port: u16) -> Result<String, String> {
    if response.get("status").and_then(|v| v.as_str()) != Some("success") {
        return Err("Playit не подтвердил настройки агента".into());
    }
    response
        .pointer("/data/tunnels")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .find(|t| {
            t.get("disabled").is_none_or(|v| v.is_null())
                && (t.get("assigned_domain").and_then(|v| v.as_str()) == Some(host)
                    || t.get("custom_domain").and_then(|v| v.as_str()) == Some(host))
                && t.pointer("/port/from").and_then(|v| v.as_u64()) == Some(port as u64)
        })
        .and_then(|t| t.get("id"))
        .and_then(|id| id.as_str())
        .filter(|id| id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))
        .map(str::to_owned)
        .ok_or_else(|| "У агента нет действующего туннеля с указанным адресом".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapped_tunnel_requires_matching_active_allocation() {
        let mut response = serde_json::json!({"status":"success","data":{"tunnels":[{
            "id":"00000000-0000-0000-0000-000000000001", "assigned_domain":"test.ply.gg",
            "port":{"from":12345,"to":12346}, "disabled":null
        }]}});
        assert!(select_tunnel(&response, "test.ply.gg", 12345).is_ok());
        assert!(select_tunnel(&response, "other.ply.gg", 12345).is_err());
        response["data"]["tunnels"][0]["disabled"] = serde_json::json!("ByUser");
        assert!(select_tunnel(&response, "test.ply.gg", 12345).is_err());
    }
}
