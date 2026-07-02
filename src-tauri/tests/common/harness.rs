//! In-process Tauri-app simulator for SPF e2e replay.
//!
//! `TestApp::new()` builds an `AppState` with:
//! - An in-memory SQLite (`Connection::open_in_memory()`) seeded with
//!   the production schema via `pos_db::schema::run_migrations`.
//! - A fresh `PosConfig` whose `fdm_url` points at a `wiremock::MockServer`
//!   started for this test.
//! - A booking-period UUID + cancellation token like the real `AppState`.
//!
//! Tests then call `mock_response(&app, "signSale", &fixture)` to wire a
//! canonical SPF response for a single mutation, run the command's
//! inner function (`commands::checkout::do_checkout`, etc.), and assert
//! against `app.fiscal_events()` and `app.counter_for(EventLabel::N)`.
//!
//! The harness is deliberately thin — it doesn't try to model the full
//! Tauri runtime (windows, plugins, IPC), only what the command bodies
//! need: an `AppState` value and a reachable FDM URL.

use std::sync::Mutex;

#[cfg(feature = "hubrise")]
use std::sync::Arc;

use pos_core::config::PosConfig;
use pos_fiscal::types::EventLabel;
use rusqlite::Connection;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use moyun_pos::state::AppState;

use super::fixtures::{ResponseKind, UcFixture, read_response_envelope};

/// One row from the `fiscal_events` table, decoded enough to write
/// expressive assertions.
#[derive(Debug, Clone)]
pub struct FiscalEventRow {
    pub id: i64,
    pub event_type: String,
    pub sequence_number: i64,
    pub order_id: Option<i64>,
    pub user_insz: String,
    pub fdm_id: Option<String>,
    pub event_label: Option<String>,
    pub event_counter: Option<i64>,
    pub total_counter: Option<i64>,
    pub short_signature: Option<String>,
    pub event_data_json: Option<String>,
    pub fdm_response_json: Option<String>,
}

/// In-process test app: an `AppState` + a wiremock server pretending
/// to be the FDM. Tests drop both at end-of-scope.
pub struct TestApp {
    pub state: AppState,
    pub fdm: MockServer,
}

impl TestApp {
    /// Build a fresh `TestApp`. The SQLite DB is in-memory and the
    /// PosConfig points at the wiremock URL. Pos identification
    /// matches the SPF test setup (`UC_TestSetup_20240806.md` uses
    /// `pos_id=CFOD0061234567`).
    pub async fn new() -> Self {
        let fdm = MockServer::start().await;
        Self::with_fdm(fdm).await
    }

    /// Construct a TestApp around a caller-supplied wiremock server.
    /// Useful when a test needs to register expectations before the
    /// state is built.
    pub async fn with_fdm(fdm: MockServer) -> Self {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        pos_db::schema::run_migrations(&conn).expect("migrate");

        let config = PosConfig {
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            device_id: "DEV001".to_string(),
            // Identity from the official SPF test setup
            // (UC_TestSetup_20240806: "THE FPS FINANCES EXPERIENCE",
            // VAT BE0000000097, establishment 2000000042) so exported
            // §IV evidence carries conforming identification fields.
            restaurant_name: "The FPS Finances Experience".to_string(),
            restaurant_address: "Boulevard Albert II 33, 1030 Brussels".to_string(),
            restaurant_vat: "BE0000000097".to_string(),
            restaurant_est_no: "2000000042".to_string(),
            fdm_url: format!("{}/graphql", fdm.uri()),
            printer_address: None,
            training_mode: false,
            language: "fr".to_string(),
            backup_dir: None,
        };

        let state = AppState {
            db: Mutex::new(conn),
            #[cfg(feature = "hubrise")]
            poller_db: Arc::new(Mutex::new(
                Connection::open_in_memory().expect("poller sqlite"),
            )),
            config: Mutex::new(config),
            booking_period_id: Mutex::new(uuid::Uuid::new_v4().to_string()),
            #[cfg(feature = "hubrise")]
            shared_http: reqwest::Client::new(),
            shutdown_token: CancellationToken::new(),
        };

        Self { state, fdm }
    }

    /// Snapshot the entire `fiscal_events` table, oldest first.
    /// `event_counter` / `total_counter` are extracted from the
    /// archived `fdm_response_json` (the FDM owns those numbers; the
    /// POS only stores the response blob), mirroring what
    /// `pos_db::queries::fiscal::get_fiscal_event` does at runtime.
    pub fn fiscal_events(&self) -> Vec<FiscalEventRow> {
        let conn = self.state.db.lock().expect("db lock");
        let mut stmt = conn
            .prepare(
                "SELECT id, event_type, sequence_number, order_id, user_insz,
                        fdm_id, event_label,
                        short_signature, event_data_json, fdm_response_json
                 FROM fiscal_events
                 ORDER BY id ASC",
            )
            .expect("prepare");
        stmt.query_map([], |row| {
            let fdm_response_json: Option<String> = row.get(9)?;
            let (event_counter, total_counter) = match fdm_response_json
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| v.get("fdmRef").cloned())
            {
                Some(fdm_ref) => (
                    fdm_ref.get("eventCounter").and_then(|v| v.as_i64()),
                    fdm_ref.get("totalCounter").and_then(|v| v.as_i64()),
                ),
                None => (None, None),
            };
            Ok(FiscalEventRow {
                id: row.get(0)?,
                event_type: row.get(1)?,
                sequence_number: row.get(2)?,
                order_id: row.get(3)?,
                user_insz: row.get(4)?,
                fdm_id: row.get(5)?,
                event_label: row.get(6)?,
                event_counter,
                total_counter,
                short_signature: row.get(7)?,
                event_data_json: row.get(8)?,
                fdm_response_json,
            })
        })
        .expect("query_map")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect")
    }

    /// Current counter value for an event label (without incrementing).
    pub fn counter_for(&self, label: EventLabel) -> i64 {
        let conn = self.state.db.lock().expect("db lock");
        pos_fiscal::counter::FiscalCounter::current(&conn, label).expect("counter")
    }

    /// Order status from the `orders` table.
    pub fn order_status(&self, order_id: i64) -> Option<String> {
        let conn = self.state.db.lock().expect("db lock");
        conn.query_row(
            "SELECT status FROM orders WHERE id = ?1",
            rusqlite::params![order_id],
            |row| row.get(0),
        )
        .ok()
    }

    /// §IV evidence export. When `UC_EVIDENCE_DIR` is set, dump every
    /// archived fiscal event of this test as pretty-printed JSON:
    ///
    /// - `<UC>_event<N>_<label>_request.json` — the event payload our
    ///   command layer actually built and sent to the FDM
    ///   (`fiscal_events.event_data_json`).
    /// - `<UC>_event<N>_<label>_response.json` — the (fixture) FDM
    ///   response that was archived alongside it.
    ///
    /// No-op when the env var is unset, so normal `cargo test` runs
    /// are unaffected. Run `scripts/capture_uc_evidence.sh` to
    /// produce the full evidence set.
    pub fn dump_evidence(&self, uc_id: &str) {
        let Ok(dir) = std::env::var("UC_EVIDENCE_DIR") else {
            return;
        };
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).expect("create evidence dir");

        let pretty = |raw: &str| -> String {
            serde_json::from_str::<serde_json::Value>(raw)
                .and_then(|v| serde_json::to_string_pretty(&v))
                .unwrap_or_else(|_| raw.to_string())
        };

        for (i, ev) in self.fiscal_events().iter().enumerate() {
            let label = ev.event_type.as_str();
            if let Some(ref req) = ev.event_data_json {
                let p = dir.join(format!("{uc_id}_event{n}_{label}_request.json", n = i + 1));
                std::fs::write(p, pretty(req)).expect("write request evidence");
            }
            if let Some(ref resp) = ev.fdm_response_json {
                let p = dir.join(format!("{uc_id}_event{n}_{label}_response.json", n = i + 1));
                std::fs::write(p, pretty(resp)).expect("write response evidence");
            }
        }
    }
}

/// Register a single wiremock mock that responds to ANY GraphQL POST
/// with the supplied envelope. Matches by HTTP method + path only — the
/// test relies on each command issuing exactly the mutation we're
/// servicing. For multi-step UCs, register multiple mocks; wiremock
/// dispatches in registration order via `up_to_n_times(1)`.
pub async fn mock_response(app: &TestApp, response_envelope: Value) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_envelope))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
}

/// Convenience helper: load the fixture's response file and register
/// it as a one-shot mock. Works for any UC where the published
/// `*_Response.json` is the full envelope to replay (`{"data": {…}}`).
pub async fn mock_from_fixture(app: &TestApp, fixture: &UcFixture) {
    let envelope = read_response_envelope(fixture);
    mock_response(app, envelope).await;
}

/// For UC sets where the on-disk response file is the GraphQL inner
/// payload only (no `{"data": …}` wrapper), wrap it. Most SPF
/// fixtures already include the wrapper — this is a fallback.
#[allow(dead_code)]
pub async fn mock_inner_payload(app: &TestApp, mutation_name: &str, inner: Value) {
    let body = serde_json::json!({ "data": { mutation_name: inner } });
    mock_response(app, body).await;
}

/// Sanity: assert the FDM was actually called.
pub async fn assert_fdm_was_called(app: &TestApp) {
    let received = app.fdm.received_requests().await.unwrap_or_default();
    assert!(
        !received.is_empty(),
        "expected at least one POST to /graphql, got 0"
    );
}

/// Re-export for downstream tests that want to know which kind of
/// response a fixture carries (sign vs status).
#[allow(dead_code)]
pub fn response_kind(fixture: &UcFixture) -> ResponseKind {
    fixture.response_kind
}
