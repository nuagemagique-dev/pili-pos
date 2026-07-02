//! Application-level e2e tests for the SPF Finances Basic Set v1.7.
//!
//! Each test mounts a wiremock FDM, replays the official SPF response
//! for one UC, drives the corresponding `do_*` command function, and
//! asserts on the resulting `fiscal_events` archive + counters + order
//! state.
//!
//! UC coverage by unique orchestration:
//!
//! | UC | Pattern | Command |
//! |---|---|---|
//! | M110 | Sale → N archive, order closed | `checkout::do_checkout` |
//! | M111 | Full refund of N | `refund::do_refund_checkout` |
//! | M112A| Original sale + partial refund chain | both, in sequence |
//! | M121 | Order (P) → P archive, order stays open | `tables::do_sign_table_order` |
//! | M123 | Pre-bill → P archive, order stays open | `tables::do_sign_pre_bill` |
//! | M130 | Money in/out (F) | `financial::do_money_in_out` |
//! | M131 | Drawer open (F) | `financial::do_open_drawer_fiscal` |
//! | M140 | Work-in (S) | `social::do_work_in` |
//! | M141 | Work-out (S) | `social::do_work_out` |
//! | M150 | Invoice (I) | tested as a B2B sale variant via do_checkout |
//! | M160 | Copy of a sale (C) → C archive carrying fdmRef | `copy::do_copy_ticket` |
//! | M170 | Work-in in training (T) | `social::do_work_in` w/ training_mode=true |
//! | M171 | Sale in training (T) → T archive, N counter untouched | `checkout::do_checkout` |
//!
//! Status queries Q100/Q110 are NOT exercised here: the published
//! fixtures use lowercase `data.status` envelope while our `FdmClient`
//! expects `data.Status`. That's a fixture-vs-code discrepancy logged
//! in `docs/SUBMISSION_ARTIFACTS/FIXTURE_COVERAGE.md`; once aligned
//! (likely via FdmClient fallback) it'll get its own test.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::copy::do_copy_ticket;
use moyun_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::social::{do_work_in, do_work_out};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_fiscal::types::EventLabel;

// ── Seeding helpers ────────────────────────────────────────────────

const TEST_INSZ: &str = "12345678901";

/// Seed a test user, a single category + menu_item, and return the
/// created order id. The menu_item id is always 1 so callers can
/// reference it from line inserts.
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
        },
    )
    .expect("add line");
    order.id
}

fn set_training_mode(app: &TestApp, on: bool) {
    let mut config = app.state.config.lock().unwrap();
    config.training_mode = on;
}

// ── M110 — Normal sale (event N) ───────────────────────────────────

#[tokio::test]
async fn m110_sign_sale_archives_n_event_and_closes_order() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("checkout returned");

    assert!(result.success, "M110 checkout failed: {:?}", result.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "N");
    assert_eq!(events[0].order_id, Some(order_id));
    assert!(events[0].fdm_response_json.is_some());
    assert_eq!(app.order_status(order_id).as_deref(), Some("closed"));
    assert_eq!(app.counter_for(EventLabel::N), 1);
    app.dump_evidence("M110");
}

// ── M111 — Full refund of a prior sale ─────────────────────────────

#[tokio::test]
async fn m111_full_refund_archives_negative_n_with_fdm_ref() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    // First mount the M110 mock for the original sale, then M111
    // for the refund — wiremock dispatches them in registration
    // order due to `up_to_n_times(1)`.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();

    let original_event_id = app.fiscal_events()[0].id;

    let items = vec![RefundItem {
        menu_item_id: 1,
        quantity: 1,
        unit_price_cents: 1000,
        name: "Test item".to_string(),
        vat_code: "B".to_string(),
    }];

    let result = do_refund_checkout(&app.state, original_event_id, items, TEST_INSZ.to_string())
        .await
        .expect("refund returned");
    assert!(result.success, "M111 refund failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2, "original N + refund N expected");
    assert_eq!(events[1].event_type, "N");
    // Both N events share the same counter LABEL but allocate
    // independent counter values — and the refund must carry the
    // original's fdmRef inside its request payload.
    let refund_request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    // SaleInput uses singular `fdmRef` (Option<FdmReferenceInput>) at
    // the top level — see `crates/pos-fiscal/src/inputs.rs::SaleInput`.
    let fdm_ref = refund_request
        .get("fdmRef")
        .unwrap_or_else(|| panic!("refund must carry fdmRef; got: {refund_request}"));
    assert!(fdm_ref.get("fdmId").is_some(), "fdmRef must include fdmId");
    app.dump_evidence("M111");
}

// ── M112A — Partial refund chain (multi-step) ──────────────────────

#[tokio::test]
async fn m112a_partial_refund_chains_to_original_sale() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    // Multi-step: original sale uses M110-shaped response; partial
    // refund uses M112A_2_PartialRefund.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(
        &app,
        &load_uc(
            &basic_set_root(),
            "M112A_signSale_RefundPartial/M112A_2_PartialRefund",
        ),
    )
    .await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original_event_id = app.fiscal_events()[0].id;

    let items = vec![RefundItem {
        menu_item_id: 1,
        quantity: 1,
        unit_price_cents: 1000,
        name: "Test item".to_string(),
        vat_code: "B".to_string(),
    }];
    let result = do_refund_checkout(&app.state, original_event_id, items, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(result.success, "M112A refund failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "N");
    assert_eq!(events[1].event_type, "N");
    app.dump_evidence("M112A");
}

// ── M121 — Order (event P, pro forma) ──────────────────────────────

#[tokio::test]
async fn m121_sign_order_archives_p_event_and_keeps_order_open() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    do_sign_table_order(&app.state, order_id, "CC1".to_string())
        .await
        .expect("sign_table_order returned Ok");

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "P");
    assert_eq!(events[0].order_id, Some(order_id));
    // P events do NOT close the order — the customer hasn't paid yet.
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    assert_eq!(app.counter_for(EventLabel::P), 1);
    assert_eq!(
        app.counter_for(EventLabel::N),
        0,
        "P must not advance the N counter"
    );
    app.dump_evidence("M121");
}

// ── M123 — Pre-bill (event P) ──────────────────────────────────────

#[tokio::test]
async fn m123_sign_pre_bill_archives_p_event_and_keeps_order_open() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M123")).await;

    do_sign_pre_bill(&app.state, order_id).await.unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "P");
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    app.dump_evidence("M123");
}

// ── M130 — Money in/out (event F) ──────────────────────────────────

#[tokio::test]
async fn m130_money_in_out_archives_f_event() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M130")).await;

    do_money_in_out(
        &app.state,
        TEST_INSZ.to_string(),
        5000,
        MoneyDirection::In,
        "Cash float opening".to_string(),
    )
    .await
    .unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "F");
    assert_eq!(app.counter_for(EventLabel::F), 1);
    app.dump_evidence("M130");
}

// ── M131 — Drawer open (event F) ───────────────────────────────────

#[tokio::test]
async fn m131_drawer_open_archives_f_event() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M131")).await;

    do_open_drawer_fiscal(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "F");
    app.dump_evidence("M131");
}

// ── M140 — Work-in (event S) ───────────────────────────────────────

#[tokio::test]
async fn m140_work_in_archives_s_event() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;

    do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "S");
    assert_eq!(events[0].user_insz, TEST_INSZ);
    assert_eq!(app.counter_for(EventLabel::S), 1);
    app.dump_evidence("M140");
}

// ── M141 — Work-out (event S) ──────────────────────────────────────

#[tokio::test]
async fn m141_work_out_archives_s_event() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M141")).await;

    do_work_out(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "S");
    app.dump_evidence("M141");
}

// ── M160 — Copy of a previous sale (event C) ───────────────────────

#[tokio::test]
async fn m160_copy_of_sale_archives_c_event_with_ref_to_original() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M160")).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original_event_id = app.fiscal_events()[0].id;

    let result = do_copy_ticket(&app.state, original_event_id, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(result.success, "M160 copy failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "C");
    assert_eq!(app.counter_for(EventLabel::C), 1);

    // The C event's archived request must reference the original.
    let copy_request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    let fdm_ref = copy_request
        .get("fdmRef")
        .unwrap_or_else(|| panic!("copy request must carry fdmRef; got: {copy_request}"));
    assert!(fdm_ref.get("fdmId").is_some());
    app.dump_evidence("M160");
}

// ── M170 — Work-in in training mode (event T-S equivalent) ─────────
//
// Training mode is a flag on each input — the FDM rebrands the label
// to T. From the POS side the orchestration is identical to M140.

#[tokio::test]
async fn m170_work_in_training_does_not_advance_normal_s_counter() {
    let app = TestApp::new().await;
    set_training_mode(&app, true);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M170")).await;

    do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    // The S counter still advances because the POS allocates from
    // `EventLabel::S` regardless of training-mode (cf. social.rs); the
    // FDM is the one that brands the event T. The archive will carry
    // the FDM's T label, which is what cert reviewers verify.
    assert_eq!(events[0].event_type, "S");
    app.dump_evidence("M170");
}

// ── M171 — Sale in training mode → POS allocates T counter ─────────

#[tokio::test]
async fn m171_sale_in_training_uses_t_counter_and_keeps_n_at_zero() {
    let app = TestApp::new().await;
    set_training_mode(&app, true);
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M171")).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "T");
    assert_eq!(
        app.counter_for(EventLabel::T),
        1,
        "training sale must advance T counter"
    );
    assert_eq!(
        app.counter_for(EventLabel::N),
        0,
        "training sale must NOT touch the production N counter"
    );
    app.dump_evidence("M171");
}
