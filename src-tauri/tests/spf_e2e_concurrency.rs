//! Two sign calls firing at once on the same order/label.
//!
//! moyun-pos is a single-terminal POS (CLAUDE.md: "Multi-terminal
//! sync — single terminal for MVP"), so this is a narrow scenario in
//! practice — a UI double-tap slipping past its loading guard, a
//! stuck request the operator retries while the first is still
//! in-flight — but the task asks for it explicitly, and getting it
//! wrong would be bad in a very specific way: `commit_signed_event`
//! (and the equivalent inline blocks in `checkout.rs`/`refund.rs`/
//! `report.rs`/`invoice.rs`) commit the counter and the archive in one
//! transaction, but TWO callers can both *peek* the same ticket number
//! before either commits.
//!
//! What we found and fixed: those five sites used to assert the
//! invariant with `debug_assert_eq!`. Under this exact race the loser
//! would panic *while still holding the `state.db` MutexGuard* —
//! poisoning that mutex for the rest of the process's life, meaning
//! every subsequent command (not just the racing one) would start
//! failing. That's a `fix(...)` commit alongside this test, converting
//! every occurrence to a graceful `Err` return. This file is the
//! regression test for it: it proves (1) neither call panics, (2)
//! exactly one wins, (3) nothing is double-archived, (4) the counter
//! never sits ahead of what's archived (a "phantom" advance would be a
//! silent permanent gap), and (5) — the part `debug_assert_eq!` could
//! never guarantee — the app is still alive afterwards.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, CheckoutResult, do_checkout};
use moyun_pos::commands::social::do_work_in;
use moyun_pos::commands::tables::do_sign_table_order;
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
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

fn describe_social(r: &Result<moyun_pos::commands::social::SocialResult, String>) -> String {
    match r {
        Ok(res) => format!("Ok(success={}, error={:?})", res.success, res.error),
        Err(e) => format!("Err({e})"),
    }
}

#[tokio::test]
async fn concurrent_checkouts_of_the_same_order_do_not_double_archive_or_create_a_gap() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    // Both racers need a successful FDM response available.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    // `tokio::join!` runs both futures on the current task, interleaved
    // at their `.await` points — no `tokio::spawn`/`Send` needed, and
    // (unlike `spawn`) a panic in either branch would abort this test
    // function normally rather than being silently swallowed, so this
    // also would have caught the pre-fix `debug_assert_eq!` panic.
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
    // The loser must be a clean error, not a hang or a silent
    // "succeeded" that actually didn't archive anything.
    let losers = [&r1, &r2]
        .iter()
        .filter(|r| !matches!(r, Ok(res) if res.success))
        .count();
    assert_eq!(losers, 1);

    let n_events: Vec<_> = app
        .fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "N")
        .collect();
    assert_eq!(n_events.len(), 1, "must never double-archive the same sale");
    assert_eq!(
        app.counter_for(EventLabel::N),
        1,
        "the counter must exactly match what's archived — a counter sitting \
         ahead of the archive is a silent permanent gap"
    );
    assert_eq!(app.order_status(order_id).as_deref(), Some("closed"));

    // The critical assertion `debug_assert_eq!` could never give us:
    // `state.db` must not be poisoned by the loser's (now-graceful)
    // failure. If it were, every command below would itself return an
    // Err from `state.db.lock()`, not the specific "already closed"
    // domain error we're asserting for.
    let post_race = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("state.db must not be poisoned by the loser's graceful error");
    assert!(!post_race.success);
    assert_eq!(post_race.error.as_deref(), Some("Order already closed"));

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
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    let (r1, r2) = tokio::join!(
        do_sign_table_order(&app.state, order_id),
        do_sign_table_order(&app.state, order_id),
    );

    let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        successes, 1,
        "exactly one concurrent table-order sign must win: {r1:?} / {r2:?}"
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

    // The one journalled entry must be marked sent exactly once — not
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
async fn concurrent_work_in_for_the_same_employee_does_not_double_archive_or_create_a_gap() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;

    let (r1, r2) = tokio::join!(
        do_work_in(&app.state, TEST_INSZ.to_string()),
        do_work_in(&app.state, TEST_INSZ.to_string()),
    );
    // The counter-drift check returns its `Err(String)` straight from
    // `do_work_in` (via `?` inside `do_work_event`), the same as
    // `do_sign_table_order` above — unlike `do_checkout`, which wraps
    // most failures in `Ok(CheckoutResult{success:false,..})`.
    let successes = [&r1, &r2]
        .iter()
        .filter(|r| matches!(r, Ok(res) if res.success))
        .count();
    assert_eq!(
        successes,
        1,
        "exactly one concurrent work-in must win: {} / {}",
        describe_social(&r1),
        describe_social(&r2)
    );

    assert_eq!(
        app.fiscal_events()
            .into_iter()
            .filter(|e| e.event_type == "S")
            .count(),
        1
    );
    assert_eq!(app.counter_for(EventLabel::S), 1);
}
