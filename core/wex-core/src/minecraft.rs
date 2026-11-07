use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::config::WexPaths;
use crate::download::{download_file_resumable, download_pool, DownloadTask, HashType};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};

pub const PISTON_VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const MINECRAFT_RESOURCES_BASE_URL: &str =
    "https://resources.download.minecraft.net";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PistonVersionManifest {
    pub versions: Vec<PistonVersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PistonVersionEntry {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadItem {
    pub sha1: String,
    pub size: u64,
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: DownloadItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryRule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<DownloadItem>,
    pub classifiers: Option<HashMap<String, DownloadItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<LibraryRule>>,
    pub natives: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionInfo {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionPackage {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub downloads: VersionDownloads,
    pub libraries: Vec<Library>,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
}

pub fn evaluates_rule_for_current_os(rules: &[LibraryRule]) -> bool {
    let current_os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };

    let mut allowed = false;

    for rule in rules {
        let os_match = if let Some(os) = &rule.os {
            if let Some(os_name) = &os.name {
                os_name.eq_ignore_ascii_case(current_os)
            } else {
                true
            }
        } else {
            true
        };

        if os_match {
            allowed = rule.action == "allow";
        }
    }

    allowed
}

pub fn is_library_allowed(lib: &Library) -> bool {
    if let Some(rules) = &lib.rules {
        evaluates_rule_for_current_os(rules)
    } else {
        true
    }
}

pub async fn fetch_version_package(client: &reqwest::Client, version: &str) -> Result<VersionPackage> {
    let manifest_resp = client
        .get(PISTON_VERSION_MANIFEST_URL)
        .send()
        .await?
        .error_for_status()?;
    let manifest: PistonVersionManifest = manifest_resp.json().await?;

    let entry = manifest
        .versions
        .into_iter()
        .find(|v| v.id == version)
        .ok_or_else(|| WexError::UnsupportedVersion(format!("Minecraft version {} not found in piston meta", version)))?;

    let pkg_resp = client.get(&entry.url).send().await?.error_for_status()?;
    let pkg: VersionPackage = pkg_resp.json().await?;
    Ok(pkg)
}

pub async fn install_minecraft_client(
    paths: &WexPaths,
    version: &str,
    pkg: &VersionPackage,
    client: &reqwest::Client,
    events: &dyn EventHandler,
) -> Result<PathBuf> {
    let ver_dir = paths.versions_dir.join(version);
    fs::create_dir_all(&ver_dir)?;

    let client_jar = ver_dir.join(format!("{}.jar", version));
    let client_download = &pkg.downloads.client;

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!("Downloading Minecraft {} client.jar...", version),
    });

    download_file_resumable(
        client,
        &client_download.url,
        &client_jar,
        Some(client_download.size),
        Some((&client_download.sha1, HashType::Sha1)),
        events,
    )
    .await?;

    Ok(client_jar)
}

pub async fn install_vanilla_libraries(
    paths: &WexPaths,
    version: &str,
    pkg: &VersionPackage,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<Vec<PathBuf>> {
    let mut classpath_jars = Vec::new();
    let mut download_tasks = Vec::new();
    let mut native_jars_to_extract = Vec::new();

    let natives_dir = paths.instance_natives_dir(version);
    fs::create_dir_all(&natives_dir)?;

    for lib in &pkg.libraries {
        if !is_library_allowed(lib) {
            continue;
        }

        if let Some(downloads) = &lib.downloads {
            // Artifact jar
            if let Some(artifact) = &downloads.artifact {
                if let Some(rel_path) = &artifact.path {
                    let target_path = paths.libraries_dir.join(rel_path);
                    classpath_jars.push(target_path.clone());

                    download_tasks.push(DownloadTask {
                        url: artifact.url.clone(),
                        target_path,
                        size: Some(artifact.size),
                        hash: Some((artifact.sha1.clone(), HashType::Sha1)),
                    });
                }
            }

            // Classifiers (natives)
            if let Some(classifiers) = &downloads.classifiers {
                let native_key = if cfg!(target_os = "windows") {
                    "natives-windows"
                } else if cfg!(target_os = "macos") {
                    "natives-macos"
                } else {
                    "natives-linux"
                };

                let native_item = classifiers
                    .get(native_key)
                    .or_else(|| classifiers.get("natives-windows-64"))
                    .or_else(|| classifiers.get("natives-windows-32"));

                if let Some(item) = native_item {
                    let rel_path = item
                        .path
                        .clone()
                        .unwrap_or_else(|| format!("natives/{}-{}.jar", lib.name.replace(':', "-"), native_key));
                    let target_path = paths.libraries_dir.join(rel_path);
                    native_jars_to_extract.push(target_path.clone());

                    download_tasks.push(DownloadTask {
                        url: item.url.clone(),
                        target_path,
                        size: Some(item.size),
                        hash: Some((item.sha1.clone(), HashType::Sha1)),
                    });
                }
            }
        }
    }

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Checking/downloading {} vanilla libraries for Minecraft {}...",
            download_tasks.len(),
            version
        ),
    });

    download_pool(client, download_tasks, concurrency, events, "Vanilla Libraries").await?;

    // Extract natives
    for native_jar in native_jars_to_extract {
        if native_jar.exists() {
            extract_native_dlls(&native_jar, &natives_dir)?;
        }
    }

    Ok(classpath_jars)
}

fn extract_native_dlls(jar_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if (name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib"))
            && !name.starts_with("META-INF/")
        {
            let file_name = match Path::new(&name).file_name() {
                Some(f) => f,
                None => continue,
            };
            let outpath = dest_dir.join(file_name);
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

pub async fn install_assets(
    paths: &WexPaths,
    pkg: &VersionPackage,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<()> {
    let index_file = paths
        .assets_indexes_dir()
        .join(format!("{}.json", pkg.asset_index.id));

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!("Downloading asset index {}...", pkg.asset_index.id),
    });

    download_file_resumable(
        client,
        &pkg.asset_index.url,
        &index_file,
        Some(pkg.asset_index.size),
        Some((&pkg.asset_index.sha1, HashType::Sha1)),
        events,
    )
    .await?;

    let index_content = fs::read_to_string(&index_file)?;
    let asset_index: AssetIndex = serde_json::from_str(&index_content)?;

    let mut download_tasks = Vec::new();
    for object in asset_index.objects.values() {
        let hash = &object.hash;
        if hash.len() < 2 {
            continue;
        }
        let prefix = &hash[..2];
        let target_path = paths.assets_objects_dir().join(prefix).join(hash);

        let url = format!("{}/{}/{}", MINECRAFT_RESOURCES_BASE_URL, prefix, hash);
        download_tasks.push(DownloadTask {
            url,
            target_path,
            size: Some(object.size),
            hash: Some((hash.clone(), HashType::Sha1)),
        });
    }

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Checking/downloading {} asset objects for index {}...",
            download_tasks.len(),
            pkg.asset_index.id
        ),
    });

    download_pool(client, download_tasks, concurrency, events, "Assets").await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_rule_evaluation() {
        let windows_rule = LibraryRule {
            action: "allow".to_string(),
            os: Some(OsRule {
                name: Some("windows".to_string()),
                arch: None,
            }),
        };
        let osx_rule = LibraryRule {
            action: "allow".to_string(),
            os: Some(OsRule {
                name: Some("osx".to_string()),
                arch: None,
            }),
        };

        if cfg!(target_os = "windows") {
            assert!(evaluates_rule_for_current_os(&[windows_rule]));
            assert!(!evaluates_rule_for_current_os(&[osx_rule]));
        }
    }
}
