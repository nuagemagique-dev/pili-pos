//! Sign requests are kept byte for byte, so one whose answer was lost
//! is sent again identically (POS description v2.2, duplicate requests).

use std::time::Duration;

use pos_core::config::PosConfig;
use pos_fiscal::event::build_work_in_out;
use pos_fiscal::fdm_client::{FdmClient, FdmError, SignMutation, SignOutcome, SignRequest};
use wiremock::matchers::{body_string, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn config(fdm_url: &str) -> PosConfig {
    PosConfig {
        table_count: 20,
        pos_id: "CPOS0031234567".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "DEV001".to_string(),
        restaurant_name: "Tokyo Sushi Bar".to_string(),
        restaurant_address: "Rue de Flandre 1, 1000 Bruxelles".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456791".to_string(),
        fdm_url: fdm_url.to_string(),
        printer_address: None,
        training_mode: false,
        hubrise_enabled: false,
        hubrise: None,
        language: "fr".to_string(),
        backup_dir: None,
        kitchen_printer_address: None,
        fdm_token_file: None,
        fdm_ca_file: None,
        fdm_client_cert_file: None,
        fdm_client_key_file: None,
        restaurant_legal_name: None,
        day_end_hour: 5,
        digital_ticket: None,
    }
}

fn work_in() -> pos_fiscal::inputs::WorkInOutInput {
    let booking = pos_fiscal::event::Booking {
        period_id: "bp-1".into(),
        date: "2026-07-03".into(),
    };
    build_work_in_out(&config("http://unused"), 1, "93051800189", &booking)
}

/// The body as every sign call built it before requests were stored.
fn body_as_before(input: &pos_fiscal::inputs::WorkInOutInput) -> String {
    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Vars<'a> {
        data: &'a pos_fiscal::inputs::WorkInOutInput,
        is_training: bool,
    }
    #[derive(serde::Serialize)]
    struct Body<'a> {
        query: String,
        variables: Vars<'a>,
    }
    let sel = "{ posId posFiscalTicketNo posDateTime terminalId deviceId eventOperation fdmRef { fdmId fdmDateTime eventLabel eventCounter totalCounter } fdmSwVersion digitalSignature shortSignature verificationUrl vatCalc { label rate taxableAmount vatAmount totalAmount outOfScope } bufferCapacityUsed warnings { message extensions { category code showPos } } informations { message extensions { category code showPos } } footer }";
    pos_fiscal::canonical::to_canonical(&Body {
        query: format!(
            "mutation($data: WorkInOutInput!, $isTraining: Boolean!) {{ signWorkIn(data: $data, isTraining: $isTraining) {sel} }}"
        ),
        variables: Vars {
            data: input,
            is_training: false,
        },
    })
    .unwrap()
}

#[test]
fn the_request_bytes_are_unchanged() {
    let input = work_in();
    let request = SignRequest::new(SignMutation::WorkIn, &input, false).unwrap();
    assert_eq!(request.body(), body_as_before(&input));
}

#[test]
fn a_stored_request_comes_back_identical() {
    let request = SignRequest::new(SignMutation::WorkIn, &work_in(), true).unwrap();
    let stored = SignRequest::from_stored("signWorkIn", true, request.body().to_string()).unwrap();
    assert_eq!(stored, request);

    let body = request.body().to_string();
    assert!(SignRequest::from_stored("signWorkOut", true, body.clone()).is_err());
    assert!(SignRequest::from_stored("signWorkIn", false, body.clone()).is_err());
    assert!(SignRequest::from_stored("signNothing", true, body).is_err());
    assert!(SignRequest::from_stored("signWorkIn", true, "{}".into()).is_err());
}

#[test]
fn mutation_names_round_trip() {
    for mutation in SignMutation::ALL {
        assert_eq!(SignMutation::from_name(mutation.name()), Some(mutation));
    }
}

#[tokio::test]
async fn a_stored_request_is_sent_as_is() {
    let server = MockServer::start().await;
    let input = work_in();
    let request = SignRequest::new(SignMutation::WorkIn, &input, false).unwrap();
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string(request.body()))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "data": {
                "signWorkIn": {
                    "posId": "CPOS0031234567",
                    "posFiscalTicketNo": 1,
                    "posDateTime": serde_json::to_value(&input).unwrap()["posDateTime"],
                    "terminalId": "POS1",
                    "deviceId": "DEV001",
                    "eventOperation": "WORK_IN",
                    "fdmRef": {
                        "fdmId": "MCK00000001",
                        "fdmDateTime": "2026-07-03T10:00:00Z",
                        "eventLabel": "S",
                        "eventCounter": 1,
                        "totalCounter": 1
                    },
                    "fdmSwVersion": "MOCK",
                    "digitalSignature": "c2ln",
                    "shortSignature": null,
                    "verificationUrl": null,
                    "vatCalc": null,
                    "bufferCapacityUsed": 1.0,
                    "warnings": [],
                    "informations": [],
                    "footer": []
                }
            }})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let client = FdmClient::new(&format!("{}/graphql", server.uri()));
    let stored = SignRequest::from_stored("signWorkIn", false, request.body().to_string()).unwrap();
    let results = client.send(&stored).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].fdm_ref.event_counter, 1);
}

#[tokio::test]
async fn the_z_pair_is_answered_twice_identically() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum_serve(listener).await });
    let client = FdmClient::new(&format!("http://{addr}/graphql"));

    let vars: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                "../../docs/20260821_UC_Reports_1.3/M184_signReportTurnoverZUserZ/M184_signReportTurnoverZUserZ_GQL-Variables.json",
            ),
        )
        .unwrap(),
    )
    .unwrap();
    let mut turnover = vars["turnover"].clone();
    let mut user = vars["user"].clone();
    // The mock checks posDateTime against the Belgian offset.
    for input in [&mut turnover, &mut user] {
        input["posDateTime"] = serde_json::json!("2026-07-03T23:00:00+02:00");
    }
    let request = SignRequest::turnover_z_user_z(&turnover, &user, false).unwrap();
    let first = client.send(&request).await.unwrap();
    let again = client.send(&request).await.unwrap();
    assert_eq!(first.len(), 2);
    for (a, b) in first.iter().zip(&again) {
        assert_eq!(a.fdm_ref.total_counter, b.fdm_ref.total_counter);
        assert_eq!(a.digital_signature, b.digital_signature);
    }
}

async fn axum_serve(listener: tokio::net::TcpListener) {
    axum::serve(listener, pos_fdm_mock::app()).await.unwrap();
}

#[tokio::test]
async fn failures_say_whether_the_event_may_be_signed() {
    let refused = |codes: &[&str]| FdmError::Refused {
        codes: codes.iter().map(|c| c.to_string()).collect(),
        message: String::new(),
    };
    assert_eq!(FdmError::NotConnected.outcome(), SignOutcome::NotSent);
    assert_eq!(
        FdmError::CertificateRefused("ca".into()).outcome(),
        SignOutcome::NotSent
    );
    assert_eq!(
        FdmError::Serialization("x".into()).outcome(),
        SignOutcome::NotSent
    );
    assert_eq!(refused(&["BUFFER_FULL"]).outcome(), SignOutcome::Refused);
    assert_eq!(
        refused(&["FDM_LOCKED", "INVALID_REQUEST"]).outcome(),
        SignOutcome::Refused
    );
    assert_eq!(
        refused(&["BUFFER_FULL", "INTERNAL_ERROR"]).outcome(),
        SignOutcome::Unknown
    );
    assert_eq!(refused(&[]).outcome(), SignOutcome::Unknown);
    assert_eq!(
        FdmError::BadResponse("no data".into()).outcome(),
        SignOutcome::Unknown
    );
    assert_eq!(
        FdmError::ResponseMismatch("posId".into()).outcome(),
        SignOutcome::Unknown
    );

    // An FDM that takes the request and never answers.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    });
    let client = FdmClient::with_timeouts(
        &format!("http://{addr}/graphql"),
        Duration::from_millis(500),
        Duration::from_millis(300),
    );
    let err = client.sign_work_in(work_in(), false).await.unwrap_err();
    assert_eq!(err.outcome(), SignOutcome::Unknown, "{err:?}");
}
