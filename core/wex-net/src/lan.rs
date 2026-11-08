//! Minecraft LAN World Discovery (SPEC 1.4).
//!
//! Standard Minecraft LAN broadcast protocol:
//! - Multicast address: `224.0.2.60:4445` (UDP)
//! - Payload format: `[MOTD]<world_name_or_motd>[/MOTD][AD]<port>[/AD]`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub const MINECRAFT_LAN_MULTICAST_ADDR: &str = "224.0.2.60";
pub const MINECRAFT_LAN_PORT: u16 = 4445;

/// Parsed Minecraft LAN announcement datagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanWorldAnnouncement {
    pub motd: String,
    pub port: u16,
}

/// A discovered Minecraft LAN world in the local network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredLanWorld {
    pub motd: String,
    pub clean_motd: String,
    pub address: String,
    pub host_ip: String,
    pub port: u16,
    pub last_seen_epoch_ms: u64,
}

/// Strip Minecraft color and formatting codes (`§a`, `§l`, `§r`, etc.).
pub fn strip_minecraft_formatting(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            // Skip formatting code character if present
            chars.next();
        } else {
            result.push(c);
        }
    }
    result.trim().to_string()
}

/// Parse a raw Minecraft LAN announcement packet payload.
/// Expected format: `[MOTD]<motd>[/MOTD][AD]<port>[/AD]`
pub fn parse_lan_announcement(packet: &str) -> Option<LanWorldAnnouncement> {
    let motd_start = packet.find("[MOTD]")? + 6;
    let motd_end = packet.find("[/MOTD]")?;
    if motd_end < motd_start {
        return None;
    }
    let motd = packet[motd_start..motd_end].to_string();

    let ad_start = packet.find("[AD]")? + 4;
    let ad_end = packet.find("[/AD]")?;
    if ad_end < ad_start {
        return None;
    }
    let port_str = packet[ad_start..ad_end].trim();
    let port = port_str.parse::<u16>().ok()?;
    if port == 0 {
        return None;
    }

    Some(LanWorldAnnouncement { motd, port })
}

/// Format a Minecraft LAN announcement packet payload.
pub fn format_lan_announcement(motd: &str, port: u16) -> String {
    format!("[MOTD]{}[/MOTD][AD]{}[/AD]", motd, port)
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Thread-safe in-memory cache and supervisor for discovered LAN worlds.
#[derive(Clone)]
pub struct LanDiscoveryService {
    cache: Arc<Mutex<HashMap<String, DiscoveredLanWorld>>>,
    ttl: Duration,
}

impl Default for LanDiscoveryService {
    fn default() -> Self {
        Self::new(Duration::from_secs(6))
    }
}

impl LanDiscoveryService {
    pub fn new(ttl: Duration) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    /// Record a discovered announcement from a specific sender IP.
    pub async fn record_announcement(
        &self,
        packet: &str,
        sender_ip: &str,
    ) -> Option<DiscoveredLanWorld> {
        let announcement = parse_lan_announcement(packet)?;
        let clean_motd = strip_minecraft_formatting(&announcement.motd);
        let address = format!("{}:{}", sender_ip, announcement.port);

        let world = DiscoveredLanWorld {
            motd: announcement.motd,
            clean_motd,
            address: address.clone(),
            host_ip: sender_ip.to_string(),
            port: announcement.port,
            last_seen_epoch_ms: current_time_ms(),
        };

        let mut lock = self.cache.lock().await;
        lock.insert(address, world.clone());
        Some(world)
    }

    /// Retrieve all currently active discovered LAN worlds, removing stale entries.
    pub async fn get_discovered_worlds(&self) -> Vec<DiscoveredLanWorld> {
        let now = current_time_ms();
        let ttl_ms = self.ttl.as_millis() as u64;

        let mut lock = self.cache.lock().await;
        lock.retain(|_, world| now.saturating_sub(world.last_seen_epoch_ms) <= ttl_ms);

        let mut list: Vec<DiscoveredLanWorld> = lock.values().cloned().collect();
        list.sort_by_key(|a| std::cmp::Reverse(a.last_seen_epoch_ms));
        list
    }

    /// Clear all cached discovered worlds.
    pub async fn clear(&self) {
        let mut lock = self.cache.lock().await;
        lock.clear();
    }

    /// Start background UDP multicast listener on `224.0.2.60:4445`.
    /// Safe to call both inside and outside of an active Tokio runtime.
    pub fn spawn_listener(self: Arc<Self>) {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                self.listen_loop().await;
            });
        } else {
            std::thread::Builder::new()
                .name("wex-lan-discovery".to_string())
                .spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        rt.block_on(async move {
                            self.listen_loop().await;
                        });
                    }
                })
                .ok();
        }
    }

    /// Background UDP multicast receiver loop.
    pub async fn listen_loop(self: Arc<Self>) {
        let multicast_ip: Ipv4Addr = match MINECRAFT_LAN_MULTICAST_ADDR.parse() {
            Ok(ip) => ip,
            Err(e) => {
                warn!("Failed to parse LAN multicast IP: {}", e);
                return;
            }
        };

        let bind_addr = SocketAddr::from(([0, 0, 0, 0], MINECRAFT_LAN_PORT));

        // Create UDP socket
        let std_socket = match std::net::UdpSocket::bind(bind_addr) {
            Ok(s) => s,
            Err(e) => {
                debug!("LAN discovery bind on 0.0.0.0:{} failed (may be already bound or blocked): {}", MINECRAFT_LAN_PORT, e);
                return;
            }
        };

        if let Err(e) = std_socket.join_multicast_v4(&multicast_ip, &Ipv4Addr::UNSPECIFIED) {
            debug!("LAN discovery join_multicast_v4 failed: {}", e);
            return;
        }

        let _ = std_socket.set_nonblocking(true);

        let socket = match tokio::net::UdpSocket::from_std(std_socket) {
            Ok(s) => s,
            Err(e) => {
                debug!("Failed to convert std UdpSocket to tokio: {}", e);
                return;
            }
        };

        debug!(
            "Minecraft LAN discovery listener active on 224.0.2.60:{}",
            MINECRAFT_LAN_PORT
        );

        let mut buf = [0u8; 2048];
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, peer)) => {
                    if let Ok(text) = std::str::from_utf8(&buf[..len]) {
                        let peer_ip = peer.ip().to_string();
                        self.record_announcement(text, &peer_ip).await;
                    }
                }
                Err(e) => {
                    debug!("LAN discovery recv error: {}", e);
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_announcement() {
        let packet = "[MOTD]Steve - Survival[/MOTD][AD]54321[/AD]";
        let res = parse_lan_announcement(packet).expect("should parse valid announcement");
        assert_eq!(res.motd, "Steve - Survival");
        assert_eq!(res.port, 54321);
    }

    #[test]
    fn test_parse_announcement_with_minecraft_formatting() {
        let packet = "[MOTD]§eAlex's §aCreative §lWorld§r[/MOTD][AD]25565[/AD]";
        let res = parse_lan_announcement(packet).expect("should parse formatted announcement");
        assert_eq!(res.motd, "§eAlex's §aCreative §lWorld§r");
        assert_eq!(res.port, 25565);

        let clean = strip_minecraft_formatting(&res.motd);
        assert_eq!(clean, "Alex's Creative World");
    }

    #[test]
    fn test_parse_malformed_announcements() {
        // Missing MOTD end tag
        assert!(parse_lan_announcement("[MOTD]Steve[AD]54321[/AD]").is_none());
        // Missing AD end tag
        assert!(parse_lan_announcement("[MOTD]Steve[/MOTD][AD]54321").is_none());
        // Non-numeric port
        assert!(parse_lan_announcement("[MOTD]Steve[/MOTD][AD]abc[/AD]").is_none());
        // Port 0
        assert!(parse_lan_announcement("[MOTD]Steve[/MOTD][AD]0[/AD]").is_none());
        // Empty packet
        assert!(parse_lan_announcement("").is_none());
        // Inverted tags
        assert!(parse_lan_announcement("[/MOTD]Test[MOTD][AD]54321[/AD]").is_none());
    }

    #[test]
    fn test_format_and_parse_roundtrip() {
        let original_motd = "Server of Legends";
        let original_port = 49152;
        let formatted = format_lan_announcement(original_motd, original_port);
        assert_eq!(formatted, "[MOTD]Server of Legends[/MOTD][AD]49152[/AD]");

        let parsed = parse_lan_announcement(&formatted).expect("roundtrip parse should succeed");
        assert_eq!(parsed.motd, original_motd);
        assert_eq!(parsed.port, original_port);
    }

    #[tokio::test]
    async fn test_discovery_service_record_and_expiration() {
        let service = LanDiscoveryService::new(Duration::from_millis(100));

        let packet = "[MOTD]§6Awesome §bWorld[/MOTD][AD]55555[/AD]";
        let recorded = service
            .record_announcement(packet, "192.168.1.42")
            .await
            .expect("record");

        assert_eq!(recorded.address, "192.168.1.42:55555");
        assert_eq!(recorded.clean_motd, "Awesome World");
        assert_eq!(recorded.port, 55555);

        let worlds = service.get_discovered_worlds().await;
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].address, "192.168.1.42:55555");
        assert_eq!(worlds[0].clean_motd, "Awesome World");

        // Wait for TTL expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        let expired_worlds = service.get_discovered_worlds().await;
        assert_eq!(expired_worlds.len(), 0);
    }
}
