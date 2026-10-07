//! The installation's HubRise switch. Off: nothing reaches HubRise and
//! no new online order is taken, but an order already signed is always
//! settled. Admins switch it; every change is logged; off waits until
//! the online orders are settled.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::TestApp;
use common::hubrise::{connected_app, drain_outbox, poller};
use pos_core::user::UserRole;
use pos_engine::commands::checkout::PaymentPart;
use pos_engine::commands::errors;
use pos_engine::commands::integrations::{do_get_integrations, do_set_hubrise_enabled};
use pos_engine::commands::webshop::{do_accept_webshop_order, do_handover_webshop_order};
use pos_hubrise::poller::{poll_once, start};

const CASHIER: &str = "12345678958";
const MANAGER: &str = "85073003328";

async fn app() -> (TestApp, tempfile::TempDir, common::hubrise::HubriseMock) {
    let (app, db, mock) = connected_app().await;
    app.use_admission_mock().await;
    app.login_as(CASHIER, UserRole::Cashier);
    (app, db, mock)
}

fn order_id(app: &TestApp, hubrise_id: &str) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM orders WHERE hubrise_order_id = ?1",
            [hubrise_id],
            |r| r.get(0),
        )
        .unwrap()
}

/// A soup taken away, paid cash at the till.
async fn till_sale(app: &TestApp) -> pos_engine::commands::checkout::CheckoutResult {
    use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
    use pos_engine::commands::order::{
        AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
    };
    let miso: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM menu_items WHERE external_id = 'MISO-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: pos_core::order::OrderType::Takeaway,
            table_number: None,
            user_insz: CASHIER.into(),
        },
    )
    .unwrap();
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order.id,
            menu_item_id: miso,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    do_checkout(&app.state, order.id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap()
}

#[tokio::test]
async fn off_nothing_reaches_hubrise() {
    let (app, _db, mock) = app().await;
    app.state.hubrise_switch.set(false);
    mock.inject("deliveroo_prepaid");
    start(&poller(&app)).await.unwrap();
    poll_once(&poller(&app)).await.unwrap();
    assert!(mock.requests("GET", "/").is_empty(), "no call at all");
    assert!(mock.requests("POST", "/").is_empty());
    let known: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM webshop_orders", [], |r| r.get(0))
        .unwrap();
    assert_eq!(known, 0, "no order taken");
}

#[tokio::test]
async fn off_no_new_order_is_accepted_and_no_till_sale_is_sent() {
    let (app, _db, mock) = app().await;
    let hr = mock.inject("deliveroo_prepaid");
    start(&poller(&app)).await.unwrap();
    let id = order_id(&app, hr["id"].as_str().unwrap());
    app.state.hubrise_switch.set(false);
    let refused = do_accept_webshop_order(&app.state, id, None)
        .await
        .unwrap_err();
    assert_eq!(refused, errors::hubrise::DISABLED);
    assert!(app.fiscal_events().is_empty());

    // A till sale is not echoed to HubRise either.
    let sale = till_sale(&app).await;
    assert!(sale.success, "{:?}", sale.error);
    let queued: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM hubrise_outbox WHERE kind = 'create_order'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(queued, 0);
}

#[tokio::test]
async fn an_order_already_signed_is_settled_with_the_switch_off() {
    let (app, _db, mock) = app().await;
    let hr = mock.inject("webshop_cash_collection");
    start(&poller(&app)).await.unwrap();
    let id = order_id(&app, hr["id"].as_str().unwrap());
    let p = do_accept_webshop_order(&app.state, id, None).await.unwrap();
    assert_eq!(p.fiscal_state, "signed_p");

    // Switched off in pos.json meanwhile: the cash still closes it.
    app.state.hubrise_switch.set(false);
    let n = do_handover_webshop_order(&app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(n.success, "{:?}", n.error);
    let labels: Vec<_> = app
        .fiscal_events()
        .into_iter()
        .map(|e| e.event_type)
        .collect();
    assert_eq!(labels, vec!["P", "N"]);
    // HubRise hears of it once switched on again.
    drain_outbox(&app).await;
}

#[tokio::test]
async fn admins_switch_it_and_every_change_is_logged() {
    let (app, _db, _mock) = app().await;
    let refused = do_set_hubrise_enabled(&app.state, false).unwrap_err();
    assert!(!refused.is_empty(), "a cashier cannot");
    assert!(app.state.hubrise_switch.is_on());

    app.login_as(MANAGER, UserRole::Admin);
    {
        // The installation's starting value, as at start-up.
        let db = app.state.db.lock().unwrap();
        pos_db::queries::integrations::hubrise_at_start(&db, true).unwrap();
    }
    let off = do_set_hubrise_enabled(&app.state, false).unwrap();
    assert!(!off.hubrise_enabled);
    assert!(!app.state.hubrise_switch.is_on());
    let on = do_set_hubrise_enabled(&app.state, true).unwrap();
    assert!(on.hubrise_enabled);
    let log: Vec<_> = on
        .hubrise_log
        .iter()
        .map(|c| (c.enabled, c.source.as_str(), c.changed_by.as_deref()))
        .collect();
    assert_eq!(
        log,
        vec![
            (true, "admin", Some(MANAGER)),
            (false, "admin", Some(MANAGER)),
            (true, "config", None),
        ]
    );
}

#[tokio::test]
async fn off_waits_until_the_online_orders_are_settled() {
    let (app, _db, mock) = app().await;
    let hr = mock.inject("deliveroo_prepaid");
    start(&poller(&app)).await.unwrap();
    let id = order_id(&app, hr["id"].as_str().unwrap());
    app.login_as(MANAGER, UserRole::Admin);
    {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::integrations::hubrise_at_start(&db, true).unwrap();
    }
    let refused = do_set_hubrise_enabled(&app.state, false).unwrap_err();
    assert!(
        refused.starts_with(errors::hubrise::ONLINE_ORDERS_OPEN),
        "{refused}"
    );
    assert_eq!(
        do_get_integrations(&app.state).unwrap().online_orders_open,
        1
    );

    // Answered (signed N): settled, off is allowed.
    app.login_as(CASHIER, UserRole::Cashier);
    do_accept_webshop_order(&app.state, id, None).await.unwrap();
    app.login_as(MANAGER, UserRole::Admin);
    assert!(
        !do_set_hubrise_enabled(&app.state, false)
            .unwrap()
            .hubrise_enabled
    );
}
