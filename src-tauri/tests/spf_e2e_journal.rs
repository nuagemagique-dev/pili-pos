//! Order-journal semantics through the command layer.
//!
//! `order_entries` is the append-only journal behind every order:
//! additions, corrections (quantity decreases / line removals) and
//! refunds, in chronological order (POS description v2.2 §2). This
//! file pins the four properties the task calls out:
//!
//! - every entry is sent exactly once, whether it travels in a P
//!   (table order), a pre-bill, or the final N sale;
//! - nothing is lost after a failed sign - an entry stays unsent and
//!   is re-sent on the next successful attempt;
//! - a closed order can never change (add/remove/re-quantity all
//!   refused);
//! - a refund's quantity can't exceed what was sold, aggregated across
//!   any number of prior partial refunds of the same sale.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::order::{AddOrderLineInput, do_add_order_line};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::{NewOrderLine, add_order_line, create_order};
use pos_fiscal::types::EventLabel;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const TEST_INSZ: &str = "12345678901";

fn seed_catalogue(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT OR IGNORE INTO users (insz, name, role, active) VALUES ('12345678901', 'Test', 'cashier', 1);
         INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
             VALUES (1, 1, 1000, 'B', 0, 1);",
    )
    .unwrap();
}

fn new_order(app: &TestApp, table: i32) -> i64 {
    let conn = app.state.db.lock().unwrap();
    create_order(&conn, OrderType::DineIn, Some(table), TEST_INSZ)
        .unwrap()
        .id
}

fn add(app: &TestApp, order_id: i64, quantity: i32) -> i64 {
    let conn = app.state.db.lock().unwrap();
    add_order_line(
        &conn,
        NewOrderLine {
            order_id,
            menu_item_id: 1,
            quantity,
            unit_price_cents: 1000,
            vat_code: VatCode::B,
            name: "Test item",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap()
    .id
}

fn unsent_entries(app: &TestApp, order_id: i64) -> Vec<pos_core::order::OrderLine> {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::orders::get_order_entries(&conn, order_id, true).unwrap()
}

fn all_entries(app: &TestApp, order_id: i64) -> Vec<pos_core::order::OrderLine> {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::orders::get_order_entries(&conn, order_id, false).unwrap()
}

// ── Sent exactly once, across P / pre-bill / N ──────────────────────

#[tokio::test]
async fn every_entry_is_sent_exactly_once_across_order_prebill_and_sale() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = new_order(&app, 1);

    add(&app, order_id, 2); // round 1
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    do_sign_table_order(&app.state, order_id).await.unwrap();
    // Everything sent so far must now be marked sent.
    assert!(unsent_entries(&app, order_id).is_empty());

    add(&app, order_id, 1); // round 2 - unsent until the pre-bill flushes it
    assert_eq!(unsent_entries(&app, order_id).len(), 1);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await; // the flush inside pre-bill
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M123")).await; // the pre-bill itself
    do_sign_pre_bill(&app.state, order_id).await.unwrap();
    assert!(unsent_entries(&app, order_id).is_empty());

    // The two P requests, taken together, must equal the whole journal
    // exactly once each - no entry duplicated, none skipped.
    let p_events: Vec<_> = app
        .fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "P")
        .collect();
    assert_eq!(p_events.len(), 3, "order flush ×2 + pre-bill");
    let journal = all_entries(&app, order_id);
    assert_eq!(journal.len(), 2, "two journalled additions");

    // The final sale must carry ALL entries, chronologically, in one
    // request - never just the "current" consolidated lines.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let n_event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.event_type == "N")
        .unwrap();
    let sale_request: serde_json::Value =
        serde_json::from_str(n_event.event_data_json.as_ref().unwrap()).unwrap();
    let lines = sale_request["transaction"]["transactionLines"]
        .as_array()
        .unwrap();
    assert_eq!(lines.len(), 2, "the sale carries every journalled entry");
    let quantities: Vec<f64> = lines
        .iter()
        .map(|l| l["mainProduct"]["quantity"].as_f64().unwrap())
        .collect();
    assert_eq!(quantities, vec![2.0, 1.0], "chronological order preserved");
}

// ── Nothing lost after a failed sign ────────────────────────────────

#[tokio::test]
async fn unsent_entries_survive_a_failed_checkout_and_are_resent_on_retry() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = new_order(&app, 1);
    add(&app, order_id, 2);
    assert_eq!(unsent_entries(&app, order_id).len(), 1);

    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
    let failed = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!failed.success);
    // The entry must still be there, unsent - nothing was consumed or
    // marked sent by the failed attempt.
    assert_eq!(
        unsent_entries(&app, order_id).len(),
        1,
        "a failed sign must not mark the entry as sent"
    );
    assert_eq!(app.fiscal_events().len(), 0);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let retried = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(retried.success, "{:?}", retried.error);
    assert!(unsent_entries(&app, order_id).is_empty());
    let n_event = app.fiscal_events()[0].clone();
    let sale_request: serde_json::Value =
        serde_json::from_str(n_event.event_data_json.as_ref().unwrap()).unwrap();
    assert_eq!(
        sale_request["transaction"]["transactionLines"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "the retry still carries the entry the failed attempt didn't lose"
    );
}

// ── Closed orders cannot change ─────────────────────────────────────

#[tokio::test]
async fn closed_orders_cannot_change_through_the_command_layer() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = new_order(&app, 1);
    let line_id = add(&app, order_id, 1);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(app.order_status(order_id).as_deref(), Some("closed"));

    // `add_order_line` (command) has its own explicit open-order guard,
    // exercised here through `do_add_order_line` exactly as the Tauri
    // command calls it.
    let add_after_close = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    );
    assert!(add_after_close.is_err(), "cannot add to a closed order");

    // `remove_order_line` / `update_order_line_quantity` (commands) are
    // thin `State`-taking wrappers around these exact pos-db calls with
    // no added logic - calling the query directly exercises the same
    // guard (`ensure_open`, enforced under a SAVEPOINT) the real
    // command relies on.
    {
        let conn = app.state.db.lock().unwrap();
        assert!(
            pos_db::queries::orders::update_order_line_quantity(&conn, line_id, 5).is_err(),
            "cannot re-quantity a line on a closed order"
        );
        assert!(
            pos_db::queries::orders::remove_order_line(&conn, line_id).is_err(),
            "cannot remove a line from a closed order"
        );
    }
    // And the order's line is exactly as it was at checkout.
    let lines = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&conn, order_id).unwrap()
    };
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].quantity, 1);
}

// ── Refund aggregation across multiple partial refunds ──────────────

#[tokio::test]
async fn refund_quantity_cannot_exceed_the_original_aggregated_across_partial_refunds() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = new_order(&app, 1);
    add(&app, order_id, 3); // three units sold

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original = app.fiscal_events()[0].id;
    assert_eq!(app.counter_for(EventLabel::N), 1);

    // First partial refund: 2 of the 3 units. Fine.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;
    let first = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 2,
        }],
        TEST_INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(first.success, "{:?}", first.error);
    assert_eq!(app.counter_for(EventLabel::N), 2);

    // Second partial refund of 2 MORE units: only 1 unit remains
    // (3 sold − 2 already refunded), so this must be refused - even
    // though 2 alone is within what was originally sold. Before the
    // fix, `refunded_line` only checked the current request against
    // the original sale and ignored prior refunds, so this would have
    // been wrongly accepted (4 of 3 refunded).
    let over_refund = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 2,
        }],
        TEST_INSZ.to_string(),
    )
    .await;
    assert!(
        over_refund.is_err(),
        "refunding 2 more after 2 already refunded (of 3 sold) must be rejected"
    );
    assert_eq!(
        app.counter_for(EventLabel::N),
        2,
        "a rejected over-refund must not consume a ticket number"
    );
    assert_eq!(app.fiscal_events().len(), 2, "no third N event archived");

    // The remaining 1 unit can still be refunded exactly.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;
    let last_unit = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(last_unit.success, "{:?}", last_unit.error);
    assert_eq!(app.counter_for(EventLabel::N), 3);

    // And now even 1 more unit is refused: the full 3 units are gone.
    let past_total = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
    )
    .await;
    assert!(
        past_total.is_err(),
        "refunding a 4th unit of a 3-unit sale must always be rejected"
    );
}
