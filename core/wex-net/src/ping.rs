use std::io;
use std::time::{Duration, Instant};

use hickory_resolver::TokioAsyncResolver;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Error, Debug)]
pub enum PingError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Connection timed out")]
    Timeout,

    #[error("Invalid protocol response: {0}")]
    Protocol(String),

    #[error("Failed to parse JSON response: {0}")]
    Json(#[from] serde_json::Error),

    #[error("DNS resolution error: {0}")]
    Dns(String),
}

/// Result of querying a Minecraft server via Server List Ping (SLP)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerPingResponse {
    pub host: String,
    pub port: u16,
    pub latency_ms: u64,
    pub version: ServerVersionInfo,
    pub players: ServerPlayersInfo,
    pub description: String,
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerVersionInfo {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerPlayersInfo {
    pub max: u32,
    pub online: u32,
    pub sample: Option<Vec<PlayerSample>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}

/// Raw payload received from server before normalization
#[derive(Deserialize, Debug)]
struct RawSlpResponse {
    version: Option<RawVersion>,
    players: Option<RawPlayers>,
    #[serde(deserialize_with = "deserialize_description")]
    description: Option<String>,
    favicon: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RawVersion {
    name: Option<String>,
    protocol: Option<i32>,
}

#[derive(Deserialize, Debug)]
struct RawPlayers {
    max: Option<u32>,
    online: Option<u32>,
    sample: Option<Vec<PlayerSample>>,
}

/// Helper to deserialize description whether it is a plain string or a Chat Component object
fn deserialize_description<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let val: Option<serde_json::Value> = Deserialize::deserialize(deserializer)?;
    match val {
        None => Ok(None),
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Object(map)) => {
            if let Some(serde_json::Value::String(text)) = map.get("text") {
                Ok(Some(text.clone()))
            } else {
                Ok(Some(serde_json::to_string(&map).unwrap_or_default()))
            }
        }
        Some(other) => Ok(Some(other.to_string())),
    }
}

// ---------------------------------------------------------------------------
// VarInt Utilities
// ---------------------------------------------------------------------------

pub async fn write_varint<W: AsyncWrite + Unpin>(writer: &mut W, mut value: i32) -> io::Result<()> {
    loop {
        if (value & !0x7F) == 0 {
            writer.write_u8(value as u8).await?;
            return Ok(());
        }
        writer.write_u8(((value & 0x7F) | 0x80) as u8).await?;
        value = ((value as u32) >> 7) as i32;
    }
}

pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<i32> {
    let mut value: i32 = 0;
    let mut position: u8 = 0;

    loop {
        let byte = reader.read_u8().await?;
        value |= ((byte & 0x7F) as i32) << position;

        if (byte & 0x80) == 0 {
            break;
        }

        position += 7;
        if position >= 35 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "VarInt is too big (> 5 bytes)",
            ));
        }
    }

    Ok(value)
}

// ---------------------------------------------------------------------------
// SRV Resolution
// ---------------------------------------------------------------------------

/// Resolves host and port, querying `_minecraft._tcp.<domain>` if no explicit port is given.
pub async fn resolve_minecraft_address(address: &str) -> (String, u16) {
    let address = address.trim();

    // Check for explicit port
    if let Some(colon_pos) = address.rfind(':') {
        let host = &address[..colon_pos];
        let port_str = &address[colon_pos + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            let clean_host = host.trim_matches(|c| c == '[' || c == ']');
            return (clean_host.to_string(), port);
        }
    }

    // Try resolving SRV record: _minecraft._tcp.<address>
    let srv_domain = format!("_minecraft._tcp.{}.", address.trim_end_matches('.'));
    if let Ok(resolver) = TokioAsyncResolver::tokio_from_system_conf() {
        if let Ok(response) = resolver.srv_lookup(&srv_domain).await {
            if let Some(record) = response.iter().next() {
                let target = record.target().to_string();
                let clean_target = target.trim_end_matches('.').to_string();
                let port = record.port();
                return (clean_target, port);
            }
        }
    }

    (address.to_string(), 25565)
}

// ---------------------------------------------------------------------------
// Server List Ping implementation
// ---------------------------------------------------------------------------

pub struct ServerPinger {
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub protocol_version: i32,
}

impl Default for ServerPinger {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_millis(3000),
            read_timeout: Duration::from_millis(3000),
            protocol_version: 763, // 1.20.1 default
        }
    }
}

impl ServerPinger {
    pub fn new(connect_timeout_ms: u64, read_timeout_ms: u64, protocol_version: i32) -> Self {
        Self {
            connect_timeout: Duration::from_millis(connect_timeout_ms),
            read_timeout: Duration::from_millis(read_timeout_ms),
            protocol_version,
        }
    }

    /// Pings a Minecraft server at the given address (with automatic SRV resolution)
    pub async fn ping(&self, raw_address: &str) -> Result<ServerPingResponse, PingError> {
        let (resolved_host, resolved_port) = resolve_minecraft_address(raw_address).await;
        self.ping_resolved(&resolved_host, resolved_port).await
    }

    /// Pings an already resolved (host, port) pair
    pub async fn ping_resolved(
        &self,
        host: &str,
        port: u16,
    ) -> Result<ServerPingResponse, PingError> {
        let start = Instant::now();

        let mut stream = timeout(self.connect_timeout, TcpStream::connect((host, port)))
            .await
            .map_err(|_| PingError::Timeout)?
            .map_err(PingError::Io)?;

        // Perform handshake and read status
        let (raw_response, latency_ms) = timeout(
            self.read_timeout,
            execute_slp(&mut stream, host, port, self.protocol_version, start),
        )
        .await
        .map_err(|_| PingError::Timeout)??;

        let parsed: RawSlpResponse = serde_json::from_str(&raw_response)?;

        let version = ServerVersionInfo {
            name: parsed
                .version
                .as_ref()
                .and_then(|v| v.name.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            protocol: parsed.version.and_then(|v| v.protocol).unwrap_or(0),
        };

        let players = ServerPlayersInfo {
            max: parsed.players.as_ref().and_then(|p| p.max).unwrap_or(0),
            online: parsed.players.as_ref().and_then(|p| p.online).unwrap_or(0),
            sample: parsed.players.and_then(|p| p.sample),
        };

        Ok(ServerPingResponse {
            host: host.to_string(),
            port,
            latency_ms,
            version,
            players,
            description: parsed.description.unwrap_or_default(),
            favicon: parsed.favicon,
        })
    }
}

async fn execute_slp<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    host: &str,
    port: u16,
    protocol_version: i32,
    start: Instant,
) -> Result<(String, u64), PingError> {
    // 1. Send Handshake Packet (ID 0x00)
    let mut handshake_payload = Vec::with_capacity(host.len() + 16);
    write_varint(&mut handshake_payload, 0x00).await?; // Packet ID
    write_varint(&mut handshake_payload, protocol_version).await?;
    write_varint(&mut handshake_payload, host.len() as i32).await?;
    handshake_payload.extend_from_slice(host.as_bytes());
    handshake_payload.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake_payload, 1).await?; // NextState = 1 (Status)

    write_varint(stream, handshake_payload.len() as i32).await?;
    stream.write_all(&handshake_payload).await?;

    // 2. Send Status Request Packet (Length 1, ID 0x00)
    let mut status_request = Vec::with_capacity(2);
    write_varint(&mut status_request, 0x00).await?; // Packet ID 0x00
    write_varint(stream, status_request.len() as i32).await?;
    stream.write_all(&status_request).await?;
    stream.flush().await?;

    // 3. Read Status Response
    let packet_len = read_varint(stream).await?;
    if packet_len <= 0 || packet_len > 1_048_576 {
        return Err(PingError::Protocol(format!(
            "Invalid response packet length: {packet_len}"
        )));
    }

    let packet_id = read_varint(stream).await?;
    if packet_id != 0x00 {
        return Err(PingError::Protocol(format!(
            "Expected packet ID 0x00, got 0x{packet_id:02X}"
        )));
    }

    let json_len = read_varint(stream).await?;
    if json_len <= 0 || json_len > 1_048_576 {
        return Err(PingError::Protocol(format!(
            "Invalid JSON payload length: {json_len}"
        )));
    }

    let mut json_bytes = vec![0u8; json_len as usize];
    stream.read_exact(&mut json_bytes).await?;

    let latency_ms = start.elapsed().as_millis() as u64;
    let json_str = String::from_utf8(json_bytes)
        .map_err(|e| PingError::Protocol(format!("Invalid UTF-8 in response: {e}")))?;

    Ok((json_str, latency_ms))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::duplex;

    #[tokio::test]
    async fn test_varint_roundtrip() {
        let test_cases = vec![0, 1, 127, 128, 255, 25565, 2097151, -1, i32::MAX, i32::MIN];
        for val in test_cases {
            let mut buf = Vec::new();
            write_varint(&mut buf, val).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let read_val = read_varint(&mut cursor).await.unwrap();
            assert_eq!(val, read_val);
        }
    }

    #[tokio::test]
    async fn test_slp_mock_stream() {
        let (mut client, mut server) = duplex(4096);

        let server_task = tokio::spawn(async move {
            // Read Handshake
            let _hs_len = read_varint(&mut server).await.unwrap();
            let hs_id = read_varint(&mut server).await.unwrap();
            assert_eq!(hs_id, 0);
            let _proto = read_varint(&mut server).await.unwrap();
            let host_len = read_varint(&mut server).await.unwrap();
            let mut host_bytes = vec![0u8; host_len as usize];
            server.read_exact(&mut host_bytes).await.unwrap();
            let mut port_bytes = [0u8; 2];
            server.read_exact(&mut port_bytes).await.unwrap();
            let next_state = read_varint(&mut server).await.unwrap();
            assert_eq!(next_state, 1);

            // Read Status Request
            let _req_len = read_varint(&mut server).await.unwrap();
            let req_id = read_varint(&mut server).await.unwrap();
            assert_eq!(req_id, 0);

            // Send Response
            let sample_json = r#"{"version":{"name":"1.20.1","protocol":763},"players":{"max":100,"online":5},"description":"Welcome to Aura Server"}"#;
            let mut body = Vec::new();
            write_varint(&mut body, 0x00).await.unwrap();
            write_varint(&mut body, sample_json.len() as i32)
                .await
                .unwrap();
            body.extend_from_slice(sample_json.as_bytes());

            write_varint(&mut server, body.len() as i32).await.unwrap();
            server.write_all(&body).await.unwrap();
            server.flush().await.unwrap();
        });

        let start = Instant::now();
        let (json_res, latency) = execute_slp(&mut client, "localhost", 25565, 763, start)
            .await
            .unwrap();

        server_task.await.unwrap();

        assert!(json_res.contains("Welcome to Aura Server"));
        assert!(latency < 1000);
    }

    #[tokio::test]
    async fn test_deserialize_chat_component_description() {
        let json = r#"{
            "version": { "name": "1.20.1", "protocol": 763 },
            "players": { "max": 20, "online": 3 },
            "description": { "text": "Formatted MOTD Text" }
        }"#;

        let parsed: RawSlpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.description.unwrap(), "Formatted MOTD Text");
    }

    #[tokio::test]
    async fn test_resolve_address_with_explicit_port() {
        let (host, port) = resolve_minecraft_address("play.example.com:25577").await;
        assert_eq!(host, "play.example.com");
        assert_eq!(port, 25577);
    }

    #[tokio::test]
    #[ignore]
    async fn test_live_ping_hypixel() {
        let pinger = ServerPinger::new(5000, 5000, 763);
        let result = pinger.ping("mc.hypixel.net").await;
        match result {
            Ok(resp) => {
                println!("=== Hypixel Ping Success ===");
                println!("Host: {}:{}", resp.host, resp.port);
                println!("Latency: {} ms", resp.latency_ms);
                println!(
                    "Version: {} (protocol {})",
                    resp.version.name, resp.version.protocol
                );
                println!("Players: {}/{}", resp.players.online, resp.players.max);
                println!("MOTD: {}", resp.description);
                assert!(resp.players.max > 0);
            }
            Err(e) => {
                panic!("Failed to ping mc.hypixel.net: {e}");
            }
        }
    }
}
