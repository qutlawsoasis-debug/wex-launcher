use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Network events emitted across wex-net modules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetEvent {
    /// Server ping check completed
    PingCompleted {
        server: String,
        latency_ms: u64,
        online: u32,
        max: u32,
        version: String,
    },
    /// Playit tunnel status changed
    TunnelStatusChanged { status: TunnelStatus },
    /// File download progress (for binary/asset downloads)
    DownloadProgress {
        file: String,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    /// Heartbeat sent to Lobby API
    LobbyHeartbeatSent { code: String },
    /// Heartbeat failed to send
    LobbyHeartbeatFailed { code: String, error: String },
}

/// Status of the Playit.gg tunnel process
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TunnelStatus {
    Inactive,
    Downloading { progress: f32 },
    Starting,
    Running { address: String, port: u16 },
    Stopped,
    Error(String),
}

/// Event bus broadcaster for wex-net
#[derive(Clone, Debug)]
pub struct NetEventBus {
    sender: broadcast::Sender<NetEvent>,
}

impl NetEventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<NetEvent> {
        self.sender.subscribe()
    }

    pub fn emit(&self, event: NetEvent) {
        // Ignore error if there are no active subscribers
        let _ = self.sender.send(event);
    }
}

impl Default for NetEventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus() {
        let bus = NetEventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(NetEvent::TunnelStatusChanged {
            status: TunnelStatus::Starting,
        });

        let received = rx.recv().await.unwrap();
        assert_eq!(
            received,
            NetEvent::TunnelStatusChanged {
                status: TunnelStatus::Starting,
            }
        );
    }
}
