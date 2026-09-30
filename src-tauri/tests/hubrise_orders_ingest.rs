//! Online orders from the mock reach the board resolved against the
//! menu, acknowledged, and never among the POS's tables and takeaways.
//! Nothing is signed at this stage.

#![cfg(feature = "hubrise")]

mod common;

use common::TestApp;
use common::hubrise::{POS_TOKEN, connected_app, poller};
use pili_pos::commands::hubrise as cmd;
use pili_pos::commands::order::{CreateOrderInput, do_create_order, do_list_open_orders};
use pili_pos::commands::tables::do_list_open_takeaways;
use pos_core::order::OrderType;
use pos_hubrise::poller::{poll_once, start};
use pos_hubrise_mock::builders::{FOOD, OrderBuilder, seed_catalog_builder, template};

async fn started() -> (TestApp, tempfile::TempDir, common::hubrise::HubriseMock) {
    let (app, dir, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    (app, dir, mock)
}

fn board(app: &TestApp) -> Vec<cmd::WebshopOrderSummary> {
    cmd::do_list_webshop_orders(&app.state).unwrap()
}

fn fiscal_events(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
        .unwrap()
}

#[tokio::test]
async fn every_template_reaches_the_board_resolved_and_unsigned() {
    let (app, _db, mock) = started().await;
    for name in pos_hubrise_mock::builders::TEMPLATES {
        mock.inject(name);
    }
    poll_once(&poller(&app)).await.unwrap();

    let orders = board(&app);
    assert_eq!(orders.len(), pos_hubrise_mock::builders::TEMPLATES.len());
    let by_source = |source: &str, service: &str| {
        orders
            .iter()
            .find(|o| o.source == source && o.service_type.as_deref() == Some(service))
            .and_then(|o| o.webshop.clone())
            .unwrap_or_else(|| panic!("{source} {service}"))
    };

    let deliveroo = by_source("Deliveroo", "delivery");
    assert_eq!(deliveroo.resolution, "ok", "{:?}", deliveroo.attention);
    assert_eq!(deliveroo.payment_mode.as_deref(), Some("prepaid"));
    assert_eq!(deliveroo.payment_type.as_deref(), Some("APP"));
    assert_eq!(deliveroo.fiscal_state, "pending");

    let cash = by_source("Website", "delivery");
    assert_eq!(cash.resolution, "ok", "{:?}", cash.attention);
    assert_eq!(cash.payment_mode.as_deref(), Some("unpaid"));
    assert_eq!(cash.payment_type, None);

    let qr = by_source("Table QR", "eat_in");
    assert_eq!(qr.resolution, "needs_review");

    assert_eq!(fiscal_events(&app), 0, "nothing is signed on ingest");
}

#[tokio::test]
async fn online_orders_stay_out_of_tables_and_takeaways() {
    let (app, _db, mock) = started().await;
    mock.inject("webshop_cash_collection");
    mock.inject("eat_in");
    poll_once(&poller(&app)).await.unwrap();
    // A POS takeaway, to show the lists still work.
    app.login_as("12345678958", pos_core::user::UserRole::Admin);
    let local = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: "12345678958".into(),
        },
    )
    .unwrap();

    let open: Vec<i64> = do_list_open_orders(&app.state)
        .unwrap()
        .into_iter()
        .map(|o| o.id)
        .collect();
    assert_eq!(open, vec![local.id]);
    assert!(do_list_open_takeaways(&app.state).unwrap().is_empty());
    assert_eq!(board(&app).len(), 2);
}

#[tokio::test]
async fn a_new_order_is_acknowledged_once() {
    let (app, _db, mock) = started().await;
    let order = mock.inject("ubereats_prepaid");
    let id = order["id"].as_str().unwrap().to_owned();
    let ctx = poller(&app);
    poll_once(&ctx).await.unwrap();
    common::hubrise::drain_outbox(&app).await;
    assert_eq!(mock.order(&id)["status"], "received");
}

#[tokio::test]
async fn a_platform_cancelling_before_acceptance_voids_it() {
    let (app, _db, mock) = started().await;
    let order = mock.inject("deliveroo_prepaid");
    let id = order["id"].as_str().unwrap().to_owned();
    let ctx = poller(&app);
    poll_once(&ctx).await.unwrap();
    mock.update(&id, serde_json::json!({ "status": "cancelled" }));
    poll_once(&ctx).await.unwrap();

    let entry = board(&app)
        .into_iter()
        .find(|o| o.hubrise_order_id.as_deref() == Some(id.as_str()))
        .unwrap();
    assert_eq!(entry.status, "voided");
    assert_eq!(entry.webshop.unwrap().fiscal_state, "voided");
    assert_eq!(fiscal_events(&app), 0);
}

#[tokio::test]
async fn an_order_ahead_of_the_catalog_triggers_a_catalog_refresh() {
    let (app, _db, mock) = started().await;
    {
        // The catalog gains a product without an event to the POS.
        let mut inner = mock.state.inner.lock().unwrap();
        let catalog_id = inner.default_catalog_id.clone();
        let data = seed_catalog_builder()
            .product(
                "GYOZA",
                "MAKI",
                "Gyoza",
                FOOD,
                &[("GYOZA-6", None, 550, &[])],
            )
            .build();
        inner.replace_catalog(POS_TOKEN, &catalog_id, None, data);
    }
    // The last sync is older than the one-minute debounce.
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE hubrise_connection SET last_catalog_sync_at = datetime('now', '-2 minutes')",
            [],
        )
        .unwrap();
    mock.inject_body(
        OrderBuilder::new("Deliveroo", "delivery")
            .item("GYOZA-6", "Gyoza", 550, 1)
            .paid_by("DELIVEROO", "Deliveroo")
            .build(),
    );
    poll_once(&poller(&app)).await.unwrap();

    let state = board(&app)[0].webshop.clone().unwrap();
    assert_eq!(state.resolution, "ok", "{:?}", state.attention);
    assert_eq!(state.lines[0].name, "Gyoza");
}

#[tokio::test]
async fn the_catalog_refresh_waits_a_minute_between_tries() {
    let (app, _db, mock) = started().await;
    mock.inject_body(
        OrderBuilder::new("Deliveroo", "delivery")
            .item("NOPE-1", "Inconnu", 100, 1)
            .paid_by("DELIVEROO", "Deliveroo")
            .build(),
    );
    let reads = mock.requests("GET", "/v1/catalogs/").len();
    poll_once(&poller(&app)).await.unwrap();
    poll_once(&poller(&app)).await.unwrap();
    assert_eq!(
        mock.requests("GET", "/v1/catalogs/").len(),
        reads,
        "synced at start less than a minute ago"
    );
    let state = board(&app)[0].webshop.clone().unwrap();
    assert_eq!(state.resolution, "needs_review");
}

#[tokio::test]
async fn mapping_a_channel_releases_its_orders() {
    let (app, _db, mock) = started().await;
    mock.inject("eat_in");
    poll_once(&poller(&app)).await.unwrap();
    let fixed = cmd::do_set_channel(
        &app.state,
        &pos_hubrise::webshop::Channel {
            channel: "Table QR".into(),
            kind: "platform".into(),
            payment_type: "APP".into(),
            charge_policy: "include".into(),
        },
    )
    .unwrap();
    assert_eq!(fixed, 1);
    let state = board(&app)[0].webshop.clone().unwrap();
    assert_eq!(
        (state.resolution.as_str(), state.payment_type.as_deref()),
        ("ok", Some("APP"))
    );
    assert!(
        cmd::do_list_channels(&app.state)
            .unwrap()
            .iter()
            .any(|c| c.channel == "table qr")
    );
}

#[tokio::test]
async fn channel_settings_need_an_admin_and_valid_values() {
    let (app, _db, _mock) = started().await;
    let mut bad = pos_hubrise::webshop::Channel {
        channel: "X".into(),
        kind: "shop".into(),
        payment_type: "APP".into(),
        charge_policy: "include".into(),
    };
    assert_eq!(
        cmd::do_set_channel(&app.state, &bad).unwrap_err(),
        pili_pos::commands::errors::hubrise::CHANNEL_INVALID
    );
    bad.kind = "platform".into();
    app.login_as("12345678958", pos_core::user::UserRole::Cashier);
    assert!(cmd::do_set_channel(&app.state, &bad).is_err());
    assert!(cmd::do_list_channels(&app.state).is_err());
}

#[tokio::test]
async fn the_board_keeps_the_order_details() {
    let (app, _db, mock) = started().await;
    mock.inject_body(template("webshop_cash_delivery").unwrap().build());
    poll_once(&poller(&app)).await.unwrap();
    let order = &board(&app)[0];
    assert_eq!(order.customer_city.as_deref(), Some("Liège"));
    let state = order.webshop.clone().unwrap();
    assert_eq!(state.lines.len(), 3, "two products and the delivery charge");
    assert_eq!(order.total_cents, 1600);
}
