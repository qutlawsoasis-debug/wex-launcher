use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use futures_util::StreamExt;
use reqwest::header::RANGE;
use sha1::Digest as _;
use tokio::sync::Semaphore;
use crate::error::{Result, WexError};
use crate::event::{Event, EventHandler};

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

pub fn verify_file(path: &Path, expected_size: Option<u64>, expected_hash: Option<(&str, HashType)>) -> bool {
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
        fs::create_dir_all(parent)?;
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
                tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64)).await;
                continue;
            }
        };

        let status = resp.status();
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            if attempts >= MAX_ATTEMPTS {
                return Err(WexError::DownloadError {
                    url: url.to_string(),
                    message: format!("HTTP error status: {}", status),
                });
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500 * attempts as u64)).await;
            continue;
        }

        let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;
        let mut file = if is_partial && resuming {
            OpenOptions::new().create(true).append(true).open(&part_path)?
        } else {
            existing_len = 0;
            File::create(&part_path)?
        };

        let mut stream = resp.bytes_stream();
        let mut downloaded_bytes = existing_len;

        while let Some(chunk_res) = stream.next().await {
            if let Some(token) = cancel_token {
                if token.is_cancelled() {
                    return Err(WexError::Cancelled);
                }
            }
            match chunk_res {
                Ok(chunk) => {
                    file.write_all(&chunk)?;
                    downloaded_bytes += chunk.len() as u64;
                }
                Err(e) => {
                    if attempts >= MAX_ATTEMPTS {
                        return Err(WexError::DownloadError {
                            url: url.to_string(),
                            message: format!("Stream error after {} attempts: {}", attempts, e),
                        });
                    }
                    existing_len = downloaded_bytes;
                    break;
                }
            }
        }

        file.flush()?;
        drop(file);

        // Verification
        if let Some(exp_size) = expected_size {
            let actual_len = fs::metadata(&part_path)?.len();
            if actual_len != exp_size {
                if attempts >= MAX_ATTEMPTS {
                    let _ = fs::remove_file(&part_path);
                    return Err(WexError::SizeMismatch {
                        path: target_path.to_path_buf(),
                        expected: exp_size,
                        actual: actual_len,
                    });
                }
                continue;
            }
        }

        if let Some((exp_hash, hash_type)) = expected_hash {
            let actual_hash = compute_file_hash(&part_path, hash_type)?;
            if !actual_hash.eq_ignore_ascii_case(exp_hash) {
                let _ = fs::remove_file(&part_path);
                if attempts >= MAX_ATTEMPTS {
                    return Err(WexError::HashMismatch {
                        path: target_path.to_path_buf(),
                        expected: exp_hash.to_string(),
                        actual: actual_hash,
                    });
                }
                existing_len = 0;
                continue;
            }
        }

        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
        fs::rename(&part_path, target_path)?;

        let final_len = fs::metadata(target_path)?.len();
        events.on_event(&Event::FileDownloaded {
            path: target_path.display().to_string(),
            bytes: final_len,
        });

        return Ok(true);
    }
}

pub struct DownloadTask {
    pub url: String,
    pub target_path: PathBuf,
    pub size: Option<u64>,
    pub hash: Option<(String, HashType)>,
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
            let res = download_file_resumable_cancellable(
                &client,
                &task.url,
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
        assert!(verify_file(&file_path, Some(11), Some((&sha1, HashType::Sha1))));
        assert!(verify_file(&file_path, Some(11), Some((&sha256, HashType::Sha256))));
    }
}
