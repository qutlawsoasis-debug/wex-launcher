use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use wex_ipc::commands::*;
use wex_ipc::core_trait::LauncherCore;
use wex_ipc::error::{IpcError, IpcResult};
use wex_ipc::models::{InstalledVersion, LauncherConfig as IpcConfig};

use crate::config::{LauncherConfig as CoreConfig, WexPaths};
use crate::event::{EventHandler, NoopEventHandler};
use crate::launcher::{install_full_environment, prepare_and_launch, GameProcess};

pub struct WexLauncherCore {
    paths: WexPaths,
    client: reqwest::Client,
    event_handler: Arc<dyn EventHandler>,
    running_processes: Arc<Mutex<HashMap<String, GameProcess>>>,
    active_cancel_tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl WexLauncherCore {
    pub fn new(paths: WexPaths) -> Self {
        Self::with_event_handler(paths, Arc::new(NoopEventHandler))
    }

    pub fn with_event_handler(paths: WexPaths, events: Arc<dyn EventHandler>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("WexLauncher/0.1.0")
            .build()
            .unwrap_or_default();
        Self {
            paths,
            client,
            event_handler: events,
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
        let mut core_cfg = CoreConfig::load_or_default(&self.paths.config_file);
        core_cfg.selected_version = config.selected_version;
        core_cfg.nickname = config.player_name;
        core_cfg.max_memory_mb = if config.ram_gb <= 64 {
            config.ram_gb * 1024
        } else {
            config.ram_gb
        };
        core_cfg.reduce_motion = config.reduce_motion;
        core_cfg.reduce_transparency = config.reduce_transparency;
        core_cfg.appearance = config.appearance;
        core_cfg.accent = config.accent;
        core_cfg.fast_start_jvm = config.fast_start_jvm;
        core_cfg.fast_launch = config.fast_launch;
        core_cfg.save(&self.paths.config_file).map_err(|e| {
            IpcError::internal(
                "Не удалось сохранить файл конфигурации. Проверьте свободное место на диске.",
                Some(e.to_string()),
            )
        })
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
        let events = self.event_handler.clone();
        match install_full_environment(
            &self.paths,
            &request.version,
            &self.client,
            events.as_ref(),
            8,
        )
        .await
        {
            Ok(_) => Ok(InstallInstanceResponse {
                success: true,
                version: request.version,
                message: "Файлы игры успешно установлены".to_string(),
            }),
            Err(e) => Err(e.into()),
        }
    }

    async fn launch_game(&self, request: LaunchGameRequest) -> IpcResult<LaunchGameResponse> {
        let mut cfg = CoreConfig::load_or_default(&self.paths.config_file);
        cfg.nickname = request.nickname;
        cfg.selected_version = request.version.clone();
        cfg.quick_play_multiplayer = request.quick_play_multiplayer;
        let _ = cfg.save(&self.paths.config_file);

        let events = self.event_handler.clone();
        let jvm_args = request.jvm_args.unwrap_or_default();

        let cancel_token = CancellationToken::new();
        {
            let mut tokens = self.active_cancel_tokens.lock().await;
            tokens.insert(request.version.clone(), cancel_token);
        }

        let res = match prepare_and_launch(
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
            Err(e) => Err(e.into()),
        };

        {
            let mut tokens = self.active_cancel_tokens.lock().await;
            tokens.remove(&request.version);
        }

        res
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ipc_launcher_core_config_and_versions() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        paths.ensure_dirs().unwrap();

        let core = WexLauncherCore::new(paths.clone());

        // Initial default config
        let cfg = core.get_config().await.unwrap();
        assert_eq!(cfg.player_name, "Player");

        // Save updated config
        let mut new_cfg = cfg.clone();
        new_cfg.player_name = "Alex".to_string();
        new_cfg.ram_gb = 6;
        core.save_config(new_cfg).await.unwrap();

        // Reload and verify
        let reloaded = core.get_config().await.unwrap();
        assert_eq!(reloaded.player_name, "Alex");
        assert_eq!(reloaded.ram_gb, 6);

        // Check installed versions (initially false)
        let versions = core.get_installed_versions().await.unwrap();
        assert_eq!(versions.len(), 2);
        assert!(!versions[0].is_installed);
        assert!(!versions[1].is_installed);

        // Create client.jar for 1.20.1 and recheck
        let v120_jar = paths.client_jar("1.20.1");
        fs::create_dir_all(v120_jar.parent().unwrap()).unwrap();
        fs::write(&v120_jar, b"PK").unwrap();

        let updated_versions = core.get_installed_versions().await.unwrap();
        let v120 = updated_versions.iter().find(|v| v.id == "1.20.1").unwrap();
        assert!(v120.is_installed);
    }
}
