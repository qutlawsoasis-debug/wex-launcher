use crate::config::WexPaths;
use crate::download::{download_pool, DownloadTask, HashType};
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler, LogLevel};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_FABRIC_LOADER: &str = "0.19.5";
pub const FABRIC_META_BASE: &str = "https://meta.fabricmc.net/v2/versions/loader";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    pub url: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricProfile {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub libraries: Vec<FabricLibrary>,
}

pub fn maven_coordinate_to_path(coordinate: &str) -> Result<String> {
    let parts: Vec<&str> = coordinate.split(':').collect();
    if parts.len() < 3 {
        return Err(WexError::ManifestError(format!(
            "Invalid Maven coordinate: {}",
            coordinate
        )));
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];

    let (classifier, extension) = if parts.len() >= 4 {
        let last = parts[3];
        if let Some((cls, ext)) = last.split_once('@') {
            (Some(cls), ext)
        } else {
            (Some(last), "jar")
        }
    } else {
        (None, "jar")
    };

    let filename = if let Some(cls) = classifier {
        format!("{}-{}-{}.{}", artifact, version, cls, extension)
    } else {
        format!("{}-{}.{}", artifact, version, extension)
    };

    Ok(format!("{}/{}/{}/{}", group, artifact, version, filename))
}

pub async fn fetch_fabric_profile(
    client: &reqwest::Client,
    mc_version: &str,
    loader_version: Option<&str>,
) -> Result<FabricProfile> {
    let loader = loader_version.unwrap_or(DEFAULT_FABRIC_LOADER);
    let url = format!(
        "{}/{}/{}/profile/json",
        FABRIC_META_BASE, mc_version, loader
    );

    let resp = client.get(&url).send().await?.error_for_status()?;
    let profile: FabricProfile = resp.json().await?;
    Ok(profile)
}

pub fn resolve_fabric_libraries(paths: &WexPaths, profile: &FabricProfile) -> Vec<PathBuf> {
    let mut classpath_jars = Vec::new();
    for lib in &profile.libraries {
        if let Ok(rel_path) = maven_coordinate_to_path(&lib.name) {
            classpath_jars.push(paths.libraries_dir.join(rel_path));
        }
    }
    classpath_jars
}

pub async fn install_fabric_libraries(
    paths: &WexPaths,
    profile: &FabricProfile,
    client: &reqwest::Client,
    events: &dyn EventHandler,
    concurrency: usize,
) -> Result<Vec<PathBuf>> {
    let mut classpath_jars = Vec::new();
    let mut download_tasks = Vec::new();

    for lib in &profile.libraries {
        let rel_path = maven_coordinate_to_path(&lib.name)?;
        let target_path = paths.libraries_dir.join(&rel_path);
        classpath_jars.push(target_path.clone());

        let base_url = lib
            .url
            .as_deref()
            .unwrap_or("https://maven.fabricmc.net/")
            .trim_end_matches('/');

        let url = format!("{}/{}", base_url, rel_path);

        let hash_opt = if let Some(sha1) = &lib.sha1 {
            Some((sha1.clone(), HashType::Sha1))
        } else {
            lib.sha256
                .as_ref()
                .map(|sha256| (sha256.clone(), HashType::Sha256))
        };

        download_tasks.push(DownloadTask {
            url,
            target_path,
            size: lib.size,
            hash: hash_opt,
        });
    }

    events.on_event(&Event::Log {
        level: LogLevel::Info,
        message: format!(
            "Checking/downloading {} Fabric libraries (Loader {})...",
            download_tasks.len(),
            DEFAULT_FABRIC_LOADER
        ),
    });

    download_pool(
        client,
        download_tasks,
        concurrency,
        events,
        "Fabric Libraries",
    )
    .await?;

    Ok(classpath_jars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maven_coordinate_parsing() {
        let coord = "net.fabricmc:fabric-loader:0.19.5";
        let path = maven_coordinate_to_path(coord).unwrap();
        assert_eq!(
            path,
            "net/fabricmc/fabric-loader/0.19.5/fabric-loader-0.19.5.jar"
        );

        let coord_asm = "org.ow2.asm:asm:9.10.1";
        let path_asm = maven_coordinate_to_path(coord_asm).unwrap();
        assert_eq!(path_asm, "org/ow2/asm/asm/9.10.1/asm-9.10.1.jar");
    }
}
