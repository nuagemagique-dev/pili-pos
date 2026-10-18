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
async fn a_bad_peppol_section_is_reported_with_its_own_code() {
    let app = TestApp::new().await;
    app.state.config.lock().unwrap().peppol = Some(pos_core::config::PeppolConfig {
        provider: "storecove".into(),
        base_url: None,
        api_key_file: None,
        legal_entity_id: None,
        sender_id: "0208:0000000097".into(),
        supplier_street: "Rue 1".into(),
        supplier_postal_code: "1030".into(),
        supplier_city: "Bruxelles".into(),
        supplier_country: "BE".into(),
    });
    let problems = do_get_settings(&app.state).unwrap().config_problems;
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(
        problems
            .iter()
            .all(|p| p.starts_with("config_peppol|detail=peppol.")),
        "{problems:?}"
    );
}
