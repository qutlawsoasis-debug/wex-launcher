use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use wex_ipc::models::DeepLinkActionDto;

pub const EVT_DEEP_LINK_TRIGGERED: &str = "wex://deep-link";

/// Internal parsed representation of a deep link action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLinkAction {
    JoinLobby { code: String },
    ConnectServer { address: String },
    AddFriend { code: String },
}

impl DeepLinkAction {
    pub fn to_dto(&self, raw_uri: &str) -> DeepLinkActionDto {
        match self {
            DeepLinkAction::JoinLobby { code } => DeepLinkActionDto {
                kind: "join".to_string(),
                target: code.clone(),
                raw_uri: raw_uri.to_string(),
                title: format!("Войти в лобби {}?", code),
                subtitle: "Подключиться к игре хоста по ссылке?".to_string(),
                confirm_text: "Войти".to_string(),
            },
            DeepLinkAction::ConnectServer { address } => DeepLinkActionDto {
                kind: "server".to_string(),
                target: address.clone(),
                raw_uri: raw_uri.to_string(),
                title: format!("Подключиться к серверу {}?", address),
                subtitle: "Запустить Minecraft и подключиться к серверу по ссылке?".to_string(),
                confirm_text: "Подключиться".to_string(),
            },
            DeepLinkAction::AddFriend { code } => DeepLinkActionDto {
                kind: "friend".to_string(),
                target: code.clone(),
                raw_uri: raw_uri.to_string(),
                title: format!("Отправить заявку в друзья игроку {}?", code),
                subtitle: "Добавить пользователя в список друзей?".to_string(),
                confirm_text: "Отправить".to_string(),
            },
        }
    }
}

/// State holding any pending deep link action waiting for user confirmation.
#[derive(Default)]
pub struct DeepLinkState {
    pub pending: Mutex<Option<DeepLinkActionDto>>,
}

impl DeepLinkState {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(None),
        }
    }

    pub fn set_pending(&self, action: DeepLinkActionDto) {
        if let Ok(mut lock) = self.pending.lock() {
            *lock = Some(action);
        }
    }

    pub fn get_pending(&self) -> Option<DeepLinkActionDto> {
        self.pending.lock().ok().and_then(|lock| lock.clone())
    }

    pub fn clear(&self) {
        if let Ok(mut lock) = self.pending.lock() {
            *lock = None;
        }
    }
}

/// Parse and validate a raw deep link URI (`wex://...` or `aura://...`).
pub fn parse_deep_link(raw: &str) -> Result<DeepLinkAction, String> {
    let trimmed = raw.trim().trim_matches(|c| c == '\'' || c == '"').trim();
    if trimmed.is_empty() {
        return Err("Пустая ссылка протокола".to_string());
    }

    // Scheme extraction (wex:// or aura:// or wex: / aura:)
    let after_scheme = if let Some(rest) = strip_prefix_case_insensitive(trimmed, "wex://") {
        rest
    } else if let Some(rest) = strip_prefix_case_insensitive(trimmed, "aura://") {
        rest
    } else if let Some(rest) = strip_prefix_case_insensitive(trimmed, "wex:") {
        rest
    } else if let Some(rest) = strip_prefix_case_insensitive(trimmed, "aura:") {
        rest
    } else {
        return Err(format!("Неподдерживаемая схема протокола: {}", trimmed));
    };

    let cleaned = after_scheme.trim_matches('/');
    if cleaned.is_empty() {
        return Err("Отсутствуют параметры ссылки".to_string());
    }

    let parts: Vec<&str> = cleaned.splitn(2, '/').collect();
    if parts.len() < 2 {
        return Err(
            "Недостаточно параметров в ссылке (ожидается <действие>/<значение>)".to_string(),
        );
    }

    let action = parts[0].trim().to_ascii_lowercase();
    let value = parts[1].trim();

    match action.as_str() {
        "join" => {
            let code = value.to_ascii_uppercase();
            if !validate_code_format(&code) {
                return Err(format!("Некорректный код лобби: {}", code));
            }
            Ok(DeepLinkAction::JoinLobby { code })
        }
        "server" => {
            let address = value.trim_matches('/').to_string();
            if !validate_server_address(&address) {
                return Err(format!("Некорректный адрес сервера: {}", address));
            }
            Ok(DeepLinkAction::ConnectServer { address })
        }
        "friend" => {
            let code = value.to_ascii_uppercase();
            if !validate_code_format(&code) && !validate_nickname(value) {
                return Err(format!("Некорректный код или никнейм друга: {}", value));
            }
            Ok(DeepLinkAction::AddFriend { code })
        }
        unknown => Err(format!("Неизвестное действие протокола: {}", unknown)),
    }
}

fn strip_prefix_case_insensitive<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Matches `^[A-Z0-9]{6,8}$` from old launcher specification.
pub fn validate_code_format(code: &str) -> bool {
    let len = code.len();
    if !(6..=8).contains(&len) {
        return false;
    }
    code.chars()
        .all(|c| c.is_ascii_alphanumeric() && (c.is_ascii_digit() || c.is_ascii_uppercase()))
}

/// Validates Minecraft server address (`hostname[:port]` or `ip[:port]`).
pub fn validate_server_address(addr: &str) -> bool {
    if addr.is_empty() || addr.len() > 255 {
        return false;
    }
    // Disallow spaces, control characters, shell metacharacters
    if addr.chars().any(|c| {
        c.is_whitespace() || c == ';' || c == '&' || c == '|' || c == '`' || c == '$' || c == '\\'
    }) {
        return false;
    }

    let parts: Vec<&str> = addr.split(':').collect();
    match parts.len() {
        1 => {
            let host = parts[0];
            !host.is_empty() && !host.starts_with('.') && !host.ends_with('.')
        }
        2 => {
            let host = parts[0];
            let port_str = parts[1];
            if host.is_empty() || host.starts_with('.') || host.ends_with('.') {
                return false;
            }
            if let Ok(port) = port_str.parse::<u16>() {
                port > 0
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn validate_nickname(nick: &str) -> bool {
    let len = nick.len();
    (3..=16).contains(&len) && nick.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Parse a raw deep link URI into a structured action DTO with UI presentation texts.
#[tauri::command]
pub fn parse_deep_link_uri(uri: String) -> Result<DeepLinkActionDto, String> {
    let action = parse_deep_link(&uri)?;
    Ok(action.to_dto(&uri))
}

/// Get currently pending deep link action (if app was launched with one or received in background).
#[tauri::command]
pub fn get_pending_deep_link(state: State<'_, DeepLinkState>) -> Option<DeepLinkActionDto> {
    state.get_pending()
}

/// Clear pending deep link once processed or cancelled by player.
#[tauri::command]
pub fn clear_pending_deep_link(state: State<'_, DeepLinkState>) {
    state.clear();
}

/// Submit an incoming deep link at runtime, storing it as pending and emitting `wex://deep-link`.
#[tauri::command]
pub fn handle_incoming_deep_link(
    app: AppHandle,
    uri: String,
    state: State<'_, DeepLinkState>,
) -> Result<DeepLinkActionDto, String> {
    let action = parse_deep_link(&uri)?;
    let dto = action.to_dto(&uri);
    state.set_pending(dto.clone());

    // Bring launcher window to focus if possible
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    let _ = app.emit(EVT_DEEP_LINK_TRIGGERED, dto.clone());
    Ok(dto)
}

/// Register `wex` and `aura` URL protocol schemes via `tauri-plugin-deep-link` / OS.
#[tauri::command]
pub fn register_deep_link_schemes(app: AppHandle) -> Result<(), String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_deep_link::DeepLinkExt;
        app.deep_link().register_all().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_join_wex_scheme() {
        let action = parse_deep_link("wex://join/ABC1234").unwrap();
        assert_eq!(
            action,
            DeepLinkAction::JoinLobby {
                code: "ABC1234".to_string()
            }
        );

        let dto = action.to_dto("wex://join/ABC1234");
        assert_eq!(dto.kind, "join");
        assert_eq!(dto.target, "ABC1234");
        assert_eq!(dto.title, "Войти в лобби ABC1234?");
        assert_eq!(dto.confirm_text, "Войти");
    }

    #[test]
    fn test_parse_valid_join_aura_scheme() {
        let action = parse_deep_link("aura://join/ROOM42").unwrap();
        assert_eq!(
            action,
            DeepLinkAction::JoinLobby {
                code: "ROOM42".to_string()
            }
        );
    }

    #[test]
    fn test_parse_valid_server_scheme() {
        let action = parse_deep_link("wex://server/play.hypixel.net:25565").unwrap();
        assert_eq!(
            action,
            DeepLinkAction::ConnectServer {
                address: "play.hypixel.net:25565".to_string()
            }
        );

        let dto = action.to_dto("wex://server/play.hypixel.net:25565");
        assert_eq!(dto.kind, "server");
        assert_eq!(dto.target, "play.hypixel.net:25565");
        assert_eq!(dto.title, "Подключиться к серверу play.hypixel.net:25565?");
        assert_eq!(dto.confirm_text, "Подключиться");
    }

    #[test]
    fn test_parse_valid_server_scheme_ip_only() {
        let action = parse_deep_link("wex://server/192.168.1.100").unwrap();
        assert_eq!(
            action,
            DeepLinkAction::ConnectServer {
                address: "192.168.1.100".to_string()
            }
        );
    }

    #[test]
    fn test_parse_valid_friend_scheme() {
        let action = parse_deep_link("wex://friend/MAJ372WR").unwrap();
        assert_eq!(
            action,
            DeepLinkAction::AddFriend {
                code: "MAJ372WR".to_string()
            }
        );

        let dto = action.to_dto("wex://friend/MAJ372WR");
        assert_eq!(dto.kind, "friend");
        assert_eq!(dto.target, "MAJ372WR");
        assert_eq!(dto.title, "Отправить заявку в друзья игроку MAJ372WR?");
        assert_eq!(dto.confirm_text, "Отправить");
    }

    #[test]
    fn test_reject_invalid_scheme() {
        let err = parse_deep_link("https://example.com/join/ABC1234").unwrap_err();
        assert!(err.contains("Неподдерживаемая схема"));
    }

    #[test]
    fn test_reject_empty_and_insufficient_parts() {
        assert!(parse_deep_link("").is_err());
        assert!(parse_deep_link("wex://").is_err());
        assert!(parse_deep_link("wex://join").is_err());
    }

    #[test]
    fn test_reject_invalid_lobby_code() {
        // Less than 6 chars
        assert!(parse_deep_link("wex://join/AB").is_err());
        // More than 8 chars
        assert!(parse_deep_link("wex://join/ABCDEFGHIJ").is_err());
        // Special characters / injection
        assert!(parse_deep_link("wex://join/ABC$123").is_err());
    }

    #[test]
    fn test_reject_invalid_server_address() {
        // Contains space
        assert!(parse_deep_link("wex://server/play hypixel net").is_err());
        // Shell injection attempt
        assert!(parse_deep_link("wex://server/play;rm -rf /").is_err());
        // Invalid port
        assert!(parse_deep_link("wex://server/mc.net:999999").is_err());
    }

    #[test]
    fn test_deep_link_state_management() {
        let state = DeepLinkState::new();
        assert!(state.get_pending().is_none());

        let action = parse_deep_link("wex://join/LOBBY99").unwrap();
        let dto = action.to_dto("wex://join/LOBBY99");
        state.set_pending(dto.clone());

        let retrieved = state.get_pending().unwrap();
        assert_eq!(retrieved.target, "LOBBY99");
        assert_eq!(retrieved.kind, "join");

        state.clear();
        assert!(state.get_pending().is_none());
    }
}
