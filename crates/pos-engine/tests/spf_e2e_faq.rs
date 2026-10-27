//! Rules of the SPF producers' FAQ (20260717) that no Basic Set or
//! Reports scenario exercises on its own, played through the commands
//! the UI calls.
//!
//! - Q205: a direct sale (takeaway) ends in N, never in a P: it stays
//!   a listed draft until it is paid;
//! - Q207: a signOrder carries only the lines added since the last one;
//! - Q211: every table session has its own costCenter reference;
//! - Q504: a copy is not turnover;
//! - Q506: signing never waits for the Status query;
//! - Q509: a user's totalAmount is the sum of their sales, the cash
//!   rounding left out.
//!
//! Events are signed by the admission image's mock FDM, except Q506
//! which needs to see every request the FDM receives.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::report::{do_generate_user_x_report, do_generate_x_report};
use pos_engine::commands::tables::{do_list_open_takeaways, do_sign_table_order};

const MANAGER: &str = "12345678958";

/// Catalogue: 1 = 3,04 (A), 2 = 12,00 (B).
async fn app() -> TestApp {
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
                (1, 1, 304, 'A', 0, 1),
                (2, 1, 1200, 'B', 1, 1);",
        )
        .unwrap();
    app
}

fn open(app: &TestApp, table: Option<i32>) -> i64 {
    do_create_order(
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
    .id
}

fn add(app: &TestApp, order_id: i64, menu_item_id: i64, quantity: i32) {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
}

/// The archived requests, oldest first.
fn requests(app: &TestApp) -> Vec<Value> {
    app.fiscal_events()
        .iter()
        .map(|e| serde_json::from_str(e.event_data_json.as_deref().unwrap()).unwrap())
        .collect()
}

fn product_ids(request: &Value) -> Vec<String> {
    request["transaction"]["transactionLines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["mainProduct"]["productId"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn faq_q205_a_takeaway_is_never_registered_as_an_order() {
    let app = app().await;
    let takeaway = open(&app, None);
    add(&app, takeaway, 1, 1);
    let refused = do_sign_table_order(&app.state, takeaway).await;
    assert_eq!(refused, Err(errors::TAKEAWAY_NOT_REGISTERED.to_string()));
    assert!(app.fiscal_events().is_empty());

    // Paid, it ends in a single N, without a cost center.
    let sale = do_checkout(&app.state, takeaway, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(labels, ["N"]);
    assert!(requests(&app)[0].get("costCenter").is_none());
}

#[tokio::test]
async fn faq_q205_open_takeaways_are_listed_until_paid() {
    let app = app().await;
    let empty = open(&app, None);
    let first = open(&app, None);
    add(&app, first, 1, 2);
    let second = open(&app, None);
    add(&app, second, 2, 1);
    let table = open(&app, Some(3));
    add(&app, table, 1, 1);

    let listed = |app: &TestApp| -> Vec<(i64, i64, i64)> {
        do_list_open_takeaways(&app.state)
            .unwrap()
            .iter()
            .map(|t| (t.order_id, t.total_cents, t.line_count))
            .collect()
    };
    // Empty drafts and tables are not takeaways in progress.
    assert_eq!(listed(&app), [(first, 608, 1), (second, 1200, 1)]);
    assert_ne!(empty, first);

    let sale = do_checkout(&app.state, first, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    assert_eq!(listed(&app), [(second, 1200, 1)]);
}

#[tokio::test]
async fn faq_q207_a_second_order_carries_only_the_new_lines() {
    let app = app().await;
    let order = open(&app, Some(7));
    add(&app, order, 1, 2);
    do_sign_table_order(&app.state, order).await.unwrap();
    add(&app, order, 2, 1);
    do_sign_table_order(&app.state, order).await.unwrap();
    // Nothing new: no third order.
    do_sign_table_order(&app.state, order).await.unwrap();

    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(labels, ["P", "P"]);
    let r = requests(&app);
    assert_eq!(product_ids(&r[0]), ["1"]);
    assert_eq!(product_ids(&r[1]), ["2"]);
    assert_eq!(r[1]["transaction"]["transactionTotal"].as_f64(), Some(12.0));
}

#[tokio::test]
async fn faq_q211_each_table_session_has_its_own_reference() {
    let app = app().await;
    let first = open(&app, Some(7));
    add(&app, first, 1, 1);
    do_sign_table_order(&app.state, first).await.unwrap();
    let paid = do_checkout(&app.state, first, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(paid.success, "{:?}", paid.error);

    let second = open(&app, Some(7));
    assert_ne!(second, first);
    add(&app, second, 2, 1);
    do_sign_table_order(&app.state, second).await.unwrap();

    let r = requests(&app);
    let reference = |i: usize| {
        r[i]["costCenter"]["reference"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(r[0]["costCenter"]["id"], "T07");
    assert_eq!(r[2]["costCenter"]["id"], "T07");
    // The order and its sale share the session's reference...
    assert_eq!(reference(0), reference(1));
    // ...and the next guests at the same table get a new one.
    assert_ne!(reference(0), reference(2));
}

#[tokio::test]
async fn faq_q504_a_copy_is_not_turnover() {
    let app = app().await;
    let order = open(&app, None);
    add(&app, order, 1, 1);
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let copy = do_copy_ticket(&app.state, sale.fiscal_event_id.unwrap(), MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    assert_eq!(app.fiscal_events()[1].event_type, "C");

    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    let report = requests(&app).pop().unwrap();
    let transactions: Vec<_> = report["turnover"]["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["eventLabel"].as_str().unwrap().to_string(),
                t["ticketCount"].as_i64().unwrap(),
                t["amount"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(transactions, [("N".to_string(), 1, 3.04)]);
}

#[tokio::test]
async fn faq_q506_a_sale_is_signed_without_a_status_query() {
    let app = TestApp::new().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let order = {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 304, 'A', 0, 1);",
        )
        .unwrap();
        drop(conn);
        open(&app, None)
    };
    add(&app, order, 1, 1);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);

    let received = app.fdm.received_requests().await.unwrap();
    assert_eq!(received.len(), 1, "one request: the signSale");
    let body: Value = serde_json::from_slice(&received[0].body).unwrap();
    let query = body["query"].as_str().unwrap();
    assert!(query.trim_start().starts_with("mutation"), "{query}");
    assert!(query.contains("signSale"), "{query}");
}

#[tokio::test]
async fn faq_q509_the_user_total_leaves_the_rounding_out() {
    let app = app().await;
    let order = open(&app, None);
    add(&app, order, 1, 2);
    // 6,08 paid cash: 6,10 in the drawer, rounding +0,02.
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);

    let report = do_generate_user_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(report.success, "{:?}", report.error);
    let request = requests(&app).pop().unwrap();
    let users = request["users"].as_array().expect("users");
    let me = users
        .iter()
        .find(|u| u["employeeId"] == MANAGER)
        .expect("the operator's totals");
    assert_eq!(me["totalAmount"].as_f64(), Some(6.08));
}
