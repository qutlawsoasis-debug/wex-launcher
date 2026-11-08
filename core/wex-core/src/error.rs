use std::path::PathBuf;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, WexError>;

#[derive(Error, Debug)]
pub enum WexError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Hash mismatch for {path}: expected {expected}, actual {actual}")]
    HashMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },

    #[error("File size mismatch for {path}: expected {expected}, actual {actual}")]
    SizeMismatch {
        path: PathBuf,
        expected: u64,
        actual: u64,
    },

    #[error("Download error for {url}: {message}")]
    DownloadError { url: String, message: String },

    #[error("Manifest error: {0}")]
    ManifestError(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    #[error("Java runtime error: {0}")]
    JavaError(String),

    #[error("Launch error: {0}")]
    LaunchError(String),

    #[error("Process exited with code {0}")]
    ProcessFailed(i32),

    #[error("Operation cancelled")]
    Cancelled,
}
