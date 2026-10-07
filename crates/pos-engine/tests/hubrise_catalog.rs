//! The POS menu mirrors the HubRise catalog: applied at start, on a
//! catalog event and by the drift check, with stable product ids, the
//! VAT labels of the catalog, and the menu kept when a catalog is bad.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::TestApp;
use common::hubrise::{HubriseMock, POS_TOKEN, WEBSHOP_TOKEN, connect, connected_app, poller};
use pos_catalog::ApplyOutcome;
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_engine::commands::hubrise as cmd;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_hubrise::poller::{check_catalog, poll_once, start};
use pos_hubrise_mock::builders::{CatalogBuilder, seed_catalog, seed_catalog_builder};
use rusqlite::params;

/// A valid INSZ, logged in as admin before ordering.
const INSZ: &str = "12345678958";

fn item_id(app: &TestApp, key: &str) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM menu_items WHERE external_id = ?1",
            params![key],
            |r| r.get(0),
        )
        .unwrap()
}

/// Order `key` once, eaten in or taken away; the line as the POS
/// priced it.
fn line(app: &TestApp, key: &str, dine_in: bool) -> pos_core::order::OrderLine {
    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if dine_in {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: dine_in.then_some(7),
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap();
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order.id,
            menu_item_id: item_id(app, key),
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap()
}

/// Replace the connected catalog as the side that fills it (an event
/// is queued for the POS).
fn push_catalog(mock: &HubriseMock, data: pos_hubrise_types::CatalogData) {
    let mut inner = mock.state.inner.lock().unwrap();
    let id = inner.default_catalog_id.clone();
    inner.replace_catalog(WEBSHOP_TOKEN, &id, None, data);
}

fn with_maki_price(cents: i64) -> pos_hubrise_types::CatalogData {
    let mut json = seed_catalog_builder().build_json();
    json["products"][0]["skus"][0]["price"] =
        format!("{}.{:02} EUR", cents / 100, cents % 100).into();
    serde_json::from_value(json).unwrap()
}

#[tokio::test]
async fn start_mirrors_the_catalog_into_the_menu() {
    let (app, _db, _mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();

    let grid: Vec<String> = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::menu::get_menu_items(&db, None, "fr")
            .unwrap()
            .into_iter()
            .map(|i| i.name)
            .collect()
    };
    assert_eq!(
        grid,
        vec![
            "Maki saumon 6 pièces",
            "Maki saumon 12 pièces",
            "California roll",
            "Soupe miso",
            "Coca-Cola 33cl",
            "Bière Asahi 33cl",
        ]
    );
    let tabs = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::menu::get_categories(&db, "fr")
            .unwrap()
            .len()
    };
    assert_eq!(tabs, 3, "the charges category is hidden");

    let zh: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT value FROM translations WHERE entity_type = 'menu_item'
             AND entity_id = (SELECT id FROM menu_items WHERE external_id = 'CALIFORNIA-8')
             AND lang = 'zh'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(zh, "加州卷");
}

#[tokio::test]
async fn food_is_b_eaten_in_and_c_taken_away_drinks_follow_their_rates() {
    let (app, _db, _mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();

    let eat_in = line(&app, "MAKI-SAUMON-6", true);
    let takeaway = line(&app, "MAKI-SAUMON-6", false);
    assert_eq!(
        (eat_in.vat_code, takeaway.vat_code),
        (VatCode::B, VatCode::C)
    );
    assert_eq!(eat_in.unit_price_cents, 550);
    assert_eq!(eat_in.name, "Maki saumon 6 pièces");

    assert_eq!(line(&app, "COCA-33", true).vat_code, VatCode::A);
    assert_eq!(line(&app, "COCA-33", false).vat_code, VatCode::C);
    assert_eq!(line(&app, "ASAHI-33", false).vat_code, VatCode::A);
}

#[tokio::test]
async fn a_catalog_event_changes_the_price_and_keeps_the_id() {
    let (app, _db, mock) = connected_app().await;
    let ctx = poller(&app);
    start(&ctx).await.unwrap();
    let before = item_id(&app, "MAKI-SAUMON-6");

    push_catalog(&mock, with_maki_price(600));
    poll_once(&ctx).await.unwrap();

    assert_eq!(item_id(&app, "MAKI-SAUMON-6"), before);
    assert_eq!(line(&app, "MAKI-SAUMON-6", true).unit_price_cents, 600);
}

#[tokio::test]
async fn a_removed_product_leaves_the_menu_and_returns_with_its_id() {
    let (app, _db, mock) = connected_app().await;
    let ctx = poller(&app);
    start(&ctx).await.unwrap();
    let miso = item_id(&app, "MISO-1");

    let mut json = seed_catalog_builder().build_json();
    json["products"]
        .as_array_mut()
        .unwrap()
        .retain(|p| p["ref"] != "MISO");
    push_catalog(&mock, serde_json::from_value(json).unwrap());
    poll_once(&ctx).await.unwrap();
    let on_sale: bool = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT active FROM menu_items WHERE id = ?1",
            params![miso],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!on_sale);

    push_catalog(&mock, seed_catalog());
    poll_once(&ctx).await.unwrap();
    assert_eq!(item_id(&app, "MISO-1"), miso);
    assert_eq!(line(&app, "MISO-1", true).unit_price_cents, 350);
}

#[tokio::test]
async fn a_bad_catalog_keeps_the_menu_and_shows_its_issues() {
    let (app, _db, mock) = connected_app().await;
    let ctx = poller(&app);
    start(&ctx).await.unwrap();

    let bad = CatalogBuilder::new()
        .category("MAKI", "Makis")
        .product(
            "MAKI-SAUMON",
            "MAKI",
            "Maki saumon",
            ("20.0", "6.0", "6.0"),
            &[("MAKI-SAUMON-6", None, 999, &[])],
        )
        .build();
    push_catalog(&mock, bad);
    poll_once(&ctx).await.unwrap();

    assert_eq!(line(&app, "MAKI-SAUMON-6", true).unit_price_cents, 550);
    let status = cmd::do_hubrise_status(&app.state).unwrap();
    let catalog = status.catalog.expect("catalog state");
    assert!(catalog.rejected_at.is_some());
    assert_eq!(
        catalog.issues[0].code,
        pos_catalog::IssueCode::UnknownVatRate
    );
    assert_eq!(catalog.issues[0].key, "MAKI-SAUMON-6");
}

#[tokio::test]
async fn the_drift_check_catches_a_change_without_event() {
    let (app, _db, mock) = connected_app().await;
    let ctx = poller(&app);
    start(&ctx).await.unwrap();
    {
        // Changed by the POS's own client: HubRise sends no event.
        let mut inner = mock.state.inner.lock().unwrap();
        let id = inner.default_catalog_id.clone();
        inner.replace_catalog(POS_TOKEN, &id, None, with_maki_price(575));
    }
    poll_once(&ctx).await.unwrap();
    assert_eq!(
        line(&app, "MAKI-SAUMON-6", true).unit_price_cents,
        550,
        "no event"
    );

    let outcome = check_catalog(&ctx).await.unwrap().unwrap();
    assert!(
        matches!(outcome, ApplyOutcome::Applied { .. }),
        "{outcome:?}"
    );
    assert_eq!(line(&app, "MAKI-SAUMON-6", true).unit_price_cents, 575);
    assert_eq!(
        check_catalog(&ctx).await.unwrap(),
        Some(ApplyOutcome::Unchanged)
    );
}

#[tokio::test]
async fn a_rate_limited_catalog_read_does_not_stop_the_start() {
    let (app, _db, mock) = connected_app().await;
    mock.state.inner.lock().unwrap().catalog_reads_per_minute = 0;
    mock.inject("deliveroo_prepaid");
    let ctx = poller(&app);
    start(&ctx).await.unwrap();
    let orders: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM orders WHERE hubrise_order_id IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(orders, 1, "the backfill still ran");

    mock.state.inner.lock().unwrap().catalog_reads_per_minute = 10;
    let outcome = check_catalog(&ctx).await.unwrap().unwrap();
    assert!(matches!(outcome, ApplyOutcome::Applied { .. }));
}

/// A product entered by hand before HubRise, named and priced like the
/// catalog's.
fn hand_entered(app: &TestApp, name: &str, price: i64) -> i64 {
    let db = app.state.db.lock().unwrap();
    db.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order) VALUES (900, 0)",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO menu_items (category_id, price_cents, vat_code) VALUES (900, ?1, 'B')",
        params![price],
    )
    .unwrap();
    let id = db.last_insert_rowid();
    db.execute(
        "INSERT INTO translations (entity_type, entity_id, field, lang, value)
         VALUES ('menu_item', ?1, 'name', 'fr', ?2)",
        params![id, name],
    )
    .unwrap();
    id
}

#[tokio::test]
async fn a_hand_entered_menu_waits_for_the_manager_then_keeps_adopted_ids() {
    let (app, _db) = TestApp::with_shared_db().await;
    let miso = hand_entered(&app, "Soupe miso", 350);
    let gyoza = hand_entered(&app, "Gyoza", 550);
    let mock = HubriseMock::start().await;
    connect(&app, &mock);
    start(&poller(&app)).await.unwrap();

    let status = cmd::do_hubrise_status(&app.state).unwrap();
    assert!(status.catalog_awaiting_adoption);
    let grid = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::menu::get_menu_items(&db, None, "fr")
            .unwrap()
            .len()
    };
    assert_eq!(
        grid, 2,
        "the hand-entered menu stays until the manager decides"
    );

    let preview = cmd::do_adoption_preview(&app.state).unwrap();
    assert_eq!(preview.len(), 1);
    assert_eq!(
        (preview[0].menu_item_id, preview[0].key.as_str()),
        (miso, "MISO-1")
    );

    let outcome = cmd::do_adopt_catalog(&app.state, &preview).unwrap();
    assert!(
        matches!(outcome, ApplyOutcome::Applied { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        item_id(&app, "MISO-1"),
        miso,
        "the adopted product keeps its id"
    );
    let gyoza_on_sale: bool = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT active FROM menu_items WHERE id = ?1",
            params![gyoza],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!gyoza_on_sale);
    assert!(
        !cmd::do_hubrise_status(&app.state)
            .unwrap()
            .catalog_awaiting_adoption
    );
}

#[tokio::test]
async fn taking_over_the_menu_waits_for_open_orders_and_a_manager() {
    let (app, _db) = TestApp::with_shared_db().await;
    let gyoza = hand_entered(&app, "Gyoza", 550);
    let mock = HubriseMock::start().await;
    connect(&app, &mock);
    start(&poller(&app)).await.unwrap();

    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(3),
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap();
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order.id,
            menu_item_id: gyoza,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    assert_eq!(
        cmd::do_adopt_catalog(&app.state, &[]).unwrap_err(),
        pos_engine::commands::errors::hubrise::OPEN_ORDERS
    );

    app.login_as(INSZ, pos_core::user::UserRole::Cashier);
    assert!(cmd::do_adoption_preview(&app.state).is_err());
    assert!(cmd::do_adopt_catalog(&app.state, &[]).is_err());
}

#[tokio::test]
async fn nothing_to_adopt_once_the_menu_is_mirrored() {
    let (app, _db, _mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    assert_eq!(
        cmd::do_adoption_preview(&app.state).unwrap_err(),
        pos_engine::commands::errors::hubrise::NOTHING_PENDING
    );
}

#[tokio::test]
async fn staff_can_resync_the_catalog() {
    let (app, _db, _mock) = connected_app().await;
    let outcome = cmd::do_sync_catalog(&app.state).await.unwrap();
    assert!(matches!(outcome, Some(ApplyOutcome::Applied { .. })));
    assert_eq!(
        cmd::do_sync_catalog(&app.state).await.unwrap(),
        Some(ApplyOutcome::Unchanged)
    );
}

#[tokio::test]
async fn resyncing_without_a_connection_is_refused() {
    let (app, _db) = TestApp::with_shared_db().await;
    assert_eq!(
        cmd::do_sync_catalog(&app.state).await.unwrap_err(),
        pos_engine::commands::errors::hubrise::DISABLED,
        "switched off first"
    );
    app.state.hubrise_switch.set(true);
    assert_eq!(
        cmd::do_sync_catalog(&app.state).await.unwrap_err(),
        pos_engine::commands::errors::hubrise::NOT_CONNECTED
    );
}
