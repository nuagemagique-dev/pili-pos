//! The table plan: the numbers the Tables screen shows, set by a
//! manager, every change logged, and no open order ever hidden.

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::table_plan::{do_get_table_plan, do_set_table_plan};
use pos_engine::commands::tables::do_list_configured_tables;

/// A mod-97-valid INSZ.
const MANAGER: &str = "12345678958";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active, open_price)
                 VALUES (1, 1, 1200, 'A', 0, 1, 0);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                 VALUES ('menu_item', 1, 'name', 'fr', 'Eau');",
        )
        .unwrap();
    app.login_as(MANAGER, UserRole::Admin);
    app
}

fn numbers(app: &TestApp) -> Vec<i32> {
    do_list_configured_tables(&app.state)
        .unwrap()
        .iter()
        .map(|t| t.table_number)
        .collect()
}

#[tokio::test]
async fn without_a_plan_the_tables_are_1_to_table_count() {
    let app = app().await;
    let count = app.state.config.lock().unwrap().table_count as i32;
    assert_eq!(numbers(&app), (1..=count).collect::<Vec<_>>());
    let plan = do_get_table_plan(&app.state).unwrap();
    assert_eq!(plan.plan, format!("1-{count}"));
    assert_eq!(plan.table_count, count as i64);
}

#[tokio::test]
async fn a_manager_sets_ranges_and_the_change_is_logged() {
    let app = app().await;
    let plan = do_set_table_plan(&app.state, "20-22,1-3").unwrap();
    assert_eq!(plan.plan, "1-3, 20-22", "stored in its canonical form");
    assert_eq!(plan.table_count, 6);
    assert_eq!(numbers(&app), vec![1, 2, 3, 20, 21, 22]);
    assert_eq!(plan.log[0].source, "admin");
    assert_eq!(plan.log[0].changed_by.as_deref(), Some(MANAGER));
}

#[tokio::test]
async fn a_wrong_plan_is_refused_and_the_old_one_kept() {
    let app = app().await;
    do_set_table_plan(&app.state, "1-4").unwrap();
    for bad in ["", "0-3", "5-2", "1-500", "A1"] {
        assert_eq!(
            do_set_table_plan(&app.state, bad).unwrap_err(),
            errors::TABLE_PLAN_INVALID,
            "{bad:?}"
        );
    }
    assert_eq!(numbers(&app), vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn only_a_manager_changes_the_plan() {
    let app = app().await;
    app.login_as(MANAGER, UserRole::Cashier);
    assert!(do_set_table_plan(&app.state, "1-4").is_err());
    assert!(do_get_table_plan(&app.state).is_ok(), "everyone reads it");
}

#[tokio::test]
async fn a_table_left_out_of_the_plan_stays_while_its_order_is_open() {
    let app = app().await;
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(9),
            user_insz: MANAGER.to_string(),
        },
    )
    .unwrap();
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order.id,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    do_set_table_plan(&app.state, "1-3").unwrap();
    let tables = do_list_configured_tables(&app.state).unwrap();
    let listed: Vec<i32> = tables.iter().map(|t| t.table_number).collect();
    assert_eq!(listed, vec![1, 2, 3, 9]);
    assert_eq!(tables[3].order_id, Some(order.id));
}

#[tokio::test]
async fn an_unreadable_orders_table_is_an_error_never_a_free_table() {
    let app = app().await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch("ALTER TABLE order_lines RENAME TO order_lines_gone")
        .unwrap();
    let err = pos_engine::commands::tables::do_list_tables(&app.state, 3)
        .err()
        .expect("a database error");
    assert!(err.starts_with("internal_error"), "{err}");
}
