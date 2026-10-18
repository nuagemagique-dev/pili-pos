//! fdm_client error-handling contract tests.
//!
//! FDM communication is the fiscal-compliance-critical path: per
//! CLAUDE.md rule 1, "POS MUST block if FDM is disconnected - no
//! transactions without FDM." That means every failure mode on this
//! wire must surface as a clean `Err`, never a panic and never a
//! fake `Ok` built from a partial/malformed response. This file
//! exercises each failure mode with wiremock (and, for the network
//! failures, raw TCP listeners) and asserts `Err` every time.

mod common;

use std::time::Duration;

use pos_core::config::PosConfig;
use pos_fiscal::event::build_work_in_out;
use pos_fiscal::fdm_client::{FdmClient, FdmError};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A booking period for tests.
fn bk(period_id: &str) -> pos_fiscal::event::Booking {
    pos_fiscal::event::Booking {
        period_id: period_id.to_string(),
        date: "2026-07-03".to_string(),
    }
}

fn config(fdm_url: &str) -> PosConfig {
    PosConfig {
        table_count: 20,
        tables: None,
        payment_terminal: None,
        peppol: None,
        pos_id: "CFOD0061234567".to_string(),
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
        export: None,
    }
}

/// A well-formed `signWorkIn` response body, for the "not an error"
/// control cases.
fn sample_sign_result_json() -> serde_json::Value {
    serde_json::json!({
        "posId": "CFOD0061234567",
        "posFiscalTicketNo": 1,
        "posDateTime": "2026-09-25T10:00:00+02:00",
        "terminalId": "POS1",
        "deviceId": "DEV001",
        "eventOperation": "WORK_IN",
        "fdmRef": {
            "fdmId": "BMC0TEST0001",
            "fdmDateTime": "2026-09-25T08:00:00.000Z",
            "eventLabel": "S",
            "eventCounter": 1,
            "totalCounter": 1
        },
        "fdmSwVersion": "BMC-FDM-1.0",
        "digitalSignature": "c2lnbmF0dXJl",
        "shortSignature": null,
        "verificationUrl": null,
        "vatCalc": null,
        "bufferCapacityUsed": 1.0,
        "warnings": [],
        "informations": [],
        "footer": []
    })
}

async fn sign_work_in_call(
    client: &FdmClient,
    cfg: &PosConfig,
) -> Result<pos_fiscal::response::SignResult, FdmError> {
    let input = build_work_in_out(cfg, &ident(), 1, "93051800189", &bk("bp-1"));
    client.sign_work_in(input, false).await
}

// ── GraphQL-level errors ────────────────────────────────────────

#[tokio::test]
async fn graphql_errors_array_yields_err_not_panic() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("signWorkIn"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": null,
            "errors": [{ "message": "duplicate booking period" }]
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);

    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    match err {
        FdmError::Refused { message, .. } => assert!(
            message.contains("duplicate booking period"),
            "message was: {message}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// The spec's own error response (POS description v2.2): the code is
/// in `extensions`, `message` is free text. The code must reach the
/// caller, which maps it to the operator's message.
#[tokio::test]
async fn the_code_of_a_spec_shaped_error_is_kept() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "errors": [{
                "message": "FDM buffer is full.",
                "extensions": { "category": "SPF_FOD", "code": "BUFFER_FULL", "showPos": "MANDATORY" }
            }]
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let err = sign_work_in_call(&FdmClient::new(&cfg.fdm_url), &cfg)
        .await
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "GraphQL error: BUFFER_FULL: FDM buffer is full."
    );
}

/// A code the message already names is not repeated.
#[tokio::test]
async fn a_message_that_is_the_code_is_not_doubled() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "errors": [{ "message": "FDM_LOCKED", "extensions": { "code": "FDM_LOCKED" } }]
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let err = sign_work_in_call(&FdmClient::new(&cfg.fdm_url), &cfg)
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "GraphQL error: FDM_LOCKED");
}

#[tokio::test]
async fn empty_errors_array_with_data_is_not_treated_as_an_error() {
    // Some GraphQL servers send `"errors": []` alongside valid data.
    // That must NOT be mistaken for a failure.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(body_string_contains("signWorkIn"))
        .respond_with(common::EchoRequest(serde_json::json!({
            "data": { "signWorkIn": sample_sign_result_json() },
            "errors": []
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let result = sign_work_in_call(&client, &cfg).await;
    assert!(result.is_ok(), "empty errors array must not fail the call");
}

// ── Transport / HTTP-status errors ──────────────────────────────

#[tokio::test]
async fn http_500_with_non_json_body_yields_err_not_panic() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    // The body isn't JSON at all, so this must fail at the
    // `.json()` decode step, not silently succeed or panic.
    assert!(
        matches!(err, FdmError::Http(_)),
        "expected Http decode error, got {err:?}"
    );
}

#[tokio::test]
async fn http_500_with_valid_graphql_error_body_still_surfaces_the_graphql_error() {
    // The client does not gate on HTTP status before parsing the
    // body (see fdm_client.rs::execute_raw) - if the FDM (or a proxy
    // in front of it) returns 500 with a well-formed GraphQL error
    // envelope, that envelope's error must still be the one reported.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "data": null,
            "errors": [{ "message": "internal FDM failure" }]
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    match err {
        FdmError::Refused { message, .. } => assert!(message.contains("internal FDM failure")),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_json_response_yields_err_not_panic() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not valid json"))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    assert!(
        matches!(err, FdmError::Http(_)),
        "expected Http decode error, got {err:?}"
    );
}

#[tokio::test]
async fn status_query_malformed_json_yields_err_not_panic() {
    // Same failure mode, but on the query path (status()) rather
    // than a sign mutation - the two share `execute_raw` but this
    // pins the query path down explicitly.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>not json</html>"))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = client.status().await.unwrap_err();
    assert!(matches!(err, FdmError::Http(_)), "got {err:?}");
}

// ── Missing / incomplete fields ─────────────────────────────────

#[tokio::test]
async fn response_with_neither_data_nor_errors_yields_err() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    match err {
        FdmError::BadResponse(msg) => assert!(msg.contains("No data")),
        other => panic!("expected GraphQl(\"No data...\"), got {other:?}"),
    }
}

#[tokio::test]
async fn response_missing_the_mutation_field_yields_err() {
    // `data` is present but doesn't contain the `signWorkIn` key -
    // e.g. a server that answered a different operation.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "somethingElse": 1 }
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    match err {
        FdmError::BadResponse(msg) => assert!(msg.contains("Missing signWorkIn")),
        other => panic!("expected GraphQl(\"Missing signWorkIn...\"), got {other:?}"),
    }
}

#[tokio::test]
async fn response_with_malformed_sign_result_shape_yields_err() {
    // The mutation field is present, but its value doesn't deserialize
    // into `SignResult` (required fields missing/wrong types).
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "signWorkIn": { "unexpected": "shape" } }
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let err = sign_work_in_call(&client, &cfg).await.unwrap_err();
    match err {
        FdmError::BadResponse(msg) => assert!(msg.contains("Failed to parse signWorkIn")),
        other => panic!("expected GraphQl(\"Failed to parse signWorkIn...\"), got {other:?}"),
    }
}

#[tokio::test]
async fn status_response_missing_device_block_yields_err() {
    // FdmStatus.device is a required (non-Option) field - a response
    // that omits it must fail deserialization cleanly, not panic.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "fdmStatus": {
                    "initialized": true,
                    "informations": [],
                    "warnings": [],
                    "errors": []
                }
            }
        })))
        .mount(&server)
        .await;

    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    assert!(client.status().await.is_err());
}

// ── Network failures: refused connection / timeout ──────────────

#[tokio::test]
async fn refused_connection_yields_not_connected_quickly() {
    // Bind then immediately drop a listener: the OS will refuse new
    // connections on that port, giving us a genuine "nothing there"
    // target without depending on external network access.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let client = FdmClient::with_timeouts(
        &format!("http://{addr}/graphql"),
        Duration::from_millis(500),
        Duration::from_millis(800),
    );

    let start = std::time::Instant::now();
    let err = client.status().await.unwrap_err();
    let elapsed = start.elapsed();

    assert!(
        matches!(err, FdmError::NotConnected),
        "expected NotConnected, got {err:?}"
    );
    assert!(
        elapsed < Duration::from_secs(1),
        "refused connection should fail fast, took {elapsed:?}"
    );
}

#[tokio::test]
async fn hung_connection_times_out_quickly_and_yields_err() {
    // Accept the TCP connection but never write a response - this
    // forces the client to hit its own *total* request timeout
    // rather than a fast connection-refused error.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::task::spawn_blocking(move || {
        if let Ok((stream, _)) = listener.accept() {
            // Hold the connection open, unresponsive, well past the
            // client's total timeout below, then let it drop.
            std::thread::sleep(Duration::from_secs(2));
            drop(stream);
        }
    });

    let client = FdmClient::with_timeouts(
        &format!("http://{addr}/graphql"),
        Duration::from_millis(200),
        Duration::from_millis(400),
    );

    let start = std::time::Instant::now();
    let result = client.status().await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "a hung FDM must never look like success");
    assert!(
        elapsed < Duration::from_secs(2),
        "must respect the configured total timeout, took {elapsed:?}"
    );
}

// ── Responses that do not answer the request ────────────────────

/// Mount `result` (echoed or not) as the answer to `signWorkIn`.
async fn answer_work_in(result: serde_json::Value, echo: bool) -> Result<(), FdmError> {
    let server = MockServer::start().await;
    let body = serde_json::json!({ "data": { "signWorkIn": result } });
    let mock = Mock::given(method("POST")).and(path("/graphql"));
    if echo {
        mock.respond_with(common::EchoRequest(body))
            .mount(&server)
            .await;
    } else {
        mock.respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;
    }
    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    sign_work_in_call(&client, &cfg).await.map(|_| ())
}

fn mismatch_field(result: Result<(), FdmError>) -> String {
    match result {
        Err(FdmError::ResponseMismatch(msg)) => msg,
        other => panic!("expected ResponseMismatch, got {other:?}"),
    }
}

#[tokio::test]
async fn a_response_answering_another_request_is_refused() {
    // The canned body carries ticket 1 and a fixed date: never the
    // request's date, so the echo check trips.
    let msg = mismatch_field(answer_work_in(sample_sign_result_json(), false).await);
    assert!(msg.starts_with("posDateTime"), "{msg}");
}

#[tokio::test]
async fn a_response_for_another_pos_is_refused() {
    let server = MockServer::start().await;
    let body = serde_json::json!({ "data": { "signWorkIn": sample_sign_result_json() } });
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(move |req: &wiremock::Request| {
            let mut answer = common::answer(&body, &req.body);
            answer["data"]["signWorkIn"]["posId"] = "CPOS0031234567".into();
            ResponseTemplate::new(200).set_body_json(answer)
        })
        .mount(&server)
        .await;
    let cfg = config(&format!("{}/graphql", server.uri()));
    let client = FdmClient::new(&cfg.fdm_url);
    let msg = mismatch_field(sign_work_in_call(&client, &cfg).await.map(|_| ()));
    assert_eq!(msg, "posId attendu CFOD0061234567, reçu CPOS0031234567");
}

#[tokio::test]
async fn a_response_with_another_label_is_refused() {
    let mut result = sample_sign_result_json();
    result["fdmRef"]["eventLabel"] = "N".into();
    let msg = mismatch_field(answer_work_in(result, true).await);
    assert!(msg.starts_with("eventLabel attendu S"), "{msg}");
}

#[tokio::test]
async fn a_response_with_another_operation_is_refused() {
    let mut result = sample_sign_result_json();
    result["eventOperation"] = "WORK_OUT".into();
    let msg = mismatch_field(answer_work_in(result, true).await);
    assert!(msg.starts_with("eventOperation"), "{msg}");
}

#[tokio::test]
async fn a_response_without_signature_is_refused() {
    let mut result = sample_sign_result_json();
    result["digitalSignature"] = "".into();
    let msg = mismatch_field(answer_work_in(result, true).await);
    assert!(msg.starts_with("digitalSignature"), "{msg}");
}

#[tokio::test]
async fn an_echoed_response_is_accepted() {
    answer_work_in(sample_sign_result_json(), true)
        .await
        .expect("a consistent answer is accepted");
}

#[tokio::test]
async fn the_mismatch_reaches_the_ui_with_its_code() {
    let err = answer_work_in(sample_sign_result_json(), false)
        .await
        .unwrap_err();
    let ui_text = format!("FDM error: {err}");
    assert!(
        ui_text.starts_with("FDM error: FDM_RESPONSE_MISMATCH: "),
        "{ui_text}"
    );
}

/// The software identity of the Linux shell at version 1.0.0.
fn ident() -> pos_fiscal::identity::SoftwareIdentity {
    pos_fiscal::identity::SoftwareIdentity::new("pili-pos/1.0.0").unwrap()
}
