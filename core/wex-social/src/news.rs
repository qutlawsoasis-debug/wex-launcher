use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NewsError {
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка разбора JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteCategory {
    New,
    Improved,
    Fixed,
    General,
}

impl NoteCategory {
    pub fn display_badge(&self) -> &'static str {
        match self {
            NoteCategory::New => "Новое",
            NoteCategory::Improved => "Улучшено",
            NoteCategory::Fixed => "Исправлено",
            NoteCategory::General => "Инфо",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseNoteItem {
    pub category: NoteCategory,
    pub text: String,
    pub raw: String,
}

impl ReleaseNoteItem {
    pub fn parse(raw: impl Into<String>) -> Self {
        let raw_str = raw.into();
        let trimmed = raw_str.trim();

        if let Some(stripped) = trimmed.strip_prefix("[Новое]") {
            ReleaseNoteItem {
                category: NoteCategory::New,
                text: stripped.trim().to_string(),
                raw: raw_str,
            }
        } else if let Some(stripped) = trimmed.strip_prefix("[Улучшено]") {
            ReleaseNoteItem {
                category: NoteCategory::Improved,
                text: stripped.trim().to_string(),
                raw: raw_str,
            }
        } else if let Some(stripped) = trimmed.strip_prefix("[Исправлено]") {
            ReleaseNoteItem {
                category: NoteCategory::Fixed,
                text: stripped.trim().to_string(),
                raw: raw_str,
            }
        } else {
            ReleaseNoteItem {
                category: NoteCategory::General,
                text: trimmed.to_string(),
                raw: raw_str,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseNoteVersion {
    pub version: String,
    pub internal_version: String,
    pub date_text: String,
    pub short_title: String,
    pub is_current: bool,
    pub items: Vec<ReleaseNoteItem>,
}

/// Release notes shipped inside the launcher, shown until a newer
/// `version.json` replaces them. Keep this in sync with CHANGELOG.md:
/// only list what is actually in the release.
pub fn get_bundled_release_notes() -> Vec<ReleaseNoteVersion> {
    vec![
        ReleaseNoteVersion {
            version: format!("v{}", env!("CARGO_PKG_VERSION")),
            internal_version: env!("CARGO_PKG_VERSION").to_string(),
            date_text: "10 ноября 2026".to_string(),
            short_title: "Тихое обновление и правки интерфейса".to_string(),
            is_current: true,
            items: vec![
                ReleaseNoteItem::parse(
                    "[Улучшено] Обновление ставится без окна установщика, лаунчер открывается сам",
                ),
                ReleaseNoteItem::parse(
                    "[Улучшено] Сообщение об обновлении переехало в боковую панель",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Синяя рамка не остаётся на кнопке после клика мышью",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Кнопка «Играть» не выходит за край панели в маленьком окне",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Текст на карточках читается на любом фоне",
                ),
            ],
        },
        ReleaseNoteVersion {
            version: "v0.9.1".to_string(),
            internal_version: "0.9.1".to_string(),
            date_text: "10 ноября 2026".to_string(),
            short_title: "Исправления после первой версии".to_string(),
            is_current: false,
            items: vec![
                ReleaseNoteItem::parse(
                    "[Исправлено] Заставка больше не показывает «Сетевая ошибка» при запуске",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Мастер первой настройки не мигает между шагами",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Пинг сервера на главном экране показывается только после настоящей проверки",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Ник в нижней панели не обрезается, виджет музыки не выходит за край",
                ),
                ReleaseNoteItem::parse(
                    "[Улучшено] Список изменений на главном экране показывает настоящие изменения",
                ),
            ],
        },
        ReleaseNoteVersion {
            version: "v0.9.0".to_string(),
            internal_version: "0.9.0".to_string(),
            date_text: "10 ноября 2026".to_string(),
            short_title: "Первая версия Wex".to_string(),
            is_current: false,
            items: vec![
                ReleaseNoteItem::parse(
                    "[Новое] Minecraft 1.20.1 и 1.21.1 со своей Java для каждой версии",
                ),
                ReleaseNoteItem::parse("[Новое] Быстрый запуск: игра готовится в фоне заранее"),
                ReleaseNoteItem::parse("[Новое] Игра без интернета на уже скачанных версиях"),
                ReleaseNoteItem::parse("[Новое] Лобби с кодом и друзья в сети"),
                ReleaseNoteItem::parse(
                    "[Новое] Перенос ника, серверов и миров из старого лаунчера",
                ),
                ReleaseNoteItem::parse("[Новое] Автообновление лаунчера с проверкой подписи"),
            ],
        },
    ]
}

pub struct NewsManager {
    cache_path: PathBuf,
}

impl NewsManager {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_path: storage_dir.as_ref().join("news_cache.json"),
        }
    }

    pub fn cache_path(&self) -> &Path {
        &self.cache_path
    }

    pub fn load_cached_or_default(&self) -> Vec<ReleaseNoteVersion> {
        if self.cache_path.is_file() {
            if let Ok(text) = fs::read_to_string(&self.cache_path) {
                if let Ok(items) = serde_json::from_str::<Vec<ReleaseNoteVersion>>(&text) {
                    if !items.is_empty() {
                        return items;
                    }
                }
            }
        }
        get_bundled_release_notes()
    }

    pub fn save_cache(&self, versions: &[ReleaseNoteVersion]) -> Result<(), NewsError> {
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(versions)?;
        fs::write(&self.cache_path, json)?;
        Ok(())
    }

    pub fn update_from_version_json(
        &self,
        json_content: &str,
        current_version: &str,
    ) -> Result<Vec<ReleaseNoteVersion>, NewsError> {
        #[derive(Deserialize)]
        struct VersionJson {
            #[serde(default)]
            changelog: Vec<String>,
            #[serde(default, rename = "userFacingVersion")]
            user_facing_version: Option<String>,
            #[serde(default, rename = "internalVersion")]
            internal_version: Option<String>,
            #[serde(default, rename = "dateText")]
            date_text: Option<String>,
            #[serde(default, rename = "title")]
            title: Option<String>,
        }

        let raw: VersionJson = serde_json::from_str(json_content)?;
        let mut list = self.load_cached_or_default();

        let v_name = raw
            .user_facing_version
            .unwrap_or_else(|| current_version.to_string());
        let int_v = raw
            .internal_version
            .unwrap_or_else(|| current_version.to_string());
        let date_txt = raw
            .date_text
            .unwrap_or_else(|| Utc::now().format("%d %B %Y").to_string());
        let short_title = raw
            .title
            .unwrap_or_else(|| "Обновление лаунчера".to_string());

        let parsed_items: Vec<ReleaseNoteItem> = raw
            .changelog
            .into_iter()
            .map(ReleaseNoteItem::parse)
            .collect();

        // Mark previously current as false
        for item in &mut list {
            item.is_current = false;
        }

        // Insert or update current
        list.retain(|v| v.version != v_name);
        list.insert(
            0,
            ReleaseNoteVersion {
                version: v_name,
                internal_version: int_v,
                date_text: date_txt,
                short_title,
                is_current: true,
                items: parsed_items,
            },
        );

        self.save_cache(&list)?;
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_release_note_items() {
        let i1 = ReleaseNoteItem::parse("[Новое] Добавлена темная тема");
        assert_eq!(i1.category, NoteCategory::New);
        assert_eq!(i1.text, "Добавлена темная тема");

        let i2 = ReleaseNoteItem::parse("[Улучшено] Плавность прокрутки списков");
        assert_eq!(i2.category, NoteCategory::Improved);

        let i3 = ReleaseNoteItem::parse("[Исправлено] Сбой при выходе");
        assert_eq!(i3.category, NoteCategory::Fixed);

        let i4 = ReleaseNoteItem::parse("Просто текстовое примечание");
        assert_eq!(i4.category, NoteCategory::General);
    }

    #[test]
    fn test_news_manager_caching_and_version_json_update() {
        let dir = tempdir().unwrap();
        let mgr = NewsManager::new(dir.path());

        let initial = mgr.load_cached_or_default();
        assert!(!initial.is_empty());

        let version_json = r#"{
            "userFacingVersion": "v1.1.0",
            "internalVersion": "1.1.0",
            "title": "Большое обновление",
            "dateText": "8 ноября 2026",
            "changelog": [
                "[Новое] Поддержка Svelte 5",
                "[Улучшено] Оптимизация сети"
            ]
        }"#;

        let updated = mgr
            .update_from_version_json(version_json, "v1.1.0")
            .unwrap();

        assert_eq!(updated[0].version, "v1.1.0");
        assert!(updated[0].is_current);
        assert_eq!(updated[0].items.len(), 2);
        assert_eq!(updated[0].items[0].category, NoteCategory::New);

        // Verify cache persistence
        let reloaded = mgr.load_cached_or_default();
        assert_eq!(reloaded[0].version, "v1.1.0");
    }
}
