//! Crash safety: what happens when something goes wrong strictly
//! AFTER the FDM has successfully signed an event, but BEFORE the
//! local commit finishes.
//!
//! There's no portable way to kill the process mid-`fsync` from a
//! Rust integration test, so this simulates the same failure mode a
//! real crash (power loss, disk full, a corrupted index) would
//! produce: the local SQLite write that follows a successful sign
//! fails. Concretely, each test pre-seeds a `fiscal_events` row that
//! collides with the `UNIQUE(event_type, sequence_number)` constraint
//! on the exact ticket number the command is about to archive, then
//! lets the FDM sign for real (`assert_fdm_was_called` proves the sign
//! genuinely happened) before the local `INSERT` fails.
//!
//! `commit_signed_event` (and the equivalent inline blocks in
//! `checkout.rs`/`report.rs`) do the counter advance, the archive
//! insert, and any order/report bookkeeping inside ONE SQLite
//! transaction specifically so that a failure at ANY point rolls back
//! ALL of it — this file proves that holds even when the failure comes
//! from the database itself, not from the FDM.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::report::do_generate_z_report;
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

/// Pre-seed a row that collides with `(event_type, sequence_number)`
/// on the very ticket number the command under test is about to
/// archive — forcing its `INSERT` to fail with the same
/// `SQLITE_CONSTRAINT` a genuinely corrupted index would produce.
fn poison_sequence_number(app: &TestApp, event_type: &str, sequence_number: i64) {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT INTO fiscal_events
            (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
         VALUES (?1, ?2, 'PRESEEDED', 'PRESEEDED', ?3, '{}')",
        rusqlite::params![event_type, sequence_number, TEST_INSZ],
    )
    .expect("seed the colliding row");
}

#[tokio::test]
async fn checkout_local_commit_failure_after_a_successful_sign_leaves_no_partial_state() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    // The counter starts at 0, so `peek_next` will hand out ticket 1 —
    // pre-occupy exactly that slot.
    poison_sequence_number(&app, "N", 1);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash).await;
    assert!(
        result.is_err(),
        "a local commit failure after a successful sign must surface as an error"
    );

    // The FDM really did sign it — the failure is strictly local,
    // after the point of no return on the FDM's side.
    assert_fdm_was_called(&app).await;

    // Nothing from this attempt persisted: the counter increment
    // rolled back (still 0 — `peek_next` would hand out 1 again on
    // retry, colliding with nothing once the test's own poison row is
    // ignored), only the pre-seeded row exists, and the order is still
    // open.
    assert_eq!(
        app.counter_for(EventLabel::N),
        0,
        "the counter increment must roll back with the rest of the transaction"
    );
    assert_eq!(
        app.fiscal_events().len(),
        1,
        "only the pre-seeded poison row — nothing from the failed attempt"
    );
    assert_eq!(
        app.order_status(order_id).as_deref(),
        Some("open"),
        "the order-close UPDATE must roll back too — same transaction"
    );
}

#[tokio::test]
async fn z_report_local_commit_failure_on_the_second_archive_rolls_back_the_first_too() {
    let app = TestApp::new().await;
    // The turnover Z will get ticket 1, the user Z ticket 2 (M184
    // pattern: `report.rs::do_generate_z_report`, "un = tn + 1"). Only
    // poison the SECOND slot: the first archive must succeed
    // in-transaction before the failure hits, which is exactly what
    // proves the rollback also undoes an EARLIER successful write in
    // the same transaction, not just the one that failed.
    poison_sequence_number(&app, "R", 2);
    mock_from_fixture(&app, &load_uc(&reports_root(), "M184")).await;

    let result = do_generate_z_report(&app.state, TEST_INSZ.to_string()).await;
    assert!(
        result.is_err(),
        "a local commit failure on the second (user) archive must surface as an error"
    );
    assert_fdm_was_called(&app).await;

    assert_eq!(
        app.counter_for(EventLabel::R),
        0,
        "BOTH counter increments must roll back, including the turnover Z's, \
         even though its own archive INSERT succeeded before the user Z's failed"
    );
    assert_eq!(
        app.fiscal_events().len(),
        1,
        "only the pre-seeded poison row — the turnover Z event must not survive \
         a rollback of the transaction it was written in"
    );
    let z_reports: i64 = {
        let conn = app.state.db.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM z_reports", [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(
        z_reports, 0,
        "the z_reports row (written after both archives) must not survive either"
    );

    // And a retry against a healthy DB (drop the poison) must succeed
    // cleanly, from ticket 1 — no gap was ever observable outside this
    // process.
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute("DELETE FROM fiscal_events WHERE pos_id = 'PRESEEDED'", [])
            .unwrap();
    }
    mock_from_fixture(&app, &load_uc(&reports_root(), "M184")).await;
    let retried = do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(retried.success, "{:?}", retried.error);
    let events = app.fiscal_events();
    let r_events: Vec<_> = events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(r_events.len(), 2);
    assert_eq!(r_events[0].sequence_number, 1, "no gap — retry starts at 1");
    assert_eq!(r_events[1].sequence_number, 2);
}
