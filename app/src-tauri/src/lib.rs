pub mod ipc;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use ipc::{InstallProgressPayload, LaunchStatusPayload, LauncherConfig};

struct LaunchState {
    is_launching: AtomicBool,
    cancel_requested: AtomicBool,
    config: Mutex<LauncherConfig>,
}

#[tauri::command]
fn get_launcher_config(state: State<'_, LaunchState>) -> Result<LauncherConfig, String> {
    state
        .config
        .lock()
        .map(|c| c.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn save_launcher_config(
    new_config: LauncherConfig,
    state: State<'_, LaunchState>,
) -> Result<(), String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    Ok(())
}

#[tauri::command]
fn get_config(state: State<'_, LaunchState>) -> Result<LauncherConfig, String> {
    get_launcher_config(state)
}

#[tauri::command]
fn set_config(new_config: LauncherConfig, state: State<'_, LaunchState>) -> Result<(), String> {
    save_launcher_config(new_config, state)
}

#[tauri::command]
fn get_installed_versions() -> Vec<ipc::InstalledVersion> {
    vec![
        ipc::InstalledVersion {
            id: "1.20.1".to_string(),
            name: "1.20.1".to_string(),
            is_installed: true,
        },
        ipc::InstalledVersion {
            id: "1.21.1".to_string(),
            name: "1.21.1".to_string(),
            is_installed: true,
        },
    ]
}

#[tauri::command]
async fn launch_game(
    app: AppHandle,
    version: String,
    nickname: String,
    state: State<'_, LaunchState>,
) -> Result<String, String> {
    if state.is_launching.swap(true, Ordering::SeqCst) {
        return Err("Запуск уже выполняется".to_string());
    }
    state.cancel_requested.store(false, Ordering::SeqCst);

    let _ = app.emit(
        "launch-status",
        LaunchStatusPayload {
            state: "preparing".to_string(),
            message: format!("Подготовка к запуску {version}..."),
            exit_code: None,
        },
    );

    let steps = [
        ("Проверка клиента Aura", 15.0, "Проверка хешей jar и библиотек", 0, 100_000_000, 15_000_000),
        ("Синхронизация Aura-Pack", 40.0, "Загрузка манифеста модов и шейдеров", 15_400_000, 100_000_000, 40_000_000),
        ("Проверка Java Runtime", 75.0, "OpenJDK 21 HotSpot готов", 8_200_000, 100_000_000, 75_000_000),
        ("Запуск Minecraft", 100.0, "Инициализация Fabric Loader...", 0, 100_000_000, 100_000_000),
    ];

    for (stage, pct, detail, speed, total, downloaded) in steps {
        if state.cancel_requested.load(Ordering::SeqCst) {
            state.is_launching.store(false, Ordering::SeqCst);
            let _ = app.emit(
                "launch-status",
                LaunchStatusPayload {
                    state: "cancelled".to_string(),
                    message: "Запуск остановлен пользователем".to_string(),
                    exit_code: None,
                },
            );
            return Ok("Запуск отменён".to_string());
        }

        let _ = app.emit(
            "install-progress",
            InstallProgressPayload {
                stage: stage.to_string(),
                percentage: pct,
                detail: format!("{detail} ({nickname} / {version})"),
                speed_bytes_per_sec: speed,
                total_bytes: total,
                downloaded_bytes: downloaded,
            },
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;
    }

    let _ = app.emit(
        "launch-status",
        LaunchStatusPayload {
            state: "running".to_string(),
            message: "Minecraft запущен".to_string(),
            exit_code: None,
        },
    );

    state.is_launching.store(false, Ordering::SeqCst);
    Ok("Игра запущена успешно".to_string())
}

#[tauri::command]
async fn start_game(
    app: AppHandle,
    version: String,
    nickname: String,
    state: State<'_, LaunchState>,
) -> Result<String, String> {
    launch_game(app, version, nickname, state).await
}

#[tauri::command]
fn cancel_launch(state: State<'_, LaunchState>) -> Result<(), String> {
    state.cancel_requested.store(true, Ordering::SeqCst);
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(LaunchState {
            is_launching: AtomicBool::new(false),
            cancel_requested: AtomicBool::new(false),
            config: Mutex::new(LauncherConfig::default()),
        })
        .invoke_handler(tauri::generate_handler![
            get_launcher_config,
            save_launcher_config,
            get_config,
            set_config,
            get_installed_versions,
            launch_game,
            start_game,
            cancel_launch
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
