use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;
use wex_extras::image::{self, Rgba, RgbaImage};

use crate::commands::launch::PlayState;
use wex_extras::skins::{
    convert_64x32_to_64x64, encode_png_bytes, generate_csl_config, validate_skin_bytes,
    SkinHistoryManager,
};
use wex_ipc::commands::{
    ExtrasConvertSkinRequest, ExtrasConvertSkinResponse, ExtrasGenerateCslConfigRequest,
    ExtrasListSkinHistoryRequest, ExtrasListSkinHistoryResponse, ExtrasSaveSkinRequest,
    ExtrasValidateSkinRequest, WardrobeApplyRequest, WardrobeApplyResponse,
    WardrobeDeleteHistoryRequest, WardrobeDeleteHistoryResponse,
};
use wex_ipc::models::{SkinModelDto, SkinValidationResultDto, WardrobeItemDto, WardrobeStateDto};

/// Generate official Aura Cyan cape (64x32 PNG)
pub fn generate_aura_cyan_cape() -> Vec<u8> {
    let mut img = RgbaImage::from_pixel(64, 32, Rgba([0, 0, 0, 0]));
    // Cape body: (0, 0) to (22, 17)
    for y in 0..17 {
        for x in 0..22 {
            let base_color = if y == 0 {
                Rgba([10, 132, 255, 255]) // systemBlue #0A84FF
            } else if x >= 12 {
                // Back face of cape
                if (y == 8 || y == 9) && (x >= 15 && x <= 19) {
                    Rgba([255, 255, 255, 255]) // White Aura logo mark
                } else if y >= 14 {
                    Rgba([0, 91, 181, 255]) // Darker cyan border
                } else {
                    Rgba([10, 132, 255, 255])
                }
            } else {
                // Front face and sides
                Rgba([10, 132, 255, 255])
            };
            img.put_pixel(x, y, base_color);
        }
    }
    encode_png_bytes(&img).unwrap_or_default()
}

/// Generate official Aura Silver cape (64x32 PNG)
pub fn generate_aura_silver_cape() -> Vec<u8> {
    let mut img = RgbaImage::from_pixel(64, 32, Rgba([0, 0, 0, 0]));
    for y in 0..17 {
        for x in 0..22 {
            let base_color = if y == 0 {
                Rgba([142, 142, 147, 255]) // systemGray #8E8E93
            } else if x >= 12 {
                if (y == 8 || y == 9) && (x >= 15 && x <= 19) {
                    Rgba([255, 255, 255, 255])
                } else if y >= 14 {
                    Rgba([58, 58, 60, 255]) // Dark graphite trim
                } else {
                    Rgba([142, 142, 147, 255])
                }
            } else {
                Rgba([142, 142, 147, 255])
            };
            img.put_pixel(x, y, base_color);
        }
    }
    encode_png_bytes(&img).unwrap_or_default()
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
pub async fn extras_save_skin(request: ExtrasSaveSkinRequest) -> Result<WardrobeItemDto, String> {
    let bytes = BASE64
        .decode(&request.skin_bytes_base64)
        .map_err(|e| format!("Ошибка декодирования base64: {}", e))?;

    let mgr = SkinHistoryManager::new(&request.history_dir);
    let item = mgr
        .add_skin(&request.skin_name, &bytes, Some(request.model.into()))
        .map_err(|e| e.to_string())?;

    Ok(item.into())
}

#[tauri::command]
pub async fn extras_list_skin_history(
    request: ExtrasListSkinHistoryRequest,
) -> Result<ExtrasListSkinHistoryResponse, String> {
    let mgr = SkinHistoryManager::new(&request.history_dir);
    let items = mgr.get_history().map_err(|e| e.to_string())?;

    Ok(ExtrasListSkinHistoryResponse {
        items: items.into_iter().map(Into::into).collect(),
    })
}

#[tauri::command]
pub async fn wardrobe_delete_history(
    request: WardrobeDeleteHistoryRequest,
) -> Result<WardrobeDeleteHistoryResponse, String> {
    let mgr = SkinHistoryManager::new(&request.history_dir);
    let success = mgr
        .remove_item(&request.item_id)
        .map_err(|e| e.to_string())?;

    Ok(WardrobeDeleteHistoryResponse { success })
}

#[tauri::command]
pub async fn extras_generate_csl_config(
    request: ExtrasGenerateCslConfigRequest,
) -> Result<(), String> {
    let target_path = PathBuf::from(&request.output_path);
    if let Some(parent) = target_path.parent() {
        let _ = fs::create_dir_all(parent);
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
                "name": "AuraLobby",
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

    // Default state: Classic Steve, no cape
    Ok(WardrobeStateDto {
        nickname: "Player".to_string(),
        skin_name: "Steve (Классический)".to_string(),
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
        "cyan" => Some(generate_aura_cyan_cape()),
        "silver" => Some(generate_aura_silver_cape()),
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

    // 3. Save skin to Wex history & skins pool if provided
    if let Some(ref bytes) = skin_bytes {
        let mgr = SkinHistoryManager::new(&state.paths.root);
        let _ = mgr.add_skin(&request.skin_name, bytes, Some(request.model.into()));
    }

    // 4. Export to CustomSkinLoader for all supported Minecraft versions
    let versions = ["1.20.1", "1.21.1"];
    for v in &versions {
        let inst_dir = state.paths.instance_dir(v);
        let _ = fs::create_dir_all(&inst_dir);
        if let Err(e) = generate_csl_config(
            &inst_dir,
            &request.nickname,
            skin_bytes.as_deref(),
            cape_bytes.as_deref(),
        ) {
            tracing::warn!("Не удалось экспортировать скин в инстанс {}: {}", v, e);
        }
    }

    // 5. Persist active state in %APPDATA%\Wex\wardrobe.json
    let state_dto = WardrobeStateDto {
        nickname: request.nickname,
        skin_name: request.skin_name,
        skin_base64: request.skin_base64,
        model: request.model,
        cape_id: request.cape_id,
        cape_name: request.cape_name,
        cape_base64: request.cape_base64,
    };
    let json_text = serde_json::to_string_pretty(&state_dto).map_err(|e| e.to_string())?;
    fs::write(active_wardrobe_path(&state.paths.root), json_text)
        .map_err(|e| format!("Не удалось сохранить wardrobe.json: {}", e))?;

    Ok(WardrobeApplyResponse {
        success: true,
        message: "Гардероб успешно применён к сборкам игры".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
        let history_dir = dir.path().to_string_lossy().to_string();

        let (_bytes, b64) = make_test_skin_png(64, 64);
        let saved = extras_save_skin(ExtrasSaveSkinRequest {
            history_dir: history_dir.clone(),
            skin_name: "TestHero".to_string(),
            skin_bytes_base64: b64,
            model: SkinModelDto::Slim,
        })
        .await
        .unwrap();

        assert_eq!(saved.name, "TestHero");
        assert_eq!(saved.model, "slim");

        let list = extras_list_skin_history(ExtrasListSkinHistoryRequest {
            history_dir: history_dir.clone(),
        })
        .await
        .unwrap();

        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].id, saved.id);

        let del = wardrobe_delete_history(WardrobeDeleteHistoryRequest {
            history_dir: history_dir.clone(),
            item_id: saved.id,
        })
        .await
        .unwrap();
        assert!(del.success);

        let list_after = extras_list_skin_history(ExtrasListSkinHistoryRequest { history_dir })
            .await
            .unwrap();
        assert_eq!(list_after.items.len(), 0);
    }

    #[test]
    fn test_capes_generation() {
        let cyan = generate_aura_cyan_cape();
        assert!(!cyan.is_empty());
        let val_cyan = image::load_from_memory(&cyan).unwrap();
        assert_eq!(val_cyan.dimensions(), (64, 32));

        let silver = generate_aura_silver_cape();
        assert!(!silver.is_empty());
        let val_silver = image::load_from_memory(&silver).unwrap();
        assert_eq!(val_silver.dimensions(), (64, 32));
    }
}
