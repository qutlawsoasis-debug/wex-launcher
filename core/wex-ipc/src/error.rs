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
            _ => "IO_ERROR",
        };
        let message = match err.kind() {
            std::io::ErrorKind::NotFound => "Файл или каталог не найден на диске",
            std::io::ErrorKind::PermissionDenied => "Доступ к файлу запрещён операционной системой",
            std::io::ErrorKind::AlreadyExists => "Файл или каталог уже существует",
            std::io::ErrorKind::TimedOut => "Истекло время ожидания файловой операции",
            _ => "Произошла ошибка при выполнении операции ввода-вывода на диске",
        };
        Self::new(code, message, Some(err.to_string()))
    }
}

impl From<serde_json::Error> for IpcError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(
            "SERIALIZATION_ERROR",
            "Ошибка сериализации или разбора формата данных (JSON)",
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
                    "Таймаут подключения к сервису лобби"
                } else {
                    "Сетевая ошибка при связи с сервисом лобби"
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
                let text = if *status >= 500 {
                    "Сервер Lobby API временно недоступен"
                } else if !message.is_empty() {
                    message.as_str()
                } else {
                    "Ошибка обращения к сервису лобби"
                };
                return Self::new(code, text, Some(format!("HTTP {status}: {message}")));
            }
            LobbyError::Json(_) => (
                "NET_LOBBY_JSON_ERROR",
                "Некорректный формат ответа от сервиса лобби",
            ),
            LobbyError::Url(_) => (
                "NET_LOBBY_URL_ERROR",
                "Некорректный адрес URL сервиса лобби",
            ),
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
                "SOCIAL_NETWORK_ERROR",
                if e.is_timeout() {
                    "Истекло время ожидания ответа сервиса друзей"
                } else {
                    "Сетевая ошибка при связи с сервисом друзей"
                },
            ),
            FriendError::Api { status, message } => {
                let code = match status.as_u16() {
                    401 | 403 => "SOCIAL_UNAUTHORIZED",
                    404 => "SOCIAL_NOT_FOUND",
                    _ => "SOCIAL_API_ERROR",
                };
                return Self::new(code, message.as_str(), Some(format!("HTTP {status}: {message}")));
            }
            FriendError::Json(_) => (
                "SOCIAL_JSON_ERROR",
                "Ошибка разбора структуры ответа сервиса друзей",
            ),
            FriendError::Unauthorized => (
                "SOCIAL_UNAUTHORIZED",
                "Ошибка авторизации пользователя: отсутствует токен",
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
                "Ошибка дисковых операций хранилища достижений",
            ),
            AchievementError::Json(_) => (
                "ACHIEVEMENT_SERIALIZE_ERROR",
                "Ошибка сериализации/разбора файла достижений",
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
                "Указанный каталог или файл мира не найден",
            ),
            WorkshopError::Io(_) => (
                "WORKSHOP_IO_ERROR",
                "Ошибка файловой системы при работе с мирами или скриншотами",
            ),
            WorkshopError::Zip(_) => (
                "WORKSHOP_ZIP_ERROR",
                "Ошибка создания или распаковки ZIP-архива мира",
            ),
            WorkshopError::InvalidLevelDat(_) => (
                "WORKSHOP_INVALID_LEVEL_DAT",
                "Файл level.dat повреждён или имеет неподдерживаемый формат NBT",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}

impl From<wex_social::launcher_update::UpdateError> for IpcError {
    fn from(err: wex_social::launcher_update::UpdateError) -> Self {
        use wex_social::launcher_update::UpdateError;
        let (code, msg) = match &err {
            UpdateError::Http(_) => (
                "UPDATE_NETWORK_ERROR",
                "Не удалось загрузить манифест обновлений лаунчера",
            ),
            UpdateError::Json(_) => (
                "UPDATE_INVALID_MANIFEST",
                "Файл манифеста обновлений повреждён или имеет неверный формат JSON",
            ),
            UpdateError::PlatformNotFound(_) => (
                "UPDATE_PLATFORM_NOT_FOUND",
                "Обновление для текущей операционной системы отсутствует в манифесте",
            ),
            UpdateError::InvalidSignature(_) => (
                "UPDATE_SIGNATURE_FAILED",
                "Цифровая подпись обновления лаунчера не прошла проверку подлинности",
            ),
        };
        Self::new(code, msg, Some(err.to_string()))
    }
}
