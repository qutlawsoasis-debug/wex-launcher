use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use futures_util::StreamExt;
use regex::Regex;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{watch, Mutex};

use crate::events::{NetEvent, NetEventBus, TunnelStatus};

pub const PLAYIT_DOWNLOAD_URL: &str =
    "https://github.com/playit-cloud/playit-agent/releases/download/v0.15.26/playit-windows-x86_64-signed.exe";
pub const PLAYIT_EXPECTED_SHA256: &str =
    "dd1acb19e47bca4a935f2f72a68390bd2fc3a8ed608af7c9c247d3a69d7fba0a";
pub const PLAYIT_VERSION: &str = "0.15.26";

#[derive(Error, Debug)]
pub enum TunnelError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("HTTP download error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("SHA-256 hash mismatch! Expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Tunnel process failed to start: {0}")]
    ProcessSpawn(String),

    #[error("Operating system job object error: {0}")]
    JobObject(String),
}

/// Computes SHA-256 hex string of a file
pub fn compute_file_sha256<P: AsRef<Path>>(path: P) -> Result<String, io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    use std::io::Read;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Ensures the official playit-agent binary exists and is verified by SHA-256.
pub async fn ensure_binary_downloaded(
    tools_dir: &Path,
    event_bus: Option<&NetEventBus>,
) -> Result<PathBuf, TunnelError> {
    fs::create_dir_all(tools_dir)?;
    let binary_name = format!("playit-{PLAYIT_VERSION}.exe");
    let target_path = tools_dir.join(&binary_name);

    if target_path.exists() {
        if let Ok(hash) = compute_file_sha256(&target_path) {
            if hash.eq_ignore_ascii_case(PLAYIT_EXPECTED_SHA256) {
                return Ok(target_path);
            }
        }
        let _ = fs::remove_file(&target_path);
    }

    if let Some(bus) = event_bus {
        bus.emit(NetEvent::TunnelStatusChanged {
            status: TunnelStatus::Downloading { progress: 0.0 },
        });
    }

    let client = reqwest::Client::new();
    let response = client.get(PLAYIT_DOWNLOAD_URL).send().await?;

    let total_bytes = response.content_length().unwrap_or(0);
    let mut stream = response.bytes_stream();

    let temp_path = tools_dir.join(format!("playit-{PLAYIT_VERSION}.tmp"));
    let mut temp_file = tokio::fs::File::create(&temp_path).await?;
    let mut downloaded_bytes: u64 = 0;

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let data = chunk?;
        temp_file.write_all(&data).await?;
        downloaded_bytes += data.len() as u64;

        if let Some(bus) = event_bus {
            bus.emit(NetEvent::DownloadProgress {
                file: binary_name.clone(),
                downloaded_bytes,
                total_bytes,
            });

            if total_bytes > 0 {
                bus.emit(NetEvent::TunnelStatusChanged {
                    status: TunnelStatus::Downloading {
                        progress: (downloaded_bytes as f32) / (total_bytes as f32),
                    },
                });
            }
        }
    }
    temp_file.flush().await?;
    drop(temp_file);

    let actual_hash = compute_file_sha256(&temp_path)?;
    if !actual_hash.eq_ignore_ascii_case(PLAYIT_EXPECTED_SHA256) {
        let _ = fs::remove_file(&temp_path);
        return Err(TunnelError::ChecksumMismatch {
            expected: PLAYIT_EXPECTED_SHA256.to_string(),
            actual: actual_hash,
        });
    }

    fs::rename(&temp_path, &target_path)?;
    Ok(target_path)
}

/// Parses public tunnel host and port from playit agent output lines.
pub fn parse_tunnel_address(line: &str) -> Option<(String, u16)> {
    // Typical patterns in playit output:
    // "tunnel running: pgsql-jill.tun.ply.gg:38062"
    // "Registered tunnel at playit.gg: 123.45.67.89:25565"
    // "... abc-xyz.ply.gg:12345"
    let re = Regex::new(r"([a-zA-Z0-9.-]+\.ply\.gg):(\d+)").ok()?;
    if let Some(caps) = re.captures(line) {
        let host = caps.get(1)?.as_str().to_string();
        let port: u16 = caps.get(2)?.as_str().parse().ok()?;
        return Some((host, port));
    }

    let re_generic = Regex::new(r"(?:tunnel running|address):\s*([a-zA-Z0-9.-]+):(\d+)").ok()?;
    if let Some(caps) = re_generic.captures(line) {
        let host = caps.get(1)?.as_str().to_string();
        let port: u16 = caps.get(2)?.as_str().parse().ok()?;
        return Some((host, port));
    }

    None
}

// ---------------------------------------------------------------------------
// Process Supervision & Windows Job Object
// ---------------------------------------------------------------------------

#[cfg(windows)]
struct WindowsJobObject {
    job_handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
impl WindowsJobObject {
    fn new() -> Result<Self, TunnelError> {
        use std::mem::size_of;
        use std::ptr;
        use windows_sys::Win32::System::JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        };

        unsafe {
            let job = CreateJobObjectW(ptr::null(), ptr::null());
            if job.is_null() {
                return Err(TunnelError::JobObject(
                    "CreateJobObjectW failed".to_string(),
                ));
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let success = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if success == 0 {
                windows_sys::Win32::Foundation::CloseHandle(job);
                return Err(TunnelError::JobObject(
                    "SetInformationJobObject failed".to_string(),
                ));
            }

            Ok(Self { job_handle: job })
        }
    }

    fn assign_process(
        &self,
        process_handle: windows_sys::Win32::Foundation::HANDLE,
    ) -> Result<(), TunnelError> {
        use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

        unsafe {
            let res = AssignProcessToJobObject(self.job_handle, process_handle);
            if res == 0 {
                return Err(TunnelError::JobObject(
                    "AssignProcessToJobObject failed".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
impl Drop for WindowsJobObject {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            if !self.job_handle.is_null() {
                CloseHandle(self.job_handle);
            }
        }
    }
}

pub struct PlayitSupervisor {
    child: Arc<Mutex<Option<Child>>>,
    stop_tx: watch::Sender<bool>,
    events: Option<Arc<NetEventBus>>,
    #[cfg(windows)]
    _job_object: Option<WindowsJobObject>,
}

impl PlayitSupervisor {
    pub fn new(events: Option<Arc<NetEventBus>>) -> Self {
        let (stop_tx, _) = watch::channel(false);
        Self {
            child: Arc::new(Mutex::new(None)),
            stop_tx,
            events,
            #[cfg(windows)]
            _job_object: WindowsJobObject::new().ok(),
        }
    }

    /// Starts playit-agent as a supervised child process.
    pub async fn start(
        &mut self,
        binary_path: &Path,
        secret_path: &Path,
    ) -> Result<(), TunnelError> {
        self.stop().await;

        if let Some(bus) = &self.events {
            bus.emit(NetEvent::TunnelStatusChanged {
                status: TunnelStatus::Starting,
            });
        }

        let mut cmd = Command::new(binary_path);
        cmd.arg("--secret_path")
            .arg(secret_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        #[cfg(windows)]
        {
            // CREATE_NO_WINDOW = 0x08000000
            cmd.creation_flags(0x08000000);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| TunnelError::ProcessSpawn(e.to_string()))?;

        #[cfg(windows)]
        {
            if let Some(job) = &self._job_object {
                if let Some(raw_handle) = child.raw_handle() {
                    let _ = job.assign_process(raw_handle as _);
                }
            }
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (stop_tx, mut stop_rx) = watch::channel(false);
        self.stop_tx = stop_tx;

        let events_clone = self.events.clone();

        // Stream output in background task
        tokio::spawn(async move {
            let mut stdout_reader = stdout.map(|s| BufReader::new(s).lines());
            let mut stderr_reader = stderr.map(|s| BufReader::new(s).lines());
            let mut found_address = false;

            loop {
                tokio::select! {
                    _ = stop_rx.changed() => {
                        if *stop_rx.borrow() {
                            break;
                        }
                    }
                    line = async {
                        if let Some(reader) = &mut stdout_reader {
                            reader.next_line().await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        if let Ok(Some(l)) = line {
                            if !found_address {
                                if let Some((addr, port)) = parse_tunnel_address(&l) {
                                    found_address = true;
                                    if let Some(bus) = &events_clone {
                                        bus.emit(NetEvent::TunnelStatusChanged {
                                            status: TunnelStatus::Running { address: addr, port },
                                        });
                                    }
                                }
                            }
                        }
                    }
                    err_line = async {
                        if let Some(reader) = &mut stderr_reader {
                            reader.next_line().await
                        } else {
                            std::future::pending().await
                        }
                    } => {
                        if let Ok(Some(l)) = err_line {
                            if !found_address {
                                if let Some((addr, port)) = parse_tunnel_address(&l) {
                                    found_address = true;
                                    if let Some(bus) = &events_clone {
                                        bus.emit(NetEvent::TunnelStatusChanged {
                                            status: TunnelStatus::Running { address: addr, port },
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        *self.child.lock().await = Some(child);
        Ok(())
    }

    /// Stops the playit child process
    pub async fn stop(&mut self) {
        let _ = self.stop_tx.send(true);
        let mut lock = self.child.lock().await;
        if let Some(mut child) = lock.take() {
            let _ = child.kill().await;
        }

        if let Some(bus) = &self.events {
            bus.emit(NetEvent::TunnelStatusChanged {
                status: TunnelStatus::Stopped,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tunnel_address_formats() {
        let l1 = "2026-11-07 23:00:00 INFO tunnel running: pgsql-jill.tun.ply.gg:38062";
        assert_eq!(
            parse_tunnel_address(l1),
            Some(("pgsql-jill.tun.ply.gg".to_string(), 38062))
        );

        let l2 = "Connected to tunnel server! Public address: custom-sub.ply.gg:12345";
        assert_eq!(
            parse_tunnel_address(l2),
            Some(("custom-sub.ply.gg".to_string(), 12345))
        );

        let l3 = "Unrelated log line without address";
        assert_eq!(parse_tunnel_address(l3), None);
    }

    #[test]
    fn test_compute_sha256_mock_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.bin");
        fs::write(&file_path, b"hello playit tunnel").unwrap();

        let hash = compute_file_sha256(&file_path).unwrap();
        assert_eq!(hash.len(), 64);

        // Expected hash for "hello playit tunnel"
        let mut hasher = Sha256::new();
        hasher.update(b"hello playit tunnel");
        let expected = hex::encode(hasher.finalize());
        assert_eq!(hash, expected);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_job_object_creation() {
        let job = WindowsJobObject::new();
        assert!(job.is_ok());
    }
}
