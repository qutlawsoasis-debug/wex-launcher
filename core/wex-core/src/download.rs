use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler};
use futures_util::StreamExt;
use reqwest::header::RANGE;
use sha1::Digest as _;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashType {
    Sha1,
    Sha256,
}

pub fn compute_file_hash(path: &Path, hash_type: HashType) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 65536];

    match hash_type {
        HashType::Sha1 => {
            let mut hasher = sha1::Sha1::new();
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            Ok(hex::encode(hasher.finalize()))
        }
        HashType::Sha256 => {
            let mut hasher = sha2::Sha256::new();
            loop {
                let n = file.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            Ok(hex::encode(hasher.finalize()))
        }
    }
}

pub fn is_disk_full_error(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::StorageFull {
        return true;
    }
    match err.raw_os_error() {
        #[cfg(windows)]
        Some(112) | Some(39) => true,
        #[cfg(not(windows))]
        Some(28) => true,
        _ => false,
    }
}

pub fn map_io_err(err: std::io::Error, path: &Path) -> WexError {
    if is_disk_full_error(&err) {
        WexError::DiskFull {
            path: path.to_path_buf(),
            source: err,
        }
    } else {
        WexError::Io(err)
    }
}

pub fn verify_file(
    path: &Path,
    expected_size: Option<u64>,
    expected_hash: Option<(&str, HashType)>,
) -> bool {
    if !path.exists() {
        return false;
    }

    if let Ok(meta) = fs::metadata(path) {
        if let Some(expected_len) = expected_size {
            if meta.len() != expected_len {
                return false;
            }
        }

        if let Some((hash_str, hash_type)) = expected_hash {
            if let Ok(actual_hash) = compute_file_hash(path, hash_type) {
                return actual_hash.eq_ignore_ascii_case(hash_str);
            } else {
                return false;
            }
        }
        true
    } else {
        false
    }
}

use tokio_util::sync::CancellationToken;

pub async fn download_file_resumable(
    client: &reqwest::Client,
    url: &str,
    target_path: &Path,
    expected_size: Option<u64>,
    expected_hash: Option<(&str, HashType)>,
    events: &dyn EventHandler,
) -> Result<bool> {
    download_file_resumable_cancellable(
        client,
        url,
        target_path,
        expected_size,
        expected_hash,
        events,
        None,
    )
    .await
}

pub async fn download_file_resumable_cancellable(
    client: &reqwest::Client,
    url: &str,
    target_path: &Path,
    expected_size: Option<u64>,
    expected_hash: Option<(&str, HashType)>,
    events: &dyn EventHandler,
    cancel_token: Option<&CancellationToken>,
) -> Result<bool> {
    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            return Err(WexError::Cancelled);
        }
    }

    if verify_file(target_path, expected_size, expected_hash) {
        return Ok(false);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| map_io_err(e, parent))?;
    }

    let part_path = PathBuf::from(format!("{}.part", target_path.display()));
    let mut existing_len = 0u64;

    if part_path.exists() {
        if let Ok(meta) = fs::metadata(&part_path) {
            existing_len = meta.len();
            if let Some(exp_size) = expected_size {
                if existing_len >= exp_size {
                    let _ = fs::remove_file(&part_path);
                    existing_len = 0;
                }
            }
        }
    }

    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 3;

    loop {
        attempts += 1;
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                let _ = fs::remove_file(&part_path);
                return Err(WexError::Cancelled);
            }
        }

        let mut req = client.get(url);
        let resuming = existing_len > 0;
        if resuming {
            req = req.header(RANGE, format!("bytes={}-", existing_len));
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                if attempts >= MAX_ATTEMPTS {
                    return Err(WexError::DownloadError {
                        url: url.to_string(),
                        message: format!("HTTP request failed after {} attempts: {}", attempts, e),
                    });
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts as u64)).await;
                continue;
            }
        };

        let status = resp.status();
        // If range was requested but server returned 416 (Range Not Satisfiable), restart from byte 0
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            let _ = fs::remove_file(&part_path);
            existing_len = 0;
            if attempts >= MAX_ATTEMPTS {
                return Err(WexError::DownloadError {
                    url: url.to_string(),
                    message: format!("HTTP 416 Range Not Satisfiable after {} attempts", attempts),
                });
            }
            continue;
        }

        // If client permanent error 404 (Not Found) or 410 (Gone): fail immediately without sleeping/retrying
        if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE {
            let _ = fs::remove_file(&part_path);
            return Err(WexError::DownloadError {
                url: url.to_string(),
                message: format!("HTTP error status: {}", status),
            });
        }

        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            if attempts >= MAX_ATTEMPTS {
                let _ = fs::remove_file(&part_path);
                return Err(WexError::DownloadError {
                    url: url.to_string(),
                    message: format!("HTTP error status: {}", status),
                });
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts as u64)).await;
            continue;
        }

        let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut file = if is_partial && resuming {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part_path)
                .map_err(|e| {
                    let _ = fs::remove_file(&part_path);
                    map_io_err(e, &part_path)
                })?
        } else {
            existing_len = 0;
            File::create(&part_path).map_err(|e| {
                let _ = fs::remove_file(&part_path);
                map_io_err(e, &part_path)
            })?
        };

        let mut stream = resp.bytes_stream();
        let mut downloaded_bytes = existing_len;
        let mut stream_interrupted = false;

        while let Some(chunk_res) = stream.next().await {
            if let Some(token) = cancel_token {
                if token.is_cancelled() {
                    drop(file);
                    let _ = fs::remove_file(&part_path);
                    return Err(WexError::Cancelled);
                }
            }
            match chunk_res {
                Ok(chunk) => {
                    if let Err(e) = file.write_all(&chunk) {
                        drop(file);
                        let _ = fs::remove_file(&part_path);
                        return Err(map_io_err(e, &part_path));
                    }
                    downloaded_bytes += chunk.len() as u64;
                }
                Err(e) => {
                    events.on_event(&Event::Log {
                        level: crate::event::LogLevel::Warn,
                        message: format!("Stream interrupted for {}: {}", url, e),
                    });
                    stream_interrupted = true;
                    existing_len = downloaded_bytes;
                    break;
                }
            }
        }

        if let Err(e) = file.flush() {
            drop(file);
            let _ = fs::remove_file(&part_path);
            return Err(map_io_err(e, &part_path));
        }
        drop(file);

        if stream_interrupted {
            if attempts >= MAX_ATTEMPTS {
                let _ = fs::remove_file(&part_path);
                return Err(WexError::DownloadError {
                    url: url.to_string(),
                    message: format!("Stream error after {} attempts", attempts),
                });
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts as u64)).await;
            continue;
        }

        // Verification of file size
        let actual_len = match fs::metadata(&part_path) {
            Ok(m) => m.len(),
            Err(e) => {
                let _ = fs::remove_file(&part_path);
                return Err(map_io_err(e, &part_path));
            }
        };

        if let Some(exp_size) = expected_size {
            if actual_len != exp_size {
                if actual_len < exp_size {
                    // Stream was cut off without explicit error: save downloaded length for resume
                    existing_len = actual_len;
                } else {
                    // Oversized file: purge and restart from byte 0
                    let _ = fs::remove_file(&part_path);
                    existing_len = 0;
                }
                if attempts >= MAX_ATTEMPTS {
                    let _ = fs::remove_file(&part_path);
                    return Err(WexError::SizeMismatch {
                        path: target_path.to_path_buf(),
                        expected: exp_size,
                        actual: actual_len,
                    });
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts as u64)).await;
                continue;
            }
        }

        // Verification of hash
        if let Some((exp_hash, hash_type)) = expected_hash {
            let actual_hash = compute_file_hash(&part_path, hash_type)?;
            if !actual_hash.eq_ignore_ascii_case(exp_hash) {
                events.on_event(&Event::Log {
                    level: crate::event::LogLevel::Warn,
                    message: format!(
                        "Hash mismatch for {} (expected {}, got {}), retrying...",
                        target_path.display(),
                        exp_hash,
                        actual_hash
                    ),
                });
                let _ = fs::remove_file(&part_path);
                if attempts >= MAX_ATTEMPTS {
                    return Err(WexError::HashMismatch {
                        path: target_path.to_path_buf(),
                        expected: exp_hash.to_string(),
                        actual: actual_hash,
                    });
                }
                existing_len = 0;
                tokio::time::sleep(tokio::time::Duration::from_millis(50 * attempts as u64)).await;
                continue;
            }
        }

        // Atomically replace target file
        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
        if let Err(e) = fs::rename(&part_path, target_path) {
            let _ = fs::remove_file(&part_path);
            return Err(map_io_err(e, target_path));
        }

        let final_len = fs::metadata(target_path)
            .map(|m| m.len())
            .unwrap_or(actual_len);

        events.on_event(&Event::FileDownloaded {
            path: target_path.display().to_string(),
            bytes: final_len,
        });

        return Ok(true);
    }
}

#[derive(Debug, Clone)]
pub struct DownloadTask {
    pub url: String,
    pub fallback_url: Option<String>,
    pub target_path: PathBuf,
    pub size: Option<u64>,
    pub hash: Option<(String, HashType)>,
}

#[allow(clippy::too_many_arguments)]
pub async fn download_file_with_fallback(
    client: &reqwest::Client,
    url: &str,
    fallback_url: Option<&str>,
    target_path: &Path,
    expected_size: Option<u64>,
    expected_hash: Option<(&str, HashType)>,
    events: &dyn EventHandler,
    cancel_token: Option<&CancellationToken>,
) -> Result<bool> {
    match download_file_resumable_cancellable(
        client,
        url,
        target_path,
        expected_size,
        expected_hash,
        events,
        cancel_token,
    )
    .await
    {
        Ok(res) => Ok(res),
        Err(primary_err) => {
            if matches!(primary_err, WexError::Cancelled) {
                return Err(primary_err);
            }
            if let Some(fallback) = fallback_url {
                events.on_event(&Event::Log {
                    level: crate::event::LogLevel::Warn,
                    message: format!(
                        "Download failed from primary URL {}, falling back to {}: {}",
                        url, fallback, primary_err
                    ),
                });
                match download_file_resumable_cancellable(
                    client,
                    fallback,
                    target_path,
                    expected_size,
                    expected_hash,
                    events,
                    cancel_token,
                )
                .await
                {
                    Ok(res) => Ok(res),
                    Err(fallback_err) => Err(WexError::DownloadError {
                        url: url.to_string(),
                        message: format!(
                            "Primary URL failed ({}), and fallback URL ({}) failed: {}",
                            primary_err, fallback, fallback_err
                        ),
                    }),
                }
            } else {
                Err(primary_err)
            }
        }
    }
}

pub async fn download_pool(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    events: &dyn EventHandler,
    stage_name: &str,
) -> Result<u64> {
    download_pool_cancellable(client, tasks, concurrency, events, stage_name, None).await
}

pub async fn download_pool_cancellable(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    concurrency: usize,
    events: &dyn EventHandler,
    stage_name: &str,
    cancel_token: Option<CancellationToken>,
) -> Result<u64> {
    let total_tasks = tasks.len();
    if total_tasks == 0 {
        return Ok(0);
    }

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let total_bytes_downloaded = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let mut join_set = tokio::task::JoinSet::new();

    for task in tasks {
        let sem = semaphore.clone();
        let client = client.clone();
        let total_bytes = total_bytes_downloaded.clone();
        let token_clone = cancel_token.clone();

        join_set.spawn(async move {
            if let Some(t) = &token_clone {
                if t.is_cancelled() {
                    return Err(WexError::Cancelled);
                }
            }

            let _permit = sem.acquire().await.map_err(|e| WexError::DownloadError {
                url: task.url.clone(),
                message: e.to_string(),
            })?;

            if let Some(t) = &token_clone {
                if t.is_cancelled() {
                    return Err(WexError::Cancelled);
                }
            }

            let expected_hash = task.hash.as_ref().map(|(h, t)| (h.as_str(), *t));
            let noop = crate::event::NoopEventHandler;
            let res = download_file_with_fallback(
                &client,
                &task.url,
                task.fallback_url.as_deref(),
                &task.target_path,
                task.size,
                expected_hash,
                &noop,
                token_clone.as_ref(),
            )
            .await;

            if let Ok(downloaded) = &res {
                if *downloaded {
                    if let Some(s) = task.size {
                        total_bytes.fetch_add(s, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }

            res
        });
    }

    let mut completed: usize = 0;
    let progress_step = (total_tasks / 10).max(1);

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(inner_res) => {
                inner_res?;
                completed += 1;
                if completed.is_multiple_of(progress_step) || completed == total_tasks {
                    events.on_event(&Event::Progress {
                        stage: stage_name.to_string(),
                        current: completed as u64,
                        total: total_tasks as u64,
                        message: None,
                    });
                }
            }
            Err(e) => {
                return Err(WexError::DownloadError {
                    url: "task_join".to_string(),
                    message: e.to_string(),
                });
            }
        }
    }

    Ok(total_bytes_downloaded.load(std::sync::atomic::Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_hash_calculation() {
        let tmp = tempdir().unwrap();
        let file_path = tmp.path().join("test.txt");
        fs::write(&file_path, "Hello Aura!").unwrap();

        let sha1 = compute_file_hash(&file_path, HashType::Sha1).unwrap();
        let sha256 = compute_file_hash(&file_path, HashType::Sha256).unwrap();

        assert!(!sha1.is_empty());
        assert!(!sha256.is_empty());
        assert!(verify_file(
            &file_path,
            Some(11),
            Some((&sha1, HashType::Sha1))
        ));
        assert!(verify_file(
            &file_path,
            Some(11),
            Some((&sha256, HashType::Sha256))
        ));
    }

    #[tokio::test]
    async fn test_cancellation_token_cancels_download() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("file.bin");
        let token = CancellationToken::new();
        token.cancel(); // Cancel immediately

        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;

        let res = download_file_resumable_cancellable(
            &client,
            "http://127.0.0.1:9999/nonexistent",
            &target,
            None,
            None,
            &events,
            Some(&token),
        )
        .await;

        assert!(matches!(res, Err(WexError::Cancelled)));
    }

    #[tokio::test]
    async fn test_download_resume_partial_stream_interrupted() {
        use sha2::Digest;
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let full_payload = b"ABCDEFGH1234567890"; // 18 bytes total
        let mut hasher = sha2::Sha256::new();
        hasher.update(full_payload);
        let expected_hash = hex::encode(hasher.finalize());

        // Range response for bytes=8- (remaining 10 bytes: 1234567890)
        Mock::given(method("GET"))
            .and(path("/resume-file.bin"))
            .and(header("Range", "bytes=8-"))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(b"1234567890".to_vec()))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let target = tmp.path().join("file.bin");
        let part_file = tmp.path().join("file.bin.part");
        // Pre-create .part file with first 8 bytes
        fs::write(&part_file, b"ABCDEFGH").unwrap();

        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;
        let url = format!("{}/resume-file.bin", mock_server.uri());

        let res = download_file_resumable_cancellable(
            &client,
            &url,
            &target,
            Some(18),
            Some((&expected_hash, HashType::Sha256)),
            &events,
            None,
        )
        .await;

        assert!(res.is_ok(), "Resumed download must succeed: {:?}", res);
        assert!(target.exists(), "Target file must be written");
        assert!(
            !part_file.exists(),
            "Part file must be removed after finish"
        );
        let disk_data = fs::read(&target).unwrap();
        assert_eq!(
            disk_data, full_payload,
            "Target file content must match full payload"
        );
    }

    #[tokio::test]
    async fn test_download_fallback_on_404_and_500() {
        use sha2::Digest;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let payload = b"resilient fallback data";
        let mut hasher = sha2::Sha256::new();
        hasher.update(payload);
        let expected_hash = hex::encode(hasher.finalize());

        // 1. Primary 404, fallback 200
        Mock::given(method("GET"))
            .and(path("/bad_404.bin"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/good_fallback_1.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.to_vec()))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let target1 = tmp.path().join("file1.bin");
        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;

        let res1 = download_file_with_fallback(
            &client,
            &format!("{}/bad_404.bin", mock_server.uri()),
            Some(&format!("{}/good_fallback_1.bin", mock_server.uri())),
            &target1,
            Some(payload.len() as u64),
            Some((&expected_hash, HashType::Sha256)),
            &events,
            None,
        )
        .await;
        assert!(res1.is_ok(), "Fallback from 404 must succeed: {:?}", res1);
        assert_eq!(fs::read(&target1).unwrap(), payload);

        // 2. Primary 500, fallback 200
        Mock::given(method("GET"))
            .and(path("/bad_500.bin"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/good_fallback_2.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.to_vec()))
            .mount(&mock_server)
            .await;

        let target2 = tmp.path().join("file2.bin");
        let res2 = download_file_with_fallback(
            &client,
            &format!("{}/bad_500.bin", mock_server.uri()),
            Some(&format!("{}/good_fallback_2.bin", mock_server.uri())),
            &target2,
            Some(payload.len() as u64),
            Some((&expected_hash, HashType::Sha256)),
            &events,
            None,
        )
        .await;
        assert!(res2.is_ok(), "Fallback from 500 must succeed: {:?}", res2);
        assert_eq!(fs::read(&target2).unwrap(), payload);

        // 3. Both 500: returns clear error
        Mock::given(method("GET"))
            .and(path("/both_bad_1.bin"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/both_bad_2.bin"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let target3 = tmp.path().join("file3.bin");
        let res3 = download_file_with_fallback(
            &client,
            &format!("{}/both_bad_1.bin", mock_server.uri()),
            Some(&format!("{}/both_bad_2.bin", mock_server.uri())),
            &target3,
            Some(payload.len() as u64),
            Some((&expected_hash, HashType::Sha256)),
            &events,
            None,
        )
        .await;
        assert!(res3.is_err(), "Must return error when both URLs fail");
        let err_str = res3.unwrap_err().to_string();
        assert!(err_str.contains("both_bad_1.bin") && err_str.contains("both_bad_2.bin"));
    }

    #[tokio::test]
    async fn test_download_hash_mismatch_preserves_installed_file() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let corrupted_payload = b"this content is completely corrupted";

        Mock::given(method("GET"))
            .and(path("/corrupt.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(corrupted_payload.to_vec()))
            .mount(&mock_server)
            .await;

        let tmp = tempdir().unwrap();
        let target = tmp.path().join("installed_mod.jar");
        let original_data = b"already installed valid mod content v1.0";
        fs::write(&target, original_data).unwrap();

        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;
        let url = format!("{}/corrupt.bin", mock_server.uri());

        let res = download_file_resumable_cancellable(
            &client,
            &url,
            &target,
            Some(corrupted_payload.len() as u64),
            Some(("expected_sha256_that_does_not_match", HashType::Sha256)),
            &events,
            None,
        )
        .await;

        assert!(matches!(res, Err(WexError::HashMismatch { .. })));
        assert!(target.exists(), "Original target file must NOT be deleted");
        assert_eq!(
            fs::read(&target).unwrap(),
            original_data,
            "Existing installed file must NOT be corrupted by bad download"
        );
        let part_file = tmp.path().join("installed_mod.jar.part");
        assert!(
            !part_file.exists(),
            "Corrupted part file must be cleaned up"
        );
    }

    #[test]
    fn test_disk_full_error_handling_and_preservation() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("vital_mod.jar");
        let original_data = b"safe installed mod content";
        fs::write(&target, original_data).unwrap();

        // 1. Verify detection of disk full
        #[cfg(windows)]
        let disk_err = std::io::Error::from_raw_os_error(112); // ERROR_DISK_FULL
        #[cfg(not(windows))]
        let disk_err = std::io::Error::from_raw_os_error(28); // ENOSPC

        assert!(is_disk_full_error(&disk_err));

        let wex_err = map_io_err(disk_err, &target);
        assert!(matches!(wex_err, WexError::DiskFull { .. }));

        // 2. Verify conversion to clear IpcError
        let ipc_err: wex_ipc::error::IpcError = wex_err.into();
        assert_eq!(ipc_err.code, "IO_DISK_FULL");
        assert_eq!(
            ipc_err.message,
            "На диске закончилось свободное место. Освободите место на диске и повторите попытку."
        );

        // 3. Verify installed file was never touched
        assert_eq!(fs::read(&target).unwrap(), original_data);
    }

    #[tokio::test]
    async fn test_cancellation_preserves_installed_file() {
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("my_mod.jar");
        let original_data = b"user installed mod content";
        fs::write(&target, original_data).unwrap();

        let token = CancellationToken::new();
        token.cancel();

        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;

        let res = download_file_resumable_cancellable(
            &client,
            "http://127.0.0.1:9999/dummy",
            &target,
            Some(100),
            Some(("fakehash", HashType::Sha256)),
            &events,
            Some(&token),
        )
        .await;

        assert!(matches!(res, Err(WexError::Cancelled)));
        assert_eq!(
            fs::read(&target).unwrap(),
            original_data,
            "Installed file must remain intact upon cancellation"
        );
        let part_file = tmp.path().join("my_mod.jar.part");
        assert!(!part_file.exists());
    }
}
