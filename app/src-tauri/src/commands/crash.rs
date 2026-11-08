use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

use crate::commands::launch::PlayState;
use wex_core::config::WexPaths;
use wex_extras::crash::{analyze_crash, analyze_log_text};
use wex_ipc::commands::ExtrasAnalyzeCrashRequest;
use wex_ipc::models::CrashDiagnosisDto;

/// Resolves the instance directory for a specific Minecraft version.
fn resolve_instance_dir(paths: &WexPaths, version: Option<&str>) -> PathBuf {
    let ver = version.unwrap_or("1.21.1");
    paths.instance_dir(ver)
}

/// Helper function to analyze crash logs either from input text or from disk.
pub fn analyze_crash_impl(
    paths: &WexPaths,
    request: &ExtrasAnalyzeCrashRequest,
    version: Option<&str>,
) -> CrashDiagnosisDto {
    let combined_input = match (&request.log_text, &request.crash_report_text) {
        (Some(log), Some(report)) => format!("{}\n\n--- Crash Report ---\n{}", log, report),
        (Some(log), None) => log.clone(),
        (None, Some(report)) => report.clone(),
        (None, None) => String::new(),
    };

    let diagnosis = if !combined_input.trim().is_empty() {
        analyze_log_text(request.exit_code, &combined_input)
    } else {
        let inst_dir = resolve_instance_dir(paths, version);
        analyze_crash(&inst_dir, request.exit_code)
    };

    diagnosis.into()
}

/// Locates the most recent log or crash report file in the instance directory.
pub fn find_latest_log_path(inst_dir: &Path) -> Option<PathBuf> {
    // 1. Check crash-reports/*.txt first
    let crash_dir = inst_dir.join("crash-reports");
    if crash_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&crash_dir) {
            let mut reports: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "txt")
                        .unwrap_or(false)
                })
                .collect();

            reports.sort_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            });

            if let Some(latest) = reports.last() {
                return Some(latest.path());
            }
        }
    }

    // 2. Check logs/latest.log
    let latest_log = inst_dir.join("logs").join("latest.log");
    if latest_log.is_file() {
        return Some(latest_log);
    }

    // 3. Check hs_err_pid*.log in instance root
    if let Ok(entries) = fs::read_dir(inst_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with("hs_err_pid") && fname.ends_with(".log") {
                return Some(entry.path());
            }
        }
    }

    None
}

/// Opens the specified file or its directory in the OS file explorer/viewer.
pub fn open_file_in_os(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if path.is_file() {
            // Select file in Windows Explorer
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.to_string_lossy()))
                .spawn()
                .map_err(|e| format!("Не удалось открыть проводник: {}", e))?;
        } else {
            std::process::Command::new("explorer")
                .arg(path)
                .spawn()
                .map_err(|e| format!("Не удалось открыть проводник: {}", e))?;
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Не удалось открыть Finder: {}", e))?;
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        let target = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        std::process::Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Не удалось открыть менеджер файлов: {}", e))?;
        Ok(())
    }
}

/// Disables a mod by appending `.disabled` to its filename in the instance `mods/` directory.
pub fn disable_offending_mod_impl(
    paths: &WexPaths,
    version: Option<&str>,
    mod_identifier: &str,
) -> Result<String, String> {
    let inst_dir = resolve_instance_dir(paths, version);
    let mods_dir = inst_dir.join("mods");
    if !mods_dir.is_dir() {
        return Err("Папка модов не найдена".to_string());
    }

    let search_term = mod_identifier.trim().to_lowercase();
    if search_term.is_empty() {
        return Err("Не указан идентификатор мода".to_string());
    }

    let entries =
        fs::read_dir(&mods_dir).map_err(|e| format!("Не удалось прочитать папку модов: {}", e))?;

    let mut found_file: Option<PathBuf> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if fname.ends_with(".disabled") {
            continue;
        }

        if !fname.ends_with(".jar") {
            continue;
        }

        let clean_name = fname.to_lowercase();
        if clean_name == search_term
            || clean_name.starts_with(&format!("{}-", search_term))
            || clean_name.starts_with(&format!("{}_", search_term))
            || clean_name.contains(&search_term)
        {
            found_file = Some(path);
            break;
        }
    }

    if let Some(src_path) = found_file {
        let orig_name = src_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("mod.jar");
        let new_name = format!("{}.disabled", orig_name);
        let dest_path = mods_dir.join(&new_name);

        fs::rename(&src_path, &dest_path)
            .map_err(|e| format!("Не удалось переименовать файл мода: {}", e))?;

        Ok(new_name)
    } else {
        Err(format!(
            "Файл для мода «{}» не найден в папке mods",
            mod_identifier
        ))
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Analyze crash log texts or instance logs using wex-extras rule engine.
#[tauri::command]
pub async fn extras_analyze_crash(
    request: ExtrasAnalyzeCrashRequest,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<CrashDiagnosisDto, String> {
    Ok(analyze_crash_impl(
        &state.paths,
        &request,
        version.as_deref(),
    ))
}

/// Retrieve the latest crash diagnosis from the instance logs.
#[tauri::command]
pub async fn crash_get_last_diagnosis(
    version: Option<String>,
    exit_code: Option<i32>,
    state: State<'_, PlayState>,
) -> Result<CrashDiagnosisDto, String> {
    let req = ExtrasAnalyzeCrashRequest {
        exit_code,
        log_text: None,
        crash_report_text: None,
    };
    Ok(analyze_crash_impl(&state.paths, &req, version.as_deref()))
}

/// Opens the latest crash report or `latest.log` file in the OS file explorer.
#[tauri::command]
pub async fn crash_open_log_file(
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    let inst_dir = resolve_instance_dir(&state.paths, version.as_deref());
    if let Some(log_path) = find_latest_log_path(&inst_dir) {
        let path_str = log_path.to_string_lossy().to_string();
        open_file_in_os(&log_path)?;
        Ok(path_str)
    } else {
        // Fallback: open the logs directory itself
        let logs_dir = inst_dir.join("logs");
        fs::create_dir_all(&logs_dir)
            .map_err(|e| format!("Не удалось создать папку logs: {}", e))?;
        let path_str = logs_dir.to_string_lossy().to_string();
        open_file_in_os(&logs_dir)?;
        Ok(path_str)
    }
}

/// Disables an offending mod by appending `.disabled` to its filename.
#[tauri::command]
pub async fn crash_disable_mod(
    mod_identifier: String,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    disable_offending_mod_impl(&state.paths, version.as_deref(), &mod_identifier)
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_ipc::models::CrashKindDto;

    #[test]
    fn test_diagnose_out_of_memory() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let req = ExtrasAnalyzeCrashRequest {
            exit_code: Some(1),
            log_text: Some(
                "Exception in thread \"main\" java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.Main.main(Main.java:123)".to_string(),
            ),
            crash_report_text: None,
        };

        let diag = analyze_crash_impl(&paths, &req, Some("1.21.1"));
        assert_eq!(diag.kind, CrashKindDto::OutOfMemory);
        assert!(diag.title.contains("памяти"));
        assert!(diag.user_advice.contains("Настройках"));
    }

    #[test]
    fn test_diagnose_mod_dependency_and_disable() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let inst = paths.instance_dir("1.21.1");
        let mods_dir = inst.join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        // Create a fake mod file
        let mod_file = mods_dir.join("iris-mc1.21-1.7.0.jar");
        fs::write(&mod_file, b"dummy mod content").unwrap();

        let req = ExtrasAnalyzeCrashRequest {
            exit_code: Some(1),
            log_text: Some(
                "net.fabricmc.loader.impl.FormattedException: Mod 'iris' (1.7.0) requires 'sodium' (0.5.8), but only 0.5.2 is present!".to_string(),
            ),
            crash_report_text: None,
        };

        let diag = analyze_crash_impl(&paths, &req, Some("1.21.1"));
        match &diag.kind {
            CrashKindDto::MissingFabricDependency {
                mod_name,
                required_dep,
            } => {
                assert_eq!(mod_name, "iris");
                assert_eq!(required_dep, "sodium");
            }
            other => panic!("Expected MissingFabricDependency, got {:?}", other),
        }

        // Test disabling mod
        let res = disable_offending_mod_impl(&paths, Some("1.21.1"), "iris");
        assert!(res.is_ok());
        let disabled_name = res.unwrap();
        assert_eq!(disabled_name, "iris-mc1.21-1.7.0.jar.disabled");
        assert!(mods_dir.join("iris-mc1.21-1.7.0.jar.disabled").exists());
        assert!(!mod_file.exists());
    }

    #[test]
    fn test_diagnose_invalid_java_version() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let req = ExtrasAnalyzeCrashRequest {
            exit_code: Some(1),
            log_text: Some(
                "java.lang.UnsupportedClassVersionError: net/minecraft/client/Main has been compiled by a more recent version of the Java Runtime (class file version 65.0), this version of the Java Runtime only recognizes class file versions up to 61.0".to_string(),
            ),
            crash_report_text: None,
        };

        let diag = analyze_crash_impl(&paths, &req, Some("1.21.1"));
        match &diag.kind {
            CrashKindDto::InvalidJavaVersion { required_java, .. } => {
                assert!(required_java.contains("Java 21"));
            }
            other => panic!("Expected InvalidJavaVersion, got {:?}", other),
        }
        assert!(diag.title.contains("Java"));
    }

    #[test]
    fn test_diagnose_unknown_clean_fallback() {
        let dir = tempdir().unwrap();
        let paths = WexPaths::new(Some(dir.path().to_path_buf()));
        let req = ExtrasAnalyzeCrashRequest {
            exit_code: Some(-1),
            log_text: Some(
                "Normal game log without known error markers\nProcess finished with exit code -1"
                    .to_string(),
            ),
            crash_report_text: None,
        };

        let diag = analyze_crash_impl(&paths, &req, Some("1.21.1"));
        match &diag.kind {
            CrashKindDto::Unknown { exit_code } => {
                assert_eq!(*exit_code, Some(-1));
            }
            other => panic!("Expected Unknown, got {:?}", other),
        }
        // Zero fake fabricated causes
        assert!(diag.title.contains("код -1") || diag.title.contains("ошибкой"));
        assert!(diag.technical_summary.contains("Normal game log"));
    }
}
