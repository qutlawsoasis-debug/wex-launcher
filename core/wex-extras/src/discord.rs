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
    Installing {
        version: String,
        stage: Option<String>,
    },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscordPreview {
    pub is_enabled: bool,
    pub is_connected: bool,
    pub title: String,
    pub details: String,
    pub state: String,
    pub small_text: Option<String>,
    pub has_timestamp: bool,
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
        label: "Скачать Wex Launcher".to_string(),
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
                large_text: Some("Wex Launcher".to_string()),
                small_image: None,
                small_text: None,
            }),
        ),
        Activity::Installing { version, stage } => {
            let details_str = format!("Установка Minecraft {}", version);
            let state_str = if let Some(s) = stage {
                format!("Этап: {}", s)
            } else {
                "Загрузка ресурсов...".to_string()
            };
            (
                Some(details_str),
                Some(state_str),
                None,
                Some(Assets {
                    large_image: "aura_logo".to_string(),
                    large_text: Some("Wex Launcher".to_string()),
                    small_image: Some("minecraft".to_string()),
                    small_text: Some(format!("Minecraft {}", version)),
                }),
            )
        }
        Activity::Playing {
            version,
            world_name,
            mod_count,
            start_timestamp,
        } => {
            let details_str = if let Some(w) = world_name {
                if w.contains('.')
                    || w.contains(':')
                    || w.to_lowercase().contains("сервер")
                    || w.to_lowercase().contains("server")
                {
                    format!("На сервере: {}", w)
                } else {
                    format!("В мире: {}", w)
                }
            } else {
                format!("В мире Minecraft ({})", version)
            };

            let state_str = if let Some(count) = mod_count {
                format!("Сборка Wex • Модов: {}", count)
            } else {
                "Сборка Wex".to_string()
            };

            let ts = start_timestamp.map(|s| Timestamps { start: Some(s) });

            (
                Some(details_str),
                Some(state_str),
                ts,
                Some(Assets {
                    large_image: "aura_logo".to_string(),
                    large_text: Some("Wex Launcher".to_string()),
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
                    large_text: Some("Wex Launcher".to_string()),
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

pub fn build_clear_activity_payload(pid: u32, nonce: &str) -> serde_json::Value {
    serde_json::json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": serde_json::Value::Null
        },
        "nonce": nonce
    })
}

pub fn get_activity_preview(
    activity: Option<&Activity>,
    is_enabled: bool,
    is_connected: bool,
) -> DiscordPreview {
    if !is_enabled {
        return DiscordPreview {
            is_enabled: false,
            is_connected,
            title: "Wex Launcher".to_string(),
            details: "Статус отключён в настройках".to_string(),
            state: "Не отображается в Discord".to_string(),
            small_text: None,
            has_timestamp: false,
        };
    }

    match activity {
        None | Some(Activity::InLauncher) => DiscordPreview {
            is_enabled: true,
            is_connected,
            title: "Wex Launcher".to_string(),
            details: "В главном меню".to_string(),
            state: "Выбирает режим игры".to_string(),
            small_text: None,
            has_timestamp: false,
        },
        Some(Activity::Installing { version, stage }) => DiscordPreview {
            is_enabled: true,
            is_connected,
            title: "Wex Launcher".to_string(),
            details: format!("Установка Minecraft {}", version),
            state: if let Some(s) = stage {
                format!("Этап: {}", s)
            } else {
                "Загрузка ресурсов...".to_string()
            },
            small_text: Some(format!("Minecraft {}", version)),
            has_timestamp: false,
        },
        Some(Activity::Playing {
            version,
            world_name,
            mod_count,
            start_timestamp,
        }) => {
            let details = if let Some(w) = world_name {
                if w.contains('.')
                    || w.contains(':')
                    || w.to_lowercase().contains("сервер")
                    || w.to_lowercase().contains("server")
                {
                    format!("На сервере: {}", w)
                } else {
                    format!("В мире: {}", w)
                }
            } else {
                format!("В игре Minecraft {}", version)
            };

            let state = if let Some(count) = mod_count {
                format!("Сборка Wex • Модов: {}", count)
            } else {
                "Сборка Wex".to_string()
            };

            DiscordPreview {
                is_enabled: true,
                is_connected,
                title: "Wex Launcher".to_string(),
                details,
                state,
                small_text: Some(format!("Minecraft {}", version)),
                has_timestamp: start_timestamp.is_some(),
            }
        }
        Some(Activity::InLobby {
            lobby_code,
            player_count,
            host_name,
        }) => {
            let details = if let Some(code) = lobby_code {
                format!("В лобби #{}", code)
            } else {
                "В лобби мультиплеера".to_string()
            };

            let state = if let Some(host) = host_name {
                format!("Игроков: {}/8 • Хост: {}", player_count, host)
            } else {
                format!("Игроков: {}/8", player_count)
            };

            DiscordPreview {
                is_enabled: true,
                is_connected,
                title: "Wex Launcher".to_string(),
                details,
                state,
                small_text: Some("P2P Multiplayer".to_string()),
                has_timestamp: false,
            }
        }
    }
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
    enabled: bool,
    current_activity: Option<Activity>,
    packet_sent_count: usize,
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
                            enabled: true,
                            current_activity: None,
                            packet_sent_count: 1,
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
            enabled: true,
            current_activity: None,
            packet_sent_count: 0,
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

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn current_activity(&self) -> Option<&Activity> {
        self.current_activity.as_ref()
    }

    pub fn packet_sent_count(&self) -> usize {
        self.packet_sent_count
    }

    pub async fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        if !enabled {
            self.clear_activity().await;
        } else if let Some(act) = self.current_activity.clone() {
            // Re-apply current activity upon re-enabling
            self.current_activity = None;
            self.set_activity(act).await;
        }
    }

    pub async fn set_activity(&mut self, activity: Activity) {
        if !self.enabled {
            self.current_activity = Some(activity);
            return;
        }

        // Avoid duplicate packet sends when activity has not changed
        if self.current_activity.as_ref() == Some(&activity) {
            return;
        }

        self.current_activity = Some(activity.clone());

        if !self.connected {
            return;
        }

        let pid = std::process::id();
        let nonce = Uuid::new_v4().to_string();
        let payload = build_activity_payload(pid, &activity, &nonce);
        let payload_str = payload.to_string();
        let packet = encode_ipc_packet(OP_FRAME, &payload_str);
        self.packet_sent_count += 1;

        // The Discord IPC pipe is only implemented on Windows.
        #[cfg(not(windows))]
        let _ = packet;

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

    pub async fn clear_activity(&mut self) {
        if self.current_activity.is_none() && !self.connected {
            return;
        }

        self.current_activity = None;

        if !self.connected {
            return;
        }

        let pid = std::process::id();
        let nonce = Uuid::new_v4().to_string();
        let payload = build_clear_activity_payload(pid, &nonce);
        let payload_str = payload.to_string();
        let packet = encode_ipc_packet(OP_FRAME, &payload_str);
        self.packet_sent_count += 1;

        // The Discord IPC pipe is only implemented on Windows.
        #[cfg(not(windows))]
        let _ = packet;

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

    pub fn preview(&self) -> DiscordPreview {
        get_activity_preview(self.current_activity.as_ref(), self.enabled, self.connected)
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

        // 2. Installing
        let installing = Activity::Installing {
            version: "1.21.1".to_string(),
            stage: Some("Загрузка библиотек".to_string()),
        };
        let p_inst = build_activity_payload(pid, &installing, nonce);
        assert_eq!(
            p_inst["args"]["activity"]["details"],
            "Установка Minecraft 1.21.1"
        );
        assert_eq!(
            p_inst["args"]["activity"]["state"],
            "Этап: Загрузка библиотек"
        );

        // 3. Playing (Singleplayer / World)
        let playing_world = Activity::Playing {
            version: "1.20.1".to_string(),
            world_name: Some("Survival World".to_string()),
            mod_count: Some(184),
            start_timestamp: Some(1730000000),
        };
        let p2 = build_activity_payload(pid, &playing_world, nonce);
        assert_eq!(p2["args"]["activity"]["details"], "В мире: Survival World");
        assert_eq!(p2["args"]["activity"]["state"], "Сборка Wex • Модов: 184");
        assert_eq!(p2["args"]["activity"]["timestamps"]["start"], 1730000000);

        // 4. Playing on Server
        let playing_server = Activity::Playing {
            version: "1.21.1".to_string(),
            world_name: Some("mc.auramc.net:25565".to_string()),
            mod_count: None,
            start_timestamp: Some(1730000500),
        };
        let p_srv = build_activity_payload(pid, &playing_server, nonce);
        assert_eq!(
            p_srv["args"]["activity"]["details"],
            "На сервере: mc.auramc.net:25565"
        );
        assert_eq!(p_srv["args"]["activity"]["state"], "Сборка Wex");

        // 5. Lobby
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

    #[test]
    fn test_build_clear_activity_payload() {
        let pid = 12345;
        let nonce = "test-clear-nonce";
        let clear_payload = build_clear_activity_payload(pid, nonce);
        assert_eq!(clear_payload["cmd"], "SET_ACTIVITY");
        assert_eq!(clear_payload["args"]["pid"], pid);
        assert!(clear_payload["args"]["activity"].is_null());
    }

    #[tokio::test]
    async fn test_discord_state_transitions_and_deduplication() {
        let mut client = DiscordRpcClient::connect_or_noop(DEFAULT_DISCORD_APP_ID).await;

        // Transition 1: InLauncher
        client.set_activity(Activity::InLauncher).await;
        assert_eq!(client.current_activity(), Some(&Activity::InLauncher));

        // Deduplication: re-setting InLauncher should not send new packet
        let count_before = client.packet_sent_count();
        client.set_activity(Activity::InLauncher).await;
        assert_eq!(client.packet_sent_count(), count_before);

        // Transition 2: Installing
        client
            .set_activity(Activity::Installing {
                version: "1.21.1".to_string(),
                stage: Some("Подготовка".to_string()),
            })
            .await;
        assert!(matches!(
            client.current_activity(),
            Some(Activity::Installing { .. })
        ));

        // Transition 3: Playing on server
        client
            .set_activity(Activity::Playing {
                version: "1.21.1".to_string(),
                world_name: Some("mc.hypixel.net".to_string()),
                mod_count: None,
                start_timestamp: Some(1730000000),
            })
            .await;
        assert!(matches!(
            client.current_activity(),
            Some(Activity::Playing { world_name: Some(ref s), .. }) if s == "mc.hypixel.net"
        ));

        // Transition 4: InLobby
        client
            .set_activity(Activity::InLobby {
                lobby_code: Some("AURA12".to_string()),
                player_count: 3,
                host_name: Some("Alex".to_string()),
            })
            .await;
        assert!(matches!(
            client.current_activity(),
            Some(Activity::InLobby {
                player_count: 3,
                ..
            })
        ));

        // Transition 5: Back to InLauncher
        client.set_activity(Activity::InLauncher).await;
        assert_eq!(client.current_activity(), Some(&Activity::InLauncher));
    }

    #[tokio::test]
    async fn test_discord_disabling_and_clearing() {
        let mut client = DiscordRpcClient::connect_or_noop(DEFAULT_DISCORD_APP_ID).await;
        client.set_activity(Activity::InLauncher).await;
        assert!(client.is_enabled());

        // Disabling RPC
        client.set_enabled(false).await;
        assert!(!client.is_enabled());
        assert_eq!(client.current_activity(), None);

        let preview_disabled = client.preview();
        assert!(!preview_disabled.is_enabled);
        assert_eq!(preview_disabled.details, "Статус отключён в настройках");

        // Attempting to set activity while disabled does not broadcast
        client.set_activity(Activity::InLauncher).await;
        let preview_after = client.preview();
        assert!(!preview_after.is_enabled);

        // Re-enabling RPC restores functionality
        client.set_enabled(true).await;
        assert!(client.is_enabled());
        let preview_re_enabled = client.preview();
        assert!(preview_re_enabled.is_enabled);
        assert_eq!(preview_re_enabled.details, "В главном меню");
    }

    #[test]
    fn test_activity_previews() {
        // Disabled preview
        let p_off = get_activity_preview(None, false, false);
        assert!(!p_off.is_enabled);

        // InLauncher preview
        let p_launcher = get_activity_preview(Some(&Activity::InLauncher), true, false);
        assert!(p_launcher.is_enabled);
        assert_eq!(p_launcher.details, "В главном меню");

        // Installing preview
        let p_inst = get_activity_preview(
            Some(&Activity::Installing {
                version: "1.20.1".to_string(),
                stage: Some("Загрузка модов".to_string()),
            }),
            true,
            false,
        );
        assert_eq!(p_inst.details, "Установка Minecraft 1.20.1");
        assert_eq!(p_inst.state, "Этап: Загрузка модов");

        // Playing on server preview
        let p_server = get_activity_preview(
            Some(&Activity::Playing {
                version: "1.21.1".to_string(),
                world_name: Some("mc.auramc.net".to_string()),
                mod_count: None,
                start_timestamp: Some(100),
            }),
            true,
            false,
        );
        assert_eq!(p_server.details, "На сервере: mc.auramc.net");
        assert!(p_server.has_timestamp);

        // InLobby preview
        let p_lobby = get_activity_preview(
            Some(&Activity::InLobby {
                lobby_code: Some("XYZ".to_string()),
                player_count: 5,
                host_name: Some("HostUser".to_string()),
            }),
            true,
            false,
        );
        assert_eq!(p_lobby.details, "В лобби #XYZ");
        assert_eq!(p_lobby.state, "Игроков: 5/8 • Хост: HostUser");
    }
}
