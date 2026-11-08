use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_DISCORD_APP_ID: &str = "1556968494673690674";

#[derive(Debug, Error)]
pub enum DiscordError {
    #[error("Ошибка ввода-вывода IPC: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка сериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Некорректный заголовок пакета IPC: длина {0}")]
    InvalidPacket(usize),
}

pub const OP_HANDSHAKE: u32 = 0;
pub const OP_FRAME: u32 = 1;
pub const OP_CLOSE: u32 = 2;
pub const OP_PING: u32 = 3;
pub const OP_PONG: u32 = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordButton {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Activity {
    InLauncher,
    Playing {
        version: String,
        world_name: Option<String>,
        mod_count: Option<usize>,
        start_timestamp: Option<i64>,
    },
    InLobby {
        lobby_code: Option<String>,
        player_count: usize,
        host_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordActivityPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<Timestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<Assets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<DiscordButton>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assets {
    pub large_image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

pub fn encode_ipc_packet(opcode: u32, payload: &str) -> Vec<u8> {
    let payload_bytes = payload.as_bytes();
    let mut out = Vec::with_capacity(8 + payload_bytes.len());
    out.extend_from_slice(&opcode.to_le_bytes());
    out.extend_from_slice(&(payload_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(payload_bytes);
    out
}

pub fn decode_ipc_packet(bytes: &[u8]) -> Result<(u32, String), DiscordError> {
    if bytes.len() < 8 {
        return Err(DiscordError::InvalidPacket(bytes.len()));
    }
    let opcode = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let length = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    if bytes.len() < 8 + length {
        return Err(DiscordError::InvalidPacket(bytes.len()));
    }
    let payload_str = String::from_utf8_lossy(&bytes[8..8 + length]).to_string();
    Ok((opcode, payload_str))
}

pub fn default_buttons() -> Vec<DiscordButton> {
    vec![DiscordButton {
        label: "Скачать Aura Launcher".to_string(),
        url: "https://github.com/qutlawsoasis-debug/Aura-Launcher".to_string(),
    }]
}

pub fn build_activity_payload(pid: u32, activity: &Activity, nonce: &str) -> serde_json::Value {
    let (details, state, timestamps, assets) = match activity {
        Activity::InLauncher => (
            Some("В главном меню".to_string()),
            Some("Выбирает режим игры".to_string()),
            None,
            Some(Assets {
                large_image: "aura_logo".to_string(),
                large_text: Some("Aura Launcher".to_string()),
                small_image: None,
                small_text: None,
            }),
        ),
        Activity::Playing {
            version,
            world_name,
            mod_count,
            start_timestamp,
        } => {
            let details_str = if let Some(w) = world_name {
                format!("В мире: {}", w)
            } else {
                format!("В мире Minecraft ({})", version)
            };

            let state_str = if let Some(count) = mod_count {
                format!("Сборка Aura Pack • Модов: {}", count)
            } else {
                "Сборка Aura Pack".to_string()
            };

            let ts = start_timestamp.map(|s| Timestamps { start: Some(s) });

            (
                Some(details_str),
                Some(state_str),
                ts,
                Some(Assets {
                    large_image: "aura_logo".to_string(),
                    large_text: Some("Aura Launcher".to_string()),
                    small_image: Some("minecraft".to_string()),
                    small_text: Some(format!("Minecraft {}", version)),
                }),
            )
        }
        Activity::InLobby {
            lobby_code,
            player_count,
            host_name,
        } => {
            let details_str = if let Some(code) = lobby_code {
                format!("В лобби #{}", code)
            } else {
                "В лобби мультиплеера".to_string()
            };

            let state_str = if let Some(host) = host_name {
                format!("Игроков: {}/8 • Хост: {}", player_count, host)
            } else {
                format!("Игроков: {}/8", player_count)
            };

            (
                Some(details_str),
                Some(state_str),
                None,
                Some(Assets {
                    large_image: "aura_logo".to_string(),
                    large_text: Some("Aura Launcher".to_string()),
                    small_image: Some("online".to_string()),
                    small_text: Some("P2P Multiplayer".to_string()),
                }),
            )
        }
    };

    let activity_obj = DiscordActivityPayload {
        details,
        state,
        timestamps,
        assets,
        buttons: Some(default_buttons()),
    };

    serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": activity_obj
        },
        "nonce": nonce
    })
}

pub fn get_pipe_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(windows)]
    {
        for i in 0..10 {
            paths.push(PathBuf::from(format!(r"\\.\pipe\discord-ipc-{}", i)));
        }
    }
    #[cfg(not(windows))]
    {
        let base_dirs = vec![
            std::env::var("XDG_RUNTIME_DIR").ok().map(PathBuf::from),
            std::env::var("TMPDIR").ok().map(PathBuf::from),
            Some(PathBuf::from("/tmp")),
        ];
        for base in base_dirs.into_iter().flatten() {
            for i in 0..10 {
                paths.push(base.join(format!("discord-ipc-{}", i)));
            }
        }
    }
    paths
}

pub struct DiscordRpcClient {
    app_id: String,
    connected: bool,
    #[cfg(windows)]
    pipe_stream: Option<tokio::net::windows::named_pipe::NamedPipeClient>,
}

impl DiscordRpcClient {
    pub async fn connect_or_noop(app_id: &str) -> Self {
        #[cfg(windows)]
        {
            use tokio::net::windows::named_pipe::ClientOptions;

            for pipe_path in get_pipe_paths() {
                if let Ok(client) = ClientOptions::new().open(&pipe_path) {
                    // Send handshake
                    let handshake_json = serde_json::json!({
                        "v": 1,
                        "client_id": app_id
                    })
                    .to_string();
                    let packet = encode_ipc_packet(OP_HANDSHAKE, &handshake_json);
                    use tokio::io::AsyncWriteExt;
                    let mut stream = client;
                    if stream.write_all(&packet).await.is_ok() {
                        return Self {
                            app_id: app_id.to_string(),
                            connected: true,
                            pipe_stream: Some(stream),
                        };
                    }
                }
            }
        }

        // Fallback: silent no-op when Discord is not running
        Self {
            app_id: app_id.to_string(),
            connected: false,
            #[cfg(windows)]
            pipe_stream: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn app_id(&self) -> &str {
        &self.app_id
    }

    pub async fn set_activity(&mut self, activity: Activity) {
        if !self.connected {
            return;
        }

        let pid = std::process::id();
        let nonce = Uuid::new_v4().to_string();
        let payload = build_activity_payload(pid, &activity, &nonce);
        let payload_str = payload.to_string();
        let packet = encode_ipc_packet(OP_FRAME, &payload_str);

        #[cfg(windows)]
        {
            if let Some(stream) = &mut self.pipe_stream {
                use tokio::io::AsyncWriteExt;
                if stream.write_all(&packet).await.is_err() {
                    self.connected = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_packet_encoding_and_decoding() {
        let payload = r#"{"v":1,"client_id":"1556968494673690674"}"#;
        let encoded = encode_ipc_packet(OP_HANDSHAKE, payload);

        assert_eq!(encoded.len(), 8 + payload.len());
        let (opcode, decoded_str) = decode_ipc_packet(&encoded).unwrap();
        assert_eq!(opcode, OP_HANDSHAKE);
        assert_eq!(decoded_str, payload);
    }

    #[test]
    fn test_build_activity_payloads() {
        let pid = 12345;
        let nonce = "test-nonce-123";

        // 1. Launcher
        let launcher = Activity::InLauncher;
        let p1 = build_activity_payload(pid, &launcher, nonce);
        assert_eq!(p1["cmd"], "SET_ACTIVITY");
        assert_eq!(p1["args"]["activity"]["details"], "В главном меню");
        assert_eq!(p1["args"]["activity"]["state"], "Выбирает режим игры");
        assert_eq!(p1["args"]["activity"]["assets"]["large_image"], "aura_logo");

        // 2. Playing
        let playing = Activity::Playing {
            version: "1.20.1".to_string(),
            world_name: Some("Survival World".to_string()),
            mod_count: Some(184),
            start_timestamp: Some(1730000000),
        };
        let p2 = build_activity_payload(pid, &playing, nonce);
        assert_eq!(p2["args"]["activity"]["details"], "В мире: Survival World");
        assert_eq!(
            p2["args"]["activity"]["state"],
            "Сборка Aura Pack • Модов: 184"
        );
        assert_eq!(p2["args"]["activity"]["timestamps"]["start"], 1730000000);

        // 3. Lobby
        let lobby = Activity::InLobby {
            lobby_code: Some("AURA77".to_string()),
            player_count: 4,
            host_name: Some("HostPlayer".to_string()),
        };
        let p3 = build_activity_payload(pid, &lobby, nonce);
        assert_eq!(p3["args"]["activity"]["details"], "В лобби #AURA77");
        assert_eq!(
            p3["args"]["activity"]["state"],
            "Игроков: 4/8 • Хост: HostPlayer"
        );
    }

    #[tokio::test]
    async fn test_discord_connect_or_noop_does_not_panic_when_offline() {
        let mut client = DiscordRpcClient::connect_or_noop("1556968494673690674").await;
        // Even if Discord is not running, calling set_activity must silently succeed without errors
        client.set_activity(Activity::InLauncher).await;
        client
            .set_activity(Activity::Playing {
                version: "1.21.1".to_string(),
                world_name: None,
                mod_count: Some(140),
                start_timestamp: None,
            })
            .await;
    }
}
