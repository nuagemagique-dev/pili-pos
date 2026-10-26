//! Correcting the payment method of a sale (event F,
//! `signPaymentCorrection`, Basic Set M132, `commands/payment_correction.rs`):
//! every refusal, one correction per sale, a split sale corrected one
//! line at a time, an FDM that is down or answers late, the X / Z and
//! user report totals, the printed report line, training and access.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::sync::Arc;

use pos_engine_testkit as common;

use common::*;
use pos_eft::CardType;
use pos_eft::scripted::Script;
use serde_json::Value;

use pos_core::user::UserRole;
use pos_engine::commands::auth::do_set_pin;
use pos_engine::commands::checkout::{
    CheckoutPaymentMethod, PaymentPart, do_checkout, do_checkout_split,
};
use pos_engine::commands::eft::do_eft_pay;
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::payment_correction::{
    SalePayments, do_correct_payment, do_list_sale_payments,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::{
    close_overdue_day, do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use pos_engine::commands::unpaid::do_close_unpaid;
use pos_fiscal::payment::PaymentMethod;
use pos_fiscal::types::EventLabel;

const MANAGER: &str = "12345678958";
const WAITER: &str = "84022899837";
const PIN: &str = "4321";

async fn setup(fdm: &LossyFdm) -> TestApp {
    let mut app = TestApp::new().await;
    app.use_lossy_fdm(fdm);
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    // A terminal is linked, for the card it approves.
    let _ = app.link_terminal();
    app
}

async fn app() -> (TestApp, LossyFdm) {
    let fdm = LossyFdm::start().await;
    let app = setup(&fdm).await;
    (app, fdm)
}

/// A sale of one product at `cents`, paid in `method`; returns the
/// archive id of the N.
async fn sale(app: &TestApp, cents: i64, method: CheckoutPaymentMethod) -> i64 {
    let order = app.order_of(cents);
    let r = do_checkout(&app.state, order, method).await.unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

async fn cash_sale(app: &TestApp, cents: i64) -> i64 {
    sale(app, cents, CheckoutPaymentMethod::Cash).await
}

/// A sale paid `card_cents` by card, the rest in cash.
async fn split_sale(app: &TestApp, total: i64, card_cents: i64) -> i64 {
    let order = app.order_of(total);
    let parts = [
        PaymentPart::of(PaymentMethod::Card, card_cents),
        PaymentPart::cash(),
    ];
    let r = do_checkout_split(&app.state, order, &parts, None)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

async fn correct(app: &TestApp, event: i64, line: i32, method: PaymentMethod) -> Option<String> {
    let r = do_correct_payment(&app.state, event, line, method, MANAGER.into(), PIN.into())
        .await
        .unwrap();
    if r.success { None } else { r.error }
}

async fn refused(app: &TestApp, event: i64, line: i32, method: PaymentMethod) -> String {
    correct(app, event, line, method)
        .await
        .expect("the correction is refused")
}

fn list(app: &TestApp, event: i64) -> SalePayments {
    do_list_sale_payments(&app.state, event).unwrap()
}

fn requests(app: &TestApp) -> Vec<Value> {
    app.fiscal_events()
        .iter()
        .filter_map(|e| e.event_data_json.as_deref())
        .map(|j| serde_json::from_str(j).unwrap())
        .collect()
}

fn corrections(app: &TestApp) -> Vec<Value> {
    requests(app)
        .into_iter()
        .filter(|r| r.get("fdmRef").is_some() && r.get("financials").is_some())
        .filter(|r| r.get("transaction").is_none())
        .collect()
}

fn event_count(app: &TestApp) -> usize {
    app.fiscal_events().len()
}

// ── The correction itself ──────────────────────────────────────────

#[tokio::test]
async fn a_cash_sale_is_corrected_to_a_card() {
    let (app, fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    let before = list(&app, n);
    assert!(before.correctable, "{:?}", before.refusal);
    assert_eq!((before.total_cents, before.lines.len()), (5200, 1));
    assert_eq!(
        (
            before.lines[0].payment_type.as_str(),
            before.lines[0].input_method.as_str()
        ),
        ("CASH", "MANUAL")
    );
    assert!(before.corrected.is_none());

    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);

    // One F, no ticket, no drawer, naming the sale it corrects.
    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "F");
    assert_eq!(app.counter_for(EventLabel::F), 1);
    let f = &corrections(&app)[0];
    assert_eq!(f["ticketMedium"], "NONE");
    assert!(f.get("drawer").is_none());
    assert_eq!(f["employeeId"], MANAGER);
    let sale = &requests(&app)[0];
    let sold = serde_json::from_str::<Value>(events[0].fdm_response_json.as_deref().unwrap())
        .unwrap()["fdmRef"]
        .clone();
    assert_eq!(f["fdmRef"], sold);
    let lines: Vec<_> = f["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["type"].as_str().unwrap(),
                l["amount"].as_f64().unwrap(),
                l["inputMethod"].as_str().unwrap(),
                l["amountType"].as_str().unwrap(),
                l["id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        vec![
            ("CASH", -52.0, "MANUAL", "PAYMENT", "1"),
            ("CARD_DEBIT", 52.0, "MANUAL", "PAYMENT", "2"),
        ]
    );
    assert_eq!(sale["financials"][0]["type"], "CASH");
    assert_eq!(fdm.sent_op("signPaymentCorrection").len(), 1);

    // The sale shows what was done; the correction is a one-off.
    let after = list(&app, n);
    let done = after.corrected.expect("the correction is shown");
    assert_eq!(
        (
            done.from_type.as_str(),
            done.to_type.as_str(),
            done.amount_cents
        ),
        ("CASH", "CARD_DEBIT", 5200)
    );
    assert!(!after.correctable);
    assert_eq!(
        after.refusal.as_deref(),
        Some(errors::PAYMENT_CORRECTION_ALREADY_DONE)
    );
}

#[tokio::test]
async fn every_till_method_can_be_the_target() {
    let (app, _fdm) = app().await;
    let cash = cash_sale(&app, 3000).await;
    assert_eq!(
        correct(&app, cash, 0, PaymentMethod::MealVoucher).await,
        None
    );
    let voucher = &corrections(&app)[0]["financials"];
    assert_eq!(voucher[1]["type"], "CHEQUE_MEAL");
    assert_eq!(voucher[1]["id"], "3");

    let card = sale(&app, 3000, CheckoutPaymentMethod::Card).await;
    assert_eq!(correct(&app, card, 0, PaymentMethod::Cash).await, None);
    let cash_back = &corrections(&app)[1]["financials"];
    assert_eq!(cash_back[0]["type"], "CARD_DEBIT");
    assert_eq!(cash_back[0]["amount"], -30.0);
    assert_eq!(cash_back[1]["type"], "CASH");
    assert_eq!(cash_back[1]["id"], "1");
}

#[tokio::test]
async fn a_split_sale_corrects_one_line_and_leaves_the_other() {
    let (app, _fdm) = app().await;
    let n = split_sale(&app, 5200, 2000).await;
    let before = list(&app, n);
    let kinds: Vec<_> = before
        .lines
        .iter()
        .map(|l| (l.index, l.payment_type.as_str(), l.amount_cents))
        .collect();
    assert_eq!(kinds, vec![(0, "CARD_DEBIT", 2000), (1, "CASH", 3200)]);

    // The 20 EUR card goes to cash: 20,00 is a multiple of 5 cents.
    assert_eq!(correct(&app, n, 0, PaymentMethod::Cash).await, None);
    let f = &corrections(&app)[0];
    assert_eq!(f["financials"][0]["type"], "CARD_DEBIT");
    assert_eq!(f["financials"][0]["amount"], -20.0);
    assert_eq!(f["financials"][1]["type"], "CASH");
    assert_eq!(f["financials"][1]["amount"], 20.0);
    // The other line was not touched: the F names only the one line.
    assert_eq!(f["financials"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn nothing_is_ever_printed_or_opened_by_a_correction() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let n = cash_sale(&app, 1500).await;
    // The sale's own receipt and drawer kick.
    printer.settle(1).await;
    let kicks = printer.drawer_kicks();
    assert_eq!(kicks, 1);
    let printed_jobs = printer.rendered().len();

    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);

    printer.settle(2).await;
    assert_eq!(printer.drawer_kicks(), kicks);
    assert_eq!(printer.rendered().len(), printed_jobs);
}

// ── Refusals ───────────────────────────────────────────────────────

#[tokio::test]
async fn a_wrong_pin_or_a_waiter_signs_nothing() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    let events = event_count(&app);

    let wrong = do_correct_payment(
        &app.state,
        n,
        0,
        PaymentMethod::Card,
        MANAGER.into(),
        "0000".into(),
    )
    .await
    .unwrap();
    assert_eq!(wrong.error.as_deref(), Some(errors::PIN_WRONG));
    // Someone else's name is not the operator's.
    let other = do_correct_payment(
        &app.state,
        n,
        0,
        PaymentMethod::Card,
        WAITER.into(),
        PIN.into(),
    )
    .await
    .unwrap();
    assert!(!other.success);

    app.login_as(WAITER, UserRole::Waiter);
    do_set_pin(&app.state, WAITER.into(), PIN.into(), None).unwrap();
    let waiter = do_correct_payment(
        &app.state,
        n,
        0,
        PaymentMethod::Card,
        WAITER.into(),
        PIN.into(),
    )
    .await
    .unwrap();
    assert_eq!(waiter.error.as_deref(), Some(errors::FORBIDDEN));
    assert!(do_list_sale_payments(&app.state, n).is_err());

    app.logout();
    assert!(do_list_sale_payments(&app.state, n).is_err());
    assert_eq!(event_count(&app), events);
}

#[tokio::test]
async fn a_method_the_till_does_not_take_is_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    for method in [PaymentMethod::App, PaymentMethod::Online] {
        assert_eq!(
            refused(&app, n, 0, method).await,
            errors::PAYMENT_CORRECTION_METHOD
        );
    }
    assert_eq!(event_count(&app), 1);
}

#[tokio::test]
async fn the_same_method_or_an_unknown_line_is_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Cash).await,
        errors::PAYMENT_CORRECTION_SAME_METHOD
    );
    for line in [-1, 1, 7] {
        assert_eq!(
            refused(&app, n, line, PaymentMethod::Card).await,
            errors::PAYMENT_CORRECTION_BAD_LINE,
            "line {line}"
        );
    }
    assert_eq!(event_count(&app), 1);
}

#[tokio::test]
async fn a_second_correction_of_the_same_sale_is_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);
    // Whatever the line or the method.
    for (line, method) in [
        (0, PaymentMethod::Cash),
        (1, PaymentMethod::MealVoucher),
        (0, PaymentMethod::Card),
    ] {
        assert_eq!(
            refused(&app, n, line, method).await,
            errors::PAYMENT_CORRECTION_ALREADY_DONE
        );
    }
    assert_eq!(corrections(&app).len(), 1);
}

#[tokio::test]
async fn a_sale_of_a_closed_day_is_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    let z = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);

    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_PERIOD_CLOSED
    );
    let shown = list(&app, n);
    assert!(!shown.correctable);
    assert_eq!(
        shown.refusal.as_deref(),
        Some(errors::PAYMENT_CORRECTION_PERIOD_CLOSED)
    );
    // The sale of the new day is corrected normally.
    let today = cash_sale(&app, 900).await;
    assert_eq!(correct(&app, today, 0, PaymentMethod::Card).await, None);
}

#[tokio::test]
async fn a_refund_and_a_sale_closed_unpaid_are_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    let item: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT MAX(id) FROM menu_items", [], |r| r.get(0))
        .unwrap();
    let r = do_refund_checkout(
        &app.state,
        n,
        vec![RefundItem {
            menu_item_id: item,
            quantity: 1,
            line_id: None,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);
    let refund = r.fiscal_event_id.unwrap();

    // The refund is no sale; the refunded sale is out of reach too.
    assert_eq!(
        refused(&app, refund, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_NOT_A_SALE
    );
    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_REFUNDED
    );
    assert_eq!(
        list(&app, n).refusal.as_deref(),
        Some(errors::PAYMENT_CORRECTION_REFUNDED)
    );

    let order = app.order_of(1800);
    let unpaid = do_close_unpaid(&app.state, order, MANAGER, PIN)
        .await
        .unwrap();
    assert!(unpaid.success, "{:?}", unpaid.error);
    assert_eq!(
        refused(
            &app,
            unpaid.fiscal_event_id.unwrap(),
            0,
            PaymentMethod::Card
        )
        .await,
        errors::PAYMENT_CORRECTION_UNPAID
    );
}

fn last_item(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT MAX(id) FROM menu_items", [], |r| r.get(0))
        .unwrap()
}

fn one_of(item: i64) -> Vec<RefundItem> {
    vec![RefundItem {
        menu_item_id: item,
        quantity: 1,
        line_id: None,
    }]
}

/// The real refund command in training mode (V1F-D69): it never signs a
/// training refund, of a training sale or of a real one, so nothing is
/// archived; the correction of the real sale stays refused in training,
/// and the training sale, never refunded, stays correctable.
#[tokio::test]
async fn a_refund_run_in_training_signs_nothing_and_the_correction_is_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    let real_item = last_item(&app);
    app.state.config.lock().unwrap().training_mode = true;
    let t = cash_sale(&app, 3000).await;
    let training_item = last_item(&app);
    let count = event_count(&app);

    // A training sale (label T) is not refundable.
    let error = do_refund_checkout(&app.state, t, one_of(training_item), MANAGER.into())
        .await
        .expect_err("a T sale is not refundable");
    assert!(error.starts_with(errors::NOT_REFUNDABLE), "{error}");
    // A real sale is not refunded in training.
    let r = do_refund_checkout(&app.state, n, one_of(real_item), MANAGER.into())
        .await
        .unwrap();
    assert!(!r.success);
    assert_eq!(r.error.as_deref(), Some(errors::REFUND_IN_TRAINING));
    assert_eq!(event_count(&app), count, "no training refund is signed");

    // The correction of the real sale is refused in training.
    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_TRAINING
    );
    assert_eq!(event_count(&app), count, "nothing signed");
    assert!(list(&app, t).refusal.is_none(), "{:?}", list(&app, t));
    assert_eq!(correct(&app, t, 0, PaymentMethod::Card).await, None);
}

/// In training mode every event is labelled T: a refund and a
/// correction of a training sale are told apart by their FDM operation,
/// not by the label.
#[tokio::test]
async fn training_events_are_told_apart_by_their_fdm_operation() {
    let (app, _fdm) = app().await;
    app.state.config.lock().unwrap().training_mode = true;
    let n = cash_sale(&app, 5200).await;
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);
    let last = app.fiscal_events().pop().unwrap();
    assert_eq!(last.event_type, "T");

    // The T correction counts as the correction, and not as a refund.
    assert_eq!(
        refused(&app, n, 0, PaymentMethod::MealVoucher).await,
        errors::PAYMENT_CORRECTION_ALREADY_DONE
    );
    assert!(list(&app, n).corrected.is_some());

    // A T event that names a second training sale and that the FDM
    // answered as a SALE is a refund of it: it blocks the correction
    // and is not taken for a correction.
    let m = cash_sale(&app, 3000).await;
    let events = app.fiscal_events();
    let sale_m = events.iter().find(|e| e.id == m).unwrap();
    let mut data: serde_json::Value =
        serde_json::from_str(last.event_data_json.as_deref().unwrap()).unwrap();
    let response: serde_json::Value =
        serde_json::from_str(sale_m.fdm_response_json.as_deref().unwrap()).unwrap();
    data["fdmRef"] = response["fdmRef"].clone();
    let answer = last
        .fdm_response_json
        .unwrap()
        .replace("PAYMENT_CORRECTION", "SALE");
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id,
                 user_insz, event_data_json, fdm_response_json)
             VALUES ('T', 9999, 'p', 't', ?1, ?2, ?3)",
            rusqlite::params![MANAGER, data.to_string(), answer],
        )
        .unwrap();
    assert_eq!(
        refused(&app, m, 0, PaymentMethod::MealVoucher).await,
        errors::PAYMENT_CORRECTION_REFUNDED
    );
    let listed = list(&app, m);
    assert_eq!(
        listed.refusal.as_deref(),
        Some(errors::PAYMENT_CORRECTION_REFUNDED)
    );
    assert!(listed.corrected.is_none(), "a refund is no correction");
}

#[tokio::test]
async fn an_event_that_is_not_a_sale_is_refused() {
    let (app, _fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    // A copy is no sale either.
    let copy = pos_engine::commands::copy::do_copy_ticket(&app.state, n, MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    assert_eq!(
        refused(&app, copy.fiscal_event_id.unwrap(), 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_NOT_A_SALE
    );
    assert_eq!(
        do_list_sale_payments(&app.state, 99_999).unwrap_err(),
        errors::EVENT_NOT_FOUND
    );
}

#[tokio::test]
async fn an_online_order_is_refused() {
    let (app, _fdm) = app().await;
    let order = app.order_of(2500);
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let n = r.fiscal_event_id.unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE orders SET hubrise_order_id = 'HR-1' WHERE id = ?1",
            [order],
        )
        .unwrap();

    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_ONLINE
    );
    assert!(!list(&app, n).correctable);
}

#[tokio::test]
async fn a_card_the_terminal_approved_is_never_corrected() {
    let fdm = LossyFdm::start().await;
    let mut app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    let script: Arc<Script> = app.link_terminal();

    // The whole sale is one terminal card: nothing to pick.
    let order = app.order_of(2200);
    script.on_pay(Ok(approval(2200, "Bancontact", CardType::Debit)));
    let payment = do_eft_pay(&app.state, order, 2200).await.unwrap();
    let parts = [PaymentPart::terminal(2200, payment.id)];
    let sold = do_checkout_split(&app.state, order, &parts, None)
        .await
        .unwrap();
    assert!(sold.success, "{:?}", sold.error);
    let n = sold.fiscal_event_id.unwrap();

    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Cash).await,
        errors::PAYMENT_CORRECTION_TERMINAL_CARD
    );
    let shown = list(&app, n);
    assert!(!shown.correctable);
    assert_eq!(
        shown.refusal.as_deref(),
        Some(errors::PAYMENT_CORRECTION_TERMINAL_CARD)
    );
    assert_eq!(shown.lines[0].input_method, "AUTOMATIC");
    assert_eq!(shown.lines[0].brand.as_deref(), Some("Bancontact"));

    // A split sale: the terminal line stays out, the cash line is open.
    let order = app.order_of(5200);
    script.on_pay(Ok(approval(2000, "Bancontact", CardType::Debit)));
    let payment = do_eft_pay(&app.state, order, 2000).await.unwrap();
    let parts = [PaymentPart::terminal(2000, payment.id), PaymentPart::cash()];
    let split = do_checkout_split(&app.state, order, &parts, None)
        .await
        .unwrap()
        .fiscal_event_id
        .unwrap();
    assert!(list(&app, split).correctable);
    assert_eq!(
        refused(&app, split, 0, PaymentMethod::Cash).await,
        errors::PAYMENT_CORRECTION_TERMINAL_CARD
    );
    assert_eq!(
        correct(&app, split, 1, PaymentMethod::MealVoucher).await,
        None
    );
}

// ── Rounding ───────────────────────────────────────────────────────

#[tokio::test]
async fn a_cash_line_of_a_rounded_sale_is_refused() {
    let (app, _fdm) = app().await;
    // 52,03 in cash: 52,05 with a +0,02 ROUNDING line.
    let n = cash_sale(&app, 5203).await;
    let shown = list(&app, n);
    assert_eq!(
        shown
            .lines
            .iter()
            .map(|l| l.amount_type.as_str())
            .collect::<Vec<_>>(),
        vec!["PAYMENT", "ROUNDING"]
    );
    assert_eq!(
        refused(&app, n, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_ROUNDING
    );
    // The rounding line itself is not a payment.
    assert_eq!(
        refused(&app, n, 1, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_BAD_LINE
    );
    assert_eq!(event_count(&app), 1);
}

#[tokio::test]
async fn a_card_amount_that_cash_could_not_take_is_refused() {
    let (app, _fdm) = app().await;
    // 52,03 by card is exact; as cash it would be rounded.
    let card = sale(&app, 5203, CheckoutPaymentMethod::Card).await;
    assert_eq!(
        refused(&app, card, 0, PaymentMethod::Cash).await,
        errors::PAYMENT_CORRECTION_ROUNDING
    );
    // To a meal voucher the exact amount is fine.
    assert_eq!(
        correct(&app, card, 0, PaymentMethod::MealVoucher).await,
        None
    );
    // And a multiple of 5 cents goes to cash.
    let even = sale(&app, 5205, CheckoutPaymentMethod::Card).await;
    assert_eq!(correct(&app, even, 0, PaymentMethod::Cash).await, None);
}

// ── Training ───────────────────────────────────────────────────────

#[tokio::test]
async fn a_sale_and_its_correction_must_be_of_the_same_mode() {
    let (app, _fdm) = app().await;
    let real = cash_sale(&app, 5200).await;
    app.state.config.lock().unwrap().training_mode = true;
    let training = cash_sale(&app, 5200).await;

    // In training only the training sale is corrected.
    assert_eq!(
        refused(&app, real, 0, PaymentMethod::Card).await,
        errors::PAYMENT_CORRECTION_TRAINING
    );
    assert_eq!(correct(&app, training, 0, PaymentMethod::Card).await, None);
    let last = app.fiscal_events().pop().unwrap();
    assert_eq!(last.event_type, "T", "a training correction is a T");
    assert_eq!(app.counter_for(EventLabel::F), 0);

    app.state.config.lock().unwrap().training_mode = false;
    let n = cash_sale(&app, 100).await;
    assert_eq!(
        refused(&app, training, 0, PaymentMethod::MealVoucher).await,
        errors::PAYMENT_CORRECTION_TRAINING
    );
    // The real sale can be corrected again outside training.
    assert_eq!(correct(&app, real, 0, PaymentMethod::Card).await, None);
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);
}

// ── FDM failures ───────────────────────────────────────────────────

#[tokio::test]
async fn with_the_fdm_down_nothing_is_signed() {
    let (app, fdm) = app().await;
    let n = cash_sale(&app, 5200).await;
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();

    let error = refused(&app, n, 0, PaymentMethod::Card).await;

    assert!(error.starts_with("FDM error"), "{error}");
    assert_eq!(event_count(&app), 1);
    assert_eq!(app.counter_for(EventLabel::F), 0);
    // Back up: the same correction goes through.
    app.use_lossy_fdm(&fdm);
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);
}

#[tokio::test]
async fn a_late_answer_is_recovered_when_the_request_is_sent_again() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signPaymentCorrection");
    let app = setup(&fdm).await;
    let n = cash_sale(&app, 5200).await;

    let first = refused(&app, n, 0, PaymentMethod::Card).await;
    assert!(first.contains(errors::FDM_ANSWER_PENDING), "{first}");
    assert!(app.open_pending().is_some());
    assert_eq!(event_count(&app), 1, "not archived yet");

    // The same request again: the FDM returns its first answer and the
    // correction is archived once.
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);
    let sent = fdm.sent_op("signPaymentCorrection");
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].body, sent[1].body, "the same bytes");
    assert_eq!(event_count(&app), 2);
    assert!(app.open_pending().is_none());
    assert_eq!(app.counter_for(EventLabel::F), 1);
    // And it counts as the sale's one correction.
    assert_eq!(
        refused(&app, n, 0, PaymentMethod::MealVoucher).await,
        errors::PAYMENT_CORRECTION_ALREADY_DONE
    );
}

#[tokio::test]
async fn the_background_tick_archives_a_late_answer() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signPaymentCorrection");
    let app = setup(&fdm).await;
    let n = cash_sale(&app, 5200).await;
    refused(&app, n, 0, PaymentMethod::Card).await;

    assert_eq!(close_overdue_day(&app.state).await, Ok(false));

    assert_eq!(event_count(&app), 2);
    assert!(list(&app, n).corrected.is_some());
}

// ── Reports ────────────────────────────────────────────────────────

/// The payments of the turnover a report sent to the FDM.
fn sent_payments(fdm: &LossyFdm, op: &str) -> Vec<Value> {
    let request = fdm
        .sent_op(op)
        .pop()
        .expect("the report was sent")
        .variables()["data"]
        .clone();
    request.get("turnover").cloned().unwrap_or(request)["payments"]
        .as_array()
        .unwrap()
        .clone()
}

fn amounts_of<'a>(payments: &'a [Value], kind: &str) -> &'a Value {
    &payments
        .iter()
        .find(|p| p["type"] == kind)
        .unwrap_or_else(|| panic!("no {kind} in {payments:?}"))["amount"][0]
}

#[tokio::test]
async fn the_x_and_z_totals_show_the_correction_per_method() {
    let (app, fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let n = cash_sale(&app, 5200).await;
    cash_sale(&app, 1000).await;
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);

    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    let payments = sent_payments(&fdm, "signReportTurnoverX");
    let cash = amounts_of(&payments, "CASH");
    assert_eq!(cash["normalAmount"], 62.0);
    assert_eq!(cash["negativeCorrections"], -52.0);
    assert_eq!(cash["positiveCorrections"], 0.0);
    assert_eq!(cash["correctionsCount"], 1);
    assert_eq!(cash["totalAmount"], 10.0);
    // The card exists only through the correction.
    let card = amounts_of(&payments, "CARD_DEBIT");
    assert_eq!(card["normalAmount"], 0.0);
    assert_eq!(card["negativeCorrections"], 0.0);
    assert_eq!(card["positiveCorrections"], 52.0);
    assert_eq!(card["correctionsCount"], 1);
    assert_eq!(card["totalAmount"], 52.0);

    // The paper shows the corrected totals and the corrections line.
    printer.settle(3).await;
    let text = printer
        .rendered()
        .into_iter()
        .find(|t| t.contains("Corrections de paiement"))
        .expect("the corrections line is printed");
    let line = text
        .lines()
        .find(|l| l.contains("Corrections de paiement"))
        .unwrap();
    assert!(line.contains('1') && line.contains("52,00"), "{line}");

    let z = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);
    // The Z travels with the user Z in one document: read the archive.
    let archived = app
        .fiscal_events()
        .into_iter()
        .find(|e| {
            e.event_type == "R"
                && e.event_data_json
                    .as_deref()
                    .unwrap()
                    .contains("\"payments\"")
        })
        .expect("the turnover Z");
    let z: Value = serde_json::from_str(archived.event_data_json.as_deref().unwrap()).unwrap();
    let payments = z.get("turnover").unwrap_or(&z)["payments"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(amounts_of(&payments, "CASH")["totalAmount"], 10.0);
    assert_eq!(amounts_of(&payments, "CARD_DEBIT")["totalAmount"], 52.0);
    assert_eq!(amounts_of(&payments, "CARD_DEBIT")["correctionsCount"], 1);
}

#[tokio::test]
async fn the_user_report_credits_the_manager_who_signed_the_correction() {
    let (app, fdm) = app().await;
    // A cashier sells; the manager corrects.
    #[expect(
        clippy::items_after_statements,
        reason = "a local helper beside the one test that uses it"
    )]
    const CASHIER: &str = "94060242576";
    app.login_as(CASHIER, UserRole::Cashier);
    let n = cash_sale(&app, 5200).await;
    app.login_as(MANAGER, UserRole::Admin);
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);

    let r = do_generate_user_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let request = fdm.sent_op("signReportUserX").pop().unwrap().variables()["data"].clone();
    let users = request
        .get("users")
        .cloned()
        .or_else(|| {
            request["userReport"]["users"]
                .as_array()
                .map(|a| a.clone().into())
        })
        .unwrap_or_else(|| request.clone());
    let users = users
        .as_array()
        .unwrap_or_else(|| panic!("users in {request}"));
    let of = |id: &str| users.iter().find(|u| u["employeeId"] == id).unwrap();
    let cashier = of(CASHIER);
    let manager = of(MANAGER);
    let cashier_cash = amounts_of(cashier["payments"].as_array().unwrap(), "CASH");
    assert_eq!(cashier_cash["normalAmount"], 52.0);
    assert_eq!(cashier_cash["correctionsCount"], 0);
    let manager_payments = manager["payments"].as_array().unwrap();
    assert_eq!(
        amounts_of(manager_payments, "CASH")["negativeCorrections"],
        -52.0
    );
    assert_eq!(
        amounts_of(manager_payments, "CARD_DEBIT")["positiveCorrections"],
        52.0
    );
    assert_eq!(manager["totalAmount"], 0.0);
}

#[tokio::test]
async fn a_training_correction_stays_out_of_the_real_totals() {
    let (app, fdm) = app().await;
    cash_sale(&app, 1000).await;
    app.state.config.lock().unwrap().training_mode = true;
    let t = cash_sale(&app, 5200).await;
    assert_eq!(correct(&app, t, 0, PaymentMethod::Card).await, None);
    app.state.config.lock().unwrap().training_mode = false;

    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    let payments = sent_payments(&fdm, "signReportTurnoverX");
    assert_eq!(payments.len(), 1, "{payments:?}");
    let cash = amounts_of(&payments, "CASH");
    assert_eq!(cash["totalAmount"], 10.0);
    assert_eq!(cash["correctionsCount"], 0);
}

// ── Sanity of the single-line helper used above ────────────────────

#[tokio::test]
async fn orders_with_several_lines_are_corrected_as_one_payment() {
    let (app, _fdm) = app().await;
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: pos_core::order::OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    let item = {
        let db = app.state.db.lock().unwrap();
        db.execute(
            "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO menu_items (category_id, price_cents, vat_code, sort_order, active)
             VALUES (1, 650, 'A', 0, 1)",
            [],
        )
        .unwrap();
        db.last_insert_rowid()
    };
    for quantity in [2, 3] {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: item,
                quantity,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    let sold = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let n = sold.fiscal_event_id.unwrap();
    assert_eq!(correct(&app, n, 0, PaymentMethod::Card).await, None);
    let f = &corrections(&app)[0];
    assert_eq!(f["financials"][1]["amount"], 32.5);
}
