use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::broadcast;

#[derive(Debug, Error)]
pub enum AchievementError {
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ошибка сериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub target: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AchievementProgress {
    pub id: String,
    pub current: f64,
    pub unlocked: bool,
    pub unlocked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementUnlockedEvent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub unlocked_at: String,
}

pub fn get_standard_achievements() -> Vec<AchievementDefinition> {
    vec![
        AchievementDefinition {
            id: "first_launch".to_string(),
            title: "Искра".to_string(),
            description: "Первый запуск игры".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "ten_launches".to_string(),
            title: "Постоялец".to_string(),
            description: "10 запусков игры".to_string(),
            target: 10,
        },
        AchievementDefinition {
            id: "hundred_hours".to_string(),
            title: "Сто часов".to_string(),
            description: "100 часов в игре".to_string(),
            target: 100,
        },
        AchievementDefinition {
            id: "marathon".to_string(),
            title: "Марафон".to_string(),
            description: "5 часов в одной сессии".to_string(),
            target: 5,
        },
        AchievementDefinition {
            id: "night_shift".to_string(),
            title: "Ночная смена".to_string(),
            description: "Запуск с 03:00 до 05:00".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "first_lobby".to_string(),
            title: "Хост".to_string(),
            description: "Первое созданное лобби".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "full_table".to_string(),
            title: "Полный стол".to_string(),
            description: "4 игрока в твоём лобби".to_string(),
            target: 4,
        },
        AchievementDefinition {
            id: "reconnected".to_string(),
            title: "Вернулся".to_string(),
            description: "Нажал «Переподключиться»".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "first_friend".to_string(),
            title: "Свой человек".to_string(),
            description: "Первый друг в списке".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "five_friends".to_string(),
            title: "Компания".to_string(),
            description: "5 друзей в списке".to_string(),
            target: 5,
        },
        AchievementDefinition {
            id: "first_backup".to_string(),
            title: "Страховка".to_string(),
            description: "Первый созданный бэкап мира".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "ten_backups".to_string(),
            title: "Параноик".to_string(),
            description: "10 бэкапов миров".to_string(),
            target: 10,
        },
        AchievementDefinition {
            id: "photographer".to_string(),
            title: "Фотограф".to_string(),
            description: "10 скриншотов в галерее".to_string(),
            target: 10,
        },
        AchievementDefinition {
            id: "own_style".to_string(),
            title: "Свой стиль".to_string(),
            description: "Установлен кастомный скин".to_string(),
            target: 1,
        },
        // Winter look (reported by the UI: SnowPlay, Santa, Snowballs).
        AchievementDefinition {
            id: "first_snowman".to_string(),
            title: "Первый снеговик".to_string(),
            description: "Слепил снеговика на плитке сборки".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "snowman_town".to_string(),
            title: "Снеговичный город".to_string(),
            description: "3 снеговика одновременно".to_string(),
            target: 3,
        },
        AchievementDefinition {
            id: "saw_santa".to_string(),
            title: "Видел Санту".to_string(),
            description: "Санта пролетел над лаунчером".to_string(),
            target: 1,
        },
        AchievementDefinition {
            id: "snowball_fight".to_string(),
            title: "Снежный бой".to_string(),
            description: "25 брошенных снежков".to_string(),
            target: 25,
        },
    ]
}

pub struct AchievementManager {
    storage_path: PathBuf,
    toast_tx: broadcast::Sender<AchievementUnlockedEvent>,
}

impl AchievementManager {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        let (toast_tx, _) = broadcast::channel(32);
        Self {
            storage_path: storage_dir.as_ref().join("achievements.json"),
            toast_tx,
        }
    }

    pub fn subscribe_toasts(&self) -> broadcast::Receiver<AchievementUnlockedEvent> {
        self.toast_tx.subscribe()
    }

    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    pub fn load_progress(&self) -> Result<HashMap<String, AchievementProgress>, AchievementError> {
        let defs = get_standard_achievements();
        let mut map = HashMap::new();

        for def in defs {
            map.insert(
                def.id.clone(),
                AchievementProgress {
                    id: def.id,
                    current: 0.0,
                    unlocked: false,
                    unlocked_at: None,
                },
            );
        }

        if self.storage_path.is_file() {
            let text = fs::read_to_string(&self.storage_path)?;
            if let Ok(saved) = serde_json::from_str::<HashMap<String, AchievementProgress>>(&text) {
                for (k, v) in saved {
                    if let Some(entry) = map.get_mut(&k) {
                        entry.current = v.current;
                        entry.unlocked = v.unlocked;
                        entry.unlocked_at = v.unlocked_at;
                    }
                }
            }
        }

        Ok(map)
    }

    pub fn save_progress(
        &self,
        progress: &HashMap<String, AchievementProgress>,
    ) -> Result<(), AchievementError> {
        if let Some(parent) = self.storage_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(progress)?;
        fs::write(&self.storage_path, json)?;
        Ok(())
    }

    pub fn set_progress(
        &self,
        id: &str,
        value: f64,
    ) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        let mut progress_map = self.load_progress()?;
        let defs = get_standard_achievements();
        let def = match defs.iter().find(|d| d.id == id) {
            Some(d) => d,
            None => return Ok(None),
        };

        let entry = match progress_map.get_mut(id) {
            Some(e) => e,
            None => return Ok(None),
        };

        if entry.unlocked {
            return Ok(None);
        }

        entry.current = value;
        if entry.current >= def.target as f64 {
            entry.unlocked = true;
            let now = Utc::now().to_rfc3339();
            entry.unlocked_at = Some(now.clone());

            let event = AchievementUnlockedEvent {
                id: def.id.clone(),
                title: def.title.clone(),
                description: def.description.clone(),
                unlocked_at: now,
            };

            let _ = self.toast_tx.send(event.clone());
            self.save_progress(&progress_map)?;
            return Ok(Some(event));
        }

        self.save_progress(&progress_map)?;
        Ok(None)
    }

    pub fn increment_progress(
        &self,
        id: &str,
        delta: f64,
    ) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        let current_val = self
            .load_progress()?
            .get(id)
            .map(|p| p.current)
            .unwrap_or(0.0);
        self.set_progress(id, current_val + delta)
    }

    // Trigger Helpers
    pub fn on_game_launch(
        &self,
        hour_of_day: u32,
    ) -> Result<Vec<AchievementUnlockedEvent>, AchievementError> {
        let mut unlocked = Vec::new();
        if let Some(e) = self.increment_progress("first_launch", 1.0)? {
            unlocked.push(e);
        }
        if let Some(e) = self.increment_progress("ten_launches", 1.0)? {
            unlocked.push(e);
        }
        if (3..5).contains(&hour_of_day) {
            if let Some(e) = self.set_progress("night_shift", 1.0)? {
                unlocked.push(e);
            }
        }
        Ok(unlocked)
    }

    pub fn on_session_ended(
        &self,
        session_hours: f64,
        total_hours: f64,
    ) -> Result<Vec<AchievementUnlockedEvent>, AchievementError> {
        let mut unlocked = Vec::new();
        if session_hours >= 5.0 {
            if let Some(e) = self.set_progress("marathon", session_hours)? {
                unlocked.push(e);
            }
        }
        if let Some(e) = self.set_progress("hundred_hours", total_hours)? {
            unlocked.push(e);
        }
        Ok(unlocked)
    }

    pub fn on_lobby_created(&self) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        self.set_progress("first_lobby", 1.0)
    }

    pub fn on_lobby_player_count(
        &self,
        count: usize,
    ) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        if count >= 4 {
            self.set_progress("full_table", count as f64)
        } else {
            Ok(None)
        }
    }

    pub fn on_reconnected(&self) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        self.set_progress("reconnected", 1.0)
    }

    pub fn on_friends_count(
        &self,
        count: usize,
    ) -> Result<Vec<AchievementUnlockedEvent>, AchievementError> {
        let mut unlocked = Vec::new();
        if count >= 1 {
            if let Some(e) = self.set_progress("first_friend", count as f64)? {
                unlocked.push(e);
            }
        }
        if count >= 5 {
            if let Some(e) = self.set_progress("five_friends", count as f64)? {
                unlocked.push(e);
            }
        }
        Ok(unlocked)
    }

    pub fn on_backup_created(
        &self,
        total_backups: usize,
    ) -> Result<Vec<AchievementUnlockedEvent>, AchievementError> {
        let mut unlocked = Vec::new();
        if total_backups >= 1 {
            if let Some(e) = self.set_progress("first_backup", total_backups as f64)? {
                unlocked.push(e);
            }
        }
        if total_backups >= 10 {
            if let Some(e) = self.set_progress("ten_backups", total_backups as f64)? {
                unlocked.push(e);
            }
        }
        Ok(unlocked)
    }

    pub fn on_screenshots_count(
        &self,
        total_screenshots: usize,
    ) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        if total_screenshots >= 10 {
            self.set_progress("photographer", total_screenshots as f64)
        } else {
            Ok(None)
        }
    }

    pub fn on_custom_skin_applied(
        &self,
    ) -> Result<Option<AchievementUnlockedEvent>, AchievementError> {
        self.set_progress("own_style", 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_achievement_triggers_and_toasts() {
        let dir = tempdir().unwrap();
        let mgr = AchievementManager::new(dir.path());
        let mut rx = mgr.subscribe_toasts();

        // 1. Launch 1 -> "first_launch"
        let u1 = mgr.on_game_launch(14).unwrap();
        assert_eq!(u1.len(), 1);
        assert_eq!(u1[0].id, "first_launch");
        assert_eq!(u1[0].title, "Искра");

        let toast = rx.recv().await.unwrap();
        assert_eq!(toast.id, "first_launch");

        // 2. Launch at 04:00 -> "night_shift"
        let u2 = mgr.on_game_launch(4).unwrap();
        assert!(u2.iter().any(|e| e.id == "night_shift"));

        // 3. Play time: 5.5 hours session, 102 hours total
        let u3 = mgr.on_session_ended(5.5, 102.0).unwrap();
        assert!(u3.iter().any(|e| e.id == "marathon"));
        assert!(u3.iter().any(|e| e.id == "hundred_hours"));

        // 4. Persistence verification
        let reloaded = mgr.load_progress().unwrap();
        assert!(reloaded["first_launch"].unlocked);
        assert!(reloaded["night_shift"].unlocked);
        assert!(reloaded["marathon"].unlocked);
        assert!(reloaded["hundred_hours"].unlocked);
    }

    #[test]
    fn winter_achievements_are_defined() {
        let defs = get_standard_achievements();
        let find = |id: &str| defs.iter().find(|d| d.id == id).cloned().unwrap();
        assert_eq!(find("first_snowman").title, "Первый снеговик");
        assert_eq!(find("first_snowman").target, 1);
        assert_eq!(find("snowman_town").title, "Снеговичный город");
        assert_eq!(find("snowman_town").target, 3);
        assert_eq!(find("saw_santa").title, "Видел Санту");
        assert_eq!(find("saw_santa").target, 1);
        assert_eq!(find("snowball_fight").title, "Снежный бой");
        assert_eq!(find("snowball_fight").target, 25);
        let mut ids: Vec<_> = defs.iter().map(|d| d.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), defs.len(), "achievement ids are unique");
    }

    #[test]
    fn snowball_fight_unlocks_after_25_throws() {
        let dir = tempdir().unwrap();
        let mgr = AchievementManager::new(dir.path());
        assert!(mgr
            .increment_progress("snowball_fight", 24.0)
            .unwrap()
            .is_none());
        let e = mgr
            .increment_progress("snowball_fight", 1.0)
            .unwrap()
            .unwrap();
        assert_eq!(e.id, "snowball_fight");
        assert!(mgr
            .increment_progress("snowball_fight", 1.0)
            .unwrap()
            .is_none());
        assert!(mgr.load_progress().unwrap()["snowball_fight"].unlocked);
    }
}
