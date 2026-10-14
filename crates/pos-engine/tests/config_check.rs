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
