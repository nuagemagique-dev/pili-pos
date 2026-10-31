#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

//! End-to-end outbox tests: `send_now` / `run_once` driving real
//! SQLite (`pos-db`) rows against a mocked TEM endpoint (`wiremock`).

use std::sync::Mutex;
use std::time::Duration;

use pos_db::queries::deliveries;
use pos_mail::outbox::{self, DeliveryPolicy, SendNowOutcome};
use pos_mail::tem::TemClient;
use rusqlite::Connection;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_db() -> Mutex<Connection> {
    let conn = Connection::open_in_memory().unwrap();
    pos_db::schema::run_migrations(&conn).unwrap();
    Mutex::new(conn)
}

fn fiscal_event(db: &Mutex<Connection>) -> i64 {
    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
         VALUES ('N', 1, 'P', 'T', '00000000029', '{}')",
        [],
    )
    .unwrap();
    conn.last_insert_rowid()
}

fn enqueue(db: &Mutex<Connection>, fiscal_event_id: i64) -> i64 {
    let conn = db.lock().unwrap();
    deliveries::enqueue(
        &conn,
        deliveries::NewTicketDelivery {
            fiscal_event_id,
            medium: "DIGITAL",
            recipient: "customer@example.be",
            recipient_hint: "c***@e***.be",
            ticket_text: "TICKET DE CAISSE T.V.A.\nSaumon roll x8",
            verification_url: Some("https://www.gs2.be/abc123"),
        },
    )
    .unwrap()
}

fn client_for(server: &MockServer) -> TemClient {
    TemClient::new(
        reqwest::Client::new(),
        server.uri(),
        "fr-par",
        "project-1",
        "secret",
        "from@example.com",
        "Restaurant Exemple",
        Duration::from_secs(5),
    )
}

fn status_of(db: &Mutex<Connection>, id: i64) -> String {
    let conn = db.lock().unwrap();
    deliveries::get(&conn, id).unwrap().unwrap().status
}

// ── send_now ────────────────────────────────────────────────────

#[tokio::test]
async fn send_now_succeeds_and_nulls_the_recipient() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    let outcome = outbox::send_now(
        &db,
        &client,
        "Restaurant Exemple",
        id,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(
        outcome,
        SendNowOutcome::Sent {
            message_id: "msg-1".to_string()
        }
    );

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert_eq!(row.status, "sent");
    assert!(row.recipient.is_none());
    assert_eq!(row.provider_message_id.as_deref(), Some("msg-1"));
}

#[tokio::test]
async fn send_now_on_failure_leaves_the_row_queued_for_retry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503).set_body_string("down"))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    let outcome = outbox::send_now(
        &db,
        &client,
        "Restaurant Exemple",
        id,
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(outcome, SendNowOutcome::Queued);

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert_eq!(row.status, "pending");
    assert_eq!(row.attempts, 1);
    assert!(row.next_attempt_at.is_some());
    // Still has its recipient - nothing was purged or lost.
    assert!(row.recipient.is_some());
}

#[tokio::test]
async fn send_now_on_permanent_error_gives_up_immediately() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("invalid recipient"))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    outbox::send_now(
        &db,
        &client,
        "Restaurant Exemple",
        id,
        Duration::from_secs(5),
    )
    .await;

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert_eq!(row.status, "failed", "a 400 must never be retried");
    assert_eq!(row.attempts, 1);
    assert!(row.next_attempt_at.is_none());
}

#[tokio::test]
async fn send_now_past_its_timeout_requeues_the_row_promptly() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_millis(500))
                .set_body_json(serde_json::json!({ "emails": [{ "message_id": "late" }] })),
        )
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    // Client-level timeout longer than the inline budget below, so the
    // *inline* budget (not the HTTP client) is what fires first.
    let client = TemClient::new(
        reqwest::Client::new(),
        server.uri(),
        "fr-par",
        "project-1",
        "secret",
        "from@example.com",
        "Restaurant Exemple",
        Duration::from_secs(10),
    );

    let outcome = outbox::send_now(
        &db,
        &client,
        "Restaurant Exemple",
        id,
        Duration::from_millis(50),
    )
    .await;
    assert_eq!(outcome, SendNowOutcome::Queued);

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    // Requeued promptly rather than left stuck in `sending`.
    assert_eq!(row.status, "pending");
    assert_eq!(row.attempts, 1);
    assert!(row.last_error.as_deref().unwrap().contains("timed out"));
}

#[tokio::test]
async fn send_now_on_an_unknown_row_is_queued_without_panicking() {
    let server = MockServer::start().await;
    let db = test_db();
    let client = client_for(&server);
    let outcome = outbox::send_now(
        &db,
        &client,
        "Restaurant Exemple",
        999,
        Duration::from_secs(1),
    )
    .await;
    assert_eq!(outcome, SendNowOutcome::Queued);
}

// ── run_once: backoff schedule + giving up ─────────────────────

#[tokio::test]
async fn run_once_retries_with_the_first_backoff_step_after_a_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503).set_body_string("down"))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    let report = outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();
    assert_eq!(report.retried, 1);
    assert_eq!(report.sent, 0);
    assert_eq!(report.given_up, 0);

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert_eq!(row.status, "pending");
    assert_eq!(row.attempts, 1);
    let next = row.next_attempt_at.unwrap();
    let scheduled: chrono::DateTime<chrono::Utc> = next.parse().unwrap();
    let expected_min = chrono::Utc::now() + chrono::Duration::seconds(55); // ~1 min, some slack
    let expected_max = chrono::Utc::now() + chrono::Duration::seconds(65);
    assert!(
        scheduled > expected_min && scheduled < expected_max,
        "expected the ~60s backoff step, got next_attempt_at={next}"
    );
}

#[tokio::test]
async fn run_once_does_not_retry_a_row_whose_backoff_has_not_elapsed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        .expect(0) // must never be called: the row isn't due yet
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    {
        let conn = db.lock().unwrap();
        deliveries::mark_failed(&conn, id, "boom", Some("2099-01-01T00:00:00.000Z")).unwrap();
    }

    let client = client_for(&server);
    let report = outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();
    assert_eq!(report.sent, 0);
    assert_eq!(report.retried, 0);
    assert_eq!(status_of(&db, id), "pending");
}

#[tokio::test]
async fn run_once_gives_up_after_the_backoff_table_is_exhausted() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503).set_body_string("down"))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    // Drive the row to just short of MAX_ATTEMPTS by failing it
    // directly (cheaper than actually waiting out the backoff between
    // real `run_once` calls) and confirm the final `run_once` call -
    // the one that pushes attempts to MAX_ATTEMPTS - gives up.
    {
        let conn = db.lock().unwrap();
        for _ in 0..(DeliveryPolicy::MAX_ATTEMPTS - 1) {
            deliveries::mark_failed(&conn, id, "boom", Some("2000-01-01T00:00:00.000Z")).unwrap();
        }
    }
    assert_eq!(
        status_of(&db, id),
        "pending",
        "still retryable before the last attempt"
    );

    let report = outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();
    assert_eq!(report.given_up, 1);
    assert_eq!(report.retried, 0);

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert_eq!(row.status, "failed");
    assert_eq!(row.attempts, DeliveryPolicy::MAX_ATTEMPTS);
    assert!(row.next_attempt_at.is_none());
}

// ── purge: after send, and after 72h ───────────────────────────

#[tokio::test]
async fn run_once_purges_recipients_older_than_72h_regardless_of_status() {
    let server = MockServer::start().await;
    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    {
        let conn = db.lock().unwrap();
        // Give up (status = failed) then backdate it past the GDPR
        // window.
        deliveries::mark_failed(&conn, id, "boom", None).unwrap();
        conn.execute(
            "UPDATE ticket_deliveries SET created_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
            [id],
        )
        .unwrap();
    }

    let client = client_for(&server);
    let report = outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();
    assert_eq!(report.purged_recipients, 1);

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert!(row.recipient.is_none());
    assert_eq!(row.status, "purged");
}

#[tokio::test]
async fn recipient_is_already_gone_immediately_after_a_successful_send() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();

    let conn = db.lock().unwrap();
    let row = deliveries::get(&conn, id).unwrap().unwrap();
    assert_eq!(row.status, "sent");
    assert!(
        row.recipient.is_none(),
        "GDPR: a successful send nulls the recipient right away, not after 72h"
    );
}

// ── fallback only once (via pos-db, exercised alongside the outbox) ──

#[tokio::test]
async fn fallback_can_only_be_recorded_once_even_if_the_send_also_later_fails() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    let client = client_for(&server);

    // The integrator prints a paper fallback and records it...
    {
        let conn = db.lock().unwrap();
        deliveries::mark_fallback_printed(&conn, id).unwrap();
    }
    // ...independently, the digital attempt (already in flight, or
    // retried later) still runs and gives up permanently.
    outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();

    // A second fallback print for the same delivery must still be
    // refused, no matter what happened to the digital side meanwhile.
    let conn = db.lock().unwrap();
    assert!(deliveries::mark_fallback_printed(&conn, id).is_err());
}

// ── concurrent run_once never double-sends ─────────────────────

#[tokio::test]
async fn two_concurrent_run_once_calls_send_the_row_exactly_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        // If claiming weren't atomic, both `run_once` calls would try
        // to send this row and this expectation (exactly one POST)
        // would fail the test.
        .expect(1)
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let _id = enqueue(&db, fe);
    let client = client_for(&server);

    // A single `Mutex<Connection>` behind two logical workers, exactly
    // like two independently-scheduled `run_once` ticks racing in the
    // same process - the DB layer's atomic `claim` is what prevents
    // the double-send, not any ordering `.await` gives us here.
    let (a, b) = tokio::join!(
        outbox::run_once(&db, &client, "Restaurant Exemple"),
        outbox::run_once(&db, &client, "Restaurant Exemple"),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.sent + b.sent, 1);

    server.verify().await;
}

// ── stale `sending` recovery ────────────────────────────────────

#[tokio::test]
async fn run_once_recovers_a_stuck_sending_row_and_then_retries_it() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "emails": [{ "message_id": "msg-1" }]
        })))
        .mount(&server)
        .await;

    let db = test_db();
    let fe = fiscal_event(&db);
    let id = enqueue(&db, fe);
    {
        let conn = db.lock().unwrap();
        assert!(deliveries::claim(&conn, id).unwrap());
        // Simulate a crash mid-send: `updated_at` far in the past.
        conn.execute(
            "UPDATE ticket_deliveries SET updated_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
            [id],
        )
        .unwrap();
    }

    let client = client_for(&server);
    let report = outbox::run_once(&db, &client, "Restaurant Exemple")
        .await
        .unwrap();
    assert_eq!(report.recovered_stale, 1);
    assert_eq!(report.sent, 1);
    assert_eq!(status_of(&db, id), "sent");
}
