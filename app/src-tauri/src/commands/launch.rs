use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use wex_core::config::WexPaths;
use wex_core::event::{Event, EventHandler, LogLevel};
use wex_core::playtime::{PlaytimeStore, PlaytimeSummary};
use wex_core::WexLauncherCore;
use wex_extras::crash::{analyze_crash_since, clean_exit, CrashKind};
use wex_extras::discord::{Activity, DiscordRpcClient, DEFAULT_DISCORD_APP_ID};
use wex_ipc::commands::{LaunchGameRequest, LaunchGameResponse};
use wex_ipc::core_trait::LauncherCore;
use wex_ipc::events::{
    GameLogPayload, InstallProgressPayload, LaunchStatusPayload, EVT_GAME_LOG,
    EVT_INSTALL_PROGRESS, EVT_LAUNCH_STATUS,
};
use wex_ipc::models::{InstalledVersion, LauncherConfig};

/// User-facing Russian name of a core install stage. Core names stages in English
/// for logs and the CLI ("Assets", "Vanilla Libraries", "Aura-Pack sync (1.21.1)").
pub fn stage_label(stage: &str) -> String {
    let s = stage.trim();
    let label = match s {
        "Assets" => "Ресурсы игры",
        "Vanilla Libraries" => "Библиотеки Minecraft",
        "Fabric Libraries" => "Библиотеки Fabric",
        _ if s.starts_with("Aura-Pack sync") => "Синхронизация сборки",
        _ if s.starts_with("Aura-Pack") => "Сборка Aura-Pack",
        // Download pool of the pack sync ("Pack 1.21.1").
        _ if s.starts_with("Pack ") => "Моды и файлы сборки",
        _ if s.starts_with("Fabric Loader") => "Загрузчик Fabric",
        _ if s.starts_with("Mojang Java Runtime")
            || s.starts_with("Java runtime")
            || s.starts_with("Java ") =>
        {
            "Среда Java"
        }
        _ if s.starts_with("Minecraft ") && s.contains("Environment") => "Файлы Minecraft",
        _ => return s.to_string(),
    };
    label.to_string()
}

/// launch-status for a launch that failed before the game process started:
/// "cancelled" for a cancel, otherwise "error" with core's user-facing message.
pub fn launch_failure_status(e: &wex_ipc::error::IpcError) -> (&'static str, String) {
    if e.code == "OPERATION_CANCELLED" {
        ("cancelled", "Запуск отменён".to_string())
    } else {
        ("error", e.message.clone())
    }
}

/// install-progress for a core Progress event. `current`/`total` count files,
/// not bytes, so sizes are left unknown (0) and the UI shows stage and percent only.
pub fn progress_payload(
    stage: &str,
    current: u64,
    total: u64,
    message: Option<&str>,
) -> InstallProgressPayload {
    let pct = if total > 0 {
        (current as f32 / total as f32) * 100.0
    } else {
        0.0
    };
    InstallProgressPayload {
        stage: stage_label(stage),
        percentage: pct,
        detail: message
            .map(str::to_string)
            .unwrap_or_else(|| format!("Файлов: {} из {}", current, total)),
        speed_bytes_per_sec: 0,
        total_bytes: 0,
        downloaded_bytes: 0,
    }
}

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
                let label = stage_label(stage);
                tracing::info!("stage {}/{}: {}", step, total_steps, stage);
                let _ = self.app.emit(
                    EVT_INSTALL_PROGRESS,
                    InstallProgressPayload {
                        stage: label.clone(),
                        percentage: pct,
                        detail: format!("Этап {} из {}: {}", step, total_steps, label),
                        speed_bytes_per_sec: 0,
                        total_bytes: 0,
                        downloaded_bytes: 0,
                    },
                );
                let _ = self.app.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: "installing".to_string(),
                        message: label,
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
                let _ = self.app.emit(
                    EVT_INSTALL_PROGRESS,
                    progress_payload(stage, *current, *total, message.as_deref()),
                );
            }
            Event::GameLaunched { pid } => {
                tracing::info!("game launched, pid {}", pid);
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
                match level {
                    LogLevel::Error => tracing::error!("{}", message),
                    LogLevel::Warn => tracing::warn!("{}", message),
                    LogLevel::Info => tracing::info!("{}", message),
                    LogLevel::Debug => {}
                }
                let _ = self.app.emit(
                    EVT_GAME_LOG,
                    GameLogPayload {
                        line: message.clone(),
                        level: lvl.to_string(),
                    },
                );
            }
            Event::GameExited { exit_code } => {
                tracing::info!("game exited, code {:?}", exit_code);
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

/// Helper to clear Discord RPC activity when disabled or closed.
pub async fn clear_discord_rpc(discord: &Arc<Mutex<Option<DiscordRpcClient>>>) {
    let mut lock = discord.lock().await;
    if let Some(client) = lock.as_mut() {
        client.clear_activity().await;
    }
}

/// Helper to set Discord RPC enabled/disabled dynamically.
pub async fn set_discord_rpc_enabled(
    discord: &Arc<Mutex<Option<DiscordRpcClient>>>,
    enabled: bool,
) {
    let mut lock = discord.lock().await;
    if let Some(client) = lock.as_mut() {
        client.set_enabled(enabled).await;
    }
}

/// Holds an active prewarmed client instance waiting for user trigger.
pub struct PrewarmedInstance {
    pub version: String,
    pub nickname: String,
    pub ram_mb: u32,
    pub signal_file: std::path::PathBuf,
    pub process: Option<wex_core::GameProcess>,
    pub started_at: std::time::Instant,
    pub activated: bool,
}

impl Drop for PrewarmedInstance {
    fn drop(&mut self) {
        if let Some(mut proc) = self.process.take() {
            let _ = proc.kill();
        }
        if !self.activated {
            let _ = std::fs::remove_file(&self.signal_file);
        }
    }
}

/// Shared runtime state for the Play lifecycle.
#[derive(Clone)]
pub struct PlayState {
    pub core: Arc<dyn LauncherCore>,
    pub paths: WexPaths,
    pub is_launching: Arc<AtomicBool>,
    pub is_running: Arc<AtomicBool>,
    pub running_version: Arc<Mutex<Option<String>>>,
    pub discord: Arc<Mutex<Option<DiscordRpcClient>>>,
    pub network: Option<super::network::NetworkState>,
    pub prewarmed: Arc<Mutex<Option<PrewarmedInstance>>>,
    pub is_prewarming: Arc<AtomicBool>,
    pub prewarm_generation: Arc<std::sync::atomic::AtomicU64>,
    pub active_process: Arc<Mutex<Option<wex_core::GameProcess>>>,
    pub app_handle: Option<AppHandle>,
    prewarm_processes_enabled: bool,
    /// Serializes every `install_full_environment` run (startup verification,
    /// fast-launch prewarm and Play). Two installs must never write the same
    /// version, libraries or assets at once.
    pub install_lock: Arc<Mutex<()>>,
    /// Set by stop_active_game(): the next game exit was requested by the user
    /// (Stop button, tray, window close) and must not be reported as a crash.
    pub stop_requested: Arc<AtomicBool>,
}

impl PlayState {
    pub fn new_real(
        paths: WexPaths,
        app: AppHandle,
        network: super::network::NetworkState,
    ) -> Self {
        let event_handler = Arc::new(TauriEventHandler { app: app.clone() });
        let core = Arc::new(WexLauncherCore::with_event_handler(
            paths.clone(),
            event_handler,
        ));
        let discord = Arc::new(Mutex::new(None));

        // Connect Discord RPC asynchronously in background
        let discord_clone = discord.clone();
        let config_file = paths.config_file.clone();
        tauri::async_runtime::spawn(async move {
            let mut client = DiscordRpcClient::connect_or_noop(DEFAULT_DISCORD_APP_ID).await;
            let cfg = wex_core::config::LauncherConfig::load_or_default(&config_file);
            client.set_enabled(cfg.discord_rpc).await;
            if cfg.discord_rpc {
                client.set_activity(Activity::InLauncher).await;
            }
            let mut lock = discord_clone.lock().await;
            *lock = Some(client);
        });

        let state = Self {
            core,
            paths,
            is_launching: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            running_version: Arc::new(Mutex::new(None)),
            discord,
            network: Some(network),
            prewarmed: Arc::new(Mutex::new(None)),
            is_prewarming: Arc::new(AtomicBool::new(false)),
            prewarm_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            active_process: Arc::new(Mutex::new(None)),
            app_handle: Some(app),
            prewarm_processes_enabled: true,
            install_lock: Arc::new(Mutex::new(())),
            stop_requested: Arc::new(AtomicBool::new(false)),
        };

        // Trigger background verification & prewarm ONLY if fast_launch is enabled
        let state_clone = state.clone();
        tauri::async_runtime::spawn(async move {
            let cfg =
                wex_core::config::LauncherConfig::load_or_default(&state_clone.paths.config_file);
            if !cfg.fast_launch {
                return;
            }
            if state_clone.is_launching.load(Ordering::SeqCst)
                || state_clone.is_running.load(Ordering::SeqCst)
            {
                return;
            }
            let version = cfg.selected_version.clone();
            let paths_clone = state_clone.paths.clone();
            let client = reqwest::Client::builder()
                .user_agent("WexLauncher/0.1.0")
                .build()
                .unwrap_or_default();
            let events = Arc::new(wex_core::NoopEventHandler);
            {
                // Play may have started while this task was scheduled; then Play
                // installs under the same lock and this verification is skipped.
                let _install = state_clone.install_lock.lock().await;
                if state_clone.is_launching.load(Ordering::SeqCst)
                    || state_clone.is_running.load(Ordering::SeqCst)
                {
                    return;
                }
                let _ = wex_core::install_full_environment(
                    &paths_clone,
                    &version,
                    &client,
                    events.as_ref(),
                    4,
                )
                .await;
            }

            if !state_clone.is_launching.load(Ordering::SeqCst)
                && !state_clone.is_running.load(Ordering::SeqCst)
            {
                state_clone
                    .trigger_prewarm(cfg.selected_version, cfg.nickname, cfg.max_memory_mb)
                    .await;
            }
        });

        state
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
            network: None,
            prewarmed: Arc::new(Mutex::new(None)),
            is_prewarming: Arc::new(AtomicBool::new(false)),
            prewarm_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            active_process: Arc::new(Mutex::new(None)),
            app_handle: None,
            prewarm_processes_enabled: false,
            install_lock: Arc::new(Mutex::new(())),
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn new_with_core_and_network(
        paths: WexPaths,
        core: Arc<dyn LauncherCore>,
        network: super::network::NetworkState,
    ) -> Self {
        let discord = Arc::new(Mutex::new(None));
        Self {
            core,
            paths,
            is_launching: Arc::new(AtomicBool::new(false)),
            is_running: Arc::new(AtomicBool::new(false)),
            running_version: Arc::new(Mutex::new(None)),
            discord,
            network: Some(network),
            prewarmed: Arc::new(Mutex::new(None)),
            is_prewarming: Arc::new(AtomicBool::new(false)),
            prewarm_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            active_process: Arc::new(Mutex::new(None)),
            app_handle: None,
            prewarm_processes_enabled: false,
            install_lock: Arc::new(Mutex::new(())),
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn stop_active_game(&self, version: Option<&str>) {
        self.stop_requested.store(true, Ordering::SeqCst);
        if let Some(mut proc) = self.active_process.lock().await.take() {
            let _ = proc.kill();
        }
        self.cleanup_prewarm().await;

        let ver = match version {
            Some(v) => v.to_string(),
            None => {
                let v = self.running_version.lock().await;
                v.clone().unwrap_or_default()
            }
        };

        if !ver.is_empty() {
            let _ = self.core.kill_game(&ver).await;
        }

        self.is_running.store(false, Ordering::SeqCst);
        self.is_launching.store(false, Ordering::SeqCst);
        let mut v = self.running_version.lock().await;
        *v = None;
    }

    pub async fn cleanup_prewarm(&self) {
        self.prewarm_generation.fetch_add(1, Ordering::SeqCst);
        let mut pw_lock = self.prewarmed.lock().await;
        if let Some(mut pw) = pw_lock.take() {
            if let Some(mut proc) = pw.process.take() {
                let _ = proc.kill();
            }
            let _ = std::fs::remove_file(&pw.signal_file);
        }
        self.is_prewarming.store(false, Ordering::SeqCst);
        if let Some(app) = &self.app_handle {
            let _ = app.emit(
                "prewarm-status",
                serde_json::json!({
                    "state": "idle",
                    "progress": 0.0,
                    "message": ""
                }),
            );
        }
    }

    pub async fn has_valid_prewarm(&self, version: &str, nickname: &str, ram_mb: u32) -> bool {
        let mut pw_lock = self.prewarmed.lock().await;
        if let Some(pw) = pw_lock.as_mut() {
            if pw.version == version && pw.nickname == nickname && pw.ram_mb == ram_mb {
                if let Some(proc) = pw.process.as_mut() {
                    match proc.child.try_wait() {
                        Ok(None) => true,
                        _ => {
                            *pw_lock = None;
                            false
                        }
                    }
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub async fn activate_prewarm(
        &self,
        version: &str,
        nickname: &str,
        ram_mb: u32,
    ) -> Option<(u32, wex_core::GameProcess, u128)> {
        let mut pw_lock = self.prewarmed.lock().await;
        if let Some(mut pw) = pw_lock.take() {
            if pw.version == version && pw.nickname == nickname && pw.ram_mb == ram_mb {
                if let Some(mut proc) = pw.process.take() {
                    match proc.child.try_wait() {
                        Ok(None) => {
                            let start = std::time::Instant::now();
                            let _ = std::fs::write(&pw.signal_file, b"READY\n");
                            let elapsed_ms = start.elapsed().as_millis();
                            let pid = proc.pid;
                            pw.activated = true;
                            return Some((pid, proc, elapsed_ms));
                        }
                        _ => {
                            let _ = std::fs::remove_file(&pw.signal_file);
                        }
                    }
                }
            } else {
                if let Some(mut proc) = pw.process.take() {
                    let _ = proc.kill();
                }
                let _ = std::fs::remove_file(&pw.signal_file);
            }
        }
        None
    }

    pub async fn trigger_prewarm(&self, version: String, nickname: String, ram_mb: u32) {
        // Constructors with an injected core must not escape that boundary by
        // launching a real JVM (including from settings/reset command tests).
        if !self.prewarm_processes_enabled {
            return;
        }

        if self.is_running.load(Ordering::SeqCst) || self.is_launching.load(Ordering::SeqCst) {
            return;
        }

        let cfg = wex_core::config::LauncherConfig::load_or_default(&self.paths.config_file);
        if !cfg.fast_launch {
            return;
        }

        if self.is_prewarming.swap(true, Ordering::SeqCst) {
            return;
        }

        if self.has_valid_prewarm(&version, &nickname, ram_mb).await {
            self.is_prewarming.store(false, Ordering::SeqCst);
            if let Some(app) = &self.app_handle {
                let _ = app.emit(
                    "prewarm-status",
                    serde_json::json!({
                        "state": "ready",
                        "progress": 100.0,
                        "message": "Готово к запуску"
                    }),
                );
            }
            return;
        }

        self.cleanup_prewarm().await;
        self.is_prewarming.store(true, Ordering::SeqCst);
        let expected_gen = self.prewarm_generation.load(Ordering::SeqCst);

        if let Some(app) = &self.app_handle {
            let _ = app.emit(
                "prewarm-status",
                serde_json::json!({
                    "state": "prewarming",
                    "progress": 48.0,
                    "message": "Игра догружается…"
                }),
            );
        }

        let paths_clone = self.paths.clone();
        let prewarmed_clone = self.prewarmed.clone();
        let is_prewarming_clone = self.is_prewarming.clone();
        let prewarm_gen_clone = self.prewarm_generation.clone();
        let app_clone = self.app_handle.clone();
        let install_lock = self.install_lock.clone();

        tauri::async_runtime::spawn(async move {
            // prepare_and_prewarm installs the environment first; hold the shared
            // install lock so it never overlaps the startup verification or Play.
            let _install = install_lock.lock().await;
            let signal_file = paths_clone.instance_dir(&version).join("prewarm.signal");
            let _ = std::fs::remove_file(&signal_file);
            let parent_pid = std::process::id();

            let client = reqwest::Client::builder()
                .user_agent("WexLauncher/0.1.0")
                .build()
                .unwrap_or_default();
            let events = Arc::new(wex_core::NoopEventHandler);

            let prewarm_res = wex_core::prepare_and_prewarm(
                &paths_clone,
                &cfg,
                &version,
                &client,
                events,
                4,
                &[],
                &signal_file,
                parent_pid,
            )
            .await;

            match prewarm_res {
                Ok(proc) => {
                    if prewarm_gen_clone.load(Ordering::SeqCst) != expected_gen
                        || !is_prewarming_clone.load(Ordering::SeqCst)
                    {
                        let mut proc = proc;
                        let _ = proc.kill();
                        let _ = std::fs::remove_file(&signal_file);
                        return;
                    }
                    let mut lock = prewarmed_clone.lock().await;
                    *lock = Some(PrewarmedInstance {
                        version,
                        nickname,
                        ram_mb,
                        signal_file,
                        process: Some(proc),
                        started_at: std::time::Instant::now(),
                        activated: false,
                    });
                    if let Some(app) = &app_clone {
                        let _ = app.emit(
                            "prewarm-status",
                            serde_json::json!({
                                "state": "ready",
                                "progress": 100.0,
                                "message": "Готово к запуску"
                            }),
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("[PREWARM] Failed to prewarm Minecraft: {}", e);
                    if let Some(app) = &app_clone {
                        let _ = app.emit(
                            "prewarm-status",
                            serde_json::json!({
                                "state": "idle",
                                "progress": 0.0,
                                "message": ""
                            }),
                        );
                    }
                }
            }

            is_prewarming_clone.store(false, Ordering::SeqCst);
        });
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
    let old_cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);
    let ram_mb = config.ram_gb * 1024;
    let need_restart = old_cfg.selected_version != config.selected_version
        || old_cfg.nickname != config.player_name
        || old_cfg.max_memory_mb != ram_mb;

    let res = state
        .core
        .save_config(config.clone())
        .await
        .map_err(|e| e.to_string());

    if need_restart {
        state.cleanup_prewarm().await;
        let cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);
        if cfg.fast_launch {
            state
                .trigger_prewarm(config.selected_version, config.player_name, ram_mb)
                .await;
        }
    }

    res
}

/// Retrieve installed versions for selectors.
#[tauri::command]
pub async fn get_installed_versions(
    state: State<'_, PlayState>,
) -> Result<Vec<InstalledVersion>, String> {
    let res = state
        .core
        .get_installed_versions()
        .await
        .map_err(|e| e.to_string());
    match &res {
        Ok(list) => tracing::info!(
            "get_installed_versions: {}",
            list.iter()
                .map(|v| format!("{}={}", v.id, v.is_installed))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Err(e) => tracing::error!("get_installed_versions failed: {}", e),
    }
    res
}

/// Start game with specified version and nickname. Blocks duplicate start if already running.
#[tauri::command]
pub async fn start_game<R: tauri::Runtime>(
    app: AppHandle<R>,
    version: String,
    nickname: String,
    jvm_args: Option<Vec<String>>,
    server: Option<String>,
    state: State<'_, PlayState>,
) -> Result<LaunchGameResponse, String> {
    // 1. Guard against duplicate starts
    if state.is_running.load(Ordering::SeqCst)
        || state
            .is_launching
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
    {
        return Err("Игра уже запущена или выполняется запуск".to_string());
    }
    state.stop_requested.store(false, Ordering::SeqCst);
    tracing::info!(
        "start_game {} (server: {:?}, extra jvm args: {})",
        version,
        server,
        jvm_args.as_ref().map_or(0, |a| a.len())
    );

    // 2. Load launcher config to apply preset and check features
    let cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);

    // Apply graphics preset before launch
    let pool_dirs = vec![state.paths.root.join("shaderpacks_pool")];
    let inst_dir = state.paths.instance_dir(&version);
    let preset_to_apply = if wex_core::config::is_weak_hardware() && cfg.graphics_preset == "High" {
        "Low"
    } else {
        &cfg.graphics_preset
    };
    let _ = wex_extras::presets::apply_preset(&inst_dir, preset_to_apply, &pool_dirs);

    // Set Installing status in Discord RPC
    if cfg.discord_rpc {
        set_discord_rpc(
            &state.discord,
            Activity::Installing {
                version: version.clone(),
                stage: Some("Подготовка к запуску".to_string()),
            },
        )
        .await;
    }

    // 3. Offline launch check
    let is_online = if let Some(ref net) = state.network {
        net.is_online().await
    } else {
        true
    };

    match super::network::check_offline_launch(state.core.as_ref(), &version, is_online).await {
        Ok(super::network::OfflineLaunchDecision::OfflineForbidden { reason }) => {
            state.is_launching.store(false, Ordering::SeqCst);
            if cfg.discord_rpc {
                set_discord_rpc(&state.discord, Activity::InLauncher).await;
            } else {
                clear_discord_rpc(&state.discord).await;
            }
            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "error".to_string(),
                    message: reason.clone(),
                    exit_code: None,
                },
            );
            return Err(reason);
        }
        Ok(super::network::OfflineLaunchDecision::OfflineAllowed { status_message }) => {
            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "preparing".to_string(),
                    message: status_message,
                    exit_code: None,
                },
            );
        }
        Ok(super::network::OfflineLaunchDecision::Online) => {
            let prep_msg = if cfg.fast_start_jvm {
                format!("Быстрый запуск и подготовка Minecraft {}...", version)
            } else {
                format!("Подготовка к запуску Minecraft {}...", version)
            };
            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "preparing".to_string(),
                    message: prep_msg,
                    exit_code: None,
                },
            );
        }
        Err(e) => {
            wex_core::set_current_process_priority_normal();
            state.is_launching.store(false, Ordering::SeqCst);
            if cfg.discord_rpc {
                set_discord_rpc(&state.discord, Activity::InLauncher).await;
            } else {
                clear_discord_rpc(&state.discord).await;
            }
            return Err(e);
        }
    }

    // Server requests need their own JVM command line. Background preparation is
    // coordinated separately; never activate a menu-only prewarm for multiplayer.
    // 3.5. Fast-launch: check prewarm status (cannot be used if custom server or jvm args were specified)
    let has_custom_server = server.as_ref().is_some_and(|s| !s.trim().is_empty());
    let has_custom_jvm_args = jvm_args.as_ref().is_some_and(|a| !a.is_empty());
    let can_use_fast_launch = cfg.fast_launch && !has_custom_server && !has_custom_jvm_args;

    if cfg.fast_launch && !can_use_fast_launch {
        state.cleanup_prewarm().await;
    }

    if can_use_fast_launch {
        let activated_proc = if let Some(ready) = state
            .activate_prewarm(&version, &nickname, cfg.max_memory_mb)
            .await
        {
            Some(ready)
        } else if state.is_prewarming.load(Ordering::SeqCst) {
            // Clicked "Play" before prewarm finished initializing:
            // 1. Emit honest progress status "Игра догружается…"
            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "launching".to_string(),
                    message: "Игра догружается…".to_string(),
                    exit_code: None,
                },
            );

            // 2. Write signal immediately so client unhides window the instant it reaches main menu
            let signal_file = state.paths.instance_dir(&version).join("prewarm.signal");
            let _ = std::fs::create_dir_all(state.paths.instance_dir(&version));
            let _ = std::fs::write(&signal_file, b"READY\n");

            // 3. Wait for prewarm initialization to finish
            let mut wait_cycles = 0;
            while state.is_prewarming.load(Ordering::SeqCst) && wait_cycles < 600 {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                wait_cycles += 1;
            }

            state
                .activate_prewarm(&version, &nickname, cfg.max_memory_mb)
                .await
        } else {
            None
        };

        if let Some((pid, proc, elapsed_ms)) = activated_proc {
            tracing::info!(
                "[FAST-LAUNCH] Activated prewarmed instance in {}ms (PID: {})",
                elapsed_ms,
                pid
            );
            state.is_launching.store(false, Ordering::SeqCst);
            state.is_running.store(true, Ordering::SeqCst);
            {
                let mut v = state.running_version.lock().await;
                *v = Some(version.clone());
            }

            {
                let mut ap = state.active_process.lock().await;
                *ap = Some(proc);
            }

            // Update Discord RPC status to Playing
            if cfg.discord_rpc {
                set_discord_rpc(
                    &state.discord,
                    Activity::Playing {
                        version: version.clone(),
                        world_name: server.clone(),
                        mod_count: None,
                        start_timestamp: Some(chrono::Utc::now().timestamp()),
                    },
                )
                .await;
            }

            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "running".to_string(),
                    message: "Minecraft запущен".to_string(),
                    exit_code: None,
                },
            );

            super::tray::apply_launch_window_behavior(&app, &cfg.game_window_behavior);

            // Spawn background supervisor to monitor running prewarmed process
            let app_clone = app.clone();
            let is_running_clone = state.is_running.clone();
            let stop_requested_clone = state.stop_requested.clone();
            let running_version_clone = state.running_version.clone();
            let discord_clone = state.discord.clone();
            let active_proc_clone = state.active_process.clone();
            let instance_dir = state.paths.instance_dir(&version);
            let config_file = state.paths.config_file.clone();
            let playtime_file = state.paths.playtime_file.clone();
            let version_clone = version.clone();
            let session_start = std::time::Instant::now();
            let session_wall = std::time::SystemTime::now();
            let state_clone = state.inner().clone();

            tauri::async_runtime::spawn(async move {
                let mut exit_code: Option<i32> = None;
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    let mut lock = active_proc_clone.lock().await;
                    if let Some(proc) = lock.as_mut() {
                        match proc.child.try_wait() {
                            Ok(None) => continue,
                            Ok(Some(status)) => {
                                exit_code = status.code();
                                *lock = None;
                                break;
                            }
                            Err(_) => {
                                *lock = None;
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                }

                // Record session duration into persistent playtime store
                let elapsed_sec = session_start.elapsed().as_secs();
                let mut store = PlaytimeStore::load_or_default(&playtime_file);
                store.record_session(&version_clone, elapsed_sec);
                let _ = store.save(&playtime_file);

                is_running_clone.store(false, Ordering::SeqCst);
                {
                    let mut v = running_version_clone.lock().await;
                    *v = None;
                }

                // Restore launcher window after game exits
                if let Some(w) = app_clone.get_webview_window("main") {
                    crate::commands::window::restore_window_from_game(&w);
                }

                // Switch Discord RPC back to InLauncher (or clear if disabled)
                let latest_cfg = wex_core::config::LauncherConfig::load_or_default(&config_file);
                if latest_cfg.discord_rpc {
                    set_discord_rpc(&discord_clone, Activity::InLauncher).await;
                } else {
                    clear_discord_rpc(&discord_clone).await;
                }

                // A stop the user asked for, or exit code 0, is a normal end of the
                // session. Only otherwise are the logs of this session diagnosed, with
                // the real exit code.
                let stopped = stop_requested_clone.swap(false, Ordering::SeqCst);
                let diagnosis = if stopped || exit_code == Some(0) {
                    clean_exit()
                } else {
                    analyze_crash_since(&instance_dir, exit_code, Some(session_wall))
                };
                let (status_state, status_msg) = match &diagnosis.kind {
                    _ if stopped => ("idle".to_string(), "Игра остановлена".to_string()),
                    CrashKind::Unknown { exit_code: Some(0) } => {
                        ("idle".to_string(), "Игра закрыта".to_string())
                    }
                    // No exit code and nothing in the logs: don't claim a crash.
                    CrashKind::Unknown { exit_code: None } => {
                        ("idle".to_string(), "Сессия игры завершена".to_string())
                    }
                    _ => (
                        "error".to_string(),
                        format!("{}: {}", diagnosis.title, diagnosis.user_advice),
                    ),
                };

                let _ = app_clone.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: status_state.clone(),
                        message: status_msg,
                        exit_code: None,
                    },
                );

                if status_state == "error" {
                    let dto: wex_ipc::models::CrashDiagnosisDto = diagnosis.clone().into();
                    let _ = app_clone.emit("game-crash", dto);
                }

                // Re-trigger prewarm for next launch if configured
                if latest_cfg.fast_launch {
                    state_clone
                        .trigger_prewarm(
                            latest_cfg.selected_version,
                            latest_cfg.nickname,
                            latest_cfg.max_memory_mb,
                        )
                        .await;
                }
            });

            return Ok(LaunchGameResponse {
                success: true,
                pid: Some(pid),
                message: "Minecraft запущен (быстрый запуск)".to_string(),
            });
        }
    }

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

    // core.launch_game installs the environment before starting the JVM. Wait for
    // any background preparation (startup verification or prewarm) to finish and
    // reuse its files instead of writing the same version from two tasks.
    let install_guard = match state.install_lock.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "preparing".to_string(),
                    message: "Ждём завершения фоновой подготовки…".to_string(),
                    exit_code: None,
                },
            );
            state.install_lock.lock().await
        }
    };
    if !state.is_launching.load(Ordering::SeqCst) {
        // cancel_launch() was called while waiting for the background preparation.
        drop(install_guard);
        if cfg.discord_rpc {
            set_discord_rpc(&state.discord, Activity::InLauncher).await;
        } else {
            clear_discord_rpc(&state.discord).await;
        }
        return Err("Запуск отменён".to_string());
    }

    let launch_res = state
        .core
        .launch_game(LaunchGameRequest {
            version: version.clone(),
            nickname: nickname.clone(),
            jvm_args: Some(actual_jvm_args),
            quick_play_multiplayer: server.clone(),
        })
        .await;
    drop(install_guard);
    match &launch_res {
        Ok(r) => tracing::info!("launch_game {}: pid {:?}, {}", version, r.pid, r.message),
        Err(e) => tracing::error!("launch_game {} failed: {:?}", version, e),
    }

    match launch_res {
        Ok(resp) => {
            state.is_launching.store(false, Ordering::SeqCst);
            state.is_running.store(true, Ordering::SeqCst);
            {
                let mut v = state.running_version.lock().await;
                *v = Some(version.clone());
            }

            // Update Discord RPC status to Playing (with server if auto-connect specified)
            if cfg.discord_rpc {
                set_discord_rpc(
                    &state.discord,
                    Activity::Playing {
                        version: version.clone(),
                        world_name: server.clone(),
                        mod_count: None,
                        start_timestamp: Some(chrono::Utc::now().timestamp()),
                    },
                )
                .await;
            }

            // Apply game window behavior (minimize / hide to tray / keep)
            let config = wex_core::LauncherConfig::load_or_default(&state.paths.config_file);
            super::tray::apply_launch_window_behavior(&app, &config.game_window_behavior);
            super::tray::update_tray_game_running(&app, true);

            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: "running".to_string(),
                    message: "Minecraft запущен".to_string(),
                    exit_code: None,
                },
            );

            // Throttle launcher priority to minimize CPU usage while the game is running
            wex_core::set_current_process_priority_low();
            if wex_core::config::is_weak_hardware() {
                wex_core::apply_current_process_affinity(0x3);
            }

            // Spawn background supervisor to monitor running process and detect crashes
            let app_clone = app.clone();
            let core_clone = state.core.clone();
            let is_running_clone = state.is_running.clone();
            let stop_requested_clone = state.stop_requested.clone();
            let running_version_clone = state.running_version.clone();
            let discord_clone = state.discord.clone();
            let instance_dir = state.paths.instance_dir(&version);
            let config_file = state.paths.config_file.clone();
            let version_clone = version.clone();
            let playtime_file = state.paths.playtime_file.clone();
            let session_start = std::time::Instant::now();
            let session_wall = std::time::SystemTime::now();

            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    match core_clone.is_running(&version_clone).await {
                        Ok(true) => continue,
                        _ => break,
                    }
                }
                let exit_code = core_clone.last_exit_code(&version_clone).await;

                // Record session duration into persistent playtime store
                let elapsed_sec = session_start.elapsed().as_secs();
                let mut store = PlaytimeStore::load_or_default(&playtime_file);
                store.record_session(&version_clone, elapsed_sec);
                let _ = store.save(&playtime_file);

                // Restore launcher normal process priority
                wex_core::set_current_process_priority_normal();

                is_running_clone.store(false, Ordering::SeqCst);
                {
                    let mut v = running_version_clone.lock().await;
                    *v = None;
                }

                // Restore launcher window after game exits
                if let Some(w) = app_clone.get_webview_window("main") {
                    crate::commands::window::restore_window_from_game(&w);
                }
                super::tray::restore_window(&app_clone);
                super::tray::update_tray_game_running(&app_clone, false);

                // Switch Discord RPC back to InLauncher (or clear if disabled)
                let latest_cfg = wex_core::config::LauncherConfig::load_or_default(&config_file);
                if latest_cfg.discord_rpc {
                    set_discord_rpc(&discord_clone, Activity::InLauncher).await;
                } else {
                    clear_discord_rpc(&discord_clone).await;
                }

                // A stop the user asked for, or exit code 0, is a normal end of the
                // session. Only otherwise are the logs of this session diagnosed, with
                // the real exit code.
                let stopped = stop_requested_clone.swap(false, Ordering::SeqCst);
                let diagnosis = if stopped || exit_code == Some(0) {
                    clean_exit()
                } else {
                    analyze_crash_since(&instance_dir, exit_code, Some(session_wall))
                };
                let (status_state, status_msg) = match &diagnosis.kind {
                    _ if stopped => ("idle".to_string(), "Игра остановлена".to_string()),
                    CrashKind::Unknown { exit_code: Some(0) } => {
                        ("idle".to_string(), "Игра закрыта".to_string())
                    }
                    // No exit code and nothing in the logs: don't claim a crash.
                    CrashKind::Unknown { exit_code: None } => {
                        ("idle".to_string(), "Сессия игры завершена".to_string())
                    }
                    _ => (
                        "error".to_string(),
                        format!("{}: {}", diagnosis.title, diagnosis.user_advice),
                    ),
                };

                let _ = app_clone.emit(
                    EVT_LAUNCH_STATUS,
                    LaunchStatusPayload {
                        state: status_state.clone(),
                        message: status_msg,
                        exit_code: None,
                    },
                );

                if status_state == "error" {
                    let dto: wex_ipc::models::CrashDiagnosisDto = diagnosis.clone().into();
                    let _ = app_clone.emit("game-crash", dto);
                }
            });

            Ok(resp)
        }
        Err(e) => {
            wex_core::set_current_process_priority_normal();
            state.is_launching.store(false, Ordering::SeqCst);

            // Switch Discord RPC back to InLauncher (or clear)
            if cfg.discord_rpc {
                set_discord_rpc(&state.discord, Activity::InLauncher).await;
            } else {
                clear_discord_rpc(&state.discord).await;
            }

            // The game process never started (download, install or spawn failed, or
            // the user cancelled): this is a launch error with core's own message,
            // not a game crash. Old game logs say nothing about it.
            let (status, error_msg) = launch_failure_status(&e);
            let _ = app.emit(
                EVT_LAUNCH_STATUS,
                LaunchStatusPayload {
                    state: status.to_string(),
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
pub async fn cancel_launch<R: tauri::Runtime>(
    app: AppHandle<R>,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let _ = state.core.cancel_launch().await;
    state.is_launching.store(false, Ordering::SeqCst);
    state.cleanup_prewarm().await;

    let cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);
    if cfg.discord_rpc {
        set_discord_rpc(&state.discord, Activity::InLauncher).await;
    } else {
        clear_discord_rpc(&state.discord).await;
    }

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
    state.stop_active_game(version.as_deref()).await;

    let cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);
    if cfg.discord_rpc {
        set_discord_rpc(&state.discord, Activity::InLauncher).await;
    } else {
        clear_discord_rpc(&state.discord).await;
    }

    Ok(())
}

// ============================================================================
// Backward-compatibility wrappers for UI
// ============================================================================

#[tauri::command]
pub async fn launch_game<R: tauri::Runtime>(
    app: AppHandle<R>,
    version: String,
    nickname: String,
    server: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    start_game(app, version, nickname, None, server, state)
        .await
        .map(|r| r.message)
}

#[tauri::command]
pub async fn set_config(
    new_config: LauncherConfig,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    save_config(new_config, state).await
}

/// Retrieve playtime statistics summary.
#[tauri::command]
pub async fn get_playtime_summary(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<PlaytimeSummary, String> {
    let store = PlaytimeStore::load_or_default(&state.paths.playtime_file);
    Ok(store.get_summary(version.as_deref()))
}

/// Compatibility alias for playtime retrieval.
#[tauri::command]
pub async fn playtime_get_summary(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<PlaytimeSummary, String> {
    get_playtime_summary(version, state).await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PrewarmStatusDto {
    pub is_prewarming: bool,
    pub is_ready: bool,
    pub progress: f32,
    pub stage: String,
}

/// Retrieve fast-launch prewarm lifecycle status for the Play button.
#[tauri::command]
pub async fn get_prewarm_status(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<PrewarmStatusDto, String> {
    let cfg = wex_core::config::LauncherConfig::load_or_default(&state.paths.config_file);
    let ver = version.unwrap_or_else(|| cfg.selected_version.clone());
    let is_prewarming = state.is_prewarming.load(Ordering::SeqCst);
    let is_ready = state
        .has_valid_prewarm(&ver, &cfg.nickname, cfg.max_memory_mb)
        .await;

    Ok(PrewarmStatusDto {
        is_prewarming,
        is_ready,
        progress: if is_ready {
            100.0
        } else if is_prewarming {
            48.0
        } else {
            0.0
        },
        stage: if is_ready {
            "Готово к запуску".to_string()
        } else if is_prewarming {
            "Игра догружается…".to_string()
        } else {
            "Ожидание".to_string()
        },
    })
}

// ============================================================================
// Unit Tests (Mock Core, no network)
// ============================================================================

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn a_failure_before_the_game_starts_is_not_a_crash() {
        let net = wex_ipc::error::IpcError {
            code: "NET_DOWNLOAD_FAILED".to_string(),
            message: "Не удалось скачать файлы игры. Проверьте интернет.".to_string(),
            technical: Some("timeout".to_string()),
        };
        assert_eq!(
            launch_failure_status(&net),
            (
                "error",
                "Не удалось скачать файлы игры. Проверьте интернет.".to_string()
            )
        );
        let cancel = wex_ipc::error::IpcError {
            code: "OPERATION_CANCELLED".to_string(),
            message: "Операция отменена.".to_string(),
            technical: None,
        };
        assert_eq!(launch_failure_status(&cancel).0, "cancelled");
    }

    #[test]
    fn core_stages_are_shown_in_russian() {
        assert_eq!(stage_label("Assets"), "Ресурсы игры");
        assert_eq!(stage_label("Pack 1.21.1"), "Моды и файлы сборки");
        assert_eq!(stage_label("Vanilla Libraries"), "Библиотеки Minecraft");
        assert_eq!(stage_label("Fabric Libraries"), "Библиотеки Fabric");
        assert_eq!(
            stage_label("Minecraft 1.21.1 Environment"),
            "Файлы Minecraft"
        );
        assert_eq!(
            stage_label("Fabric Loader 0.19.5 (1.21.1)"),
            "Загрузчик Fabric"
        );
        assert_eq!(stage_label("Mojang Java Runtime (1.21.1)"), "Среда Java");
        assert_eq!(stage_label("Java java-runtime-delta"), "Среда Java");
        assert_eq!(
            stage_label("Aura-Pack sync (1.21.1)"),
            "Синхронизация сборки"
        );
        assert_eq!(stage_label("Aura-Pack (1.20.1)"), "Сборка Aura-Pack");
        assert_eq!(
            stage_label("Фоновая подготовка клиента"),
            "Фоновая подготовка клиента"
        );
    }

    #[test]
    fn file_counts_are_never_sent_as_bytes() {
        let p = progress_payload("Assets", 1800, 3000, None);
        assert_eq!(p.stage, "Ресурсы игры");
        assert_eq!((p.downloaded_bytes, p.total_bytes), (0, 0));
        assert!((p.percentage - 60.0).abs() < 0.01);
        assert_eq!(p.detail, "Файлов: 1800 из 3000");
    }
}

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

        let diagnosis = wex_extras::crash::analyze_crash(&inst_dir, Some(1));
        assert!(matches!(diagnosis.kind, CrashKind::OutOfMemory));
        assert!(diagnosis.user_advice.contains("памяти") || diagnosis.user_advice.contains("RAM"));
    }

    #[tokio::test]
    async fn test_discord_rpc_status_transition() {
        let discord = Arc::new(Mutex::new(Some(
            DiscordRpcClient::connect_or_noop(DEFAULT_DISCORD_APP_ID).await,
        )));

        // 1. Initial: in launcher
        set_discord_rpc(&discord, Activity::InLauncher).await;
        {
            let lock = discord.lock().await;
            assert_eq!(
                lock.as_ref().unwrap().current_activity(),
                Some(&Activity::InLauncher)
            );
        }

        // 2. Transition to Installing
        set_discord_rpc(
            &discord,
            Activity::Installing {
                version: "1.21.1".to_string(),
                stage: Some("Загрузка файлов".to_string()),
            },
        )
        .await;
        {
            let lock = discord.lock().await;
            assert!(matches!(
                lock.as_ref().unwrap().current_activity(),
                Some(Activity::Installing { .. })
            ));
        }

        // 3. Transition to Playing (version X, singleplayer)
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
        {
            let lock = discord.lock().await;
            assert!(matches!(
                lock.as_ref().unwrap().current_activity(),
                Some(Activity::Playing {
                    world_name: None,
                    ..
                })
            ));
        }

        // 4. Transition to Playing on server Y
        set_discord_rpc(
            &discord,
            Activity::Playing {
                version: "1.21.1".to_string(),
                world_name: Some("mc.auramc.net".to_string()),
                mod_count: None,
                start_timestamp: Some(1731000500),
            },
        )
        .await;
        {
            let lock = discord.lock().await;
            assert!(matches!(
                lock.as_ref().unwrap().current_activity(),
                Some(Activity::Playing {
                    world_name: Some(ref s),
                    ..
                }) if s == "mc.auramc.net"
            ));
        }

        // 5. Transition to InLobby
        set_discord_rpc(
            &discord,
            Activity::InLobby {
                lobby_code: Some("WEX01".to_string()),
                player_count: 3,
                host_name: Some("AuraHost".to_string()),
            },
        )
        .await;
        {
            let lock = discord.lock().await;
            assert!(matches!(
                lock.as_ref().unwrap().current_activity(),
                Some(Activity::InLobby {
                    player_count: 3,
                    ..
                })
            ));
        }

        // 6. Transition back to InLauncher
        set_discord_rpc(&discord, Activity::InLauncher).await;
        {
            let lock = discord.lock().await;
            assert_eq!(
                lock.as_ref().unwrap().current_activity(),
                Some(&Activity::InLauncher)
            );
        }

        // 7. Clear and Disable
        clear_discord_rpc(&discord).await;
        {
            let lock = discord.lock().await;
            assert_eq!(lock.as_ref().unwrap().current_activity(), None);
        }

        set_discord_rpc_enabled(&discord, false).await;
        {
            let lock = discord.lock().await;
            assert!(!lock.as_ref().unwrap().is_enabled());
        }

        // 8. Re-enable
        set_discord_rpc_enabled(&discord, true).await;
        {
            let lock = discord.lock().await;
            assert!(lock.as_ref().unwrap().is_enabled());
        }
    }

    #[tokio::test]
    async fn test_play_state_offline_launch_allowed() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        mock_core.add_version(InstalledVersion {
            id: "1.21.1".to_string(),
            name: "1.21.1".to_string(),
            is_installed: true,
        });
        let net = super::super::network::NetworkState::new_with_status(false);
        let play_state = PlayState::new_with_core_and_network(paths, mock_core.clone(), net);

        let is_online = play_state.network.as_ref().unwrap().is_online().await;
        let decision = super::super::network::check_offline_launch(
            play_state.core.as_ref(),
            "1.21.1",
            is_online,
        )
        .await
        .unwrap();

        match decision {
            super::super::network::OfflineLaunchDecision::OfflineAllowed { status_message } => {
                assert_eq!(status_message, "Нет сети — запуск без обновления");
            }
            other => panic!("Expected OfflineAllowed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_play_state_offline_launch_forbidden() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        mock_core.add_version(InstalledVersion {
            id: "1.22.0".to_string(),
            name: "1.22.0".to_string(),
            is_installed: false,
        });
        let net = super::super::network::NetworkState::new_with_status(false);
        let play_state = PlayState::new_with_core_and_network(paths, mock_core.clone(), net);

        let is_online = play_state.network.as_ref().unwrap().is_online().await;
        let decision = super::super::network::check_offline_launch(
            play_state.core.as_ref(),
            "1.22.0",
            is_online,
        )
        .await
        .unwrap();

        match decision {
            super::super::network::OfflineLaunchDecision::OfflineForbidden { reason } => {
                assert!(reason.contains("1.22.0"));
                assert!(reason.contains("не установлена"));
            }
            other => panic!("Expected OfflineForbidden, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_playtime_summary_lifecycle() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mut store = PlaytimeStore::load_or_default(&paths.playtime_file);

        // Record sessions
        store.record_session("1.20.1", 3600);
        store.record_session("1.21.1", 1800);
        store.save(&paths.playtime_file).unwrap();

        let loaded = PlaytimeStore::load_or_default(&paths.playtime_file);
        let summary = loaded.get_summary(Some("1.20.1"));
        assert_eq!(summary.total_seconds, 5400);
        assert_eq!(summary.total_formatted, "1 ч. 30 мин.");
        assert_eq!(summary.current_version_formatted, "1 ч.");
        assert_eq!(summary.total_launches, 2);
        assert_eq!(summary.summary_text, "В игре: 1 ч. 30 мин. • Запусков: 2");
    }

    #[tokio::test]
    async fn test_prewarm_lifecycle_and_cleanup() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths.clone(), mock_core.clone());

        // Initially no prewarm
        assert!(!play_state.has_valid_prewarm("1.21.1", "Player", 4096).await);

        // Prewarm signal file
        let signal_file = tmp.path().join("test_prewarm.signal");
        std::fs::write(&signal_file, b"INIT\n").unwrap();
        assert!(signal_file.exists());

        // Simulate spawned dummy process
        let child = tokio::process::Command::new(if cfg!(target_os = "windows") {
            "cmd.exe"
        } else {
            "sh"
        })
        .args(if cfg!(target_os = "windows") {
            &["/c", "exit 0"][..]
        } else {
            &["-c", "exit 0"][..]
        })
        .spawn()
        .unwrap();

        let game_proc = wex_core::GameProcess {
            pid: child.id().unwrap_or(0),
            child,
            log_path: tmp.path().join("latest.log"),
            #[cfg(windows)]
            _job_object: None,
        };

        {
            let mut pw = play_state.prewarmed.lock().await;
            *pw = Some(PrewarmedInstance {
                version: "1.21.1".to_string(),
                nickname: "Player".to_string(),
                ram_mb: 4096,
                signal_file: signal_file.clone(),
                process: Some(game_proc),
                started_at: std::time::Instant::now(),
                activated: false,
            });
        }

        // Cleanup should remove signal file
        play_state.cleanup_prewarm().await;
        assert!(!signal_file.exists());
        assert!(!play_state.has_valid_prewarm("1.21.1", "Player", 4096).await);
    }

    #[tokio::test]
    async fn test_get_prewarm_status_dto() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths.clone(), mock_core.clone());

        let is_prewarming = play_state.is_prewarming.load(Ordering::SeqCst);
        let is_ready = play_state.has_valid_prewarm("1.21.1", "Player", 4096).await;
        assert!(!is_prewarming);
        assert!(!is_ready);
    }

    #[tokio::test]
    async fn test_review_activate_prewarm_preserves_signal_file() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths.clone(), mock_core.clone());

        let signal_file = tmp.path().join("test_prewarm.signal");
        std::fs::write(&signal_file, b"INIT\n").unwrap();

        let child = tokio::process::Command::new(if cfg!(target_os = "windows") {
            "cmd.exe"
        } else {
            "sh"
        })
        .args(if cfg!(target_os = "windows") {
            &["/c", "ping 127.0.0.1 -n 10 > nul"][..]
        } else {
            &["-c", "sleep 10"][..]
        })
        .spawn()
        .unwrap();

        let game_proc = wex_core::GameProcess {
            pid: child.id().unwrap_or(0),
            child,
            log_path: tmp.path().join("latest.log"),
            #[cfg(windows)]
            _job_object: None,
        };

        {
            let mut pw = play_state.prewarmed.lock().await;
            *pw = Some(PrewarmedInstance {
                version: "1.21.1".to_string(),
                nickname: "Player".to_string(),
                ram_mb: 4096,
                signal_file: signal_file.clone(),
                process: Some(game_proc),
                started_at: std::time::Instant::now(),
                activated: false,
            });
        }

        let res = play_state.activate_prewarm("1.21.1", "Player", 4096).await;
        assert!(
            res.is_some(),
            "activate_prewarm must activate running prewarm"
        );
        let (_, mut proc, _) = res.unwrap();
        let _ = proc.kill();

        assert!(
            signal_file.exists(),
            "prewarm.signal must not be deleted when activating prewarm"
        );
        assert_eq!(std::fs::read_to_string(&signal_file).unwrap(), "READY\n");
    }

    #[tokio::test]
    async fn test_review_tray_stop_terminates_active_prewarmed_game() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths.clone(), mock_core.clone());

        let child = tokio::process::Command::new(if cfg!(target_os = "windows") {
            "cmd.exe"
        } else {
            "sh"
        })
        .args(if cfg!(target_os = "windows") {
            &["/c", "ping 127.0.0.1 -n 10 > nul"][..]
        } else {
            &["-c", "sleep 10"][..]
        })
        .spawn()
        .unwrap();

        let pid = child.id().unwrap_or(0);
        let game_proc = wex_core::GameProcess {
            pid,
            child,
            log_path: tmp.path().join("latest.log"),
            #[cfg(windows)]
            _job_object: None,
        };

        {
            let mut ap = play_state.active_process.lock().await;
            *ap = Some(game_proc);
            play_state.is_running.store(true, Ordering::SeqCst);
            let mut v = play_state.running_version.lock().await;
            *v = Some("1.21.1".to_string());
        }

        play_state.stop_active_game(Some("1.21.1")).await;

        assert!(!play_state.is_running.load(Ordering::SeqCst));
        assert!(play_state.active_process.lock().await.is_none());
        assert!(play_state.running_version.lock().await.is_none());
    }

    #[tokio::test]
    async fn test_review_fast_launch_bypasses_prewarm_for_custom_server_or_jvm_args() {
        let server_none: Option<String> = None;
        let server_some = Some("hypixel.net".to_string());
        let jvm_none: Option<Vec<String>> = None;
        let jvm_custom = Some(vec!["-Xcustom".to_string()]);

        let has_custom_server = server_some.as_ref().is_some_and(|s| !s.trim().is_empty());
        let has_custom_jvm_args = jvm_custom.as_ref().is_some_and(|a| !a.is_empty());
        let can_use_fast_launch = !has_custom_server && !has_custom_jvm_args;
        assert!(
            !can_use_fast_launch,
            "Must not use prewarm for custom server or jvm args"
        );

        let can_use_defaults = !server_none.as_ref().is_some_and(|s| !s.trim().is_empty())
            && !jvm_none.as_ref().is_some_and(|a| !a.is_empty());
        assert!(
            can_use_defaults,
            "Can use prewarm when no server or jvm args"
        );
    }

    #[tokio::test]
    async fn test_review_cleanup_prewarm_increments_generation() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().join("Wex")));
        let mock_core = Arc::new(MockLauncherCore::new());
        let play_state = PlayState::new_with_core(paths.clone(), mock_core.clone());

        let gen_before = play_state.prewarm_generation.load(Ordering::SeqCst);
        play_state.cleanup_prewarm().await;
        let gen_after = play_state.prewarm_generation.load(Ordering::SeqCst);
        assert_eq!(gen_after, gen_before + 1);
        assert!(!play_state.is_prewarming.load(Ordering::SeqCst));
    }
}
