use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

/// Result alias for IPC operations.
pub type IpcResult<T> = Result<T, IpcError>;

/// Unified error payload exchanged between Rust core/services and UI.
///
/// Contains an uppercase alphanumeric error code, a localized human-readable
/// explanation in Russian for end-user notifications/toasts, and optional
/// technical details (exceptions, stack traces, raw error codes) for logs.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Error)]
pub struct IpcError {
    /// Machine-readable error code (e.g. "NET_PING_TIMEOUT", "AUTH_INVALID_TOKEN").
    pub code: String,
    /// User-friendly Russian explanation suitable for display in UI toasts or dialogs.
    pub message: String,
    /// Detailed technical information for debugging, crash analytics, and logging.
    pub technical: Option<String>,
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.technical {
            Some(tech) => write!(f, "[{}] {} ({})", self.code, self.message, tech),
            None => write!(f, "[{}] {}", self.code, self.message),
        }
    }
}

impl IpcError {
    /// Create a new IPC error with full details.
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        technical: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            technical,
        }
    }

    /// Helper for "not found" errors.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new("NOT_FOUND", message, None)
    }

    /// Helper for invalid client requests or parameters.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new("INVALID_ARGUMENT", message, None)
    }

    /// Helper for unexpected internal errors.
    pub fn internal(message: impl Into<String>, technical: Option<String>) -> Self {
        Self::new("INTERNAL_ERROR", message, technical)
    }

    /// Helper for timeout errors.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new("TIMEOUT", message, None)
    }

    /// Helper for launch errors.
    pub fn launch(
        code: impl Into<String>,
        message: impl Into<String>,
        technical: Option<String>,
    ) -> Self {
        Self::new(code, message, technical)
    }

    /// Helper for installation errors.
    pub fn install(
        code: impl Into<String>,
        message: impl Into<String>,
        technical: Option<String>,
    ) -> Self {
        Self::new(code, message, technical)
    }

    /// Helper for Java runtime errors.
    pub fn java(
        code: impl Into<String>,
        message: impl Into<String>,
        technical: Option<String>,
    ) -> Self {
        Self::new(code, message, technical)
    }
}

// ============================================================================
// Standard Library Conversions
// ============================================================================

impl From<std::io::Error> for IpcError {
    fn from(err: std::io::Error) -> Self {
        let code = match err.kind() {
            std::io::ErrorKind::NotFound => "IO_NOT_FOUND",
            std::io::ErrorKind::PermissionDenied => "IO_PERMISSION_DENIED",
            std::io::ErrorKind::AlreadyExists => "IO_ALREADY_EXISTS",
            std::io::ErrorKind::TimedOut => "IO_TIMED_OUT",
            std::io::ErrorKind::StorageFull => "IO_DISK_FULL",
            _ => {
                #[cfg(windows)]
                if matches!(err.raw_os_error(), Some(112) | Some(39)) {
                    "IO_DISK_FULL"
                } else if matches!(err.raw_os_error(), Some(5)) {
                    "IO_PERMISSION_DENIED"
                } else {
                    "IO_ERROR"
                }
                #[cfg(not(windows))]
                if matches!(err.raw_os_error(), Some(28)) {
                    "IO_DISK_FULL"
                } else if matches!(err.raw_os_error(), Some(13)) {
                    "IO_PERMISSION_DENIED"
                } else {
                    "IO_ERROR"
                }
            }
        };
        let message = match code {
            "IO_NOT_FOUND" => "Файл не найден на диске. Проверьте целостность файлов игры.",
            "IO_PERMISSION_DENIED" => "Антивирус или система заблокировали доступ к файлу. Добавьте лаунчер в исключения антивируса и повторите попытку.",
            "IO_ALREADY_EXISTS" => "Файл с таким именем уже существует. Выберите другое имя или удалите старый файл.",
            "IO_TIMED_OUT" => "Диск или сервер не отвечает вовремя. Подождите немного и повторите попытку.",
            "IO_DISK_FULL" => "На диске закончилось свободное место. Освободите место на диске и повторите попытку.",
            _ => "Не удалось выполнить операцию с файлами на диске. Проверьте права доступа и свободное место на диске.",
        };
        Self::new(code, message, Some(err.to_string()))
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(
            "SERIALIZATION_ERROR",
            "Файл настроек повреждён. Лаунчер восстановит параметры по умолчанию.",
            Some(err.to_string()),
        )
    }
}

// ============================================================================
// wex-net Conversions
// ============================================================================

impl From<wex_net::lobby::LobbyError> for IpcError {
    fn from(err: wex_net::lobby::LobbyError) -> Self {
        use wex_net::lobby::LobbyError;
        let (code, msg) = match &err {
            LobbyError::Reqwest(e) => (
                "NET_LOBBY_NETWORK_ERROR",
                if e.is_timeout() {
                    "Сервер комнат не отвечает на запрос. Проверьте интернет-соединение или повторите попытку позже."
                } else {
                    "Сервис комнат недоступен из-за проблем с сетью. Проверьте подключение к интернету и повторите попытку."
                },
            ),
            LobbyError::Api { status, message } => {
                let code = if *status == 404 {
                    "NET_LOBBY_NOT_FOUND"
                } else if *status == 401 || *status == 403 {
                    "NET_LOBBY_UNAUTHORIZED"
                } else {
                    "NET_LOBBY_API_ERROR"
                };
                let text = if *status == 404 {
                    "Комната не найдена или уже закрыта. Проверьте код комнаты или создайте новую."
                } else if *status == 401 || *status == 403 {
                    "Доступ к комнате ограничен. Попросите организатора отправить вам новое приглашение."
                } else if *status >= 500 {
                    "Сервер комнат временно недоступен. Попробуйте подключиться через несколько минут."
                } else if !message.is_empty() {
                    "Не удалось подключиться к комнате. Проверьте код комнаты и попробуйте снова."
                } else {
                    "Не удалось выполнить действие в комнате. Попробуйте повторить попытку позже."
                };
                return Self::new(code, text, Some(format!("HTTP {status}: {message}")));
            }
            LobbyError::Json(_) => (
                "NET_LOBBY_JSON_ERROR",
                "Сервер комнат вернул некорректный ответ. Попробуйте повторить попытку позже.",
            ),
            LobbyError::Url(_) => (
                "NET_LOBBY_URL_ERROR",
                "Указан неверный адрес сервера комнат. Проверьте настройки подключения.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_net::ping::PingError> for IpcError {
    fn from(err: wex_net::ping::PingError) -> Self {
        use wex_net::ping::PingError;
        let (code, msg) = match &err {
            PingError::Timeout => (
                "IO_TIMED_OUT",
                "Сервер не отвечает на запрос. Проверьте адрес сервера или состояние сети.",
            ),
            PingError::Dns(_) => (
                "NET_DNS_ERROR",
                "Не удалось найти сервер по указанному адресу. Проверьте правильность написания адреса.",
            ),
            PingError::Io(e) => {
                if e.kind() == std::io::ErrorKind::TimedOut {
                    (
                        "IO_TIMED_OUT",
                        "Сервер не отвечает на запрос. Проверьте интернет-соединение или попробуйте позже.",
                    )
                } else {
                    (
                        "NETWORK_ERROR",
                        "Не удалось подключиться к серверу. Проверьте адрес сервера и подключение к интернету.",
                    )
                }
            }
            PingError::Protocol(_) | PingError::Json(_) => (
                "NET_PROTOCOL_ERROR",
                "Сервер вернул ответ в неподдерживаемом формате. Проверьте версию сервера.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_net::tunnel::TunnelError> for IpcError {
    fn from(err: wex_net::tunnel::TunnelError) -> Self {
        use wex_net::tunnel::TunnelError;
        let (code, msg) = match &err {
            TunnelError::ChecksumMismatch { .. } => (
                "INSTALL_HASH_MISMATCH",
                "Файл сетевого модуля повреждён при скачивании. Лаунчер скачает его заново.",
            ),
            TunnelError::ProcessSpawn(_) => (
                "NET_TUNNEL_ERROR",
                "Не удалось запустить сетевой туннель. Проверьте исключения брандмауэра и антивируса.",
            ),
            TunnelError::JobObject(_) => (
                "NET_TUNNEL_ERROR",
                "Система ограничила запуск сетевого процесса. Запустите лаунчер от имени администратора.",
            ),
            TunnelError::Reqwest(e) => {
                if e.is_timeout() {
                    (
                        "IO_TIMED_OUT",
                        "Сервер не отвечает на запрос. Проверьте интернет-соединение или повторите попытку позже.",
                    )
                } else {
                    (
                        "NETWORK_ERROR",
                        "Нет подключения к интернету. Проверьте сеть и повторите попытку.",
                    )
                }
            }
            TunnelError::Io(e) => return std::io::Error::new(e.kind(), e.to_string()).into(),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

// ============================================================================
// wex-social Conversions
// ============================================================================

impl From<wex_social::friends::FriendError> for IpcError {
    fn from(err: wex_social::friends::FriendError) -> Self {
        use wex_social::friends::FriendError;
        let (code, msg) = match &err {
            FriendError::Http(e) => (
                if e.is_timeout() {
                    "IO_TIMED_OUT"
                } else {
                    "NETWORK_ERROR"
                },
                if e.is_timeout() {
                    "Сервер друзей не отвечает на запрос. Проверьте подключение к интернету или попробуйте позже."
                } else {
                    "Нет подключения к интернету. Проверьте сетевое соединение и повторите попытку."
                },
            ),
            FriendError::Api { status, message } => {
                let code = match status.as_u16() {
                    401 | 403 => "SOCIAL_UNAUTHORIZED",
                    404 => "SOCIAL_NOT_FOUND",
                    _ => "SOCIAL_API_ERROR",
                };
                let text = match status.as_u16() {
                    401 | 403 => {
                        "Сессия пользователя устарела. Войдите в профиль заново в настройках."
                    }
                    404 => "Игрок или заявка не найдены. Проверьте никнейм или код друга.",
                    _ => "Не удалось обновить список друзей. Попробуйте повторить попытку позже.",
                };
                return Self::new(code, text, Some(format!("HTTP {status}: {message}")));
            }
            FriendError::Json(_) => (
                "SOCIAL_JSON_ERROR",
                "Сервер друзей вернул некорректные данные. Попробуйте повторить попытку позже.",
            ),
            FriendError::Unauthorized => (
                "SOCIAL_UNAUTHORIZED",
                "Для этого действия требуется прохождение авторизации. Войдите в профиль в настройках лаунчера.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_social::achievements::AchievementError> for IpcError {
    fn from(err: wex_social::achievements::AchievementError) -> Self {
        use wex_social::achievements::AchievementError;
        let (code, msg) = match &err {
            AchievementError::Io(_) => (
                "ACHIEVEMENT_IO_ERROR",
                "Не удалось сохранить прогресс достижений на диск. Проверьте свободное место на диске.",
            ),
            AchievementError::Json(_) => (
                "ACHIEVEMENT_SERIALIZE_ERROR",
                "Файл достижений повреждён. Лаунчер создаст новый список достижений.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_social::workshop::WorkshopError> for IpcError {
    fn from(err: wex_social::workshop::WorkshopError) -> Self {
        use wex_social::workshop::WorkshopError;
        let (code, msg) = match &err {
            WorkshopError::NotFound(_p) => (
                "WORKSHOP_NOT_FOUND",
                "Файл сохранения или мира не найден. Обновите список сохранений в Мастерской.",
            ),
            WorkshopError::Io(_) => (
                "WORKSHOP_IO_ERROR",
                "Не удалось прочитать или записать файлы сохранения. Проверьте свободное место на диске.",
            ),
            WorkshopError::Zip(_) => (
                "WORKSHOP_ZIP_ERROR",
                "Не удалось распаковать или создать архив мира. Проверьте свободное место на диске.",
            ),
            WorkshopError::InvalidLevelDat(_) => (
                "WORKSHOP_INVALID_LEVEL_DAT",
                "Файл мира повреждён или имеет неподдерживаемый формат. Восстановите сохранение из резервной копии.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_social::launcher_update::UpdateError> for IpcError {
    fn from(err: wex_social::launcher_update::UpdateError) -> Self {
        use wex_social::launcher_update::UpdateError;
        #[allow(unreachable_patterns)]
        let (code, msg) = match &err {
            UpdateError::Http(_) => (
                "UPDATE_NETWORK_ERROR",
                "Не удалось загрузить данные обновления. Проверьте подключение к интернету или попробуйте позже.",
            ),
            UpdateError::Json(_) => (
                "UPDATE_INVALID_MANIFEST",
                "Сервер обновлений вернул повреждённый ответ. Попробуйте проверить обновления позже.",
            ),
            UpdateError::PlatformNotFound(_) => (
                "UPDATE_PLATFORM_NOT_FOUND",
                "Для вашей операционной системы нет подходящей версии обновления. Обратитесь к разработчикам.",
            ),
            UpdateError::InvalidSignature(_) => (
                "UPDATE_SIGNATURE_FAILED",
                "Цифровая подпись обновления не подтверждена. Загрузка отменена в целях безопасности.",
            ),
            _ => (
                "UPDATE_UNKNOWN",
                "Неизвестная ошибка обновления.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_extras::SkinError> for IpcError {
    fn from(err: wex_extras::SkinError) -> Self {
        use wex_extras::SkinError;
        let (code, msg) = match &err {
            SkinError::ImageError(_) => (
                "SKIN_IMAGE_ERROR",
                "Не удалось открыть файл скина. Выберите изображение в формате PNG.",
            ),
            SkinError::InvalidPng(_) => (
                "SKIN_INVALID_PNG",
                "Файл скина имеет неверный формат. Выберите изображение в формате PNG.",
            ),
            SkinError::UnsupportedDimensions { .. } => (
                "SKIN_UNSUPPORTED_DIMENSIONS",
                "Неподдерживаемый размер скина. Используйте изображение размером 64x64 или 64x32 пикселей.",
            ),
            SkinError::FileTooLarge => (
                "SKIN_FILE_TOO_LARGE",
                "Файл скина слишком большой. Выберите изображение размером не более 2 МБ.",
            ),
            SkinError::Io(_) => (
                "IO_ERROR",
                "Не удалось сохранить файл скина на диск. Проверьте свободное место на диске.",
            ),
            SkinError::Json(_) => (
                "SERIALIZATION_ERROR",
                "Не удалось обновить гардероб. Попробуйте выбрать скин ещё раз.",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
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

    fn assert_clean_user_message(msg: &str) {
        let lower = msg.to_lowercase();
        for word in BANNED_WORDS {
            assert!(
                !lower.contains(word),
                "User-facing message contains banned technical word '{}': {}",
                word,
                msg
            );
        }
        // Must have Russian letters (Cyrillic)
        assert!(
            msg.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)),
            "Message must be localized in Russian: {}",
            msg
        );
    }

    #[test]
    fn test_io_errors_user_messages_are_clean_russian() {
        let errs = vec![
            std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Access is denied. (os error 5)",
            ),
            std::io::Error::new(
                std::io::ErrorKind::StorageFull,
                "There is not enough space on the disk. (os error 112)",
            ),
            std::io::Error::new(std::io::ErrorKind::TimedOut, "The operation timed out"),
            std::io::Error::new(std::io::ErrorKind::AlreadyExists, "File already exists"),
        ];

        for err in errs {
            let ipc: IpcError = err.into();
            assert_clean_user_message(&ipc.message);
        }
    }

    #[test]
    fn test_core_scenarios_no_antivirus_disk_net_words() {
        let disk_err = std::io::Error::new(std::io::ErrorKind::StorageFull, "disk full");
        let ipc_disk: IpcError = disk_err.into();
        assert_eq!(ipc_disk.code, "IO_DISK_FULL");
        assert!(ipc_disk
            .message
            .contains("На диске закончилось свободное место"));
        assert_clean_user_message(&ipc_disk.message);

        let perm_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let ipc_perm: IpcError = perm_err.into();
        assert_eq!(ipc_perm.code, "IO_PERMISSION_DENIED");
        assert!(ipc_perm
            .message
            .contains("Антивирус или система заблокировали"));
        assert_clean_user_message(&ipc_perm.message);

        let time_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        let ipc_time: IpcError = time_err.into();
        assert_eq!(ipc_time.code, "IO_TIMED_OUT");
        assert_clean_user_message(&ipc_time.message);
    }
}
