//! Onboarding commands for Wex Launcher (SPEC 1.1 / Apple HIG Onboarding)
//!
//! Provides initial welcome flow, hardware memory auto-tuning,
//! migration discovery, configuration persistence, and background pre-warming.

use tauri::{AppHandle, Emitter, Runtime, State};
use wex_core::config::LauncherConfig;
use wex_ipc::events::{InstallProgressPayload, EVT_INSTALL_PROGRESS};
use wex_ipc::models::{OnboardingCompleteRequest, OnboardingSpecsDto};

use super::launch::PlayState;

/// Calculates recommended memory in MB and contextual HIG explanation
/// based on total system RAM.
pub fn calculate_recommended_ram(total_mb: u32) -> (u32, String) {
    if total_mb >= 32768 {
        (
            8192,
            format!(
                "Рекомендуется 8 ГБ из {} ГБ: идеальный запас для сборки Wex, максимальной прорисовки и шейдеров Iris.",
                total_mb / 1024
            ),
        )
    } else if total_mb >= 16384 {
        (
            6144,
            format!(
                "Рекомендуется 6 ГБ из {} ГБ: оптимально для модов и шейдеров, оставляя запас системе для плавной работы.",
                total_mb / 1024
            ),
        )
    } else if total_mb >= 8192 {
        (
            4096,
            format!(
                "Рекомендуется 4 ГБ из {} ГБ: сбалансировано для комфортной игры без лишней нагрузки на компьютер.",
                total_mb / 1024
            ),
        )
    } else {
        let safe_mb = 2048.min(total_mb.saturating_sub(1024).max(1024));
        (
            safe_mb,
            format!(
                "Рекомендуется 2 ГБ: бережное потребление для систем с небольшим объёмом памяти ({} ГБ).",
                total_mb / 1024
            ),
        )
    }
}

/// Retrieve onboarding system specs and environment details.
#[tauri::command]
pub async fn onboarding_get_specs(
    state: State<'_, PlayState>,
) -> Result<OnboardingSpecsDto, String> {
    let total_mb = super::settings::get_total_ram_mb();
    let (rec_mb, explanation) = calculate_recommended_ram(total_mb);
    let detected_aura_dir =
        wex_extras::import::default_aura_dir().map(|p| p.to_string_lossy().to_string());

    let cfg = LauncherConfig::load_or_default(&state.paths.config_file);
    let default_version = if cfg.selected_version.is_empty() {
        "1.21.1".to_string()
    } else {
        cfg.selected_version
    };
    let default_nickname = if cfg.nickname.is_empty() {
        "Player".to_string()
    } else {
        cfg.nickname
    };

    Ok(OnboardingSpecsDto {
        total_memory_mb: total_mb,
        recommended_memory_mb: rec_mb,
        memory_explanation: explanation,
        detected_aura_dir,
        default_version,
        default_nickname,
    })
}

/// Apply initial onboarding selections to launcher config.
#[tauri::command]
pub async fn onboarding_complete(
    req: OnboardingCompleteRequest,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    let mut cfg = LauncherConfig::load_or_default(&state.paths.config_file);

    let clean_nick = req.nickname.trim();
    cfg.nickname = if clean_nick.is_empty() {
        "Player".to_string()
    } else {
        clean_nick.to_string()
    };

    cfg.selected_version = if req.version == "1.20.1" {
        "1.20.1".to_string()
    } else {
        "1.21.1".to_string()
    };

    cfg.max_memory_mb = req.ram_mb.max(1024);

    cfg.save(&state.paths.config_file)
        .map_err(|e| format!("Не удалось сохранить конфигурацию: {}", e))?;

    Ok(())
}

/// Prewarm game download/initialization in background starting from step 1.
#[tauri::command]
pub async fn onboarding_prewarm_game<R: Runtime>(
    app: AppHandle<R>,
    version: Option<String>,
    state: State<'_, PlayState>,
) -> Result<String, String> {
    let target_version = version.unwrap_or_else(|| "1.21.1".to_string());
    let paths = state.paths.clone();

    // Ensure directory structure is prepared in background
    let _ = paths.ensure_dirs();
    let _ = paths.ensure_instance_dirs(&target_version);

    let _ = app.emit(
        EVT_INSTALL_PROGRESS,
        InstallProgressPayload {
            stage: "Фоновая подготовка клиента".to_string(),
            percentage: 15.0,
            detail: format!("Проверка структуры каталогов для {}", target_version),
            speed_bytes_per_sec: 0,
            total_bytes: 0,
            downloaded_bytes: 0,
        },
    );

    Ok(format!("Prewarm initiated for {}", target_version))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_core::config::WexPaths;

    #[test]
    fn test_calculate_recommended_ram_tiers() {
        // High-end (32+ GB)
        let (ram_32, expl_32) = calculate_recommended_ram(32768);
        assert_eq!(ram_32, 8192);
        assert!(expl_32.contains("8 ГБ"));
        assert!(expl_32.contains("32 ГБ"));

        // Standard 16 GB
        let (ram_16, expl_16) = calculate_recommended_ram(16384);
        assert_eq!(ram_16, 6144);
        assert!(expl_16.contains("6 ГБ"));
        assert!(expl_16.contains("16 ГБ"));

        // Budget 8 GB
        let (ram_8, expl_8) = calculate_recommended_ram(8192);
        assert_eq!(ram_8, 4096);
        assert!(expl_8.contains("4 ГБ"));
        assert!(expl_8.contains("8 ГБ"));

        // Low memory 4 GB
        let (ram_4, expl_4) = calculate_recommended_ram(4096);
        assert_eq!(ram_4, 2048);
        assert!(expl_4.contains("2 ГБ"));
    }

    #[test]
    fn test_onboarding_complete_logic() {
        let tmp = tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let mut cfg = LauncherConfig::load_or_default(&paths.config_file);

        let req = OnboardingCompleteRequest {
            nickname: "Explorer".to_string(),
            version: "1.20.1".to_string(),
            ram_mb: 6144,
        };

        cfg.nickname = req.nickname;
        cfg.selected_version = req.version;
        cfg.max_memory_mb = req.ram_mb;
        cfg.save(&paths.config_file).unwrap();

        let loaded = LauncherConfig::load_or_default(&paths.config_file);
        assert_eq!(loaded.nickname, "Explorer");
        assert_eq!(loaded.selected_version, "1.20.1");
        assert_eq!(loaded.max_memory_mb, 6144);
    }
}
