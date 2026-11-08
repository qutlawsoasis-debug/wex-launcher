use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use wex_ipc::commands::*;
use wex_ipc::core_trait::LauncherCore;
use wex_ipc::error::{IpcError, IpcResult};
use wex_ipc::models::{InstalledVersion, LauncherConfig as IpcConfig};

use crate::config::{LauncherConfig as CoreConfig, WexPaths};
use crate::event::NoopEventHandler;
use crate::launcher::{install_full_environment, prepare_and_launch, GameProcess};

pub struct WexLauncherCore {
    paths: WexPaths,
    client: reqwest::Client,
    running_processes: Arc<Mutex<HashMap<String, GameProcess>>>,
    active_cancel_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl WexLauncherCore {
    pub fn new(paths: WexPaths) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("WexLauncher/0.1.0")
            .build()
            .unwrap_or_default();
        Self {
            paths,
            client,
            running_processes: Arc::new(Mutex::new(HashMap::new())),
            active_cancel_tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl LauncherCore for WexLauncherCore {
    async fn get_config(&self) -> IpcResult<IpcConfig> {
        let cfg = CoreConfig::load_or_default(&self.paths.config_file);
        Ok(cfg.into())
    }

    async fn save_config(&self, config: IpcConfig) -> IpcResult<()> {
        let core_cfg: CoreConfig = config.into();
        core_cfg
            .save(&self.paths.config_file)
            .map_err(|e| IpcError::internal("Failed to save config", Some(e.to_string())))
    }

    async fn get_installed_versions(&self) -> IpcResult<Vec<InstalledVersion>> {
        let v120_installed = self.paths.client_jar("1.20.1").exists();
        let v121_installed = self.paths.client_jar("1.21.1").exists();

        Ok(vec![
            InstalledVersion {
                id: "1.20.1".to_string(),
                name: "1.20.1 (Aura Client)".to_string(),
                is_installed: v120_installed,
            },
            InstalledVersion {
                id: "1.21.1".to_string(),
                name: "1.21.1 (Aura Client)".to_string(),
                is_installed: v121_installed,
            },
        ])
    }

    async fn install_instance(
        &self,
        request: InstallInstanceRequest,
    ) -> IpcResult<InstallInstanceResponse> {
        let events = NoopEventHandler;
        match install_full_environment(&self.paths, &request.version, &self.client, &events, 8).await {
            Ok(_) => Ok(InstallInstanceResponse {
                success: true,
                version: request.version,
                message: "Environment installed successfully".to_string(),
            }),
            Err(e) => Err(IpcError::internal(
                format!("Installation failed: {}", e),
                Some(e.to_string()),
            )),
        }
    }

    async fn launch_game(&self, request: LaunchGameRequest) -> IpcResult<LaunchGameResponse> {
        let mut cfg = CoreConfig::load_or_default(&self.paths.config_file);
        cfg.nickname = request.nickname;
        cfg.selected_version = request.version.clone();
        let _ = cfg.save(&self.paths.config_file);

        let events = Arc::new(NoopEventHandler);
        let jvm_args = request.jvm_args.unwrap_or_default();

        match prepare_and_launch(
            &self.paths,
            &cfg,
            &request.version,
            &self.client,
            events,
            8,
            &jvm_args,
        )
        .await
        {
            Ok(process) => {
                let pid = process.pid;
                let mut map = self.running_processes.lock().await;
                map.insert(request.version.clone(), process);

                Ok(LaunchGameResponse {
                    success: true,
                    pid: Some(pid),
                    message: "Game launched successfully".to_string(),
                })
            }
            Err(e) => Err(IpcError::internal(
                format!("Launch failed: {}", e),
                Some(e.to_string()),
            )),
        }
    }

    async fn cancel_launch(&self) -> IpcResult<()> {
        let tokens = self.active_cancel_tokens.lock().await;
        for token in tokens.values() {
            token.cancel();
        }
        Ok(())
    }

    async fn is_running(&self, instance_id: &str) -> IpcResult<bool> {
        let mut map = self.running_processes.lock().await;
        if let Some(process) = map.get_mut(instance_id) {
            match process.child.try_wait() {
                Ok(Some(_)) => {
                    map.remove(instance_id);
                    Ok(false)
                }
                Ok(None) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    async fn kill_game(&self, instance_id: &str) -> IpcResult<()> {
        let mut map = self.running_processes.lock().await;
        if let Some(mut process) = map.remove(instance_id) {
            let _ = process.kill();
        }
        Ok(())
    }
}
