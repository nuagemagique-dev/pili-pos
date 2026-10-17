//! Discounts at the till (manager PIN): an order discount in percent,
//! amount or "Offert", and one free line, as priceChanges inside the
//! sale N and the table order P (POS spec v2.2 `priceChange`, Advanced
//! Set UC223, UC225A/B, UC229B; producers FAQ Q206, Q209, Q303, Q304).
//!
//! - the amounts, the spread to the cent, the rounding;
//! - where a change goes: on the line's only unsent entry, or on a
//!   PRICE_CHANGE pair of journal entries; the immediate P of a table
//!   the FDM already knows;
//! - every refusal, signing nothing and writing nothing;
//! - the bar slip, the cart, the pre-bill, the refund, the transfer, the
//!   card terminal's floor, training, the audit rows, the HubRise echo,
//!   the report's reductions and negQuantities.
//!
//! Events are signed by the admission image's mock FDM.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::discount::{
    DiscountInput, DiscountKind, DiscountReason, DiscountResult, DiscountTarget, do_apply_discount,
    do_remove_discount,
};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_list_cart_lines,
    do_update_order_line_quantity,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::do_generate_x_report;
use pos_engine::commands::table_transfer::do_transfer_table;
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};

const MANAGER: &str = "12345678958";
const WAITER: &str = "84022899837";
const PIN: &str = "4321";

/// Catalogue: 1 Sake 6,50 A (bar), 2 Maki 12,00 B (kitchen), 3 Eau 3,00 A
/// (off the slip), 4 one cent C, 5 3,33 A, 6 3,34 B, 7 1,10 A, 8 1,05 A.
async fn world() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    seed(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

fn seed(app: &TestApp) {
    app.login_as(MANAGER, UserRole::Admin);
    pos_engine::commands::auth::do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 1, 1200, 'B', 1, 1, 1),
                (3, 1, 300, 'A', 0, 2, 1),
                (4, 1, 1, 'C', 0, 3, 1),
                (5, 1, 333, 'A', 0, 4, 1),
                (6, 1, 334, 'B', 0, 5, 1),
                (7, 1, 110, 'A', 0, 6, 1),
                (8, 1, 105, 'A', 0, 7, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Maki saumon'),
                ('menu_item', 3, 'name', 'fr', 'Eau plate');",
        )
        .unwrap();
}

fn order(app: &TestApp, order_type: OrderType, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type,
            table_number: table,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id
}

fn table_order(app: &TestApp, table: i32) -> i64 {
    order(app, OrderType::DineIn, Some(table))
}

fn takeaway(app: &TestApp) -> i64 {
    order(app, OrderType::Takeaway, None)
}

fn add(app: &TestApp, order: i64, item: i64, quantity: i32) -> i64 {
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
    .unwrap()
    .id
}

fn input(kind: DiscountKind, value: Option<i64>) -> DiscountInput {
    DiscountInput {
        target: DiscountTarget::Order,
        kind,
        value,
        reason_code: DiscountReason::CommercialGesture,
        reason_text: None,
    }
}

fn pct(value: i64) -> DiscountInput {
    input(DiscountKind::Percent, Some(value))
}

fn free_line(line_id: i64) -> DiscountInput {
    DiscountInput {
        target: DiscountTarget::Line { line_id },
        kind: DiscountKind::Free,
        value: None,
        reason_code: DiscountReason::StaffMeal,
        reason_text: Some("repas du personnel".into()),
    }
}

async fn apply(app: &TestApp, order: i64, input: DiscountInput) -> Result<DiscountResult, String> {
    do_apply_discount(&app.state, order, input, MANAGER.into(), PIN.into()).await
}

async fn remove(app: &TestApp, order: i64, line: Option<i64>) -> Result<DiscountResult, String> {
    do_remove_discount(&app.state, order, line, MANAGER.into(), PIN.into()).await
}

fn total(app: &TestApp, order: i64) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT total_cents FROM orders WHERE id = ?1",
            [order],
            |r| r.get(0),
        )
        .unwrap()
}

/// `(line_id, entry_id, grouping_id, change_id, scope, amount_cents)` of
/// each recorded price change, oldest first.
fn changes(app: &TestApp, order: i64) -> Vec<(i64, i64, i32, String, String, i64)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT line_id, entry_id, grouping_id, change_id, scope, amount_cents
             FROM order_price_changes WHERE order_id = ?1 ORDER BY id",
        )
        .unwrap();
    stmt.query_map([order], |r| {
        Ok((
            r.get(0)?,
            r.get(1)?,
            r.get(2)?,
            r.get(3)?,
            r.get(4)?,
            r.get(5)?,
        ))
    })
    .unwrap()
    .map(Result::unwrap)
    .collect()
}

/// `(quantity, till_op, sent)` of each journal entry of the line.
fn entries(app: &TestApp, line: i64) -> Vec<(i64, Option<String>, bool)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT quantity, till_op, sent_event_id IS NOT NULL
             FROM order_entries WHERE line_id = ?1 ORDER BY id",
        )
        .unwrap();
    stmt.query_map([line], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn audit_rows(app: &TestApp, order: i64) -> usize {
    let db = app.state.db.lock().unwrap();
    pos_db::queries::till_discounts::list(&db, order)
        .unwrap()
        .len()
}

fn requests(app: &TestApp) -> Vec<Value> {
    app.fiscal_events()
        .iter()
        .filter_map(|e| e.event_data_json.as_deref())
        .map(|j| serde_json::from_str(j).unwrap())
        .collect()
}

fn labels(app: &TestApp) -> Vec<String> {
    app.fiscal_events()
        .into_iter()
        .map(|e| e.event_type)
        .collect()
}

fn cents(v: &Value) -> i64 {
    (v.as_f64().expect("an amount") * 100.0).round() as i64
}

/// The amount of each transaction line of `request`, after its changes.
fn line_totals(request: &Value) -> Vec<i64> {
    request["transaction"]["transactionLines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| cents(&l["lineTotal"]))
        .collect()
}

async fn sell(app: &TestApp, order: i64) -> i64 {
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

// ── Amounts ────────────────────────────────────────────────────────

#[tokio::test]
async fn an_order_discount_is_spread_to_the_cent() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let lines = [add(&app, o, 5, 1), add(&app, o, 5, 1), add(&app, o, 6, 1)];
    assert_eq!(total(&app, o), 1000);

    // 10 % of 10,00 is 1,00: 33 + 33 + 34 (the cent goes to the largest share).
    let done = apply(&app, o, pct(10)).await.unwrap();
    assert_eq!(done.order.total_cents, 900);
    let rows = changes(&app, o);
    let share = |line: i64| rows.iter().find(|r| r.0 == line).unwrap().5;
    assert_eq!(
        [share(lines[0]), share(lines[1]), share(lines[2])],
        [-33, -33, -34]
    );
    assert_eq!(rows.iter().map(|r| r.5).sum::<i64>(), -100);
    // One discount: one groupingId, scope EVENT.
    assert!(rows.iter().all(|r| r.2 == 1 && r.4 == "EVENT"));
}

#[tokio::test]
async fn a_one_cent_line_never_gets_more_than_its_amount() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 4, 1);
    add(&app, o, 2, 1);
    // 99 % of 12,01 = 11,89 (11,8899 rounded).
    let done = apply(&app, o, pct(99)).await.unwrap();
    assert_eq!(done.order.total_cents, 1201 - 1189);
    let rows = changes(&app, o);
    assert_eq!(rows.iter().map(|r| r.5).sum::<i64>(), -1189);
    let tiny = rows.iter().find(|r| r.5 > -2).map(|r| r.5);
    assert!(tiny.is_none_or(|a| a >= -1), "{rows:?}");
}

#[tokio::test]
async fn a_percent_rounds_half_up() {
    let (app, _p) = world().await;
    // 15 % of 1,10 = 0,165 -> 0,17.
    let a = takeaway(&app);
    add(&app, a, 7, 1);
    assert_eq!(apply(&app, a, pct(15)).await.unwrap().order.total_cents, 93);
    // 10 % of 1,05 = 0,105 -> 0,11.
    let b = takeaway(&app);
    add(&app, b, 8, 1);
    assert_eq!(apply(&app, b, pct(10)).await.unwrap().order.total_cents, 94);
    // 99 % of 1,10 leaves a cent.
    let c = takeaway(&app);
    add(&app, c, 7, 1);
    assert_eq!(apply(&app, c, pct(99)).await.unwrap().order.total_cents, 1);
    // A discount that rounds to nothing is no discount.
    let d = takeaway(&app);
    add(&app, d, 4, 1);
    assert_eq!(
        apply(&app, d, pct(10)).await.unwrap_err(),
        errors::DISCOUNT_INVALID
    );
}

#[tokio::test]
async fn an_amount_goes_up_to_the_net_total() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 1, 2);
    let too_much = apply(&app, o, input(DiscountKind::Amount, Some(1301))).await;
    assert_eq!(too_much.unwrap_err(), errors::DISCOUNT_INVALID);
    let zero = apply(&app, o, input(DiscountKind::Amount, Some(0))).await;
    assert_eq!(zero.unwrap_err(), errors::DISCOUNT_INVALID);
    assert_eq!(total(&app, o), 1300);

    let done = apply(&app, o, input(DiscountKind::Amount, Some(500))).await;
    assert_eq!(done.unwrap().order.total_cents, 800);
    let rows = changes(&app, o);
    assert_eq!(rows[0].3, "TILL-DISCOUNT");
    assert_eq!(rows[0].5, -500);

    // The whole total is allowed.
    let all = takeaway(&app);
    add(&app, all, 1, 2);
    let done = apply(&app, all, input(DiscountKind::Amount, Some(1300))).await;
    assert_eq!(done.unwrap().order.total_cents, 0);
}

#[tokio::test]
async fn offert_is_signed_at_zero_and_printed() {
    let (app, printer) = world().await;
    let o = takeaway(&app);
    add(&app, o, 1, 2);
    add(&app, o, 2, 1);
    let done = apply(&app, o, input(DiscountKind::Free, None))
        .await
        .unwrap();
    assert_eq!(done.order.total_cents, 0);
    assert!(changes(&app, o).iter().all(|c| c.3 == "TILL-FREE"));

    let event = sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 0);
    assert_eq!(line_totals(&sale), vec![0, 0]);
    // FAQ Q206 b: no money moved, so `financials` is the empty array
    // (the other form it allows is a `PAY 0` cash line). The N event
    // and its VAT ticket exist all the same.
    assert_eq!(sale["financials"], serde_json::json!([]));
    assert_ne!(sale["ticketMedium"], "NONE");
    assert!(event > 0);
    let tickets = printer.settle(1).await;
    assert!(!tickets.is_empty());
    let text = printer.rendered().join("\n");
    assert!(text.contains("Offert"), "{text}");
}

#[tokio::test]
async fn a_free_line_comes_to_zero_on_its_own_line_only() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 2, 2);
    let eau = add(&app, o, 3, 1);
    let done = apply(&app, o, free_line(eau)).await.unwrap();
    assert_eq!(done.order.total_cents, 2400);
    let rows = changes(&app, o);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].3.as_str(), rows[0].4.as_str()),
        ("TILL-FREE-LINE", "LINE")
    );
    assert_eq!(rows[0].5, -300);

    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(line_totals(&sale), vec![2400, 0]);
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 2400);
}

#[tokio::test]
async fn a_free_line_is_left_out_of_an_order_discount() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 2, 1);
    let eau = add(&app, o, 3, 1);
    apply(&app, o, free_line(eau)).await.unwrap();
    // 50 % of what is left (12,00).
    let done = apply(&app, o, pct(50)).await.unwrap();
    assert_eq!(done.order.total_cents, 600);
    let rows = changes(&app, o);
    assert_eq!(rows.iter().filter(|r| r.0 == eau).count(), 1, "{rows:?}");
}

#[tokio::test]
async fn names_follow_the_receipt_language() {
    let (app, _p) = world().await;
    app.state.config.lock().unwrap().language = "nl".into();
    let o = takeaway(&app);
    add(&app, o, 1, 1);
    apply(&app, o, pct(10)).await.unwrap();
    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    let change =
        &sale["transaction"]["transactionLines"][0]["mainProduct"]["vats"][0]["priceChanges"][0];
    assert_eq!(change["name"], "Korting 10 %");
    assert_eq!(change["id"], "TILL-DISCOUNT");
    assert_eq!(change["type"], "PUBLIC");
    assert_eq!(change["scope"], "EVENT");
}

// ── Refusals ───────────────────────────────────────────────────────

#[tokio::test]
async fn every_refusal_signs_nothing_and_writes_nothing() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 2);
    let before = (
        total(&app, o),
        changes(&app, o).len(),
        app.fiscal_events().len(),
    );

    async fn refused(app: &TestApp, o: i64, input: DiscountInput) -> String {
        apply(app, o, input).await.expect_err("refused")
    }
    // The shape.
    for bad in [None, Some(0), Some(100), Some(-3)] {
        assert_eq!(
            refused(&app, o, input(DiscountKind::Percent, bad)).await,
            errors::DISCOUNT_INVALID
        );
    }
    let mut on_line = pct(10);
    on_line.target = DiscountTarget::Line { line_id: sake };
    assert_eq!(refused(&app, o, on_line).await, errors::DISCOUNT_INVALID);
    let mut long = pct(10);
    long.reason_text = Some("x".repeat(500));
    assert_eq!(
        refused(&app, o, long).await,
        errors::DISCOUNT_REASON_INVALID
    );
    // A line that is not on the order.
    assert_eq!(
        refused(&app, o, free_line(sake + 999)).await,
        errors::LINE_NOT_FOUND
    );
    // Nothing to remove.
    assert_eq!(
        remove(&app, o, None).await.unwrap_err(),
        errors::DISCOUNT_NONE
    );
    assert_eq!(
        remove(&app, o, Some(sake)).await.unwrap_err(),
        errors::DISCOUNT_NONE
    );
    assert_eq!(
        (
            total(&app, o),
            changes(&app, o).len(),
            app.fiscal_events().len()
        ),
        before
    );
    assert_eq!(audit_rows(&app, o), 0);

    // An order without a line.
    let empty = takeaway(&app);
    assert_eq!(refused(&app, empty, pct(10)).await, errors::ORDER_NO_LINES);
    // An online order has its own discounts.
    let online = takeaway(&app);
    add(&app, online, 1, 1);
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE orders SET hubrise_order_id = 'ONLINE-1' WHERE id = ?1",
            [online],
        )
        .unwrap();
    assert_eq!(
        refused(&app, online, pct(10)).await,
        errors::DISCOUNT_ONLINE_ORDER
    );
    // A closed order.
    sell(&app, o).await;
    assert_eq!(refused(&app, o, pct(10)).await, errors::ORDER_NOT_OPEN);
}

#[tokio::test]
async fn only_a_manager_with_the_right_pin_gives_a_discount() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 1, 1);

    let wrong = do_apply_discount(&app.state, o, pct(10), MANAGER.into(), "0000".into()).await;
    assert_eq!(wrong.unwrap_err(), errors::PIN_WRONG);
    // Someone else's INSZ.
    let other = do_apply_discount(&app.state, o, pct(10), WAITER.into(), PIN.into()).await;
    assert_eq!(other.unwrap_err(), errors::NOT_THE_OPERATOR);
    for role in [UserRole::Waiter, UserRole::Cashier] {
        app.login_as(WAITER, role);
        let r = do_apply_discount(&app.state, o, pct(10), WAITER.into(), PIN.into()).await;
        assert_eq!(r.unwrap_err(), errors::FORBIDDEN, "{role:?}");
        let r = do_remove_discount(&app.state, o, None, WAITER.into(), PIN.into()).await;
        assert_eq!(r.unwrap_err(), errors::FORBIDDEN, "{role:?}");
    }
    app.logout();
    let r = do_apply_discount(&app.state, o, pct(10), MANAGER.into(), PIN.into()).await;
    assert_eq!(r.unwrap_err(), errors::NOT_LOGGED_IN);

    assert_eq!(changes(&app, o).len(), 0);
    assert!(app.fiscal_events().is_empty());
    assert_eq!(audit_rows(&app, o), 0);
}

#[tokio::test]
async fn one_discount_at_a_time() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 1);
    let maki = add(&app, o, 2, 1);
    apply(&app, o, pct(10)).await.unwrap();
    let rows = changes(&app, o).len();
    // A second order discount, and a free line under it, wait for the removal.
    assert_eq!(
        apply(&app, o, pct(20)).await.unwrap_err(),
        errors::DISCOUNT_ALREADY_APPLIED
    );
    assert_eq!(
        apply(&app, o, free_line(maki)).await.unwrap_err(),
        errors::DISCOUNT_ALREADY_APPLIED
    );
    assert_eq!(changes(&app, o).len(), rows);

    remove(&app, o, None).await.unwrap();
    assert_eq!(total(&app, o), 1850);
    apply(&app, o, free_line(sake)).await.unwrap();
    // A line that already carries a change is not offered twice.
    assert_eq!(
        apply(&app, o, free_line(sake)).await.unwrap_err(),
        errors::DISCOUNT_ALREADY_APPLIED
    );
}

#[tokio::test]
async fn a_discounted_line_keeps_its_quantity() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 2);
    apply(&app, o, pct(10)).await.unwrap();
    let err = do_update_order_line_quantity(&app.state, sake, 3).unwrap_err();
    assert_eq!(err, errors::LINE_HAS_PRICE_CHANGES);
}

#[tokio::test]
async fn an_answer_still_awaited_stops_the_discount() {
    let fdm = LossyFdm::start().await;
    let app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    seed(&app);
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    // The registration has no answer, and neither has the next try.
    fdm.rule(|op, _| {
        if op == "signOrder" {
            pos_engine_testkit::Fate::LoseAnswer
        } else {
            pos_engine_testkit::Fate::Answer
        }
    });
    assert!(do_sign_table_order(&app.state, o).await.is_err());
    let err = apply(&app, o, pct(10)).await.unwrap_err();
    assert!(err.contains(errors::FDM_ANSWER_PENDING), "{err}");
    assert_eq!(changes(&app, o).len(), 0);
    assert_eq!(audit_rows(&app, o), 0);
}

#[tokio::test]
async fn a_lost_answer_keeps_the_discount_and_the_next_command_registers_it() {
    let fdm = LossyFdm::start().await;
    let app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    seed(&app);
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    // The immediate P of the discount gets no answer.
    let lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    fdm.rule(move |op, _| {
        if op == "signOrder" && !lost.swap(true, std::sync::atomic::Ordering::SeqCst) {
            Fate::LoseAnswer
        } else {
            Fate::Answer
        }
    });
    let done = apply(&app, o, pct(50)).await.unwrap();
    // The discount is recorded and told apart from a refusal.
    let warning = done.registration_error.expect("the registration waits");
    assert!(warning.contains(errors::FDM_ANSWER_PENDING), "{warning}");
    assert_eq!(done.order.total_cents, 650);
    assert_eq!(audit_rows(&app, o), 1);
    assert!(app.open_pending().is_some());

    // The same bytes again: the FDM returns its first answer and the P
    // is archived once; nothing is left to send.
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(labels(&app), vec!["P", "P"]);
    assert!(app.open_pending().is_none());
    let sent = fdm.sent_op("signOrder");
    assert_eq!(sent.len(), 3);
    assert_eq!(sent[1].body, sent[2].body, "the same bytes");
}

// ── Where a change goes ────────────────────────────────────────────

#[tokio::test]
async fn an_unsent_line_takes_the_change_on_its_only_entry() {
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    let sake = add(&app, o, 1, 2);
    let done = apply(&app, o, pct(50)).await.unwrap();
    assert_eq!(done.order.total_cents, 650);
    // No new entry, no event: the table was never registered.
    assert_eq!(entries(&app, sake), vec![(2, None, false)]);
    assert!(app.fiscal_events().is_empty());
    assert_eq!(done.kitchen_slip, SlipOutcome::NothingToPrint);
    assert!(done.registration_error.is_none());
    assert_eq!(changes(&app, o).len(), 1);
}

#[tokio::test]
async fn a_registered_line_gets_a_price_change_pair_and_a_p_at_once() {
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    let sake = add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(labels(&app), vec!["P"]);

    let done = apply(&app, o, pct(50)).await.unwrap();
    assert_eq!(done.order.total_cents, 650);
    assert!(done.registration_error.is_none());
    // The original entry, then the correction and the re-entry.
    assert_eq!(
        entries(&app, sake),
        vec![
            (2, None, true),
            (-2, Some("price_change".into()), true),
            (2, Some("price_change".into()), true),
        ]
    );
    // The correction carries nothing (no earlier change), the re-entry
    // the new one.
    let rows = changes(&app, o);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].5, -650);

    // The P the discount signed: -2 with PRICE_CHANGE, +2 with the change.
    assert_eq!(labels(&app), vec!["P", "P"]);
    let p = &requests(&app)[1];
    let lines = p["transaction"]["transactionLines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["mainProduct"]["quantity"], -2.0);
    assert_eq!(lines[0]["mainProduct"]["negQuantityReason"], "PRICE_CHANGE");
    assert!(
        lines[0]["mainProduct"]["vats"][0]
            .get("priceChanges")
            .is_none()
    );
    assert_eq!(lines[1]["mainProduct"]["quantity"], 2.0);
    assert_eq!(
        lines[1]["mainProduct"]["vats"][0]["priceChanges"][0]["amount"],
        -6.5
    );
    assert_eq!(line_totals(p), vec![-1300, 650]);
    assert_eq!(cents(&p["transaction"]["transactionTotal"]), -650);
}

#[tokio::test]
async fn the_immediate_p_follows_the_table_only() {
    let (app, _p) = world().await;
    // A table not yet registered: no event.
    let fresh = table_order(&app, 1);
    add(&app, fresh, 1, 1);
    apply(&app, fresh, pct(10)).await.unwrap();
    assert!(app.fiscal_events().is_empty());
    // A takeaway has no P, whatever happened before.
    let away = takeaway(&app);
    add(&app, away, 1, 1);
    apply(&app, away, pct(10)).await.unwrap();
    remove(&app, away, None).await.unwrap();
    assert!(app.fiscal_events().is_empty());
    // A registered table does.
    let known = table_order(&app, 2);
    add(&app, known, 2, 1);
    do_sign_table_order(&app.state, known).await.unwrap();
    apply(&app, known, pct(10)).await.unwrap();
    assert_eq!(labels(&app), vec!["P", "P"]);
}

#[tokio::test]
async fn other_unsent_entries_are_registered_with_the_change_and_printed() {
    let (app, printer) = world().await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    // Added after the registration, then the discount.
    add(&app, o, 2, 1);
    let done = apply(&app, o, pct(10)).await.unwrap();
    assert_eq!(done.kitchen_slip, SlipOutcome::Printed);

    let slips = printer.settle_slips(2).await;
    assert!(slips[1].contains("1 x Maki saumon"), "{}", slips[1]);
    // The PRICE_CHANGE pair orders nothing.
    assert!(!slips[1].contains("Sake"), "{}", slips[1]);
    assert!(!slips[1].contains("-1 x"), "{}", slips[1]);
}

#[tokio::test]
async fn a_pair_alone_prints_nothing_and_leaves_the_cart_quantity_alone() {
    let (app, printer) = world().await;
    let o = table_order(&app, 4);
    let sake = add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    let done = apply(&app, o, pct(50)).await.unwrap();
    assert_eq!(done.kitchen_slip, SlipOutcome::NothingToPrint);
    assert_eq!(printer.settle_slips(1).await.len(), 1, "no second slip");

    let cart = do_list_cart_lines(&app.state, o).unwrap();
    assert_eq!(cart.len(), 1);
    assert_eq!(cart[0].line.id, sake);
    assert_eq!(cart[0].unsent_quantity, 0);
    assert_eq!(cart[0].discount_cents, -650);
    assert_eq!(cart[0].price_changes.len(), 1);
    assert_eq!(cart[0].price_changes[0].name, "Remise 50 %");
    assert_eq!(cart[0].price_changes[0].scope, "EVENT");
    assert_eq!(cart[0].price_changes[0].amount_cents, -650);
}

#[tokio::test]
async fn a_removal_is_the_same_pair_the_other_way_and_the_grouping_moves_on() {
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    let sake = add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    apply(&app, o, pct(50)).await.unwrap();

    let done = remove(&app, o, None).await.unwrap();
    assert_eq!(done.order.total_cents, 1300);
    assert_eq!(entries(&app, sake).len(), 5);
    let rows = changes(&app, o);
    // -6,50 on the re-entry, then +6,50 on the next correction.
    assert_eq!(
        rows.iter().map(|r| (r.2, r.5)).collect::<Vec<_>>(),
        vec![(1, -650), (1, 650)]
    );
    assert_eq!(labels(&app), vec!["P", "P", "P"]);
    let p = &requests(&app)[2];
    let lines = p["transaction"]["transactionLines"].as_array().unwrap();
    assert_eq!(lines[0]["mainProduct"]["negQuantityReason"], "PRICE_CHANGE");
    assert_eq!(
        lines[0]["mainProduct"]["vats"][0]["priceChanges"][0]["amount"],
        6.5
    );
    assert_eq!(line_totals(p), vec![-650, 1300]);

    // A new discount takes the next groupingId (FAQ Q304 b).
    apply(&app, o, pct(20)).await.unwrap();
    let rows = changes(&app, o);
    assert_eq!(rows.last().unwrap().2, 2);
    assert_eq!(total(&app, o), 1040);

    // And the sale carries the whole journal.
    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 1040);
    let reasons: Vec<_> = sale["transaction"]["transactionLines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["mainProduct"]["quantity"].as_f64().unwrap() < 0.0)
        .map(|l| l["mainProduct"]["negQuantityReason"].clone())
        .collect();
    assert_eq!(reasons, vec!["PRICE_CHANGE"; 3]);
}

#[tokio::test]
async fn a_direct_sale_with_a_removed_discount_marks_its_corrections() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 1, 1);
    apply(&app, o, pct(10)).await.unwrap();
    remove(&app, o, None).await.unwrap();
    assert_eq!(total(&app, o), 650);
    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    let lines = sale["transaction"]["transactionLines"].as_array().unwrap();
    // +1 with the change, -1 PRICE_CHANGE carrying it back, +1 clean.
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[1]["mainProduct"]["negQuantityReason"], "PRICE_CHANGE");
    // The correction carries the change back (+0,65): -6,50 + 0,65.
    assert_eq!(line_totals(&sale), vec![585, -585, 650]);
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 650);
}

// ── Around the sale ────────────────────────────────────────────────

#[tokio::test]
async fn the_pre_bill_shows_the_net_amounts() {
    let (app, printer) = world().await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    apply(&app, o, pct(50)).await.unwrap();

    do_sign_pre_bill(&app.state, o).await.unwrap();
    let pre_bill = requests(&app).pop().unwrap();
    assert_eq!(labels(&app), vec!["P", "P"]);
    assert_eq!(cents(&pre_bill["transaction"]["transactionTotal"]), 650);
    assert_eq!(
        pre_bill["transaction"]["transactionLines"][0]["mainProduct"]["vats"][0]["priceChanges"][0]
            ["amount"],
        -6.5
    );
    printer.settle(1).await;
    let rendered = printer.rendered().join("\n");
    assert!(rendered.contains("Remise 50 %"), "{rendered}");
}

#[tokio::test]
async fn a_discounted_order_moves_with_its_changes() {
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    apply(&app, o, pct(50)).await.unwrap();

    do_transfer_table(&app.state, o, 8).await.unwrap();
    let transfer = requests(&app).pop().unwrap();
    let side = |name: &str| {
        let line = &transfer["transfer"][name][0]["transaction"]["transactionLines"][0];
        let change = &line["mainProduct"]["vats"][0]["priceChanges"][0];
        (
            cents(&change["amount"]),
            change["groupingId"].clone(),
            cents(&line["lineTotal"]),
        )
    };
    // Off the old table: the change negated, same groupingId.
    assert_eq!(side("from"), (650, serde_json::json!(1), -650));
    assert_eq!(side("to"), (-650, serde_json::json!(1), 650));

    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(sale["costCenter"]["id"], "T08");
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 650);
}

#[tokio::test]
async fn a_discounted_sale_is_refunded_whole_at_its_net_amount() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 1, 2);
    add(&app, o, 2, 1);
    apply(&app, o, pct(20)).await.unwrap();
    assert_eq!(total(&app, o), 2000);
    let event = sell(&app, o).await;

    // Part of a discounted line is refused.
    let partial = do_refund_checkout(
        &app.state,
        event,
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await;
    let err = partial.expect_err("a discounted line is refunded whole");
    assert!(
        err.starts_with(errors::hubrise::REFUND_DISCOUNTED_LINE),
        "{err}"
    );

    // The whole order gives back what was paid.
    let whole = do_refund_checkout(
        &app.state,
        event,
        vec![
            RefundItem {
                line_id: None,
                menu_item_id: 1,
                quantity: 2,
            },
            RefundItem {
                line_id: None,
                menu_item_id: 2,
                quantity: 1,
            },
        ],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(whole.success, "{:?}", whole.error);
    let refund = requests(&app).pop().unwrap();
    assert_eq!(cents(&refund["transaction"]["transactionTotal"]), -2000);
}

#[tokio::test]
async fn a_card_already_charged_bounds_the_discount() {
    use pos_eft::CardType;
    use pos_engine::commands::eft::do_eft_pay;
    let (mut app, _p) = {
        let mut app = TestApp::new().await;
        app.use_admission_mock().await;
        seed(&app);
        let printer = PrinterSink::start().await;
        app.set_printer(&printer.addr);
        let _ = &mut app;
        (app, printer)
    };
    let script = app.link_terminal();
    let o = takeaway(&app);
    add(&app, o, 2, 2);
    script.on_pay(Ok(approval(2400, "VISA", CardType::Debit)));
    do_eft_pay(&app.state, o, 2400).await.unwrap();

    let err = apply(&app, o, pct(50)).await.unwrap_err();
    assert!(err.starts_with(errors::EFT_BELOW_CHARGED), "{err}");
    assert_eq!(total(&app, o), 2400);
    assert_eq!(changes(&app, o).len(), 0);
    assert_eq!(audit_rows(&app, o), 0);
}

#[tokio::test]
async fn training_signs_t_events() {
    let (app, _p) = world().await;
    app.state.config.lock().unwrap().training_mode = true;
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    apply(&app, o, pct(50)).await.unwrap();
    sell(&app, o).await;
    assert_eq!(labels(&app), vec!["T", "T", "T"]);
    let sale = requests(&app).pop().unwrap();
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 650);
}

#[tokio::test]
async fn the_audit_keeps_who_why_and_how_much() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 2);
    let mut staff = pct(20);
    staff.reason_code = DiscountReason::StaffMeal;
    staff.reason_text = Some("  repas  ".into());
    apply(&app, o, staff).await.unwrap();
    remove(&app, o, None).await.unwrap();
    apply(&app, o, free_line(sake)).await.unwrap();

    let rows = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::till_discounts::list(&db, o).unwrap()
    };
    assert_eq!(rows.len(), 3);
    let (apply_row, remove_row, line_row) = (&rows[0], &rows[1], &rows[2]);
    assert_eq!(apply_row.action, "apply");
    assert_eq!(apply_row.kind.as_deref(), Some("percent"));
    assert_eq!(apply_row.value, Some(20));
    assert_eq!(apply_row.amount_cents, -260);
    assert_eq!(apply_row.change_id, "TILL-DISCOUNT");
    assert_eq!(apply_row.reason_code, "staff_meal");
    assert_eq!(apply_row.reason_text.as_deref(), Some("repas"));
    assert_eq!(apply_row.manager_insz, MANAGER);
    assert_eq!(apply_row.line_id, None);
    assert_eq!(apply_row.grouping_id, 1);

    assert_eq!(remove_row.action, "remove");
    assert_eq!(remove_row.kind, None);
    assert_eq!(remove_row.amount_cents, 260);

    assert_eq!(line_row.line_id, Some(sake));
    assert_eq!(line_row.kind.as_deref(), Some("free"));
    assert_eq!(line_row.change_id, "TILL-FREE-LINE");
    assert_eq!(line_row.grouping_id, 2);
    // The reason is local: it is nowhere in a signed request.
    sell(&app, o).await;
    let sale = serde_json::to_string(&requests(&app).pop().unwrap()).unwrap();
    assert!(!sale.contains("staff_meal") && !sale.contains("repas"));
}

#[tokio::test]
async fn the_echo_to_hubrise_carries_the_discount() {
    let (app, _p) = world().await;
    app.state.hubrise_switch.set(true);
    let o = takeaway(&app);
    add(&app, o, 2, 1);
    apply(&app, o, pct(25)).await.unwrap();
    sell(&app, o).await;

    let body: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT body_json FROM hubrise_outbox WHERE kind = 'create_order'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["discounts"][0]["name"], "Remise 25 %");
    assert_eq!(body["discounts"][0]["price_off"], "3.00 EUR");
    assert_eq!(body["total"], "9.00 EUR");
    assert_eq!(body["payments"][0]["amount"], "9.00 EUR");
}

// ── Reports ────────────────────────────────────────────────────────

async fn x_report(app: &TestApp) -> Value {
    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    app.fiscal_events()
        .into_iter()
        .rev()
        .find(|e| e.event_type == "R")
        .and_then(|e| e.event_data_json)
        .map(|j| serde_json::from_str(&j).unwrap())
        .expect("the report")
}

#[tokio::test]
async fn the_printed_report_splits_the_reductions_by_scope() {
    let (app, printer) = world().await;
    // A ticket discount on one sale, a free line on another.
    let a = takeaway(&app);
    add(&app, a, 2, 1);
    apply(&app, a, pct(50)).await.unwrap();
    sell(&app, a).await;
    let b = takeaway(&app);
    add(&app, b, 2, 1);
    let eau = add(&app, b, 3, 1);
    apply(&app, b, free_line(eau)).await.unwrap();
    sell(&app, b).await;

    let before = printer.receipts().len();
    let report = x_report(&app).await;
    // The signed payload is not split: both changes are in it.
    let signed = report["turnover"]["priceChanges"].as_array().unwrap();
    let ids: Vec<_> = signed.iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"TILL-DISCOUNT") && ids.contains(&"TILL-FREE-LINE"));

    printer.settle(before + 1).await;
    let text = printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("CHIFFRE D'AFFAIRES"))
        .expect("the turnover report");
    let row = |start: &str| {
        text.lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .find(|l| l.starts_with(start))
            .unwrap_or_else(|| panic!("no {start:?} row in:\n{text}"))
    };
    assert!(row("Reductions de lignes").starts_with("Reductions de lignes 1"));
    assert!(
        row("Reductions de lignes").contains("-3,00"),
        "{}",
        row("Reductions de lignes")
    );
    assert!(row("Reductions d'evenements").starts_with("Reductions d'evenements 1"));
    assert!(row("Reductions d'evenements").contains("-6,00"));
}

#[tokio::test]
async fn price_change_corrections_are_counted_once_under_their_reason() {
    let (app, _p) = world().await;
    // Registered by a P, corrected by the discount's P, then sold: the
    // sale repeats the pair, which must not count a second time.
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    apply(&app, o, pct(50)).await.unwrap();
    sell(&app, o).await;

    let report = x_report(&app).await;
    let neg = report["turnover"]["negQuantities"].as_array().unwrap();
    let price_change = neg
        .iter()
        .find(|n| n["negQuantityReason"] == "PRICE_CHANGE")
        .expect("PRICE_CHANGE in negQuantities");
    assert_eq!(price_change["negQuantityCount"], 1);
    assert_eq!(price_change["ticketCount"], 1);
    assert_eq!(cents(&price_change["amount"]), -1300);
    assert!(
        neg.iter().all(|n| n["negQuantityReason"] != "CORRECTION"),
        "{neg:?}"
    );
}

#[tokio::test]
async fn a_direct_sale_counts_its_price_change_corrections() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 1, 1);
    apply(&app, o, pct(10)).await.unwrap();
    remove(&app, o, None).await.unwrap();
    sell(&app, o).await;

    let report = x_report(&app).await;
    let neg = report["turnover"]["negQuantities"].as_array().unwrap();
    let price_change = neg
        .iter()
        .find(|n| n["negQuantityReason"] == "PRICE_CHANGE")
        .expect("PRICE_CHANGE in negQuantities");
    assert_eq!(price_change["negQuantityCount"], 1);
    assert_eq!(price_change["ticketCount"], 1);
}

// ── Removing the offer of one line ─────────────────────────────────

#[tokio::test]
async fn removing_an_offered_line_gives_it_back_its_full_price() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    add(&app, o, 2, 1);
    let eau = add(&app, o, 3, 1);
    apply(&app, o, free_line(eau)).await.unwrap();
    assert_eq!(total(&app, o), 1200);

    let done = remove(&app, o, Some(eau)).await.unwrap();

    assert_eq!(done.order.total_cents, 1500);
    assert_eq!(total(&app, o), 1500);
    // Only TILL-FREE-LINE is removed: -300 first, +300 on the correction.
    let rows = changes(&app, o);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_eq!((rows[0].3.as_str(), rows[0].5), ("TILL-FREE-LINE", -300));
    assert_eq!(
        (rows[1].3.as_str(), rows[1].4.as_str(), rows[1].5),
        ("TILL-FREE-LINE", "LINE", 300)
    );
    assert_ne!(
        rows[0].1, rows[1].1,
        "the opposite change is on another entry"
    );
    assert_eq!(
        entries(&app, eau),
        vec![
            (1, None, false),
            (-1, Some("price_change".into()), false),
            (1, Some("price_change".into()), false),
        ]
    );
    // The audit row names the line and the amount given back.
    let audit = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::till_discounts::list(&db, o).unwrap()
    };
    assert_eq!(audit.len(), 2);
    assert_eq!(audit[1].action, "remove");
    assert_eq!(audit[1].line_id, Some(eau));
    assert_eq!(audit[1].amount_cents, 300);

    // The sale: Maki, then Eau with its change, the PRICE_CHANGE
    // correction carrying it back, and Eau at its full price.
    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(line_totals(&sale), vec![1200, 0, 0, 300]);
    let lines = sale["transaction"]["transactionLines"].as_array().unwrap();
    assert_eq!(lines[2]["mainProduct"]["quantity"], -1.0);
    assert_eq!(lines[2]["mainProduct"]["negQuantityReason"], "PRICE_CHANGE");
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 1500);
}

#[tokio::test]
async fn removing_an_offered_line_on_a_registered_table_signs_the_p_at_once() {
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    add(&app, o, 2, 1);
    let eau = add(&app, o, 3, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    apply(&app, o, free_line(eau)).await.unwrap();
    assert_eq!(labels(&app), vec!["P", "P"]);

    let done = remove(&app, o, Some(eau)).await.unwrap();

    assert_eq!(done.order.total_cents, 1500);
    assert!(done.registration_error.is_none());
    assert_eq!(labels(&app), vec!["P", "P", "P"]);
    let p = &requests(&app)[2];
    let lines = p["transaction"]["transactionLines"].as_array().unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["mainProduct"]["quantity"], -1.0);
    assert_eq!(lines[0]["mainProduct"]["negQuantityReason"], "PRICE_CHANGE");
    assert_eq!(lines[1]["mainProduct"]["quantity"], 1.0);
    assert!(
        lines[1]["mainProduct"]["vats"][0]
            .get("priceChanges")
            .is_none()
    );
    assert_eq!(line_totals(p), vec![0, 300]);
    assert_eq!(cents(&p["transaction"]["transactionTotal"]), 300);
}

// ── A free line on a line a P already registered ───────────────────

#[tokio::test]
async fn a_free_line_on_a_registered_line_goes_through_a_price_change_pair() {
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    let eau = add(&app, o, 3, 1);
    do_sign_table_order(&app.state, o).await.unwrap();

    let done = apply(&app, o, free_line(eau)).await.unwrap();
    assert_eq!(done.order.total_cents, 1300);
    assert_eq!(labels(&app), vec!["P", "P"]);

    let p = &requests(&app)[1];
    let lines = p["transaction"]["transactionLines"].as_array().unwrap();
    assert_eq!(lines.len(), 2, "only the Eau is corrected");
    assert_eq!(lines[0]["mainProduct"]["quantity"], -1.0);
    assert_eq!(lines[0]["mainProduct"]["productName"], "Eau plate");
    assert_eq!(lines[0]["mainProduct"]["negQuantityReason"], "PRICE_CHANGE");
    assert_eq!(lines[1]["mainProduct"]["quantity"], 1.0);
    let change = &lines[1]["mainProduct"]["vats"][0]["priceChanges"][0];
    assert_eq!(change["scope"], "LINE");
    assert_eq!(change["amount"], -3.0);
    assert_eq!(cents(&lines[1]["lineTotal"]), 0);
    assert_eq!(line_totals(p), vec![-300, 0]);

    // The sale carries the journal: 2 Sake, the Eau, its pair.
    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 1300);
}

// ── A discounted line removed from the cart ────────────────────────

#[tokio::test]
async fn a_discounted_line_removed_from_the_cart_carries_its_change_negated() {
    use pos_engine::commands::order::do_remove_order_line;
    let (app, _p) = world().await;
    let o = table_order(&app, 4);
    let sake = add(&app, o, 1, 2);
    let maki = add(&app, o, 2, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    apply(&app, o, pct(50)).await.unwrap();
    assert_eq!(total(&app, o), 1250);
    let changes_before = changes(&app, o).len();

    do_remove_order_line(&app.state, sake).unwrap();

    // Maki is left at 50 %: 6,00.
    assert_eq!(total(&app, o), 600);
    let rows = changes(&app, o);
    assert!(rows.len() > changes_before, "{rows:?}");
    let negated = rows
        .iter()
        .find(|r| r.0 == sake && r.5 > 0)
        .expect("the removed line's change comes back negated");
    assert_eq!(negated.3, "TILL-DISCOUNT");
    assert_eq!(negated.5, 650);
    // The other line keeps its own discount.
    assert_eq!(
        rows.iter()
            .filter(|r| r.0 == maki)
            .map(|r| r.5)
            .sum::<i64>(),
        -600
    );

    sell(&app, o).await;
    let sale = requests(&app).pop().unwrap();
    assert_eq!(cents(&sale["transaction"]["transactionTotal"]), 600);
    let lines = sale["transaction"]["transactionLines"].as_array().unwrap();
    let removed: Vec<_> = lines
        .iter()
        .filter(|l| l["mainProduct"]["productName"] == "Sake")
        .collect();
    // The last Sake entry is the removal itself (the earlier -q is the
    // PRICE_CHANGE pair of the discount).
    let correction = removed
        .iter()
        .rev()
        .find(|l| l["mainProduct"]["quantity"].as_f64().unwrap() < 0.0)
        .expect("the removed line's -q correction");
    assert_eq!(correction["mainProduct"]["quantity"], -2.0);
    assert_eq!(correction["mainProduct"]["negQuantityReason"], "CORRECTION");
    assert_eq!(cents(&correction["lineTotal"]), -650);
    assert_eq!(
        correction["mainProduct"]["vats"][0]["priceChanges"][0]["id"],
        "TILL-DISCOUNT"
    );
    assert_eq!(
        correction["mainProduct"]["vats"][0]["priceChanges"][0]["amount"],
        6.5
    );
    // The Sake lines net to nothing; only the Maki at 50 % is sold.
    let net: i64 = removed.iter().map(|l| cents(&l["lineTotal"])).sum();
    assert_eq!(net, 0);
}

// ── The pre-bill is reset by a discount ────────────────────────────

fn pre_bill_printed(app: &TestApp, table: i32) -> bool {
    pos_engine::commands::tables::do_list_tables(&app.state, 25)
        .unwrap()
        .into_iter()
        .find(|t| t.table_number == table)
        .unwrap()
        .pre_bill_printed
}

#[tokio::test]
async fn a_discount_resets_the_pre_bill_flag() {
    let (app, _p) = world().await;
    // Signing the pre-bill first registers every unsent entry as a P ORDER
    // (sign_table_order_locked), so the line IS registered when the discount
    // arrives: it is a PRICE_CHANGE pair plus one immediate P.
    let fresh = table_order(&app, 1);
    add(&app, fresh, 1, 2);
    do_sign_pre_bill(&app.state, fresh).await.unwrap();
    assert!(pre_bill_printed(&app, 1));
    let before = app.fiscal_events().len();
    apply(&app, fresh, pct(10)).await.unwrap();
    let events = app.fiscal_events();
    assert_eq!(
        events.len(),
        before + 1,
        "the discount on a line registered by the pre-bill signs one immediate P"
    );
    assert_eq!(events.last().unwrap().event_type, "P");
    assert!(!pre_bill_printed(&app, 1), "apply after the pre-bill");

    // A registered table: apply, a new bill, then remove.
    let known = table_order(&app, 2);
    add(&app, known, 1, 2);
    do_sign_table_order(&app.state, known).await.unwrap();
    apply(&app, known, pct(10)).await.unwrap();
    do_sign_pre_bill(&app.state, known).await.unwrap();
    assert!(pre_bill_printed(&app, 2));
    remove(&app, known, None).await.unwrap();
    assert!(!pre_bill_printed(&app, 2), "remove on a registered line");
}
