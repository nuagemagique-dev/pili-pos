//! A sale paid in several parts: card, meal vouchers and cash for the
//! rest, through the command the checkout screen calls.
//!
//! - card and meal voucher parts are exact, never rounded; only the
//!   cash rest is rounded, on its own ROUNDING line (certification
//!   manual rounding tests 2 and 3: ROUND4, ROUND5; FAQ Q402, Q403);
//! - every part is a PAYMENT line of its own method, on the ticket,
//!   its copy and the reports;
//! - a payment that does not settle the sale is refused before the FDM;
//! - the drawer opens only when cash is taken;
//! - a refund is paid back in cash, whatever paid the sale.
//!
//! Events are signed by the admission image's mock FDM.

mod common;

use common::*;
use serde_json::Value;

use pili_pos::commands::checkout::{
    CheckoutPaymentMethod::{Card, MealVoucher},
    PaymentPart, do_checkout_split,
};
use pili_pos::commands::copy::do_copy_ticket;
use pili_pos::commands::errors;
use pili_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pili_pos::commands::refund::{RefundItem, do_refund_checkout};
use pili_pos::commands::report::do_generate_x_report;
use pos_core::order::OrderType;
use pos_fiscal::types::EventLabel;

const MANAGER: &str = "12345678958";
const DAME_BLANCHE: i64 = 25003;
const TAPAS_VARIATION: i64 = 28007;

/// The manual's rounding products: Dame Blanche 11,52 and Tapas
/// Variation 3,34, both B.
async fn app() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (25003, 1, 1152, 'B', 0, 1),
                (28007, 1, 334, 'B', 1, 1);",
        )
        .unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

/// A direct sale of one Dame Blanche and one Tapas Variation: 14,86.
fn both_desserts(app: &TestApp) -> i64 {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    for item in [DAME_BLANCHE, TAPAS_VARIATION] {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: item,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    order
}

async fn pay(app: &TestApp, order: i64, parts: &[PaymentPart]) -> i64 {
    let r = do_checkout_split(&app.state, order, parts, None)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

/// `(type, amountType, amount)` of the financials of a signed event.
fn financials(app: &TestApp, event_id: i64) -> Vec<(String, String, f64)> {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .unwrap();
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["type"].as_str().unwrap().to_string(),
                f["amountType"].as_str().unwrap().to_string(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect()
}

fn row(t: &str, a: &str, x: f64) -> (String, String, f64) {
    (t.to_string(), a.to_string(), x)
}

/// The order's payment history (method, cents).
fn payment_rows(app: &TestApp) -> Vec<(String, i64)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT method, amount_cents FROM payments ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// A printed ticket's text, by its position among the receipts.
async fn printed(printer: &PrinterSink, n: usize) -> String {
    printer.settle(n).await;
    printer
        .rendered()
        .into_iter()
        .filter(|r| r.contains("INFORMATIONS FDM"))
        .nth(n - 1)
        .expect("the ticket")
}

/// A row of a printed text, its words joined by one space.
fn text_row(text: &str, start: &str) -> String {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("no {start:?} row in:\n{text}"))
}

/// Manual rounding test 2 (situation 4): 11,52 in meal vouchers, never
/// rounded; the rest 3,34 in cash, paid 3,35.
#[tokio::test]
async fn round4_meal_vouchers_then_cash() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    let sale = pay(
        &app,
        order,
        &[PaymentPart::of(MealVoucher, 1152), PaymentPart::cash()],
    )
    .await;

    assert_eq!(
        financials(&app, sale),
        [
            row("CHEQUE_MEAL", "PAYMENT", 11.52),
            row("CASH", "PAYMENT", 3.34),
            row("CASH", "ROUNDING", 0.01),
        ]
    );
    assert_eq!(
        payment_rows(&app),
        [
            ("meal_voucher".to_string(), 1152),
            ("cash".to_string(), 335)
        ]
    );
    let ticket = printed(&printer, 1).await;
    assert_eq!(text_row(&ticket, "Cheques-repas"), "Cheques-repas 11,52");
    assert_eq!(text_row(&ticket, "Especes"), "Especes 3,34");
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi 0,01");
    assert_eq!(printer.drawer_kicks(), 1, "cash was taken");
}

/// Manual rounding test 3 (situation 5): 3,13 in meal vouchers, 3,97
/// by debit card, the rest 7,76 in cash, paid 7,75.
#[tokio::test]
async fn round5_meal_vouchers_card_and_cash() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    let sale = pay(
        &app,
        order,
        &[
            PaymentPart::of(MealVoucher, 313),
            PaymentPart::of(Card, 397),
            PaymentPart::cash(),
        ],
    )
    .await;

    assert_eq!(
        financials(&app, sale),
        [
            row("CHEQUE_MEAL", "PAYMENT", 3.13),
            row("CARD_DEBIT", "PAYMENT", 3.97),
            row("CASH", "PAYMENT", 7.76),
            row("CASH", "ROUNDING", -0.01),
        ]
    );
    let ticket = printed(&printer, 1).await;
    for expected in [
        "Cheques-repas 3,13",
        "Carte 3,97",
        "Especes 7,76",
        "Arrondi -0,01",
    ] {
        let start = expected.rsplit_once(' ').unwrap().0;
        assert_eq!(text_row(&ticket, start), expected);
    }
}

#[tokio::test]
async fn meal_vouchers_for_the_whole_sale_keep_the_drawer_shut() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    let sale = pay(&app, order, &[PaymentPart::of(MealVoucher, 1486)]).await;
    assert_eq!(
        financials(&app, sale),
        [row("CHEQUE_MEAL", "PAYMENT", 14.86)]
    );
    printed(&printer, 1).await;
    assert_eq!(printer.drawer_kicks(), 0);
}

#[tokio::test]
async fn card_and_meal_vouchers_keep_the_drawer_shut() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    pay(
        &app,
        order,
        &[
            PaymentPart::of(Card, 1000),
            PaymentPart::of(MealVoucher, 486),
        ],
    )
    .await;
    printed(&printer, 1).await;
    assert_eq!(printer.drawer_kicks(), 0);
}

/// Nothing is signed when the payment does not settle the sale.
#[tokio::test]
async fn a_payment_that_does_not_settle_the_sale_is_refused_before_the_fdm() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    for (parts, code) in [
        (
            vec![PaymentPart::of(Card, 1000)],
            errors::PAYMENT_INCOMPLETE,
        ),
        (
            vec![
                PaymentPart::of(Card, 1000),
                PaymentPart::of(MealVoucher, 500),
            ],
            errors::PAYMENT_EXCEEDS_TOTAL,
        ),
        (
            vec![PaymentPart::of(MealVoucher, 0), PaymentPart::cash()],
            errors::PAYMENT_INVALID,
        ),
        (
            vec![PaymentPart::cash(), PaymentPart::cash()],
            errors::PAYMENT_INVALID,
        ),
    ] {
        let r = do_checkout_split(&app.state, order, &parts, None)
            .await
            .unwrap();
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some(code));
    }
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::N), 0);
    assert_eq!(app.order_status(order).as_deref(), Some("open"));
    assert!(printer.settle(0).await.is_empty());
}

/// The X report totals each method apart, the rounding apart from the
/// cash payment, and counts one payment per method.
#[tokio::test]
async fn the_x_report_totals_each_method_apart() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    pay(
        &app,
        order,
        &[
            PaymentPart::of(MealVoucher, 313),
            PaymentPart::of(Card, 397),
            PaymentPart::cash(),
        ],
    )
    .await;
    printed(&printer, 1).await;
    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);

    let report = app.fiscal_events().pop().unwrap();
    let request: Value = serde_json::from_str(report.event_data_json.as_deref().unwrap()).unwrap();
    let mut totals: Vec<(String, String, f64)> = request["turnover"]["payments"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| {
            p["amount"].as_array().unwrap().iter().map(|a| {
                (
                    p["type"].as_str().unwrap().to_string(),
                    a["type"].as_str().unwrap().to_string(),
                    a["totalAmount"].as_f64().unwrap(),
                )
            })
        })
        .collect();
    totals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(
        totals,
        [
            row("CARD_DEBIT", "PAYMENT", 3.97),
            row("CASH", "PAYMENT", 7.76),
            row("CASH", "ROUNDING", -0.01),
            row("CHEQUE_MEAL", "PAYMENT", 3.13),
        ]
    );

    printer.settle(2).await;
    let text = printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("CHIFFRE D'AFFAIRES"))
        .expect("the X report");
    assert_eq!(text_row(&text, "Cheques-repas"), "Cheques-repas 3,13");
    assert_eq!(
        text_row(&text, "Total des paiements"),
        "Total des paiements 14,86"
    );
    assert_eq!(text_row(&text, "Arrondis"), "Arrondis -0,01");
}

/// The user report counts a rounded cash sale as one payment.
#[tokio::test]
async fn a_rounded_cash_sale_is_one_payment_on_the_user_report() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    pay(&app, order, &[PaymentPart::cash()]).await;
    printed(&printer, 1).await;
    let r = pili_pos::commands::report::do_generate_user_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    printer.settle(2).await;
    let text = printer.rendered().pop().unwrap();
    assert!(text.contains("Especes (1x)"), "{text}");
    assert!(!text.contains("(2x)"), "{text}");
}

#[tokio::test]
async fn a_copy_of_a_split_sale_reprints_every_payment() {
    let (app, printer) = app().await;
    let order = both_desserts(&app);
    let sale = pay(
        &app,
        order,
        &[
            PaymentPart::of(MealVoucher, 313),
            PaymentPart::of(Card, 397),
            PaymentPart::cash(),
        ],
    )
    .await;
    printed(&printer, 1).await;
    let copy = do_copy_ticket(&app.state, sale, MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let text = printed(&printer, 2).await;
    assert!(text.contains("COPIE"), "{text}");
    assert_eq!(text_row(&text, "Cheques-repas"), "Cheques-repas 3,13");
    assert_eq!(text_row(&text, "Carte"), "Carte 3,97");
    assert_eq!(text_row(&text, "Especes"), "Especes 7,76");
    assert_eq!(text_row(&text, "Arrondi"), "Arrondi -0,01");
}

/// A refund pays back in cash, whatever paid the sale.
#[tokio::test]
async fn a_split_sale_is_refunded_in_cash() {
    let (app, _printer) = app().await;
    let order = both_desserts(&app);
    let sale = pay(
        &app,
        order,
        &[PaymentPart::of(MealVoucher, 1152), PaymentPart::cash()],
    )
    .await;
    let refund = do_refund_checkout(
        &app.state,
        sale,
        vec![RefundItem {
            line_id: None,
            menu_item_id: TAPAS_VARIATION,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    assert_eq!(
        financials(&app, refund.fiscal_event_id.unwrap()),
        [row("CASH", "PAYMENT", -3.34)]
    );
}
