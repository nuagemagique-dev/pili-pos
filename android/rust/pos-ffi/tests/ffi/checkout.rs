//! Tests of the checkout facade (engine modules checkout, unpaid, refund, copy, delivery, print):
//!
//! - contract tests: every output type of the domain is built from a real engine value (a command
//!   run on a `TestApp` against the admission FDM mock) and converted with `convert::out`; the
//!   engine's serde form and the record's must be equal (for `FdmStatusReport`, which is typed
//!   from a JSON with absent keys, equal once the record's `null` top-level keys are dropped, so
//!   no key of the engine JSON is lost and none is invented). Hand-built values cover the
//!   `Option`s a real run leaves empty or set. Input types are checked field by field.
//! - wiring tests: every one of the 15 methods called on an engine opened by `support::open_seeded`
//!   (no operator is signed in, the FFI has no login yet in this card: a method that needs one
//!   must answer its coded refusal, never `Internal`).
//! - the money helpers `settle_preview` and `cash_due_cents` with the engine's own vectors
//!   (`crates/pos-fiscal/src/payment.rs` tests).

use std::io::{Read, Write};

use pos_core::order::OrderType;
use pos_db::queries::fiscal::{
    CopyableTicket as EngineCopyableTicket, RefundableEvent as EngineRefundableEvent,
};
use pos_engine::commands::checkout::{
    CheckoutResult as EngineCheckoutResult, do_check_fdm_status, do_checkout_split,
};
use pos_engine::commands::copy::{
    CopyResult as EngineCopyResult, do_copy_ticket, do_list_copyable_tickets, do_list_fiscal_events,
};
use pos_engine::commands::delivery::{
    DeliveryView as EngineDeliveryView, MailStatus as EngineMailStatus, RequestedMedium,
    TicketDeliveryRequest as EngineDeliveryRequest, do_check_mail_status,
    do_list_ticket_deliveries,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{RefundItem as EngineRefundItem, do_list_refundable_events};
use pos_engine_testkit::{TestApp, tem_accepts, tem_probe};
use pos_ffi::PosError;
use pos_ffi::records::checkout as rec;
use pos_ffi::records::common::{
    self as common, PaymentMethod, PaymentPart, TicketDeliveryRequest, contract_inp, contract_out,
};
use pos_fiscal::payment::PaymentPart as EnginePaymentPart;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json, to_value};

use crate::support::{self, MANAGER_INSZ};

// ───────────────────────────── helpers ─────────────────────────────

const INSZ: &str = "12345678958";

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

/// A hand-built engine-shaped JSON value converts to the record and back to the same JSON.
fn json_round_trip<F: Serialize + DeserializeOwned>(value: Value) -> F {
    let record: F = contract_out(&value).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(value, json_of(&record), "JSON vs FFI serde form");
    record
}

/// `value` without its `null` top-level keys.
fn without_null_keys(value: &Value) -> Value {
    let mut value = value.clone();
    if let Value::Object(map) = &mut value {
        map.retain(|_, v| !v.is_null());
    }
    value
}

/// The FDM status JSON of the engine converts to a report that holds exactly its non-null keys.
fn status_report(engine_json: &Value) -> rec::FdmStatusReport {
    let record: rec::FdmStatusReport =
        contract_out(engine_json).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        without_null_keys(engine_json),
        without_null_keys(&json_of(&record)),
        "engine JSON vs report"
    );
    record
}

fn err_of<T: std::fmt::Debug>(r: Result<T, PosError>) -> PosError {
    r.expect_err("an error")
}

fn assert_coded<T: std::fmt::Debug>(r: Result<T, PosError>, expected: &str) {
    match err_of(r) {
        PosError::Coded { code, params } => {
            assert_eq!(code, expected, "params: {params:?}");
        }
        other => panic!("expected the code {expected}, got {other:?}"),
    }
}

// ───────────────────────────── real engine values ─────────────────────────────

/// A `TestApp` that signs on the admission FDM mock, with two menu items and the operator logged
/// in as admin. Optionally a printer.
async fn app_on_mock(printer: bool) -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 1200, 'A', 0, 1),
                (2, 2, 2800, 'B', 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('category', 1, 'name', 'fr', 'Boissons'),
                ('category', 2, 'name', 'fr', 'Cuisine'),
                ('menu_item', 1, 'name', 'fr', 'Dry Martini'),
                ('menu_item', 2, 'name', 'fr', 'Burger du chef');",
        )
        .unwrap();
    }
    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    if printer {
        let mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
        let addr = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), mock)
            .await
            .expect("bind the printer mock")
            .to_string();
        app.set_printer(&addr);
    }
    app
}

/// Serve an FDM mock with the given states and point the app at it.
async fn use_fdm_with(app: &TestApp, control: pos_fdm_mock::Control) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, pos_fdm_mock::app_with(control))
            .await
            .unwrap();
    });
    app.state.config.lock().unwrap().fdm_url = format!("http://{addr}/graphql");
}

fn order(app: &TestApp, lines: &[(i64, i32)]) -> i64 {
    let id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
    .id;
    for &(item, qty) in lines {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: id,
                menu_item_id: item,
                quantity: qty,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    id
}

async fn cash_sale(
    app: &TestApp,
    delivery: Option<&EngineDeliveryRequest>,
) -> EngineCheckoutResult {
    let id = order(app, &[(1, 2), (2, 1)]);
    let result = do_checkout_split(&app.state, id, &[EnginePaymentPart::cash()], delivery)
        .await
        .unwrap();
    assert!(result.success, "sale: {:?}", result.error);
    result
}

// ───────────────────────────── contract: FDM status ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn an_ok_fdm_status_converts_without_losing_a_key() {
    let app = app_on_mock(false).await;
    let status = do_check_fdm_status(&app.state).await.unwrap();
    // The FDM answered: its own shape, with `connected` and `error` absent.
    assert!(status.get("connected").is_none() && status.get("error").is_none());
    let report = status_report(&status);
    assert_eq!(report.connected, None);
    assert_eq!(report.error, None);
    assert_eq!(report.initialized, Some(true));
    let device = report.device.expect("the FDM names itself");
    assert!(!device.fdm_id.is_empty());
    assert!(!device.fdm_sw_version.is_empty());
    assert!(!device.fdm_date_time.is_empty());
    assert!((0.0..=100.0).contains(&device.buffer_capacity_used));
    assert_eq!(report.informations, Some(vec![]));
    assert_eq!(report.warnings, Some(vec![]));
    assert_eq!(report.errors, Some(vec![]));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_fdm_status_with_errors_and_warnings_converts_with_its_messages() {
    let app = app_on_mock(false).await;
    use_fdm_with(
        &app,
        pos_fdm_mock::Control {
            errors: vec!["BUFFER_FULL".into()],
            warnings: vec!["CLIENT_CERT_NEAR_EXPIRATION".into()],
            ..Default::default()
        },
    )
    .await;
    let status = do_check_fdm_status(&app.state).await.unwrap();
    let report = status_report(&status);
    assert_eq!(report.initialized, Some(false));

    let errors = report.errors.expect("the error list");
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(!errors[0].message.is_empty());
    assert_eq!(
        errors[0].extensions.code,
        common::FdmWarningCode::BufferFull
    );
    // The category is the origin of the message (SPF_FOD where the table names two).
    assert_eq!(
        errors[0].extensions.category,
        common::MessageCategory::SpfFod
    );

    // A full buffer is also nearly full (module2 lists both).
    let warnings = report.warnings.expect("the warning list");
    let codes: Vec<_> = warnings.iter().map(|m| m.extensions.code).collect();
    assert!(
        codes.contains(&common::FdmWarningCode::ClientCertNearExpiration),
        "{codes:?}"
    );
    assert!(
        codes.contains(&common::FdmWarningCode::BufferNearFull),
        "{codes:?}"
    );
    assert_eq!(report.device.expect("device").buffer_capacity_used, 100.0);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_fdm_converts_to_connected_false_and_the_error() {
    let app = app_on_mock(false).await;
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".to_string();
    let status = do_check_fdm_status(&app.state).await.unwrap();
    assert_eq!(status["connected"], false);
    let report = status_report(&status);
    assert_eq!(report.connected, Some(false));
    assert!(report.error.as_deref().is_some_and(|e| !e.is_empty()));
    assert_eq!(report.device, None);
    assert_eq!(report.initialized, None);
    assert_eq!(report.informations, None);
    assert_eq!(report.warnings, None);
    assert_eq!(report.errors, None);
}

fn message(code: &str, with_optionals: bool) -> Value {
    json!({
        "message": format!("message {code}"),
        "locations": if with_optionals { json!([{"line": 3, "column": 14}]) } else { Value::Null },
        "extensions": {
            "category": "SPF_FOD",
            "code": code,
            "data": if with_optionals {
                json!([{"name": "daysLeft", "value": "12"}])
            } else {
                Value::Null
            },
            "showPos": "MANDATORY"
        }
    })
}

#[test]
fn a_status_with_every_field_set_round_trips() {
    let full = json!({
        "device": {
            "fdmId": "FDM02000425",
            "fdmSwVersion": "2.2.0.5",
            "fdmDateTime": "2026-09-27T14:40:00.000Z",
            "bufferCapacityUsed": 82.53
        },
        "initialized": true,
        "informations": [message("UPDATE_VAT_RATES_FAILED", false)],
        "warnings": [message("CLIENT_CERT_NEAR_EXPIRATION", true), message("BUFFER_NEAR_FULL", false)],
        "errors": [message("FDM_LOCKED", true)]
    });
    let report = status_report(&full);
    assert_eq!(
        without_null_keys(&json_of(&report)),
        full,
        "nothing is lost"
    );
    let warning = &report.warnings.unwrap()[0];
    assert_eq!(
        warning.locations,
        Some(vec![common::LocationItem {
            line: 3,
            column: 14
        }])
    );
    assert_eq!(
        warning.extensions.data,
        Some(vec![common::DataItem {
            name: "daysLeft".into(),
            value: "12".into()
        }])
    );
    assert_eq!(
        warning.extensions.show_pos,
        common::FdmMessageDisplay::Mandatory
    );

    // `null` lists (the FDM sent nothing) stay `None`.
    let nulls = json!({
        "device": full["device"],
        "initialized": false,
        "informations": null,
        "warnings": null,
        "errors": null
    });
    let report = status_report(&nulls);
    assert_eq!(report.informations, None);
    assert_eq!(report.errors, None);

    // The unreachable shape.
    let down = json!({"connected": false, "error": "connection refused"});
    let report = status_report(&down);
    assert_eq!(report.connected, Some(false));
    assert_eq!(report.error.as_deref(), Some("connection refused"));
}

#[test]
fn a_status_with_an_unknown_key_is_a_contract_error_not_a_silent_loss() {
    let err = contract_out::<_, rec::FdmStatusReport>(&json!({
        "initialized": true,
        "newField": 1
    }))
    .expect_err("deny_unknown_fields");
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");
    let err = contract_out::<_, rec::FdmDevice>(&json!({
        "fdmId": "x", "fdmSwVersion": "1", "fdmDateTime": "t", "bufferCapacityUsed": 1.0,
        "extra": true
    }))
    .expect_err("deny_unknown_fields");
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");
}

// ───────────────────────────── contract: fiscal events, refund, copy ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn fiscal_events_of_a_real_run_convert() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, None).await;
    let events = do_list_fiscal_events(&app.state, None).unwrap();
    // The JSON is an array of objects with exactly the eight keys, nulls included.
    let rows = events.as_array().expect("an array");
    assert!(!rows.is_empty());
    for row in rows {
        let mut keys: Vec<_> = row.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "eventLabel",
                "eventType",
                "fdmDatetime",
                "fdmId",
                "id",
                "orderId",
                "sequenceNumber",
                "shortSignature"
            ]
        );
    }
    let records: Vec<rec::FiscalEventSummary> = out_equal(&events);
    assert_eq!(records.len(), rows.len());
    let record = records
        .iter()
        .find(|r| Some(r.id) == sale.fiscal_event_id)
        .expect("the sale is listed");
    assert_eq!(record.event_type, "N");
    assert_eq!(record.event_label.as_deref(), Some("N"));
    assert!(record.fdm_id.is_some());
    assert!(record.fdm_datetime.is_some());
    assert!(record.short_signature.is_some());
    assert!(record.order_id.is_some());
    assert!(record.sequence_number >= 1);

    // An empty archive is an empty list.
    let empty: Vec<rec::FiscalEventSummary> = out_equal(&json!([]));
    assert!(empty.is_empty());
}

#[test]
fn fiscal_event_summaries_with_every_option_set_or_empty_round_trip() {
    let full: rec::FiscalEventSummary = json_round_trip(json!({
        "id": 9, "eventType": "C", "sequenceNumber": 4, "orderId": 7, "fdmId": "FDM1",
        "fdmDatetime": "2026-10-03T10:00:00.000Z", "eventLabel": "N", "shortSignature": "ABCDEF"
    }));
    assert_eq!(
        full.fdm_datetime.as_deref(),
        Some("2026-10-03T10:00:00.000Z")
    );
    let empty: rec::FiscalEventSummary = json_round_trip(json!({
        "id": 1, "eventType": "R", "sequenceNumber": 1, "orderId": null, "fdmId": null,
        "fdmDatetime": null, "eventLabel": null, "shortSignature": null
    }));
    assert_eq!(empty.order_id, None);
    assert!(
        contract_out::<_, rec::FiscalEventSummary>(&json!({
            "id": 1, "eventType": "R", "sequenceNumber": 1, "orderId": null, "fdmId": null,
            "fdmDatetime": null, "eventLabel": null, "shortSignature": null, "qrData": "x"
        }))
        .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn refundable_and_copyable_tickets_of_a_real_sale_convert() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, None).await;

    let refundable: Vec<EngineRefundableEvent> =
        do_list_refundable_events(&app.state, None).unwrap();
    assert_eq!(refundable.len(), 1);
    let records: Vec<rec::RefundableEvent> = out_equal(&refundable);
    assert_eq!(Some(records[0].fiscal_event_id), sale.fiscal_event_id);
    assert_eq!(records[0].total_cents, 5200);
    assert!(records[0].ticket_number >= 1);
    assert_eq!(records[0].table_number, None);

    let copyable: Vec<EngineCopyableTicket> = do_list_copyable_tickets(&app.state, None).unwrap();
    assert!(!copyable.is_empty());
    let records: Vec<rec::CopyableTicket> = out_equal(&copyable);
    assert_eq!(Some(records[0].fiscal_event_id), sale.fiscal_event_id);
    assert_eq!(records[0].event_label, "N");
    assert_eq!(records[0].total_cents, 5200);
}

#[test]
fn refundable_and_copyable_tickets_with_a_table_round_trip() {
    let refundable: rec::RefundableEvent = json_round_trip(json!({
        "fiscalEventId": 3, "ticketNumber": 12, "orderId": 8, "tableNumber": 5,
        "totalCents": 4200, "createdAt": "2026-10-03 10:00:00"
    }));
    assert_eq!(refundable.table_number, Some(5));
    let copyable: rec::CopyableTicket = json_round_trip(json!({
        "fiscalEventId": 3, "eventLabel": "P", "ticketNumber": 12, "tableNumber": 5,
        "totalCents": -4200, "createdAt": "2026-10-03 10:00:00"
    }));
    assert_eq!(copyable.total_cents, -4200);
    assert!(
        contract_out::<_, rec::RefundableEvent>(&json!({
            "fiscalEventId": 3, "ticketNumber": 12, "orderId": 8, "tableNumber": null,
            "totalCents": 4200, "createdAt": "x", "status": "closed"
        }))
        .is_err()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_result_of_a_real_copy_converts() {
    let app = app_on_mock(true).await;
    let sale = cash_sale(&app, None).await;
    let copy = do_copy_ticket(&app.state, sale.fiscal_event_id.unwrap(), INSZ.to_string())
        .await
        .unwrap();
    assert!(copy.success, "copy: {:?}", copy.error);
    let record: rec::CopyResult = out_equal(&copy);
    assert!(record.success);
    assert_eq!(record.error, None);
    assert!(record.fiscal_event_id.is_some());
    let ticket = record.ticket.expect("a copy of a sale has a ticket");
    assert!(ticket.copy_made_at.is_some());
    assert!(ticket.original.is_some());
    let delivery = record.delivery.expect("the copy is delivered");
    assert_eq!(delivery.medium, common::TicketMedium::Paper);
    assert!(delivery.paper_printed);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refused_copy_converts() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, None).await;
    let copy: EngineCopyResult = do_copy_ticket(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        "00000000000".to_string(),
    )
    .await
    .unwrap();
    assert!(!copy.success);
    let record: rec::CopyResult = out_equal(&copy);
    assert!(!record.success);
    assert!(record.error.is_some());
    assert_eq!(record.ticket, None);
    assert_eq!(record.fiscal_event_id, None);
    assert_eq!(record.delivery, None);
}

// ───────────────────────────── contract: delivery ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn the_mail_status_converts_for_every_state() {
    // No email set up.
    let app = app_on_mock(false).await;
    let status: EngineMailStatus = do_check_mail_status(&app.state).await.unwrap();
    let record: rec::MailStatus = out_equal(&status);
    assert_eq!(
        record,
        rec::MailStatus {
            configured: false,
            reachable: false,
            allow_digital_only: false
        }
    );

    // Set up and reachable, digital-only allowed. The TEM server is the (unused) FDM server of a
    // second app, as the testkit takes a `MockServer`.
    let tem = TestApp::new().await.fdm;
    tem_probe(&tem, 200).await;
    let mut app = app_on_mock(false).await;
    app.enable_mail(&tem, true);
    let status = do_check_mail_status(&app.state).await.unwrap();
    let record: rec::MailStatus = out_equal(&status);
    assert_eq!(
        record,
        rec::MailStatus {
            configured: true,
            reachable: true,
            allow_digital_only: true
        }
    );

    // Set up, service down.
    let down = TestApp::new().await.fdm;
    tem_probe(&down, 503).await;
    let mut app = app_on_mock(false).await;
    app.enable_mail(&down, false);
    let status = do_check_mail_status(&app.state).await.unwrap();
    let record: rec::MailStatus = out_equal(&status);
    assert_eq!(
        record,
        rec::MailStatus {
            configured: true,
            reachable: false,
            allow_digital_only: false
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn deliveries_of_real_digital_tickets_convert() {
    let tem = TestApp::new().await.fdm;
    tem_probe(&tem, 200).await;
    tem_accepts(&tem).await;
    let mut app = app_on_mock(false).await;
    app.enable_mail(&tem, true);
    let request = EngineDeliveryRequest::email(RequestedMedium::Digital, "client@example.be");
    let sale = cash_sale(&app, Some(&request)).await;

    let views: Vec<EngineDeliveryView> = do_list_ticket_deliveries(&app.state, None).unwrap();
    assert_eq!(views.len(), 1);
    let records: Vec<rec::DeliveryView> = out_equal(&views);
    assert_eq!(Some(records[0].fiscal_event_id), sale.fiscal_event_id);
    assert!(
        matches!(records[0].medium.as_str(), "DIGITAL" | "PAPER_DIGITAL"),
        "{}",
        records[0].medium
    );
    assert!(!records[0].status.is_empty());
    assert!(!records[0].recipient_hint.is_empty());
    assert!(
        !records[0].recipient_hint.contains("client@example.be"),
        "never the address itself"
    );
    assert!(!records[0].created_at.is_empty());
}

#[test]
fn delivery_views_with_and_without_an_error_round_trip() {
    let failed: rec::DeliveryView = json_round_trip(json!({
        "id": 2, "fiscalEventId": 5, "medium": "DIGITAL", "recipientHint": "c***@example.be",
        "status": "queued", "attempts": 3, "lastError": "TEM answered 503",
        "fallbackPrinted": true, "createdAt": "2026-10-03 10:00:00"
    }));
    assert_eq!(failed.last_error.as_deref(), Some("TEM answered 503"));
    let sent: rec::DeliveryView = json_round_trip(json!({
        "id": 1, "fiscalEventId": 4, "medium": "PAPER_DIGITAL", "recipientHint": "",
        "status": "sent", "attempts": 1, "lastError": null,
        "fallbackPrinted": false, "createdAt": "2026-10-03 09:00:00"
    }));
    assert_eq!(sent.last_error, None);
    let mail: rec::MailStatus = json_round_trip(json!({
        "configured": true, "reachable": false, "allowDigitalOnly": true
    }));
    assert!(mail.allow_digital_only);
}

// ───────────────────────────── contract: inputs ─────────────────────────────

#[test]
fn a_refund_item_converts_field_by_field() {
    let full = rec::RefundItem {
        menu_item_id: 7,
        quantity: 3,
        line_id: Some(41),
    };
    let engine: EngineRefundItem = contract_inp(&full).unwrap();
    assert_eq!(engine.menu_item_id, 7);
    assert_eq!(engine.quantity, 3);
    assert_eq!(engine.line_id, Some(41));

    let bare = rec::RefundItem {
        menu_item_id: 8,
        quantity: 1,
        line_id: None,
    };
    let engine: EngineRefundItem = contract_inp(&bare).unwrap();
    assert_eq!(engine.menu_item_id, 8);
    assert_eq!(engine.quantity, 1);
    assert_eq!(engine.line_id, None);

    // The serde form is the engine's: camelCase keys, `lineId` optional.
    assert_eq!(
        json_of(&full),
        json!({"menuItemId": 7, "quantity": 3, "lineId": 41})
    );
    let parsed: rec::RefundItem =
        serde_json::from_value(json!({"menuItemId": 8, "quantity": 1})).unwrap();
    assert_eq!(parsed, bare);
    assert!(
        serde_json::from_value::<rec::RefundItem>(
            json!({"menuItemId": 8, "quantity": 1, "price": 1})
        )
        .is_err(),
        "a price from the screen is refused, never ignored"
    );
}

#[test]
fn payment_parts_and_the_delivery_request_convert_field_by_field() {
    let part = PaymentPart {
        method: PaymentMethod::MealVoucher,
        amount_cents: Some(1152),
    };
    let engine: EnginePaymentPart = contract_inp(&part).unwrap();
    assert_eq!(
        engine,
        EnginePaymentPart::of(pos_fiscal::payment::PaymentMethod::MealVoucher, 1152)
    );
    let cash = PaymentPart {
        method: PaymentMethod::Cash,
        amount_cents: None,
    };
    assert_eq!(
        contract_inp::<_, EnginePaymentPart>(&cash).unwrap(),
        EnginePaymentPart::cash()
    );

    let request = TicketDeliveryRequest {
        medium: common::RequestedMedium::PaperDigital,
        email: Some("a@b.be".into()),
    };
    let engine: EngineDeliveryRequest = contract_inp(&request).unwrap();
    assert_eq!(engine.medium, RequestedMedium::PaperDigital);
    assert_eq!(engine.email.as_deref(), Some("a@b.be"));
}

// ───────────────────────────── contract: settlement preview ─────────────────────────────

#[test]
fn a_settlement_preview_serializes_with_its_fields() {
    let preview = rec::SettlementPreview {
        payments: vec![
            rec::SettledPaymentRow {
                method: PaymentMethod::Card,
                amount_cents: 400,
            },
            rec::SettledPaymentRow {
                method: PaymentMethod::Cash,
                amount_cents: 752,
            },
        ],
        rounding_cents: -2,
        cash_due_cents: 750,
        has_cash: true,
    };
    let value = json_of(&preview);
    assert_eq!(
        value,
        json!({
            "payments": [
                {"method": "card", "amount_cents": 400},
                {"method": "cash", "amount_cents": 752}
            ],
            "rounding_cents": -2,
            "cash_due_cents": 750,
            "has_cash": true
        })
    );
    let back: rec::SettlementPreview = serde_json::from_value(value).unwrap();
    assert_eq!(back, preview);
    assert!(
        serde_json::from_value::<rec::SettlementPreview>(json!({
            "payments": [], "rounding_cents": 0, "cash_due_cents": 0, "has_cash": false,
            "total": 1
        }))
        .is_err()
    );
}

// ───────────────────────────── wiring ─────────────────────────────

/// Engine without a signed-in operator: a method answers a value or a coded refusal. `Internal`
/// is a broken contract, `EngineClosed` a lifecycle bug.
fn assert_ok_or_coded<T: std::fmt::Debug>(name: &str, r: Result<T, PosError>) -> Option<T> {
    match r {
        Ok(v) => Some(v),
        Err(PosError::Coded { .. }) | Err(PosError::Fdm { .. }) => None,
        Err(other) => panic!("{name}: {other:?}"),
    }
}

fn cash_only() -> Vec<PaymentPart> {
    vec![PaymentPart {
        method: PaymentMethod::Cash,
        amount_cents: None,
    }]
}

#[tokio::test(flavor = "multi_thread")]
async fn every_command_of_the_domain_is_wired() {
    let te = support::open_seeded().await;
    let e = &te.engine;

    // The queries that need no operator.
    assert_eq!(e.list_fiscal_events(None).await.unwrap(), vec![]);
    assert_eq!(e.list_fiscal_events(Some(5)).await.unwrap(), vec![]);
    assert_eq!(e.list_refundable_events(None).await.unwrap(), vec![]);
    assert_eq!(e.list_ticket_deliveries(Some(10)).await.unwrap(), vec![]);
    assert_eq!(
        e.check_mail_status().await.unwrap(),
        rec::MailStatus {
            configured: false,
            reachable: false,
            allow_digital_only: false
        }
    );
    assert!(e.check_printer().await.unwrap(), "the printer mock answers");
    assert_eq!(e.check_kitchen_printer().await.unwrap(), None);

    // No operator is signed in: the commands that sign or list for a manager refuse.
    let sale = e.checkout(1, cash_only(), None).await;
    if let Some(result) = assert_ok_or_coded("checkout", sale) {
        assert!(!result.success);
        assert!(result.error.is_some());
        assert_eq!(result.ticket, None);
    }
    let unpaid = e.close_unpaid(1, MANAGER_INSZ.into(), "0000".into()).await;
    if let Some(result) = assert_ok_or_coded("close_unpaid", unpaid) {
        assert!(!result.success, "a wrong PIN closes nothing");
    }
    let refund = e
        .refund_checkout(
            1,
            vec![rec::RefundItem {
                menu_item_id: 1,
                quantity: 1,
                line_id: None,
            }],
            MANAGER_INSZ.into(),
            Some(TicketDeliveryRequest {
                medium: common::RequestedMedium::Paper,
                email: None,
            }),
        )
        .await;
    if let Some(result) = assert_ok_or_coded("refund_checkout", refund) {
        assert!(!result.success);
    }
    let copy = e.copy_ticket(1, "00000000000".into(), None).await;
    if let Some(result) = assert_ok_or_coded("copy_ticket", copy) {
        assert!(!result.success);
    }
    assert_ok_or_coded("list_copyable_tickets", e.list_copyable_tickets(None).await);
    assert_ok_or_coded("print_missing_original", e.print_missing_original(1).await);
    assert_ok_or_coded("print_webshop_kitchen", e.print_webshop_kitchen(1).await);
    assert_ok_or_coded("print_webshop_customer", e.print_webshop_customer(1).await);

    // Manager commands keep refusing without a session, with a code and not `Internal`.
    assert!(matches!(
        err_of(e.list_copyable_tickets(Some(3)).await),
        PosError::Coded { .. }
    ));
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn check_fdm_status_answers_a_typed_report_from_the_mock() {
    let te = support::open_seeded().await;
    let report = te.engine.check_fdm_status().await.unwrap();
    assert_eq!(report.connected, None);
    assert_eq!(report.error, None);
    assert_eq!(report.initialized, Some(true));
    let device = report.device.expect("the mock names itself");
    assert!(!device.fdm_id.is_empty());
    assert_eq!(report.errors, Some(vec![]));
    assert_eq!(report.warnings, Some(vec![]));
    assert_eq!(report.informations, Some(vec![]));
    te.engine.shutdown().await.unwrap();
}

/// `POST /mock/control` of the FDM mock, over a plain socket.
fn set_mock_control(fdm_url: &str, body: &str) {
    let addr = fdm_url
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap();
    let mut stream = std::net::TcpStream::connect(addr).expect("connect to the FDM mock");
    let request = format!(
        "POST /mock/control HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
}

#[tokio::test(flavor = "multi_thread")]
async fn check_fdm_status_keeps_the_messages_of_an_fdm_in_error() {
    let te = support::open_seeded().await;
    set_mock_control(
        &te.fdm_url,
        r#"{"errors":["FDM_LOCKED"],"warnings":["RTC_SYNC_FAILED"]}"#,
    );
    let report = te.engine.check_fdm_status().await.unwrap();
    assert_eq!(report.initialized, Some(false));
    let errors = report.errors.expect("errors");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].extensions.code, common::FdmWarningCode::FdmLocked);
    assert!(!errors[0].message.is_empty());
    let warnings = report.warnings.expect("warnings");
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].extensions.code,
        common::FdmWarningCode::RtcSyncFailed
    );
    te.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn check_fdm_status_reports_a_dead_fdm_without_an_error() {
    let te = support::open_seeded().await;
    // Nothing listens on the port any more once the mock's listener is gone: use a fresh engine
    // whose bundle points at a closed port.
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    let pos_json = std::fs::read_to_string(te.bundle_json())
        .unwrap()
        .replace(&te.fdm_url, "http://127.0.0.1:1/graphql");
    std::fs::write(bundle.join("pos.json"), pos_json).unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed)).unwrap();
    let engine = pos_ffi::PosEngine::open(support::options_for(dir.path())).unwrap();

    let report = engine.check_fdm_status().await.unwrap();
    assert_eq!(report.connected, Some(false));
    assert!(report.error.is_some_and(|e| !e.is_empty()));
    assert_eq!(report.device, None);
    engine.shutdown().await.unwrap();
    te.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_methods_refuse_after_shutdown() {
    let te = support::open_seeded().await;
    te.engine.shutdown().await.unwrap();
    assert!(matches!(
        err_of(te.engine.check_printer().await),
        PosError::EngineClosed
    ));
    assert!(matches!(
        err_of(te.engine.list_fiscal_events(None).await),
        PosError::EngineClosed
    ));
    assert!(matches!(
        err_of(te.engine.checkout(1, cash_only(), None).await),
        PosError::EngineClosed
    ));
}

// ───────────────────────────── money helpers ─────────────────────────────

fn card(amount: i64) -> PaymentPart {
    PaymentPart {
        method: PaymentMethod::Card,
        amount_cents: Some(amount),
    }
}

fn voucher(amount: i64) -> PaymentPart {
    PaymentPart {
        method: PaymentMethod::MealVoucher,
        amount_cents: Some(amount),
    }
}

fn cash() -> PaymentPart {
    PaymentPart {
        method: PaymentMethod::Cash,
        amount_cents: None,
    }
}

fn row(method: PaymentMethod, amount_cents: i64) -> rec::SettledPaymentRow {
    rec::SettledPaymentRow {
        method,
        amount_cents,
    }
}

#[tokio::test]
async fn cash_alone_is_rounded() {
    let p = pos_ffi::records::checkout::SettlementPreview {
        payments: vec![row(PaymentMethod::Cash, 1152)],
        rounding_cents: -2,
        cash_due_cents: 1150,
        has_cash: true,
    };
    assert_eq!(
        pos_ffi::api::checkout::settle_preview(1152, vec![cash()])
            .await
            .unwrap(),
        p
    );
    assert_eq!(
        pos_ffi::api::checkout::cash_due_cents(1152, vec![cash()])
            .await
            .unwrap(),
        1150
    );
}

#[tokio::test]
async fn a_card_is_never_rounded() {
    let p = pos_ffi::api::checkout::settle_preview(1152, vec![card(1152)])
        .await
        .unwrap();
    assert_eq!(p.payments, vec![row(PaymentMethod::Card, 1152)]);
    assert_eq!(p.rounding_cents, 0);
    assert_eq!(p.cash_due_cents, 0);
    assert!(!p.has_cash);
}

#[tokio::test]
async fn round4_meal_vouchers_then_cash_rounds_the_cash_rest_only() {
    let p = pos_ffi::api::checkout::settle_preview(1486, vec![voucher(1152), cash()])
        .await
        .unwrap();
    assert_eq!(
        p.payments,
        vec![
            row(PaymentMethod::MealVoucher, 1152),
            row(PaymentMethod::Cash, 334)
        ]
    );
    assert_eq!(p.rounding_cents, 1);
    assert_eq!(p.cash_due_cents, 335);
    assert!(p.has_cash);
}

#[tokio::test]
async fn round5_meal_vouchers_card_and_cash() {
    let parts = vec![voucher(313), card(397), cash()];
    let p = pos_ffi::api::checkout::settle_preview(1486, parts.clone())
        .await
        .unwrap();
    assert_eq!(
        p.payments,
        vec![
            row(PaymentMethod::MealVoucher, 313),
            row(PaymentMethod::Card, 397),
            row(PaymentMethod::Cash, 776)
        ]
    );
    assert_eq!(p.rounding_cents, -1);
    assert_eq!(p.cash_due_cents, 775);
    assert_eq!(
        pos_ffi::api::checkout::cash_due_cents(1486, parts)
            .await
            .unwrap(),
        775
    );
}

#[tokio::test]
async fn uc4119_three_thirds_cash_last() {
    let p = pos_ffi::api::checkout::settle_preview(9312, vec![cash(), card(3104), voucher(3104)])
        .await
        .unwrap();
    assert_eq!(
        p.payments,
        vec![
            row(PaymentMethod::Card, 3104),
            row(PaymentMethod::MealVoucher, 3104),
            row(PaymentMethod::Cash, 3104)
        ]
    );
    assert_eq!(p.rounding_cents, 1);
    assert_eq!(p.cash_due_cents, 3105);
}

#[tokio::test]
async fn meal_vouchers_for_the_whole_sale_need_no_cash() {
    let p = pos_ffi::api::checkout::settle_preview(1486, vec![voucher(1486)])
        .await
        .unwrap();
    assert_eq!(p.payments, vec![row(PaymentMethod::MealVoucher, 1486)]);
    assert_eq!(p.rounding_cents, 0);
    // A cash part with nothing left to pay is dropped.
    let p = pos_ffi::api::checkout::settle_preview(1486, vec![voucher(1486), cash()])
        .await
        .unwrap();
    assert_eq!(p.payments, vec![row(PaymentMethod::MealVoucher, 1486)]);
    assert!(!p.has_cash);
    assert_eq!(p.cash_due_cents, 0);
}

#[tokio::test]
async fn a_zero_sale_is_a_cash_payment_of_zero() {
    let p = pos_ffi::api::checkout::settle_preview(0, vec![card(100)])
        .await
        .unwrap();
    assert_eq!(p.payments, vec![row(PaymentMethod::Cash, 0)]);
    assert_eq!(p.rounding_cents, 0);
    assert_eq!(p.cash_due_cents, 0);
    assert!(!p.has_cash);
    assert_eq!(
        pos_ffi::api::checkout::cash_due_cents(0, vec![])
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn refused_settlements_answer_the_checkout_codes() {
    use pos_ffi::api::checkout::{cash_due_cents, settle_preview};
    let refuse = |code: &'static str, total: i64, parts: Vec<PaymentPart>| async move {
        assert_coded(settle_preview(total, parts.clone()).await, code);
        assert_coded(cash_due_cents(total, parts).await, code);
    };

    refuse("payment_exceeds_total", 1000, vec![card(1001)]).await;
    refuse("payment_exceeds_total", 1000, vec![card(600), voucher(500)]).await;
    refuse("payment_incomplete", 1000, vec![card(600)]).await;
    refuse("payment_incomplete", 1000, vec![]).await;
    refuse("payment_invalid", 1000, vec![card(0), cash()]).await;
    refuse("payment_invalid", 1000, vec![card(-5), cash()]).await;
    refuse(
        "payment_invalid",
        1000,
        vec![PaymentPart {
            method: PaymentMethod::Card,
            amount_cents: None,
        }],
    )
    .await;
    refuse(
        "payment_invalid",
        1000,
        vec![PaymentPart {
            method: PaymentMethod::Cash,
            amount_cents: Some(500),
        }],
    )
    .await;
    refuse("payment_invalid", 1000, vec![cash(), cash()]).await;
    refuse("payment_invalid", -100, vec![cash()]).await;
    // Parts that would wrap an i64 sum are refused, not settled.
    refuse(
        "payment_exceeds_total",
        1000,
        vec![card(i64::MAX), card(i64::MAX), cash()],
    )
    .await;
    refuse(
        "payment_exceeds_total",
        1000,
        vec![card(i64::MAX), voucher(2)],
    )
    .await;
    // A method never offered at the till is refused like `checkout` refuses it.
    for method in [PaymentMethod::App, PaymentMethod::Online] {
        refuse(
            "payment_invalid",
            1000,
            vec![PaymentPart {
                method,
                amount_cents: Some(1000),
            }],
        )
        .await;
    }

    // The refusal carries the code alone, like the engine's.
    match err_of(settle_preview(1000, vec![card(600)]).await) {
        PosError::Coded { params, .. } => assert!(params.is_empty()),
        other => panic!("{other:?}"),
    }
}
