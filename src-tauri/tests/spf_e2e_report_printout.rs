//! The printed X and Z turnover reports carry what the AM of 29 April
//! 2024 art. 29 asks, in the order of the SPF example Z report of
//! 2026-09-24: issuer and establishment unit, report number and time,
//! opening period, the FDM's first and last event, turnover per event
//! label, VAT for every rate, departments, corrections, refunds,
//! drawer openings, training tickets, reductions, invoices, payments
//! with their total and the rounding apart, and the mention that a
//! report is not a VAT ticket.
//!
//! Events are signed by the admission image's mock FDM.

mod common;

use common::*;

use pili_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pili_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pili_pos::commands::refund::{RefundItem, do_refund_checkout};
use pili_pos::commands::report::{do_generate_x_report, do_generate_z_report};
use pos_core::order::OrderType;
use pos_core::user::UserRole;

const MANAGER: &str = "12345678958";

async fn app() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                VALUES ('category', 1, 'name', 'fr', 'Boissons');
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 304, 'A', 0, 1);",
        )
        .unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

/// 2 x 3,04 paid cash (rounded to 6,10), then one of them refunded.
async fn a_day(app: &TestApp) {
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
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: 1,
            quantity: 2,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let refund = do_refund_checkout(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
}

/// The printed turnover report (the user report of the Z follows it).
async fn printed_report(printer: &PrinterSink, receipts_before: usize) -> String {
    printer.settle(receipts_before + 1).await;
    printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("CHIFFRE D'AFFAIRES"))
        .expect("the turnover report")
}

/// A row of the report, its words joined by one space.
fn row(text: &str, start: &str) -> String {
    text.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .find(|l| l.starts_with(start))
        .unwrap_or_else(|| panic!("no {start:?} row in:\n{text}"))
}

#[tokio::test]
async fn m181_the_z_report_prints_the_article_29_content() {
    let (app, printer) = app().await;
    a_day(&app).await;
    let before = printer.receipts().len();
    let r = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let text = printed_report(&printer, before).await;

    for expected in [
        "Z CHIFFRE D'AFFAIRES",
        "The FPS Finances Experience",
        "Unite d'etablissement 2000000042",
        "Rapport nr. 1",
        "Periode d'ouverture",
        "BookingPeriodId:",
        "FDMID",
        "Apercu TVA",
        "Departements du chiffre d'affaires",
        "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
    ] {
        assert!(
            one_line(&text).contains(expected),
            "{expected:?} missing from:\n{text}"
        );
    }
    // The FDM's first and last event: total counter and time.
    assert!(row(&text, "Premier").split(' ').count() >= 3, "{text}");
    assert!(row(&text, "Dernier").split(' ').count() >= 3, "{text}");
    // Turnover: 6,08 sold, 3,04 refunded.
    assert_eq!(row(&text, "Boissons"), "Boissons 3,04");
    assert_eq!(row(&text, "Remboursements 1"), "Remboursements 1 -3,04");
    assert_eq!(
        row(&text, "Tickets de formation"),
        "Tickets de formation 0 0,00"
    );
    assert_eq!(row(&text, "Factures"), "Factures 0");
    // Every rate, "-" where nothing was sold.
    assert!(row(&text, "21%").ends_with("3,04"), "{text}");
    assert_eq!(row(&text, "12%"), "12% - - -");
    // Payments recorded, the rounding apart (FAQ Q403).
    assert_eq!(
        row(&text, "Total des paiements"),
        "Total des paiements 3,04"
    );
    assert_eq!(row(&text, "Arrondis"), "Arrondis 0,02");
}

#[tokio::test]
async fn m180_the_x_report_is_not_numbered() {
    let (app, printer) = app().await;
    a_day(&app).await;
    let before = printer.receipts().len();
    let r = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let text = printed_report(&printer, before).await;
    assert!(text.contains("X CHIFFRE D'AFFAIRES"), "{text}");
    assert!(!text.contains("Rapport nr."), "{text}");
    assert_eq!(
        row(&text, "Total des paiements"),
        "Total des paiements 3,04"
    );
}
