use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use wex_extras::image;

use crate::commands::launch::PlayState;
use wex_core::config::WexPaths;
use wex_extras::skins::{
    convert_64x32_to_64x64, encode_png_bytes, generate_csl_config, remove_csl_player_files,
    validate_csl_inputs, validate_skin_bytes, SkinHistoryManager,
};

/// Every Minecraft version Wex installs gets the wardrobe.
const INSTANCE_VERSIONS: [&str; 2] = ["1.20.1", "1.21.1"];
use wex_ipc::commands::{
    ExtrasConvertSkinRequest, ExtrasConvertSkinResponse, ExtrasGenerateCslConfigRequest,
    ExtrasListSkinHistoryRequest, ExtrasListSkinHistoryResponse, ExtrasSaveSkinRequest,
    ExtrasValidateSkinRequest, WardrobeApplyRequest, WardrobeApplyResponse,
    WardrobeDeleteHistoryRequest, WardrobeDeleteHistoryResponse, WardrobeGetSkinRequest,
    WardrobeGetSkinResponse,
};
use wex_ipc::models::{SkinModelDto, SkinValidationResultDto, WardrobeItemDto, WardrobeStateDto};

pub fn validate_single_component(name: &str) -> Result<&str, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Имя не может быть пустым".to_string());
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
        || trimmed == "."
        || trimmed == ".."
    {
        return Err("Имя должно быть одиночным компонентом пути".to_string());
    }
    let p = Path::new(trimmed);
    let mut comps = p.components();
    match (comps.next(), comps.next()) {
        (Some(std::path::Component::Normal(_)), None) => Ok(trimmed),
        _ => Err("Имя должно быть одиночным компонентом пути".to_string()),
    }
}

/// Resolve a folder or file the UI names to a path under the Wex data root
/// `root`. Empty means `root` itself. The path must be absolute, without `..`,
/// and its deepest existing ancestor, after resolving links, must lie under the
/// canonical `root`; a folder that merely happens to be called «Wex» elsewhere
/// is refused.
pub fn resolve_under_root(root: &Path, requested: &str) -> Result<PathBuf, String> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Ok(root.to_path_buf());
    }
    let refused = || "Путь должен находиться внутри папки данных Wex".to_string();
    let path = PathBuf::from(requested);
    if !path.is_absolute()
        || path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
    {
        return Err(refused());
    }
    let canonical_root = root.canonicalize().map_err(|_| refused())?;
    let mut existing = path.as_path();
    let mut missing = Vec::new();
    while !existing.exists() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name.to_os_string());
                existing = parent;
            }
            _ => return Err(refused()),
        }
    }
    let mut resolved = existing.canonicalize().map_err(|_| refused())?;
    if !resolved.starts_with(&canonical_root) {
        return Err(refused());
    }
    for name in missing.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

#[tauri::command]
pub async fn extras_validate_skin(
    request: ExtrasValidateSkinRequest,
) -> Result<SkinValidationResultDto, String> {
    let bytes = BASE64
        .decode(&request.skin_bytes_base64)
        .map_err(|e| format!("Ошибка декодирования base64: {}", e))?;

    let res = validate_skin_bytes(&bytes);
    Ok(res.into())
}

#[tauri::command]
pub async fn extras_convert_skin(
    request: ExtrasConvertSkinRequest,
) -> Result<ExtrasConvertSkinResponse, String> {
    let bytes = BASE64
        .decode(&request.skin_bytes_base64)
        .map_err(|e| format!("Ошибка декодирования base64: {}", e))?;

    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("Некорректный PNG: {}", e))?
        .to_rgba8();

    let converted = convert_64x32_to_64x64(&img).map_err(|e| e.to_string())?;
    let png_bytes = encode_png_bytes(&converted).map_err(|e| e.to_string())?;

    Ok(ExtrasConvertSkinResponse {
        converted_bytes_base64: BASE64.encode(&png_bytes),
    })
}

#[tauri::command]
pub async fn extras_save_skin(
    request: ExtrasSaveSkinRequest,
    state: State<'_, PlayState>,
) -> Result<WardrobeItemDto, String> {
    save_skin_impl(&state.paths.root, request)
}

pub fn save_skin_impl(
    root: &Path,
    request: ExtrasSaveSkinRequest,
) -> Result<WardrobeItemDto, String> {
    let bytes = BASE64
        .decode(&request.skin_bytes_base64)
        .map_err(|e| format!("Ошибка декодирования base64: {}", e))?;

    let val = validate_skin_bytes(&bytes);
    if !val.is_valid {
        return Err(val
            .error
            .unwrap_or_else(|| "Файл не является корректным PNG скином".to_string()));
    }

    validate_single_component(&request.skin_name)
        .map_err(|e| format!("Недопустимое имя скина: {}", e))?;

    let history_dir = resolve_under_root(root, &request.history_dir)?;

    let mgr = SkinHistoryManager::new(&history_dir);
    let item = mgr
        .add_skin(&request.skin_name, &bytes, Some(request.model.into()))
        .map_err(|e| e.to_string())?;

    Ok(item.into())
}

pub fn list_skin_history_impl(dir: &Path) -> Result<Vec<WardrobeItemDto>, String> {
    let mgr = SkinHistoryManager::new(dir);
    let items = mgr.get_history().map_err(|e| e.to_string())?;
    Ok(items.into_iter().map(Into::into).collect())
}

pub fn delete_history_skin_impl(dir: &Path, item_id: &str) -> Result<bool, String> {
    let mgr = SkinHistoryManager::new(dir);
    mgr.remove_item(item_id).map_err(|e| e.to_string())
}

pub fn get_skin_impl(dir: &Path, item_id: &str) -> Result<(WardrobeItemDto, String), String> {
    let mgr = SkinHistoryManager::new(dir);
    let (bytes, item) = mgr
        .get_skin_bytes_by_id(item_id)
        .map_err(|e| e.to_string())?;
    Ok((item.into(), BASE64.encode(&bytes)))
}

#[tauri::command]
pub async fn extras_list_skin_history(
    request: ExtrasListSkinHistoryRequest,
    state: State<'_, PlayState>,
) -> Result<ExtrasListSkinHistoryResponse, String> {
    let dir = resolve_under_root(&state.paths.root, &request.history_dir)?;
    let items = list_skin_history_impl(&dir)?;
    Ok(ExtrasListSkinHistoryResponse { items })
}

#[tauri::command]
pub async fn wardrobe_delete_history(
    request: WardrobeDeleteHistoryRequest,
    state: State<'_, PlayState>,
) -> Result<WardrobeDeleteHistoryResponse, String> {
    let dir = resolve_under_root(&state.paths.root, &request.history_dir)?;
    validate_single_component(&request.item_id)
        .map_err(|e| format!("Недопустимый ID скина: {}", e))?;
    let success = delete_history_skin_impl(&dir, &request.item_id)?;
    Ok(WardrobeDeleteHistoryResponse { success })
}

#[tauri::command]
pub async fn wardrobe_get_skin(
    request: WardrobeGetSkinRequest,
    state: State<'_, PlayState>,
) -> Result<WardrobeGetSkinResponse, String> {
    let dir = resolve_under_root(&state.paths.root, &request.history_dir)?;
    validate_single_component(&request.item_id)
        .map_err(|e| format!("Недопустимый ID скина: {}", e))?;
    let (item, skin_bytes_base64) = get_skin_impl(&dir, &request.item_id)?;
    Ok(WardrobeGetSkinResponse {
        item,
        skin_bytes_base64,
    })
}

#[tauri::command]
pub async fn extras_generate_csl_config(
    request: ExtrasGenerateCslConfigRequest,
    state: State<'_, PlayState>,
) -> Result<(), String> {
    generate_csl_config_impl(&state.paths.root, request)
}

/// The config file must lie under the Wex data root (see `resolve_under_root`).
pub fn generate_csl_config_impl(
    root: &Path,
    request: ExtrasGenerateCslConfigRequest,
) -> Result<(), String> {
    if request.output_path.trim().is_empty() {
        return Err("Не указан путь конфигурации CSL".to_string());
    }
    let target_path = resolve_under_root(root, &request.output_path)
        .map_err(|_| "Путь конфигурации CSL должен находиться внутри папки Wex".to_string())?;
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let config_json = serde_json::json!({
        "version": "15.0.1",
        "buildNumber": 40,
        "loadlist": [
            {
                "name": "LocalSkin",
                "type": "Legacy",
                "checkPNG": false,
                "skin": "LocalSkin/skins/{USERNAME}.png",
                "model": "auto",
                "cape": "LocalSkin/capes/{USERNAME}.png",
                "elytra": "LocalSkin/elytras/{USERNAME}.png"
            },
            {
                "name": "WexLobby",
                "type": "CustomSkinAPI",
                "root": request.csl_api_root
            }
        ],
        "enableCape": true,
        "forceDisableCache": true
    });
    fs::write(
        &target_path,
        serde_json::to_string_pretty(&config_json).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn active_wardrobe_path(root: &Path) -> PathBuf {
    root.join("wardrobe.json")
}

#[tauri::command]
pub async fn wardrobe_get_state(state: State<'_, PlayState>) -> Result<WardrobeStateDto, String> {
    let path = active_wardrobe_path(&state.paths.root);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(dto) = serde_json::from_str::<WardrobeStateDto>(&content) {
                return Ok(dto);
            }
        }
    }

    // Default state: standard skin, no cape
    Ok(WardrobeStateDto {
        nickname: "Player".to_string(),
        skin_name: "Стандартный скин".to_string(),
        skin_base64: None,
        model: SkinModelDto::Classic,
        cape_id: "none".to_string(),
        cape_name: "Без плаща".to_string(),
        cape_base64: None,
    })
}

#[tauri::command]
pub async fn wardrobe_apply(
    request: WardrobeApplyRequest,
    state: State<'_, PlayState>,
) -> Result<WardrobeApplyResponse, String> {
    validate_single_component(&request.nickname)
        .map_err(|e| format!("Недопустимый никнейм: {}", e))?;
    validate_single_component(&request.skin_name)
        .map_err(|e| format!("Недопустимое имя скина: {}", e))?;

    // 1. Resolve skin bytes
    let skin_bytes = if let Some(ref b64) = request.skin_base64 {
        Some(
            BASE64
                .decode(b64)
                .map_err(|e| format!("Ошибка skin base64: {}", e))?,
        )
    } else {
        None
    };

    // 2. Resolve cape bytes
    let cape_bytes: Option<Vec<u8>> = match request.cape_id.as_str() {
        "custom" => {
            if let Some(ref b64) = request.cape_base64 {
                Some(
                    BASE64
                        .decode(b64)
                        .map_err(|e| format!("Ошибка cape base64: {}", e))?,
                )
            } else {
                None
            }
        }
        _ => None,
    };

    // 3. Validate everything before anything is written.
    validate_csl_inputs(
        &request.nickname,
        skin_bytes.as_deref(),
        cape_bytes.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    // 4. Save skin to Wex history & skins pool if provided
    if let Some(ref bytes) = skin_bytes {
        let mgr = SkinHistoryManager::new(&state.paths.root);
        mgr.add_skin(&request.skin_name, bytes, Some(request.model.into()))
            .map_err(|e| format!("Не удалось сохранить скин в историю: {}", e))?;
    }

    // 5. Export to CustomSkinLoader for all supported Minecraft versions
    export_to_instances(
        &state.paths,
        &request.nickname,
        skin_bytes.as_deref(),
        cape_bytes.as_deref(),
        request.model,
    )?;

    // 6. Persist active state in %APPDATA%\Wex\wardrobe.json
    let state_dto = WardrobeStateDto {
        nickname: request.nickname,
        skin_name: request.skin_name,
        skin_base64: request.skin_base64,
        model: request.model,
        cape_id: request.cape_id,
        cape_name: request.cape_name,
        cape_base64: request.cape_base64,
    };
    write_wardrobe_state(&state.paths.root, &state_dto)?;

    Ok(WardrobeApplyResponse {
        success: true,
        message: "Гардероб успешно применён к сборкам игры".to_string(),
    })
}

/// Both game versions always get the same textures. Every version is tried;
/// the error names each one that failed.
fn export_to_instances(
    paths: &WexPaths,
    nickname: &str,
    skin: Option<&[u8]>,
    cape: Option<&[u8]>,
    model: SkinModelDto,
) -> Result<(), String> {
    let mut failed = Vec::new();
    for v in INSTANCE_VERSIONS {
        let inst_dir = paths.instance_dir(v);
        let result = fs::create_dir_all(&inst_dir)
            .map_err(|e| e.to_string())
            .and_then(|()| {
                generate_csl_config(&inst_dir, nickname, skin, cape, Some(model.into()))
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = result {
            tracing::warn!("Не удалось экспортировать скин в инстанс {}: {}", v, e);
            failed.push(format!("{v}: {e}"));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Не удалось применить скин к сборкам: {}",
            failed.join("; ")
        ))
    }
}

fn write_wardrobe_state(root: &Path, dto: &WardrobeStateDto) -> Result<(), String> {
    let json_text = serde_json::to_string_pretty(dto).map_err(|e| e.to_string())?;
    fs::write(active_wardrobe_path(root), json_text)
        .map_err(|e| format!("Не удалось сохранить wardrobe.json: {}", e))
}

/// After the active account changed: the saved wardrobe follows the new player
/// name. The textures are exported for the new name in both versions, the old
/// name's LocalSkin files are deleted and wardrobe.json gets the new nickname.
/// Nothing happens without a saved wardrobe or when the name is unchanged.
pub fn sync_wardrobe_nickname(paths: &WexPaths, new_name: &str) -> Result<(), String> {
    let path = active_wardrobe_path(&paths.root);
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let Ok(mut dto) = serde_json::from_str::<WardrobeStateDto>(&content) else {
        return Ok(());
    };
    let new_name = new_name.trim();
    if dto.nickname == new_name {
        return Ok(());
    }
    validate_single_component(new_name).map_err(|e| format!("Недопустимый никнейм: {}", e))?;
    let decode = |b64: &Option<String>| -> Result<Option<Vec<u8>>, String> {
        b64.as_deref()
            .map(|b| BASE64.decode(b).map_err(|e| e.to_string()))
            .transpose()
    };
    let skin = decode(&dto.skin_base64)?;
    let cape = if dto.cape_id == "custom" {
        decode(&dto.cape_base64)?
    } else {
        None
    };
    validate_csl_inputs(new_name, skin.as_deref(), cape.as_deref()).map_err(|e| e.to_string())?;
    export_to_instances(paths, new_name, skin.as_deref(), cape.as_deref(), dto.model)?;
    if validate_single_component(&dto.nickname).is_ok() {
        for v in INSTANCE_VERSIONS {
            remove_csl_player_files(&paths.instance_dir(v), &dto.nickname)
                .map_err(|e| e.to_string())?;
        }
    }
    dto.nickname = new_name.to_string();
    write_wardrobe_state(&paths.root, &dto)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use wex_extras::image::{GenericImageView, Rgba, RgbaImage};

    fn make_test_skin_png(width: u32, height: u32) -> (Vec<u8>, String) {
        let img = RgbaImage::from_pixel(width, height, Rgba([30, 60, 90, 255]));
        let bytes = encode_png_bytes(&img).unwrap();
        let b64 = BASE64.encode(&bytes);
        (bytes, b64)
    }

    #[tokio::test]
    async fn test_validate_skin_command() {
        let (_bytes, b64_valid) = make_test_skin_png(64, 64);
        let res = extras_validate_skin(ExtrasValidateSkinRequest {
            skin_bytes_base64: b64_valid,
        })
        .await
        .unwrap();
        assert!(res.is_valid);
        assert_eq!(res.width, 64);
        assert_eq!(res.height, 64);

        let (_bytes, b64_bad) = make_test_skin_png(32, 32);
        let res_bad = extras_validate_skin(ExtrasValidateSkinRequest {
            skin_bytes_base64: b64_bad,
        })
        .await
        .unwrap();
        assert!(!res_bad.is_valid);
    }

    #[tokio::test]
    async fn test_convert_skin_command() {
        let (_bytes, b64_32) = make_test_skin_png(64, 32);
        let res = extras_convert_skin(ExtrasConvertSkinRequest {
            skin_bytes_base64: b64_32,
        })
        .await
        .unwrap();

        let converted_bytes = BASE64.decode(&res.converted_bytes_base64).unwrap();
        let val = validate_skin_bytes(&converted_bytes);
        assert!(val.is_valid);
        assert_eq!(val.width, 64);
        assert_eq!(val.height, 64);
    }

    #[tokio::test]
    async fn test_save_and_list_skin_history() {
        let dir = tempdir().unwrap();
        let history_dir = dir.path().join("wex");
        fs::create_dir_all(&history_dir).unwrap();

        let (_bytes, b64) = make_test_skin_png(64, 64);
        let saved = save_skin_impl(
            &history_dir,
            ExtrasSaveSkinRequest {
                history_dir: history_dir.to_string_lossy().to_string(),
                skin_name: "TestHero".to_string(),
                skin_bytes_base64: b64,
                model: SkinModelDto::Slim,
            },
        )
        .unwrap();

        assert_eq!(saved.name, "TestHero");
        assert_eq!(saved.model, "slim");

        let items = list_skin_history_impl(&history_dir).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, saved.id);

        let (item, skin_b64) = get_skin_impl(&history_dir, &saved.id).unwrap();
        assert_eq!(item.name, "TestHero");
        assert!(!skin_b64.is_empty());

        let del = delete_history_skin_impl(&history_dir, &saved.id).unwrap();
        assert!(del);

        let items_after = list_skin_history_impl(&history_dir).unwrap();
        assert_eq!(items_after.len(), 0);
    }

    #[test]
    fn test_custom_cape_decode() {
        let img = RgbaImage::from_pixel(64, 32, Rgba([20, 40, 60, 255]));
        let bytes = encode_png_bytes(&img).unwrap();
        let b64 = BASE64.encode(&bytes);
        let decoded = BASE64.decode(&b64).unwrap();
        let val = image::load_from_memory(&decoded).unwrap();
        assert_eq!(val.dimensions(), (64, 32));
    }

    #[tokio::test]
    async fn test_sec_extras_save_skin_rejects_outside_history_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("Wex");
        fs::create_dir_all(&root).unwrap();
        let outside_dir = dir.path().join("arbitrary_external_folder");

        let (_bytes, b64) = make_test_skin_png(64, 64);
        let res = save_skin_impl(
            &root,
            ExtrasSaveSkinRequest {
                history_dir: outside_dir.to_string_lossy().to_string(),
                skin_name: "TestHero".to_string(),
                skin_bytes_base64: b64,
                model: SkinModelDto::Slim,
            },
        );

        assert!(
            res.is_err(),
            "extras_save_skin must reject arbitrary outside history_dir"
        );
        assert!(
            !outside_dir.join("skins").exists(),
            "Must not write skins to arbitrary external folder"
        );
    }

    #[tokio::test]
    async fn test_sec_extras_save_skin_rejects_traversal_history_dir() {
        let dir = tempdir().unwrap();
        let (_bytes, b64) = make_test_skin_png(64, 64);
        let res = save_skin_impl(
            dir.path(),
            ExtrasSaveSkinRequest {
                history_dir: "../../escaped_dir".to_string(),
                skin_name: "TestHero".to_string(),
                skin_bytes_base64: b64,
                model: SkinModelDto::Slim,
            },
        );

        assert!(
            res.is_err(),
            "extras_save_skin must reject relative traversal in history_dir"
        );
    }

    #[test]
    fn a_folder_named_wex_outside_the_data_root_is_refused() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("data");
        fs::create_dir_all(&root).unwrap();
        let elsewhere = dir.path().join("elsewhere").join("Wex");
        fs::create_dir_all(&elsewhere).unwrap();
        assert!(resolve_under_root(&root, &elsewhere.to_string_lossy()).is_err());
        assert!(resolve_under_root(&root, "relative/wex").is_err());
        let traversal = root.join("..").join("elsewhere").join("Wex");
        assert!(resolve_under_root(&root, &traversal.to_string_lossy()).is_err());
        // Inside the root, existing or not yet created, and the root itself.
        assert!(resolve_under_root(&root, &root.join("a/b.json").to_string_lossy()).is_ok());
        assert_eq!(resolve_under_root(&root, "  ").unwrap(), root);
    }

    #[tokio::test]
    async fn csl_config_outside_the_data_root_is_refused() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("data");
        fs::create_dir_all(&root).unwrap();
        let outside = dir.path().join("wex").join("CustomSkinLoader.json");
        let res = generate_csl_config_impl(
            &root,
            ExtrasGenerateCslConfigRequest {
                output_path: outside.to_string_lossy().into(),
                csl_api_root: "https://fixture.invalid/".into(),
            },
        );
        assert!(res.is_err());
        assert!(!outside.exists());
    }

    fn paths_in(dir: &Path) -> WexPaths {
        let paths = WexPaths::new(Some(dir.join("Wex")));
        fs::create_dir_all(&paths.root).unwrap();
        paths
    }

    #[test]
    fn switching_account_moves_the_wardrobe_to_the_new_name() {
        let dir = tempdir().unwrap();
        let paths = paths_in(dir.path());
        let (_bytes, skin) = make_test_skin_png(64, 64);
        let (_bytes, cape) = make_test_skin_png(64, 32);
        let dto = WardrobeStateDto {
            nickname: "OldName".into(),
            skin_name: "Hero".into(),
            skin_base64: Some(skin),
            model: SkinModelDto::Slim,
            cape_id: "custom".into(),
            cape_name: "Cape".into(),
            cape_base64: Some(cape),
        };
        write_wardrobe_state(&paths.root, &dto).unwrap();
        for v in INSTANCE_VERSIONS {
            let skins = paths
                .instance_dir(v)
                .join("CustomSkinLoader/LocalSkin/skins");
            fs::create_dir_all(&skins).unwrap();
            fs::write(skins.join("OldName.png"), b"old").unwrap();
        }

        sync_wardrobe_nickname(&paths, "NewName").unwrap();

        for v in INSTANCE_VERSIONS {
            let local = paths.instance_dir(v).join("CustomSkinLoader/LocalSkin");
            assert!(local.join("skins/NewName.png").is_file(), "{v}");
            assert!(local.join("capes/NewName.png").is_file(), "{v}");
            assert!(!local.join("skins/OldName.png").exists(), "{v}");
        }
        let saved: WardrobeStateDto =
            serde_json::from_str(&fs::read_to_string(paths.root.join("wardrobe.json")).unwrap())
                .unwrap();
        assert_eq!(saved.nickname, "NewName");
        assert_eq!(saved.model, SkinModelDto::Slim);
    }

    #[test]
    fn switching_account_without_a_saved_wardrobe_does_nothing() {
        let dir = tempdir().unwrap();
        let paths = paths_in(dir.path());
        sync_wardrobe_nickname(&paths, "NewName").unwrap();
        assert!(!paths.root.join("wardrobe.json").exists());
        assert!(!paths.instance_dir("1.20.1").exists());
    }
}
