//! Every fiscal command refuses an FDM response that does not answer
//! its request.
//!
//! The FDM signs what the POS sends and echoes its identity back
//! (`posId`, `posFiscalTicketNo`, `posDateTime`, `deviceId`,
//! `terminalId`), with the event operation and label of the request
//! and, for a normal sale, a short signature derived from the digital
//! signature (`pos_fiscal::response_check`). A response that breaks
//! any of these must never become a fiscal record.
//!
//! Each case replays an official response, echoed like a real FDM
//! does, then breaks one field. In every case: nothing is archived, the
//! counter does not move, the order stays open, nothing is printed, and
//! the error carries `FDM_RESPONSE_MISMATCH` for the screen. The
//! request stays open: the next operation sends it again unchanged,
//! and a consistent answer archives it under the same ticket number.

mod common;

use common::*;

use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, Request, ResponseTemplate};

use pili_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pili_pos::commands::copy::do_copy_ticket;
use pili_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use pili_pos::commands::refund::{RefundItem, do_refund_checkout};
use pili_pos::commands::report::{
    do_generate_user_x_report, do_generate_x_report, do_generate_z_report, do_list_z_reports,
};
use pili_pos::commands::social::do_work_in;
use pili_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_fiscal::types::EventLabel;

const TEST_INSZ: &str = "12345678901";
const CODE: &str = "FDM_RESPONSE_MISMATCH";

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

async fn seed_closed_sale(app: &TestApp) -> i64 {
    let order_id = seed_minimal_open_order(app);
    mock_from_fixture(app, &load_uc(&basic_set_root(), "M110")).await;
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "seed sale: {:?}", result.error);
    app.fiscal_events()[0].id
}

// ── Doctored answers ───────────────────────────────────────────────

/// One way of breaking a consistent answer.
type Breakage = fn(&mut Value);

fn set(result: &mut Value, key: &str, value: Value) {
    result[key] = value;
}

const WRONG_POS: Breakage = |r| set(r, "posId", "CPOS0031234567".into());
const WRONG_TICKET: Breakage = |r| {
    let n = r["posFiscalTicketNo"].as_i64().unwrap();
    set(r, "posFiscalTicketNo", (n + 1).into());
};
const WRONG_DATE: Breakage = |r| set(r, "posDateTime", "2020-01-01T00:00:00+01:00".into());
const WRONG_DEVICE: Breakage = |r| set(r, "deviceId", "ff:ff:ff:ff:ff:ff".into());
const WRONG_TERMINAL: Breakage = |r| set(r, "terminalId", "ANOTHER".into());
const WRONG_OPERATION: Breakage = |r| set(r, "eventOperation", "ORDER".into());
const TRAINING_LABEL: Breakage = |r| r["fdmRef"]["eventLabel"] = "T".into();
const WRONG_SHORT_SIGNATURE: Breakage = |r| {
    set(
        r,
        "shortSignature",
        "ca39a3ee5e6b4b0d3255bfef95601890afd80709".into(),
    )
};
const NO_SHORT_SIGNATURE: Breakage = |r| set(r, "shortSignature", Value::Null);
const NO_VERIFICATION_URL: Breakage = |r| set(r, "verificationUrl", Value::Null);
const NO_SIGNATURE: Breakage = |r| set(r, "digitalSignature", "".into());

/// Answer the next request with `envelope`, echoed like a real FDM,
/// then broken by `breakage` in the result named `field` (every result
/// when `field` is `None`).
async fn fdm_answers_broken(
    app: &TestApp,
    envelope: Value,
    field: Option<&'static str>,
    breakage: Breakage,
) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(move |request: &Request| {
            let mut answer = EchoRequest::answer(&envelope, &request.body);
            let results = answer["data"].as_object_mut().expect("sign envelope");
            for (name, result) in results.iter_mut() {
                if field.is_none_or(|f| f == name) {
                    breakage(result);
                }
            }
            ResponseTemplate::new(200).set_body_json(answer)
        })
        .up_to_n_times(1)
        .mount(&app.fdm)
        .await;
}

fn envelope(root: &std::path::Path, uc: &str) -> Value {
    read_response_envelope(&load_uc(root, uc))
}

fn assert_refused(error: Option<&str>, what: &str) {
    let error = error.unwrap_or_else(|| panic!("{what}: an error is shown"));
    assert!(
        error.starts_with("FDM error: ") && error.contains(CODE),
        "{what}: {error}"
    );
}

// ── Sale (N): every rule ───────────────────────────────────────────

#[tokio::test]
async fn a_sale_answered_inconsistently_is_refused_for_every_rule() {
    let cases: [(&str, Breakage); 11] = [
        ("another POS", WRONG_POS),
        ("another ticket number", WRONG_TICKET),
        ("another date", WRONG_DATE),
        ("another device", WRONG_DEVICE),
        ("another terminal", WRONG_TERMINAL),
        ("another operation", WRONG_OPERATION),
        ("a training label for a real sale", TRAINING_LABEL),
        (
            "a short signature not derived from the signature",
            WRONG_SHORT_SIGNATURE,
        ),
        ("no short signature", NO_SHORT_SIGNATURE),
        ("no verification URL", NO_VERIFICATION_URL),
        ("no signature", NO_SIGNATURE),
    ];
    for (what, breakage) in cases {
        let app = TestApp::new().await;
        let printer = PrinterSink::start().await;
        app.set_printer(&printer.addr);
        let order_id = seed_minimal_open_order(&app);
        fdm_answers_broken(&app, envelope(&basic_set_root(), "M110"), None, breakage).await;

        let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
            .await
            .expect("checkout returns a result");

        assert!(!result.success, "{what}: the sale is refused");
        assert_refused(result.error.as_deref(), what);
        assert!(app.fiscal_events().is_empty(), "{what}: nothing archived");
        assert_eq!(
            app.counter_for(EventLabel::N),
            0,
            "{what}: counter unchanged"
        );
        assert_eq!(
            app.order_status(order_id).as_deref(),
            Some("open"),
            "{what}: the order stays open"
        );
        assert!(
            printer.settle(0).await.is_empty(),
            "{what}: no ticket is printed"
        );
    }
}

#[tokio::test]
async fn a_training_sale_answered_with_a_real_label_is_refused() {
    let app = TestApp::new().await;
    app.state.config.lock().unwrap().training_mode = true;
    let order_id = seed_minimal_open_order(&app);
    // The published M110 answer is labelled N: in training the FDM
    // must answer T.
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "training sale");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::T), 0);
}

#[tokio::test]
async fn a_sale_is_retried_with_the_same_ticket_number_after_an_inconsistent_answer() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    fdm_answers_broken(
        &app,
        envelope(&basic_set_root(), "M110"),
        None,
        WRONG_TICKET,
    )
    .await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;

    let first = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!first.success);

    let second = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(second.success, "{:?}", second.error);
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].sequence_number, 1, "no gap after the refusal");
    assert_eq!(app.order_status(order_id).as_deref(), Some("closed"));
}

// ── Every other fiscal command ─────────────────────────────────────

#[tokio::test]
async fn a_refund_answered_inconsistently_is_refused() {
    for breakage in [WRONG_TICKET, NO_SHORT_SIGNATURE] {
        let app = TestApp::new().await;
        let original = seed_closed_sale(&app).await;
        fdm_answers_broken(&app, envelope(&basic_set_root(), "M111"), None, breakage).await;

        let result = do_refund_checkout(
            &app.state,
            original,
            vec![RefundItem {
                menu_item_id: 1,
                quantity: 1,
            }],
            TEST_INSZ.to_string(),
        )
        .await
        .unwrap();

        assert!(!result.success);
        assert_refused(result.error.as_deref(), "refund");
        assert_eq!(app.fiscal_events().len(), 1, "only the original sale");
        assert_eq!(app.counter_for(EventLabel::N), 1);
    }
}

#[tokio::test]
async fn a_copy_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    let original = seed_closed_sale(&app).await;
    fdm_answers_broken(&app, envelope(&basic_set_root(), "M160"), None, WRONG_POS).await;

    let result = do_copy_ticket(&app.state, original, TEST_INSZ.to_string())
        .await
        .unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "copy");
    assert_eq!(app.fiscal_events().len(), 1);
    assert_eq!(app.counter_for(EventLabel::C), 0);
}

#[tokio::test]
async fn a_table_order_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    fdm_answers_broken(&app, envelope(&basic_set_root(), "M121"), None, WRONG_DATE).await;

    let error = do_sign_table_order(&app.state, order_id)
        .await
        .expect_err("refused");

    assert_refused(Some(&error), "table order");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::P), 0);
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
}

#[tokio::test]
async fn a_pre_bill_answered_inconsistently_keeps_only_the_order_registration() {
    let app = TestApp::new().await;
    let order_id = seed_minimal_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M121")).await;
    fdm_answers_broken(
        &app,
        envelope(&basic_set_root(), "M123"),
        None,
        WRONG_OPERATION,
    )
    .await;

    let error = do_sign_pre_bill(&app.state, order_id)
        .await
        .expect_err("refused");

    assert_refused(Some(&error), "pre-bill");
    assert_eq!(app.fiscal_events().len(), 1, "the order registration only");
    assert_eq!(app.counter_for(EventLabel::P), 1);
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
}

#[tokio::test]
async fn a_money_movement_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    fdm_answers_broken(
        &app,
        envelope(&basic_set_root(), "M130"),
        None,
        WRONG_DEVICE,
    )
    .await;

    let result = do_money_in_out(
        &app.state,
        TEST_INSZ.to_string(),
        5000,
        MoneyDirection::In,
        "Fond de caisse".to_string(),
    )
    .await
    .unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "money in/out");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::F), 0);
}

#[tokio::test]
async fn a_drawer_opening_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    fdm_answers_broken(
        &app,
        envelope(&basic_set_root(), "M131"),
        None,
        TRAINING_LABEL,
    )
    .await;

    let result = do_open_drawer_fiscal(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "drawer");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::F), 0);
}

#[tokio::test]
async fn a_work_in_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    fdm_answers_broken(
        &app,
        envelope(&basic_set_root(), "M140"),
        None,
        NO_SIGNATURE,
    )
    .await;

    let result = do_work_in(&app.state, TEST_INSZ.to_string()).await.unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "work in");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::S), 0);
}

#[tokio::test]
async fn an_x_report_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    fdm_answers_broken(&app, envelope(&reports_root(), "M180"), None, WRONG_TICKET).await;

    let result = do_generate_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "X report");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.counter_for(EventLabel::R), 0);
}

#[tokio::test]
async fn a_user_x_report_answered_inconsistently_is_refused() {
    let app = TestApp::new().await;
    fdm_answers_broken(&app, envelope(&reports_root(), "M182"), None, WRONG_POS).await;

    let result = do_generate_user_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();

    assert!(!result.success);
    assert_refused(result.error.as_deref(), "user X report");
    assert!(app.fiscal_events().is_empty());
}

#[tokio::test]
async fn a_z_report_with_one_inconsistent_half_archives_neither_half() {
    for half in ["signReportTurnoverZ", "signReportUserZ"] {
        let app = TestApp::new().await;
        fdm_answers_broken(
            &app,
            envelope(&reports_root(), "M184"),
            Some(half),
            WRONG_TICKET,
        )
        .await;

        let result = do_generate_z_report(&app.state, TEST_INSZ.to_string())
            .await
            .unwrap();

        assert!(!result.success, "{half}");
        assert_refused(result.error.as_deref(), half);
        assert!(app.fiscal_events().is_empty(), "{half}: nothing archived");
        assert_eq!(app.counter_for(EventLabel::R), 0, "{half}");
        assert!(
            do_list_z_reports(&app.state).unwrap().is_empty(),
            "{half}: no Z recorded"
        );
    }
}
