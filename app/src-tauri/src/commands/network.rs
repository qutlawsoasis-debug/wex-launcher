use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::RwLock;

use wex_ipc::core_trait::LauncherCore;

pub const EVT_NETWORK_STATUS: &str = "network-status";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkStatusPayload {
    pub is_online: bool,
}

/// Shared network state and connectivity supervisor.
#[derive(Clone)]
pub struct NetworkState {
    pub is_online: Arc<AtomicBool>,
    pub override_online: Arc<RwLock<Option<bool>>>,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkState {
    pub fn new() -> Self {
        Self {
            is_online: Arc::new(AtomicBool::new(true)),
            override_online: Arc::new(RwLock::new(None)),
        }
    }

    pub fn new_with_status(initial_online: bool) -> Self {
        Self {
            is_online: Arc::new(AtomicBool::new(initial_online)),
            override_online: Arc::new(RwLock::new(None)),
        }
    }

    /// Check if connectivity is currently active (respecting test overrides).
    pub async fn is_online(&self) -> bool {
        if let Some(forced) = *self.override_online.read().await {
            return forced;
        }
        self.is_online.load(Ordering::SeqCst)
    }

    /// Set simulated connectivity override for unit/integration tests.
    pub async fn set_override(&self, forced: Option<bool>) {
        let mut lock = self.override_online.write().await;
        *lock = forced;
    }

    /// Start background supervisor to monitor connection status and auto-notify UI.
    pub fn start_supervisor(&self, app: AppHandle) {
        let is_online_clone = self.is_online.clone();
        let override_clone = self.override_online.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                // If an override is set (e.g. during test/mocking), do not probe real socket
                let forced = *override_clone.read().await;
                let active = match forced {
                    Some(val) => val,
                    None => check_connectivity().await,
                };

                let prev = is_online_clone.swap(active, Ordering::SeqCst);
                if prev != active {
                    let _ = app.emit(EVT_NETWORK_STATUS, NetworkStatusPayload { is_online: active });
                }
            }
        });
    }
}

/// Lightweight connectivity test: probes public DNS servers with timeout.
pub async fn check_connectivity() -> bool {
    let targets = ["1.1.1.1:53", "8.8.8.8:53"];
    for target in targets {
        if let Ok(Ok(_)) = tokio::time::timeout(
            std::time::Duration::from_millis(1500),
            tokio::net::TcpStream::connect(target),
        )
        .await
        {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Offline Launch Validation Decision
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflineLaunchDecision {
    Online,
    OfflineAllowed { status_message: String },
    OfflineForbidden { reason: String },
}

/// Validates whether a launch request is permissible under current network conditions.
///
/// Rules (SPEC & Task wex-offline):
/// 1. If online: normal launch proceeds.
/// 2. If offline and version is already installed: allowed without pack sync with status
///    «Нет сети — запуск без обновления».
/// 3. If offline and version is NOT installed: forbidden with clear user reason.
pub async fn check_offline_launch(
    core: &dyn LauncherCore,
    version: &str,
    is_online: bool,
) -> Result<OfflineLaunchDecision, String> {
    if is_online {
        return Ok(OfflineLaunchDecision::Online);
    }

    let installed_versions = core
        .get_installed_versions()
        .await
        .map_err(|e| e.to_string())?;

    let is_installed = installed_versions
        .iter()
        .any(|v| v.id == version && v.is_installed);

    if is_installed {
        Ok(OfflineLaunchDecision::OfflineAllowed {
            status_message: "Нет сети — запуск без обновления".to_string(),
        })
    } else {
        Ok(OfflineLaunchDecision::OfflineForbidden {
            reason: format!(
                "Версия {} не установлена. Требуется подключение к сети для загрузки.",
                version
            ),
        })
    }
}

// ---------------------------------------------------------------------------
// Tauri Command Handlers
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_network_status(
    state: State<'_, NetworkState>,
) -> Result<NetworkStatusPayload, String> {
    let is_online = state.is_online().await;
    Ok(NetworkStatusPayload { is_online })
}

// ---------------------------------------------------------------------------
// Unit Tests (Mock Core & Network Status Transitions)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wex_ipc::core_trait::MockLauncherCore;
    use wex_ipc::models::InstalledVersion;

    #[tokio::test]
    async fn test_network_state_override_and_status() {
        let net = NetworkState::new();
        assert!(net.is_online().await);

        // Simulate going offline
        net.set_override(Some(false)).await;
        assert!(!net.is_online().await);

        // Simulate returning online
        net.set_override(Some(true)).await;
        assert!(net.is_online().await);

        // Reset override
        net.set_override(None).await;
    }

    #[tokio::test]
    async fn test_offline_launch_installed_version_succeeds() {
        let mock_core = MockLauncherCore::new();
        mock_core.add_version(InstalledVersion {
            id: "1.21.1".to_string(),
            name: "1.21.1".to_string(),
            is_installed: true,
        });

        // Launching installed version offline
        let decision = check_offline_launch(&mock_core, "1.21.1", false)
            .await
            .unwrap();

        match decision {
            OfflineLaunchDecision::OfflineAllowed { status_message } => {
                assert_eq!(status_message, "Нет сети — запуск без обновления");
            }
            other => panic!("Expected OfflineAllowed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_offline_launch_uninstalled_version_fails() {
        let mock_core = MockLauncherCore::new();
        mock_core.add_version(InstalledVersion {
            id: "1.22.0".to_string(),
            name: "1.22.0".to_string(),
            is_installed: false, // NOT installed
        });

        // Launching uninstalled version offline
        let decision = check_offline_launch(&mock_core, "1.22.0", false)
            .await
            .unwrap();

        match decision {
            OfflineLaunchDecision::OfflineForbidden { reason } => {
                assert!(reason.contains("1.22.0"));
                assert!(reason.contains("не установлена"));
                assert!(reason.contains("подключение к сети"));
            }
            other => panic!("Expected OfflineForbidden, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_online_launch_decision() {
        let mock_core = MockLauncherCore::new();

        // When online, decision is always Online regardless of local files
        let decision = check_offline_launch(&mock_core, "1.21.1", true)
            .await
            .unwrap();

        assert_eq!(decision, OfflineLaunchDecision::Online);
    }
}
