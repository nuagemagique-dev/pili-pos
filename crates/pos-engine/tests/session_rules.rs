//! The engine owns leaving an order and the screen lock:
//!
//! - leaving a dine-in order registers what was ordered (event P);
//!   a takeaway and an order that is no longer open call no FDM;
//! - sending to the kitchen registers too, and keeps the order active;
//! - locking the session registers the active order while the operator
//!   is still logged in, then closes the session and emits
//!   `session-locked` once; a failing FDM never blocks the lock.

use std::sync::{Arc, Mutex};

use pos_engine_testkit as common;

use common::*;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::errors;
use pos_engine::commands::session::{
    LeaveReason, LockOutcome, LockReason, SessionLocked, do_leave_order, do_lock_session,
    do_set_active_order,
};
use pos_engine::{EngineEvent, EventSink};

const INSZ: &str = "12345678901";

/// Keeps every event, with the state of the session at the moment it
/// was emitted.
#[derive(Default)]
struct Recorder {
    events: Mutex<Vec<(String, Option<SessionLocked>)>>,
}

impl EventSink for Recorder {
    fn emit(&self, event: EngineEvent) {
        let payload = match &event {
            EngineEvent::SessionLocked(p) => Some(p.clone()),
            _ => None,
        };
        self.events
            .lock()
            .unwrap()
            .push((event.topic().to_string(), payload));
    }
}

impl Recorder {
    fn locked(&self) -> Vec<SessionLocked> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|(topic, _)| topic == "session-locked")
            .filter_map(|(_, p)| p.clone())
            .collect()
    }
}

async fn app_with_recorder() -> (TestApp, Arc<Recorder>) {
    let mut app = TestApp::new().await;
    let recorder = Arc::new(Recorder::default());
    app.state.events = recorder.clone();
    (app, recorder)
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

/// An order of `order_type` with one line of product 1.
fn order_with_line(app: &TestApp, order_type: OrderType, table: Option<i32>) -> i64 {
    seed_catalog(app);
    let conn = app.state.db.lock().unwrap();
    let order = pos_db::queries::orders::create_order(&conn, order_type, table, INSZ)
        .expect("create order");
    add_line(&conn, order.id);
    order.id
}

fn add_line(conn: &rusqlite::Connection, order_id: i64) {
    pos_db::queries::orders::add_order_line(
        conn,
        NewOrderLine {
            order_id,
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
}

fn dine_in(app: &TestApp) -> i64 {
    order_with_line(app, OrderType::DineIn, Some(1))
}

/// How many requests reached the FDM.
async fn fdm_requests(app: &TestApp) -> usize {
    app.fdm.received_requests().await.unwrap_or_default().len()
}

/// How many signOrder mutations reached the FDM.
async fn sign_orders(app: &TestApp) -> usize {
    app.fdm
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| String::from_utf8_lossy(&r.body).contains("signOrder"))
        .count()
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

#[tokio::test]
async fn leaving_a_dine_in_order_registers_it_once_and_clears_the_active_order() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();
    assert_eq!(app.state.session_rules.active_order(), Some(order_id));

    do_leave_order(&app.state, order_id, LeaveReason::Navigate)
        .await
        .expect("leave");
    assert_eq!(sign_orders(&app).await, 1);
    assert_eq!(app.state.session_rules.active_order(), None);
    assert_eq!(app.fiscal_events().len(), 1);
}

#[tokio::test]
async fn leaving_again_with_nothing_new_sends_nothing() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    do_leave_order(&app.state, order_id, LeaveReason::Navigate)
        .await
        .unwrap();
    assert_eq!(fdm_requests(&app).await, 1);

    let again = do_leave_order(&app.state, order_id, LeaveReason::SwitchOrder)
        .await
        .unwrap();
    assert_eq!(again, SlipOutcome::NothingToPrint);
    assert_eq!(fdm_requests(&app).await, 1, "no second request");
}

#[tokio::test]
async fn a_takeaway_is_a_direct_sale_and_calls_no_fdm() {
    let (app, _) = app_with_recorder().await;
    let order_id = order_with_line(&app, OrderType::Takeaway, None);
    do_set_active_order(&app.state, Some(order_id)).unwrap();

    let outcome = do_leave_order(&app.state, order_id, LeaveReason::Navigate)
        .await
        .expect("a takeaway is not an error");
    assert_eq!(outcome, SlipOutcome::NothingToPrint);
    assert_eq!(fdm_requests(&app).await, 0);
    assert_eq!(app.state.session_rules.active_order(), None);
}

#[tokio::test]
async fn an_order_that_is_not_open_calls_no_fdm() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE orders SET status = 'closed' WHERE id = ?1",
            rusqlite::params![order_id],
        )
        .unwrap();
    do_set_active_order(&app.state, Some(order_id)).unwrap();

    let outcome = do_leave_order(&app.state, order_id, LeaveReason::Navigate)
        .await
        .expect("a closed order is not an error");
    assert_eq!(outcome, SlipOutcome::NothingToPrint);
    assert_eq!(fdm_requests(&app).await, 0);
    assert_eq!(app.state.session_rules.active_order(), None);
}

#[tokio::test]
async fn sending_to_the_kitchen_registers_and_keeps_the_order_active() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();

    do_leave_order(&app.state, order_id, LeaveReason::SendToKitchen)
        .await
        .expect("send to the kitchen");
    assert_eq!(sign_orders(&app).await, 1);
    assert_eq!(app.state.session_rules.active_order(), Some(order_id));
}

#[tokio::test]
async fn a_failed_registration_keeps_the_active_order_and_the_entries_pending() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);
    fdm_answers_with_an_error(&app).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();

    do_leave_order(&app.state, order_id, LeaveReason::Navigate)
        .await
        .expect_err("the FDM refuses");
    assert_eq!(app.state.session_rules.active_order(), Some(order_id));
    let unsent = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_entries(&conn, order_id, true).unwrap()
    };
    assert_eq!(unsent.len(), 1, "the entry stays pending");
}

#[tokio::test]
async fn leaving_and_setting_the_active_order_need_a_session() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);
    app.logout();

    let err = do_leave_order(&app.state, order_id, LeaveReason::Navigate)
        .await
        .expect_err("no session");
    assert_eq!(err, errors::NOT_LOGGED_IN);
    let err = do_set_active_order(&app.state, Some(order_id)).expect_err("no session");
    assert_eq!(err, errors::NOT_LOGGED_IN);
    assert_eq!(fdm_requests(&app).await, 0);
}

#[tokio::test]
async fn the_active_order_must_exist_and_none_clears_it() {
    let (app, _) = app_with_recorder().await;
    let order_id = dine_in(&app);

    do_set_active_order(&app.state, Some(order_id + 1000)).expect_err("unknown order");
    assert_eq!(app.state.session_rules.active_order(), None);

    do_set_active_order(&app.state, Some(order_id)).unwrap();
    assert_eq!(app.state.session_rules.active_order(), Some(order_id));
    do_set_active_order(&app.state, None).unwrap();
    assert_eq!(app.state.session_rules.active_order(), None);
}

#[tokio::test]
async fn locking_registers_the_order_while_the_session_is_open_then_closes_it() {
    let (app, recorder) = app_with_recorder().await;
    let order_id = dine_in(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();

    let outcome = do_lock_session(&app.state).await.expect("lock");
    assert_eq!(outcome, LockOutcome { order_error: None });
    // The P was signed in the operator's name: the session was open.
    assert_eq!(sign_orders(&app).await, 1);
    assert_eq!(app.fiscal_events().len(), 1);
    assert!(app.state.session.lock().unwrap().is_none());
    assert_eq!(app.state.session_rules.active_order(), None);
    assert_eq!(
        recorder.locked(),
        vec![SessionLocked {
            reason: LockReason::Manual,
            order_error: None
        }]
    );
}

#[tokio::test]
async fn locking_without_an_active_order_just_closes_the_session() {
    let (app, recorder) = app_with_recorder().await;

    let outcome = do_lock_session(&app.state).await.expect("lock");
    assert_eq!(outcome.order_error, None);
    assert_eq!(fdm_requests(&app).await, 0);
    assert!(app.state.session.lock().unwrap().is_none());
    assert_eq!(recorder.locked().len(), 1);
}

#[tokio::test]
async fn an_fdm_failure_does_not_block_the_lock() {
    let (app, recorder) = app_with_recorder().await;
    let order_id = dine_in(&app);
    fdm_answers_with_an_error(&app).await;
    do_set_active_order(&app.state, Some(order_id)).unwrap();

    let outcome = do_lock_session(&app.state).await.expect("lock");
    let error = outcome
        .order_error
        .clone()
        .expect("the failure is reported");
    assert!(!error.is_empty());
    assert!(app.state.session.lock().unwrap().is_none());
    assert_eq!(app.state.session_rules.active_order(), None);
    assert_eq!(
        recorder.locked(),
        vec![SessionLocked {
            reason: LockReason::Manual,
            order_error: Some(error)
        }]
    );
    // The entry is still pending for the next registration.
    let unsent = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_entries(&conn, order_id, true).unwrap()
    };
    assert_eq!(unsent.len(), 1);
}

#[tokio::test]
async fn locking_without_a_session_does_nothing() {
    let (app, recorder) = app_with_recorder().await;
    let order_id = dine_in(&app);
    do_set_active_order(&app.state, Some(order_id)).unwrap();
    app.logout();

    let outcome = do_lock_session(&app.state).await.expect("lock");
    assert_eq!(outcome, LockOutcome { order_error: None });
    assert!(recorder.events.lock().unwrap().is_empty(), "no event");
    assert_eq!(fdm_requests(&app).await, 0);
}

#[test]
fn the_reasons_are_snake_case_strings() {
    let leave = [
        (LeaveReason::Navigate, "navigate"),
        (LeaveReason::SwitchOrder, "switch_order"),
        (LeaveReason::SendToKitchen, "send_to_kitchen"),
        (LeaveReason::Lock, "lock"),
        (LeaveReason::Idle, "idle"),
    ];
    for (reason, text) in leave {
        assert_eq!(serde_json::to_value(reason).unwrap(), json!(text));
        assert_eq!(
            serde_json::from_value::<LeaveReason>(json!(text)).unwrap(),
            reason
        );
    }
    for (reason, text) in [(LockReason::Manual, "manual"), (LockReason::Idle, "idle")] {
        assert_eq!(serde_json::to_value(reason).unwrap(), json!(text));
        assert_eq!(
            serde_json::from_value::<LockReason>(json!(text)).unwrap(),
            reason
        );
    }
}

#[test]
fn the_lock_payloads_are_camel_case() {
    let locked = SessionLocked {
        reason: LockReason::Manual,
        order_error: Some("fdm_unreachable".into()),
    };
    let value = serde_json::to_value(&locked).unwrap();
    assert_eq!(
        value,
        json!({ "reason": "manual", "orderError": "fdm_unreachable" })
    );
    assert_eq!(
        serde_json::from_value::<SessionLocked>(value).unwrap(),
        locked
    );
    assert_eq!(
        serde_json::to_value(LockOutcome { order_error: None }).unwrap(),
        json!({ "orderError": null })
    );
}

// ── Empty orders ───────────────────────────────────────────────────

/// An order created by opening a table or the takeaway tab, left with
/// nothing on it.
fn empty_order(app: &TestApp, order_type: OrderType, table: Option<i32>) -> i64 {
    seed_catalog(app);
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::orders::create_order(&conn, order_type, table, INSZ)
        .expect("create order")
        .id
}

#[tokio::test]
async fn leaving_an_empty_order_voids_it_whatever_the_screen() {
    let (app, _) = app_with_recorder().await;
    for (reason, order_type, table) in [
        (LeaveReason::Navigate, OrderType::DineIn, Some(1)),
        (LeaveReason::Navigate, OrderType::Takeaway, None),
        (LeaveReason::SwitchOrder, OrderType::Takeaway, None),
        (LeaveReason::Lock, OrderType::DineIn, Some(2)),
        (LeaveReason::Idle, OrderType::DineIn, Some(3)),
    ] {
        let id = empty_order(&app, order_type, table);
        do_set_active_order(&app.state, Some(id)).unwrap();
        do_leave_order(&app.state, id, reason).await.unwrap();
        assert_eq!(
            app.order_status(id).as_deref(),
            Some("voided"),
            "{reason:?}"
        );
        assert_eq!(app.state.session_rules.active_order(), None);
    }
    assert_eq!(fdm_requests(&app).await, 0, "nothing to register");
    let open = pos_db::queries::orders::list_open_orders(&app.state.db.lock().unwrap()).unwrap();
    assert!(open.is_empty());
}

#[tokio::test]
async fn sending_an_empty_order_to_the_kitchen_keeps_it_open() {
    let (app, _) = app_with_recorder().await;
    let id = empty_order(&app, OrderType::DineIn, Some(1));
    do_leave_order(&app.state, id, LeaveReason::SendToKitchen)
        .await
        .unwrap();
    assert_eq!(app.order_status(id).as_deref(), Some("open"));
}

#[tokio::test]
async fn an_order_with_a_line_is_never_voided_on_leaving() {
    let (app, _) = app_with_recorder().await;
    let id = order_with_line(&app, OrderType::Takeaway, None);
    do_leave_order(&app.state, id, LeaveReason::Navigate)
        .await
        .unwrap();
    assert_eq!(app.order_status(id).as_deref(), Some("open"));
}

/// A signed order whose lines were removed again keeps its fiscal
/// event: it stays open, never voided.
#[tokio::test]
async fn an_order_with_a_fiscal_event_is_never_voided_even_without_lines() {
    let (app, _) = app_with_recorder().await;
    let id = dine_in(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_leave_order(&app.state, id, LeaveReason::SendToKitchen)
        .await
        .unwrap();
    assert_eq!(app.fiscal_events().len(), 1);
    app.state
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM order_lines WHERE order_id = ?1", [id])
        .unwrap();

    do_leave_order(&app.state, id, LeaveReason::Navigate)
        .await
        .unwrap();
    assert_eq!(app.order_status(id).as_deref(), Some("open"));
}

#[tokio::test]
async fn an_order_with_a_terminal_payment_is_never_voided() {
    let (app, _) = app_with_recorder().await;
    let id = empty_order(&app, OrderType::DineIn, Some(1));
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO eft_payments
                (reference, order_id, terminal_kind, amount_cents, status, card_type, user_insz)
             VALUES ('ref-1', ?1, 'scripted', 1000, 'sent', NULL, ?2)",
            rusqlite::params![id, INSZ],
        )
        .unwrap();
    do_leave_order(&app.state, id, LeaveReason::Navigate)
        .await
        .unwrap();
    assert_eq!(app.order_status(id).as_deref(), Some("open"));
}
