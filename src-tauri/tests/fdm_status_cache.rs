//! The header polls the FDM status from every screen that shows it.
//! Answers are reused for 20 s, so two screens polling at once, or a
//! screen mounting again, cost the FDM one query.

mod common;

use common::*;
use moyun_pos::commands::checkout::do_check_fdm_status;
use serde_json::json;
use wiremock::matchers::{body_string_contains, method};
use wiremock::{Mock, ResponseTemplate};

fn status(fdm_id: &str) -> serde_json::Value {
    json!({ "data": { "fdmStatus": {
        "device": {
            "fdmId": fdm_id,
            "fdmSwVersion": "2.2.0.5",
            "fdmDateTime": "2026-09-27T14:40:00.000Z",
            "bufferCapacityUsed": 0.0
        },
        "initialized": true,
        "informations": [],
        "warnings": [],
        "errors": []
    } } })
}

#[tokio::test]
async fn a_status_answer_is_reused_for_twenty_seconds() {
    let app = TestApp::new().await;
    Mock::given(method("POST"))
        .and(body_string_contains("status(language: FR)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status("FDM02000425")))
        .expect(1)
        .mount(&app.fdm)
        .await;

    let first = do_check_fdm_status(&app.state).await.unwrap();
    let second = do_check_fdm_status(&app.state).await.unwrap();
    assert_eq!(first["device"]["fdmId"], "FDM02000425");
    assert_eq!(first, second);
}

#[tokio::test]
async fn another_language_is_queried_again() {
    let app = TestApp::new().await;
    for (lang, fdm_id) in [("FR", "FDM-FR"), ("NL", "FDM-NL")] {
        Mock::given(method("POST"))
            .and(body_string_contains(format!("status(language: {lang})")))
            .respond_with(ResponseTemplate::new(200).set_body_json(status(fdm_id)))
            .expect(1)
            .mount(&app.fdm)
            .await;
    }

    let fr = do_check_fdm_status(&app.state).await.unwrap();
    app.state.config.lock().unwrap().language = "nl".into();
    let nl = do_check_fdm_status(&app.state).await.unwrap();
    assert_eq!(fr["device"]["fdmId"], "FDM-FR");
    assert_eq!(nl["device"]["fdmId"], "FDM-NL");
}
