//! The launcher's own log: `<data root>\logs\launcher.log`.
//!
//! Everything sent through `tracing` (the app, core events bridged in
//! `TauriEventHandler`, and `ui_log` from the interface) lands here, so a run can
//! be read back after the fact. One previous run is kept as `launcher.old.log`.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The current log file name inside the logs folder.
pub const LOG_FILE: &str = "launcher.log";
/// The previous run's log.
pub const OLD_LOG_FILE: &str = "launcher.old.log";

/// Moves the last run's log aside and opens a fresh one.
pub fn open_log_file(logs_dir: &Path) -> std::io::Result<(File, PathBuf)> {
    fs::create_dir_all(logs_dir)?;
    let path = logs_dir.join(LOG_FILE);
    if path.exists() {
        let _ = fs::rename(&path, logs_dir.join(OLD_LOG_FILE));
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((file, path))
}

/// Installs the file logger. Safe to call once; later calls do nothing.
pub fn init(logs_dir: &Path) {
    let Ok((file, path)) = open_log_file(logs_dir) else {
        return;
    };
    let installed = tracing_subscriber::fmt()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .is_ok();
    if installed {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            tracing::error!("panic: {}", info);
            default_hook(info);
        }));
        tracing::info!(
            "Wex {} started, log {}",
            env!("CARGO_PKG_VERSION"),
            path.display()
        );
    }
}

/// A line from the interface (`ui_log`), so UI-side decisions show up in the same file.
#[tauri::command]
pub fn ui_log(level: String, message: String) {
    let message: String = message.chars().take(4000).collect();
    match level.as_str() {
        "error" => tracing::error!("[ui] {}", message),
        "warn" => tracing::warn!("[ui] {}", message),
        _ => tracing::info!("[ui] {}", message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_one_previous_run() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(LOG_FILE), b"run 1").unwrap();
        let (_f, path) = open_log_file(dir.path()).unwrap();
        assert_eq!(path, dir.path().join(LOG_FILE));
        assert_eq!(fs::read(dir.path().join(OLD_LOG_FILE)).unwrap(), b"run 1");
        assert_eq!(fs::read(&path).unwrap(), b"");
    }

    #[test]
    fn creates_the_logs_folder() {
        let dir = tempfile::tempdir().unwrap();
        let logs = dir.path().join("logs");
        open_log_file(&logs).unwrap();
        assert!(logs.join(LOG_FILE).exists());
    }
}
