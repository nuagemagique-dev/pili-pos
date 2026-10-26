//! How a sale's payment reaches the FDM.
//!
//! - cash rounding is its own `ROUNDING` line of the cash payment,
//!   never part of the payment amount (producers FAQ Q403, Advanced
//!   Set UC201), and stays out of the user turnover (FAQ Q509);
//! - a sale that comes to 0,00 EUR is still signed as an N event with
//!   a VAT ticket, as a cash payment of 0 (FAQ Q206, Q209);
//! - a negative sale is refused before the FDM (refunds have their own
//!   command).
//!
//! Events are signed by the admission image's mock FDM.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::report::do_generate_x_report;
use serde_json::{Value, json};

/// A manager (reports are theirs), with a well-formed INSZ.
const OPERATOR: &str = "12345678958";

/// Catalogue: 1 = 3,04 (A), 2 = free (0,00, B).
async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
            (1, 1, 304, 'A', 0, 1),
            (2, 1, 0, 'B', 1, 1);",
    )
    .unwrap();
    drop(conn);
    app
}

#[expect(
    clippy::unused_async,
    reason = "async like its siblings, so every test awaits the helpers the same way"
)]
async fn order_of(app: &TestApp, lines: &[(i64, i32)]) -> i64 {
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
    for &(item, quantity) in lines {
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
    order
}

/// The `financials` the POS sent for its last signed event.
fn last_financials(app: &TestApp) -> Value {
    let event = app.fiscal_events().pop().expect("a signed event");
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    request["financials"].clone()
}

/// UC201: 2 x 3,04 = 6,08 paid cash is rounded to 6,10. The payment
/// line carries the sale amount, the rounding its own line.
#[tokio::test]
// covers: FAQ_Q402
async fn uc201_cash_rounding_is_its_own_payment_line() {
    let app = app().await;
    let order = order_of(&app, &[(1, 2)]).await;
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    let financials = last_financials(&app);
    let lines: Vec<(&str, &str, f64)> = financials
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["type"].as_str().unwrap(),
                f["amountType"].as_str().unwrap(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        [("CASH", "PAYMENT", 6.08), ("CASH", "ROUNDING", 0.02)]
    );
    // Both lines belong to the same payment method.
    assert_eq!(financials[0]["id"], financials[1]["id"]);
}

// ── Manual "Arrondissement des méthodes de paiement", test 1 ───────
// Direct sales of the manual's Dame Blanche (11,52, B) and Tapas
// Variation (3,34, B), paid cash only (situations 1 to 3). Tests 2 and
// 3 (ROUND4, ROUND5) pay one sale with meal vouchers, card and cash,
// which this version does not take.

const DAME_BLANCHE: i64 = 25003;
const TAPAS_VARIATION: i64 = 28007;

/// The cash lines (PAYMENT, ROUNDING) a direct cash sale sends.
async fn cash_sale(lines: &[(i64, i32)]) -> Vec<(String, f64)> {
    let app = app().await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (25003, 1, 1152, 'B', 2, 1),
                (28007, 1, 334, 'B', 3, 1);",
        )
        .unwrap();
    let order = order_of(&app, lines).await;
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    last_financials(&app)
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            assert_eq!(f["type"], "CASH");
            (
                f["amountType"].as_str().unwrap().to_string(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect()
}

fn lines(v: &[(&str, f64)]) -> Vec<(String, f64)> {
    v.iter().map(|(t, a)| (t.to_string(), *a)).collect()
}

/// Situation 1: 11,52 is paid 11,50, rounding -0,02.
#[tokio::test]
async fn round1_11_52_is_paid_11_50() {
    assert_eq!(
        cash_sale(&[(DAME_BLANCHE, 1)]).await,
        lines(&[("PAYMENT", 11.52), ("ROUNDING", -0.02)])
    );
}

/// Situation 2: 3,34 is paid 3,35, rounding +0,01.
#[tokio::test]
async fn round2_3_34_is_paid_3_35() {
    assert_eq!(
        cash_sale(&[(TAPAS_VARIATION, 1)]).await,
        lines(&[("PAYMENT", 3.34), ("ROUNDING", 0.01)])
    );
}

/// Situation 3: 11,52 + 3,34 = 14,86 is paid 14,85, rounding -0,01.
#[tokio::test]
async fn round3_14_86_is_paid_14_85() {
    assert_eq!(
        cash_sale(&[(DAME_BLANCHE, 1), (TAPAS_VARIATION, 1)]).await,
        lines(&[("PAYMENT", 14.86), ("ROUNDING", -0.01)])
    );
}

/// A card payment is never rounded: one line, the exact amount.
#[tokio::test]
async fn faq_q403_a_card_payment_has_no_rounding_line() {
    let app = app().await;
    let order = order_of(&app, &[(1, 2)]).await;
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let financials = last_financials(&app);
    assert_eq!(financials.as_array().unwrap().len(), 1);
    assert_eq!(financials[0]["amountType"], "PAYMENT");
    assert_eq!(financials[0]["amount"], json!(6.08));
}

/// The X report totals the rounding apart from the payment, and the
/// turnover is the sale total, rounding excluded.
#[tokio::test]
async fn faq_q403_rounding_is_reported_apart_and_left_out_of_the_turnover() {
    let app = app().await;
    let order = order_of(&app, &[(1, 2)]).await;
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    do_generate_x_report(&app.state, OPERATOR.into())
        .await
        .unwrap();
    let report = app.fiscal_events().pop().unwrap();
    let request: Value = serde_json::from_str(report.event_data_json.as_deref().unwrap()).unwrap();
    let turnover = &request["turnover"];
    let cash = turnover["payments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["type"] == "CASH")
        .expect("a cash total");
    let amounts: Vec<(&str, f64)> = cash["amount"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["type"].as_str().unwrap(),
                a["totalAmount"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(amounts, [("PAYMENT", 6.08), ("ROUNDING", 0.02)]);
    let sales = turnover["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["eventLabel"] == "N")
        .expect("the sales total");
    assert_eq!(sales["amount"], json!(6.08));
}

/// Q206, Q209: a sale that comes to 0,00 EUR is an N event with a VAT
/// ticket, sent with an empty `financials` array (Q206 b), and opens no drawer.
#[tokio::test]
// covers: FAQ_Q209
async fn faq_q206_a_zero_sale_is_signed_with_a_vat_ticket() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let order = order_of(&app, &[(2, 1)]).await;
    // Even when the operator taps "card".
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert!(r.ticket.is_some(), "a VAT ticket is issued");

    let event = app.fiscal_events().pop().unwrap();
    assert_eq!(event.event_type, "N");
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    assert_eq!(request["transaction"]["transactionTotal"], json!(0.0));
    assert_ne!(request["ticketMedium"], "NONE");
    // Q206 b: no money moved, so the empty `financials` array (the
    // other allowed form is a `PAY 0` cash line).
    assert_eq!(request["financials"], json!([]));

    printer.settle(1).await;
    assert_eq!(printer.drawer_kicks(), 0, "nothing to put in the drawer");
}

/// A negative sale never reaches the FDM.
#[tokio::test]
async fn a_negative_sale_is_refused_before_the_fdm() {
    let app = app().await;
    let order = order_of(&app, &[(1, 1)]).await;
    {
        // Not reachable from the catalogue: forced in the database.
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::add_order_line(
            &conn,
            NewOrderLine {
                order_id: order,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: -1000,
                vat_code: VatCode::A,
                name: "Negative",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();
    }
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!r.success);
    assert_eq!(r.error.as_deref(), Some(errors::ORDER_TOTAL_NEGATIVE));
    assert!(app.fiscal_events().is_empty());
}
