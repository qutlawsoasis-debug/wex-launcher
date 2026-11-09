use super::*;

#[tokio::test]
async fn extras_validate_skin_success_checks_real_png_dimensions() {
    let result = wardrobe::extras_validate_skin(ExtrasValidateSkinRequest {
        skin_bytes_base64: png(64, 64),
    })
    .await
    .unwrap();
    assert!(result.is_valid);
    assert_eq!((result.width, result.height), (64, 64));
}
#[tokio::test]
async fn extras_validate_skin_error_invalid_base64() {
    assert!(wardrobe::extras_validate_skin(ExtrasValidateSkinRequest {
        skin_bytes_base64: "!!!".into()
    })
    .await
    .is_err());
}
#[tokio::test]
async fn extras_validate_skin_invalid_dimensions_return_validation_failure() {
    let result = wardrobe::extras_validate_skin(ExtrasValidateSkinRequest {
        skin_bytes_base64: png(1, 1),
    })
    .await
    .unwrap();
    assert!(!result.is_valid);
    assert!(result.error.is_some());
}
#[tokio::test]
async fn extras_convert_skin_success_produces_64_square_png() {
    use wex_extras::image::GenericImageView;
    let result = wardrobe::extras_convert_skin(ExtrasConvertSkinRequest {
        skin_bytes_base64: png(64, 32),
    })
    .await
    .unwrap();
    let bytes = BASE64.decode(result.converted_bytes_base64).unwrap();
    assert_eq!(
        wex_extras::image::load_from_memory(&bytes)
            .unwrap()
            .dimensions(),
        (64, 64)
    );
}
#[tokio::test]
async fn extras_convert_skin_error_invalid_png() {
    assert!(wardrobe::extras_convert_skin(ExtrasConvertSkinRequest {
        skin_bytes_base64: BASE64.encode(b"not PNG")
    })
    .await
    .is_err());
}
#[tokio::test]
async fn extras_save_skin_success_is_visible_in_history_command() {
    let f = Fixture::new().await;
    let saved = wardrobe::extras_save_skin(ExtrasSaveSkinRequest {
        history_dir: f.paths.root.to_string_lossy().into(),
        skin_name: "Fixture".into(),
        skin_bytes_base64: png(64, 64),
        model: SkinModelDto::Classic,
    })
    .await
    .unwrap();
    let history = wardrobe::extras_list_skin_history(
        ExtrasListSkinHistoryRequest {
            history_dir: f.paths.root.to_string_lossy().into(),
        },
        f.play(),
    )
    .await
    .unwrap();
    assert_eq!(history.items.len(), 1);
    assert_eq!(history.items[0].id, saved.id);
    assert_eq!(history.items[0].name, "Fixture");
}
#[tokio::test]
async fn extras_save_skin_error_invalid_base64_leaves_history_unchanged() {
    let f = Fixture::new().await;
    assert!(wardrobe::extras_save_skin(ExtrasSaveSkinRequest {
        history_dir: f.paths.root.to_string_lossy().into(),
        skin_name: "Fixture".into(),
        skin_bytes_base64: "!!!".into(),
        model: SkinModelDto::Classic
    })
    .await
    .is_err());
    assert!(!f.paths.root.join("wardrobe_history.json").exists());
}
#[tokio::test]
async fn extras_list_skin_history_success_absent_history_is_empty() {
    let f = Fixture::new().await;
    assert!(
        wardrobe::extras_list_skin_history(
            ExtrasListSkinHistoryRequest {
                history_dir: f.paths.root.to_string_lossy().into()
            },
            f.play(),
        )
        .await
        .unwrap()
        .items
        .is_empty()
    );
}
#[tokio::test]
async fn extras_list_skin_history_error_corrupt_history_is_reported() {
    let f = Fixture::new().await;
    fs::write(f.paths.root.join("wardrobe_history.json"), b"corrupt").unwrap();
    assert!(
        wardrobe::extras_list_skin_history(
            ExtrasListSkinHistoryRequest {
                history_dir: f.paths.root.to_string_lossy().into()
            },
            f.play(),
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn wardrobe_delete_history_success_removes_selected_entry() {
    let f = Fixture::new().await;
    let saved = wardrobe::extras_save_skin(ExtrasSaveSkinRequest {
        history_dir: f.paths.root.to_string_lossy().into(),
        skin_name: "Fixture".into(),
        skin_bytes_base64: png(64, 64),
        model: SkinModelDto::Classic,
    })
    .await
    .unwrap();
    let response = wardrobe::wardrobe_delete_history(
        WardrobeDeleteHistoryRequest {
            history_dir: f.paths.root.to_string_lossy().into(),
            item_id: saved.id,
        },
        f.play(),
    )
    .await
    .unwrap();
    assert!(response.success);
    assert!(
        wardrobe::extras_list_skin_history(
            ExtrasListSkinHistoryRequest {
                history_dir: f.paths.root.to_string_lossy().into()
            },
            f.play(),
        )
        .await
        .unwrap()
        .items
        .is_empty()
    );
}
#[tokio::test]
async fn wardrobe_delete_history_error_corrupt_history() {
    let f = Fixture::new().await;
    fs::write(f.paths.root.join("wardrobe_history.json"), b"corrupt").unwrap();
    assert!(
        wardrobe::wardrobe_delete_history(
            WardrobeDeleteHistoryRequest {
                history_dir: f.paths.root.to_string_lossy().into(),
                item_id: "missing".into()
            },
            f.play(),
        )
        .await
        .is_err()
    );
}
#[tokio::test]
async fn extras_generate_csl_config_success_preserves_api_root() {
    let f = Fixture::new().await;
    let output = f.paths.root.join("nested/CustomSkinLoader.json");
    wardrobe::extras_generate_csl_config(ExtrasGenerateCslConfigRequest {
        output_path: output.to_string_lossy().into(),
        csl_api_root: "https://fixture.invalid/".into(),
    })
    .await
    .unwrap();
    let config: Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert_eq!(config["loadlist"][1]["root"], "https://fixture.invalid/");
    assert_eq!(config["enableCape"], true);
}
#[tokio::test]
async fn extras_generate_csl_config_error_when_parent_is_file() {
    let f = Fixture::new().await;
    let blocker = f.paths.root.join("blocker");
    fs::write(&blocker, b"block").unwrap();
    assert!(
        wardrobe::extras_generate_csl_config(ExtrasGenerateCslConfigRequest {
            output_path: blocker.join("config.json").to_string_lossy().into(),
            csl_api_root: "fixture".into()
        })
        .await
        .is_err()
    );
}
fn wardrobe_request() -> WardrobeApplyRequest {
    WardrobeApplyRequest {
        nickname: "Fixture".into(),
        skin_name: "Fixture".into(),
        skin_base64: Some(png(64, 64)),
        model: SkinModelDto::Classic,
        cape_id: "none".into(),
        cape_name: "None".into(),
        cape_base64: None,
    }
}
#[tokio::test]
async fn wardrobe_apply_success_exports_to_both_versions_and_persists_state() {
    let f = Fixture::new().await;
    assert!(
        wardrobe::wardrobe_apply(wardrobe_request(), f.play())
            .await
            .unwrap()
            .success
    );
    for v in ["1.20.1", "1.21.1"] {
        assert!(f
            .paths
            .instance_dir(v)
            .join("CustomSkinLoader/LocalSkin/skins/Fixture.png")
            .is_file());
    }
    assert_eq!(
        wardrobe::wardrobe_get_state(f.play())
            .await
            .unwrap()
            .nickname,
        "Fixture"
    );
}
#[tokio::test]
async fn wardrobe_apply_error_invalid_skin_does_not_write_active_state() {
    let f = Fixture::new().await;
    let mut request = wardrobe_request();
    request.skin_base64 = Some("!!!".into());
    assert!(wardrobe::wardrobe_apply(request, f.play()).await.is_err());
    assert!(!f.paths.root.join("wardrobe.json").exists());
}
#[tokio::test]
async fn wardrobe_apply_error_invalid_custom_cape_does_not_write_active_state() {
    let f = Fixture::new().await;
    let mut request = wardrobe_request();
    request.cape_id = "custom".into();
    request.cape_base64 = Some("!!!".into());
    assert!(wardrobe::wardrobe_apply(request, f.play()).await.is_err());
    assert!(!f.paths.root.join("wardrobe.json").exists());
}
#[tokio::test]
async fn wardrobe_get_state_success_reads_saved_state() {
    let f = Fixture::new().await;
    let request = wardrobe_request();
    fs::write(f.paths.root.join("wardrobe.json"),serde_json::to_vec(&json!({"nickname":request.nickname,"skin_name":request.skin_name,"skin_base64":null,"model":"Classic","cape_id":"none","cape_name":"None","cape_base64":null})).unwrap()).unwrap();
    assert_eq!(
        wardrobe::wardrobe_get_state(f.play())
            .await
            .unwrap()
            .nickname,
        "Fixture"
    );
}
#[tokio::test]
async fn wardrobe_get_state_corrupt_json_returns_default_without_overwriting() {
    let f = Fixture::new().await;
    let file = f.paths.root.join("wardrobe.json");
    fs::write(&file, b"corrupt").unwrap();
    assert_eq!(
        wardrobe::wardrobe_get_state(f.play())
            .await
            .unwrap()
            .nickname,
        "Player"
    );
    assert_eq!(fs::read(file).unwrap(), b"corrupt");
}

#[tokio::test]
async fn overview_get_news_success_returns_bundled_versions() {
    let f = Fixture::new().await;
    assert!(!overview::overview_get_news(f.play())
        .await
        .unwrap()
        .is_empty());
}
#[tokio::test]
async fn social_get_news_success_returns_wrapped_versions() {
    let f = Fixture::new().await;
    assert!(!overview::social_get_news(f.play(), None)
        .await
        .unwrap()
        .versions
        .is_empty());
}
#[tokio::test]
async fn social_get_news_corrupt_cache_falls_back_to_bundled_news() {
    let f = Fixture::new().await;
    let cache = f.paths.root.join("cache-file");
    fs::write(&cache, b"not directory").unwrap();
    let request = SocialGetNewsRequest {
        cache_dir: cache.to_string_lossy().into(),
        remote_url: None,
        force_refresh: false,
    };
    assert!(!overview::social_get_news(f.play(), Some(request))
        .await
        .unwrap()
        .versions
        .is_empty());
}
fn update_manifest() -> Value {
    json!({"version":"2.0.0","notes":"fixture","platforms":{"windows-x86_64":{"signature":"fixture-signature-at-least-thirty-two-chars","url":"https://fixture.invalid/wex.exe"}}})
}
#[tokio::test]
async fn overview_check_update_success_uses_mock_manifest() {
    let f = Fixture::new().await;
    f.respond("/latest.json", 200, update_manifest()).await;
    let result =
        overview::overview_check_update(f.play(), Some(format!("{}/latest.json", f.server.uri())))
            .await
            .unwrap();
    assert!(result.update_available);
    assert_eq!(result.latest_version.as_deref(), Some("2.0.0"));
}
#[tokio::test]
async fn overview_check_update_error_invalid_json() {
    let f = Fixture::new().await;
    f.respond("/latest.json", 200, json!({"invalid":true}))
        .await;
    assert!(overview::overview_check_update(
        f.play(),
        Some(format!("{}/latest.json", f.server.uri()))
    )
    .await
    .is_err());
}
#[tokio::test]
async fn social_check_launcher_update_success_uses_request_current_version() {
    let f = Fixture::new().await;
    f.respond("/latest.json", 200, update_manifest()).await;
    let request = SocialCheckLauncherUpdateRequest {
        current_version: "2.0.0".into(),
        manifest_url: format!("{}/latest.json", f.server.uri()),
        target_platform: Some("windows-x86_64".into()),
    };
    let result = overview::social_check_launcher_update(f.play(), Some(request))
        .await
        .unwrap();
    assert!(!result.update_available);
    assert_eq!(result.current_version, "2.0.0");
}
#[tokio::test]
async fn social_check_launcher_update_error_http_failure() {
    let f = Fixture::new().await;
    f.respond("/latest.json", 503, json!({})).await;
    let request = SocialCheckLauncherUpdateRequest {
        current_version: "1.0.0".into(),
        manifest_url: format!("{}/latest.json", f.server.uri()),
        target_platform: None,
    };
    assert!(
        overview::social_check_launcher_update(f.play(), Some(request))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn extras_import_old_launcher_success_reads_only_explicit_fixture_directory() {
    let f = Fixture::new().await;
    let old = f.paths.root.join("old-fixture");
    fs::create_dir(&old).unwrap();
    fs::write(
        old.join("config.json"),
        br#"{"Nickname":"Imported","RamMb":6144,"SelectedVersion":"1.20.1"}"#,
    )
    .unwrap();
    let imported = overview::extras_import_old_launcher(ExtrasImportOldLauncherRequest {
        old_launcher_dir: old.to_string_lossy().into(),
    })
    .await
    .unwrap();
    assert_eq!(imported.nickname, "Imported");
    assert_eq!(imported.ram_mb, 6144);
    assert!(old.join("config.json").exists());
}
#[tokio::test]
async fn extras_import_old_launcher_error_missing_directory() {
    let f = Fixture::new().await;
    assert!(
        overview::extras_import_old_launcher(ExtrasImportOldLauncherRequest {
            old_launcher_dir: f.paths.root.join("missing").to_string_lossy().into()
        })
        .await
        .is_err()
    );
}
#[tokio::test]
async fn extras_apply_import_success_updates_core_without_touching_source() {
    let f = Fixture::new().await;
    let request = dto(
        json!({"old_launcher_dir":f.paths.root.join("unused-old"),"import_nickname":true,"nickname":"Imported","import_ram":true,"ram_mb":8192,"import_version":true,"selected_version":"1.20.1"}),
    );
    let response = overview::extras_apply_import(request, f.play())
        .await
        .unwrap();
    assert_eq!(response.imported_nickname.as_deref(), Some("Imported"));
    let config = launch::get_config(f.play()).await.unwrap();
    assert_eq!(config.player_name, "Imported");
    assert_eq!(config.ram_gb, 8);
    assert_eq!(config.selected_version, "1.20.1");
}
#[tokio::test]
async fn extras_apply_import_error_propagates_core_failure() {
    let f = Fixture::with_core(Arc::new(RejectingCore)).await;
    let request = dto(json!({"old_launcher_dir":f.paths.root}));
    assert!(overview::extras_apply_import(request, f.play())
        .await
        .unwrap_err()
        .contains("fixture failure"));
}
