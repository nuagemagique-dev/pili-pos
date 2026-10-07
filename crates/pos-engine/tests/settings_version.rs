//! `get_settings` carries the software version (`posSwVersion`, AM
//! Art. 47) for the login screen (PD38).

use pos_engine_testkit as common;

use common::*;

use pos_engine::commands::settings::do_get_settings;

#[tokio::test]
async fn get_settings_carries_the_software_version() {
    let app = TestApp::new().await;
    let settings = do_get_settings(&app.state).unwrap();
    assert_eq!(settings.pos_sw_version, "pili-pos/1.0.0");
    let json = serde_json::to_value(&settings).unwrap();
    assert_eq!(json["posSwVersion"], "pili-pos/1.0.0");
}
