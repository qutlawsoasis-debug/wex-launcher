use chrono::Utc;
use image::codecs::png::PngEncoder;
use image::{GenericImageView, ImageEncoder, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SkinError {
    #[error("Ошибка декодирования изображения: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Файл не является корректным PNG: {0}")]
    InvalidPng(String),
    #[error("Неподдерживаемый размер скина ({width}x{height}): требуется 64x64 или 64x32")]
    UnsupportedDimensions { width: u32, height: u32 },
    #[error("Размер файла превышает лимит в 2 МБ")]
    FileTooLarge,
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка сериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkinModel {
    #[serde(rename = "default")]
    Classic, // Steve 4px
    #[serde(rename = "slim")]
    Slim, // Alex 3px
}

impl SkinModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkinModel::Classic => "default",
            SkinModel::Slim => "slim",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkinValidationResult {
    pub is_valid: bool,
    pub error: Option<String>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WardrobeItem {
    pub id: String,
    pub item_type: String, // "skin" or "cape"
    pub name: String,
    pub hash: String,
    pub model: String, // "default" or "slim"
    pub relative_path: String,
    pub created_utc: String,
}

pub fn validate_skin_bytes(bytes: &[u8]) -> SkinValidationResult {
    if bytes.is_empty() {
        return SkinValidationResult {
            is_valid: false,
            error: Some("Файл скина пуст".to_string()),
            width: 0,
            height: 0,
        };
    }

    if bytes.len() > 2 * 1024 * 1024 {
        return SkinValidationResult {
            is_valid: false,
            error: Some("Размер файла скина превышает 2 МБ".to_string()),
            width: 0,
            height: 0,
        };
    }

    // PNG signature check
    if bytes.len() < 8 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return SkinValidationResult {
            is_valid: false,
            error: Some("Файл не является PNG изображением".to_string()),
            width: 0,
            height: 0,
        };
    }

    match image::load_from_memory(bytes) {
        Ok(img) => {
            let (w, h) = img.dimensions();
            if (w == 64 && h == 64) || (w == 64 && h == 32) {
                SkinValidationResult {
                    is_valid: true,
                    error: None,
                    width: w,
                    height: h,
                }
            } else {
                SkinValidationResult {
                    is_valid: false,
                    error: Some(format!(
                        "Неподдерживаемый размер скина ({}x{}). Требуется 64x64 или 64x32",
                        w, h
                    )),
                    width: w,
                    height: h,
                }
            }
        }
        Err(e) => SkinValidationResult {
            is_valid: false,
            error: Some(format!("Поврежденный файл PNG: {}", e)),
            width: 0,
            height: 0,
        },
    }
}

pub fn detect_skin_model(img: &RgbaImage) -> SkinModel {
    let (w, h) = img.dimensions();
    if w != 64 || h != 64 {
        return SkinModel::Classic;
    }

    // Alex (Slim): right arm has width of 3px (x=40..53).
    // Column 55 in rows y=20..31 is completely transparent (Alpha == 0).
    // Steve (Classic): column 55 contains the back side of the 4px arm and has alpha > 0.
    for y in 20..32 {
        let pixel = img.get_pixel(55, y);
        if pixel[3] > 0 {
            return SkinModel::Classic;
        }
    }

    SkinModel::Slim
}

pub fn convert_64x32_to_64x64(img_32: &RgbaImage) -> Result<RgbaImage, SkinError> {
    let (w, h) = img_32.dimensions();
    if w != 64 || h != 32 {
        return Err(SkinError::UnsupportedDimensions { width: w, height: h });
    }

    let mut img_64 = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));

    // 1. Copy top half 64x32
    for y in 0..32 {
        for x in 0..64 {
            img_64.put_pixel(x, y, *img_32.get_pixel(x, y));
        }
    }

    // 2. Duplicate right leg (0, 16, 16, 16) -> left leg (16, 48, 16, 16)
    for dy in 0..16 {
        for dx in 0..16 {
            let px = *img_32.get_pixel(dx, 16 + dy);
            img_64.put_pixel(16 + dx, 48 + dy, px);
        }
    }

    // 3. Duplicate right arm (40, 16, 16, 16) -> left arm (32, 48, 16, 16)
    for dy in 0..16 {
        for dx in 0..16 {
            let px = *img_32.get_pixel(40 + dx, 16 + dy);
            img_64.put_pixel(32 + dx, 48 + dy, px);
        }
    }

    Ok(img_64)
}

pub fn encode_png_bytes(img: &RgbaImage) -> Result<Vec<u8>, SkinError> {
    let mut buf = Vec::new();
    let encoder = PngEncoder::new(&mut buf);
    encoder.write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(buf)
}

pub struct SkinHistoryManager {
    wex_dir: PathBuf,
}

impl SkinHistoryManager {
    pub fn new(wex_dir: impl AsRef<Path>) -> Self {
        Self {
            wex_dir: wex_dir.as_ref().to_path_buf(),
        }
    }

    pub fn skins_dir(&self) -> PathBuf {
        self.wex_dir.join("skins")
    }

    pub fn capes_dir(&self) -> PathBuf {
        self.wex_dir.join("capes")
    }

    pub fn history_file(&self) -> PathBuf {
        self.wex_dir.join("wardrobe_history.json")
    }

    pub fn add_skin(
        &self,
        name: &str,
        raw_bytes: &[u8],
        forced_model: Option<SkinModel>,
    ) -> Result<WardrobeItem, SkinError> {
        let val = validate_skin_bytes(raw_bytes);
        if !val.is_valid {
            return Err(SkinError::InvalidPng(
                val.error.unwrap_or_else(|| "Недопустимый скин".to_string()),
            ));
        }

        let mut img = image::load_from_memory(raw_bytes)?.to_rgba8();
        if val.height == 32 {
            img = convert_64x32_to_64x64(&img)?;
        }

        let model = forced_model.unwrap_or_else(|| detect_skin_model(&img));
        let png_bytes = encode_png_bytes(&img)?;

        let mut hasher = Sha256::new();
        hasher.update(&png_bytes);
        let hash = hex::encode(hasher.finalize());

        fs::create_dir_all(self.skins_dir())?;
        let filename = format!("{}.png", hash);
        let target_path = self.skins_dir().join(&filename);
        fs::write(&target_path, &png_bytes)?;

        let mut history = self.get_history()?;
        let id = format!("skin_{}", hash);

        // Deduplicate in history
        history.retain(|i| i.id != id);

        let item = WardrobeItem {
            id: id.clone(),
            item_type: "skin".to_string(),
            name: name.to_string(),
            hash: hash.clone(),
            model: model.as_str().to_string(),
            relative_path: format!("skins/{}", filename),
            created_utc: Utc::now().to_rfc3339(),
        };

        history.insert(0, item.clone());
        self.save_history(&history)?;

        Ok(item)
    }

    pub fn get_history(&self) -> Result<Vec<WardrobeItem>, SkinError> {
        let path = self.history_file();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let text = fs::read_to_string(path)?;
        let history: Vec<WardrobeItem> = serde_json::from_str(&text)?;
        Ok(history)
    }

    pub fn save_history(&self, items: &[WardrobeItem]) -> Result<(), SkinError> {
        fs::create_dir_all(&self.wex_dir)?;
        let json = serde_json::to_string_pretty(items)?;
        fs::write(self.history_file(), json)?;
        Ok(())
    }

    pub fn remove_item(&self, id: &str) -> Result<bool, SkinError> {
        let mut history = self.get_history()?;
        let orig_len = history.len();
        history.retain(|i| i.id != id);
        if history.len() != orig_len {
            self.save_history(&history)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn generate_csl_config(
    game_dir: &Path,
    nickname: &str,
    skin_png_bytes: Option<&[u8]>,
    cape_png_bytes: Option<&[u8]>,
) -> Result<(), SkinError> {
    let csl_dir = game_dir.join("CustomSkinLoader");
    let local_skins_dir = csl_dir.join("LocalSkin").join("skins");
    let local_capes_dir = csl_dir.join("LocalSkin").join("capes");

    fs::create_dir_all(&local_skins_dir)?;
    fs::create_dir_all(&local_capes_dir)?;

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
                "root": "https://lobby-api.vercel.app/csl/"
            }
        ],
        "enableDynamicSkull": true,
        "enableTransparentSkin": false,
        "forceLoadAllTextures": true,
        "enableCape": true,
        "threadPoolSize": 4,
        "enableLogStdOut": false,
        "cacheExpiry": 0,
        "forceUpdateSkull": true,
        "enableLocalProfileCache": false,
        "enableCacheAutoClean": true,
        "forceDisableCache": true
    });

    let config_path = csl_dir.join("CustomSkinLoader.json");
    fs::write(
        config_path,
        serde_json::to_string_pretty(&config_json)?,
    )?;

    if let Some(bytes) = skin_png_bytes {
        let skin_path = local_skins_dir.join(format!("{}.png", nickname));
        fs::write(skin_path, bytes)?;
    }

    if let Some(bytes) = cape_png_bytes {
        let cape_path = local_capes_dir.join(format!("{}.png", nickname));
        fs::write(cape_path, bytes)?;
    }

    // Clear caches directory
    let caches_dir = csl_dir.join("caches");
    if caches_dir.exists() {
        let _ = fs::remove_dir_all(caches_dir);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_skin_image(width: u32, height: u32, is_slim: bool) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(width, height, Rgba([255, 128, 64, 255]));
        if width == 64 && height == 64 && is_slim {
            // make column 55 transparent for rows 20..32
            for y in 20..32 {
                img.put_pixel(55, y, Rgba([0, 0, 0, 0]));
            }
        }
        img
    }

    #[test]
    fn test_skin_validation_dimensions() {
        let img64 = make_test_skin_image(64, 64, false);
        let bytes64 = encode_png_bytes(&img64).unwrap();
        let res64 = validate_skin_bytes(&bytes64);
        assert!(res64.is_valid);
        assert_eq!(res64.width, 64);
        assert_eq!(res64.height, 64);

        let img32 = make_test_skin_image(64, 32, false);
        let bytes32 = encode_png_bytes(&img32).unwrap();
        let res32 = validate_skin_bytes(&bytes32);
        assert!(res32.is_valid);
        assert_eq!(res32.width, 64);
        assert_eq!(res32.height, 32);

        let img_bad = make_test_skin_image(32, 32, false);
        let bytes_bad = encode_png_bytes(&img_bad).unwrap();
        let res_bad = validate_skin_bytes(&bytes_bad);
        assert!(!res_bad.is_valid);
    }

    #[test]
    fn test_detect_slim_and_classic() {
        let classic_img = make_test_skin_image(64, 64, false);
        assert_eq!(detect_skin_model(&classic_img), SkinModel::Classic);

        let slim_img = make_test_skin_image(64, 64, true);
        assert_eq!(detect_skin_model(&slim_img), SkinModel::Slim);
    }

    #[test]
    fn test_convert_64x32_to_64x64() {
        let mut img32 = RgbaImage::from_pixel(64, 32, Rgba([10, 20, 30, 255]));
        // Set distinct pixels for leg (0, 16) and arm (40, 16)
        img32.put_pixel(0, 16, Rgba([100, 150, 200, 255]));
        img32.put_pixel(40, 16, Rgba([200, 220, 240, 255]));

        let img64 = convert_64x32_to_64x64(&img32).unwrap();
        assert_eq!(img64.dimensions(), (64, 64));

        // Check top half preserved
        assert_eq!(img64.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));

        // Check duplicated left leg
        assert_eq!(img64.get_pixel(16, 48), &Rgba([100, 150, 200, 255]));

        // Check duplicated left arm
        assert_eq!(img64.get_pixel(32, 48), &Rgba([200, 220, 240, 255]));
    }

    #[test]
    fn test_skin_history_manager() {
        let dir = tempdir().unwrap();
        let mgr = SkinHistoryManager::new(dir.path());

        let img = make_test_skin_image(64, 64, true);
        let bytes = encode_png_bytes(&img).unwrap();

        let item = mgr.add_skin("AlexSkin", &bytes, None).unwrap();
        assert_eq!(item.name, "AlexSkin");
        assert_eq!(item.model, "slim");
        assert!(mgr.skins_dir().join(format!("{}.png", item.hash)).exists());

        let history = mgr.get_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, item.id);

        let removed = mgr.remove_item(&item.id).unwrap();
        assert!(removed);
        assert_eq!(mgr.get_history().unwrap().len(), 0);
    }

    #[test]
    fn test_generate_csl_config() {
        let dir = tempdir().unwrap();
        let game_dir = dir.path();

        let img = make_test_skin_image(64, 64, false);
        let skin_bytes = encode_png_bytes(&img).unwrap();

        generate_csl_config(game_dir, "TestPlayer", Some(&skin_bytes), None).unwrap();

        let csl_json = game_dir.join("CustomSkinLoader").join("CustomSkinLoader.json");
        assert!(csl_json.is_file());

        let player_skin = game_dir
            .join("CustomSkinLoader")
            .join("LocalSkin")
            .join("skins")
            .join("TestPlayer.png");
        assert!(player_skin.is_file());
    }
}
