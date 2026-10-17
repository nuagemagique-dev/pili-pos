//! A fiscal identity the FDM would refuse is reported by the settings
//! the UI reads at start-up, so it is fixed before the first sale.

use pos_engine_testkit as common;

use common::*;

use pos_engine::commands::settings::do_get_settings;

#[tokio::test]
async fn the_test_identity_has_no_problem() {
    let app = TestApp::new().await;
    assert_eq!(
        do_get_settings(&app.state).unwrap().config_problems,
        Vec::<String>::new()
    );
}

#[tokio::test]
async fn a_bad_identity_is_reported_key_by_key() {
    let app = TestApp::new().await;
    {
        let mut config = app.state.config.lock().unwrap();
        config.restaurant_vat = "BE0000000098".into();
        config.restaurant_est_no = "123".into();
        config.pos_id = "cfod0061234567".into();
    }
    let problems = do_get_settings(&app.state).unwrap().config_problems;
    // Each is a coded error: the setting's code, then the English text
    // of the problem as the detail for the installer.
    let codes: Vec<_> = problems
        .iter()
        .map(|p| p.split('|').next().unwrap())
        .collect();
    assert_eq!(
        codes,
        [
            "config_pos_id",
            "config_restaurant_vat",
            "config_restaurant_est_no"
        ]
    );
    assert!(
        problems[1].starts_with("config_restaurant_vat|detail=restaurant_vat: BE followed by"),
        "{}",
        problems[1]
    );
}

#[tokio::test]
async fn an_unreadable_alerts_token_file_is_reported_as_config_alerts() {
    let app = TestApp::new().await;
    app.state.config.lock().unwrap().alerts = Some(pos_core::config::AlertsConfig {
        telegram_bot_token_file: Some("/nonexistent/telegram.token".into()),
        telegram_api_base: Some("http://example.com".into()),
    });
    let problems = do_get_settings(&app.state).unwrap().config_problems;
    let codes: Vec<_> = problems
        .iter()
        .map(|p| p.split('|').next().unwrap())
        .collect();
    assert_eq!(codes, ["config_alerts", "config_alerts"]);
    assert!(
        problems[0].contains("alerts.telegram_bot_token_file: cannot read"),
        "{}",
        problems[0]
    );
    assert!(problems[1].contains("alerts.telegram_api_base: plain http"));
}

#[tokio::test]
async fn a_valid_alerts_section_has_no_problem() {
    let app = TestApp::new().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("telegram.token");
    std::fs::write(&file, "123:abc\n").unwrap();
    app.state.config.lock().unwrap().alerts = Some(pos_core::config::AlertsConfig {
        telegram_bot_token_file: Some(file.to_string_lossy().into_owned()),
        telegram_api_base: None,
    });
    assert_eq!(
        do_get_settings(&app.state).unwrap().config_problems,
        Vec::<String>::new()
    );
}
