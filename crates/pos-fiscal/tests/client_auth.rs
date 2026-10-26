//! `FdmClient` bearer token: a cloud FDM (the module2 simulator) needs
//! `Authorization: Bearer <token>`; a LAN FDM gets no such header.
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_core::config::PosConfig;
use pos_fiscal::fdm_client::FdmClient;
use wiremock::matchers::{header, header_exists, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn status_body() -> serde_json::Value {
    serde_json::json!({ "data": { "fdmStatus": {
        "device": {
            "fdmId": "FDM02000425",
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

/// A token file with `content`, in a directory unique to the test.
fn token_file(test: &str, content: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("pos-fiscal-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("fdm.token");
    std::fs::write(&file, content).unwrap();
    file
}

fn config(server: &MockServer, token_file: Option<String>) -> PosConfig {
    PosConfig {
        fdm_url: format!("{}/graphql", server.uri()),
        fdm_token_file: token_file,
        ..Default::default()
    }
}

#[tokio::test]
async fn the_configured_token_is_sent_as_a_bearer_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header("authorization", "Bearer test-token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_body()))
        .expect(1)
        .mount(&server)
        .await;
    let file = token_file("bearer", "  test-token-123\n");

    let client = FdmClient::from_config(&config(&server, Some(file.display().to_string())));
    let status = client.status().await.expect("status with a token");
    assert_eq!(status.device.fdm_id, "FDM02000425");
}

#[tokio::test]
async fn a_lan_fdm_gets_no_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(401))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_body()))
        .mount(&server)
        .await;

    FdmClient::from_config(&config(&server, None))
        .status()
        .await
        .expect("status without a token");
}

#[tokio::test]
async fn an_unreadable_token_file_sends_no_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(status_body()))
        .mount(&server)
        .await;

    FdmClient::from_config(&config(&server, Some("/nonexistent/fdm.token".into())))
        .status()
        .await
        .unwrap();
    let requests: Vec<Request> = server.received_requests().await.unwrap();
    assert!(
        requests
            .iter()
            .all(|r| !r.headers.contains_key("authorization"))
    );
}

#[test]
fn check_config_reports_a_missing_or_empty_token_file() {
    let missing = PosConfig {
        fdm_token_file: Some("/nonexistent/fdm.token".into()),
        ..Default::default()
    };
    assert!(
        missing
            .problems()
            .iter()
            .any(|p| p.text().starts_with("fdm_token_file: cannot read"))
    );
    let file = token_file("empty", " \n");
    let empty = PosConfig {
        fdm_token_file: Some(file.display().to_string()),
        ..Default::default()
    };
    assert!(
        empty
            .problems()
            .iter()
            .any(|p| p.text().ends_with("is empty"))
    );
}
