//! One signing lock, and requests whose answer was lost.
//!
//! Every signing operation runs under one lock (`commands::signing`):
//! a ticket number is read and used by one operation at a time, and a
//! Z sees every sale signed before it. A request is stored before it
//! is sent. When its answer is lost, it stays open and is sent again,
//! byte for byte, before anything else is signed: the FDM recognises
//! it (spec: same key fields, same content) and gives back its first
//! answer, so the event is archived once, under its first number.
//!
//! Signed by `pos_fdm_mock` behind [`LossyFdm`], which records every
//! request and loses answers on demand.

use pos_engine_testkit as common;

use std::time::Duration;

use common::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::financial::{MoneyDirection, do_money_in_out};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::report::{
    ReportResult, close_overdue_day, do_generate_user_x_report, do_generate_x_report,
    do_generate_z_report,
};
use pos_engine::commands::signing;
use pos_fiscal::types::EventLabel;

const OPERATOR: &str = "12345678958";

async fn app_with(fdm: &LossyFdm) -> TestApp {
    let app = TestApp::new().await;
    app.use_lossy_fdm(fdm);
    seed(&app);
    app
}

fn seed(app: &TestApp) {
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
             VALUES (1, 1, 1200, 'A', 0, 1);",
        )
        .unwrap();
}

fn new_order(app: &TestApp) -> i64 {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id;
    add_line(app, order).unwrap();
    order
}

fn add_line(app: &TestApp, order_id: i64) -> Result<(), String> {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .map(|_| ())
}

async fn sell(app: &TestApp) -> i64 {
    let order = new_order(app);
    let result = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    order
}

fn events_of(app: &TestApp, label: &str) -> Vec<FiscalEventRow> {
    app.fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == label)
        .collect()
}

fn ok_report(r: &Result<ReportResult, String>) -> bool {
    matches!(r, Ok(x) if x.success)
}

fn rr(r: &Result<ReportResult, String>) -> String {
    match r {
        Ok(x) => format!("Ok(success={}, error={:?})", x.success, x.error),
        Err(e) => format!("Err({e})"),
    }
}

fn age_open_period(app: &TestApp, days: i64) {
    let day_end = i64::from(app.state.config.lock().unwrap().day_end_hour);
    let date = (pos_fiscal::event::now_in_belgium()
        - chrono::Duration::hours(day_end)
        - chrono::Duration::days(days))
    .date_naive()
    .to_string();
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE booking_periods SET booking_date = ?1 WHERE closed_at IS NULL",
            rusqlite::params![date],
        )
        .unwrap();
}

fn z_ticket_counts(fdm: &LossyFdm) -> Vec<i64> {
    fdm.sent_op("signReportTurnoverZ")
        .iter()
        .filter(|s| s.answer.is_some())
        .map(|s| {
            s.variables()["turnover"]["turnover"]["transactions"][0]["ticketCount"]
                .as_i64()
                .unwrap_or(0)
        })
        .collect()
}

fn assert_pending(error: Option<&str>, label: &str, ticket: i64) {
    let error = error.expect("an error is shown");
    assert!(
        error.starts_with("FDM error: ")
            && error.contains(&format!(
                "{}|label={label}|ticket={ticket}",
                errors::FDM_ANSWER_PENDING
            )),
        "{error}"
    );
}

// ── Races ──────────────────────────────────────────────────────────

#[tokio::test]
async fn an_x_and_a_user_x_at_once_take_consecutive_numbers() {
    let fdm = LossyFdm::start().await;
    fdm.rule(|_, _| Fate::Delay(Duration::from_millis(200)));
    let app = app_with(&fdm).await;

    let (x, user_x) = tokio::join!(
        do_generate_x_report(&app.state, OPERATOR.into()),
        do_generate_user_x_report(&app.state, OPERATOR.into()),
    );

    assert!(
        ok_report(&x) && ok_report(&user_x),
        "{} / {}",
        rr(&x),
        rr(&user_x)
    );
    let numbers: Vec<i64> = events_of(&app, "R")
        .iter()
        .map(|e| e.sequence_number)
        .collect();
    assert_eq!(numbers, [1, 2]);
    let sent: Vec<_> = fdm.sent().into_iter().filter(|s| s.op != "other").collect();
    assert_eq!(sent.len(), 2, "the FDM signed as many events as archived");
    assert_ne!(
        sent[0].variables()["data"]["posFiscalTicketNo"],
        sent[1].variables()["data"]["posFiscalTicketNo"]
    );
}

#[tokio::test]
async fn the_automatic_z_and_a_manual_x_never_share_a_number() {
    let fdm = LossyFdm::start().await;
    fdm.rule(|_, _| Fate::Delay(Duration::from_millis(200)));
    let app = app_with(&fdm).await;
    sell(&app).await;
    age_open_period(&app, 2);

    let (z, x) = tokio::join!(
        close_overdue_day(&app.state),
        do_generate_x_report(&app.state, OPERATOR.into()),
    );

    assert_eq!(z, Ok(true), "the automatic Z runs first");
    assert!(
        ok_report(&x),
        "the X then goes into the new day: {}",
        rr(&x)
    );
    let archived: usize = events_of(&app, "R").len();
    let signed: usize = fdm
        .sent()
        .iter()
        .filter(|s| s.op.starts_with("signReport"))
        .filter_map(|s| s.answer.as_ref())
        .filter_map(|a| a["data"].as_object().map(|d| d.len()))
        .sum();
    assert_eq!(archived, 3, "turnover Z, user Z, X");
    assert_eq!(signed, archived, "every signed R event is archived");
    let numbers: Vec<i64> = events_of(&app, "R")
        .iter()
        .map(|e| e.sequence_number)
        .collect();
    assert_eq!(numbers, [1, 2, 3]);
}

#[tokio::test]
async fn two_checkouts_of_one_order_sign_one_sale() {
    let fdm = LossyFdm::start().await;
    fdm.rule(|_, _| Fate::Delay(Duration::from_millis(200)));
    let app = app_with(&fdm).await;
    let order = new_order(&app);

    let (a, b) = tokio::join!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Card),
        do_checkout(&app.state, order, CheckoutPaymentMethod::Card),
    );

    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(a.success, "{:?}", a.error);
    assert_eq!(b.error.as_deref(), Some(errors::ORDER_ALREADY_CLOSED));
    assert_eq!(fdm.sent_op("signSale").len(), 1);
    assert_eq!(events_of(&app, "N").len(), 1);
    assert_eq!(app.counter_for(EventLabel::N), 1);
}

#[tokio::test]
async fn a_z_waits_for_a_sale_in_flight_and_counts_it() {
    let fdm = LossyFdm::start().await;
    fdm.rule(|op, _| {
        if op == "signSale" {
            Fate::Delay(Duration::from_millis(500))
        } else {
            Fate::Answer
        }
    });
    let app = app_with(&fdm).await;
    sell(&app).await;
    let late = new_order(&app);

    let (sale, z) = tokio::join!(
        do_checkout(&app.state, late, CheckoutPaymentMethod::Card),
        do_generate_z_report(&app.state, OPERATOR.into()),
    );

    assert!(sale.unwrap().success);
    assert!(ok_report(&z), "{}", rr(&z));
    assert_eq!(z_ticket_counts(&fdm), [2], "the Z counts both sales");
    let conn = app.state.db.lock().unwrap();
    let periods: Vec<String> = conn
        .prepare("SELECT id FROM booking_periods ORDER BY rowid")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let in_z = pos_fiscal::archive::events_in_booking_period(&conn, &periods[0]).unwrap();
    assert_eq!(
        in_z.iter()
            .filter(|e| format!("{:?}", e.event_type) == "N")
            .count(),
        2
    );
}

#[tokio::test]
async fn a_manual_z_and_the_automatic_z_at_once_make_one_z() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    sell(&app).await;
    age_open_period(&app, 2);

    let (manual, automatic) = tokio::join!(
        do_generate_z_report(&app.state, OPERATOR.into()),
        close_overdue_day(&app.state),
    );

    assert!(ok_report(&manual), "{}", rr(&manual));
    assert_eq!(automatic, Ok(false), "busy, then nothing left to close");
    assert_eq!(close_overdue_day(&app.state).await, Ok(false));
    assert_eq!(fdm.sent_op("signReportTurnoverZ").len(), 1, "no empty Z");
}

#[tokio::test]
async fn the_automatic_z_skips_its_tick_while_signing_is_busy() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    sell(&app).await;
    age_open_period(&app, 2);

    let guard = signing::acquire(&app.state).await.unwrap();
    assert_eq!(close_overdue_day(&app.state).await, Ok(false));
    drop(guard);
    assert_eq!(close_overdue_day(&app.state).await, Ok(true));
}

// ── Lost answers ───────────────────────────────────────────────────

#[tokio::test]
async fn a_lost_sale_answer_is_replayed_byte_for_byte_and_archived_once() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signSale");
    let app = app_with(&fdm).await;
    let order = new_order(&app);

    let first = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(!first.success);
    assert_pending(first.error.as_deref(), "N", 1);
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.order_status(order).as_deref(), Some("open"));
    assert!(app.open_pending().is_some());

    let second = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(second.success, "{:?}", second.error);

    let sent = fdm.sent_op("signSale");
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].body, sent[1].body, "the same bytes sent again");
    let events = events_of(&app, "N");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1);
    assert_eq!(
        events[0].event_counter,
        Some(1),
        "the FDM's first event, not a second one"
    );
    assert_eq!(app.order_status(order).as_deref(), Some("closed"));
    assert_eq!(app.pending_statuses(), [("N".into(), 1, "signed".into())]);
}

#[tokio::test]
async fn a_request_lost_before_the_fdm_saw_it_is_signed_when_sent_again() {
    let fdm = LossyFdm::start().await;
    let lost = std::sync::atomic::AtomicBool::new(false);
    fdm.rule(move |op, _| {
        if op == "signSale" && !lost.swap(true, std::sync::atomic::Ordering::SeqCst) {
            Fate::LoseRequest
        } else {
            Fate::Answer
        }
    });
    let app = app_with(&fdm).await;
    let order = new_order(&app);

    let first = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert_pending(first.error.as_deref(), "N", 1);
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
            .await
            .unwrap()
            .success
    );

    let sent = fdm.sent_op("signSale");
    assert!(sent[0].answer.is_none() && sent[1].answer.is_some());
    assert_eq!(sent[0].body, sent[1].body);
    assert_eq!(events_of(&app, "N")[0].event_counter, Some(1));
}

#[tokio::test]
async fn the_background_tick_archives_a_lost_answer_on_its_own() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signSale");
    let app = app_with(&fdm).await;
    let order = new_order(&app);
    let first = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(!first.success);

    // The tick also runs once at boot.
    assert_eq!(close_overdue_day(&app.state).await, Ok(false));

    assert_eq!(events_of(&app, "N").len(), 1);
    assert_eq!(app.order_status(order).as_deref(), Some("closed"));
    assert!(app.open_pending().is_none());
}

#[tokio::test]
async fn an_order_with_a_pending_sale_cannot_be_changed() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signSale");
    let app = app_with(&fdm).await;
    let order = new_order(&app);
    let _ = do_checkout(&app.state, order, CheckoutPaymentMethod::Card).await;

    assert_eq!(
        add_line(&app, order).unwrap_err(),
        errors::ORDER_SIGN_PENDING
    );
}

#[tokio::test]
async fn a_pending_z_is_archived_before_the_next_sale() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signReportTurnoverZ");
    let app = app_with(&fdm).await;
    sell(&app).await;

    let z = do_generate_z_report(&app.state, OPERATOR.into())
        .await
        .unwrap();
    assert!(!z.success);
    assert_pending(z.error.as_deref(), "R", 1);

    sell(&app).await;

    let types: Vec<String> = app
        .fiscal_events()
        .into_iter()
        .map(|e| e.event_type)
        .collect();
    assert_eq!(types, ["N", "R", "R", "N"], "the Z before the new sale");
    assert_eq!(z_ticket_counts(&fdm), [1, 1], "one Z, sent twice");
    let conn = app.state.db.lock().unwrap();
    let z_period: String = conn
        .query_row(
            "SELECT id FROM booking_periods WHERE closed_at IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let in_z = pos_fiscal::archive::events_in_booking_period(&conn, &z_period).unwrap();
    assert_eq!(in_z.len(), 3, "the second sale is in the next day");
}

#[tokio::test]
async fn a_money_movement_sent_again_is_recorded_once() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signMoneyInOut");
    let app = app_with(&fdm).await;
    let move_in = || {
        do_money_in_out(
            &app.state,
            OPERATOR.into(),
            5000,
            MoneyDirection::In,
            "float".into(),
        )
    };

    let first = move_in().await.unwrap();
    assert!(!first.success);
    let second = move_in().await.unwrap();
    assert!(second.success, "{:?}", second.error);

    assert_eq!(events_of(&app, "F").len(), 1, "no second movement");
    let sent = fdm.sent_op("signMoneyInOut");
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].body, sent[1].body);
}

// ── Answers that settle the outcome ────────────────────────────────

#[tokio::test]
async fn a_refused_replay_frees_the_number_for_the_next_sale() {
    let app = TestApp::new().await;
    seed(&app);
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": null,
            "errors": [{"message": "invalid", "extensions": {"code": "INVALID_REQUEST"}}]
        })))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let order = new_order(&app);

    let first = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert_pending(first.error.as_deref(), "N", 1);
    let second = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(second.success, "{:?}", second.error);

    assert_eq!(events_of(&app, "N")[0].sequence_number, 1);
    assert_eq!(
        app.pending_statuses(),
        [
            ("N".into(), 1, "refused".into()),
            ("N".into(), 1, "signed".into())
        ]
    );
}

#[tokio::test]
async fn a_request_that_never_left_is_not_kept() {
    let app = TestApp::new().await;
    seed(&app);
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();
    let order = new_order(&app);

    let result = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();

    assert!(!result.success);
    assert!(app.open_pending().is_none());
    assert_eq!(app.pending_statuses(), [("N".into(), 1, "not_sent".into())]);
}
