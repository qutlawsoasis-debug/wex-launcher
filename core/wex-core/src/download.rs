use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler};
use futures_util::StreamExt;
use reqwest::header::{HeaderMap, ETAG, IF_RANGE, LAST_MODIFIED, RANGE};
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

/// [`compute_file_hash`] on the blocking pool, so hashing big jars never stalls
/// the async runtime (and with it the UI event loop).
pub async fn compute_file_hash_async(path: PathBuf, hash_type: HashType) -> Result<String> {
    tokio::task::spawn_blocking(move || compute_file_hash(&path, hash_type))
        .await
        .map_err(|e| WexError::Io(std::io::Error::other(e)))?
}

/// [`verify_file`] on the blocking pool.
pub async fn verify_file_async(
    path: &Path,
    expected_size: Option<u64>,
    expected_hash: Option<(&str, HashType)>,
) -> bool {
    let path = path.to_path_buf();
    let expected_hash = expected_hash.map(|(h, t)| (h.to_string(), t));
    tokio::task::spawn_blocking(move || {
        verify_file(
            &path,
            expected_size,
            expected_hash.as_ref().map(|(h, t)| (h.as_str(), *t)),
        )
    })
    .await
    .unwrap_or(false)
}

/// Sharing violation (32) or access denied (5): on Windows an antivirus or the
/// search indexer briefly holds a freshly written file open.
fn is_transient_lock_error(err: &std::io::Error) -> bool {
    cfg!(windows) && matches!(err.raw_os_error(), Some(5) | Some(32))
}

const RENAME_RETRIES: u32 = 5;
const RENAME_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(200);

/// Moves a verified `.part` file over the target in one step. `fs::rename` replaces an
/// existing target atomically on Windows (MoveFileEx with REPLACE_EXISTING) and Unix,
/// so the old file is never deleted first; transient locks are retried with a delay.
pub async fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut retries = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if should_retry_rename(&e, retries) => {
                retries += 1;
                tokio::time::sleep(RENAME_RETRY_DELAY).await;
            }
            Err(e) => return Err(e),
        }
    }
}

/// [`rename_with_retry`] for synchronous callers (atomic writes, pack commit).
pub fn rename_with_retry_blocking(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut retries = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if should_retry_rename(&e, retries) => {
                retries += 1;
                std::thread::sleep(RENAME_RETRY_DELAY);
            }
            Err(e) => return Err(e),
        }
    }
}

fn should_retry_rename(err: &std::io::Error, retries_done: u32) -> bool {
    retries_done < RENAME_RETRIES && is_transient_lock_error(err)
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

    if verify_file_async(target_path, expected_size, expected_hash).await {
        return Ok(false);
    }

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| map_io_err(e, parent))?;
    }

    let part_path = PathBuf::from(format!("{}.part", target_path.display()));
    let mut existing_len = 0u64;

    // A complete, verified part from an earlier run whose final rename failed
    // (antivirus lock): finish it without downloading again.
    if expected_hash.is_some()
        && expected_size.is_some()
        && verify_file_async(&part_path, expected_size, expected_hash).await
    {
        rename_with_retry(&part_path, target_path)
            .await
            .map_err(|e| map_io_err(e, target_path))?;
        events.on_event(&Event::FileDownloaded {
            path: target_path.display().to_string(),
            bytes: expected_size.unwrap_or_default(),
        });
        return Ok(true);
    }

    if part_path.exists() {
        if let Ok(meta) = fs::metadata(&part_path) {
            existing_len = meta.len();
            if let Some(exp_size) = expected_size {
                if existing_len >= exp_size {
                    remove_part(&part_path);
                    existing_len = 0;
                }
            }
        }
    }

    // ETag / Last-Modified of the response the .part was started from.
    let mut validator = read_part_validator(&part_path);

    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 3;

    loop {
        attempts += 1;
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                remove_part(&part_path);
                return Err(WexError::Cancelled);
            }
        }

        if existing_len > 0 && validator.is_none() && expected_hash.is_none() {
            // Nothing proves the server still has the same file: start over.
            existing_len = 0;
        }
        let mut req = client.get(url);
        let resuming = existing_len > 0;
        if resuming {
            req = req.header(RANGE, format!("bytes={}-", existing_len));
            if let Some(v) = &validator {
                // A changed file comes back whole (200) instead of a mismatched tail.
                req = req.header(IF_RANGE, v.as_str());
            }
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
            remove_part(&part_path);
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
            remove_part(&part_path);
            return Err(WexError::DownloadError {
                url: url.to_string(),
                message: format!("HTTP error status: {}", status),
            });
        }

        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            if attempts >= MAX_ATTEMPTS {
                remove_part(&part_path);
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
                    remove_part(&part_path);
                    map_io_err(e, &part_path)
                })?
        } else {
            existing_len = 0;
            let file = File::create(&part_path).map_err(|e| {
                remove_part(&part_path);
                map_io_err(e, &part_path)
            })?;
            // The .part is empty now; only then does it get the new response's validator.
            validator = response_validator(resp.headers());
            write_part_validator(&part_path, validator.as_deref());
            file
        };

        let mut stream = resp.bytes_stream();
        let mut downloaded_bytes = existing_len;
        let mut stream_interrupted = false;

        while let Some(chunk_res) = stream.next().await {
            if let Some(token) = cancel_token {
                if token.is_cancelled() {
                    drop(file);
                    remove_part(&part_path);
                    return Err(WexError::Cancelled);
                }
            }
            match chunk_res {
                Ok(chunk) => {
                    if let Err(e) = file.write_all(&chunk) {
                        drop(file);
                        remove_part(&part_path);
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
            remove_part(&part_path);
            return Err(map_io_err(e, &part_path));
        }
        drop(file);

        if stream_interrupted {
            if attempts >= MAX_ATTEMPTS {
                remove_part(&part_path);
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
                remove_part(&part_path);
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
                    remove_part(&part_path);
                    existing_len = 0;
                }
                if attempts >= MAX_ATTEMPTS {
                    remove_part(&part_path);
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
            let actual_hash = compute_file_hash_async(part_path.clone(), hash_type).await?;
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
                remove_part(&part_path);
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

        // Atomically replace the target (never delete it first). On failure the
        // verified part is kept: the next run moves it without downloading.
        if let Err(e) = rename_with_retry(&part_path, target_path).await {
            return Err(map_io_err(e, target_path));
        }
        write_part_validator(&part_path, None);

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

/// File beside a `.part` that keeps the validator of the response it came from.
fn part_validator_path(part_path: &Path) -> PathBuf {
    let mut name = part_path.file_name().unwrap_or_default().to_os_string();
    name.push(".validator");
    part_path.with_file_name(name)
}

/// A validator usable in `If-Range`: a strong ETag, else Last-Modified (weak
/// ETags are not allowed there).
fn response_validator(headers: &HeaderMap) -> Option<String> {
    let header = |name| {
        headers
            .get(name)
            .and_then(|v: &reqwest::header::HeaderValue| v.to_str().ok())
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
    };
    header(ETAG)
        .filter(|e| !e.starts_with("W/"))
        .or_else(|| header(LAST_MODIFIED))
}

/// Delete a `.part` together with its validator.
fn remove_part(part_path: &Path) {
    let _ = fs::remove_file(part_path);
    write_part_validator(part_path, None);
}

fn read_part_validator(part_path: &Path) -> Option<String> {
    fs::read_to_string(part_validator_path(part_path))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty() && !v.contains(['\r', '\n']))
}

fn write_part_validator(part_path: &Path, validator: Option<&str>) {
    let path = part_validator_path(part_path);
    match validator {
        Some(v) => {
            let _ = fs::write(path, v);
        }
        None => {
            let _ = fs::remove_file(path);
        }
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

/// Connect timeout for every download/API client.
pub const HTTP_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
/// Idle timeout per read (not a total deadline), so big files on slow links still finish
/// while a stalled connection fails instead of hanging forever.
pub const HTTP_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Shared client builder with connect and per-read (idle) timeouts.
pub fn http_client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent("WexLauncher/0.1.0")
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .read_timeout(HTTP_READ_TIMEOUT)
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
    // Several entries may point at one file (asset names sharing a hash, a native
    // classifier equal to the artifact). Two tasks for one target would share one
    // `.part` file and corrupt each other, so only the first is kept.
    let mut seen_targets = std::collections::HashSet::new();
    let tasks: Vec<DownloadTask> = tasks
        .into_iter()
        .filter(|t| seen_targets.insert(t.target_path.clone()))
        .collect();
    let total_tasks = tasks.len();
    if total_tasks == 0 {
        return Ok(0);
    }

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
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
    async fn resume_sends_if_range_with_the_stored_validator() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/f.bin"))
            .and(header("Range", "bytes=4-"))
            .and(header("If-Range", "\"v1\""))
            .respond_with(ResponseTemplate::new(206).set_body_bytes(b"5678".to_vec()))
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("f.bin");
        let part = tmp.path().join("f.bin.part");
        fs::write(&part, b"1234").unwrap();
        fs::write(part_validator_path(&part), "\"v1\"").unwrap();

        let res = download_file_resumable_cancellable(
            &reqwest::Client::new(),
            &format!("{}/f.bin", server.uri()),
            &target,
            Some(8),
            None,
            &crate::event::NoopEventHandler,
            None,
        )
        .await;
        assert!(res.is_ok(), "{res:?}");
        assert_eq!(fs::read(&target).unwrap(), b"12345678");
        assert!(!part_validator_path(&part).exists());
    }

    #[tokio::test]
    async fn changed_file_comes_back_whole_and_replaces_the_part() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // The server's file changed: If-Range does not match, so it sends all of it.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/f.bin"))
            .and(header("If-Range", "\"old\""))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"new\"")
                    .set_body_bytes(b"NEWDATA!".to_vec()),
            )
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("f.bin");
        let part = tmp.path().join("f.bin.part");
        fs::write(&part, b"OLD-").unwrap();
        fs::write(part_validator_path(&part), "\"old\"").unwrap();

        let res = download_file_resumable_cancellable(
            &reqwest::Client::new(),
            &format!("{}/f.bin", server.uri()),
            &target,
            Some(8),
            None,
            &crate::event::NoopEventHandler,
            None,
        )
        .await;
        assert!(res.is_ok(), "{res:?}");
        assert_eq!(fs::read(&target).unwrap(), b"NEWDATA!");
    }

    #[tokio::test]
    async fn part_without_validator_or_hash_is_not_resumed() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/f.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ABCDEFGH".to_vec()))
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("f.bin");
        fs::write(tmp.path().join("f.bin.part"), b"zzzz").unwrap();

        let res = download_file_resumable_cancellable(
            &reqwest::Client::new(),
            &format!("{}/f.bin", server.uri()),
            &target,
            Some(8),
            None,
            &crate::event::NoopEventHandler,
            None,
        )
        .await;
        assert!(res.is_ok(), "{res:?}");
        assert_eq!(fs::read(&target).unwrap(), b"ABCDEFGH");
        let requests = server.received_requests().await.unwrap();
        assert!(requests.iter().all(|r| !r.headers.contains_key("range")));
    }

    #[tokio::test]
    async fn validator_is_deleted_with_the_part() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/gone.bin"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("gone.bin");
        let part = tmp.path().join("gone.bin.part");
        fs::write(&part, b"1234").unwrap();
        fs::write(part_validator_path(&part), "\"v1\"").unwrap();

        let res = download_file_resumable_cancellable(
            &reqwest::Client::new(),
            &format!("{}/gone.bin", server.uri()),
            &target,
            Some(8),
            None,
            &crate::event::NoopEventHandler,
            None,
        )
        .await;
        assert!(res.is_err());
        assert!(!part.exists());
        assert!(!part_validator_path(&part).exists());
    }

    #[test]
    fn only_strong_etags_or_dates_are_validators() {
        let mut h = HeaderMap::new();
        assert_eq!(response_validator(&h), None);
        h.insert(
            LAST_MODIFIED,
            "Wed, 21 Oct 2015 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(
            response_validator(&h).as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
        h.insert(ETAG, "W/\"weak\"".parse().unwrap());
        assert_eq!(
            response_validator(&h).as_deref(),
            Some("Wed, 21 Oct 2015 07:28:00 GMT")
        );
        h.insert(ETAG, "\"strong\"".parse().unwrap());
        assert_eq!(response_validator(&h).as_deref(), Some("\"strong\""));
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

    #[tokio::test]
    async fn test_download_pool_zero_concurrency_does_not_deadlock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/a.bin"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello".to_vec()))
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("a.bin");
        let task = DownloadTask {
            url: format!("{}/a.bin", server.uri()),
            fallback_url: None,
            target_path: target.clone(),
            size: Some(5),
            hash: None,
        };
        let client = http_client_builder().build().unwrap();
        let events = crate::event::NoopEventHandler;
        let res = tokio::time::timeout(
            std::time::Duration::from_secs(20),
            download_pool(&client, vec![task], 0, &events, "test"),
        )
        .await;
        assert!(res
            .expect("download_pool deadlocked with concurrency 0")
            .is_ok());
        assert_eq!(fs::read(&target).unwrap(), b"hello");
    }

    #[tokio::test]
    async fn download_pool_dedupes_tasks_for_the_same_target() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/shared.bin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"hello".to_vec())
                    .set_delay(std::time::Duration::from_millis(300)),
            )
            .expect(1)
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("objects/ab/abcdef");
        let task = DownloadTask {
            url: format!("{}/shared.bin", server.uri()),
            fallback_url: None,
            target_path: target.clone(),
            size: Some(5),
            hash: None,
        };
        let client = http_client_builder().build().unwrap();
        let res = download_pool(
            &client,
            vec![task.clone(), task.clone(), task],
            4,
            &crate::event::NoopEventHandler,
            "test",
        )
        .await;
        assert!(res.is_ok(), "{res:?}");
        assert_eq!(fs::read(&target).unwrap(), b"hello");
        server.verify().await;
    }

    #[tokio::test]
    async fn rename_replaces_existing_target_without_deleting_first() {
        let tmp = tempdir().unwrap();
        let part = tmp.path().join("a.jar.part");
        let target = tmp.path().join("a.jar");
        fs::write(&part, b"new").unwrap();
        fs::write(&target, b"old").unwrap();
        rename_with_retry(&part, &target).await.unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(!part.exists());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn rename_retries_while_target_is_locked() {
        use std::os::windows::fs::OpenOptionsExt;
        let tmp = tempdir().unwrap();
        let part = tmp.path().join("b.jar.part");
        let target = tmp.path().join("b.jar");
        fs::write(&part, b"new").unwrap();
        fs::write(&target, b"old").unwrap();
        // No sharing at all: like an antivirus scanner holding the file.
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .unwrap();
        let release = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(350));
            drop(lock);
        });
        let res = rename_with_retry(&part, &target).await;
        release.join().unwrap();
        assert!(res.is_ok(), "{res:?}");
        assert_eq!(fs::read(&target).unwrap(), b"new");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn rename_gives_up_after_retries_on_a_permanent_lock() {
        use std::os::windows::fs::OpenOptionsExt;
        let tmp = tempdir().unwrap();
        let part = tmp.path().join("c.jar.part");
        let target = tmp.path().join("c.jar");
        fs::write(&part, b"new").unwrap();
        fs::write(&target, b"old").unwrap();
        let _lock = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .unwrap();
        let start = std::time::Instant::now();
        assert!(rename_with_retry(&part, &target).await.is_err());
        assert!(start.elapsed() >= RENAME_RETRY_DELAY * RENAME_RETRIES);
        assert!(part.exists(), "the verified part survives for the next try");
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn verified_part_survives_a_failed_rename_and_is_reused() {
        use sha2::Digest;
        use std::os::windows::fs::OpenOptionsExt;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let body = b"fresh mod".to_vec();
        let sha = hex::encode(sha2::Sha256::digest(&body));
        Mock::given(method("GET"))
            .and(path("/m.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
            .expect(1)
            .mount(&server)
            .await;
        let tmp = tempdir().unwrap();
        let target = tmp.path().join("m.jar");
        let part = tmp.path().join("m.jar.part");
        fs::write(&target, b"old").unwrap();
        let url = format!("{}/m.jar", server.uri());
        let client = reqwest::Client::new();
        let events = crate::event::NoopEventHandler;
        let lock = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .unwrap();
        let first = download_file_resumable(
            &client,
            &url,
            &target,
            Some(body.len() as u64),
            Some((&sha, HashType::Sha256)),
            &events,
        )
        .await;
        assert!(first.is_err());
        assert_eq!(fs::read(&part).unwrap(), body, "verified part is kept");
        drop(lock);
        let second = download_file_resumable(
            &client,
            &url,
            &target,
            Some(body.len() as u64),
            Some((&sha, HashType::Sha256)),
            &events,
        )
        .await;
        assert!(second.unwrap());
        assert_eq!(fs::read(&target).unwrap(), body);
        assert!(!part.exists());
        server.verify().await;
    }
}
