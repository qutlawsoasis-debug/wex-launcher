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

    /// «Свой сервер»: the message is already a sentence for the player.
    #[error("{0}")]
    Server(String),

    #[error("Operation cancelled")]
    Cancelled,

    /// Account sign-in or token refresh failed. `message` is shown to the player as is.
    #[error("Auth error {code}: {message}")]
    Auth {
        code: &'static str,
        message: String,
        details: Option<String>,
    },

    #[error("Not enough disk space for {path}: {source}")]
    DiskFull {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl WexError {
    /// Only transport unavailability permits cached metadata. HTTP rejection,
    /// malformed responses, integrity errors and disk failures must remain errors.
    pub(crate) fn is_network_unavailable(&self) -> bool {
        matches!(self, Self::Network(e) if !e.is_status() && !e.is_decode()
            && (e.is_connect() || e.is_timeout()))
    }
}

impl From<WexError> for wex_ipc::error::IpcError {
    fn from(err: WexError) -> Self {
        match err {
            WexError::DiskFull { path, source } => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("файла");
                Self::new(
                    "IO_DISK_FULL",
                    "На диске закончилось свободное место. Освободите место на диске и повторите попытку.",
                    Some(format!("File: {}, source: {}", file_name, source)),
                )
            }
            WexError::Io(e) => e.into(),
            WexError::Auth {
                code,
                message,
                details,
            } => Self::new(code, message, details),
            WexError::Network(e) => {
                let code = if e.is_timeout() {
                    "IO_TIMED_OUT"
                } else {
                    "NETWORK_ERROR"
                };
                let msg = if e.is_timeout() {
                    "Сервер не отвечает на запрос. Проверьте подключение к интернету или попробуйте позже."
                } else {
                    "Нет подключения к интернету. Проверьте сеть и повторите попытку."
                };
                Self::new(code, msg, Some(e.to_string()))
            }
            WexError::Json(e) => e.into(),
            WexError::Zip(e) => Self::new(
                "WORKSHOP_ZIP_ERROR",
                "Не удалось распаковать архив с файлами. Проверьте свободное место на диске и повторите попытку.",
                Some(e.to_string()),
            ),
            WexError::HashMismatch {
                path,
                expected,
                actual,
            } => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("файла");
                Self::new(
                    "INSTALL_HASH_MISMATCH",
                    "Файл игры повреждён при скачивании. Лаунчер скачает его заново.",
                    Some(format!(
                        "File: {}, expected {}, actual {}",
                        file_name, expected, actual
                    )),
                )
            }
            WexError::SizeMismatch {
                path,
                expected,
                actual,
            } => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("файла");
                Self::new(
                    "INSTALL_SIZE_MISMATCH",
                    "Файл игры загрузился не полностью. Лаунчер повторит загрузку.",
                    Some(format!(
                        "File: {}, expected {} bytes, actual {} bytes",
                        file_name, expected, actual
                    )),
                )
            }
            WexError::DownloadError { url, message } => {
                let clean_name = if let Some(last_slash) = url.rfind('/') {
                    &url[last_slash + 1..]
                } else {
                    &url
                };
                Self::new(
                    "INSTALL_DOWNLOAD_FAILED",
                    "Не удалось скачать файлы игры. Проверьте подключение к интернету и повторите загрузку.",
                    Some(format!("File: {}, details: {}", clean_name, message)),
                )
            }
            WexError::ManifestError(msg) => Self::new(
                "INSTALL_MANIFEST_ERROR",
                "Не удалось прочитать список файлов игры. Проверьте подключение к интернету или попробуйте позже.",
                Some(msg),
            ),
            WexError::UnsupportedVersion(ver) => Self::new(
                "LAUNCH_FAILED",
                format!("Версия {} не поддерживается лаунчером. Выберите поддерживаемую версию в списке.", ver),
                None,
            ),
            WexError::JavaError(msg) => {
                let lower = msg.to_lowercase();
                let (code, text) = if lower.contains("not found") || lower.contains("не найден") {
                    (
                        "JAVA_NOT_FOUND",
                        "Java не найдена на компьютере. Укажите путь к Java в настройках лаунчера или установите её.",
                    )
                } else if lower.contains("version") || lower.contains("версия") || lower.contains("major.minor") {
                    (
                        "JAVA_WRONG_VERSION",
                        "Установленная версия Java не подходит для этой игры. Выберите подходящую версию Java в настройках.",
                    )
                } else {
                    (
                        "JAVA_RUNTIME_ERROR",
                        "Java не запускается. Проверьте аргументы запуска и путь к Java в настройках лаунчера.",
                    )
                };
                Self::new(code, text, Some(msg))
            }
            WexError::LaunchError(msg) => {
                let lower = msg.to_lowercase();
                if lower.contains("memory") || lower.contains("памяти") || lower.contains("heap") {
                    Self::new(
                        "LAUNCH_OUT_OF_MEMORY",
                        "Недостаточно оперативной памяти для запуска игры. Уменьшите объём памяти в настройках лаунчера.",
                        Some(msg),
                    )
                } else {
                    Self::new(
                        "LAUNCH_FAILED",
                        "Не удалось запустить игру. Проверьте настройки лаунчера и повторите запуск.",
                        Some(msg),
                    )
                }
            }
            WexError::ProcessFailed(code) => {
                if code == -805306369 {
                    Self::new(
                        "LAUNCH_OUT_OF_MEMORY",
                        "Игре не хватило оперативной памяти. Увеличьте объём выделенной памяти в настройках лаунчера.",
                        Some(format!("Process exit code: {}", code)),
                    )
                } else {
                    Self::new(
                        "LAUNCH_PROCESS_FAILED",
                        "Игра неожиданно завершила работу. Проверьте настройки графики и выделенную память в настройках.",
                        Some(format!("Process exit code: {}", code)),
                    )
                }
            }
            WexError::Server(msg) => Self::new("OWN_SERVER", msg, None),
            WexError::Cancelled => Self::new(
                "OPERATION_CANCELLED",
                "Операция отменена. Вы можете повторить её в любое время.",
                None,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANNED_WORDS: &[&str] = &[
        "reqwest",
        "ipc",
        "os error",
        "exit code",
        "status code",
        "exception",
        "errno",
        "nullpointer",
        "stacktrace",
        "failed to",
        "error:",
        "c:\\users\\",
        "/home/",
        "/users/",
    ];

    fn assert_clean_russian(msg: &str) {
        let lower = msg.to_lowercase();
        for word in BANNED_WORDS {
            assert!(
                !lower.contains(word),
                "User-facing message contains banned word '{}': {}",
                word,
                msg
            );
        }
        assert!(
            msg.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)),
            "Message must be localized in Russian: {}",
            msg
        );
    }

    #[test]
    fn test_wex_error_to_ipc_error_conversions() {
        let err = WexError::JavaError("java binary not found in PATH".into());
        let ipc: wex_ipc::error::IpcError = err.into();
        assert_eq!(ipc.code, "JAVA_NOT_FOUND");
        assert_clean_russian(&ipc.message);
        assert!(ipc.technical.unwrap().contains("not found"));

        let err2 = WexError::ProcessFailed(1);
        let ipc2: wex_ipc::error::IpcError = err2.into();
        assert_eq!(ipc2.code, "LAUNCH_PROCESS_FAILED");
        assert_clean_russian(&ipc2.message);

        let err3 = WexError::HashMismatch {
            path: PathBuf::from("client.jar"),
            expected: "aaa".into(),
            actual: "bbb".into(),
        };
        let ipc3: wex_ipc::error::IpcError = err3.into();
        assert_eq!(ipc3.code, "INSTALL_HASH_MISMATCH");
        assert_clean_russian(&ipc3.message);

        let err_oom = WexError::ProcessFailed(-805306369);
        let ipc_oom: wex_ipc::error::IpcError = err_oom.into();
        assert_eq!(ipc_oom.code, "LAUNCH_OUT_OF_MEMORY");
        assert_clean_russian(&ipc_oom.message);

        let err_disk = WexError::DiskFull {
            path: PathBuf::from("C:\\Users\\SecretUser\\test.jar"),
            source: std::io::Error::new(std::io::ErrorKind::StorageFull, "disk full"),
        };
        let ipc_disk: wex_ipc::error::IpcError = err_disk.into();
        assert_eq!(ipc_disk.code, "IO_DISK_FULL");
        assert_clean_russian(&ipc_disk.message);
        // Ensure user path is NOT in user-facing message
        assert!(!ipc_disk.message.contains("SecretUser"));
    }
}
