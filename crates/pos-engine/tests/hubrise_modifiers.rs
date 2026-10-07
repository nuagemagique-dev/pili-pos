//! Products with options on the order screen: the groups come from the
//! mirrored catalog, the backend checks each group's minimum and
//! maximum, prices the options and names the line.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::TestApp;
use common::hubrise::{connected_app, poller};
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::errors;
use pos_engine::commands::modifiers::{
    AddWithModifiersInput, do_add_order_line_with_modifiers, do_get_item_modifiers,
};
use pos_engine::commands::order::{CreateOrderInput, do_create_order};
use pos_hubrise::poller::start;

const INSZ: &str = "12345678958";

async fn started() -> (TestApp, tempfile::TempDir) {
    let (app, dir, _mock) = connected_app().await;
    start(&poller(&app)).await.unwrap();
    app.login_as(INSZ, UserRole::Cashier);
    (app, dir)
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

fn option(app: &TestApp, menu_item_id: i64, name: &str) -> i64 {
    do_get_item_modifiers(&app.state, menu_item_id, "fr")
        .unwrap()
        .iter()
        .flat_map(|g| g.options.iter())
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("{name}"))
        .id
}

fn order(app: &TestApp, order_type: OrderType) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type,
            table_number: (order_type == OrderType::DineIn).then_some(4),
            user_insz: INSZ.into(),
        },
    )
    .unwrap()
    .id
}

fn add(
    app: &TestApp,
    order_id: i64,
    menu_item_id: i64,
    ids: Vec<i64>,
) -> Result<pos_core::order::OrderLine, String> {
    do_add_order_line_with_modifiers(
        &app.state,
        &AddWithModifiersInput {
            order_id,
            menu_item_id,
            quantity: 2,
            modifier_ids: ids,
        },
    )
}

#[tokio::test]
async fn the_groups_come_from_the_catalog() {
    let (app, _db) = started().await;
    let california = item(&app, "CALIFORNIA-8");
    let groups = do_get_item_modifiers(&app.state, california, "fr").unwrap();
    let names: Vec<_> = groups.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(names, ["Sauce", "Suppléments"]);
    assert_eq!((groups[0].min, groups[0].max), (0, Some(1)));
    assert_eq!(groups[1].max, None);
    assert_eq!(groups[1].options.len(), 2);
    assert!(
        do_get_item_modifiers(&app.state, item(&app, "COCA-33"), "fr")
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn options_are_priced_and_named_by_the_backend() {
    let (app, _db) = started().await;
    let california = item(&app, "CALIFORNIA-8");
    let ids = vec![
        option(&app, california, "Cheese"),
        option(&app, california, "Soja sucrée"),
        option(&app, california, "Avocat"),
    ];
    let line = add(&app, order(&app, OrderType::DineIn), california, ids).unwrap();
    assert_eq!(line.unit_price_cents, 890 + 50 + 100 + 80);
    // Groups in the menu's order, options in theirs.
    assert!(
        line.name.ends_with("(Soja sucrée, Avocat, Cheese)"),
        "{}",
        line.name
    );
    let picked: serde_json::Value =
        serde_json::from_str(line.modifiers_json.as_deref().unwrap()).unwrap();
    assert_eq!(picked.as_array().unwrap().len(), 3);
    assert_eq!(line.quantity, 2);
}

#[tokio::test]
async fn no_option_is_the_plain_product() {
    let (app, _db) = started().await;
    let maki = item(&app, "MAKI-SAUMON-12");
    let line = add(&app, order(&app, OrderType::Takeaway), maki, vec![]).unwrap();
    assert_eq!(line.unit_price_cents, 1000);
    assert!(!line.name.contains('('));
    assert_eq!(line.modifiers_json, None);
}

#[tokio::test]
async fn a_group_maximum_and_foreign_options_are_refused() {
    let (app, _db) = started().await;
    let order_id = order(&app, OrderType::DineIn);
    let california = item(&app, "CALIFORNIA-8");
    let maki = item(&app, "MAKI-SAUMON-12");
    let two_sauces = vec![
        option(&app, california, "Soja"),
        option(&app, california, "Soja sucrée"),
    ];
    let err = add(&app, order_id, california, two_sauces).unwrap_err();
    assert!(err.starts_with(errors::hubrise::MODIFIER_COUNT), "{err}");

    // The maki's own sauce group is another group than the California's.
    let foreign = option(&app, california, "Avocat");
    let err = add(&app, order_id, maki, vec![foreign]).unwrap_err();
    assert!(err.starts_with(errors::hubrise::MODIFIER_UNKNOWN), "{err}");

    let soja = option(&app, california, "Soja");
    let err = add(&app, order_id, california, vec![soja, soja]).unwrap_err();
    assert!(err.starts_with(errors::hubrise::MODIFIER_UNKNOWN), "{err}");

    let lines: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM order_lines WHERE order_id = ?1",
            [order_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(lines, 0);
}

#[tokio::test]
async fn a_required_group_needs_its_choice() {
    let (app, _db) = started().await;
    let california = item(&app, "CALIFORNIA-8");
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE modifier_groups SET min_select = 1
             WHERE menu_item_id = ?1 AND max_select = 1",
            [california],
        )
        .unwrap();
    let order_id = order(&app, OrderType::DineIn);
    let err = add(&app, order_id, california, vec![]).unwrap_err();
    assert!(err.starts_with(errors::hubrise::MODIFIER_COUNT), "{err}");
    let soja = option(&app, california, "Soja");
    assert!(add(&app, order_id, california, vec![soja]).is_ok());
}

#[tokio::test]
async fn a_waiter_adds_but_nobody_logged_out_does() {
    let (app, _db) = started().await;
    let order_id = order(&app, OrderType::DineIn);
    let maki = item(&app, "MAKI-SAUMON-12");
    app.login_as(INSZ, UserRole::Waiter);
    assert!(add(&app, order_id, maki, vec![]).is_ok());
    app.logout();
    assert!(add(&app, order_id, maki, vec![]).is_err());
}
