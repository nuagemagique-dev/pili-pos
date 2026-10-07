//! State the screens read back from the backend, beyond the fiscal
//! record itself:
//!
//! - who is on shift: a signed clock-in opens a shift, a signed
//!   clock-out closes it (locking the screen does neither);
//! - the cart: each line's French and Chinese names, and how many
//!   units were not yet sent to the bar / kitchen;
//! - the Tables screen: when the order was opened, and whether its
//!   provisional bill is still the current one.

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_engine::commands::order::do_list_cart_lines;
use pos_engine::commands::social::{do_list_open_shifts, do_work_in, do_work_out};
use pos_engine::commands::tables::{do_list_tables, do_sign_pre_bill, do_sign_table_order};

const INSZ: &str = "12345678901";

/// A dine-in order at table 1 with one line of product 1, which has
/// a French and a Chinese name.
fn seed_order(app: &TestApp) -> (i64, i64) {
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT OR IGNORE INTO users (insz, name, role, active)
            VALUES ('12345678901', 'Test', 'cashier', 1);
         INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
            VALUES (1, 1, 1000, 'B', 0, 1);
         INSERT OR IGNORE INTO translations (entity_type, entity_id, field, lang, value) VALUES
            ('menu_item', 1, 'name', 'fr', 'Soupe miso'),
            ('menu_item', 1, 'name', 'zh', '味噌汤');",
    )
    .unwrap();
    let order = pos_db::queries::orders::create_order(&conn, OrderType::DineIn, Some(1), INSZ)
        .expect("create order");
    let line = pos_db::queries::orders::add_order_line(
        &conn,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 1,
            quantity: 2,
            unit_price_cents: 1000,
            vat_code: VatCode::B,
            name: "Soupe miso",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .expect("add line");
    (order.id, line.id)
}

fn set_quantity(app: &TestApp, line_id: i64, quantity: i32) {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::orders::update_order_line_quantity(&conn, line_id, quantity).unwrap();
}

#[tokio::test]
async fn clocking_in_and_out_opens_and_closes_the_shift() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M141")).await;

    assert!(do_list_open_shifts(&app.state).unwrap().is_empty());

    do_work_in(&app.state, INSZ.to_string()).await.unwrap();
    let open = do_list_open_shifts(&app.state).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].user_insz, INSZ);

    do_work_out(&app.state, INSZ.to_string()).await.unwrap();
    assert!(do_list_open_shifts(&app.state).unwrap().is_empty());
}

#[tokio::test]
async fn cart_lines_carry_both_names_and_the_unsent_quantity() {
    let app = TestApp::new().await;
    let (order_id, line_id) = seed_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    let lines = do_list_cart_lines(&app.state, order_id).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].name_fr.as_deref(), Some("Soupe miso"));
    assert_eq!(lines[0].name_zh.as_deref(), Some("味噌汤"));
    assert_eq!(lines[0].unsent_quantity, 2);

    // Sent to the kitchen: nothing left to send.
    do_sign_table_order(&app.state, order_id).await.unwrap();
    assert_eq!(
        do_list_cart_lines(&app.state, order_id).unwrap()[0].unsent_quantity,
        0
    );

    // One more ordered: only that one is unsent.
    set_quantity(&app, line_id, 3);
    assert_eq!(
        do_list_cart_lines(&app.state, order_id).unwrap()[0].unsent_quantity,
        1
    );
}

#[tokio::test]
async fn the_table_shows_its_bill_until_the_order_changes() {
    let app = TestApp::new().await;
    let (order_id, line_id) = seed_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M123")).await;

    let table = &do_list_tables(&app.state, 1).unwrap()[0];
    assert!(table.opened_at.is_some());
    assert!(!table.pre_bill_printed);

    do_sign_pre_bill(&app.state, order_id).await.unwrap();
    assert!(do_list_tables(&app.state, 1).unwrap()[0].pre_bill_printed);

    set_quantity(&app, line_id, 3);
    assert!(!do_list_tables(&app.state, 1).unwrap()[0].pre_bill_printed);
}
