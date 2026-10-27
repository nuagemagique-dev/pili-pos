//! POS description v2.2, Message: a warning the FDM attaches to a
//! mutation's response is shown at least once per bookingDate. Every
//! checked sign response hands its messages (minus showPos NEVER) to
//! the sink attached to the client with `with_sign_messages`, which the
//! engine forwards to the screen.
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

mod common;

use pos_fiscal::types::Mode;
use std::sync::{Arc, Mutex};

use pos_core::config::PosConfig;
use pos_fiscal::event::{Booking, build_work_in_out};
use pos_fiscal::fdm_client::{FdmClient, SignMessages};
use pos_fiscal::types::FdmWarningCode;
use serde_json::{Value, json};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer};

fn message(code: &str, show_pos: &str) -> Value {
    json!({
        "message": format!("{code} text"),
        "locations": null,
        "extensions": { "category": "SPF_FOD", "code": code, "data": null, "showPos": show_pos }
    })
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "test fixture builder, called with json! literals"
)]
fn work_in_answer(warnings: Value) -> Value {
    json!({ "data": { "signWorkIn": {
        "posId": "", "posFiscalTicketNo": 0, "posDateTime": "", "deviceId": "",
        "eventOperation": "WORK_IN",
        "fdmRef": {
            "fdmId": "FDM02000425",
            "fdmDateTime": "2026-09-27T14:40:00.000Z",
            "eventLabel": "S",
            "eventCounter": 1,
            "totalCounter": 1
        },
        "fdmSwVersion": "2.2.0.5",
        "digitalSignature": "dummy_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "shortSignature": null,
        "verificationUrl": null,
        "vatCalc": null,
        "bufferCapacityUsed": 0.09,
        "warnings": warnings,
        "informations": [],
        "footer": []
    } } })
}

#[tokio::test]
async fn sign_response_warnings_reach_the_sink_once_per_response() {
    let received: Arc<Mutex<Vec<SignMessages>>> = Arc::default();
    let server = MockServer::start().await;
    let config = PosConfig {
        pos_id: "CPOS0031234567".into(),
        terminal_id: "POS-1-BAR".into(),
        device_id: "F0:1D:3D:CC:13:C8".into(),
        fdm_url: format!("{}/graphql", server.uri()),
        ..Default::default()
    };
    let booking = Booking {
        period_id: "8f1c2c1e-0000-4000-8000-000000000001".into(),
        date: "2026-09-27".into(),
    };
    let sink = Arc::clone(&received);
    let fdm = FdmClient::from_config(&config)
        .with_sign_messages(Arc::new(move |m| sink.lock().unwrap().push(m.clone())));

    let warned = work_in_answer(json!([
        message("BUFFER_NEAR_FULL", "MANDATORY"),
        message("NOP_FAILED", "NEVER"),
    ]));
    Mock::given(method("POST"))
        .respond_with(common::EchoRequest(warned))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    fdm.sign_work_in(
        build_work_in_out(&config, &ident(), 1, "75061189702", &booking),
        Mode::Normal,
    )
    .await
    .unwrap();

    Mock::given(method("POST"))
        .respond_with(common::EchoRequest(work_in_answer(json!([]))))
        .mount(&server)
        .await;
    fdm.sign_work_in(
        build_work_in_out(&config, &ident(), 2, "75061189702", &booking),
        Mode::Normal,
    )
    .await
    .unwrap();

    let received = received.lock().unwrap();
    assert_eq!(
        received.len(),
        1,
        "a response without messages is not reported"
    );
    let got = &received[0];
    assert_eq!(got.mutation, "signWorkIn");
    assert_eq!(got.booking_date.as_deref(), Some("2026-09-27"));
    assert_eq!(got.warnings.len(), 1, "showPos NEVER is dropped");
    assert_eq!(
        got.warnings[0].extensions.code,
        FdmWarningCode::BufferNearFull
    );
    assert!(got.informations.is_empty());
}

/// The software identity of the Linux shell at version 1.0.0.
fn ident() -> pos_fiscal::identity::SoftwareIdentity {
    pos_fiscal::identity::SoftwareIdentity::new("pili-pos/1.0.0").unwrap()
}
