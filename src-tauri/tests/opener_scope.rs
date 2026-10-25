//! The opener plugin as the webview calls it, with the real
//! `tauri.conf.json` and `capabilities/default.json`: the HubRise
//! authorize host is the only URL the UI may open, and the plugin's other
//! commands are not granted. (The old shell plugin refused every call, its
//! `open` switch being off, and the UI hid the refusal.)

use serde_json::{Value, json};
use tauri::WebviewWindowBuilder;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;

fn call(cmd: &str, args: Value) -> Result<(), String> {
    let app = mock_builder()
        .plugin(tauri_plugin_opener::init())
        .build(tauri::generate_context!())
        .expect("app");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("window");
    get_ipc_response(
        &window,
        InvokeRequest {
            cmd: cmd.into(),
            callback: CallbackFn(0),
            error: CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: InvokeBody::Json(args),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_string(),
        },
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[test]
fn a_foreign_url_is_refused() {
    let err = call(
        "plugin:opener|open_url",
        json!({ "url": "https://evil.example/oauth2/v1/authorize" }),
    )
    .expect_err("out of scope");
    assert!(err.contains("Not allowed"), "{err}");
}

#[test]
fn a_file_path_is_refused() {
    let err = call("plugin:opener|open_path", json!({ "path": "/etc/passwd" }))
        .expect_err("no permission");
    assert!(
        err.contains("not allowed") || err.contains("Not allowed"),
        "{err}"
    );
}
