#[cfg(all(windows, target_env = "msvc"))]
#[link(name = "resource", kind = "static")]
extern "C" {}

use tauri::test::{get_ipc_response, mock_builder, mock_context, noop_assets, INVOKE_KEY};
use tauri::{ipc::InvokeBody, webview::InvokeRequest, WebviewWindowBuilder};
use wex_app_lib::commands::friends::{self, FriendsState};
use wex_core::config::WexPaths;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn auth() -> serde_json::Value {
    serde_json::json!({"user_id":"test-user","user_token":"test-token","friend_code":"SELF","nick":"Alice"})
}

async fn invoke_social(
    server: &MockServer,
    cmd: &str,
    body: serde_json::Value,
) -> Result<tauri::ipc::InvokeResponseBody, serde_json::Value> {
    let temp = tempfile::tempdir().unwrap();
    let app = mock_builder()
        .manage(FriendsState::new_with_client(
            WexPaths::new(Some(temp.path().join("Wex"))),
            wex_social::FriendApiClient::new(server.uri()),
        ))
        .invoke_handler(tauri::generate_handler![
            friends::social_send_friend_request,
            friends::social_set_presence
        ])
        .build(mock_context(noop_assets()))
        .unwrap();
    // MockRuntime does not create an OS window or invoke the real opener.
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    get_ipc_response(
        &webview,
        InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        },
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn friend_link_arguments_are_deserialized_by_the_actual_tauri_handler() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/friends/request"))
        .and(body_partial_json(serde_json::json!({"friendCode":"#AB12"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"message":"OK"})))
        .expect(1)
        .mount(&server)
        .await;
    let wrong = invoke_social(
        &server,
        "social_send_friend_request",
        serde_json::json!({"auth":auth(),"friend_code":"#AB12"}),
    )
    .await;
    assert!(
        wrong.is_err(),
        "the old friend_code argument must not satisfy the command contract"
    );
    let correct = invoke_social(
        &server,
        "social_send_friend_request",
        serde_json::json!({"auth":auth(),"targetFriendCode":"#AB12"}),
    )
    .await;
    assert!(
        correct.is_ok(),
        "camelCase frontend argument must reach the real API client: {correct:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn presence_api_failure_is_returned_to_the_frontend() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/user/sync"))
        .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
        .expect(1)
        .mount(&server)
        .await;
    let result = invoke_social(&server, "social_set_presence", serde_json::json!({"auth":auth(),"presence":{"status":"Playing","game_version":"1.20.1","world_name":null,"lobby_code":null}})).await;
    assert!(
        result.is_err(),
        "presence publication must not return success when the API fails"
    );
}
