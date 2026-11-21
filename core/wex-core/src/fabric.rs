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

/// Fabric profile for the dedicated server (main class `KnotServer`).
pub async fn fetch_fabric_server_profile(
    client: &reqwest::Client,
    mc_version: &str,
    loader_version: Option<&str>,
) -> Result<FabricProfile> {
    let loader = loader_version.unwrap_or(DEFAULT_FABRIC_LOADER);
    let url = format!("{}/{}/{}/server/json", FABRIC_META_BASE, mc_version, loader);
    let resp = client.get(&url).send().await?.error_for_status()?;
    Ok(resp.json().await?)
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

/// Fetch `<url>.sha1` and return the 40-hex digest. Fails closed.
async fn fetch_sha1_sidecar(client: &reqwest::Client, url: &str) -> Result<String> {
    let sidecar = format!("{}.sha1", url);
    let missing = |why: &str| {
        WexError::ManifestError(format!("No checksum for Fabric library {} ({})", url, why))
    };
    let resp = client
        .get(&sidecar)
        .send()
        .await
        .map_err(|e| missing(&e.to_string()))?
        .error_for_status()
        .map_err(|e| missing(&e.to_string()))?;
    let body = resp.text().await.map_err(|e| missing(&e.to_string()))?;
    let digest = body.split_whitespace().next().unwrap_or("");
    if digest.len() == 40 && digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(digest.to_ascii_lowercase())
    } else {
        Err(missing("malformed .sha1"))
    }
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
            (sha1.clone(), HashType::Sha1)
        } else if let Some(sha256) = &lib.sha256 {
            (sha256.clone(), HashType::Sha256)
        } else {
            // The profile carries no hash: Maven serves `<artifact>.sha1` next to
            // every jar. Without either source we refuse the library.
            (fetch_sha1_sidecar(client, &url).await?, HashType::Sha1)
        };
        let hash_opt = Some(hash_opt);

        download_tasks.push(DownloadTask {
            url,
            fallback_url: None,
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

    use crate::config::WexPaths;
    use crate::event::NoopEventHandler;
    use sha1::{Digest, Sha1};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn profile_for(server_uri: &str) -> FabricProfile {
        FabricProfile {
            id: "x".into(),
            main_class: "m".into(),
            libraries: vec![FabricLibrary {
                name: "a.b:lib:1.0".into(),
                url: Some(server_uri.to_string()),
                sha1: None,
                sha256: None,
                size: None,
            }],
        }
    }

    fn sha1_hex(b: &[u8]) -> String {
        let mut h = Sha1::new();
        h.update(b);
        h.finalize().iter().map(|x| format!("{:02x}", x)).collect()
    }

    #[tokio::test]
    async fn library_without_meta_hash_is_verified_by_sha1_sidecar() {
        let server = MockServer::start().await;
        let jar = b"real-jar-bytes".to_vec();
        Mock::given(method("GET"))
            .and(path("/a/b/lib/1.0/lib-1.0.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(jar.clone()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/a/b/lib/1.0/lib-1.0.jar.sha1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sha1_hex(&jar)))
            .mount(&server)
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let out = install_fabric_libraries(
            &paths,
            &profile_for(&server.uri()),
            &reqwest::Client::new(),
            &NoopEventHandler,
            2,
        )
        .await
        .unwrap();
        assert_eq!(std::fs::read(&out[0]).unwrap(), jar);
    }

    #[tokio::test]
    async fn library_with_tampered_jar_is_rejected_via_sha1_sidecar() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/a/b/lib/1.0/lib-1.0.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"evil".to_vec()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/a/b/lib/1.0/lib-1.0.jar.sha1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(sha1_hex(b"good")))
            .mount(&server)
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let res = install_fabric_libraries(
            &paths,
            &profile_for(&server.uri()),
            &reqwest::Client::new(),
            &NoopEventHandler,
            2,
        )
        .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn library_without_any_hash_fails_closed() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/a/b/lib/1.0/lib-1.0.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"jar".to_vec()))
            .mount(&server)
            .await;
        // .sha1 is not mounted: wiremock answers 404.
        let tmp = tempfile::tempdir().unwrap();
        let paths = WexPaths::new(Some(tmp.path().to_path_buf()));
        let res = install_fabric_libraries(
            &paths,
            &profile_for(&server.uri()),
            &reqwest::Client::new(),
            &NoopEventHandler,
            2,
        )
        .await;
        assert!(res.is_err());
        assert!(!paths.libraries_dir.join("a/b/lib/1.0/lib-1.0.jar").exists());
    }
}
