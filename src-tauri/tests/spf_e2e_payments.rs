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

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::errors;
use moyun_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use moyun_pos::commands::report::do_generate_x_report;
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
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
/// ticket, paid as cash 0, and opens no drawer.
#[tokio::test]
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
    let financials = request["financials"].as_array().unwrap();
    assert_eq!(financials.len(), 1);
    assert_eq!(financials[0]["type"], "CASH");
    assert_eq!(financials[0]["amountType"], "PAYMENT");
    assert_eq!(financials[0]["amount"], json!(0.0));

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
