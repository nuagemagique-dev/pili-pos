//! A sale closed without payment: the customer left without paying
//! (Advanced Set UC4109, `commands/unpaid.rs`).
//!
//! - what was served is sold: an N (a T in training) with every entry
//!   of the order and no payment line (`financials: []`);
//! - only a manager closes it, confirmed by their PIN;
//! - no money moves: no drawer, no payment row, never refunded;
//! - the VAT ticket prints "Non paye" where the payments would be, and
//!   so does its copy;
//! - a refusal (PIN, role, order, FDM) signs nothing and leaves the
//!   order open.
//!
//! Events are signed by the admission image's mock FDM, except the
//! inconsistent-answer case (a wiremock replay).

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_THE_OPERATOR};
use pos_engine::commands::auth::do_set_pin;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{RefundItem, do_list_refundable_events, do_refund_checkout};
use pos_engine::commands::report::do_generate_x_report;
use pos_engine::commands::tables::do_sign_table_order;
use pos_engine::commands::unpaid::do_close_unpaid;
use pos_fiscal::types::EventLabel;

const MANAGER: &str = "12345678958";
const SERVER: &str = "84022899837";
const PIN: &str = "4321";
const BURGER: i64 = 1;
const FREE_WATER: i64 = 2;

async fn setup(admission: bool) -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    if admission {
        app.use_admission_mock().await;
    }
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 2150, 'B', 0, 1),
                (2, 1, 0, 'A', 1, 1);",
        )
        .unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

async fn app() -> (TestApp, PrinterSink) {
    setup(true).await
}

/// An open order of `items`, for a table or (None) a takeaway.
fn order(app: &TestApp, table: Option<i32>, items: &[i64]) -> i64 {
    let id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    for &item in items {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: id,
                menu_item_id: item,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    id
}

fn request_of(app: &TestApp, event_id: i64) -> Value {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .unwrap();
    serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap()
}

fn payment_rows(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM payments", [], |r| r.get(0))
        .unwrap()
}

async fn close_unpaid(app: &TestApp, order: i64) -> i64 {
    let r = do_close_unpaid(&app.state, order, MANAGER, PIN)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

/// A printed VAT ticket, by its position among the receipts.
async fn printed(printer: &PrinterSink, n: usize) -> String {
    printer.settle(n).await;
    printer
        .rendered()
        .into_iter()
        .filter(|r| r.contains("INFORMATIONS FDM"))
        .nth(n - 1)
        .expect("the ticket")
}

// covers: UC4109
#[tokio::test]
async fn a_manager_closes_a_table_the_customer_left_unpaid() {
    let (app, printer) = app().await;
    let table = order(&app, Some(5), &[BURGER, BURGER]);
    do_sign_table_order(&app.state, table).await.unwrap();

    let sale = close_unpaid(&app, table).await;

    let request = request_of(&app, sale);
    assert_eq!(request["financials"], serde_json::json!([]));
    assert_eq!(request["transaction"]["transactionTotal"], 43.0);
    assert_eq!(request["costCenter"]["type"], "TABLE");
    assert_eq!(request["employeeId"], MANAGER);
    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(labels, ["P", "N"]);
    assert_eq!(app.order_status(table).as_deref(), Some("closed"));
    assert_eq!(payment_rows(&app), 0, "no money came in");

    let ticket = printed(&printer, 1).await;
    assert!(ticket.contains("Non paye"), "{ticket}");
    assert!(!ticket.contains("Especes"), "{ticket}");
    assert!(ticket.contains("TICKET DE CAISSE T.V.A."), "{ticket}");
    assert_eq!(printer.drawer_kicks(), 0, "no cash, no drawer");
}

/// Every refusal comes before the FDM: nothing signed, the counter
/// unchanged, the order still open.
#[tokio::test]
async fn a_refused_close_signs_nothing() {
    let (app, printer) = app().await;
    let table = order(&app, Some(5), &[BURGER]);
    let empty = order(&app, Some(6), &[]);
    let free = order(&app, Some(7), &[FREE_WATER]);
    let paid = order(&app, Some(8), &[BURGER]);
    assert!(
        do_checkout(&app.state, paid, CheckoutPaymentMethod::Card)
            .await
            .unwrap()
            .success
    );
    let before = app.fiscal_events().len();
    printer.settle(1).await;

    let refused = |order: i64, insz: &'static str, pin: &'static str| {
        let app = &app;
        async move {
            let r = do_close_unpaid(&app.state, order, insz, pin).await.unwrap();
            assert!(!r.success);
            r.error.unwrap()
        }
    };
    assert_eq!(refused(table, MANAGER, "0000").await, errors::PIN_WRONG);
    assert_eq!(refused(table, MANAGER, "").await, errors::PIN_WRONG);
    assert_eq!(refused(table, SERVER, PIN).await, ERR_NOT_THE_OPERATOR);
    assert_eq!(refused(empty, MANAGER, PIN).await, errors::ORDER_NO_LINES);
    assert_eq!(
        refused(free, MANAGER, PIN).await,
        errors::UNPAID_NOTHING_DUE
    );
    assert_eq!(
        refused(paid, MANAGER, PIN).await,
        errors::ORDER_ALREADY_CLOSED
    );

    // A cashier cannot, even with their own PIN.
    app.login_as(SERVER, UserRole::Cashier);
    do_set_pin(&app.state, SERVER.into(), "1234".into(), None).unwrap();
    assert_eq!(refused(table, SERVER, "1234").await, ERR_FORBIDDEN);

    assert_eq!(app.fiscal_events().len(), before);
    assert_eq!(app.counter_for(EventLabel::N), 1, "only the paid sale");
    assert_eq!(app.order_status(table).as_deref(), Some("open"));
    assert_eq!(printer.drawer_kicks(), 0);
}

#[tokio::test]
async fn a_manager_without_a_pin_must_choose_one_first() {
    let (app, _printer) = app().await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE users SET pin_hash = NULL WHERE insz = ?1",
            [MANAGER],
        )
        .unwrap();
    let table = order(&app, Some(5), &[BURGER]);
    let r = do_close_unpaid(&app.state, table, MANAGER, PIN)
        .await
        .unwrap();
    assert_eq!(r.error.as_deref(), Some(errors::PIN_SETUP_REQUIRED));
    assert!(app.fiscal_events().is_empty());
}

/// Training cannot be left while an order is open: an abandoned
/// training table is closed unpaid too, as a T.
#[tokio::test]
async fn in_training_it_signs_a_training_sale() {
    let (app, printer) = app().await;
    app.state.config.lock().unwrap().training_mode = true;
    let table = order(&app, Some(5), &[BURGER]);
    let sale = close_unpaid(&app, table).await;
    assert_eq!(request_of(&app, sale)["financials"], serde_json::json!([]));
    let event = app.fiscal_events().pop().unwrap();
    assert_eq!(event.event_type, "T");
    assert_eq!(app.counter_for(EventLabel::N), 0);
    let ticket = printed(&printer, 1).await;
    assert!(ticket.contains("Non paye"), "{ticket}");
    assert!(ticket.contains("TRAINING"), "{ticket}");
}

#[tokio::test]
async fn an_unreachable_fdm_leaves_the_order_open() {
    let (app, printer) = app().await;
    let table = order(&app, Some(5), &[BURGER]);
    let fdm = app.state.config.lock().unwrap().fdm_url.clone();
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();
    let r = do_close_unpaid(&app.state, table, MANAGER, PIN)
        .await
        .unwrap();
    assert!(!r.success);
    assert!(r.error.unwrap().contains("FDM"));
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.order_status(table).as_deref(), Some("open"));
    assert!(printer.settle(0).await.is_empty());

    // Once the FDM is back, the same order closes, with ticket 1.
    app.state.config.lock().unwrap().fdm_url = fdm;
    close_unpaid(&app, table).await;
    assert_eq!(app.fiscal_events()[0].sequence_number, 1);
}

#[tokio::test]
async fn an_inconsistent_fdm_answer_leaves_the_order_open() {
    let (app, _printer) = setup(false).await;
    let table = order(&app, Some(5), &[BURGER]);
    // The official M110 answer as published: another POS's identity.
    mock_raw_response(
        &app,
        read_response_envelope(&load_uc(&basic_set_root(), "M110")),
    )
    .await;
    let r = do_close_unpaid(&app.state, table, MANAGER, PIN)
        .await
        .unwrap();
    assert!(!r.success);
    assert!(
        r.error
            .as_deref()
            .unwrap()
            .contains("FDM_ANSWER_PENDING|label=N|ticket=1|fdm=FDM_RESPONSE_MISMATCH|"),
        "{:?}",
        r.error
    );
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::N), 0);
    assert_eq!(app.order_status(table).as_deref(), Some("open"));
}

/// The turnover counts the sale; the payments do not: nothing was
/// paid.
#[tokio::test]
async fn the_x_report_counts_the_turnover_but_no_payment() {
    let (app, printer) = app().await;
    let table = order(&app, Some(5), &[BURGER]);
    close_unpaid(&app, table).await;
    printed(&printer, 1).await;
    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);

    let report = request_of(&app, app.fiscal_events().pop().unwrap().id);
    let turnover = &report["turnover"];
    assert_eq!(turnover["vats"][0]["totalAmount"], 21.5, "{turnover}");
    assert_eq!(turnover["transactions"][0]["amount"], 21.5, "{turnover}");
    assert_eq!(turnover["payments"], serde_json::json!([]), "{turnover}");
    assert_eq!(turnover["drawersOpenCount"], 0);
}

/// Nothing was paid, so nothing is paid back.
#[tokio::test]
async fn an_unpaid_sale_is_never_refunded() {
    let (app, _printer) = app().await;
    let table = order(&app, Some(5), &[BURGER]);
    let unpaid = close_unpaid(&app, table).await;
    let paid = order(&app, Some(6), &[BURGER]);
    let paid_sale = do_checkout(&app.state, paid, CheckoutPaymentMethod::Cash)
        .await
        .unwrap()
        .fiscal_event_id
        .unwrap();

    let listed: Vec<i64> = do_list_refundable_events(&app.state, None)
        .unwrap()
        .iter()
        .map(|e| e.fiscal_event_id)
        .collect();
    assert_eq!(listed, [paid_sale], "the unpaid sale is not offered");

    let before = app.fiscal_events().len();
    let r = do_refund_checkout(
        &app.state,
        unpaid,
        vec![RefundItem {
            line_id: None,
            menu_item_id: BURGER,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(!r.success);
    assert_eq!(r.error.as_deref(), Some(errors::REFUND_UNPAID_SALE));
    assert_eq!(app.fiscal_events().len(), before);
}

#[tokio::test]
async fn its_copy_says_unpaid_too() {
    let (app, printer) = app().await;
    let table = order(&app, Some(5), &[BURGER]);
    let sale = close_unpaid(&app, table).await;
    printed(&printer, 1).await;
    let copy = do_copy_ticket(&app.state, sale, MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let text = printed(&printer, 2).await;
    assert!(text.contains("COPIE"), "{text}");
    assert!(text.contains("Non paye"), "{text}");
}

/// A takeaway draft is a direct sale: closed unpaid, it ends in an N
/// without a cost center, never a P (FAQ Q205).
// covers: FAQ_Q205
#[tokio::test]
async fn faq_q205_a_takeaway_closed_unpaid_is_a_direct_sale() {
    let (app, _printer) = app().await;
    let takeaway = order(&app, None, &[BURGER]);
    let sale = close_unpaid(&app, takeaway).await;
    let request = request_of(&app, sale);
    assert!(request.get("costCenter").is_none_or(Value::is_null));
    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(labels, ["N"]);
}
