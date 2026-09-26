//! Regression test: SCE 2.0 "strictly sequential, no gaps" counter.
//!
//! The cert spec requires that fiscal counters never have gaps. The
//! production checkout flow used to call `FiscalCounter::next` BEFORE
//! the FDM call - meaning a transient FDM failure left the counter
//! advanced but the archive empty, causing the next attempt to use a
//! higher number and a permanent gap.
//!
//! This file pins the corrected behaviour: when the FDM rejects (or
//! is unreachable) the counter must NOT have advanced, and the same
//! ticket number must be re-presented on retry. That contract is what
//! lets the FDM's idempotency cover transient network failures
//! without breaking the sequential-numbering rule.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_fiscal::types::EventLabel;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const TEST_INSZ: &str = "12345678901";

fn seed_minimal_open_order(app: &TestApp) -> i64 {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO users (insz, name, role, active)
         VALUES (?1, 'Test', 'cashier', 1)",
        rusqlite::params![TEST_INSZ],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO menu_items
         (id, category_id, price_cents, vat_code, sort_order, active)
         VALUES (1, 1, 1000, 'B', 0, 1)",
        [],
    )
    .unwrap();
    let order = pos_db::queries::orders::create_order(&conn, OrderType::DineIn, Some(1), TEST_INSZ)
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

/// When the FDM returns an error, the N counter MUST NOT advance -
/// otherwise the retry attempts at a higher counter value, creating
/// a gap in the sequence and a SCE 2.0 spec violation.
#[tokio::test]
async fn checkout_fdm_failure_does_not_advance_counter() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    // Make the mock FDM return a 500 - `FdmClient::sign_sale` will
    // surface this as `FdmError::Http(...)`, which the checkout
    // command turns into a failed `CheckoutResult` without archiving.
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&app.fdm)
        .await;

    // Counter starts at 0; peek_next would return 1.
    assert_eq!(app.counter_for(EventLabel::N), 0);

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("checkout returned (the command itself does not panic)");

    assert!(!result.success, "checkout should fail when FDM returns 500");

    // The cert-critical assertion: counter must still be 0, so the
    // next attempt uses ticket number 1, not 2.
    assert_eq!(
        app.counter_for(EventLabel::N),
        0,
        "N counter must NOT advance when FDM call fails"
    );

    // And the archive must be empty - no orphan event row.
    assert_eq!(
        app.fiscal_events().len(),
        0,
        "no fiscal event should be archived when FDM fails"
    );

    // The order is still open so the operator can retry.
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
}

/// Same property for the refund flow. The original sale succeeds;
/// the refund FDM call fails; the refund must not leave the counter
/// or archive in an inconsistent state.
#[tokio::test]
async fn refund_fdm_failure_does_not_advance_counter() {
    use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};

    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    // First: M110 success for the original sale.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    // Second: any subsequent call returns 500 - used by the refund.
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.fdm)
        .await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original_id = app.fiscal_events()[0].id;
    assert_eq!(
        app.counter_for(EventLabel::N),
        1,
        "successful sale advances the counter to 1"
    );

    let result = do_refund_checkout(
        &app.state,
        original_id,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
    )
    .await
    .expect("refund returned");

    assert!(!result.success, "refund should fail when FDM returns 500");
    // Counter must STAY at 1; only one fiscal event archived.
    assert_eq!(
        app.counter_for(EventLabel::N),
        1,
        "refund FDM failure must NOT advance counter past the original sale"
    );
    assert_eq!(app.fiscal_events().len(), 1, "no refund event archived");
}
