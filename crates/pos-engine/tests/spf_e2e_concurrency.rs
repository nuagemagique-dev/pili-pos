//! Two sign calls firing at once on the same order/label.
//!
//! pili-pos is a single-terminal POS, so this is a narrow scenario in
//! practice (a UI double-tap slipping past its loading guard, a stuck
//! request the operator retries while the first is still in flight).
//! Every signing operation runs under one lock (`commands::signing`):
//! the second caller waits, then reads the state the first left. So
//! nothing is ever double-archived, the counter always matches the
//! archive, and the app stays alive afterwards.

use pos_engine_testkit as common;

use common::*;

use pos_engine::commands::errors;

use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, CheckoutResult, do_checkout};
use pos_engine::commands::social::do_work_in;
use pos_engine::commands::tables::do_sign_table_order;
use pos_fiscal::types::EventLabel;

const TEST_INSZ: &str = "12345678901";

fn seed_minimal_open_order(app: &TestApp) -> i64 {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO users (insz, name, role, active)
         VALUES (?1, 'Test', 'cashier', 1)",
        rusqlite::params![TEST_INSZ],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO menu_items
         (id, category_id, price_cents, vat_code, sort_order, active)
         VALUES (1, 1, 1000, 'B', 0, 1)",
        [],
    )
    .unwrap();
    let order = pos_db::queries::orders::create_order(&conn, OrderType::DineIn, Some(1), TEST_INSZ)
        .expect("create order");
    pos_db::queries::orders::add_order_line(
        &conn,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 1,
            quantity: 1,
            unit_price_cents: 1000,
            vat_code: VatCode::B,
            name: "Test item",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .expect("add line");
    order.id
}

fn describe_checkout(r: &Result<CheckoutResult, String>) -> String {
    match r {
        Ok(res) => format!("Ok(success={}, error={:?})", res.success, res.error),
        Err(e) => format!("Err({e})"),
    }
}

fn describe_social(r: &Result<pos_engine::commands::social::SocialResult, String>) -> String {
    match r {
        Ok(res) => format!("Ok(success={}, error={:?})", res.success, res.error),
        Err(e) => format!("Err({e})"),
    }
}

#[tokio::test]
async fn concurrent_checkouts_of_the_same_order_do_not_double_archive_or_create_a_gap() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    // Only the first racer signs: the second finds the order closed.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    // `tokio::join!` runs both futures on the current task, interleaved
    // at their `.await` points; a panic in either branch fails the test.
    let (r1, r2) = tokio::join!(
        do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash),
        do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash),
    );

    let successes = [&r1, &r2]
        .iter()
        .filter(|r| matches!(r, Ok(res) if res.success))
        .count();
    assert_eq!(
        successes,
        1,
        "exactly one of two concurrent checkouts of the same order must win: {} / {}",
        describe_checkout(&r1),
        describe_checkout(&r2)
    );
    // The second waits for the first, then finds the order closed.
    let loser = [&r1, &r2]
        .into_iter()
        .find(|r| !matches!(r, Ok(res) if res.success))
        .unwrap();
    assert!(
        matches!(loser, Ok(res) if res.error.as_deref() == Some(errors::ORDER_ALREADY_CLOSED)),
        "{}",
        describe_checkout(loser)
    );

    let n_events: Vec<_> = app
        .fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "N")
        .collect();
    assert_eq!(n_events.len(), 1, "must never double-archive the same sale");
    assert_eq!(
        app.counter_for(EventLabel::N),
        1,
        "the counter must exactly match what's archived - a counter sitting \
         ahead of the archive is a silent permanent gap"
    );
    assert_eq!(app.order_status(order_id).as_deref(), Some("closed"));

    // `state.db` is not poisoned: a later command gets the domain error.
    let post_race = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("state.db must not be poisoned by the loser's graceful error");
    assert!(!post_race.success);
    assert_eq!(
        post_race.error.as_deref(),
        Some(errors::ORDER_ALREADY_CLOSED)
    );

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;
    let work_in = do_work_in(&app.state, TEST_INSZ.to_string())
        .await
        .expect("the app must still be able to serve unrelated commands");
    assert!(work_in.success, "{:?}", work_in.error);
}

#[tokio::test]
async fn concurrent_table_order_signs_do_not_double_archive_or_create_a_gap() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    let (r1, r2) = tokio::join!(
        do_sign_table_order(&app.state, order_id),
        do_sign_table_order(&app.state, order_id),
    );

    // Both succeed: the second waits, then has nothing left to send.
    assert!(r1.is_ok() && r2.is_ok(), "{r1:?} / {r2:?}");
    assert!(
        [&r1, &r2]
            .iter()
            .any(|r| format!("{r:?}").contains("NothingToPrint")),
        "{r1:?} / {r2:?}"
    );

    let p_events: Vec<_> = app
        .fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "P")
        .collect();
    assert_eq!(
        p_events.len(),
        1,
        "must never double-archive the same order"
    );
    assert_eq!(app.counter_for(EventLabel::P), 1);

    // The one journalled entry must be marked sent exactly once - not
    // left dangling, not double-consumed.
    let unsent = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_entries(&conn, order_id, true).unwrap()
    };
    assert!(
        unsent.is_empty(),
        "the entry must be marked sent by the winner"
    );

    // And the app survives: a second, later sign is correctly a no-op
    // (nothing new to send), not an error.
    do_sign_table_order(&app.state, order_id)
        .await
        .expect("state.db must not be poisoned; a later sign is a clean no-op");
    assert_eq!(app.fiscal_events().len(), 1, "no-op must not archive again");
}

#[tokio::test]
async fn concurrent_work_ins_are_signed_one_after_the_other() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;

    let (r1, r2) = tokio::join!(
        do_work_in(&app.state, TEST_INSZ.to_string()),
        do_work_in(&app.state, TEST_INSZ.to_string()),
    );
    // Clocking in twice is allowed: both are signed, one after the
    // other, with consecutive numbers.
    for r in [&r1, &r2] {
        assert!(
            matches!(r, Ok(res) if res.success),
            "{}",
            describe_social(r)
        );
    }
    let numbers: Vec<i64> = app
        .fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "S")
        .map(|e| e.sequence_number)
        .collect();
    assert_eq!(numbers, vec![1, 2]);
    assert_eq!(app.counter_for(EventLabel::S), 2);
}
