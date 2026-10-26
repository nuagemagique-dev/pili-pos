//! A refund pays money out: whatever stops it must leave nothing a
//! cashier could complete later.
//!
//! - every item is checked before the refund order exists, so a
//!   refused refund creates no order;
//! - an FDM refusal voids the refund order: it never lingers open, to
//!   be topped up with positive lines and paid as a sale;
//! - a refund is never refunded, and a real sale is never refunded in
//!   training mode (it would eat into what the sale may still refund).
//!
//! Events are signed by the admission image's mock FDM.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_list_open_orders,
};
use pos_engine::commands::refund::{
    ERR_REFUND_IN_TRAINING, ERR_REFUND_OF_REFUND, RefundItem, do_refund_checkout,
};

/// A manager (refunds are theirs), with a well-formed INSZ.
const OPERATOR: &str = "12345678958";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
            (1, 1, 1200, 'A', 0, 1),
            (2, 1, 2800, 'B', 1, 1);",
    )
    .unwrap();
    drop(conn);
    app
}

/// 2 x item 1 + 1 x item 2, paid by card; returns the N event id.
async fn sale(app: &TestApp) -> i64 {
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
    for (item, qty) in [(1, 2), (2, 1)] {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: item,
                quantity: qty,
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
    r.fiscal_event_id.unwrap()
}

fn items(list: &[(i64, i32)]) -> Vec<RefundItem> {
    list.iter()
        .map(|&(menu_item_id, quantity)| RefundItem {
            line_id: None,
            menu_item_id,
            quantity,
        })
        .collect()
}

async fn refund(
    app: &TestApp,
    event: i64,
    list: &[(i64, i32)],
) -> Result<(bool, Option<String>), String> {
    do_refund_checkout(&app.state, event, items(list), OPERATOR.into())
        .await
        .map(|r| (r.success, r.error))
}

fn order_count(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0))
        .unwrap()
}

fn voided_orders(app: &TestApp) -> Vec<i64> {
    let conn = app.state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id FROM orders WHERE status = 'voided' ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[tokio::test]
async fn a_refused_item_creates_no_refund_order() {
    let app = app().await;
    let original = sale(&app).await;
    let orders = order_count(&app);
    let events = app.fiscal_events().len();

    for list in [
        vec![(1, 1), (9, 1)],  // second item not on the ticket
        vec![(1, 1), (2, 2)],  // second item: more than sold
        vec![(1, 2), (1, 1)],  // same item twice, together more than sold
        vec![(2, 1), (1, 0)],  // zero
        vec![(2, 1), (1, -1)], // negative
        // Would wrap an i32 sum below the sold cap.
        vec![(1, i32::MAX), (1, i32::MAX)],
        vec![(1, i32::MAX)],
    ] {
        assert!(refund(&app, original, &list).await.is_err(), "{list:?}");
    }
    assert_eq!(order_count(&app), orders, "no refund order left behind");
    assert!(do_list_open_orders(&app.state).unwrap().is_empty());
    assert_eq!(app.fiscal_events().len(), events);

    // Everything is still refundable.
    let (ok, err) = refund(&app, original, &[(1, 2), (2, 1)]).await.unwrap();
    assert!(ok, "{err:?}");
}

#[tokio::test]
async fn an_fdm_refusal_voids_the_refund_order() {
    let app = app().await;
    let original = sale(&app).await;
    let fdm = app.state.config.lock().unwrap().fdm_url.clone();
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();

    let (ok, err) = refund(&app, original, &[(1, 1)]).await.unwrap();
    assert!(!ok);
    assert!(err.unwrap().contains("FDM"));
    assert!(
        do_list_open_orders(&app.state).unwrap().is_empty(),
        "never left open"
    );
    let voided = voided_orders(&app);
    assert_eq!(voided.len(), 1);

    // The voided order can no longer be completed and paid.
    assert!(
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: voided[0],
                menu_item_id: 2,
                quantity: 5,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .is_err()
    );
    let paid = do_checkout(&app.state, voided[0], CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!paid.success);

    // Once the FDM is back, the refund goes through, and nothing of
    // the failed attempt counts against the sale.
    app.state.config.lock().unwrap().fdm_url = fdm;
    let (ok, err) = refund(&app, original, &[(1, 2)]).await.unwrap();
    assert!(ok, "{err:?}");
    let n: Vec<_> = app
        .fiscal_events()
        .iter()
        .filter(|e| e.event_type == "N")
        .map(|e| e.order_id)
        .collect();
    assert_eq!(n.len(), 2, "the sale and one refund");
}

#[tokio::test]
async fn a_refused_ticket_medium_creates_no_refund_order() {
    let app = app().await;
    let original = sale(&app).await;
    let orders = order_count(&app);
    let request = pos_engine::commands::delivery::TicketDeliveryRequest::email(
        pos_engine::commands::delivery::RequestedMedium::Digital,
        "client@example.com",
    );
    // Email is not configured on this terminal.
    let r = pos_engine::commands::refund::do_refund_checkout_with(
        &app.state,
        original,
        items(&[(1, 1)]),
        OPERATOR.into(),
        Some(&request),
    )
    .await
    .unwrap();
    assert!(!r.success);
    assert_eq!(order_count(&app), orders);
}

#[tokio::test]
async fn a_refund_is_never_refunded() {
    let app = app().await;
    let original = sale(&app).await;
    let (ok, err) = refund(&app, original, &[(1, 1)]).await.unwrap();
    assert!(ok, "{err:?}");
    let refund_event = app.fiscal_events().last().unwrap().id;
    let orders = order_count(&app);

    let (ok, err) = refund(&app, refund_event, &[(1, 1)]).await.unwrap();
    assert!(!ok);
    assert_eq!(err.as_deref(), Some(ERR_REFUND_OF_REFUND));
    assert_eq!(order_count(&app), orders);
}

#[tokio::test]
async fn a_real_sale_is_not_refunded_in_training_mode() {
    let app = app().await;
    let original = sale(&app).await;
    app.state.config.lock().unwrap().training_mode = true;
    let (ok, err) = refund(&app, original, &[(1, 2), (2, 1)]).await.unwrap();
    assert!(!ok);
    assert_eq!(err.as_deref(), Some(ERR_REFUND_IN_TRAINING));

    app.state.config.lock().unwrap().training_mode = false;
    let (ok, err) = refund(&app, original, &[(1, 2), (2, 1)]).await.unwrap();
    assert!(ok, "the whole sale is still refundable: {err:?}");
}

/// One product sold at 1.00 and at 100.00 on the same ticket (its
/// price changed between the two lines).
async fn sale_at_two_prices(app: &TestApp) -> (i64, i64, i64) {
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
    let mut lines = Vec::new();
    for price in [100, 10_000] {
        app.state
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE menu_items SET price_cents = ?1 WHERE id = 1",
                [price],
            )
            .unwrap();
        lines.push(
            do_add_order_line(
                &app.state,
                AddOrderLineInput {
                    order_id: order,
                    menu_item_id: 1,
                    quantity: 1,
                    modifiers_json: None,
                    unit_price_cents: None,
                },
            )
            .unwrap()
            .id,
        );
    }
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    (r.fiscal_event_id.unwrap(), lines[0], lines[1])
}

async fn refund_line(
    app: &TestApp,
    event: i64,
    line_id: Option<i64>,
    quantity: i32,
) -> Result<(bool, Option<String>), String> {
    let items = vec![RefundItem {
        line_id,
        menu_item_id: 1,
        quantity,
    }];
    do_refund_checkout(&app.state, event, items, OPERATOR.into())
        .await
        .map(|r| (r.success, r.error))
}

fn last_refund_total(app: &TestApp) -> f64 {
    let event = app.fiscal_events().pop().unwrap();
    let request: serde_json::Value =
        serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    request["transaction"]["transactionTotal"].as_f64().unwrap()
}

#[tokio::test]
async fn a_refund_is_priced_from_the_line_it_names() {
    let app = app().await;
    let (original, cheap, dear) = sale_at_two_prices(&app).await;

    let ambiguous = refund_line(&app, original, None, 1).await.unwrap_err();
    assert!(
        ambiguous.starts_with("refund_line_ambiguous"),
        "{ambiguous}"
    );

    assert_eq!(
        refund_line(&app, original, Some(cheap), 1).await,
        Ok((true, None))
    );
    assert_eq!(last_refund_total(&app), -1.0, "the 1.00 line, not 100.00");

    // The cheap line is used up: it cannot be refunded again, at
    // either price.
    let again = refund_line(&app, original, Some(cheap), 1)
        .await
        .unwrap_err();
    assert!(again.starts_with("refund_exceeds_sold"), "{again}");

    let twice = refund_line(&app, original, Some(dear), 2)
        .await
        .unwrap_err();
    assert!(twice.starts_with("refund_exceeds_sold"), "{twice}");
    assert_eq!(
        refund_line(&app, original, Some(dear), 1).await,
        Ok((true, None))
    );
    assert_eq!(last_refund_total(&app), -100.0);
}

#[tokio::test]
async fn a_line_of_another_sale_is_refused() {
    let app = app().await;
    let (original, _, _) = sale_at_two_prices(&app).await;
    let (_, foreign, _) = sale_at_two_prices(&app).await;

    let err = refund_line(&app, original, Some(foreign), 1)
        .await
        .unwrap_err();
    assert!(err.starts_with("refund_product_not_sold"), "{err}");
}

/// A sale record that cannot be read must stop the refund: read as an
/// empty record it would look paid.
#[tokio::test]
async fn an_unreadable_sale_record_stops_the_refund() {
    let app = app().await;
    let original = sale(&app).await;
    {
        let conn = app.state.db.lock().unwrap();
        // Simulates on-disk corruption; the trigger is the only guard.
        conn.execute_batch("DROP TRIGGER fiscal_events_no_rewrite;")
            .unwrap();
        conn.execute(
            "UPDATE fiscal_events SET event_data_json = '{broken' WHERE id = ?1",
            [original],
        )
        .unwrap();
    }
    let orders = order_count(&app);
    assert!(refund(&app, original, &[(1, 1)]).await.is_err());
    assert_eq!(order_count(&app), orders, "no refund order left behind");
}
