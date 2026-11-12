use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::commands::{
    InstallInstanceRequest, InstallInstanceResponse, LaunchGameRequest, LaunchGameResponse,
};
use crate::error::{IpcError, IpcResult};
use crate::models::{InstalledVersion, LauncherConfig};

/// Abstract asynchronous contract defining the core launcher capabilities.
///
/// Implemented by `wex-core` (authored by agent a1) and exposed through Tauri IPC handlers.
/// A mock implementation (`MockLauncherCore`) is provided for testing, UI development, and stubbing.
#[async_trait]
pub trait LauncherCore: Send + Sync {
    /// Retrieve current launcher settings and user configuration.
    async fn get_config(&self) -> IpcResult<LauncherConfig>;

    /// Persist updated launcher settings.
    async fn save_config(&self, config: LauncherConfig) -> IpcResult<()>;

    /// Retrieve the list of installed or available Minecraft client versions.
    async fn get_installed_versions(&self) -> IpcResult<Vec<InstalledVersion>>;

    /// Verify, download, and install files for a game instance.
    async fn install_instance(
        &self,
        request: InstallInstanceRequest,
    ) -> IpcResult<InstallInstanceResponse>;

    /// Launch Minecraft client with specified parameters.
    async fn launch_game(&self, request: LaunchGameRequest) -> IpcResult<LaunchGameResponse>;

    /// Request cancellation of an ongoing game launch sequence.
    async fn cancel_launch(&self) -> IpcResult<()>;

    /// Check if a specific instance or client process is currently running.
    async fn is_running(&self, instance_id: &str) -> IpcResult<bool>;

    /// Forcefully terminate a running game instance.
    async fn kill_game(&self, instance_id: &str) -> IpcResult<()>;

    /// Exit code of the instance's last process, once `is_running` has seen it end.
    /// `None` when unknown (still running, killed without a code, or not tracked).
    async fn last_exit_code(&self, _instance_id: &str) -> Option<i32> {
        None
    }
}

/// In-memory mock implementation of `LauncherCore` for testing and UI prototyping.
pub struct MockLauncherCore {
    config: Mutex<LauncherConfig>,
    versions: Mutex<Vec<InstalledVersion>>,
    is_launching: Mutex<bool>,
    running_instances: Mutex<HashMap<String, u32>>,
    pub fail_next_launch: Mutex<bool>,
}

impl Default for MockLauncherCore {
    fn default() -> Self {
        Self::new()
    }
}

impl MockLauncherCore {
    /// Create a new mock instance initialized with default configuration and versions.
    pub fn new() -> Self {
        let default_versions = vec![
            InstalledVersion {
                id: "1.20.1".to_string(),
                name: "1.20.1 (Aura Client)".to_string(),
                is_installed: true,
            },
            InstalledVersion {
                id: "1.21.1".to_string(),
                name: "1.21.1 (Aura Client)".to_string(),
                is_installed: true,
            },
        ];

        Self {
            config: Mutex::new(LauncherConfig::default()),
            versions: Mutex::new(default_versions),
            is_launching: Mutex::new(false),
            running_instances: Mutex::new(HashMap::new()),
            fail_next_launch: Mutex::new(false),
        }
    }

    /// Add or update an installed version in the mock.
    pub fn add_version(&self, version: InstalledVersion) {
        if let Ok(mut v) = self.versions.lock() {
            v.retain(|item| item.id != version.id);
            v.push(version);
        }
    }

    /// Set simulated running instance pid.
    pub fn set_running(&self, instance_id: impl Into<String>, pid: u32) {
        if let Ok(mut map) = self.running_instances.lock() {
            map.insert(instance_id.into(), pid);
        }
    }
}

#[async_trait]
impl LauncherCore for MockLauncherCore {
    async fn get_config(&self) -> IpcResult<LauncherConfig> {
        self.config.lock().map(|c| c.clone()).map_err(|e| {
            IpcError::internal(
                "Не удалось заблокировать мьютекс конфигурации",
                Some(e.to_string()),
            )
        })
    }

    async fn save_config(&self, config: LauncherConfig) -> IpcResult<()> {
        let mut guard = self.config.lock().map_err(|e| {
            IpcError::internal(
                "Не удалось заблокировать мьютекс конфигурации",
                Some(e.to_string()),
            )
        })?;
        *guard = config;
        Ok(())
    }

    async fn get_installed_versions(&self) -> IpcResult<Vec<InstalledVersion>> {
        self.versions.lock().map(|v| v.clone()).map_err(|e| {
            IpcError::internal("Не удалось прочитать список версий", Some(e.to_string()))
        })
    }

    async fn install_instance(
        &self,
        request: InstallInstanceRequest,
    ) -> IpcResult<InstallInstanceResponse> {
        let mut versions = self
            .versions
            .lock()
            .map_err(|e| IpcError::internal("Ошибка доступа к версиям", Some(e.to_string())))?;

        if let Some(existing) = versions.iter_mut().find(|v| v.id == request.version) {
            existing.is_installed = true;
        } else {
            versions.push(InstalledVersion {
                id: request.version.clone(),
                name: format!("{} (Установлена)", request.version),
                is_installed: true,
            });
        }

        Ok(InstallInstanceResponse {
            success: true,
            version: request.version,
            message: "Инстанс успешно установлен".to_string(),
        })
    }

    async fn launch_game(&self, request: LaunchGameRequest) -> IpcResult<LaunchGameResponse> {
        let should_fail = *self
            .fail_next_launch
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if should_fail {
            return Err(IpcError::new(
                "LAUNCH_SIMULATED_FAILURE",
                "Симулированный сбой при запуске Minecraft клиента",
                Some("MockLauncherCore configured to fail next launch".to_string()),
            ));
        }

        let mut launching = self
            .is_launching
            .lock()
            .map_err(|e| IpcError::internal("Ошибка состояния запуска", Some(e.to_string())))?;

        if *launching {
            return Err(IpcError::invalid_argument("Запуск игры уже выполняется"));
        }
        *launching = true;

        let pid = 12345;
        if let Ok(mut map) = self.running_instances.lock() {
            map.insert(request.version.clone(), pid);
        }

        *launching = false;
        Ok(LaunchGameResponse {
            success: true,
            pid: Some(pid),
            message: format!(
                "Minecraft {} успешно запущен для игрока {}",
                request.version, request.nickname
            ),
        })
    }

    async fn cancel_launch(&self) -> IpcResult<()> {
        let mut launching = self
            .is_launching
            .lock()
            .map_err(|e| IpcError::internal("Ошибка состояния запуска", Some(e.to_string())))?;
        *launching = false;
        Ok(())
    }

    async fn is_running(&self, instance_id: &str) -> IpcResult<bool> {
        let map = self.running_instances.lock().map_err(|e| {
            IpcError::internal("Ошибка чтения запущенных процессов", Some(e.to_string()))
        })?;
        Ok(map.contains_key(instance_id))
    }

    async fn kill_game(&self, instance_id: &str) -> IpcResult<()> {
        let mut map = self.running_instances.lock().map_err(|e| {
            IpcError::internal("Ошибка доступа к запущенным процессам", Some(e.to_string()))
        })?;
        map.remove(instance_id);
        Ok(())
    }
}
