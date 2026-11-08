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

impl From<WexError> for wex_ipc::error::IpcError {
    fn from(err: WexError) -> Self {
        match err {
            WexError::Io(e) => e.into(),
            WexError::Network(e) => {
                let code = if e.is_timeout() {
                    "IO_TIMED_OUT"
                } else {
                    "NETWORK_ERROR"
                };
                let msg = if e.is_timeout() {
                    "Истекло время ожидания ответа сервера"
                } else {
                    "Сетевая ошибка при загрузке компонентов игры"
                };
                Self::new(code, msg, Some(e.to_string()))
            }
            WexError::Json(e) => e.into(),
            WexError::Zip(e) => Self::new(
                "WORKSHOP_ZIP_ERROR",
                "Ошибка распаковки или создания ZIP-архива",
                Some(e.to_string()),
            ),
            WexError::HashMismatch {
                path,
                expected,
                actual,
            } => Self::new(
                "INSTALL_HASH_MISMATCH",
                "Контрольная сумма загруженного файла не совпадает",
                Some(format!(
                    "Path: {}, expected {}, actual {}",
                    path.display(),
                    expected,
                    actual
                )),
            ),
            WexError::SizeMismatch {
                path,
                expected,
                actual,
            } => Self::new(
                "INSTALL_SIZE_MISMATCH",
                "Размер загруженного файла не совпадает с ожидаемым",
                Some(format!(
                    "Path: {}, expected {} bytes, actual {} bytes",
                    path.display(),
                    expected,
                    actual
                )),
            ),
            WexError::DownloadError { url, message } => Self::new(
                "INSTALL_DOWNLOAD_FAILED",
                "Не удалось загрузить файл по сети",
                Some(format!("URL: {}, error: {}", url, message)),
            ),
            WexError::ManifestError(msg) => Self::new(
                "INSTALL_MANIFEST_ERROR",
                "Ошибка структуры манифеста версии игры",
                Some(msg),
            ),
            WexError::UnsupportedVersion(ver) => Self::new(
                "LAUNCH_FAILED",
                format!("Версия {} не поддерживается лаунчером", ver),
                None,
            ),
            WexError::JavaError(msg) => {
                let code = if msg.contains("not found") || msg.contains("не найден") {
                    "JAVA_NOT_FOUND"
                } else if msg.contains("version") || msg.contains("версия") {
                    "JAVA_WRONG_VERSION"
                } else {
                    "JAVA_RUNTIME_ERROR"
                };
                Self::new(code, "Ошибка среды выполнения Java", Some(msg))
            }
            WexError::LaunchError(msg) => Self::new(
                "LAUNCH_FAILED",
                "Ошибка при подготовке или запуске игры",
                Some(msg),
            ),
            WexError::ProcessFailed(code) => Self::new(
                "LAUNCH_PROCESS_FAILED",
                format!("Игровой процесс завершился с кодом ошибки {}", code),
                Some(format!("Process exit code: {}", code)),
            ),
            WexError::Cancelled => Self::new(
                "OPERATION_CANCELLED",
                "Операция отменена пользователем",
                None,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wex_error_to_ipc_error_conversions() {
        let err = WexError::JavaError("java binary not found in PATH".into());
        let ipc: wex_ipc::error::IpcError = err.into();
        assert_eq!(ipc.code, "JAVA_NOT_FOUND");
        assert_eq!(ipc.message, "Ошибка среды выполнения Java");
        assert!(ipc.technical.unwrap().contains("not found"));

        let err2 = WexError::ProcessFailed(1);
        let ipc2: wex_ipc::error::IpcError = err2.into();
        assert_eq!(ipc2.code, "LAUNCH_PROCESS_FAILED");

        let err3 = WexError::HashMismatch {
            path: PathBuf::from("client.jar"),
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        let ipc3: wex_ipc::error::IpcError = err3.into();
        assert_eq!(ipc3.code, "INSTALL_HASH_MISMATCH");
    }
}
