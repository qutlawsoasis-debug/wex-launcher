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

pub fn get_bundled_release_notes() -> Vec<ReleaseNoteVersion> {
    vec![
        ReleaseNoteVersion {
            version: "v1.0.0".to_string(),
            internal_version: "1.0.0".to_string(),
            date_text: "7 ноября 2026".to_string(),
            short_title: "Новый Wex Лаунчер".to_string(),
            is_current: true,
            items: vec![
                ReleaseNoteItem::parse("[Новое] Полный переход на стек Tauri 2 + Rust + Svelte 5"),
                ReleaseNoteItem::parse("[Новое] Нативный Apple HIG и Liquid Glass дизайн системы"),
                ReleaseNoteItem::parse(
                    "[Новое] Изолированный безопасный импорт данных из старого лаунчера",
                ),
                ReleaseNoteItem::parse(
                    "[Улучшено] Мгновенный запуск и нулевое потребление памяти в фоне",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Полная ликвидация утечек и зависаний процессов",
                ),
            ],
        },
        ReleaseNoteVersion {
            version: "beta 1.0.80".to_string(),
            internal_version: "1.2.88".to_string(),
            date_text: "11 октября 2026".to_string(),
            short_title: "Уведомления без спама".to_string(),
            is_current: false,
            items: vec![
                ReleaseNoteItem::parse(
                    "[Исправлено] Спам уведомлениями об обновлении лаунчера в Windows",
                ),
                ReleaseNoteItem::parse(
                    "[Исправлено] Зависание старых уведомлений после завершения обновления",
                ),
            ],
        },
        ReleaseNoteVersion {
            version: "beta 1.0.79".to_string(),
            internal_version: "1.2.87".to_string(),
            date_text: "11 октября 2026".to_string(),
            short_title: "Миры друзей".to_string(),
            is_current: false,
            items: vec![
                ReleaseNoteItem::parse(
                    "[Улучшено] Миры в профиле теперь видны друзьям по умолчанию",
                ),
                ReleaseNoteItem::parse(
                    "[Улучшено] Управление видимостью отдельных одиночных миров в Настройках",
                ),
            ],
        },
        ReleaseNoteVersion {
            version: "beta 1.0.78".to_string(),
            internal_version: "1.2.86".to_string(),
            date_text: "11 октября 2026".to_string(),
            short_title: "Скриншоты и новости".to_string(),
            is_current: false,
            items: vec![
                ReleaseNoteItem::parse(
                    "[Улучшено] Отображение недавних скриншотов на главном экране",
                ),
                ReleaseNoteItem::parse(
                    "[Улучшено] Аккуратное расположение списка изменений в новостях",
                ),
            ],
        },
        ReleaseNoteVersion {
            version: "beta 1.0.77".to_string(),
            internal_version: "1.2.85".to_string(),
            date_text: "11 октября 2026".to_string(),
            short_title: "Профиль и приватность".to_string(),
            is_current: false,
            items: vec![
                ReleaseNoteItem::parse("[Исправлено] Сбой при запуске лаунчера после обновления"),
                ReleaseNoteItem::parse(
                    "[Новое] Профиль друга: просмотр статистики, достижений и миров",
                ),
                ReleaseNoteItem::parse("[Новое] Настройки приватности для отображения друзьям"),
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
