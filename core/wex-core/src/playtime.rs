use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Raw persistent store for player game time.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PlaytimeStore {
    pub total_seconds: u64,
    pub total_launches: u64,
    #[serde(default)]
    pub versions: HashMap<String, u64>,
    pub last_played_utc: Option<String>,
}

/// Formatted player-facing summary of play session history.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PlaytimeSummary {
    pub total_seconds: u64,
    pub total_formatted: String,
    pub current_version_seconds: u64,
    pub current_version_formatted: String,
    pub total_launches: u64,
    pub last_played_utc: Option<String>,
    pub summary_text: String,
}

impl PlaytimeStore {
    /// Load existing playtime data from JSON file or return a clean default.
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(store) = serde_json::from_str::<Self>(&data) {
                    return store;
                }
            }
        }
        Self::default()
    }

    /// Persist current playtime data to disk.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(path, data)
    }

    /// Record a completed game session duration for a specific version.
    pub fn record_session(&mut self, version: &str, elapsed_seconds: u64) {
        if elapsed_seconds > 0 {
            self.total_seconds = self.total_seconds.saturating_add(elapsed_seconds);
            let ver_entry = self.versions.entry(version.to_string()).or_default();
            *ver_entry = ver_entry.saturating_add(elapsed_seconds);
            self.total_launches = self.total_launches.saturating_add(1);
            self.last_played_utc = Some(chrono::Utc::now().to_rfc3339());
        }
    }

    /// Generate human-readable summary matching Apple HIG typography.
    pub fn get_summary(&self, current_version: Option<&str>) -> PlaytimeSummary {
        let cur_ver_seconds = current_version
            .and_then(|v| self.versions.get(v))
            .copied()
            .unwrap_or(0);

        let total_formatted = format_playtime(self.total_seconds);
        let cur_ver_formatted = format_playtime(cur_ver_seconds);
        let summary_text = if self.total_seconds == 0 && self.total_launches == 0 {
            "Время в игре: 0 мин. • 0 запусков".to_string()
        } else {
            format!(
                "В игре: {} • Запусков: {}",
                total_formatted, self.total_launches
            )
        };

        PlaytimeSummary {
            total_seconds: self.total_seconds,
            total_formatted,
            current_version_seconds: cur_ver_seconds,
            current_version_formatted: cur_ver_formatted,
            total_launches: self.total_launches,
            last_played_utc: self.last_played_utc.clone(),
            summary_text,
        }
    }
}

/// Format seconds into natural Russian localized string (e.g. "2 ч. 15 мин." or "45 мин.").
pub fn format_playtime(seconds: u64) -> String {
    if seconds == 0 {
        return "0 мин.".to_string();
    }
    if seconds < 60 {
        return "< 1 мин.".to_string();
    }
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        if minutes > 0 {
            format!("{hours} ч. {minutes} мин.")
        } else {
            format!("{hours} ч.")
        }
    } else {
        format!("{minutes} мин.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_format_playtime() {
        assert_eq!(format_playtime(0), "0 мин.");
        assert_eq!(format_playtime(45), "< 1 мин.");
        assert_eq!(format_playtime(60), "1 мин.");
        assert_eq!(format_playtime(150), "2 мин.");
        assert_eq!(format_playtime(3600), "1 ч.");
        assert_eq!(format_playtime(3660), "1 ч. 1 мин.");
        assert_eq!(format_playtime(7500), "2 ч. 5 мин.");
    }

    #[test]
    fn test_playtime_store_session_recording_and_persistence() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("playtime.json");

        let mut store = PlaytimeStore::load_or_default(&file);
        assert_eq!(store.total_seconds, 0);
        assert_eq!(store.total_launches, 0);

        // Record first session on 1.20.1 (30 mins = 1800s)
        store.record_session("1.20.1", 1800);
        assert_eq!(store.total_seconds, 1800);
        assert_eq!(store.total_launches, 1);
        assert_eq!(store.versions.get("1.20.1"), Some(&1800));

        // Record second session on 1.21.1 (1h 15m = 4500s)
        store.record_session("1.21.1", 4500);
        assert_eq!(store.total_seconds, 6300);
        assert_eq!(store.total_launches, 2);
        assert_eq!(store.versions.get("1.21.1"), Some(&4500));

        // Save and reload
        store.save(&file).unwrap();
        let loaded = PlaytimeStore::load_or_default(&file);
        assert_eq!(loaded.total_seconds, 6300);
        assert_eq!(loaded.total_launches, 2);
        assert_eq!(loaded.versions.get("1.20.1"), Some(&1800));
        assert_eq!(loaded.versions.get("1.21.1"), Some(&4500));

        // Verify summary
        let summary_120 = loaded.get_summary(Some("1.20.1"));
        assert_eq!(summary_120.total_formatted, "1 ч. 45 мин.");
        assert_eq!(summary_120.current_version_formatted, "30 мин.");
        assert_eq!(summary_120.total_launches, 2);
        assert_eq!(
            summary_120.summary_text,
            "В игре: 1 ч. 45 мин. • Запусков: 2"
        );

        let summary_121 = loaded.get_summary(Some("1.21.1"));
        assert_eq!(summary_121.current_version_formatted, "1 ч. 15 мин.");
    }
}
