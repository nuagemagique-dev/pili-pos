//! The installation's HubRise client comes from the `hubrise` section
//! of `pos.json`, its secret from a file. Without the section, a
//! release build cannot connect; a development build falls back to
//! the `HUBRISE_*` variables or the local mock.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::TestApp;
use common::hubrise::HubriseMock;
use pos_core::config::{HubriseConfig, PosConfig};
use pos_engine::commands::errors;
use pos_engine::commands::hubrise::{
    do_hubrise_complete_oauth, do_hubrise_start_oauth_flow, oauth_config,
};
use serde_json::Value;

fn section(mock: &HubriseMock, secret_file: &std::path::Path) -> HubriseConfig {
    serde_json::from_value(serde_json::json!({
        "client_id": "pili-pos",
        "client_secret_file": secret_file,
        "oauth_base_url": format!("{}/oauth2/v1", mock.url),
        "api_base_url": mock.api(),
    }))
    .unwrap()
}

#[test]
fn a_release_build_without_the_section_cannot_connect() {
    let refused = oauth_config(&PosConfig::default(), false).unwrap_err();
    assert_eq!(refused, errors::hubrise::NOT_CONFIGURED);
    // A development build goes on with the mock or `.env`.
    assert!(oauth_config(&PosConfig::default(), true).is_ok());
}

#[tokio::test]
async fn the_section_sets_the_client_and_its_secret_comes_from_its_file() {
    let mock = HubriseMock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("hubrise.secret");
    std::fs::write(&secret, "s3cret\n").unwrap();
    let config = PosConfig {
        hubrise: Some(section(&mock, &secret)),
        ..PosConfig::default()
    };
    // The section wins over the development fallback.
    let oauth = oauth_config(&config, true).unwrap();
    assert_eq!(oauth.client_id, "pili-pos");
    assert_eq!(oauth.client_secret, "s3cret");
    assert_eq!(oauth.base_url, mock.api());
    assert_eq!(oauth.redirect_uri, "urn:ietf:wg:oauth:2.0:oob");
}

#[tokio::test]
async fn an_unreadable_secret_is_refused_never_replaced() {
    let mock = HubriseMock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let config = PosConfig {
        hubrise: Some(section(&mock, &dir.path().join("missing.secret"))),
        ..PosConfig::default()
    };
    for dev in [false, true] {
        let refused = oauth_config(&config, dev).unwrap_err();
        assert_eq!(refused, errors::hubrise::SECRET_UNREADABLE);
    }
}

#[tokio::test]
async fn an_installation_connects_with_its_pos_json_client() {
    let (app, _db) = TestApp::with_shared_db().await;
    let mock = HubriseMock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("hubrise.secret");
    std::fs::write(&secret, "s3cret").unwrap();
    app.state.config.lock().unwrap().hubrise = Some(section(&mock, &secret));

    // Switched off, the installation does not connect.
    app.state.hubrise_switch.set(false);
    assert_eq!(
        do_hubrise_start_oauth_flow(&app.state).unwrap_err(),
        errors::hubrise::DISABLED
    );
    app.state.hubrise_switch.set(true);

    // Connecting is an admin's setting.
    *app.state.session.lock().unwrap() = None;
    assert_eq!(
        do_hubrise_start_oauth_flow(&app.state).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    app.login_as("12345678958", pos_core::user::UserRole::Cashier);
    assert_eq!(
        do_hubrise_start_oauth_flow(&app.state).unwrap_err(),
        errors::FORBIDDEN
    );
    assert_eq!(
        do_hubrise_complete_oauth(&app.state, "any-code")
            .await
            .unwrap_err(),
        errors::FORBIDDEN
    );
    app.login_as("85073003328", pos_core::user::UserRole::Admin);

    let link = do_hubrise_start_oauth_flow(&app.state).unwrap();
    // The kiosk has no browser: the link comes as a QR code too.
    assert!(link.qr_svg.starts_with("<svg"), "{}", &link.qr_svg[..40]);
    assert!(!link.qr_svg.contains("s3cret"));
    let url = link.url;
    assert!(
        url.starts_with(&format!("{}/oauth2/v1/authorize?", mock.url)),
        "{url}"
    );
    assert!(url.contains("client_id=pili-pos"), "{url}");
    assert!(!url.contains("s3cret"), "the secret never leaves in a URL");

    // The installer logs in; HubRise shows the code to paste back.
    let shown: Value = reqwest::get(&url).await.unwrap().json().await.unwrap();
    let code = shown["code"].as_str().unwrap();
    let connected = do_hubrise_complete_oauth(&app.state, code).await.unwrap();
    assert!(connected.connected);

    let stored_base: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT base_url FROM hubrise_connection", [], |r| r.get(0))
        .unwrap();
    assert_eq!(stored_base, mock.api(), "the API of the pos.json section");
}

/// A code HubRise does not accept is a coded refusal (said in the
/// screen's language, HubRise's own words as the detail), never a bare
/// English sentence.
#[tokio::test]
async fn a_refused_connection_code_is_a_coded_error() {
    let (app, _db) = TestApp::with_shared_db().await;
    let mock = HubriseMock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("hubrise.secret");
    std::fs::write(&secret, "s3cret").unwrap();
    app.state.config.lock().unwrap().hubrise = Some(section(&mock, &secret));
    app.state.hubrise_switch.set(true);
    app.login_as("85073003328", pos_core::user::UserRole::Admin);

    let refused = do_hubrise_complete_oauth(&app.state, "not-a-code")
        .await
        .unwrap_err();
    assert!(
        refused.starts_with(&format!("{}|detail=", errors::hubrise::OAUTH_FAILED)),
        "{refused}"
    );
}
