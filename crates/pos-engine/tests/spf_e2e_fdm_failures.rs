//! Exhaustive FDM-failure matrix for every fiscal command.
//!
//! `commands/mod.rs::commit_signed_event` (and the inline equivalents
//! in `checkout.rs`/`refund.rs`) implement "sign-then-commit": the
//! ticket number is only *peeked*, the FDM is called, and the counter
//! is only *committed* - atomically with the archive - once the FDM
//! has actually signed. This file proves that contract holds for
//! EVERY fiscal command (checkout N, refund, copy C, table order P,
//! pre-bill P, money in/out F, drawer open F, work in/out S, X/Z/user
//! reports R) against every way the FDM can fail:
//!
//! - HTTP 500 (`FdmError::Http`, decode error on an empty error body)
//! - a GraphQL error payload (`FdmError::GraphQl`)
//! - a malformed (non-JSON) 200 response (`FdmError::Http`, decode error)
//! - an unreachable FDM (`FdmError::NotConnected`, connection refused)
//!
//! In every case: nothing is archived, the counter for that event's
//! label does not advance, and any order involved stays open/unchanged
//! so the operator can retry. A second attempt against a healthy FDM
//! must then succeed and reuse the SAME ticket number - no gap.
//!
//! "Unreachable" is exercised as an immediate connection refusal
//! (`http://127.0.0.1:1/graphql`, nothing listens there) rather than a
//! real multi-second timeout: `FdmClient`'s own connect/read timeouts
//! are exercised in `pos-fiscal`; here we only care that the POS
//! reacts to *any* network-level failure the same way it reacts to an
//! HTTP-level one, and a real timeout would make this already-large
//! suite slow for no extra coverage.

use pos_engine_testkit as common;

use common::*;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::{
    do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use pos_engine::commands::social::{do_work_in, do_work_out};
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_fiscal::types::EventLabel;

const TEST_INSZ: &str = "12345678901";

// ── Seeding ────────────────────────────────────────────────────────

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

/// Seed an order, sell it through a real (mocked) M110, and return
/// `(order_id, original_fiscal_event_id)`. Used by refund/copy tests
/// that need a prior signed sale to reference.
async fn seed_closed_sale(app: &TestApp) -> (i64, i64) {
    let order_id = seed_minimal_open_order(app);
    mock_from_fixture(app, &load_uc(&basic_set_root(), "M110")).await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("seed sale returns a result");
    assert!(result.success, "seed sale must succeed: {:?}", result.error);
    let original = app.fiscal_events()[0].id;
    (order_id, original)
}

// ── FDM failure induction ─────────────────────────────────────────

/// The three failure shapes reproducible via a mocked HTTP response.
/// "Unreachable" (connection refused) is handled separately below
/// since it isn't a response at all.
#[derive(Clone, Copy)]
enum FdmFailure {
    Http500,
    GraphQlError,
    MalformedJson,
}

const ALL_MOCK_FAILURES: [FdmFailure; 3] = [
    FdmFailure::Http500,
    FdmFailure::GraphQlError,
    FdmFailure::MalformedJson,
];

impl FdmFailure {
    fn label(self) -> &'static str {
        match self {
            FdmFailure::Http500 => "HTTP 500",
            FdmFailure::GraphQlError => "GraphQL error",
            FdmFailure::MalformedJson => "malformed JSON",
        }
    }

    /// Mount a one-shot responder for this failure mode. Callers stack
    /// these in registration order (a failure, then a success fixture)
    /// to model "fails once, then a retry against a healthy FDM".
    async fn mount(self, app: &TestApp) {
        let template = match self {
            FdmFailure::Http500 => ResponseTemplate::new(500),
            FdmFailure::GraphQlError => ResponseTemplate::new(200).set_body_json(json!({
                "errors": [{ "message": "internal FDM error (synthetic, from the test suite)" }]
            })),
            FdmFailure::MalformedJson => {
                ResponseTemplate::new(200).set_body_string("{this is not valid json")
            }
        };
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(template)
            .up_to_n_times(1)
            .mount(&app.fdm)
            .await;
    }
}

/// Point the app at an address nothing listens on, so the very first
/// TCP connect fails (`FdmError::NotConnected`), and return the
/// original URL so the caller can restore it for a retry.
fn make_fdm_unreachable(app: &TestApp) -> String {
    let mut config = app.state.config.lock().unwrap();
    let original = config.fdm_url.clone();
    config.fdm_url = "http://127.0.0.1:1/graphql".to_string();
    original
}

fn restore_fdm_url(app: &TestApp, original: String) {
    app.state.config.lock().unwrap().fdm_url = original;
}

// ── Checkout (N) ───────────────────────────────────────────────────

#[tokio::test]
async fn checkout_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        let order_id = seed_minimal_open_order(&app);
        failure.mount(&app).await;

        let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
            .await
            .expect("checkout returns a result, never panics");

        assert!(!result.success, "{}: checkout must fail", failure.label());
        assert_eq!(
            app.counter_for(EventLabel::N),
            0,
            "{}: N counter must stay at 0",
            failure.label()
        );
        assert_eq!(
            app.fiscal_events().len(),
            0,
            "{}: nothing archived",
            failure.label()
        );
        assert_eq!(
            app.order_status(order_id).as_deref(),
            Some("open"),
            "{}: order stays open for retry",
            failure.label()
        );
    }
}

/// An FDM refusal shaped as the spec's example (code in `extensions`,
/// free text in `message`) reaches the UI with its code, which maps it
/// to the operator's message; nothing is signed.
#[tokio::test]
async fn checkout_refused_with_a_spec_shaped_error_hands_its_code_to_the_ui() {
    for (code, text) in [
        ("BUFFER_FULL", "FDM buffer is full."),
        ("FDM_LOCKED", "FDM is locked."),
    ] {
        let app = TestApp::new().await;
        let order_id = seed_minimal_open_order(&app);
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errors": [{
                    "message": text,
                    "extensions": { "category": "SPF_FOD", "code": code, "showPos": "MANDATORY" }
                }]
            })))
            .mount(&app.fdm)
            .await;

        let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
            .await
            .expect("checkout returns a result");
        assert!(!result.success, "{code}");
        let error = result.error.unwrap_or_default();
        assert!(
            error.contains(&format!("{code}: {text}")),
            "{code}: {error}"
        );
        assert_eq!(app.counter_for(EventLabel::N), 0, "{code}");
        assert_eq!(app.fiscal_events().len(), 0, "{code}");
        assert_eq!(
            app.order_status(order_id).as_deref(),
            Some("open"),
            "{code}"
        );
    }
}

/// An FDM that answers a sale with INTERNAL_ERROR, UNDEFINED_ERROR or
/// UNDEFINED_OTHER may have signed it: the outcome stays unknown (the
/// same request is sent again), but the error carries the FDM's code so
/// that the screens show what it said (certification manual §V).
#[tokio::test]
async fn checkout_with_an_unknown_outcome_error_carries_the_fdm_code() {
    for code in ["INTERNAL_ERROR", "UNDEFINED_ERROR", "UNDEFINED_OTHER"] {
        let app = TestApp::new().await;
        let order_id = seed_minimal_open_order(&app);
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "errors": [{
                    "message": "something failed",
                    "extensions": { "category": "SPF_FOD", "code": code, "showPos": "OPTIONAL" }
                }]
            })))
            .mount(&app.fdm)
            .await;

        let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
            .await
            .expect("checkout returns a result");
        assert!(!result.success, "{code}");
        let error = result.error.unwrap_or_default();
        assert!(
            error.starts_with(&format!(
                "FDM error: FDM_ANSWER_PENDING|label=N|ticket=1|fdm={code}|cause="
            )),
            "{code}: {error}"
        );
        assert_eq!(app.fiscal_events().len(), 0, "{code}");
        assert_eq!(
            app.order_status(order_id).as_deref(),
            Some("open"),
            "{code}"
        );
    }
}

/// A refusal that does not name an FDM code (a dropped answer) has no
/// `fdm` value: the screens show the instruction alone.
#[tokio::test]
async fn checkout_with_a_lost_answer_has_no_fdm_code() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&app.fdm)
        .await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("checkout returns a result");
    let error = result.error.unwrap_or_default();
    assert!(
        error.contains("FDM_ANSWER_PENDING|label=N|ticket=1|cause="),
        "{error}"
    );
    assert!(!error.contains("|fdm="), "{error}");
}

#[tokio::test]
async fn checkout_unreachable_fdm_blocks_the_sale_without_archiving() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    make_fdm_unreachable(&app);

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .expect("checkout returns a result");

    assert!(!result.success);
    assert!(
        !result.fdm_connected,
        "unreachable FDM must report fdm_connected = false"
    );
    assert_eq!(app.counter_for(EventLabel::N), 0);
    assert_eq!(app.fiscal_events().len(), 0);
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
}

#[tokio::test]
async fn checkout_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let first = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!first.success);
    assert_eq!(app.counter_for(EventLabel::N), 0);

    let second = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].sequence_number, 1,
        "retry must reuse ticket number 1 - no gap"
    );
}

// ── Refund (N) ─────────────────────────────────────────────────────

#[tokio::test]
async fn refund_fdm_failure_modes_leave_no_archive_and_counter_unchanged() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        let (_order_id, original) = seed_closed_sale(&app).await;
        assert_eq!(app.counter_for(EventLabel::N), 1);
        failure.mount(&app).await;

        let result = do_refund_checkout(
            &app.state,
            original,
            vec![RefundItem {
                line_id: None,
                menu_item_id: 1,
                quantity: 1,
            }],
            TEST_INSZ.to_string(),
        )
        .await
        .expect("refund returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(
            app.counter_for(EventLabel::N),
            1,
            "{}: N counter must not move past the original sale",
            failure.label()
        );
        assert_eq!(
            app.fiscal_events().len(),
            1,
            "{}: only the original sale archived",
            failure.label()
        );
    }
}

#[tokio::test]
async fn refund_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    let (_order_id, original) = seed_closed_sale(&app).await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;

    let first = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(!first.success);
    assert_eq!(app.counter_for(EventLabel::N), 1);

    let second = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[1].sequence_number, 2,
        "refund retry must reuse ticket number 2 - no gap"
    );
}

// ── Copy (C) ───────────────────────────────────────────────────────

#[tokio::test]
async fn copy_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        let (_order_id, original) = seed_closed_sale(&app).await;
        failure.mount(&app).await;

        let result = do_copy_ticket(&app.state, original, TEST_INSZ.to_string())
            .await
            .expect("copy returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(
            app.counter_for(EventLabel::C),
            0,
            "{}: C counter must stay at 0 - this is the exact bug fixed in \
             commands/copy.rs (it used to call FiscalCounter::next before \
             the FDM call)",
            failure.label()
        );
        assert_eq!(
            app.fiscal_events().len(),
            1,
            "{}: only the original sale archived, no orphan C row",
            failure.label()
        );
    }
}

#[tokio::test]
async fn copy_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    let (_order_id, original) = seed_closed_sale(&app).await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M160")).await;

    let first = do_copy_ticket(&app.state, original, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(!first.success);
    assert_eq!(app.counter_for(EventLabel::C), 0);

    let second = do_copy_ticket(&app.state, original, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "C");
    assert_eq!(
        events[1].sequence_number, 1,
        "copy retry must reuse ticket number 1 - no gap"
    );
}

// ── Table order (P) ────────────────────────────────────────────────

#[tokio::test]
async fn table_order_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        let order_id = seed_minimal_open_order(&app);
        failure.mount(&app).await;

        let result = do_sign_table_order(&app.state, order_id).await;
        assert!(result.is_err(), "{}", failure.label());
        assert_eq!(
            app.counter_for(EventLabel::P),
            0,
            "{}: P counter must stay at 0",
            failure.label()
        );
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
        assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    }
}

#[tokio::test]
async fn table_order_retries_after_fdm_failure_and_resends_the_same_unsent_entries() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;

    do_sign_table_order(&app.state, order_id)
        .await
        .expect_err("first attempt fails");
    assert_eq!(app.counter_for(EventLabel::P), 0);
    // The entry must still be unsent - nothing was lost.
    let unsent = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_entries(&conn, order_id, true).unwrap()
    };
    assert_eq!(
        unsent.len(),
        1,
        "the entry must stay unsent after a failed sign"
    );

    do_sign_table_order(&app.state, order_id)
        .await
        .expect("retry succeeds");
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

// ── Pre-bill (P) ───────────────────────────────────────────────────

#[tokio::test]
async fn pre_bill_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        let order_id = seed_minimal_open_order(&app);
        failure.mount(&app).await;

        let result = do_sign_pre_bill(&app.state, order_id).await;
        assert!(result.is_err(), "{}", failure.label());
        assert_eq!(
            app.counter_for(EventLabel::P),
            0,
            "{}: P counter must stay at 0",
            failure.label()
        );
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

/// The pre-bill flow is two FDM calls (flush unsent entries as an
/// order, then the pre-bill itself). This pins the failure boundary
/// between them: the order-flush succeeds and archives normally, but
/// the pre-bill call fails - only ONE P event must exist, the pre-bill's
/// own ticket must not be burned, and a retry must not re-send the
/// already-flushed entries (no double order-flush) while still using
/// the next sequential ticket number for the pre-bill.
#[tokio::test]
// covers: FAQ_Q202
async fn pre_bill_fdm_failure_on_the_second_step_does_not_burn_its_ticket_or_resend_the_order() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await; // order flush: succeeds
    FdmFailure::Http500.mount(&app).await; // pre-bill: fails

    do_sign_pre_bill(&app.state, order_id)
        .await
        .expect_err("the pre-bill step must fail");

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1, "only the order-flush P event archived");
    assert_eq!(
        app.counter_for(EventLabel::P),
        1,
        "the pre-bill's own ticket must not be consumed on failure"
    );
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));

    // Retry: the entries are already marked sent, so `do_sign_table_order`
    // must be a no-op this time - only the pre-bill call goes out.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M123")).await;
    do_sign_pre_bill(&app.state, order_id)
        .await
        .expect("retry succeeds");

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2, "the retry adds exactly one more P event");
    assert_eq!(
        events[1].sequence_number, 2,
        "pre-bill retry must use ticket 2 - no gap after the order flush"
    );
    assert_eq!(app.counter_for(EventLabel::P), 2);
}

// ── Money in/out (F) ───────────────────────────────────────────────

#[tokio::test]
async fn money_in_out_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_money_in_out(
            &app.state,
            TEST_INSZ.to_string(),
            5000,
            MoneyDirection::In,
            "Cash float opening".to_string(),
        )
        .await
        .expect("money_in_out returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(app.counter_for(EventLabel::F), 0, "{}", failure.label());
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

#[tokio::test]
async fn money_in_out_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M130")).await;

    let first = do_money_in_out(
        &app.state,
        TEST_INSZ.to_string(),
        5000,
        MoneyDirection::In,
        "Cash float opening".to_string(),
    )
    .await
    .unwrap();
    assert!(!first.success);

    let second = do_money_in_out(
        &app.state,
        TEST_INSZ.to_string(),
        5000,
        MoneyDirection::In,
        "Cash float opening".to_string(),
    )
    .await
    .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

// ── Drawer open (F) ────────────────────────────────────────────────

#[tokio::test]
async fn drawer_open_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_open_drawer_fiscal(&app.state, TEST_INSZ.to_string())
            .await
            .expect("open_drawer_fiscal returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(app.counter_for(EventLabel::F), 0, "{}", failure.label());
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

#[tokio::test]
async fn drawer_open_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M131")).await;

    let first = do_open_drawer_fiscal(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(!first.success);

    let second = do_open_drawer_fiscal(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

// ── Work-in / work-out (S) ─────────────────────────────────────────

#[tokio::test]
async fn work_in_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_work_in(&app.state, TEST_INSZ.to_string())
            .await
            .expect("work_in returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(app.counter_for(EventLabel::S), 0, "{}", failure.label());
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

#[tokio::test]
async fn work_out_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_work_out(&app.state, TEST_INSZ.to_string())
            .await
            .expect("work_out returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(app.counter_for(EventLabel::S), 0, "{}", failure.label());
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

#[tokio::test]
async fn work_in_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;

    let first = do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();
    assert!(!first.success);

    let second = do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

#[tokio::test]
async fn work_out_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M141")).await;

    let first = do_work_out(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(!first.success);

    let second = do_work_out(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

// ── X report / user X report / Z report (R) ────────────────────────

#[tokio::test]
async fn x_report_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_generate_x_report(&app.state, TEST_INSZ.to_string())
            .await
            .expect("x-report returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(app.counter_for(EventLabel::R), 0, "{}", failure.label());
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

#[tokio::test]
async fn x_report_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M180")).await;

    let first = do_generate_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(!first.success);

    let second = do_generate_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

#[tokio::test]
async fn user_x_report_fdm_failure_modes_leave_no_archive_and_counter_at_zero() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_generate_user_x_report(&app.state, TEST_INSZ.to_string())
            .await
            .expect("user x-report returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(app.counter_for(EventLabel::R), 0, "{}", failure.label());
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
    }
}

#[tokio::test]
async fn user_x_report_retries_after_fdm_failure_with_the_same_ticket_number() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M182")).await;

    let first = do_generate_user_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(!first.success);

    let second = do_generate_user_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the retry");
}

#[tokio::test]
async fn z_report_fdm_failure_modes_archive_neither_r_event() {
    for failure in ALL_MOCK_FAILURES {
        let app = TestApp::new().await;
        failure.mount(&app).await;

        let result = do_generate_z_report(&app.state, TEST_INSZ.to_string())
            .await
            .expect("z-report returns a result");

        assert!(!result.success, "{}", failure.label());
        assert_eq!(
            app.counter_for(EventLabel::R),
            0,
            "{}: R counter must stay at 0 - the Z report signs turnover+user \
             atomically, so a failure must leave BOTH uncommitted",
            failure.label()
        );
        assert_eq!(app.fiscal_events().len(), 0, "{}", failure.label());
        let z_reports: i64 = {
            let conn = app.state.db.lock().unwrap();
            conn.query_row("SELECT COUNT(*) FROM z_reports", [], |r| r.get(0))
                .unwrap()
        };
        assert_eq!(
            z_reports,
            0,
            "{}: no z_reports row without a successful sign",
            failure.label()
        );
    }
}

#[tokio::test]
async fn z_report_retries_after_fdm_failure_with_the_same_ticket_numbers() {
    let app = TestApp::new().await;
    FdmFailure::Http500.mount(&app).await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M184")).await;

    let first = do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(!first.success);
    assert_eq!(app.counter_for(EventLabel::R), 0);

    let second = do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert!(second.success, "retry must succeed: {:?}", second.error);
    assert_eq!(second.report_no, Some(1), "still the first Z of the day");
    let events = app.fiscal_events();
    let r_events: Vec<_> = events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(r_events.len(), 2, "both R events archived on the retry");
    assert_eq!(
        r_events[0].sequence_number, 1,
        "turnover Z reuses ticket 1 - no gap"
    );
    assert_eq!(
        r_events[1].sequence_number, 2,
        "user Z reuses ticket 2 - no gap"
    );
}

// ── Sanity: the unreachable helper actually restores cleanly ───────

#[tokio::test]
async fn restoring_the_fdm_url_after_unreachable_lets_the_next_call_succeed() {
    let app = TestApp::new().await;
    let original = make_fdm_unreachable(&app);
    let failed = do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();
    assert!(!failed.success);

    restore_fdm_url(&app, original);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;
    let recovered = do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();
    assert!(recovered.success, "{:?}", recovered.error);
    assert_eq!(app.counter_for(EventLabel::S), 1);
}
