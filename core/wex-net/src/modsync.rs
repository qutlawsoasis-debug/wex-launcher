use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// An entry in a player's or host's mod manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModManifestEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

/// Category of mod mismatch between host and guest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModMismatchType {
    /// Mod is installed on client, but disabled (`.jar.disabled`)
    Disabled,
    /// Mod is present on host, but missing on client
    Missing,
    /// Mod is present on client, but absent on host
    Extra,
    /// Different versions of the same mod
    Version {
        player_version: String,
        host_version: String,
    },
}

/// A specific mismatch item reported for UI and synchronization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModMismatchItem {
    pub mod_id: String,
    pub mod_name: String,
    #[serde(rename = "type")]
    pub mismatch_type: ModMismatchType,
    pub type_ru: String,
    pub is_fixable: bool,
    pub player_version: Option<String>,
    pub host_version: Option<String>,
    pub display_description: String,
}

/// Computes deterministic 16-character SHA-256 hash of a mod manifest
/// matching the Aura Launcher algorithm.
pub fn compute_manifest_hash(manifest: &[ModManifestEntry]) -> String {
    if manifest.is_empty() {
        return String::new();
    }

    let mut sorted: Vec<&ModManifestEntry> = manifest.iter().collect();
    sorted.sort_by_key(|a| a.id.to_lowercase());

    let mut hasher = Sha256::new();
    for entry in sorted {
        let line = format!(
            "{}:{}:{}\n",
            entry.id.to_lowercase(),
            entry.version,
            if entry.enabled { '1' } else { '0' }
        );
        hasher.update(line.as_bytes());
    }

    let full_hash = hex::encode(hasher.finalize());
    full_hash[..16.min(full_hash.len())].to_string()
}

/// Compares client manifest against host manifest and returns all mismatches.
pub fn compare_manifests(
    client_manifest: &[ModManifestEntry],
    host_manifest: &[ModManifestEntry],
) -> Vec<ModMismatchItem> {
    let mut mismatches = Vec::new();

    let client_map: HashMap<String, &ModManifestEntry> = client_manifest
        .iter()
        .map(|m| (m.id.to_lowercase(), m))
        .collect();

    let host_map: HashMap<String, &ModManifestEntry> = host_manifest
        .iter()
        .map(|m| (m.id.to_lowercase(), m))
        .collect();

    // 1. Check mods required by host
    for host_mod in host_manifest {
        // We only care about enabled mods on the host
        if !host_mod.enabled {
            continue;
        }

        let key = host_mod.id.to_lowercase();
        match client_map.get(&key) {
            None => {
                // Completely missing on client
                mismatches.push(ModMismatchItem {
                    mod_id: host_mod.id.clone(),
                    mod_name: host_mod.name.clone(),
                    mismatch_type: ModMismatchType::Missing,
                    type_ru: "отсутствует".to_string(),
                    is_fixable: false,
                    player_version: None,
                    host_version: Some(host_mod.version.clone()),
                    display_description: "нет у вас".to_string(),
                });
            }
            Some(client_mod) => {
                if !client_mod.enabled {
                    // Present but disabled on client
                    mismatches.push(ModMismatchItem {
                        mod_id: host_mod.id.clone(),
                        mod_name: host_mod.name.clone(),
                        mismatch_type: ModMismatchType::Disabled,
                        type_ru: "выключен".to_string(),
                        is_fixable: true,
                        player_version: Some(client_mod.version.clone()),
                        host_version: Some(host_mod.version.clone()),
                        display_description: "выключен у вас".to_string(),
                    });
                } else if client_mod.version != host_mod.version {
                    // Version mismatch
                    mismatches.push(ModMismatchItem {
                        mod_id: host_mod.id.clone(),
                        mod_name: host_mod.name.clone(),
                        mismatch_type: ModMismatchType::Version {
                            player_version: client_mod.version.clone(),
                            host_version: host_mod.version.clone(),
                        },
                        type_ru: "другая версия".to_string(),
                        is_fixable: false,
                        player_version: Some(client_mod.version.clone()),
                        host_version: Some(host_mod.version.clone()),
                        display_description: format!(
                            "версия {} у вас, {} у хоста",
                            client_mod.version, host_mod.version
                        ),
                    });
                }
            }
        }
    }

    // 2. Check extra enabled mods present on client but not on host
    for client_mod in client_manifest {
        if !client_mod.enabled {
            continue;
        }

        let key = client_mod.id.to_lowercase();
        if !host_map.contains_key(&key) {
            mismatches.push(ModMismatchItem {
                mod_id: client_mod.id.clone(),
                mod_name: client_mod.name.clone(),
                mismatch_type: ModMismatchType::Extra,
                type_ru: "лишний".to_string(),
                is_fixable: true,
                player_version: Some(client_mod.version.clone()),
                host_version: None,
                display_description: "лишний у вас".to_string(),
            });
        }
    }

    mismatches.sort_by_key(|a| a.mod_id.to_lowercase());
    mismatches
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_manifest_hash_consistency() {
        let manifest = vec![
            ModManifestEntry {
                id: "sodium".to_string(),
                name: "Sodium".to_string(),
                version: "0.5.11".to_string(),
                enabled: true,
            },
            ModManifestEntry {
                id: "iris".to_string(),
                name: "Iris".to_string(),
                version: "1.7.0".to_string(),
                enabled: true,
            },
        ];

        let hash1 = compute_manifest_hash(&manifest);
        assert_eq!(hash1.len(), 16);

        // Reverse order must yield the exact same hash
        let mut reversed = manifest.clone();
        reversed.reverse();
        let hash2 = compute_manifest_hash(&reversed);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compare_manifests_all_mismatch_types() {
        let host_manifest = vec![
            ModManifestEntry {
                id: "sodium".to_string(),
                name: "Sodium".to_string(),
                version: "0.5.11".to_string(),
                enabled: true,
            },
            ModManifestEntry {
                id: "iris".to_string(),
                name: "Iris".to_string(),
                version: "1.7.0".to_string(),
                enabled: true,
            },
            ModManifestEntry {
                id: "appleskin".to_string(),
                name: "AppleSkin".to_string(),
                version: "2.5.1".to_string(),
                enabled: true,
            },
            ModManifestEntry {
                id: "voicechat".to_string(),
                name: "Simple Voice Chat".to_string(),
                version: "2.4.0".to_string(),
                enabled: true,
            },
        ];

        let client_manifest = vec![
            ModManifestEntry {
                id: "sodium".to_string(),
                name: "Sodium".to_string(),
                version: "0.5.11".to_string(),
                enabled: true, // OK: matched
            },
            ModManifestEntry {
                id: "iris".to_string(),
                name: "Iris".to_string(),
                version: "1.7.0".to_string(),
                enabled: false, // Disabled!
            },
            ModManifestEntry {
                id: "appleskin".to_string(),
                name: "AppleSkin".to_string(),
                version: "2.4.0".to_string(), // Version mismatch!
                enabled: true,
            },
            // voicechat is completely missing on client!
            ModManifestEntry {
                id: "cheatmod".to_string(),
                name: "Cheat Mod".to_string(),
                version: "1.0.0".to_string(),
                enabled: true, // Extra mod!
            },
        ];

        let mismatches = compare_manifests(&client_manifest, &host_manifest);
        assert_eq!(mismatches.len(), 4);

        // 1. appleskin -> Version
        let mm_version = mismatches.iter().find(|m| m.mod_id == "appleskin").unwrap();
        assert_eq!(
            mm_version.mismatch_type,
            ModMismatchType::Version {
                player_version: "2.4.0".to_string(),
                host_version: "2.5.1".to_string(),
            }
        );
        assert!(!mm_version.is_fixable);

        // 2. cheatmod -> Extra
        let mm_extra = mismatches.iter().find(|m| m.mod_id == "cheatmod").unwrap();
        assert_eq!(mm_extra.mismatch_type, ModMismatchType::Extra);
        assert!(mm_extra.is_fixable);

        // 3. iris -> Disabled
        let mm_disabled = mismatches.iter().find(|m| m.mod_id == "iris").unwrap();
        assert_eq!(mm_disabled.mismatch_type, ModMismatchType::Disabled);
        assert!(mm_disabled.is_fixable);

        // 4. voicechat -> Missing
        let mm_missing = mismatches.iter().find(|m| m.mod_id == "voicechat").unwrap();
        assert_eq!(mm_missing.mismatch_type, ModMismatchType::Missing);
        assert!(!mm_missing.is_fixable);
    }
}
