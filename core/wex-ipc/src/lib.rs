pub mod commands;
pub mod core_trait;
pub mod error;
pub mod events;
pub mod models;
pub mod ts_gen;

pub use commands::*;
pub use core_trait::{LauncherCore, MockLauncherCore};
pub use error::{IpcError, IpcResult};
pub use events::*;
pub use models::*;
pub use ts_gen::{generate_all_bindings, write_bindings_to_dir};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_mock_launcher_core() {
        let core = MockLauncherCore::new();

        // 1. Config
        let cfg = core.get_config().await.unwrap();
        assert_eq!(cfg.selected_version, "1.21.1");
        assert_eq!(cfg.player_name, "Player");

        let mut updated = cfg.clone();
        updated.player_name = "ProPlayer".to_string();
        updated.ram_gb = 8;
        core.save_config(updated.clone()).await.unwrap();

        let cfg2 = core.get_config().await.unwrap();
        assert_eq!(cfg2.player_name, "ProPlayer");
        assert_eq!(cfg2.ram_gb, 8);

        // 2. Installed versions
        let versions = core.get_installed_versions().await.unwrap();
        assert_eq!(versions.len(), 2);

        // 3. Install instance
        let install_res = core
            .install_instance(InstallInstanceRequest {
                version: "1.22.0".to_string(),
                force_reinstall: false,
            })
            .await
            .unwrap();
        assert!(install_res.success);

        let versions2 = core.get_installed_versions().await.unwrap();
        assert_eq!(versions2.len(), 3);

        // 4. Launch game
        let launch_res = core
            .launch_game(LaunchGameRequest {
                version: "1.21.1".to_string(),
                nickname: "AuraFan".to_string(),
                jvm_args: None,
            })
            .await
            .unwrap();
        assert!(launch_res.success);
        assert!(launch_res.pid.is_some());

        // 5. Is running
        assert!(core.is_running("1.21.1").await.unwrap());
        assert!(!core.is_running("1.20.1").await.unwrap());

        // 6. Kill game
        core.kill_game("1.21.1").await.unwrap();
        assert!(!core.is_running("1.21.1").await.unwrap());

        // 7. Simulated launch failure
        *core.fail_next_launch.lock().unwrap() = true;
        let fail_res = core
            .launch_game(LaunchGameRequest {
                version: "1.21.1".to_string(),
                nickname: "AuraFan".to_string(),
                jvm_args: None,
            })
            .await;
        assert!(fail_res.is_err());
        let err = fail_res.unwrap_err();
        assert_eq!(err.code, "LAUNCH_SIMULATED_FAILURE");
        assert!(err.message.contains("Minecraft"));
    }

    #[test]
    fn test_error_conversions() {
        // IO error
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let ipc_err: IpcError = io_err.into();
        assert_eq!(ipc_err.code, "IO_NOT_FOUND");
        assert!(ipc_err.message.contains("не найден"));
        assert!(ipc_err.technical.unwrap().contains("file missing"));

        // JSON error
        let json_err = serde_json::from_str::<LauncherConfig>("{ invalid }").unwrap_err();
        let ipc_err2: IpcError = json_err.into();
        assert_eq!(ipc_err2.code, "SERIALIZATION_ERROR");
        assert!(ipc_err2.technical.is_some());

        // Lobby error
        let lobby_err = wex_net::lobby::LobbyError::Api {
            status: 404,
            message: "Room not found".to_string(),
        };
        let ipc_err3: IpcError = lobby_err.into();
        assert_eq!(ipc_err3.code, "NET_LOBBY_NOT_FOUND");
        assert_eq!(ipc_err3.message, "Room not found");

        // Friend error
        let friend_err = wex_social::friends::FriendError::Unauthorized;
        let ipc_err4: IpcError = friend_err.into();
        assert_eq!(ipc_err4.code, "SOCIAL_UNAUTHORIZED");
        assert!(ipc_err4.message.contains("авторизации"));

        // Workshop error
        let workshop_err = wex_social::workshop::WorkshopError::NotFound(PathBuf::from("world-1"));
        let ipc_err5: IpcError = workshop_err.into();
        assert_eq!(ipc_err5.code, "WORKSHOP_NOT_FOUND");
        assert!(ipc_err5.message.contains("не найден"));
    }

    #[test]
    fn test_ipc_event_serde_roundtrip() {
        let evt = IpcEvent::InstallProgress(InstallProgressPayload {
            stage: "downloading_mods".to_string(),
            percentage: 42.5,
            detail: "Downloading iris-1.7.0.jar".to_string(),
            speed_bytes_per_sec: 10_000_000,
            total_bytes: 100_000_000,
            downloaded_bytes: 42_500_000,
        });

        let serialized = serde_json::to_string(&evt).unwrap();
        assert!(serialized.contains("\"event\":\"install-progress\""));
        assert!(serialized.contains("\"percentage\":42.5"));

        let deserialized: IpcEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(evt, deserialized);
    }

    #[test]
    fn test_ts_generation_completeness() {
        let bindings = generate_all_bindings();
        assert!(bindings.contains_key("error.ts"));
        assert!(bindings.contains_key("models.ts"));
        assert!(bindings.contains_key("commands.ts"));
        assert!(bindings.contains_key("events.ts"));
        assert!(bindings.contains_key("core.ts"));
        assert!(bindings.contains_key("index.ts"));

        let models_ts = bindings.get("models.ts").unwrap();
        assert!(models_ts.contains("export type LauncherConfig"));
        assert!(models_ts.contains("export type ServerPingResponseDto"));
        assert!(models_ts.contains("export type CrashDiagnosisDto"));
        assert!(models_ts.contains("export type SyncResponseDto"));

        let commands_ts = bindings.get("commands.ts").unwrap();
        assert!(commands_ts.contains("export const COMMANDS ="));
        assert!(commands_ts.contains("export type LaunchGameRequest"));

        let events_ts = bindings.get("events.ts").unwrap();
        assert!(events_ts.contains("export const EVENTS ="));
        assert!(events_ts.contains("export type IpcEvent"));

        let error_ts = bindings.get("error.ts").unwrap();
        assert!(error_ts.contains("export type IpcError"));
        assert!(error_ts.contains("export const ERROR_CODES ="));
    }

    #[test]
    fn test_typescript_bindings_drift_detection() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ui_generated_dir = manifest_dir.join("../../ui/src/lib/ipc/generated");

        let expected_files = generate_all_bindings();

        assert!(
            ui_generated_dir.exists(),
            "Generated TypeScript directory does not exist: {}. Run generator to create it.",
            ui_generated_dir.display()
        );

        for (filename, expected_content) in expected_files {
            let file_path = ui_generated_dir.join(&filename);
            assert!(
                file_path.exists(),
                "TypeScript generated file missing: {}. Types are out of date!",
                file_path.display()
            );

            let actual_content = std::fs::read_to_string(&file_path).unwrap_or_else(|e| {
                panic!("Failed to read {}: {}", file_path.display(), e);
            });

            // Normalize CRLF to LF for cross-platform comparison
            let expected_normalized = expected_content.replace("\r\n", "\n");
            let actual_normalized = actual_content.replace("\r\n", "\n");

            assert_eq!(
                expected_normalized, actual_normalized,
                "Drift detected in TypeScript definition file: {}. Run generator to sync types!",
                file_path.display()
            );
        }
    }
}
