//! Sold-out products ("Épuisé", V11-D14): the till refuses a sold-out product, the state
//! expires by time, every change is logged, and nothing fiscal is touched.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_remove_order_line,
    do_update_order_line_quantity,
};
use pos_engine::commands::sold_out::{do_clear_sold_out, do_list_sold_out, do_set_sold_out};

const MANAGER: &str = "12345678958";
const CASHIER: &str = "75061189702";
const WAITER: &str = "84022899837";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
                 VALUES (1, 1, 1200, 'A', 0, 1), (2, 1, 800, 'B', 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                 VALUES ('menu_item', 1, 'name', 'fr', 'Maki saumon'),
                        ('menu_item', 2, 'name', 'fr', 'Edamame');",
        )
        .unwrap();
    app.login_as(CASHIER, UserRole::Cashier);
    app
}

fn order(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(3),
            user_insz: CASHIER.to_string(),
        },
    )
    .unwrap()
    .id
}

fn add(app: &TestApp, order_id: i64, item: i64, quantity: i32) -> Result<i64, String> {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: item,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .map(|l| l.id)
}

fn log_actions(app: &TestApp) -> Vec<(i64, String)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT menu_item_id, action FROM sold_out_log ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn menu_flags(app: &TestApp, item: i64) -> (bool, Option<String>) {
    let items = pos_engine::commands::menu::do_get_menu_items(&app.state, None, None).unwrap();
    let m = items.iter().find(|m| m.id == item).unwrap();
    (m.sold_out, m.sold_out_until.clone())
}

#[tokio::test]
async fn a_sold_out_product_is_refused_on_an_order() {
    let app = app().await;
    let o = order(&app);
    do_set_sold_out(&app.state, 1, true, None).unwrap();
    assert_eq!(
        add(&app, o, 1, 1).unwrap_err(),
        errors::coded(errors::PRODUCT_SOLD_OUT, &[("product", &1)])
    );
    // The other product is unaffected.
    add(&app, o, 2, 1).unwrap();
    do_clear_sold_out(&app.state, 1, None).unwrap();
    add(&app, o, 1, 1).unwrap();
}

#[tokio::test]
async fn a_quantity_increase_is_refused_but_a_decrease_and_a_removal_are_not() {
    let app = app().await;
    let o = order(&app);
    let line = add(&app, o, 1, 3).unwrap();
    do_set_sold_out(&app.state, 1, false, None).unwrap();

    assert_eq!(
        do_update_order_line_quantity(&app.state, line, 4).unwrap_err(),
        errors::coded(errors::PRODUCT_SOLD_OUT, &[("product", &1)])
    );
    // The same quantity and a decrease are allowed: the line is already on the order.
    do_update_order_line_quantity(&app.state, line, 3).unwrap();
    do_update_order_line_quantity(&app.state, line, 2).unwrap();
    do_remove_order_line(&app.state, line).unwrap();
}

#[tokio::test]
async fn the_menu_shows_the_state_and_the_end_of_the_day() {
    let app = app().await;
    assert_eq!(menu_flags(&app, 1), (false, None));
    do_set_sold_out(&app.state, 1, true, None).unwrap();
    let (sold, until) = menu_flags(&app, 1);
    assert!(sold);
    let until = until.expect("end of the business day");
    // RFC 3339 with the Brussels offset, at day_end_hour o'clock.
    let hour = app.state.config.lock().unwrap().day_end_hour;
    assert!(until.contains(&format!("T{hour:02}:00:00+0")), "{until}");

    do_set_sold_out(&app.state, 2, false, None).unwrap();
    assert_eq!(menu_flags(&app, 2), (true, None), "until re-enabled");
    do_clear_sold_out(&app.state, 2, None).unwrap();
    assert_eq!(menu_flags(&app, 2), (false, None));
}

#[tokio::test]
async fn a_past_end_of_day_expires_by_time_and_is_logged_once() {
    let app = app().await;
    do_set_sold_out(&app.state, 1, false, None).unwrap();
    // The end of the business day passed an hour ago.
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE menu_item_sold_out SET until = '2020-05-05T04:00:00+02:00'",
            [],
        )
        .unwrap();
    let o = order(&app);
    add(&app, o, 1, 1).unwrap();
    assert_eq!(menu_flags(&app, 1), (false, None));
    assert_eq!(do_list_sold_out(&app.state, None).unwrap().len(), 0);
    assert_eq!(do_list_sold_out(&app.state, None).unwrap().len(), 0);
    assert_eq!(
        log_actions(&app),
        vec![(1, "set".to_string()), (1, "expired".to_string())]
    );
}

#[tokio::test]
async fn every_change_is_logged_and_the_log_is_immutable() {
    let app = app().await;
    do_set_sold_out(&app.state, 1, true, None).unwrap();
    do_set_sold_out(&app.state, 1, false, None).unwrap();
    do_clear_sold_out(&app.state, 1, None).unwrap();
    // Clearing what is not sold out writes nothing.
    do_clear_sold_out(&app.state, 1, None).unwrap();
    assert_eq!(
        log_actions(&app),
        vec![
            (1, "set".to_string()),
            (1, "set".to_string()),
            (1, "clear".to_string())
        ]
    );
    let by: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT by FROM sold_out_log LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(by, CASHIER);
    let db = app.state.db.lock().unwrap();
    assert!(
        db.execute("UPDATE sold_out_log SET action = 'clear'", [])
            .is_err()
    );
    assert!(db.execute("DELETE FROM sold_out_log", []).is_err());
}

#[tokio::test]
async fn the_list_names_the_products_in_the_asked_language() {
    let app = app().await;
    do_set_sold_out(&app.state, 2, true, None).unwrap();
    let list = do_set_sold_out(&app.state, 1, false, Some("en".into())).unwrap();
    assert_eq!(
        list.iter()
            .map(|i| (i.menu_item_id, i.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(1, "Maki saumon"), (2, "Edamame")],
        "falls back to French; menu order"
    );
    assert!(list[0].until.is_none() && list[1].until.is_some());
    assert_eq!(list[0].set_by, CASHIER);
}

#[tokio::test]
async fn an_unknown_product_is_refused() {
    let app = app().await;
    let expected = errors::coded(errors::PRODUCT_UNAVAILABLE, &[("product", &99)]);
    assert_eq!(
        do_set_sold_out(&app.state, 99, true, None).unwrap_err(),
        expected
    );
    assert_eq!(
        do_clear_sold_out(&app.state, 99, None).unwrap_err(),
        expected
    );
}

#[tokio::test]
async fn a_waiter_cannot_change_it_and_nobody_signed_in_either() {
    let app = app().await;
    do_set_sold_out(&app.state, 1, true, None).unwrap();
    app.login_as(WAITER, UserRole::Waiter);
    assert_eq!(
        do_set_sold_out(&app.state, 2, true, None).unwrap_err(),
        errors::FORBIDDEN
    );
    assert_eq!(
        do_clear_sold_out(&app.state, 1, None).unwrap_err(),
        errors::FORBIDDEN
    );
    assert_eq!(
        do_list_sold_out(&app.state, None).unwrap_err(),
        errors::FORBIDDEN
    );
    // The waiter still reads the flag on the menu, and the till still refuses for them.
    assert!(menu_flags(&app, 1).0);
    let o = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(4),
            user_insz: WAITER.to_string(),
        },
    )
    .unwrap()
    .id;
    assert!(
        add(&app, o, 1, 1)
            .unwrap_err()
            .starts_with(errors::PRODUCT_SOLD_OUT)
    );

    app.logout();
    assert_eq!(
        do_set_sold_out(&app.state, 1, true, None).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    app.login_as(MANAGER, UserRole::Admin);
    assert!(do_clear_sold_out(&app.state, 1, None).is_ok());
}

#[tokio::test]
async fn sold_out_touches_nothing_fiscal() {
    let app = app().await;
    let events = |app: &TestApp| -> i64 {
        app.state
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
            .unwrap()
    };
    let before = events(&app);
    do_set_sold_out(&app.state, 1, true, None).unwrap();
    do_clear_sold_out(&app.state, 1, None).unwrap();
    assert_eq!(events(&app), before, "no event, no counter");
}

#[tokio::test]
async fn nothing_is_queued_for_hubrise_when_it_is_off_or_the_product_has_no_sku() {
    let app = app().await;
    do_set_sold_out(&app.state, 1, true, None).unwrap();
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
}
