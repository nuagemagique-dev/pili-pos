//! The order commands the order screen calls on every tap: start an
//! order, add / change / remove lines, reload it. Each one is driven
//! through its `do_*` function, as the Tauri command runs it.
//!
//! - an order is dine-in at a table or takeaway without one, for a
//!   well-formed operator INSZ;
//! - a table has one open order: asking again returns it with its
//!   lines, so an unpaid order can never hide behind a second one;
//! - quantities are at least 1, and only open orders change;
//! - the line's name, price, VAT label and department come from the
//!   catalogue, in the establishment's language (FR/NL/DE, falling
//!   back to French), never from the screen;
//! - every change keeps the order total and the journal in step.

mod common;

use common::*;

use moyun_pos::commands::errors;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_get_order,
    do_list_open_orders, do_list_order_lines, do_remove_order_line, do_update_order_line_quantity,
};
use moyun_pos::commands::tables::do_list_tables;
use pos_core::order::{OrderStatus, OrderType};
use pos_core::vat::VatCode;

/// A mod-97-valid INSZ.
const INSZ: &str = "12345678958";

fn seed(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, vat_code_takeaway, sort_order, active, open_price) VALUES
            (1, 1, 1200, 'A', NULL, 0, 1, 0),
            (2, 2, 2800, 'B', 'C', 1, 1, 0),
            (3, 2, 0,    'B', NULL, 2, 1, 1),
            (4, 2, 900,  'B', NULL, 3, 1, 0);
         INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
            ('category', 1, 'name', 'fr', 'Boissons'),
            ('category', 1, 'name', 'nl', 'Dranken'),
            ('category', 1, 'name', 'de', 'Getränke'),
            ('category', 2, 'name', 'fr', 'Cuisine'),
            ('menu_item', 1, 'name', 'fr', 'Eau pétillante'),
            ('menu_item', 1, 'name', 'nl', 'Bruiswater'),
            ('menu_item', 1, 'name', 'de', 'Sprudelwasser'),
            ('menu_item', 1, 'name', 'zh', '气泡水'),
            ('menu_item', 2, 'name', 'fr', 'Bœuf bourguignon'),
            ('menu_item', 3, 'name', 'fr', 'Plat du jour'),
            ('menu_item', 4, 'name', 'en', 'Spring rolls');",
    )
    .unwrap();
}

async fn app() -> TestApp {
    let app = TestApp::new().await;
    seed(&app);
    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    app
}

fn set_language(app: &TestApp, lang: &str) {
    app.state.config.lock().unwrap().language = lang.to_string();
}

fn start(app: &TestApp, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
    .id
}

fn add(
    app: &TestApp,
    order_id: i64,
    item: i64,
    qty: i32,
) -> Result<pos_core::order::OrderLine, String> {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: item,
            quantity: qty,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
}

fn total(app: &TestApp, order_id: i64) -> i64 {
    do_get_order(&app.state, order_id).unwrap().total_cents
}

/// Sum of the journal (`order_entries`) quantities per line.
fn journal(app: &TestApp, order_id: i64) -> Vec<(i64, i64)> {
    let conn = app.state.db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT line_id, SUM(quantity) FROM order_entries
             WHERE order_id = ?1 GROUP BY line_id ORDER BY line_id",
        )
        .unwrap();
    stmt.query_map([order_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

// ── Starting an order ──────────────────────────────────────────

#[tokio::test]
async fn an_order_needs_a_well_formed_operator() {
    let app = app().await;
    for insz in ["", "1234567890", "12345678901", "abcdefghijk"] {
        let err = do_create_order(
            &app.state,
            CreateOrderInput {
                order_type: OrderType::DineIn,
                table_number: Some(1),
                user_insz: insz.to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err, errors::OPERATOR_INVALID, "{insz:?}");
    }
    assert!(do_list_open_orders(&app.state).unwrap().is_empty());
}

#[tokio::test]
async fn dine_in_needs_a_table_and_takeaway_has_none() {
    let app = app().await;
    for (order_type, table, expected) in [
        (OrderType::DineIn, None, errors::DINE_IN_NEEDS_TABLE),
        (OrderType::DineIn, Some(0), errors::DINE_IN_NEEDS_TABLE),
        (OrderType::DineIn, Some(-3), errors::DINE_IN_NEEDS_TABLE),
        (OrderType::Takeaway, Some(4), errors::TAKEAWAY_HAS_NO_TABLE),
    ] {
        let err = do_create_order(
            &app.state,
            CreateOrderInput {
                order_type,
                table_number: table,
                user_insz: INSZ.to_string(),
            },
        )
        .unwrap_err();
        assert_eq!(err, expected, "{order_type:?} {table:?}");
    }
    assert!(do_list_open_orders(&app.state).unwrap().is_empty());
}

#[tokio::test]
async fn a_new_order_is_open_empty_and_listed() {
    let app = app().await;
    let id = start(&app, Some(3));
    let order = do_get_order(&app.state, id).unwrap();
    assert_eq!(order.status, OrderStatus::Open);
    assert_eq!(order.table_number, Some(3));
    assert_eq!(order.order_type, OrderType::DineIn);
    assert_eq!(order.user_insz, INSZ);
    assert_eq!(order.total_cents, 0);
    assert!(do_list_order_lines(&app.state, id).unwrap().is_empty());
    let open = do_list_open_orders(&app.state).unwrap();
    assert_eq!(open.iter().map(|o| o.id).collect::<Vec<_>>(), vec![id]);
}

#[tokio::test]
async fn a_busy_table_returns_its_open_order_with_its_lines() {
    let app = app().await;
    let first = start(&app, Some(5));
    add(&app, first, 1, 2).unwrap();
    // The operator leaves and taps table 5 again.
    let again = start(&app, Some(5));
    assert_eq!(again, first, "one open order per table");
    assert_eq!(do_list_order_lines(&app.state, again).unwrap().len(), 1);
    assert_eq!(do_list_open_orders(&app.state).unwrap().len(), 1);
    let tables = do_list_tables(&app.state, 6).unwrap();
    assert_eq!(tables[4].order_id, Some(first));
    assert_eq!(tables[4].total_cents, 2400);
}

#[tokio::test]
async fn an_empty_table_order_is_reused_not_duplicated() {
    let app = app().await;
    let first = start(&app, Some(2));
    assert_eq!(start(&app, Some(2)), first);
    assert_eq!(do_list_open_orders(&app.state).unwrap().len(), 1);
}

#[tokio::test]
async fn tables_do_not_share_orders() {
    let app = app().await;
    let a = start(&app, Some(1));
    let b = start(&app, Some(2));
    assert_ne!(a, b);
}

#[tokio::test]
async fn every_takeaway_is_its_own_order() {
    let app = app().await;
    let a = start(&app, None);
    let b = start(&app, None);
    assert_ne!(a, b);
    assert_eq!(do_list_open_orders(&app.state).unwrap().len(), 2);
}

#[tokio::test]
async fn a_paid_table_starts_afresh() {
    let app = app().await;
    let first = start(&app, Some(7));
    add(&app, first, 1, 1).unwrap();
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let paid = do_checkout(&app.state, first, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(paid.success, "{:?}", paid.error);
    let next = start(&app, Some(7));
    assert_ne!(next, first, "the paid order is closed");
    assert!(do_list_order_lines(&app.state, next).unwrap().is_empty());
}

// ── Lines from the catalogue ───────────────────────────────────

#[tokio::test]
async fn a_line_takes_price_vat_and_names_from_the_catalogue() {
    let app = app().await;
    let order = start(&app, Some(1));
    let line = add(&app, order, 1, 3).unwrap();
    assert_eq!(line.unit_price_cents, 1200);
    assert_eq!(line.vat_code, VatCode::A);
    assert_eq!(line.name, "Eau pétillante");
    assert_eq!(line.department_id, "1");
    assert_eq!(line.department_name, "Boissons");
    assert_eq!(line.quantity, 3);
    assert_eq!(total(&app, order), 3600);
}

#[tokio::test]
async fn names_are_in_the_establishment_language() {
    for (lang, name, department) in [
        ("fr", "Eau pétillante", "Boissons"),
        ("nl", "Bruiswater", "Dranken"),
        ("de", "Sprudelwasser", "Getränke"),
        // Not a ticket language: fiscal names stay French.
        ("zh", "Eau pétillante", "Boissons"),
        ("en", "Eau pétillante", "Boissons"),
    ] {
        let app = app().await;
        set_language(&app, lang);
        let order = start(&app, Some(1));
        let line = add(&app, order, 1, 1).unwrap();
        assert_eq!(line.name, name, "{lang}");
        assert_eq!(line.department_name, department, "{lang}");
    }
}

#[tokio::test]
async fn a_missing_translation_falls_back_to_french_then_english() {
    for lang in ["nl", "de"] {
        let app = app().await;
        set_language(&app, lang);
        let order = start(&app, None);
        let french = add(&app, order, 2, 1).unwrap();
        assert_eq!(french.name, "Bœuf bourguignon", "{lang}");
        assert_eq!(french.department_name, "Cuisine", "{lang}");
        let english = add(&app, order, 4, 1).unwrap();
        assert_eq!(english.name, "Spring rolls", "{lang}");
    }
}

#[tokio::test]
async fn takeaway_uses_the_takeaway_vat_label() {
    let app = app().await;
    let dine_in = start(&app, Some(1));
    let takeaway = start(&app, None);
    assert_eq!(add(&app, dine_in, 2, 1).unwrap().vat_code, VatCode::B);
    assert_eq!(add(&app, takeaway, 2, 1).unwrap().vat_code, VatCode::C);
    // No takeaway label: the dine-in one applies.
    assert_eq!(add(&app, takeaway, 1, 1).unwrap().vat_code, VatCode::A);
}

#[tokio::test]
async fn a_free_price_line_needs_a_positive_price_and_a_fixed_one_refuses_it() {
    let app = app().await;
    let order = start(&app, Some(1));
    let with_price = |item: i64, price: Option<i64>| {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: item,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: price,
            },
        )
    };
    for price in [None, Some(0), Some(-500)] {
        assert_eq!(
            with_price(3, price).unwrap_err(),
            errors::PRICE_POSITIVE,
            "{price:?}"
        );
    }
    assert_eq!(with_price(3, Some(1450)).unwrap().unit_price_cents, 1450);
    assert_eq!(with_price(1, Some(1)).unwrap_err(), "fixed_price|product=1");
    assert_eq!(total(&app, order), 1450);
}

#[tokio::test]
async fn unknown_products_and_orders_are_refused() {
    let app = app().await;
    let order = start(&app, Some(1));
    assert_eq!(
        add(&app, order, 999, 1).unwrap_err(),
        "product_unavailable|product=999"
    );
    assert!(add(&app, 424_242, 1, 1).is_err());
    assert!(do_get_order(&app.state, 424_242).is_err());
    assert!(do_list_order_lines(&app.state, order).unwrap().is_empty());
}

#[tokio::test]
async fn quantities_start_at_one() {
    let app = app().await;
    let order = start(&app, Some(1));
    for qty in [0, -1, i32::MIN] {
        assert_eq!(
            add(&app, order, 1, qty).unwrap_err(),
            errors::QUANTITY_POSITIVE
        );
    }
    assert!(do_list_order_lines(&app.state, order).unwrap().is_empty());
}

// ── Changing lines ─────────────────────────────────────────────

#[tokio::test]
async fn changing_a_quantity_updates_the_total_and_the_journal() {
    let app = app().await;
    let order = start(&app, Some(1));
    let line = add(&app, order, 1, 1).unwrap();
    do_update_order_line_quantity(&app.state, line.id, 4).unwrap();
    assert_eq!(total(&app, order), 4800);
    do_update_order_line_quantity(&app.state, line.id, 2).unwrap();
    assert_eq!(total(&app, order), 2400);
    assert_eq!(journal(&app, order), vec![(line.id, 2)]);
    let lines = do_list_order_lines(&app.state, order).unwrap();
    assert_eq!(lines[0].quantity, 2);
}

#[tokio::test]
async fn setting_the_same_quantity_journals_nothing() {
    let app = app().await;
    let order = start(&app, Some(1));
    let line = add(&app, order, 1, 2).unwrap();
    do_update_order_line_quantity(&app.state, line.id, 2).unwrap();
    let conn = app.state.db.lock().unwrap();
    let entries: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM order_entries WHERE line_id = ?1",
            [line.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(entries, 1, "only the first addition");
}

#[tokio::test]
async fn a_quantity_of_zero_or_less_is_refused() {
    let app = app().await;
    let order = start(&app, Some(1));
    let line = add(&app, order, 1, 2).unwrap();
    for qty in [0, -2] {
        assert_eq!(
            do_update_order_line_quantity(&app.state, line.id, qty).unwrap_err(),
            errors::QUANTITY_POSITIVE
        );
    }
    assert_eq!(total(&app, order), 2400, "nothing changed");
    assert_eq!(journal(&app, order), vec![(line.id, 2)]);
}

#[tokio::test]
async fn removing_a_line_updates_the_total_and_journals_a_correction() {
    let app = app().await;
    let order = start(&app, Some(1));
    let water = add(&app, order, 1, 2).unwrap();
    let beef = add(&app, order, 2, 1).unwrap();
    assert_eq!(total(&app, order), 5200);
    do_remove_order_line(&app.state, water.id).unwrap();
    assert_eq!(total(&app, order), 2800);
    let lines = do_list_order_lines(&app.state, order).unwrap();
    assert_eq!(
        lines.iter().map(|l| l.id).collect::<Vec<_>>(),
        vec![beef.id]
    );
    // The journal keeps the removed line, netted to zero.
    assert_eq!(journal(&app, order), vec![(water.id, 0), (beef.id, 1)]);
}

#[tokio::test]
async fn unknown_lines_are_reported_as_such() {
    let app = app().await;
    assert_eq!(
        do_remove_order_line(&app.state, 31_337).unwrap_err(),
        errors::LINE_NOT_FOUND
    );
    assert_eq!(
        do_update_order_line_quantity(&app.state, 31_337, 2).unwrap_err(),
        errors::LINE_NOT_FOUND
    );
}

#[tokio::test]
async fn a_paid_order_can_no_longer_change() {
    let app = app().await;
    let order = start(&app, Some(1));
    let line = add(&app, order, 1, 1).unwrap();
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let paid = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(paid.success, "{:?}", paid.error);

    assert!(add(&app, order, 1, 1).is_err());
    assert!(do_update_order_line_quantity(&app.state, line.id, 3).is_err());
    assert!(do_remove_order_line(&app.state, line.id).is_err());
    let lines = do_list_order_lines(&app.state, order).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].quantity, 1);
    assert_eq!(total(&app, order), 1200);
    assert!(do_list_open_orders(&app.state).unwrap().is_empty());
}

#[tokio::test]
async fn the_total_always_equals_the_sum_of_the_lines() {
    let app = app().await;
    let order = start(&app, Some(1));
    let mut ids = Vec::new();
    for (step, item) in [1, 2, 4, 1, 2].into_iter().enumerate() {
        ids.push(add(&app, order, item, (step as i32 % 3) + 1).unwrap().id);
    }
    do_update_order_line_quantity(&app.state, ids[1], 5).unwrap();
    do_remove_order_line(&app.state, ids[3]).unwrap();
    do_update_order_line_quantity(&app.state, ids[0], 1).unwrap();
    let lines = do_list_order_lines(&app.state, order).unwrap();
    let sum: i64 = lines.iter().map(|l| l.line_total()).sum();
    assert_eq!(total(&app, order), sum);
    // The journal nets to the lines still on the order.
    let journal = journal(&app, order);
    for l in &lines {
        assert!(
            journal.contains(&(l.id, i64::from(l.quantity))),
            "{journal:?}"
        );
    }
}
