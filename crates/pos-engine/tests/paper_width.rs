//! Paper width per printer (`printer_paper_mm` and
//! `kitchen_printer_paper_mm` in `pos.json`): every document is laid out
//! for the roll of the printer it goes to.
//!
//! Events are signed by the admission image's mock FDM.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::config::PaperWidth;
use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};

const OPERATOR: &str = "12345678958";

/// A direct sale of a sake and a maki, both sent to the kitchen.
async fn sell(app: &TestApp) {
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 1, 1200, 'B', 1, 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Maki saumon');",
        )
        .unwrap();
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
    for item in [1, 2] {
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
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
}

/// The longest printed line of `documents` (the renderer's QR
/// placeholder aside: the printer draws the code).
fn widest(documents: &[String]) -> usize {
    documents
        .iter()
        .flat_map(|d| d.lines())
        .filter(|l| !l.trim_start().starts_with("[QR: "))
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
}

#[tokio::test]
async fn each_printer_gets_documents_laid_out_for_its_own_roll() {
    let app = TestApp::new().await;
    let tickets = PrinterSink::start().await;
    let kitchen = PrinterSink::start().await;
    {
        let mut config = app.state.config.lock().unwrap();
        config.printer_address = Some(tickets.addr.clone());
        config.kitchen_printer_address = Some(kitchen.addr.clone());
        config.printer_paper_mm = PaperWidth::Mm58;
    }
    sell(&app).await;

    assert_eq!(tickets.settle(1).await.len(), 1);
    let receipt = tickets.rendered_for(PaperWidth::Mm58);
    assert_eq!(widest(&receipt), 35, "{receipt:#?}");
    assert!(
        one_line(&receipt[0]).contains("Maki saumon"),
        "{receipt:#?}"
    );

    assert_eq!(kitchen.settle_slips(1).await.len(), 1);
    assert_eq!(widest(&kitchen.rendered()), 48);
}

#[tokio::test]
async fn without_a_kitchen_printer_the_slip_takes_the_ticket_printer_roll() {
    let app = TestApp::new().await;
    let printer = PrinterSink::start().await;
    {
        let mut config = app.state.config.lock().unwrap();
        config.printer_address = Some(printer.addr.clone());
        config.printer_paper_mm = PaperWidth::Mm58;
        config.kitchen_printer_paper_mm = PaperWidth::Mm80;
    }
    sell(&app).await;

    assert_eq!(printer.settle_slips(1).await.len(), 1);
    printer.settle(1).await;
    assert_eq!(widest(&printer.rendered_for(PaperWidth::Mm58)), 35);
}
