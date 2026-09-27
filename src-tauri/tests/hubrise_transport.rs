//! HubRise transport against the in-process mock: callback
//! registration, event polling, acknowledgements, the outbox, errors.

#![cfg(feature = "hubrise")]

mod common;

use common::TestApp;
use common::hubrise::{
    HubriseMock, POS_TOKEN, connect, connected_app, drain_outbox, outbox_due_now, poller,
};
use pili_pos::commands::hubrise as cmd;
use pos_hubrise::poller::{poll_once, start};
use serde_json::json;

fn local_orders(app: &TestApp) -> Vec<(String, String)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT hubrise_order_id, hubrise_status FROM orders WHERE hubrise_order_id IS NOT NULL ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

#[tokio::test]
async fn the_ui_sees_what_the_poller_connection_writes() {
    let (app, _db) = TestApp::with_shared_db().await;
    let mock = HubriseMock::start().await;
    connect(&app, &mock);
    let db = app.state.db.lock().unwrap();
    let stored = pos_hubrise::token_store::load(&db)
        .unwrap()
        .expect("connected");
    assert_eq!(stored.base_url, mock.api());
}

#[tokio::test]
async fn start_registers_the_passive_callback() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    let cb = mock.state.inner.lock().unwrap().clients[POS_TOKEN]
        .registered_callback
        .clone()
        .expect("registered");
    assert!(cb.url.is_none(), "passive");
    assert!(pos_hubrise::callbacks::covers(&cb));
    let status = cmd::do_hubrise_status(&app.state).unwrap();
    assert!(status.health.callback_registered_at.is_some());

    // Already registered: the next start does not register again.
    start(&poller(&app)).await.unwrap();
    assert_eq!(mock.requests("POST", "/v1/callback").len(), 1);
}

#[tokio::test]
async fn without_the_callback_no_event_arrives_but_start_backfills() {
    let (app, _db, mock) = connected_app().await;
    let order = mock.inject("deliveroo_prepaid");
    let report = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(report.events, 0, "not registered: HubRise recorded nothing");
    assert!(local_orders(&app).is_empty());

    // Start fetches the orders HubRise holds as new.
    start(&poller(&app)).await.unwrap();
    assert_eq!(
        local_orders(&app),
        vec![(order["id"].as_str().unwrap().to_string(), "new".into())]
    );
}

#[tokio::test]
async fn a_new_order_is_stored_and_acknowledged_received() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    let order = mock.inject("webshop_online");
    let id = order["id"].as_str().unwrap();

    let report = poll_once(&poller(&app)).await.unwrap();
    assert_eq!((report.events, report.handled), (1, 1));
    assert_eq!(local_orders(&app).len(), 1);
    assert_eq!(mock.pos_events(), 0, "the event was deleted on HubRise");

    let pass = drain_outbox(&app).await;
    assert_eq!(pass.sent, 1);
    let patches = mock.requests(
        "PATCH",
        &format!("/v1/locations/pili-dev-loc-1/orders/{id}"),
    );
    assert_eq!(patches.len(), 1);
    assert_eq!(patches[0].body.as_ref().unwrap()["status"], "received");
    assert_eq!(mock.order(id)["status"], "received");
}

#[tokio::test]
async fn an_event_read_twice_makes_one_order_and_one_acknowledgement() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    mock.inject("eat_in");
    // The delete fails: HubRise lists the same event again.
    mock.fault("DELETE", "/v1/callback/events", 500, Some(1));
    poll_once(&poller(&app)).await.unwrap();
    assert_eq!(mock.pos_events(), 1);
    let again = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(again.handled, 0, "already handled");
    assert_eq!(mock.pos_events(), 0, "deleted this time");
    assert_eq!(local_orders(&app).len(), 1);
    let db = app.state.db.lock().unwrap();
    let acks: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM hubrise_outbox WHERE kind = 'patch_order'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(acks, 1);
}

#[tokio::test]
async fn an_update_refreshes_the_stored_order() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    let order = mock.inject("ubereats_prepaid");
    poll_once(&poller(&app)).await.unwrap();
    mock.update(
        order["id"].as_str().unwrap(),
        json!({ "status": "cancelled" }),
    );
    let report = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(report.handled, 1);
    assert_eq!(local_orders(&app)[0].1, "cancelled");
}

#[tokio::test]
async fn a_refused_token_shows_until_a_request_succeeds() {
    let (app, _db, mock) = connected_app().await;
    mock.fault("GET", "/v1/callback/events", 401, Some(1));
    let report = poll_once(&poller(&app)).await.unwrap();
    assert!(report.unauthorized);
    let status = cmd::do_hubrise_status(&app.state).unwrap();
    assert!(status.health.auth_error_at.is_some());

    poll_once(&poller(&app)).await.unwrap();
    let status = cmd::do_hubrise_status(&app.state).unwrap();
    assert!(status.health.auth_error_at.is_none());
    assert!(status.health.last_poll_at.is_some());
}

#[tokio::test]
async fn a_failing_order_fetch_is_retried_then_parked() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    mock.inject("eat_in");
    mock.fault("GET", "/v1/locations/pili-dev-loc-1/orders/", 500, Some(1));
    let first = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(first.failed, 1);
    assert_eq!(mock.pos_events(), 1, "kept on HubRise for the retry");
    let second = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(second.handled, 1);

    // One that never works is parked after five tries, and does not
    // block the next events.
    let stuck = mock.inject("eat_in");
    let stuck_path = format!(
        "/v1/locations/pili-dev-loc-1/orders/{}",
        stuck["id"].as_str().unwrap()
    );
    mock.fault("GET", &stuck_path, 500, None);
    for _ in 0..4 {
        assert_eq!(poll_once(&poller(&app)).await.unwrap().failed, 1);
    }
    let last = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(last.parked, 1);
    assert_eq!(mock.pos_events(), 0);
    mock.inject("eat_in");
    assert_eq!(poll_once(&poller(&app)).await.unwrap().handled, 1);
    assert_eq!(cmd::do_hubrise_status(&app.state).unwrap().parked_events, 1);
}

#[tokio::test]
async fn events_of_another_catalog_or_location_are_ignored() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    {
        let mut inner = mock.state.inner.lock().unwrap();
        let data = pos_hubrise_mock::builders::seed_catalog();
        inner.replace_catalog(common::hubrise::WEBSHOP_TOKEN, "other-catalog", None, data);
    }
    // The start read the connected catalog once.
    let reads = mock.requests("GET", "/v1/catalogs/").len();
    let report = poll_once(&poller(&app)).await.unwrap();
    assert_eq!(report.handled, 1);
    assert_eq!(
        mock.requests("GET", "/v1/catalogs/").len(),
        reads,
        "not fetched"
    );
}

#[tokio::test]
async fn a_catalog_update_is_fetched() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    {
        let mut inner = mock.state.inner.lock().unwrap();
        let id = inner.default_catalog_id.clone();
        let data = pos_hubrise_mock::builders::seed_catalog();
        inner.replace_catalog(common::hubrise::WEBSHOP_TOKEN, &id, None, data);
    }
    let reads = mock.requests("GET", "/v1/catalogs/pili-dev-cat-1").len();
    poll_once(&poller(&app)).await.unwrap();
    assert_eq!(
        mock.requests("GET", "/v1/catalogs/pili-dev-cat-1").len(),
        reads + 1
    );
}

#[tokio::test]
async fn patches_of_one_order_go_in_order() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    let order = mock.inject("webshop_online");
    let id = order["id"].as_str().unwrap().to_string();
    poll_once(&poller(&app)).await.unwrap();
    app.login_as(common::TEST_OPERATOR, pos_core::user::UserRole::Admin);
    cmd::do_patch_order_status(&app.state, &id, "in_preparation").unwrap();

    // The `received` acknowledgement fails once: `in_preparation`
    // must not overtake it.
    mock.fault(
        "PATCH",
        "/v1/locations/pili-dev-loc-1/orders/",
        500,
        Some(1),
    );
    let pass = drain_outbox(&app).await;
    assert_eq!((pass.sent, pass.retried), (0, 1));
    assert_eq!(
        mock.requests("PATCH", "/v1/locations/").len(),
        1,
        "the second waited"
    );

    outbox_due_now(&app);
    let pass = drain_outbox(&app).await;
    assert_eq!(pass.sent, 2);
    let bodies: Vec<_> = mock
        .requests("PATCH", "/v1/locations/")
        .into_iter()
        .filter(|r| r.status == 200)
        .map(|r| r.body.unwrap()["status"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(bodies, vec!["received", "in_preparation"]);
    assert_eq!(mock.order(&id)["status"], "in_preparation");
}

#[tokio::test]
async fn a_patch_hubrise_refuses_is_parked_not_retried() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    let order = mock.inject("eat_in");
    let id = order["id"].as_str().unwrap().to_string();
    poll_once(&poller(&app)).await.unwrap();
    // The platform cancels before the acknowledgement goes out.
    mock.update(&id, json!({ "status": "cancelled" }));
    let pass = drain_outbox(&app).await;
    assert_eq!(pass.parked, 1);
    let status = cmd::do_hubrise_status(&app.state).unwrap();
    assert_eq!(status.outbox.parked, 1);
    assert!(status.outbox_error.unwrap().contains("422"));
}

#[tokio::test]
async fn a_refused_token_stops_the_outbox_without_counting() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    mock.inject("eat_in");
    poll_once(&poller(&app)).await.unwrap();
    mock.fault("PATCH", "/v1/locations/", 401, Some(1));
    let pass = drain_outbox(&app).await;
    assert!(pass.unauthorized);
    let attempts: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT attempts FROM hubrise_outbox", [], |r| r.get(0))
        .unwrap();
    assert_eq!(attempts, 0);
    assert_eq!(drain_outbox(&app).await.sent, 1);
}

#[tokio::test]
async fn a_sale_push_already_created_is_not_created_twice() {
    let (app, _db, mock) = connected_app().await;
    {
        let db = app.state.db.lock().unwrap();
        pos_hubrise::outbox::enqueue(&db, 7, r#"{"status":"completed","items":[]}"#).unwrap();
    }
    // The first try reached HubRise but its answer was lost.
    mock.fault("POST", "/v1/locations/pili-dev-loc-1/orders", 500, Some(1));
    assert_eq!(drain_outbox(&app).await.retried, 1);
    {
        let mut inner = mock.state.inner.lock().unwrap();
        inner
            .create_order(
                POS_TOKEN,
                "pili-dev-loc-1",
                json!({ "status": "completed", "private_ref": "pili-order-7" }),
            )
            .unwrap();
    }
    outbox_due_now(&app);
    let pass = drain_outbox(&app).await;
    assert_eq!(pass.sent, 1);
    let posts = mock.requests("POST", "/v1/locations/pili-dev-loc-1/orders");
    assert_eq!(posts.len(), 1, "the retry looked it up instead of posting");
    assert_eq!(
        mock.requests("GET", "/v1/locations/pili-dev-loc-1/orders")
            .len(),
        1
    );
}

#[tokio::test]
async fn a_sale_push_carries_its_private_ref() {
    let (app, _db, mock) = connected_app().await;
    {
        let db = app.state.db.lock().unwrap();
        pos_hubrise::outbox::enqueue(&db, 8, r#"{"status":"completed","items":[]}"#).unwrap();
    }
    assert_eq!(drain_outbox(&app).await.sent, 1);
    let posts = mock.requests("POST", "/v1/locations/pili-dev-loc-1/orders");
    assert_eq!(
        posts[0].body.as_ref().unwrap()["private_ref"],
        "pili-order-8"
    );
}

#[tokio::test]
async fn staff_status_changes_are_checked_and_mirrored() {
    let (app, _db, mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    let order = mock.inject("webshop_cash_collection");
    let id = order["id"].as_str().unwrap().to_string();
    poll_once(&poller(&app)).await.unwrap();

    app.logout();
    assert_eq!(
        cmd::do_patch_order_status(&app.state, &id, "in_preparation").unwrap_err(),
        "not_logged_in"
    );
    app.login_as("84022899837", pos_core::user::UserRole::Waiter);
    for fiscal in ["accepted", "rejected", "completed", "cancelled", "bogus"] {
        assert_eq!(
            cmd::do_patch_order_status(&app.state, &id, fiscal).unwrap_err(),
            "hubrise_status_not_allowed",
            "{fiscal}"
        );
    }
    assert_eq!(
        cmd::do_patch_order_status(&app.state, "nope", "in_preparation").unwrap_err(),
        "hubrise_order_unknown"
    );
    cmd::do_patch_order_status(&app.state, &id, "awaiting_collection").unwrap();
    assert_eq!(local_orders(&app)[0].1, "awaiting_collection");

    assert_eq!(
        cmd::do_set_confirmed_time(&app.state, &id, "tomorrow").unwrap_err(),
        "hubrise_time_invalid"
    );
    cmd::do_set_confirmed_time(&app.state, &id, "2026-09-27T19:30:00Z").unwrap();
    let raw: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT raw_hubrise_json FROM orders WHERE hubrise_order_id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(raw.contains("2026-09-27T19:30:00Z"));

    drain_outbox(&app).await;
    let o = mock.order(&id);
    assert_eq!(o["status"], "awaiting_collection");
    assert_eq!(o["confirmed_time"], "2026-09-27T19:30:00Z");
}

#[tokio::test]
async fn the_mock_answers_the_pos_client() {
    let mock = HubriseMock::start().await;
    let client = pos_hubrise::Client::new(reqwest::Client::new(), mock.api(), POS_TOKEN);
    let location: serde_json::Value = client.get_json("/location").await.unwrap();
    assert!(location["id"].is_string());
}
