//! Sold-out products pushed to the `HubRise` inventory (V11-D15), against the in-process mock.

#![cfg(feature = "hubrise")]
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::TestApp;
use common::hubrise::{HubriseMock, connected_app, drain_inventory, inventory_due_now, poller};
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::errors;
use pos_engine::commands::modifiers::{AddWithModifiersInput, do_add_order_line_with_modifiers};
use pos_engine::commands::order::{CreateOrderInput, do_create_order};
use pos_engine::commands::sold_out::{do_clear_sold_out, do_set_sold_out};
use pos_hubrise::poller::start;
use serde_json::{Value, json};

const INSZ: &str = "12345678958";
const PATH: &str = "/v1/catalogs/pili-dev-cat-1/location/inventory";

async fn started() -> (TestApp, tempfile::TempDir, HubriseMock) {
    let (app, dir, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    app.login_as(INSZ, UserRole::Cashier);
    (app, dir, mock)
}

fn item(app: &TestApp, sku: &str) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM menu_items WHERE external_id = ?1",
            [sku],
            |r| r.get(0),
        )
        .unwrap()
}

fn patches(mock: &HubriseMock) -> Vec<Value> {
    mock.requests("PATCH", PATH)
        .into_iter()
        .map(|r| r.body.unwrap())
        .collect()
}

fn state_of(app: &TestApp, id: i64) -> (i64, i64, i64, Option<String>) {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT dirty, attempts, parked_at IS NOT NULL, error_msg
             FROM hubrise_inventory_sync WHERE menu_item_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap()
}

fn end_of_day(body: &Value) -> String {
    body[0]["expires_at"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn sold_out_for_the_day_sends_zero_stock_with_the_end_of_the_day() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, true, None).unwrap();

    assert_eq!(drain_inventory(&app).await.sent, 1);
    let sent = patches(&mock);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0][0]["sku_ref"], "CALIFORNIA-8");
    assert_eq!(sent[0][0]["stock"], "0");
    // The end of the business day, with the Brussels offset.
    let expires = end_of_day(&sent[0]);
    assert!(
        expires.ends_with("+02:00") || expires.ends_with("+01:00"),
        "{expires}"
    );
    assert!(chrono::DateTime::parse_from_rfc3339(&expires).is_ok());
    assert_eq!(state_of(&app, id).0, 0, "no longer dirty");

    // HubRise holds the entry.
    let held = mock
        .state
        .inner
        .lock()
        .unwrap()
        .inventory_of(Some("pili-dev-cat-1"));
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].stock.as_deref(), Some("0"));

    // Nothing more to send.
    assert_eq!(drain_inventory(&app).await.sent, 0);
    assert_eq!(patches(&mock).len(), 1);
}

#[tokio::test]
async fn sold_out_until_re_enabled_has_no_expiry_and_clearing_removes_the_entry() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, false, None).unwrap();
    drain_inventory(&app).await;
    assert_eq!(
        patches(&mock)[0],
        json!([{"sku_ref": "CALIFORNIA-8", "stock": "0"}])
    );

    do_clear_sold_out(&app.state, id, None).unwrap();
    drain_inventory(&app).await;
    let sent = patches(&mock);
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[1], json!([{"sku_ref": "CALIFORNIA-8", "stock": null}]));
    assert!(
        mock.state
            .inner
            .lock()
            .unwrap()
            .inventory_of(Some("pili-dev-cat-1"))
            .is_empty()
    );
}

#[tokio::test]
async fn several_changes_before_a_send_become_one_request_with_the_current_state() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, true, None).unwrap();
    do_clear_sold_out(&app.state, id, None).unwrap();
    do_set_sold_out(&app.state, id, false, None).unwrap();

    assert_eq!(drain_inventory(&app).await.sent, 1);
    let sent = patches(&mock);
    assert_eq!(sent.len(), 1, "one request, not three");
    assert_eq!(sent[0], json!([{"sku_ref": "CALIFORNIA-8", "stock": "0"}]));

    // Set then cleared while offline: the current state is "available".
    do_set_sold_out(&app.state, id, true, None).unwrap();
    do_clear_sold_out(&app.state, id, None).unwrap();
    drain_inventory(&app).await;
    assert_eq!(
        patches(&mock)[1],
        json!([{"sku_ref": "CALIFORNIA-8", "stock": null}])
    );
}

#[tokio::test]
async fn a_transient_failure_is_retried_with_backoff_and_then_sent() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, false, None).unwrap();
    mock.fault("PATCH", PATH, 503, Some(1));

    let pass = drain_inventory(&app).await;
    assert_eq!((pass.sent, pass.retried), (0, 1));
    let (dirty, attempts, parked, error) = state_of(&app, id);
    assert_eq!((dirty, attempts, parked), (1, 1, 0));
    assert!(error.unwrap().contains("503"));

    // Backing off: not due yet.
    assert_eq!(drain_inventory(&app).await.retried, 0);
    assert_eq!(mock.requests("PATCH", PATH).len(), 1);

    inventory_due_now(&app);
    assert_eq!(drain_inventory(&app).await.sent, 1);
    let (dirty, attempts, _, error) = state_of(&app, id);
    assert_eq!((dirty, attempts, error), (0, 0, None));
}

#[tokio::test]
async fn a_refusal_parks_the_row_with_its_reason_and_a_new_change_requeues_it() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, false, None).unwrap();
    mock.fault("PATCH", PATH, 403, Some(1));

    let pass = drain_inventory(&app).await;
    assert_eq!(pass.parked, 1);
    let (dirty, _, parked, error) = state_of(&app, id);
    assert_eq!((dirty, parked), (1, 1));
    assert!(error.unwrap().contains("403"));
    assert_eq!(drain_inventory(&app).await.sent, 0, "parked rows wait");
    assert_eq!(mock.requests("PATCH", PATH).len(), 1);

    // The next change of the product puts it back in the queue.
    do_clear_sold_out(&app.state, id, None).unwrap();
    assert_eq!(drain_inventory(&app).await.sent, 1);
    assert_eq!(state_of(&app, id).2, 0);

    // An operator's retry re-queues a parked row too.
    do_set_sold_out(&app.state, id, false, None).unwrap();
    mock.fault("PATCH", PATH, 422, Some(1));
    assert_eq!(drain_inventory(&app).await.parked, 1);
    assert_eq!(
        pos_hubrise::inventory::backlog(&app.state.db.lock().unwrap())
            .unwrap()
            .parked,
        1
    );
    pos_hubrise::inventory::retry_parked(&app.state.db.lock().unwrap()).unwrap();
    assert_eq!(drain_inventory(&app).await.sent, 1);
}

#[tokio::test]
async fn a_refused_token_stops_the_pass_without_counting_an_attempt() {
    let (app, _dir, mock) = started().await;
    let a = item(&app, "CALIFORNIA-8");
    let b = item(&app, "COCA-33");
    do_set_sold_out(&app.state, a, false, None).unwrap();
    do_set_sold_out(&app.state, b, false, None).unwrap();
    mock.fault("PATCH", PATH, 401, Some(1));

    let pass = drain_inventory(&app).await;
    assert!(pass.unauthorized);
    assert_eq!(mock.requests("PATCH", PATH).len(), 1, "the pass stopped");
    assert_eq!(state_of(&app, a).1, 0);
    assert_eq!(state_of(&app, b).0, 1);
    let health = pos_hubrise::token_store::health(&app.state.db.lock().unwrap()).unwrap();
    assert!(health.auth_error_at.is_some());
}

#[tokio::test]
async fn with_hubrise_off_nothing_is_queued_and_switching_on_replays_nothing() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    app.state.hubrise_switch.set(false);
    do_set_sold_out(&app.state, id, true, None).unwrap();
    app.state.hubrise_switch.set(true);

    assert_eq!(drain_inventory(&app).await.sent, 0);
    assert!(mock.requests("PATCH", PATH).is_empty());
    let queued: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM hubrise_inventory_sync", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(queued, 0);
    // The till still refuses the product locally.
    assert!(
        pos_engine::commands::menu::do_get_menu_items(&app.state, None, None)
            .unwrap()
            .iter()
            .any(|m| m.id == id && m.sold_out)
    );
}

#[tokio::test]
async fn a_local_product_without_a_sku_sends_nothing() {
    let (app, _dir, mock) = started().await;
    {
        let db = app.state.db.lock().unwrap();
        let cat: i64 = db
            .query_row("SELECT id FROM categories LIMIT 1", [], |r| r.get(0))
            .unwrap();
        db.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
             VALUES (9001, ?1, 500, 'B', 99, 1)",
            [cat],
        )
        .unwrap();
    }
    do_set_sold_out(&app.state, 9001, true, None).unwrap();
    assert_eq!(drain_inventory(&app).await.sent, 0);
    assert!(mock.requests("PATCH", PATH).is_empty());
    // It is sold out locally all the same.
    assert!(
        pos_engine::commands::menu::do_get_menu_items(&app.state, None, None)
            .unwrap()
            .iter()
            .any(|m| m.id == 9001 && m.sold_out)
    );
}

#[tokio::test]
async fn the_order_outbox_pass_leaves_the_inventory_to_its_own_pass() {
    let (app, _dir, mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, false, None).unwrap();
    // The regular outbox pass leaves the inventory to its own pass: both run in `run_loop`.
    pos_hubrise::outbox::process_pending(&app.state.shared_http, &app.state.poller_db)
        .await
        .unwrap();
    assert!(mock.requests("PATCH", PATH).is_empty());
    drain_inventory(&app).await;
    assert_eq!(mock.requests("PATCH", PATH).len(), 1);
}

#[tokio::test]
async fn an_option_product_is_refused_too() {
    let (app, _dir, _mock) = started().await;
    let id = item(&app, "CALIFORNIA-8");
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(2),
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
    .id;
    do_set_sold_out(&app.state, id, true, None).unwrap();
    let err = do_add_order_line_with_modifiers(
        &app.state,
        &AddWithModifiersInput {
            order_id: order,
            menu_item_id: id,
            quantity: 1,
            modifier_ids: vec![],
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        errors::coded(errors::PRODUCT_SOLD_OUT, &[("product", &id)])
    );
    do_clear_sold_out(&app.state, id, None).unwrap();
    assert!(
        do_add_order_line_with_modifiers(
            &app.state,
            &AddWithModifiersInput {
                order_id: order,
                menu_item_id: id,
                quantity: 1,
                modifier_ids: vec![],
            },
        )
        .is_ok()
    );
}

#[tokio::test]
async fn an_online_order_for_a_sold_out_product_is_still_taken_and_signed() {
    // V11-D14: the sold-out refusal belongs to the till (order.rs, modifiers.rs); an online
    // order was already sold and paid on the platform, so it goes through unchanged.
    let (app, _dir, mock) = started().await;
    app.use_admission_mock().await;
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, true, None).unwrap();
    // The till itself refuses it.
    let table = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(4),
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
    .id;
    assert!(
        pos_engine::commands::order::do_add_order_line(
            &app.state,
            pos_engine::commands::order::AddOrderLineInput {
                order_id: table,
                menu_item_id: id,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .is_err()
    );

    let hr = mock.inject_body(
        pos_hubrise_mock::builders::OrderBuilder::new("Deliveroo", "delivery")
            .item("CALIFORNIA-8", "California 8", 800, 2)
            .paid_by("DELIVEROO", "Deliveroo")
            .build(),
    );
    let hr_id = hr["id"].as_str().unwrap().to_owned();
    pos_hubrise::poller::poll_once(&poller(&app)).await.unwrap();
    let (order, total): (i64, i64) = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id, total_cents FROM orders WHERE hubrise_order_id = ?1",
            [&hr_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("the online order was created");
    assert_eq!(total, 1600);
    let sale = pos_engine::commands::webshop::do_accept_webshop_order(&app.state, order, None)
        .await
        .unwrap();
    let lines: Vec<(i64, i64)> = app
        .state
        .db
        .lock()
        .unwrap()
        .prepare("SELECT menu_item_id, quantity FROM order_lines WHERE order_id = ?1")
        .unwrap()
        .query_map([order], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(lines, [(id, 2)], "the sold-out product is on the order");

    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == sale.fiscal_event_id)
        .unwrap();
    assert_eq!(event.event_type, "N");
    assert!(event.fdm_id.is_some(), "signed by the FDM");

    // The sold-out state of the product is untouched.
    assert!(
        pos_engine::commands::menu::do_get_menu_items(&app.state, None, None)
            .unwrap()
            .iter()
            .any(|m| m.id == id && m.sold_out)
    );
}

#[tokio::test]
async fn the_outbox_loop_pushes_the_inventory_by_itself() {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let (app, _dir, mock) = started().await;
    let cancel = tokio_util::sync::CancellationToken::new();
    let ctx = pos_hubrise::outbox::OutboxCtx {
        http: app.state.shared_http.clone(),
        db: Arc::clone(&app.state.poller_db),
        wake: Arc::clone(&app.state.hubrise_outbox_wake),
        enabled: app.state.hubrise_switch.clone(),
    };
    let task = tokio::spawn(pos_hubrise::outbox::run_loop(ctx, cancel.clone()));

    // Marking the product wakes the loop; no pass is called by the test.
    let id = item(&app, "CALIFORNIA-8");
    do_set_sold_out(&app.state, id, false, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while mock.requests("PATCH", PATH).is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(
        patches(&mock),
        [json!([{"sku_ref": "CALIFORNIA-8", "stock": "0"}])]
    );
    assert_eq!(state_of(&app, id).0, 0, "no longer dirty");

    // With HubRise switched off the loop sends nothing.
    app.state.hubrise_switch.set(false);
    do_clear_sold_out(&app.state, id, None).unwrap();
    app.state.hubrise_outbox_wake.notify_one();
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(patches(&mock).len(), 1);

    cancel.cancel();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("the loop stops with its token")
        .unwrap();
}
