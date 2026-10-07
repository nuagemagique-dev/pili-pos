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
//! The harness is deliberately thin - it doesn't try to model the full
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

use pili_pos::engine::EnginePaths;
use pili_pos::state::AppState;

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

/// The operator every TestApp is logged in as (an admin). Most suites
/// sign their events in this name.
pub const TEST_OPERATOR: &str = "12345678901";

/// In-process test app: an `AppState` + a wiremock server pretending
/// to be the FDM. Tests drop both at end-of-scope.
pub struct TestApp {
    pub state: AppState,
    /// The data directory of `state.paths`: it lives as long as the app,
    /// so a backup a Z report makes never lands in the developer's own
    /// data directory.
    pub data_dir: tempfile::TempDir,
    pub fdm: MockServer,
    /// Signing against a live FDM ([`TestApp::live`]): canned answers
    /// cannot be mounted.
    pub live: bool,
}

impl TestApp {
    /// Build a fresh `TestApp`. The SQLite DB is in-memory and the
    /// PosConfig points at the wiremock URL. The identity is the SPF
    /// test setup's (`UC_TestSetup_20240806.md`): POS xx567 on the
    /// BAR, the FPS Finances Experience, 25 tables.
    pub async fn new() -> Self {
        let fdm = MockServer::start().await;
        Self::with_fdm(fdm).await
    }

    /// Construct a TestApp around a caller-supplied wiremock server.
    /// Useful when a test needs to register expectations before the
    /// state is built.
    pub async fn with_fdm(fdm: MockServer) -> Self {
        let conn = Connection::open_in_memory().expect("in-memory sqlite");
        #[cfg(feature = "hubrise")]
        let poller = Connection::open_in_memory().expect("poller sqlite");
        Self::with_fdm_and_db(
            fdm,
            conn,
            #[cfg(feature = "hubrise")]
            poller,
        )
    }

    /// A TestApp whose UI and poller connections share one database
    /// file, as in production: what the HubRise poller writes, the
    /// commands read. The file lives until the returned guard drops.
    #[cfg(feature = "hubrise")]
    pub async fn with_shared_db() -> (Self, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pos.db");
        let conn = Connection::open(&path).expect("sqlite file");
        pos_db::schema::run_migrations(&conn).expect("migrate");
        let poller = Connection::open(&path).expect("poller sqlite");
        let fdm = MockServer::start().await;
        (Self::with_fdm_and_db(fdm, conn, poller), dir)
    }

    fn with_fdm_and_db(
        fdm: MockServer,
        conn: Connection,
        #[cfg(feature = "hubrise")] poller: Connection,
    ) -> Self {
        pos_db::schema::run_migrations(&conn).expect("migrate");

        let config = PosConfig {
            table_count: 25,
            pos_id: "CPOS0031234567".to_string(),
            terminal_id: "POS-1-BAR".to_string(),
            // The terminal's MAC address, as install fills it (§VI row VII).
            device_id: "F0:1D:3D:CC:13:C8".to_string(),
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
        };

        let data_dir = tempfile::tempdir().expect("test data dir");
        let state = AppState {
            paths: EnginePaths::under(data_dir.path().to_path_buf()),
            db: Mutex::new(conn),
            #[cfg(feature = "hubrise")]
            poller_db: Arc::new(Mutex::new(poller)),
            config: Mutex::new(config),
            #[cfg(feature = "hubrise")]
            shared_http: reqwest::Client::new(),
            #[cfg(feature = "hubrise")]
            hubrise_poll_now: Arc::new(tokio::sync::Notify::new()),
            #[cfg(feature = "hubrise")]
            hubrise_outbox_wake: Arc::new(tokio::sync::Notify::new()),
            #[cfg(feature = "hubrise")]
            hubrise_switch: pos_hubrise::switch::Switch::default(),
            shutdown_token: CancellationToken::new(),
            mail: pili_pos::commands::delivery::MailState::disabled(),
            session: Mutex::new(None),
            support: pili_pos::commands::support::SupportState::disabled(),
            pin_throttle: Default::default(),
            signing: Default::default(),
        };

        let app = Self {
            state,
            data_dir,
            fdm,
            live: false,
        };
        app.login_as(TEST_OPERATOR, pos_core::user::UserRole::Admin);
        app
    }

    /// Register `insz` as an active user with `role` and make them the
    /// logged-in operator, as `login` does. Every TestApp starts logged
    /// in as the admin [`TEST_OPERATOR`].
    pub fn login_as(&self, insz: &str, role: pos_core::user::UserRole) {
        {
            let conn = self.state.db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (insz, name, role, active) VALUES (?1, ?2, ?3, 1)
                 ON CONFLICT(insz) DO UPDATE SET role = excluded.role, active = 1",
                rusqlite::params![insz, format!("Test {insz}"), role.as_str()],
            )
            .unwrap();
        }
        let user = pos_core::user::User {
            insz: insz.to_string(),
            name: format!("Test {insz}"),
            role,
            active: true,
            has_pin: false,
        };
        *self.state.session.lock().unwrap() = Some(user);
    }

    /// End the session, as `logout` does.
    pub fn logout(&self) {
        *self.state.session.lock().unwrap() = None;
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

    /// The request whose answer is still awaited, if any.
    #[allow(dead_code)]
    pub fn open_pending(&self) -> Option<pos_db::queries::pending_signs::PendingSign> {
        let conn = self.state.db.lock().expect("db lock");
        pos_db::queries::pending_signs::open(&conn).expect("pending signs")
    }

    /// Every stored request, oldest first.
    #[allow(dead_code)]
    pub fn pending_statuses(&self) -> Vec<(String, i64, String)> {
        let conn = self.state.db.lock().expect("db lock");
        let mut stmt = conn
            .prepare("SELECT event_label, first_ticket_no, status FROM pending_signs ORDER BY id")
            .expect("prepare");
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("rows")
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
    /// - `<UC>_event<N>_<label>_request.json` - the event payload our
    ///   command layer actually built and sent to the FDM
    ///   (`fiscal_events.event_data_json`).
    /// - `<UC>_event<N>_<label>_response.json` - the (fixture) FDM
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

        // Only export events whose request was built by the command
        // layer (every real payload embeds the common identification
        // fields, so `posId` is always present). Rows seeded directly
        // in SQLite by a test fixture carry synthetic minimal JSON
        // and must not end up in the dossier evidence.
        let real: Vec<_> = self
            .fiscal_events()
            .into_iter()
            .filter(|ev| {
                ev.event_data_json
                    .as_deref()
                    .is_some_and(|j| j.contains("\"posId\""))
            })
            .collect();
        for (i, ev) in real.iter().enumerate() {
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
/// with the supplied envelope. Matches by HTTP method + path only - the
/// test relies on each command issuing exactly the mutation we're
/// servicing. For multi-step UCs, register multiple mocks; wiremock
/// dispatches in registration order via `up_to_n_times(1)`.
///
/// The envelope is replayed through [`EchoRequest`]: its business
/// content is the fixture's, but its identity fields answer the request
/// actually sent, as a real FDM does. Use [`mock_raw_response`] to send
/// an envelope exactly as given (for inconsistent-response tests).
pub async fn mock_response(app: &TestApp, response_envelope: Value) {
    assert!(!app.live, "canned FDM answers on a live app");
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(EchoRequest(response_envelope))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
}

/// A published non-training envelope as the FDM answers it in training
/// mode: every event is labelled T (POS detailed description v2.2
/// §2.2.4, last row of `tb_eventOperation`).
#[allow(dead_code)]
pub fn in_training(mut envelope: Value) -> Value {
    if let Some(fields) = envelope.get_mut("data").and_then(Value::as_object_mut) {
        for result in fields.values_mut() {
            if let Some(fdm_ref) = result.get_mut("fdmRef").and_then(Value::as_object_mut) {
                fdm_ref.insert("eventLabel".into(), Value::String("T".into()));
            }
        }
    }
    envelope
}

/// Short signature of the digital signature carried by the published
/// M110 sale response. The fixture's own `shortSignature` is a
/// placeholder (SHA-1 of nothing); the replay derives the real one.
#[allow(dead_code)]
pub const M110_SHORT_SIGNATURE: &str = "e2c43d6bbe9be265f2ecc1578368c224ff980c26";

/// Same as [`mock_response`] but replays the envelope byte for byte,
/// without echoing the request.
#[allow(dead_code)]
pub async fn mock_raw_response(app: &TestApp, response_envelope: Value) {
    assert!(!app.live, "canned FDM answers on a live app");
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_envelope))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
}

/// Replays an official response envelope as the answer to the request
/// the POS really sent.
///
/// The published fixtures were produced by another POS (`posId`
/// CPOS0031234567, ticket numbers 1000+, placeholder short signatures
/// on M110/M111/T*). The POS now refuses any response that does not
/// echo its request, so the replay copies `posId`, `posFiscalTicketNo`,
/// `posDateTime`, `deviceId` and `terminalId` from the request and
/// derives `shortSignature` from `digitalSignature` when the fixture
/// carries one. Lines, VAT, totals, labels and counters stay the
/// fixture's. Status queries and error envelopes pass through untouched.
pub struct EchoRequest(pub Value);

impl EchoRequest {
    pub fn answer(envelope: &Value, request_body: &[u8]) -> Value {
        let mut envelope = envelope.clone();
        let Ok(request) = serde_json::from_slice::<Value>(request_body) else {
            return envelope;
        };
        let variables = &request["variables"];
        let Some(fields) = envelope.get_mut("data").and_then(Value::as_object_mut) else {
            return envelope;
        };
        for (field, result) in fields.iter_mut() {
            let input = match field.as_str() {
                "signReportTurnoverZ" if variables.get("turnover").is_some() => {
                    &variables["turnover"]
                }
                "signReportUserZ" if variables.get("user").is_some() => &variables["user"],
                _ => &variables["data"],
            };
            echo_identity(result, input);
        }
        envelope
    }
}

/// Copy the request identity into one sign result (see [`EchoRequest`]).
pub fn echo_identity(result: &mut Value, input: &Value) {
    let (Some(result), Some(input)) = (result.as_object_mut(), input.as_object()) else {
        return;
    };
    if !result.contains_key("digitalSignature") {
        return;
    }
    for key in ["posId", "posFiscalTicketNo", "posDateTime", "deviceId"] {
        if let Some(value) = input.get(key) {
            result.insert(key.to_string(), value.clone());
        }
    }
    match input.get("terminalId") {
        Some(value) => result.insert("terminalId".to_string(), value.clone()),
        None => result.remove("terminalId"),
    };
    let has_short = result
        .get("shortSignature")
        .is_some_and(|short| !short.is_null());
    if has_short
        && let Some(short) = result
            .get("digitalSignature")
            .and_then(Value::as_str)
            .and_then(|sig| pos_fiscal::signature::compute_short_signature(sig).ok())
    {
        result.insert("shortSignature".to_string(), Value::String(short));
    }
}

impl wiremock::Respond for EchoRequest {
    fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(Self::answer(&self.0, &request.body))
    }
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
/// fixtures already include the wrapper - this is a fallback.
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
