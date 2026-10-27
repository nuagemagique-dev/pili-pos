//! Tests of the reports facade (report, social, financial, settings, `fdm_monitor`).
//!
//! (a) Contract tests: every output type from a real engine value (a command run on a `TestApp`
//! against the admission FDM mock) and from hand-built values with every `Option` set and empty;
//! the engine's serde form and the FFI record's must be equal. The one input type, `MoneyDirection`,
//! is converted variant by variant.
//!
//! (b) Wiring tests: each of the 18 methods is called through a `PosEngine` opened on the seeded
//! database (`support::open_seeded`). The facade has no login (that is the auth domain), so a
//! command that needs an operator answers `not_logged_in` on a plain engine; to run them for real
//! the gated tests keep an operator session open: `end_shift` (PIN check, session open, clock-out
//! signed, session closed) is held on the FDM at its clock-out, and the commands run while it waits.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use pos_core::user::UserRole;
use pos_engine::commands::financial::{
    FinancialResult as EngineFinancialResult, MoneyDirection as EngineMoneyDirection,
    do_money_in_out, do_open_drawer_fiscal,
};
use pos_engine::commands::report::{
    ReportResult as EngineReportResult, do_generate_user_x_report, do_generate_x_report,
    do_generate_z_report, do_list_z_reports,
};
use pos_engine::commands::sales_report::{
    SalesDay as EngineSalesDay, SalesHour as EngineSalesHour, SalesProduct as EngineSalesProduct,
    SalesReport as EngineSalesReport, do_sales_report,
};
use pos_engine::commands::settings::{PosSettings as EnginePosSettings, do_get_settings};
use pos_engine::commands::social::{
    SocialResult as EngineSocialResult, do_list_open_shifts, do_work_in,
};
use pos_engine_testkit::TestApp;
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::records::reports as ffi;
use pos_ffi::{PosEngine, PosError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, to_value};
use tokio::sync::Notify;

use crate::support::{self, MANAGER_INSZ, MANAGER_PIN, TestEngine};

const INSZ: &str = "12345678958";
/// A port nothing listens on: the connection is refused at once.
const CLOSED_PORT_FDM: &str = "http://127.0.0.1:1/graphql";

// ───────────────────────────── helpers ─────────────────────────────

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

/// Engine value to FFI record through `convert::out`; the two serde forms must be equal.
fn out_equal<E: Serialize, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = contract_out(engine).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        json_of(engine),
        json_of(&record),
        "engine vs FFI serde form"
    );
    record
}

fn coded(err: &PosError, expected: &str) -> bool {
    matches!(err, PosError::Coded { code, .. } if code == expected)
}

/// A `TestApp` that signs on the admission FDM mock, with the operator logged in as admin.
async fn app_on_mock() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(INSZ, UserRole::Admin);
    app
}

/// The same, with the FDM unreachable.
async fn app_without_fdm() -> TestApp {
    let app = app_on_mock().await;
    app.state.config.lock().unwrap().fdm_url = CLOSED_PORT_FDM.to_string();
    app
}

// ───────────────────────────── ReportResult ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn report_results_of_real_reports_convert() {
    let app = app_on_mock().await;

    let x = do_generate_x_report(&app.state, INSZ.into()).await.unwrap();
    assert!(x.success, "X: {:?}", x.error);
    let record: ffi::ReportResult = out_equal(&x);
    assert_eq!(record.report_type, "X");
    assert_eq!(record.report_no, None);
    assert_eq!(record.error, None);

    let user_x = do_generate_user_x_report(&app.state, INSZ.into())
        .await
        .unwrap();
    assert!(user_x.success, "user X: {:?}", user_x.error);
    let record: ffi::ReportResult = out_equal(&user_x);
    assert_eq!(record.report_type, "X-USER");

    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    assert!(z.success, "Z: {:?}", z.error);
    let record: ffi::ReportResult = out_equal(&z);
    assert_eq!(record.report_type, "Z");
    assert_eq!(record.report_no, Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_report_keeps_its_error_text() {
    let app = app_without_fdm().await;
    let x = do_generate_x_report(&app.state, INSZ.into()).await.unwrap();
    assert!(!x.success);
    let record: ffi::ReportResult = out_equal(&x);
    assert!(!record.success);
    assert!(record.error.is_some());
}

#[test]
fn report_results_with_every_option_set_or_empty_convert() {
    for (report_no, error) in [(Some(12), Some("fdm refused".to_string())), (None, None)] {
        let engine = EngineReportResult {
            success: error.is_none(),
            report_type: "Z".into(),
            report_no,
            error,
        };
        let record: ffi::ReportResult = out_equal(&engine);
        assert_eq!(record.report_no, report_no);
    }
}

// ───────────────────────────── SalesReport ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn the_sales_report_of_a_real_day_converts() {
    let app = app_on_mock().await;
    let day = pos_fiscal::event::now_in_belgium().date_naive().to_string();
    let report = do_sales_report(&app.state, day.clone(), day.clone()).unwrap();
    let record: ffi::SalesReport = out_equal(&report);
    assert_eq!(record.from, day);
    assert_eq!(record.total_cents, 0);
    assert!(record.days.is_empty());
}

#[test]
fn a_sales_report_with_every_field_set_converts() {
    let engine = EngineSalesReport {
        from: "2026-10-16".into(),
        to: "2026-10-17".into(),
        generated_at: "2026-10-17T18:30:00+02:00".into(),
        business_date: "2026-10-17".into(),
        days: vec![
            EngineSalesDay {
                booking_date: "2026-10-16".into(),
                z_number: Some(12),
                total_cents: 1550,
                z_total_cents: Some(1550),
                open: false,
                matches_z: true,
                gap_cents: Some(0),
            },
            EngineSalesDay {
                booking_date: "2026-10-17".into(),
                z_number: None,
                total_cents: 400,
                z_total_cents: None,
                open: true,
                matches_z: false,
                gap_cents: None,
            },
        ],
        open_period: true,
        by_product: vec![EngineSalesProduct {
            product_id: "10006".into(),
            name: "Maki".into(),
            department: "Plats".into(),
            quantity_milli: -2000,
            amount_cents: -1300,
        }],
        by_hour: vec![EngineSalesHour {
            hour: 19,
            tickets: 3,
            amount_cents: 1950,
            bar_percent: 100,
        }],
        price_changes_cents: -50,
        rounding_cents: 2,
        refunds_cents: -1300,
        ticket_count: 3,
        total_cents: 1950,
    };
    let record: ffi::SalesReport = out_equal(&engine);
    assert_eq!(record.days.len(), 2);
    assert_eq!(record.by_hour[0].hour, 19);
}

// ───────────────────────────── ZReportSummary ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn z_report_summaries_of_a_real_history_convert() {
    let app = app_on_mock().await;
    assert!(do_list_z_reports(&app.state).unwrap().is_empty());
    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    assert!(z.success, "Z: {:?}", z.error);
    let rows = do_list_z_reports(&app.state).unwrap();
    assert_eq!(rows.len(), 1);
    let records: Vec<ffi::ZReportSummary> = out_equal(&rows);
    assert_eq!(records[0].sequence_number, 1);
    assert!(!records[0].report_date.is_empty());
}

#[test]
fn a_z_report_summary_with_and_without_a_fiscal_event_converts() {
    for fiscal_event_id in [Some(77_i64), None] {
        let engine = serde_json::json!({
            "id": 3, "reportDate": "2026-10-07", "sequenceNumber": 9,
            "createdAt": "2026-10-08 05:00:01", "fiscalEventId": fiscal_event_id,
        });
        let record: ffi::ZReportSummary = out_equal(&engine);
        assert_eq!(record.fiscal_event_id, fiscal_event_id);
    }
}

// ───────────────────────────── SocialResult and OpenShift ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn social_results_and_open_shifts_of_real_clock_ins_convert() {
    let app = app_on_mock().await;
    assert!(do_list_open_shifts(&app.state).unwrap().is_empty());
    let r = do_work_in(&app.state, INSZ.into()).await.unwrap();
    assert!(r.success, "work in: {:?}", r.error);
    let record: ffi::SocialResult = out_equal(&r);
    assert!(record.success);
    assert_eq!(record.error, None);

    let shifts = do_list_open_shifts(&app.state).unwrap();
    assert_eq!(shifts.len(), 1);
    let records: Vec<ffi::OpenShift> = out_equal(&shifts);
    assert_eq!(records[0].user_insz, INSZ);
    assert!(!records[0].started_at.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_clock_in_converts() {
    let app = app_without_fdm().await;
    let r = do_work_in(&app.state, INSZ.into()).await.unwrap();
    assert!(!r.success);
    let record: ffi::SocialResult = out_equal(&r);
    assert!(!record.success);
    assert!(record.error.is_some());
}

#[test]
fn social_results_with_the_error_set_or_empty_convert() {
    for error in [Some("fdm down".to_string()), None] {
        let engine = EngineSocialResult {
            success: error.is_none(),
            error: error.clone(),
        };
        let record: ffi::SocialResult = out_equal(&engine);
        assert_eq!(record.error, error);
    }
}

// ───────────────────────────── MoneyDirection and FinancialResult ─────────────────────────────

#[test]
fn every_money_direction_reaches_the_engine() {
    for (record, expected) in [
        (ffi::MoneyDirection::In, "In"),
        (ffi::MoneyDirection::Out, "Out"),
        (ffi::MoneyDirection::Declaration, "Declaration"),
    ] {
        let engine: EngineMoneyDirection =
            contract_inp(&record).unwrap_or_else(|e| panic!("convert::inp: {e}"));
        assert_eq!(format!("{engine:?}"), expected);
        // The wire name is the engine's snake_case.
        assert_eq!(
            json_of(&record),
            Value::String(expected.to_lowercase()),
            "serde name of {record:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn financial_results_of_real_events_convert() {
    let app = app_on_mock().await;
    let money = do_money_in_out(
        &app.state,
        INSZ.into(),
        5000,
        EngineMoneyDirection::In,
        "Fond".into(),
    )
    .await
    .unwrap();
    assert!(money.success, "money in: {:?}", money.error);
    let record: ffi::FinancialResult = out_equal(&money);
    assert!(record.success);

    let drawer = do_open_drawer_fiscal(&app.state, INSZ.into())
        .await
        .unwrap();
    assert!(drawer.success, "drawer: {:?}", drawer.error);
    let _: ffi::FinancialResult = out_equal(&drawer);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_financial_event_converts() {
    let app = app_without_fdm().await;
    let r = do_open_drawer_fiscal(&app.state, INSZ.into())
        .await
        .unwrap();
    assert!(!r.success);
    let record: ffi::FinancialResult = out_equal(&r);
    assert!(record.error.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_payment_lines_of_a_real_sale_and_its_correction_convert() {
    use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
    use pos_engine::commands::payment_correction::{do_correct_payment, do_list_sale_payments};
    let app = app_on_mock().await;
    pos_engine::commands::auth::do_set_pin(&app.state, INSZ.into(), "4321".into(), None).unwrap();
    let order = app.order_of(5200);
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap()
        .fiscal_event_id
        .unwrap();

    let before = do_list_sale_payments(&app.state, sale).unwrap();
    assert!(before.correctable);
    let record: ffi::SalePayments = out_equal(&before);
    assert_eq!(record.lines.len(), 1);
    assert_eq!(record.lines[0].payment_type, "CASH");
    assert!(record.corrected.is_none() && record.refusal.is_none());

    let done = do_correct_payment(
        &app.state,
        sale,
        0,
        pos_fiscal::payment::PaymentMethod::Card,
        INSZ.into(),
        "4321".into(),
    )
    .await
    .unwrap();
    assert!(done.success, "{:?}", done.error);
    let _: ffi::FinancialResult = out_equal(&done);

    let after = do_list_sale_payments(&app.state, sale).unwrap();
    let record: ffi::SalePayments = out_equal(&after);
    assert!(!record.correctable);
    assert_eq!(
        record.refusal.as_deref(),
        Some("payment_correction_already_done")
    );
    let corrected = record.corrected.expect("the correction is shown");
    assert_eq!(
        (corrected.from_type.as_str(), corrected.to_type.as_str()),
        ("CASH", "CARD_DEBIT")
    );
}

#[test]
fn sale_payments_with_every_option_set_or_empty_convert() {
    let engine = serde_json::json!({
        "fiscalEventId": 3, "ticketNumber": 12, "totalCents": 4000,
        "lines": [{ "index": 0, "paymentType": "CARD_DEBIT", "name": "Card", "amountCents": 4000,
                    "inputMethod": "AUTOMATIC", "amountType": "PAYMENT", "brand": "Bancontact" }],
        "correctable": false, "refusal": "payment_correction_terminal_card",
        "corrected": { "fromType": "CASH", "toType": "CARD_DEBIT", "amountCents": 100, "at": "2026-10-17 10:00:00" }
    });
    let record: ffi::SalePayments = out_equal(&engine);
    assert_eq!(record.lines[0].brand.as_deref(), Some("Bancontact"));
    let empty = serde_json::json!({
        "fiscalEventId": 3, "ticketNumber": 12, "totalCents": 0,
        "lines": [], "correctable": true, "refusal": null, "corrected": null
    });
    let record: ffi::SalePayments = out_equal(&empty);
    assert!(record.lines.is_empty() && record.correctable);
}

#[test]
fn financial_results_with_the_error_set_or_empty_convert() {
    for error in [Some("fdm down".to_string()), None] {
        let engine = EngineFinancialResult {
            success: error.is_none(),
            error: error.clone(),
        };
        let record: ffi::FinancialResult = out_equal(&engine);
        assert_eq!(record.error, error);
    }
}

// ───────────────────────────── PosSettings ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn settings_of_a_real_engine_convert() {
    let app = app_on_mock().await;
    let settings = do_get_settings(&app.state).unwrap();
    let record: ffi::PosSettings = out_equal(&settings);
    assert_eq!(record.pos_sw_version, settings.pos_sw_version);
    assert_eq!(record.pos_id, settings.pos_id);
    assert!(!record.training_mode);

    app.set_printer("127.0.0.1:9100");
    app.state.config.lock().unwrap().training_mode = true;
    let record: ffi::PosSettings = out_equal(&do_get_settings(&app.state).unwrap());
    assert_eq!(record.printer_address.as_deref(), Some("127.0.0.1:9100"));
    assert!(record.training_mode);
}

#[test]
fn settings_with_every_option_set_or_empty_convert() {
    for full in [true, false] {
        let engine = EnginePosSettings {
            training_mode: full,
            printer_address: full.then(|| "192.168.1.50:9100".to_string()),
            fdm_url: "http://127.0.0.1:8080/graphql".into(),
            pos_id: "CPOS0031234567".into(),
            terminal_id: "POS-2-ANDROID".into(),
            restaurant_name: "Restaurant Test".into(),
            language: "fr".into(),
            config_problems: if full {
                vec!["pos_id".into(), "terminal_id".into()]
            } else {
                vec![]
            },
            pos_sw_version: "pili-pos-android/1.0.0".into(),
            payment_terminal: full.then(|| "worldline_terminal_api".to_string()),
        };
        let record: ffi::PosSettings = out_equal(&engine);
        assert_eq!(record.config_problems.len(), if full { 2 } else { 0 });
        // The camelCase key of the wire form.
        assert_eq!(json_of(&record)["posSwVersion"], "pili-pos-android/1.0.0");
    }
}

// ───────────────────────────── wiring on a plain engine ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn get_settings_carries_the_ffi_software_version() {
    let t = support::open_seeded().await;
    let settings = t.engine.get_settings().await.unwrap();
    assert_eq!(settings.pos_sw_version, "pili-pos-android/1.0.0");
    assert_eq!(settings.pos_sw_version, pos_ffi::pos_sw_version().await);
    assert_eq!(settings.pos_id, "CPOS0031234567");
    assert_eq!(settings.terminal_id, "POS-2-ANDROID");
    assert_eq!(settings.restaurant_name, "Restaurant Test");
    assert_eq!(settings.fdm_url, t.fdm_url);
    assert_eq!(
        settings.printer_address.as_deref(),
        Some(t.printer.as_str())
    );
    assert!(!settings.training_mode);
    t.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn queries_without_an_operator_answer_on_a_fresh_engine() {
    let t = support::open_seeded().await;
    assert!(t.engine.list_z_reports().await.unwrap().is_empty());
    assert!(t.engine.list_open_shifts().await.unwrap().is_empty());
    // No open business day: no reminder.
    assert!(!t.engine.needs_z_report().await.unwrap());
    // Nothing has checked the FDM yet.
    assert_eq!(t.engine.fdm_health().await.unwrap(), None);
    t.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn fdm_check_now_reaches_the_mock_and_is_remembered() {
    let t = support::open_seeded().await;
    let health = t.engine.fdm_check_now().await.unwrap();
    assert!(health.answered, "{health:?}");
    assert!(health.connected, "{health:?}");
    assert_eq!(t.engine.fdm_health().await.unwrap(), Some(health));
    t.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_that_need_an_operator_are_refused_without_a_session() {
    let t = support::open_seeded().await;
    let e = &t.engine;
    let errors = [
        e.generate_x_report(MANAGER_INSZ.into()).await.unwrap_err(),
        e.generate_z_report(MANAGER_INSZ.into()).await.unwrap_err(),
        e.generate_user_x_report(MANAGER_INSZ.into())
            .await
            .unwrap_err(),
        e.backup_database().await.unwrap_err(),
        e.sales_report("2026-10-01".into(), "2026-10-01".into())
            .await
            .unwrap_err(),
        e.print_sales_report("2026-10-01".into(), "2026-10-01".into())
            .await
            .unwrap_err(),
        e.work_in(MANAGER_INSZ.into()).await.unwrap_err(),
        e.work_out(MANAGER_INSZ.into()).await.unwrap_err(),
        e.clock_in_employee(INSZ.into()).await.unwrap_err(),
        e.clock_out_employee(INSZ.into()).await.unwrap_err(),
        e.money_in_out(
            MANAGER_INSZ.into(),
            1000,
            ffi::MoneyDirection::Out,
            "Retrait".into(),
        )
        .await
        .unwrap_err(),
        e.open_drawer_fiscal(MANAGER_INSZ.into()).await.unwrap_err(),
        e.list_sale_payments(1).await.unwrap_err(),
        e.set_training_mode(true).await.unwrap_err(),
    ];
    for err in &errors {
        assert!(coded(err, "not_logged_in"), "{err:?}");
    }
    // The correction answers like the other PIN-confirmed command: a refused result.
    let refused = e
        .correct_payment(
            1,
            0,
            pos_ffi::records::common::PaymentMethod::Card,
            MANAGER_INSZ.into(),
            MANAGER_PIN.into(),
        )
        .await
        .unwrap();
    assert!(!refused.success);
    assert_eq!(refused.error.as_deref(), Some("not_logged_in"));
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn end_shift_checks_the_pin_signs_the_clock_out_and_closes_the_session() {
    let t = support::open_seeded().await;
    let e = &t.engine;

    let err = e
        .end_shift(MANAGER_INSZ.into(), Some("9999".into()))
        .await
        .unwrap_err();
    assert!(coded(&err, "pin_wrong"), "{err:?}");

    let done = e
        .end_shift(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert!(done.success, "{:?}", done.error);
    assert_eq!(done.error, None);

    // The session is closed again.
    let err = e.backup_database().await.unwrap_err();
    assert!(coded(&err, "not_logged_in"), "{err:?}");
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn end_shift_with_the_fdm_down_still_closes_the_session() {
    let down = seeded_on(CLOSED_PORT_FDM).await;
    let result = down
        .engine
        .end_shift(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert!(!result.success);
    assert!(result.error.is_some());
    let err = down.engine.backup_database().await.unwrap_err();
    assert!(coded(&err, "not_logged_in"), "{err:?}");
    down.engine.shutdown().await.unwrap();
}

// ───────────────────────────── wiring with an operator session ─────────────────────────────

/// Holds the first request that signs a clock-out until the test releases it.
#[derive(Clone)]
struct Gate {
    armed: Arc<AtomicBool>,
    arrived: Arc<Notify>,
    release: Arc<Notify>,
}

async fn hold_clock_out(State(gate): State<Gate>, req: Request, next: Next) -> Response {
    let (parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();
    let is_clock_out = bytes.windows(11).any(|w| w == b"signWorkOut");
    if is_clock_out && gate.armed.swap(false, Ordering::SeqCst) {
        gate.arrived.notify_one();
        gate.release.notified().await;
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}

struct Gated {
    rig: TestEngine,
    gate: Gate,
}

/// Like `support::open_seeded`, with the FDM mock behind a gate (`armed`: the first clock-out waits).
async fn open_with_gate(armed: bool) -> Gated {
    let gate = Gate {
        armed: Arc::new(AtomicBool::new(armed)),
        arrived: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    };
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
    let app = pos_fdm_mock::app().layer(axum::middleware::from_fn_with_state(
        gate.clone(),
        hold_clock_out,
    ));
    tokio::spawn(async move {
        axum::serve(fdm, app).await.expect("serve the FDM mock");
    });
    let rig = seeded_on(&fdm_url).await;
    Gated { rig, gate }
}

async fn seeded_on(fdm_url: &str) -> TestEngine {
    let printer_mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
    let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
        .await
        .expect("bind the printer mock")
        .to_string();
    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("pos.json"),
        format!(
            r#"{{
    "pos_id": "CPOS0031234567",
    "terminal_id": "POS-2-ANDROID",
    "device_id": "bundle-device",
    "restaurant_name": "Restaurant Test",
    "restaurant_address": "Rue de l'Exemple 1, 4000 Liege",
    "restaurant_vat": "BE0499999960",
    "restaurant_est_no": "2123456791",
    "fdm_url": "{fdm_url}",
    "printer_address": "{printer}",
    "table_count": 25
}}"#
        ),
    )
    .unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
        .expect("seed the database");
    let engine =
        PosEngine::open(support::options_for(dir.path())).unwrap_or_else(|e| panic!("open: {e}"));
    TestEngine {
        engine,
        dir,
        fdm_url: fdm_url.to_string(),
        printer,
    }
}

/// Start `end_shift` for the manager and wait until it holds the session open at its clock-out.
async fn open_operator_window(
    t: &Gated,
) -> tokio::task::JoinHandle<Result<ffi::SocialResult, PosError>> {
    let engine = Arc::clone(&t.rig.engine);
    let task = tokio::spawn(async move {
        engine
            .end_shift(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
            .await
    });
    tokio::time::timeout(Duration::from_secs(20), t.gate.arrived.notified())
        .await
        .expect("the clock-out reached the FDM");
    task
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_that_do_not_sign_run_for_the_operator() {
    let t = open_with_gate(true).await;
    let e = Arc::clone(&t.rig.engine);
    let shift = open_operator_window(&t).await;

    // Settings: training mode on and off again.
    assert!(e.set_training_mode(true).await.unwrap());
    assert!(e.get_settings().await.unwrap().training_mode);
    assert!(!e.set_training_mode(false).await.unwrap());
    assert!(!e.get_settings().await.unwrap().training_mode);

    // The backup is a file under the data directory's backups.
    let path = e.backup_database().await.unwrap();
    let path = std::path::PathBuf::from(path);
    assert!(path.is_file(), "{path:?}");
    assert!(
        path.starts_with(t.rig.data_dir().join("backups")),
        "{path:?} is not under the data dir's backups"
    );
    assert!(std::fs::metadata(&path).unwrap().len() > 0);

    // The sales report: read-only, range checked by the engine.
    let report = e
        .sales_report("2026-10-01".into(), "2026-10-01".into())
        .await
        .unwrap();
    assert_eq!(report.total_cents, 0);
    let err = e
        .sales_report("2026-10-02".into(), "2026-10-01".into())
        .await
        .unwrap_err();
    assert!(coded(&err, "sales_report_range_invalid"), "{err:?}");
    // The seeded engine has a printer mock: the report prints.
    e.print_sales_report("2026-10-01".into(), "2026-10-01".into())
        .await
        .unwrap();

    // Queries.
    e.list_z_reports().await.unwrap();
    e.list_open_shifts().await.unwrap();
    e.needs_z_report().await.unwrap();
    assert!(e.fdm_check_now().await.unwrap().answered);

    t.gate.release.notify_one();
    let done = shift.await.unwrap().unwrap();
    assert!(done.success, "{:?}", done.error);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[expect(
    clippy::many_single_char_names,
    reason = "short names for the engine handles of one concurrency scenario"
)]
async fn signing_commands_run_for_the_operator() {
    let t = open_with_gate(true).await;
    let e = Arc::clone(&t.rig.engine);
    let shift = open_operator_window(&t).await;

    // Each command passes its operator check now and then waits for the signing lock, which the
    // held clock-out owns. They are started one after the other so the lock hands them over in
    // this order once it is released.
    macro_rules! queue {
        ($e:ident, $call:expr) => {{
            let $e = e.clone();
            let task = tokio::spawn(async move { $call.await });
            tokio::time::sleep(Duration::from_millis(150)).await;
            task
        }};
    }
    let work_in = queue!(e, e.work_in(MANAGER_INSZ.into()));
    let money = queue!(
        e,
        e.money_in_out(
            MANAGER_INSZ.into(),
            5000,
            ffi::MoneyDirection::In,
            "Fond".into()
        )
    );
    let drawer = queue!(e, e.open_drawer_fiscal(MANAGER_INSZ.into()));
    let x = queue!(e, e.generate_x_report(MANAGER_INSZ.into()));
    let user_x = queue!(e, e.generate_user_x_report(MANAGER_INSZ.into()));
    let z = queue!(e, e.generate_z_report(MANAGER_INSZ.into()));

    t.gate.release.notify_one();
    let done = shift.await.unwrap().unwrap();
    assert!(done.success, "end_shift: {:?}", done.error);

    let r = work_in.await.unwrap().unwrap();
    assert!(r.success, "work_in: {:?}", r.error);
    let r = money.await.unwrap().unwrap();
    assert!(r.success, "money_in_out: {:?}", r.error);
    let r = drawer.await.unwrap().unwrap();
    assert!(r.success, "open_drawer_fiscal: {:?}", r.error);
    let r = x.await.unwrap().unwrap();
    assert!(r.success, "x: {:?}", r.error);
    assert_eq!((r.report_type.as_str(), r.report_no), ("X", None));
    let r = user_x.await.unwrap().unwrap();
    assert!(r.success, "user x: {:?}", r.error);
    assert_eq!(r.report_type, "X-USER");
    let r = z.await.unwrap().unwrap();
    assert!(r.success, "z: {:?}", r.error);
    assert_eq!((r.report_type.as_str(), r.report_no), ("Z", Some(1)));

    // The Z is in the history, and the clock-in/clock-out left the shift closed.
    let history = t.rig.engine.list_z_reports().await.unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].sequence_number, 1);
    t.rig.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manager_registers_the_presence_of_a_presence_only_employee() {
    let t = open_with_gate(false).await;
    let e = &t.rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    let kitchen = e
        .create_user(pos_ffi::records::auth::CreateUserInput {
            insz: INSZ.into(),
            name: "Cuisine".into(),
            role: "waiter".into(),
            initial_pin: None,
            operator_pin: None,
            presence_only: true,
        })
        .await
        .unwrap();
    assert!(kitchen.presence_only && !kitchen.has_pin);

    // Refusals are coded, nothing is signed.
    let err = e.clock_out_employee(INSZ.into()).await.unwrap_err();
    assert!(coded(&err, "presence_not_in"), "{err:?}");
    let err = e.clock_in_employee(MANAGER_INSZ.into()).await.unwrap_err();
    assert!(coded(&err, "presence_self"), "{err:?}");
    assert!(
        !e.list_open_shifts()
            .await
            .unwrap()
            .iter()
            .any(|s| s.user_insz == INSZ)
    );

    let r = e.clock_in_employee(INSZ.into()).await.unwrap();
    assert!(r.success, "{:?}", r.error);
    let open = e.list_open_shifts().await.unwrap();
    assert!(open.iter().any(|s| s.user_insz == INSZ), "{open:?}");
    let err = e.clock_in_employee(INSZ.into()).await.unwrap_err();
    assert!(coded(&err, "presence_already_in"), "{err:?}");
    // On shift: not deactivable.
    let err = e.deactivate_user(INSZ.into()).await.unwrap_err();
    assert!(coded(&err, "deactivate_on_shift"), "{err:?}");

    let r = e.clock_out_employee(INSZ.into()).await.unwrap();
    assert!(r.success, "{:?}", r.error);
    assert!(
        !e.list_open_shifts()
            .await
            .unwrap()
            .iter()
            .any(|s| s.user_insz == INSZ)
    );

    // They never log in.
    e.logout().await.unwrap();
    let err = e.login(INSZ.into(), None).await.unwrap_err();
    assert!(coded(&err, "presence_only_user"), "{err:?}");
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn work_out_is_refused_without_a_session_and_signs_with_one() {
    let t = open_with_gate(true).await;
    let e = Arc::clone(&t.rig.engine);
    let err = e.work_out(MANAGER_INSZ.into()).await.unwrap_err();
    assert!(coded(&err, "not_logged_in"), "{err:?}");

    let shift = open_operator_window(&t).await;
    let other = Arc::clone(&e);
    let work_out = tokio::spawn(async move { other.work_out(MANAGER_INSZ.into()).await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    // Someone else's INSZ is refused while the session is open.
    let err = e.work_out("75061189702".into()).await.unwrap_err();
    assert!(coded(&err, "not_the_operator"), "{err:?}");

    t.gate.release.notify_one();
    shift.await.unwrap().unwrap();
    let r = work_out.await.unwrap().unwrap();
    assert!(r.success, "work_out: {:?}", r.error);
    e.shutdown().await.unwrap();
}
