use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use wex_core::config::WexPaths;
use wex_core::event::{Event, EventHandler, LogLevel};
use wex_core::WexLauncherCore;
use wex_extras::crash::{analyze_crash, CrashKind};
use wex_extras::discord::{Activity, DiscordRpcClient, DEFAULT_DISCORD_APP_ID};
use wex_ipc::commands::{LaunchGameRequest, LaunchGameResponse};
use wex_ipc::core_trait::LauncherCore;
use wex_ipc::events::{
    GameLogPayload, InstallProgressPayload, LaunchStatusPayload, EVT_GAME_LOG,
    EVT_INSTALL_PROGRESS, EVT_LAUNCH_STATUS,
};
use wex_ipc::models::{InstalledVersion, LauncherConfig};

/// Bridges events from wex-core into Tauri 2 window events.
#[derive(Clone)]
pub struct TauriEventHandler {
    pub app: AppHandle,
}

impl EventHandler for TauriEventHandler {
    fn on_event(&self, event: &Event) {
        match event {
            Event::StageStarted {
                stage,
                step,
                total_steps,
            } => {
                let pct = if *total_steps > 0 {
                    ((*step as f32 - 1.0) / *total_steps as f32) * 100.0
                } else {
                    0.0
                };
                let _ = self.app.emit(
                    EVT_INSTALL_PROGRESS,
                    InstallProgressPayload {
                        stage: stage.clone(),
                        percentage: pct,
                        detail: format!("Этап {} из {}: {}", step, total_steps, stage),
                        speed_bytes_per_sec: 0,
                        total_bytes: 0,
                        downloaded_bytes: 0,
                    },
                );
                let _ = self.app.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: "installing".to_string(),
                        message: format!("Установка: {}", stage),
                        exit_code: None,
                    },
                );
            }
            Event::Progress {
                stage,
                current,
                total,
                message,
            } => {
                let pct = if *total > 0 {
                    (*current as f32 / *total as f32) * 100.0
                } else {
                    0.0
                };
                let detail = message
                    .clone()
                    .unwrap_or_else(|| format!("{}/{}", current, total));
                let _ = self.app.emit(
                    EVT_INSTALL_PROGRESS,
                    InstallProgressPayload {
                        stage: stage.clone(),
                        percentage: pct,
                        detail,
                        speed_bytes_per_sec: 0,
                        total_bytes: *total,
                        downloaded_bytes: *current,
                    },
                );
            }
            Event::GameLaunched { pid } => {
                let _ = self.app.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: "running".to_string(),
                        message: format!("Minecraft запущен (PID {})", pid),
                        exit_code: None,
                    },
                );
            }
            Event::GameOutput { line } => {
                let _ = self.app.emit(
                    EVT_GAME_LOG,
                    GameLogPayload {
                        line: line.clone(),
                        level: "INFO".to_string(),
                    },
                );
            }
            Event::Log { level, message } => {
                let lvl = match level {
                    LogLevel::Debug => "DEBUG",
                    LogLevel::Info => "INFO",
                    LogLevel::Warn => "WARN",
                    LogLevel::Error => "ERROR",
                };
                let _ = self.app.emit(
                    EVT_GAME_LOG,
                    GameLogPayload {
                        line: message.clone(),
                        level: lvl.to_string(),
                    },
                );
            }
            Event::GameExited { exit_code } => {
                let _ = self.app.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: if *exit_code == Some(0) {
                            "idle".to_string()
                        } else {
                            "error".to_string()
                        },
                        message: if let Some(code) = exit_code {
                            format!("Процесс игры завершился с кодом {}", code)
                        } else {
                            "Процесс игры завершился".to_string()
                        },
                        exit_code: *exit_code,
                    },
                );
            }
            _ => {}
        }
    }
}

/// Helper to set Discord RPC activity safely without blocking.
pub async fn set_discord_rpc(discord: &Arc<Mutex<Option<DiscordRpcClient>>>, activity: Activity) {
    let mut lock = discord.lock().await;
    if let Some(client) = lock.as_mut() {
        client.set_activity(activity).await;
    }
}

/// Shared runtime state for the Play lifecycle.
pub struct PlayState {
    pub core: Arc<dyn LauncherCore>,
    pub paths: WexPaths,
    pub is_launching: Arc<AtomicBool>,
    pub is_running: Arc<AtomicBool>,
    pub running_version: Arc<Mutex<Option<String>>>,
    pub discord: Arc<Mutex<Option<DiscordRpcClient>>>,
}

impl PlayState {
    pub fn new_real(paths: WexPaths, app: AppHandle) -> Self {
        let event_handler = Arc::new(TauriEventHandler { app });
        let core = Arc::new(WexLauncherCore::with_event_handler(
            paths.clone(),
            event_handler,
        ));
        let discord = Arc::new(Mutex::new(None));

        // Connect Discord RPC asynchronously in background
        let discord_clone = discord.clone();
        tokio::spawn(async move {
            let client = DiscordRpcClient::connect_or_noop(DEFAULT_DISCORD_APP_ID).await;
            let mut lock = discord_clone.lock().await;
            *lock = Some(client);
        });

        Self {
            core,
            paths,
            is_launching: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            running_version: Arc::new(Mutex::new(None)),
            discord,
        }
    }

    pub fn new_with_core(paths: WexPaths, core: Arc<dyn LauncherCore>) -> Self {
        let discord = Arc::new(Mutex::new(None));
        Self {
            core,
            paths,
            is_launching: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            running_version: Arc::new(Mutex::new(None)),
            discord,
        }
    }
}

// ============================================================================
// Tauri Commands: Launcher Core & Play
// ============================================================================

/// Retrieve launcher settings.
#[tauri::command]
pub async fn get_config(state: State<'_, PlayState>) -> Result<LauncherConfig, String> {
    state.core.get_config().await.map_err(|e| e.to_string())
}

/// Persist updated launcher settings.
#[tauri::command]
pub async fn save_config(
    config: LauncherConfig,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    state
        .core
        .save_config(config)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve installed versions for selectors.
#[tauri::command]
pub async fn get_installed_versions(
    state: State<'_, PlayState>,
) -> Result<Vec<InstalledVersion>, String> {
    state
        .core
        .get_installed_versions()
        .await
        .map_err(|e| e.to_string())
}

/// Start game with specified version and nickname. Blocks duplicate start if already running.
#[tauri::command]
pub async fn start_game(
    app: AppHandle,
    version: String,
    nickname: String,
    jvm_args: Option<Vec<String>>,
    state: State<'_, PlayState>,
) -> Result<LaunchGameResponse, String> {
    // 1. Guard against duplicate starts
    if state.is_launching.swap(true, Ordering::SeqCst) || state.is_running.load(Ordering::SeqCst) {
        return Err("Игра уже запущена или выполняется запуск".to_string());
    }

    // 2. Set Discord RPC activity
    set_discord_rpc(
        &state.discord,
        Activity::Playing {
            version: version.clone(),
            world_name: None,
            mod_count: None,
            start_timestamp: Some(chrono::Utc::now().timestamp()),
        },
    )
    .await;

    // 3. Emit preparing launch status
    let _ = app.emit(
        EVT_LAUNCH_STATUS,
        LaunchStatusPayload {
            state: "preparing".to_string(),
            message: format!("Подготовка к запуску Minecraft {}...", version),
            exit_code: None,
        },
    );

    // 4. Launch game via LauncherCore
    let mut actual_jvm_args = jvm_args.unwrap_or_default();
    if std::env::var("WEX_QA_BACKGROUND").is_ok()
        || std::env::var("WEX_QA_BACKGROUND")
            .map(|v| v == "1" || v == "true")
            .unwrap_or(false)
    {
        if !actual_jvm_args
            .iter()
            .any(|a| a.contains("wex.qa.background"))
        {
            actual_jvm_args.push("-Dwex.qa.background=true".to_string());
        }
        if !actual_jvm_args
            .iter()
            .any(|a| a.contains("aura.client.hiddenWindow"))
        {
            actual_jvm_args.push("-Daura.client.hiddenWindow=true".to_string());
        }
    }

    let launch_res = state
        .core
        .launch_game(LaunchGameRequest {
            version: version.clone(),
            nickname: nickname.clone(),
            jvm_args: Some(actual_jvm_args),
        })
        .await;

    match launch_res {
        Ok(resp) => {
            state.is_launching.store(false, Ordering::SeqCst);
            state.is_running.store(true, Ordering::SeqCst);
            {
                let mut v = state.running_version.lock().await;
                *v = Some(version.clone());
            }

            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "running".to_string(),
                    message: "Minecraft запущен".to_string(),
                    exit_code: None,
                },
            );

            // Spawn background supervisor to monitor running process and detect crashes
            let app_clone = app.clone();
            let core_clone = state.core.clone();
            let is_running_clone = state.is_running.clone();
            let running_version_clone = state.running_version.clone();
            let discord_clone = state.discord.clone();
            let instance_dir = state.paths.instance_dir(&version);
            let version_clone = version.clone();

            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    match core_clone.is_running(&version_clone).await {
                        Ok(true) => continue,
                        _ => break,
                    }
                }

                is_running_clone.store(false, Ordering::SeqCst);
                {
                    let mut v = running_version_clone.lock().await;
                    *v = None;
                }

                // Switch Discord RPC back to InLauncher
                set_discord_rpc(&discord_clone, Activity::InLauncher).await;

                // Run crash diagnosis on instance logs
                let diagnosis = analyze_crash(&instance_dir, None);
                let (status_state, status_msg) = match &diagnosis.kind {
                    CrashKind::Unknown { exit_code: Some(0) } => {
                        ("idle".to_string(), "Minecraft закрыт штатно".to_string())
                    }
                    CrashKind::Unknown { exit_code: None } => {
                        if diagnosis.title.contains("Неизвестный сбой")
                            || diagnosis.technical_summary.is_empty()
                        {
                            ("idle".to_string(), "Сессия игры завершена".to_string())
                        } else {
                            (
                                "error".to_string(),
                                format!("{}: {}", diagnosis.title, diagnosis.user_advice),
                            )
                        }
                    }
                    _ => (
                        "error".to_string(),
                        format!("{}: {}", diagnosis.title, diagnosis.user_advice),
                    ),
                };

                let _ = app_clone.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: status_state,
                        message: status_msg,
                        exit_code: None,
                    },
                );
            });

            Ok(resp)
        }
        Err(e) => {
            state.is_launching.store(false, Ordering::SeqCst);

            // Switch Discord RPC back to InLauncher
            set_discord_rpc(&state.discord, Activity::InLauncher).await;

            // Crash analysis on startup failure
            let instance_dir = state.paths.instance_dir(&version);
            let diagnosis = analyze_crash(&instance_dir, None);
            let error_msg = if matches!(diagnosis.kind, CrashKind::Unknown { .. }) {
                e.to_string()
            } else {
                format!("{}: {}", diagnosis.title, diagnosis.user_advice)
            };

            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "error".to_string(),
                    message: error_msg.clone(),
                    exit_code: None,
                },
            );

            Err(error_msg)
        }
    }
}

/// Request cancellation of an ongoing launch.
#[tauri::command]
pub async fn cancel_launch(app: AppHandle, state: State<'_, PlayState>) -> Result<(), String> {
    let _ = state.core.cancel_launch().await;
    state.is_launching.store(false, Ordering::SeqCst);

    set_discord_rpc(&state.discord, Activity::InLauncher).await;

    let _ = app.emit(
        EVT_LAUNCH_STATUS,
        LaunchStatusPayload {
            state: "cancelled".to_string(),
            message: "Запуск остановлен пользователем".to_string(),
            exit_code: None,
        },
    );

    Ok(())
}

/// Check if the game is currently running.
#[tauri::command]
pub async fn is_game_running(state: State<'_, PlayState>) -> Result<bool, String> {
    Ok(state.is_running.load(Ordering::SeqCst))
}

/// Force kill the game instance.
#[tauri::command]
pub async fn kill_game(version: Option<String>, state: State<'_, PlayState>) -> Result<(), String> {
    let ver = match version {
        Some(v) => v,
        None => {
            let v = state.running_version.lock().await;
            v.clone().unwrap_or_default()
        }
    };

    if !ver.is_empty() {
        let _ = state.core.kill_game(&ver).await;
    }
    state.is_running.store(false, Ordering::SeqCst);
    state.is_launching.store(false, Ordering::SeqCst);

    set_discord_rpc(&state.discord, Activity::InLauncher).await;

    Ok(())
}

// ============================================================================
// Backward-compatibility wrappers for UI
// ============================================================================

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    version: String,
    nickname: String,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    start_game(app, version, nickname, None, state)
        .await
        .map(|r| r.message)
}

#[tauri::command]
pub async fn get_launcher_config(state: State<'_, PlayState>) -> Result<LauncherConfig, String> {
    get_config(state).await
}

#[tauri::command]
pub async fn save_launcher_config(
    new_config: LauncherConfig,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    save_config(new_config, state).await
}

#[tauri::command]
pub async fn set_config(
    new_config: LauncherConfig,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    save_config(new_config, state).await
}

// ============================================================================
// Unit Tests (Mock Core, no network)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_ipc::core_trait::MockLauncherCore;

    #[tokio::test]
    async fn test_mock_get_and_save_config() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths, mock_core.clone());

        // Initial config
        let cfg = mock_core.get_config().await.unwrap();
        assert_eq!(cfg.player_name, "Player");

        // Save updated config
        let mut new_cfg = cfg.clone();
        new_cfg.player_name = "SpeedRunner".to_string();
        new_cfg.ram_gb = 8;
        mock_core.save_config(new_cfg).await.unwrap();

        let updated = mock_core.get_config().await.unwrap();
        assert_eq!(updated.player_name, "SpeedRunner");
        assert_eq!(updated.ram_gb, 8);
    }

    #[tokio::test]
    async fn test_mock_installed_versions() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let _play_state = PlayState::new_with_core(paths, mock_core.clone());

        let versions = mock_core.get_installed_versions().await.unwrap();
        assert_eq!(versions.len(), 2);
        assert!(versions[0].is_installed);
    }

    #[tokio::test]
    async fn test_duplicate_start_prevention() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths, mock_core.clone());

        // Simulate launch in progress
        play_state.is_launching.store(true, Ordering::SeqCst);

        // Attempting to start another game while launching should be blocked
        assert!(play_state.is_launching.load(Ordering::SeqCst));
        assert!(!play_state.is_running.load(Ordering::SeqCst));

        // When running, also blocked
        play_state.is_launching.store(false, Ordering::SeqCst);
        play_state.is_running.store(true, Ordering::SeqCst);
        assert!(play_state.is_running.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_mock_cancel_launch() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths, mock_core.clone());

        play_state.is_launching.store(true, Ordering::SeqCst);
        mock_core.cancel_launch().await.unwrap();
        play_state.is_launching.store(false, Ordering::SeqCst);

        assert!(!play_state.is_launching.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_crash_analyzer_diagnosis() {
        let tmp = tempdir().unwrap();
        let inst_dir = tmp.path().join("instance_1_21");
        let logs_dir = inst_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).unwrap();

        let log_file = logs_dir.join("latest.log");
        std::fs::write(&log_file, "Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.main.Main.main(Main.java:237)\n").unwrap();

        let diagnosis = analyze_crash(&inst_dir, Some(1));
        assert!(matches!(diagnosis.kind, CrashKind::OutOfMemory));
        assert!(diagnosis.user_advice.contains("памяти") || diagnosis.user_advice.contains("RAM"));
    }

    #[tokio::test]
    async fn test_discord_rpc_status_transition() {
        let discord = Arc::new(Mutex::new(Some(
            DiscordRpcClient::connect_or_noop(DEFAULT_DISCORD_APP_ID).await,
        )));

        // Transition to playing
        set_discord_rpc(
            &discord,
            Activity::Playing {
                version: "1.21.1".to_string(),
                world_name: None,
                mod_count: Some(140),
                start_timestamp: Some(1731000000),
            },
        )
        .await;

        // Transition back to in launcher
        set_discord_rpc(&discord, Activity::InLauncher).await;
    }
}
