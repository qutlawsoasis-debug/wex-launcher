//! Remote documents never replace the privileged bundled launcher WebView.
pub fn allowed(url: &tauri::Url) -> bool {
    let bundled = (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost"));
    let dev = cfg!(debug_assertions)
        && url.scheme() == "http"
        && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
        && url.port() == Some(5173);
    (bundled || dev) && url.username().is_empty() && url.password().is_none()
}

#[cfg(test)]
mod tests {
    use tauri::test::{get_ipc_response, mock_builder};

    #[tauri::command]
    fn get_network_status() -> &'static str {
        "allowed"
    }

    #[tauri::command]
    fn net_tunnel_start() -> &'static str {
        "must not run"
    }

    #[test]
    fn capability_enforces_local_window_and_custom_command_allowlist() {
        let app = mock_builder()
            .invoke_handler(tauri::generate_handler![
                get_network_status,
                net_tunnel_start
            ])
            .build(tauri::generate_context!())
            .unwrap();
        let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let other = tauri::WebviewWindowBuilder::new(&app, "unprivileged", Default::default())
            .build()
            .unwrap();
        let request = |cmd: &str, url: &str| tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: url.parse().unwrap(),
            body: Default::default(),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        let origin = if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        };
        assert!(get_ipc_response(&main, request("get_network_status", origin)).is_ok());
        assert!(get_ipc_response(&main, request("net_tunnel_start", origin)).is_err());
        assert!(get_ipc_response(&other, request("get_network_status", origin)).is_err());
        assert!(get_ipc_response(
            &main,
            request("get_network_status", "https://example.invalid")
        )
        .is_err());
    }

    #[test]
    fn remote_navigation_cannot_inherit_launcher_privileges() {
        for url in [
            "https://example.com/",
            "javascript:alert(1)",
            "file:///C:/test.html",
            "https://tauri.localhost.attacker.invalid/",
            "http://localhost:9000/",
            "http://user@tauri.localhost/",
        ] {
            assert!(!super::allowed(&url.parse().unwrap()), "accepted {url}");
        }
        for url in ["tauri://localhost/index.html", "http://tauri.localhost/"] {
            assert!(super::allowed(&url.parse().unwrap()));
        }
    }
}
