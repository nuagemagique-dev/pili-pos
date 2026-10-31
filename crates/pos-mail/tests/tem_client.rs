//! `TemClient` contract tests against a mocked Scaleway TEM endpoint.
//!
//! ⚠️ These mocks encode this crate's *assumed* shape for the TEM API
//! (see the module doc in `src/tem.rs`) - a green suite proves
//! `TemClient` behaves correctly against that assumed shape, not that
//! the assumption matches Scaleway's real API. Verify against
//! <https://www.scaleway.com/en/developers/api/transactional-email>/
//! before pointing this at production.

use std::time::Duration;

use pos_mail::render::{Attachment, Email};
use pos_mail::tem::{TemClient, TemError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SECRET: &str = "sk-super-secret-value-12345";

fn sample_email() -> Email {
    Email {
        to: "customer@example.be".to_string(),
        subject: "Votre ticket - Restaurant Exemple".to_string(),
        text: "TICKET DE CAISSE T.V.A.\n...\nhttps://www.gs2.be/abc".to_string(),
        html: "<html><body>...</body></html>".to_string(),
        attachments: vec![Attachment {
            name: "ticket-qr.png".to_string(),
            content_type: "image/png".to_string(),
            content: vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        }],
    }
}

fn client_for(server: &MockServer, timeout: Duration) -> TemClient {
    TemClient::new(
        reqwest::Client::new(),
        server.uri(),
        "fr-par",
        "11111111-1111-1111-1111-111111111111",
        SECRET,
        "no-reply@restaurant-exemple.be",
        "Restaurant Exemple",
        timeout,
    )
}

// ── success ─────────────────────────────────────────────────────

#[tokio::test]
async fn send_success_captures_the_message_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/transactional-email/v1alpha1/regions/fr-par/emails"))
        .and(header("X-Auth-Token", SECRET))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "id": "email-1", "message_id": "msg-abc-123" }]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let message_id = client.send(&sample_email()).await.expect("send succeeds");
    assert_eq!(message_id, "msg-abc-123");
}

#[tokio::test]
async fn send_falls_back_to_id_when_message_id_is_absent() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "id": "email-only-id" }]
        })))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let message_id = client.send(&sample_email()).await.expect("send succeeds");
    assert_eq!(message_id, "email-only-id");
}

#[tokio::test]
async fn send_sets_the_auth_header_and_request_body_fields() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    client.send(&sample_email()).await.unwrap();

    let requests = server
        .received_requests()
        .await
        .expect("request recording is enabled by default");
    assert_eq!(requests.len(), 1);
    let req = &requests[0];

    assert_eq!(req.headers.get("x-auth-token").unwrap(), SECRET);

    let body: serde_json::Value = req.body_json().unwrap();
    assert_eq!(body["to"][0]["email"], "customer@example.be");
    assert_eq!(body["subject"], "Votre ticket - Restaurant Exemple");
    assert_eq!(body["from"]["email"], "no-reply@restaurant-exemple.be");
    assert_eq!(body["from"]["name"], "Restaurant Exemple");
    assert_eq!(body["project_id"], "11111111-1111-1111-1111-111111111111");
    assert_eq!(body["attachments"][0]["name"], "ticket-qr.png");
    assert_eq!(body["attachments"][0]["type"], "image/png");
    // Attachment content is base64, not the raw magic bytes.
    assert_ne!(body["attachments"][0]["content"], serde_json::json!([]));

    // Only fields of Scaleway's CreateEmailRequest (scaleway-sdk-go,
    // api/tem/v1alpha1), so the API never sees an unknown key.
    let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "attachments",
            "from",
            "html",
            "project_id",
            "subject",
            "text",
            "to"
        ]
    );
    let content = body["attachments"][0]["content"].as_str().unwrap();
    let png = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, content)
        .expect("attachment content is standard base64");
    assert_eq!(&png[..4], b"\x89PNG");
}

#[tokio::test]
async fn the_secret_never_appears_in_the_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    client.send(&sample_email()).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let body_str = String::from_utf8_lossy(&requests[0].body);
    assert!(
        !body_str.contains(SECRET),
        "the secret must only ever travel in the X-Auth-Token header"
    );
}

// ── error mapping ───────────────────────────────────────────────

#[tokio::test]
async fn http_400_is_a_permanent_client_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("invalid recipient"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::ClientError { status: 400, .. }));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn http_401_is_a_permanent_client_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::ClientError { status: 401, .. }));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn http_429_is_retryable_rate_limiting() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(429).set_body_string("slow down"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::RateLimited { .. }));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn http_500_is_a_retryable_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::ServerError { status: 500, .. }));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn http_503_is_a_retryable_server_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::ServerError { status: 503, .. }));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn a_slow_response_past_the_client_timeout_is_a_retryable_timeout() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(300))
                .set_body_json(serde_json::json!({ "emails": [{ "message_id": "late" }] })),
        )
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_millis(50));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::Timeout));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn malformed_json_response_is_an_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json at all {"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::MalformedResponse(_)));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn empty_emails_array_is_a_malformed_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "emails": [] })))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::MalformedResponse(_)));
}

#[tokio::test]
async fn missing_message_id_and_id_is_a_malformed_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "status": "queued" }]
        })))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.send(&sample_email()).await.unwrap_err();
    assert!(matches!(err, TemError::MalformedResponse(_)));
}

// ── probe() ─────────────────────────────────────────────────────

#[tokio::test]
async fn probe_succeeds_on_2xx() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("X-Auth-Token", SECRET))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "emails": [] })))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    client.probe().await.expect("probe should succeed");
}

#[tokio::test]
async fn probe_surfaces_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_string("unauthorized"))
        .mount(&server)
        .await;

    let client = client_for(&server, Duration::from_secs(5));
    let err = client.probe().await.unwrap_err();
    assert!(matches!(err, TemError::ClientError { status: 401, .. }));
}
