//! Application-level e2e tests for the SPF Finances Basic Set v1.8.
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
//! | M112B| Original sale + partial refund as SEPARATE event (declared pattern, cf. FLUX_REMBOURSEMENT_FR.md) | both, in sequence |
//! | M121 | Order (P) → P archive, order stays open | `tables::do_sign_table_order` |
//! | M123 | Pre-bill → P archive, order stays open | `tables::do_sign_pre_bill` |
//! | M130 | Money in/out (F) | `financial::do_money_in_out` |
//! | M131 | Drawer open (F) | `financial::do_open_drawer_fiscal` |
//! | M140 | Work-in (S) | `social::do_work_in` |
//! | M141 | Work-out (S) | `social::do_work_out` |
//! | M150 | Invoice (I) referencing prior sales | `invoice::do_sign_invoice_fiscal` |
//! | M160 | Copy of a sale (C) → C archive carrying fdmRef | `copy::do_copy_ticket` |
//! | M170 | Work-in in training (T) | `social::do_work_in` w/ training_mode=true |
//! | M171 | Sale in training (T) → T archive, N counter untouched | `checkout::do_checkout` |
//! | M161 | Copy of an order (C referencing a P event) | `copy::do_copy_ticket` |
//! | M172 | Copy in training mode | `copy::do_copy_ticket` w/ training_mode=true |
//!
//! Status queries Q100/Q110 are exercised at the client level in
//! `pos-fiscal/tests/client_status.rs` (dual `Status`/`status`
//! spelling fallback + Q110 multi-language aliases) and against the
//! published fixtures in `uc_fixtures.rs`.

mod common;

use common::*;

use pili_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pili_pos::commands::copy::do_copy_ticket;
use pili_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
#[cfg(feature = "peppol")]
use pili_pos::commands::invoice::do_sign_invoice_fiscal;
use pili_pos::commands::refund::{RefundItem, do_refund_checkout};
use pili_pos::commands::social::{do_work_in, do_work_out};
use pili_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
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
            department_id: "1",
            department_name: "Test",
        },
    )
    .expect("add line");
    order.id
}

fn set_training_mode(app: &TestApp, on: bool) {
    let mut config = app.state.config.lock().unwrap();
    config.training_mode = on;
}

// ── M110 - Normal sale (event N) ───────────────────────────────────

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

/// The ticket's FDM reference and VAT lines reach the UI in the FDM's camelCase
/// (`FdmReference`, `VatCalcItem` in ui/src/types/index.ts), unlike the rest of the
/// IPC: the success screen once read `fdm_id` and showed an empty
/// "FDM:" against module2.
#[tokio::test]
async fn the_ticket_fdm_reference_reaches_the_ui_in_camel_case() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("checkout returned");

    let ipc = serde_json::to_value(&result).unwrap();
    let fdm_ref = ipc["ticket"]["fdm_ref"].as_object().expect("fdm_ref");
    let mut keys: Vec<&str> = fdm_ref.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "eventCounter",
            "eventLabel",
            "fdmDateTime",
            "fdmId",
            "totalCounter"
        ]
    );
    assert!(fdm_ref["fdmId"].as_str().is_some_and(|id| !id.is_empty()));
    let vat = ipc["ticket"]["vat_breakdown"][0]
        .as_object()
        .expect("vat line");
    for key in ["taxableAmount", "vatAmount", "totalAmount", "outOfScope"] {
        assert!(vat.contains_key(key), "{key} in {vat:?}");
    }
}

// ── M111 - Full refund of a prior sale ─────────────────────────────

#[tokio::test]
async fn m111_full_refund_archives_negative_n_with_fdm_ref() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    // First mount the M110 mock for the original sale, then M111
    // for the refund - wiremock dispatches them in registration
    // order due to `up_to_n_times(1)`.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();

    let original_event_id = app.fiscal_events()[0].id;

    let items = vec![RefundItem {
        line_id: None,
        menu_item_id: 1,
        quantity: 1,
    }];

    let result = do_refund_checkout(&app.state, original_event_id, items, TEST_INSZ.to_string())
        .await
        .expect("refund returned");
    assert!(result.success, "M111 refund failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2, "original N + refund N expected");
    assert_eq!(events[1].event_type, "N");
    // Both N events share the same counter LABEL but allocate
    // independent counter values - and the refund must carry the
    // original's fdmRef inside its request payload.
    let refund_request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    // SaleInput uses singular `fdmRef` (Option<FdmReferenceInput>) at
    // the top level - see `crates/pos-fiscal/src/inputs.rs::SaleInput`.
    let fdm_ref = refund_request
        .get("fdmRef")
        .unwrap_or_else(|| panic!("refund must carry fdmRef; got: {refund_request}"));
    assert!(fdm_ref.get("fdmId").is_some(), "fdmRef must include fdmId");
    app.dump_evidence("M111");
}

// ── M112A - Partial refund chain (multi-step) ──────────────────────

#[tokio::test]
async fn m112b_partial_refund_is_separate_event_chained_to_original() {
    // The dossier declares the M112B pattern (FLUX_REMBOURSEMENT_FR.md):
    // a partial refund is ALWAYS its own fiscal event referencing the
    // original sale - never mixed with new-sale lines on one ticket
    // (that mixed pattern is M112A, which this POS does not offer).
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    // Multi-step: original sale uses M110-shaped response; partial
    // refund uses M112B_2_PartialRefund.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(
        &app,
        &load_uc(
            &basic_set_root(),
            "M112B_signSale_RefundPartial/M112B_2_PartialRefund",
        ),
    )
    .await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original_event_id = app.fiscal_events()[0].id;

    let items = vec![RefundItem {
        line_id: None,
        menu_item_id: 1,
        quantity: 1,
    }];
    let result = do_refund_checkout(&app.state, original_event_id, items, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(result.success, "M112B refund failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "N");
    assert_eq!(events[1].event_type, "N");
    // The refund is a SEPARATE event carrying only negative lines,
    // chained to the original via fdmRef (M112B pattern).
    let refund_request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    assert!(
        refund_request.get("fdmRef").is_some(),
        "partial refund must reference the original sale"
    );
    let lines = refund_request["transaction"]["transactionLines"]
        .as_array()
        .unwrap();
    assert!(
        lines
            .iter()
            .all(|l| l["mainProduct"]["quantity"].as_f64().unwrap() < 0.0),
        "M112B: refund event must not mix positive (new sale) lines"
    );
    app.dump_evidence("M112B");
}

// ── M121 - Order (event P, pro forma) ──────────────────────────────

#[tokio::test]
async fn m121_sign_order_archives_p_event_and_keeps_order_open() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    do_sign_table_order(&app.state, order_id)
        .await
        .expect("sign_table_order returned Ok");

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "P");
    assert_eq!(events[0].order_id, Some(order_id));
    // P events do NOT close the order - the customer hasn't paid yet.
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    assert_eq!(app.counter_for(EventLabel::P), 1);
    assert_eq!(
        app.counter_for(EventLabel::N),
        0,
        "P must not advance the N counter"
    );
    app.dump_evidence("M121");
}

// ── M123 - Pre-bill (event P) ──────────────────────────────────────

#[tokio::test]
// covers: FAQ_Q202
async fn m123_sign_pre_bill_archives_p_event_and_keeps_order_open() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    // The lines not yet registered go out as an order first (§1.4: a
    // pre-bill may not add lines), then the pre-bill itself.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M123")).await;

    do_sign_pre_bill(&app.state, order_id).await.unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.event_type == "P"));
    assert_eq!(app.counter_for(EventLabel::P), 2);
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    app.dump_evidence("M123");
}

// ── M130 - Money in/out (event F) ──────────────────────────────────

#[tokio::test]
// covers: FAQ_Q401
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

// ── M131 - Drawer open (event F) ───────────────────────────────────

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

// ── M140 - Work-in (event S) ───────────────────────────────────────

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

// ── M141 - Work-out (event S) ──────────────────────────────────────

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

// ── M160 - Copy of a previous sale (event C) ───────────────────────

#[tokio::test]
// covers: FAQ_Q503
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

// ── M170 - Work-in in training mode (event T) ──────────────────────
//
// Every event recorded in training mode is a training event with label
// T (POS detailed description v2.2 §1.5): counted and archived as T.

#[tokio::test]
async fn m170_work_in_training_does_not_advance_normal_s_counter() {
    let app = TestApp::new().await;
    set_training_mode(&app, true);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M170")).await;

    do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "T");
    assert_eq!(app.counter_for(EventLabel::S), 0);
    assert_eq!(app.counter_for(EventLabel::T), 1);
    app.dump_evidence("M170");
}

// ── M171 - Sale in training mode → POS allocates T counter ─────────

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

// ── M150 - Invoice (event I) referencing prior sales ───────────────

// Invoices ship with the `peppol` feature, outside the v1.0.0 cert build.
#[cfg(feature = "peppol")]
#[tokio::test]
async fn m150_sign_invoice_archives_i_event_referencing_the_sale() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M150")).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let sale_event_id = app.fiscal_events()[0].id;

    let result = do_sign_invoice_fiscal(
        &app.state,
        vec![sale_event_id],
        "BE0308357159".to_string(),
        TEST_INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(result.success, "M150 invoice failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "I");
    assert_eq!(app.counter_for(EventLabel::I), 1);
    assert_eq!(Some(events[1].id), result.fiscal_event_id);

    // The archived I request must carry the invoice number, the
    // customer VAT and the fdmRefs of the underlying sale (M150
    // fixture shape).
    let request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    assert_eq!(request["customerVatNo"], "BE0308357159");
    assert_eq!(
        request["invoiceNo"].as_str(),
        result.invoice_no.as_deref(),
        "archived invoiceNo must match the returned one"
    );
    let refs = request["fdmRefs"].as_array().expect("fdmRefs array");
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["eventLabel"], "N");
    app.dump_evidence("M150");
}

// Invoices ship with the `peppol` feature, outside the v1.0.0 cert build.
#[cfg(feature = "peppol")]
#[tokio::test]
async fn m150_invoice_fdm_failure_does_not_advance_i_counter() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    // Only the sale is mocked - the signInvoice call hits an
    // unmatched route and fails, like an unreachable FDM.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let sale_event_id = app.fiscal_events()[0].id;

    let result = do_sign_invoice_fiscal(
        &app.state,
        vec![sale_event_id],
        "BE0308357159".to_string(),
        TEST_INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(!result.success, "invoice must fail when the FDM refuses");
    assert_eq!(
        app.counter_for(EventLabel::I),
        0,
        "failed signInvoice must not create a counter gap"
    );
    assert_eq!(app.fiscal_events().len(), 1, "no I event archived");
}

// ── M161 - Copy of an order (event C referencing a P event) ────────

#[tokio::test]
async fn m161_copy_of_order_archives_c_event_with_ref_to_p_event() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M161")).await;

    do_sign_table_order(&app.state, order_id).await.unwrap();
    let p_event_id = app.fiscal_events()[0].id;

    let result = do_copy_ticket(&app.state, p_event_id, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(result.success, "M161 copy failed: {:?}", result.error);

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "C");
    assert_eq!(app.counter_for(EventLabel::C), 1);

    // The C event must reference the ORIGINAL P event.
    let copy_request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    assert_eq!(
        copy_request["fdmRef"]["eventLabel"], "P",
        "copy of an order must chain to the P event"
    );
    app.dump_evidence("M161");
}

// ── M172 - Copy in training mode ────────────────────────────────────

#[tokio::test]
async fn m172_copy_in_training_mode_archives_c_event() {
    let app = TestApp::new().await;
    set_training_mode(&app, true);
    let order_id = seed_minimal_open_order(&app);

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M171")).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M172")).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let t_event_id = app.fiscal_events()[0].id;

    let result = do_copy_ticket(&app.state, t_event_id, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(
        result.success,
        "M172 training copy failed: {:?}",
        result.error
    );

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    // The archive carries the FDM's label - T in training mode (the
    // published M172 response brands the copy T).
    let copy_request: serde_json::Value =
        serde_json::from_str(events[1].event_data_json.as_ref().unwrap()).unwrap();
    assert_eq!(
        copy_request["fdmRef"]["eventLabel"], "T",
        "training copy must chain to the T sale"
    );
    assert!(
        copy_request["isTraining"].is_null(),
        "training rides the isTraining GraphQL variable, not the input"
    );
    app.dump_evidence("M172");
}
