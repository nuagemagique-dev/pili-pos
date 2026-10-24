//! The engine owns the idle lock (planner decision PD35): a session with
//! no activity for `timers.idle_lock` is locked by the idle loop of
//! `background.rs`, through the same code as the lock button, with
//! reason idle.
//!
//! The timeout is short (300 ms, real time). A negative assertion waits
//! at least three times the timeout; a positive one polls with a
//! generous deadline, so a slow machine cannot make a test flaky.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pos_engine_testkit as common;

use common::*;

use serde_json::json;
use tokio::task::JoinHandle;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pos_core::order::OrderType;
use pos_core::user::{User, UserRole};
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_engine::background::run_idle_lock;
use pos_engine::commands::access::open_session;
use pos_engine::commands::session::{
    LockReason, SessionLocked, do_set_active_order, do_user_activity,
};
use pos_engine::state::AppState;
use pos_engine::{EngineEvent, EventSink};

const INSZ: &str = "12345678901";
const TIMEOUT: Duration = Duration::from_millis(300);
/// Long enough to prove that nothing happens.
const QUIET: Duration = Duration::from_millis(1000);
const DEADLINE: Duration = Duration::from_secs(15);

/// Keeps every `session-locked` payload and the topics of all events.
#[derive(Default)]
struct Recorder {
    topics: Mutex<Vec<String>>,
    locked: Mutex<Vec<SessionLocked>>,
}

impl EventSink for Recorder {
    fn emit(&self, event: EngineEvent) {
        if let EngineEvent::SessionLocked(p) = &event {
            self.locked.lock().unwrap().push(p.clone());
        }
        self.topics.lock().unwrap().push(event.topic().to_string());
    }
}

impl Recorder {
    fn locked(&self) -> Vec<SessionLocked> {
        self.locked.lock().unwrap().clone()
    }

    fn topics(&self) -> Vec<String> {
        self.topics.lock().unwrap().clone()
    }
}

/// An engine state with a short idle lock and the idle loop running.
struct Idle {
    state: Arc<AppState>,
    fdm: MockServer,
    recorder: Arc<Recorder>,
    task: JoinHandle<()>,
    _data_dir: tempfile::TempDir,
}

impl Idle {
    async fn wait_for_locks(&self, count: usize) {
        let waited = tokio::time::timeout(DEADLINE, async {
            while self.recorder.locked().len() < count {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        assert!(
            waited.is_ok(),
            "{count} lock(s) expected, got {:?}",
            self.recorder.locked()
        );
    }

    fn session_open(&self) -> bool {
        self.state.session.lock().unwrap().is_some()
    }

    /// How many signOrder mutations reached the FDM.
    async fn sign_orders(&self) -> usize {
        self.fdm
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| String::from_utf8_lossy(&r.body).contains("signOrder"))
            .count()
    }

    async fn fdm_requests(&self) -> usize {
        self.fdm.received_requests().await.unwrap_or_default().len()
    }

    /// The operators of the fiscal events archived so far.
    fn fiscal_operators(&self) -> Vec<String> {
        let db = self.state.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT user_insz FROM fiscal_events ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }
}

fn operator() -> User {
    User {
        insz: INSZ.to_string(),
        name: "Test".to_string(),
        role: UserRole::Cashier,
        active: true,
        has_pin: false,
        presence_only: false,
    }
}

fn seed_catalog(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT OR IGNORE INTO users (insz, name, role, active)
            VALUES ('12345678901', 'Test', 'cashier', 1);
         INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
            VALUES (1, 1, 1000, 'B', 0, 1);",
    )
    .unwrap();
}

/// A dine-in order with one unsent line.
fn dine_in_with_a_line(app: &TestApp) -> i64 {
    seed_catalog(app);
    let conn = app.state.db.lock().unwrap();
    let order = pos_db::queries::orders::create_order(&conn, OrderType::DineIn, Some(1), INSZ)
        .expect("create order");
    pos_db::queries::orders::add_order_line(
        &conn,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 1,
            quantity: 1,
            unit_price_cents: 1000,
            vat_code: VatCode::B,
            name: "Test item",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .expect("add line");
    order.id
}

async fn fdm_answers_with_an_error(app: &TestApp) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{ "message": "internal FDM error (synthetic, from the test suite)" }]
        })))
        .mount(&app.fdm)
        .await;
}

/// Share the state of `app`, give it the recorder and the short idle
/// lock, restart the idle clock and start the loop.
fn start(app: TestApp, recorder: Arc<Recorder>) -> Idle {
    let TestApp {
        mut state,
        data_dir,
        fdm,
        ..
    } = app;
    state.events = recorder.clone();
    state.timers.idle_lock = TIMEOUT;
    let state = Arc::new(state);
    // The session of the TestApp was opened by the harness, which does
    // not touch the clock: a login would.
    state.session_rules.touch();
    let task = tokio::spawn(run_idle_lock(state.clone()));
    Idle {
        state,
        fdm,
        recorder,
        task,
        _data_dir: data_dir,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_lock_locks_an_unattended_session_once_with_its_order_registered() {
    let app = TestApp::new().await;
    let order_id = dine_in_with_a_line(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();
    // Taken before `start` touches the clock, so the lock cannot come
    // earlier than one timeout after it.
    let started = Instant::now();
    let idle = start(app, Arc::new(Recorder::default()));

    idle.wait_for_locks(1).await;
    assert!(started.elapsed() >= TIMEOUT, "locked before the timeout");
    assert_eq!(
        idle.recorder.locked(),
        vec![SessionLocked {
            reason: LockReason::Idle,
            order_error: None
        }]
    );
    // The P was signed while the session was still open: in the
    // operator's name, one signOrder.
    assert_eq!(idle.sign_orders().await, 1);
    assert_eq!(idle.fiscal_operators(), vec![INSZ.to_string()]);
    assert!(!idle.session_open());
    assert_eq!(idle.state.session_rules.active_order(), None);

    // Once: nothing more happens while the screen stays locked.
    tokio::time::sleep(QUIET).await;
    assert_eq!(idle.recorder.locked().len(), 1);
    assert_eq!(idle.sign_orders().await, 1);
    assert_eq!(idle.recorder.topics(), vec!["session-locked".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_lock_waits_while_activity_comes_at_shorter_intervals() {
    let app = TestApp::new().await;
    let idle = start(app, Arc::new(Recorder::default()));

    // More than three timeouts of activity every 100 ms.
    let busy_until = Instant::now() + QUIET + TIMEOUT;
    let mut last_touch = Instant::now();
    while Instant::now() < busy_until {
        last_touch = Instant::now();
        do_user_activity(&idle.state).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(idle.session_open(), "locked while the operator was active");
    }
    assert!(idle.recorder.topics().is_empty(), "no event while active");

    // Activity stops: the lock comes one timeout after the last touch.
    idle.wait_for_locks(1).await;
    assert!(
        last_touch.elapsed() >= TIMEOUT,
        "locked before a full timeout"
    );
    assert!(!idle.session_open());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_lock_does_nothing_without_a_session() {
    let app = TestApp::new().await;
    let order_id = dine_in_with_a_line(&app);
    do_set_active_order(&app.state, Some(order_id)).unwrap();
    app.logout();
    let idle = start(app, Arc::new(Recorder::default()));

    tokio::time::sleep(QUIET).await;
    assert!(idle.recorder.topics().is_empty(), "no event");
    assert_eq!(idle.fdm_requests().await, 0, "no FDM request");
    assert_eq!(
        idle.state.session_rules.active_order(),
        Some(order_id),
        "nothing was touched"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_lock_still_locks_when_the_fdm_fails_and_reports_the_order_error() {
    let app = TestApp::new().await;
    let order_id = dine_in_with_a_line(&app);
    fdm_answers_with_an_error(&app).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();
    let idle = start(app, Arc::new(Recorder::default()));

    idle.wait_for_locks(1).await;
    let locked = idle.recorder.locked();
    assert_eq!(locked.len(), 1);
    assert_eq!(locked[0].reason, LockReason::Idle);
    assert!(
        locked[0]
            .order_error
            .as_deref()
            .is_some_and(|e| !e.is_empty()),
        "the failure is reported: {:?}",
        locked[0]
    );
    assert!(!idle.session_open());
    // The entry stays pending for the next registration.
    let unsent = {
        let conn = idle.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_entries(&conn, order_id, true).unwrap()
    };
    assert_eq!(unsent.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_lock_re_arms_at_every_new_login() {
    let app = TestApp::new().await;
    let idle = start(app, Arc::new(Recorder::default()));
    idle.wait_for_locks(1).await;
    assert!(!idle.session_open());

    // The next login restarts the idle time, as `login` does.
    let logged_in = Instant::now();
    open_session(&idle.state, &operator()).unwrap();
    assert!(idle.session_open());
    idle.wait_for_locks(2).await;
    assert!(
        logged_in.elapsed() >= TIMEOUT,
        "the second lock came before a full timeout after the login"
    );
    assert!(!idle.session_open());
    assert!(
        idle.recorder
            .locked()
            .iter()
            .all(|l| l.reason == LockReason::Idle)
    );

    // And a third time, to show the loop keeps waiting for logins.
    open_session(&idle.state, &operator()).unwrap();
    idle.wait_for_locks(3).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_lock_loop_ends_on_shutdown_and_no_lock_follows() {
    let app = TestApp::new().await;
    let idle = start(app, Arc::new(Recorder::default()));

    idle.state.shutdown_token.cancel();
    let Idle {
        state,
        task,
        recorder,
        ..
    } = idle;
    let finished = tokio::time::timeout(Duration::from_secs(2), task).await;
    assert!(finished.is_ok(), "the idle loop outlived shutdown");
    finished.unwrap().expect("the idle loop panicked");

    tokio::time::sleep(QUIET).await;
    assert!(recorder.topics().is_empty(), "no lock after shutdown");
    assert!(state.session.lock().unwrap().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_user_activity_needs_no_session() {
    let app = TestApp::new().await;
    app.logout();
    do_user_activity(&app.state).expect("activity without a session is not an error");
}
