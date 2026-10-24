//! Certification manual, "Arrondissement des méthodes de paiement":
//! the five situations, with their evidence.
//!
//! pili-pos rounds the cash only (`pos-fiscal::payment`), so the manual
//! asks for basic tests 1 and 2 (situations 1 to 4) and for test 3
//! (situation 5), not for tests 4 and 5 (the other methods are never
//! rounded):
//!
//! | Situation | Test | Sale | Payment | Rounding |
//! |---|---|---|---|---|
//! | ROUND1 | 1 | Dame Blanche 11,52 | cash | -0,02 |
//! | ROUND2 | 1 | Tapas Variation 3,34 | cash | +0,01 |
//! | ROUND3 | 1 | both, 14,86 | cash | -0,01 |
//! | ROUND4 | 2 | both, 14,86 | meal vouchers 11,52, cash for the rest | +0,01 |
//! | ROUND5 | 3 | both, 14,86 | meal vouchers 3,13, debit card 3,97, cash for the rest | -0,01 |
//!
//! A cash refund is rounded the same way: the SPF refund ticket example
//! (`docs/20260904_Example_tickets/`) pays a Cola of -3,04 back as
//! -3,05, rounding -0,01.
//!
//! Each test goes through the command the checkout screen calls, with
//! the mock FDM of the admission image, and checks the request sent to
//! the FDM and the printed ticket against the manual's table.
//!
//! With `UC_EVIDENCE_DIR` set, the request and the response archived
//! for the sale are written as `ROUND<n>_event1_N_request.json` /
//! `_response.json`; with `TICKET_EVIDENCE_DIR` set, the printed ticket
//! is written as `ROUND<n>_ticket.txt` (see
//! `scripts/capture_uc_evidence.sh`). Nothing is written otherwise.

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{
    CheckoutPaymentMethod::{self, Card, MealVoucher},
    PaymentPart, do_checkout, do_checkout_split,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};

const MANAGER: &str = "12345678958";
const DAME_BLANCHE: i64 = 25003;
const TAPAS_VARIATION: i64 = 28007;
const COCA_COLA: i64 = 12009;

/// The manual's products, both with VAT B.
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
                (28007, 1, 334, 'B', 1, 1),
                (12009, 1, 304, 'A', 2, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('category', 1, 'name', 'fr', 'Desserts'),
                ('menu_item', 25003, 'name', 'fr', 'Dame Blanche'),
                ('menu_item', 28007, 'name', 'fr', 'Tapas Variation'),
                ('menu_item', 12009, 'name', 'fr', 'Coca Cola');",
        )
        .unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

fn direct_sale(app: &TestApp, items: &[i64]) -> i64 {
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
    for &item in items {
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

/// `(type, amountType, amount)` of the sale's financials, the printed
/// ticket; exports the evidence when asked to.
async fn settle(
    uc: &str,
    app: &TestApp,
    printer: &PrinterSink,
) -> (Vec<(String, String, f64)>, String) {
    let event = app.fiscal_events().pop().expect("the signed sale");
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    let financials = request["financials"]
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
        .collect();

    printer.settle(1).await;
    let ticket = printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("INFORMATIONS FDM"))
        .expect("the printed ticket");

    app.dump_evidence(uc);
    if let Ok(dir) = std::env::var("TICKET_EVIDENCE_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        // Drop the printer status queries (DLE EOT) that follow the cut.
        let text: String = ticket
            .chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .collect();
        std::fs::write(format!("{dir}/{uc}_ticket.txt"), text.trim_end()).unwrap();
    }
    (financials, ticket)
}

fn row(t: &str, a: &str, x: f64) -> (String, String, f64) {
    (t.to_string(), a.to_string(), x)
}

/// A row of a printed text, its words joined by one space.
fn text_row(text: &str, start: &str) -> String {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("no {start:?} row in:\n{text}"))
}

async fn cash_situation(uc: &str, items: &[i64]) -> (Vec<(String, String, f64)>, String) {
    let (app, printer) = app().await;
    let order = direct_sale(&app, items);
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    settle(uc, &app, &printer).await
}

async fn split_situation(uc: &str, parts: &[PaymentPart]) -> (Vec<(String, String, f64)>, String) {
    let (app, printer) = app().await;
    let order = direct_sale(&app, &[DAME_BLANCHE, TAPAS_VARIATION]);
    let r = do_checkout_split(&app.state, order, parts, None)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    settle(uc, &app, &printer).await
}

/// Situation 1: 11,52 is paid 11,50, rounding -0,02.
#[tokio::test]
async fn round1_dame_blanche_cash() {
    let (fin, ticket) = cash_situation("ROUND1", &[DAME_BLANCHE]).await;
    assert_eq!(
        fin,
        [
            row("CASH", "PAYMENT", 11.52),
            row("CASH", "ROUNDING", -0.02)
        ]
    );
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi -0,02");
}

/// Situation 2: 3,34 is paid 3,35, rounding +0,01.
#[tokio::test]
async fn round2_tapas_variation_cash() {
    let (fin, ticket) = cash_situation("ROUND2", &[TAPAS_VARIATION]).await;
    assert_eq!(
        fin,
        [row("CASH", "PAYMENT", 3.34), row("CASH", "ROUNDING", 0.01)]
    );
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi 0,01");
}

/// Situation 3: 14,86 is paid 14,85, rounding -0,01.
#[tokio::test]
async fn round3_both_desserts_cash() {
    let (fin, ticket) = cash_situation("ROUND3", &[DAME_BLANCHE, TAPAS_VARIATION]).await;
    assert_eq!(
        fin,
        [
            row("CASH", "PAYMENT", 14.86),
            row("CASH", "ROUNDING", -0.01)
        ]
    );
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi -0,01");
}

/// Situation 4 (test 2): 11,52 in meal vouchers, never rounded; the
/// rest 3,34 in cash, paid 3,35.
#[tokio::test]
async fn round4_meal_vouchers_then_cash() {
    let (fin, ticket) = split_situation(
        "ROUND4",
        &[PaymentPart::of(MealVoucher, 1152), PaymentPart::cash()],
    )
    .await;
    assert_eq!(
        fin,
        [
            row("CHEQUE_MEAL", "PAYMENT", 11.52),
            row("CASH", "PAYMENT", 3.34),
            row("CASH", "ROUNDING", 0.01),
        ]
    );
    assert_eq!(text_row(&ticket, "Chèques-repas"), "Chèques-repas 11,52");
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi 0,01");
}

/// Situation 5 (test 3): 3,13 in meal vouchers, 3,97 by debit card,
/// the rest 7,76 in cash, paid 7,75.
#[tokio::test]
async fn round5_meal_vouchers_card_and_cash() {
    let (fin, ticket) = split_situation(
        "ROUND5",
        &[
            PaymentPart::of(MealVoucher, 313),
            PaymentPart::of(Card, 397),
            PaymentPart::cash(),
        ],
    )
    .await;
    assert_eq!(
        fin,
        [
            row("CHEQUE_MEAL", "PAYMENT", 3.13),
            row("CARD_DEBIT", "PAYMENT", 3.97),
            row("CASH", "PAYMENT", 7.76),
            row("CASH", "ROUNDING", -0.01),
        ]
    );
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi -0,01");
}

/// The SPF refund ticket example: a Cola of 3,04 refunded in cash is
/// paid back 3,05, rounding -0,01, on the request, the ticket and in
/// the drawer.
#[tokio::test]
async fn a_cash_refund_is_rounded_like_the_spf_example() {
    let (app, printer) = app().await;
    let order = direct_sale(&app, &[COCA_COLA]);
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let refund = do_refund_checkout(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        vec![RefundItem {
            line_id: None,
            menu_item_id: COCA_COLA,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);

    let event = app.fiscal_events().pop().expect("the signed refund");
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    let financials: Vec<_> = request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            row(
                f["type"].as_str().unwrap(),
                f["amountType"].as_str().unwrap(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        financials,
        [
            row("CASH", "PAYMENT", -3.04),
            row("CASH", "ROUNDING", -0.01)
        ]
    );

    printer.settle(2).await;
    let ticket = printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("REMBOURSEMENT"))
        .expect("the refund ticket");
    assert_eq!(text_row(&ticket, "Arrondi"), "Arrondi -0,01");

    let paid_back: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT amount_cents FROM payments ORDER BY id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(paid_back, -305);
}
