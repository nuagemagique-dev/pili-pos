//! Bar / kitchen order slip (traceability ID VI_BAR, SPF example bar
//! ticket of 2026-09-24).
//!
//! - a table order prints its newly sent lines once, corrections
//!   included as negative lines;
//! - a direct sale prints the lines no table order sent, at checkout;
//! - products without the kitchen flag are left out;
//! - the slip goes to the kitchen printer, else the ticket printer;
//! - no printer, or a failed one, never undoes the signing; a printer
//!   stopped (paper out, cover open, error) counts as failed;
//! - a bar ticket shows no price and is not a VAT ticket.
//!
//! Events are signed by the admission image's mock FDM.

mod common;

use common::*;

use pili_pos::commands::bar_slip::SlipOutcome;
use pili_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pili_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_remove_order_line,
};
use pili_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;

const OPERATOR: &str = "12345678958";

/// Catalogue: 1 = Sake (bar), 2 = Maki (kitchen), 3 = Bottled water
/// (served at the counter, off the slip).
async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
            (1, 1, 650, 'A', 1, 0, 1),
            (2, 1, 1200, 'B', 1, 1, 1),
            (3, 1, 300, 'A', 0, 2, 1);
         INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
            ('menu_item', 1, 'name', 'fr', 'Sake'),
            ('menu_item', 2, 'name', 'fr', 'Maki saumon'),
            ('menu_item', 3, 'name', 'fr', 'Eau plate');",
    )
    .unwrap();
    drop(conn);
    app
}

fn order(app: &TestApp, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id
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

#[tokio::test]
async fn vi_bar_a_table_order_prints_its_new_lines_once() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(4));
    add(&app, o, 1, 2);
    add(&app, o, 2, 1);

    let outcome = do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(outcome, SlipOutcome::Printed);
    let slips = printer.settle_slips(1).await;
    assert_eq!(slips.len(), 1, "{slips:?}");
    for expected in ["Table 4", "2 x Sake", "1 x Maki saumon", "CECI N'EST PAS"] {
        assert!(slips[0].contains(expected), "{expected:?} in {}", slips[0]);
    }
    assert!(!slips[0].contains("6,50") && !slips[0].contains("12,00"));

    // Nothing new: no P event, no slip.
    let events = app.fiscal_events().len();
    let outcome = do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(outcome, SlipOutcome::NothingToPrint);
    assert_eq!(app.fiscal_events().len(), events);
    assert_eq!(printer.settle_slips(2).await.len(), 1);
}

#[tokio::test]
async fn vi_bar_a_correction_prints_as_a_negative_line() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(2));
    let line = add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();

    do_remove_order_line(&app.state, line).unwrap();
    let outcome = do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(outcome, SlipOutcome::Printed);
    let slips = printer.settle_slips(2).await;
    assert_eq!(slips.len(), 2);
    assert!(slips[1].contains("-2 x Sake"), "{}", slips[1]);
}

#[tokio::test]
async fn vi_bar_an_item_off_the_kitchen_is_left_out() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(1));
    add(&app, o, 3, 1);
    // Signed, but nothing for the bar or kitchen.
    let outcome = do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(outcome, SlipOutcome::NothingToPrint);
    assert_eq!(app.fiscal_events().last().unwrap().event_type, "P");

    add(&app, o, 3, 1);
    add(&app, o, 2, 1);
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    assert_eq!(slips.len(), 1);
    assert!(slips[0].contains("1 x Maki saumon"));
    assert!(!slips[0].contains("Eau plate"), "{}", slips[0]);
}

#[tokio::test]
async fn vi_bar_without_a_printer_the_order_is_still_signed() {
    let app = app().await;
    let o = order(&app, Some(1));
    add(&app, o, 1, 1);
    let outcome = do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(outcome, SlipOutcome::NoPrinter);
    assert_eq!(app.fiscal_events().last().unwrap().event_type, "P");
}

#[tokio::test]
async fn vi_bar_a_failed_printer_never_undoes_the_signing() {
    let app = app().await;
    app.set_printer(DEAD_PRINTER);
    let o = order(&app, Some(1));
    add(&app, o, 1, 1);
    let outcome = do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(outcome, SlipOutcome::Failed);
    let events = app.fiscal_events();
    assert_eq!(events.last().unwrap().event_type, "P");
    // The lines are sent: the next registration has nothing to add.
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::NothingToPrint
    );
    assert_eq!(app.fiscal_events().len(), events.len());
}

/// The slip against a printer in each state, as an Epson printer
/// reports it (`pos-printer-mock`): only a stopped printer fails it.
#[tokio::test]
async fn vi_bar_a_stopped_printer_fails_the_slip() {
    use pos_printer_mock::{Control, Paper, Printer};
    let cases = [
        (Control::default(), SlipOutcome::Printed),
        (
            Control {
                paper: Paper::NearEnd,
                ..Control::default()
            },
            SlipOutcome::Printed,
        ),
        (
            Control {
                paper: Paper::End,
                ..Control::default()
            },
            SlipOutcome::Failed,
        ),
        (
            Control {
                cover_open: true,
                ..Control::default()
            },
            SlipOutcome::Failed,
        ),
        (
            Control {
                error: true,
                ..Control::default()
            },
            SlipOutcome::Failed,
        ),
    ];
    for (control, expected) in cases {
        let app = app().await;
        let printer = Printer::new(control.clone(), None);
        let addr = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer)
            .await
            .unwrap();
        app.set_printer(&addr.to_string());
        let o = order(&app, Some(1));
        add(&app, o, 1, 1);
        let outcome = do_sign_table_order(&app.state, o).await.unwrap();
        assert_eq!(outcome, expected, "{control:?}");
        assert_eq!(app.fiscal_events().last().unwrap().event_type, "P");
    }
}

#[tokio::test]
async fn vi_bar_the_kitchen_printer_takes_the_slip() {
    let app = app().await;
    let tickets = PrinterSink::start().await;
    let kitchen = PrinterSink::start().await;
    app.set_printer(&tickets.addr);
    app.state.config.lock().unwrap().kitchen_printer_address = Some(kitchen.addr.clone());
    let o = order(&app, Some(5));
    add(&app, o, 2, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(kitchen.settle_slips(1).await.len(), 1);
    assert!(tickets.settle_slips(1).await.is_empty());
}

#[tokio::test]
async fn vi_bar_a_direct_sale_prints_its_lines_at_checkout() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, None);
    add(&app, o, 1, 1);
    add(&app, o, 3, 1);
    let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(r.kitchen_slip, Some(SlipOutcome::Printed));
    let slips = printer.settle_slips(1).await;
    assert_eq!(slips.len(), 1, "{slips:?}");
    assert!(slips[0].contains("Vente directe") && slips[0].contains("1 x Sake"));
    assert!(!slips[0].contains("Eau plate"));
}

#[tokio::test]
async fn vi_bar_checkout_prints_only_the_lines_not_yet_sent() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(3));
    add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    add(&app, o, 2, 2);
    let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let slips = printer.settle_slips(2).await;
    assert_eq!(slips.len(), 2, "{slips:?}");
    assert!(slips[1].contains("2 x Maki saumon"), "{}", slips[1]);
    assert!(!slips[1].contains("Sake"), "{}", slips[1]);

    // Everything sent: paying again prints nothing new.
    let o = order(&app, Some(3));
    add(&app, o, 3, 1);
    let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert_eq!(r.kitchen_slip, Some(SlipOutcome::NothingToPrint));
}

#[tokio::test]
async fn vi_bar_a_pre_bill_sends_its_new_lines_to_the_bar() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = order(&app, Some(6));
    add(&app, o, 1, 3);
    do_sign_pre_bill(&app.state, o).await.unwrap();
    let slips = printer.settle_slips(1).await;
    assert_eq!(slips.len(), 1, "{slips:?}");
    assert!(slips[0].contains("3 x Sake"));
}

#[tokio::test]
async fn vi_bar_training_mode_marks_the_slip() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    app.state.config.lock().unwrap().training_mode = true;
    let o = order(&app, Some(1));
    add(&app, o, 1, 1);
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    assert_eq!(slips.len(), 1);
    assert!(slips[0].contains("TRAINING"));
}
