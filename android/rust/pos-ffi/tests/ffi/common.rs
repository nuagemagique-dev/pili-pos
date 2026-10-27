//! Contract tests of the shared records (`records::common`, PD56).
//!
//! Output types: a real engine value (a command run on a `TestApp` against the admission FDM
//! mock) or a hand-built one with every `Option` set and one with every `Option` empty is
//! converted with `convert::out` and the two serde forms must be equal. Input types: an FFI value
//! with every field set is converted with `convert::inp` and the engine fields are checked. Enums
//! are checked variant by variant.

use std::fmt::Debug;

use pos_core::order::OrderType;
use pos_core::vat::VatCode as EngineVatCode;
use pos_engine::commands::bar_slip::SlipOutcome as EngineSlipOutcome;
use pos_engine::commands::checkout::{CheckoutResult as EngineCheckoutResult, do_checkout_split};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::delivery::{
    DeliveryOutcome as EngineDeliveryOutcome, EmailStatus as EngineEmailStatus,
    RequestedMedium as EngineRequestedMedium, TicketDeliveryRequest as EngineDeliveryRequest,
};
use pos_engine::commands::fdm_monitor::{
    FdmHealth as EngineFdmHealth, FdmSeverity as EngineFdmSeverity, do_fdm_check_now,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::session::{
    LockReason as EngineLockReason, SessionLocked as EngineSessionLocked,
};
use pos_engine::commands::social::PresenceFailed as EnginePresenceFailed;
use pos_engine::commands::tables::do_sign_table_order;
use pos_engine_testkit::TestApp;
use pos_ffi::records::common::{self as ffi, contract_inp, contract_out};
use pos_fiscal::fdm_client::SignMessages as EngineSignMessages;
use pos_fiscal::payment::{PaymentMethod as EnginePaymentMethod, PaymentPart as EnginePaymentPart};
use pos_fiscal::response::{
    DataItem as EngineDataItem, ExtensionItem as EngineExtensionItem,
    FdmReference as EngineFdmReference, LocationItem as EngineLocationItem,
    MessageItem as EngineMessageItem, VatCalcItem as EngineVatCalcItem,
};
use pos_fiscal::ticket::{
    FiscalTicket as EngineFiscalTicket, OriginalControlData as EngineOriginalControlData,
    ReceiptLanguage as EngineReceiptLanguage, TicketLine as EngineTicketLine,
    TicketPayment as EngineTicketPayment,
};
use pos_fiscal::types::{
    EventLabel as EngineEventLabel, FdmWarningCode as EngineFdmWarningCode,
    MessageCategory as EngineMessageCategory, ShowPos as EngineShowPos,
    TicketMedium as EngineTicketMedium, VatLabel as EngineVatLabel,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json, to_value};

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

/// `out_equal` plus the way back: the record converts to an engine value with the same form.
fn both_ways<E: Serialize + DeserializeOwned, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = out_equal(engine);
    let back: E = contract_inp(&record).unwrap_or_else(|e| panic!("convert::inp: {e}"));
    assert_eq!(json_of(engine), json_of(&back), "engine after a round trip");
    record
}

/// Every variant of an output-only enum survives `convert::out` and keeps its serde name.
fn enum_out<E: Serialize, F: Serialize + DeserializeOwned + PartialEq + Debug>(variants: &[E]) {
    assert!(!variants.is_empty());
    for (i, v) in variants.iter().enumerate() {
        let record: F = out_equal(v);
        for (j, w) in variants.iter().enumerate() {
            let other: F = out_equal(w);
            assert_eq!(i == j, record == other, "{record:?} vs {other:?}");
        }
    }
}

/// `enum_out` for an enum the engine also reads.
fn enum_both<
    E: Serialize + DeserializeOwned,
    F: Serialize + DeserializeOwned + PartialEq + Debug,
>(
    variants: &[E],
) {
    enum_out::<E, F>(variants);
    for v in variants {
        both_ways::<E, F>(v);
    }
}

// ───────────────────────────── real engine values ─────────────────────────────

const INSZ: &str = "12345678958";

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

fn order(app: &TestApp, table: Option<i32>, lines: &[(i64, i32)]) -> i64 {
    let id = do_create_order(
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

fn paper() -> EngineDeliveryRequest {
    EngineDeliveryRequest {
        medium: EngineRequestedMedium::Paper,
        email: None,
    }
}

async fn cash_sale(
    app: &TestApp,
    delivery: Option<&EngineDeliveryRequest>,
) -> EngineCheckoutResult {
    let id = order(app, None, &[(1, 2), (2, 1)]);
    let result = do_checkout_split(&app.state, id, &[EnginePaymentPart::cash()], delivery)
        .await
        .unwrap();
    assert!(result.success, "sale: {:?}", result.error);
    result
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cash_sale_without_printer_converts() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, Some(&paper())).await;
    let record: ffi::CheckoutResult = out_equal(&sale);
    assert!(record.success);
    let ticket = record.ticket.expect("a sale has a ticket");
    assert_eq!(ticket.total, 52.0);
    assert_eq!(ticket.event_label, ffi::EventLabel::N);
    assert!(!ticket.lines.is_empty());
    assert!(!ticket.vat_breakdown.is_empty());
    assert!(ticket.fdm_ref.is_some());
    assert!(ticket.short_signature.is_some());
    assert!(ticket.original.is_none());
    assert_eq!(record.kitchen_slip, Some(ffi::SlipOutcome::NoPrinter));
    let delivery = record.delivery.expect("a delivery was asked for");
    assert_eq!(delivery.medium, ffi::TicketMedium::Paper);
    assert_eq!(delivery.email, ffi::EmailStatus::NotRequested);
    assert!(delivery.needs_paper, "no printer: the ticket is still owed");
    assert!(!delivery.paper_printed);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cash_sale_with_printer_converts() {
    let app = app_on_mock(true).await;
    let sale = cash_sale(&app, Some(&paper())).await;
    let record: ffi::CheckoutResult = out_equal(&sale);
    assert_eq!(record.kitchen_slip, Some(ffi::SlipOutcome::Printed));
    let delivery = record.delivery.expect("a delivery was asked for");
    assert!(delivery.paper_printed);
    assert!(!delivery.needs_paper);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_sale_without_delivery_request_has_no_delivery() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, None).await;
    let record: ffi::CheckoutResult = out_equal(&sale);
    // No request means the default one: a paper ticket.
    let delivery = record.delivery.expect("the default request is paper");
    assert_eq!(delivery.medium, ffi::TicketMedium::Paper);
    assert_eq!(record.fiscal_event_id, sale.fiscal_event_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_refund_ticket_carries_the_original_control_data() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, None).await;
    let refund = do_refund_checkout(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        INSZ.to_string(),
    )
    .await
    .unwrap();
    assert!(refund.success, "refund: {:?}", refund.error);
    let record: ffi::CheckoutResult = out_equal(&refund);
    let ticket = record.ticket.expect("a refund has a ticket");
    assert!(ticket.is_refund);
    let original = ticket.original.expect("a refund refers to its sale");
    let sale_ref: ffi::FdmReference = out_equal(
        sale.ticket
            .as_ref()
            .and_then(|t| t.fdm_ref.as_ref())
            .expect("the sale's FDM reference"),
    );
    assert_eq!(original.fdm_ref, sale_ref);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_copy_ticket_has_the_copy_time() {
    let app = app_on_mock(false).await;
    let sale = cash_sale(&app, None).await;
    let copy = do_copy_ticket(&app.state, sale.fiscal_event_id.unwrap(), INSZ.to_string())
        .await
        .unwrap();
    assert!(copy.success, "copy: {:?}", copy.error);
    let ticket = copy.ticket.expect("a copy of a sale has a ticket");
    assert!(ticket.copy_made_at.is_some());
    let record: ffi::FiscalTicket = out_equal(&ticket);
    assert!(record.copy_made_at.is_some());
    assert!(record.original.is_some());
    // A copy reproduces the label of the event it copies.
    assert_eq!(record.event_label, ffi::EventLabel::N);
}

#[tokio::test(flavor = "multi_thread")]
async fn slip_outcomes_of_real_orders_convert() {
    let app = app_on_mock(false).await;
    let table = order(&app, Some(4), &[(1, 1)]);
    let no_printer = do_sign_table_order(&app.state, table).await.unwrap();
    assert_eq!(no_printer, EngineSlipOutcome::NoPrinter);
    let record: ffi::SlipOutcome = out_equal(&no_printer);
    assert_eq!(record, ffi::SlipOutcome::NoPrinter);

    let app = app_on_mock(true).await;
    let table = order(&app, Some(5), &[(1, 1)]);
    let printed = do_sign_table_order(&app.state, table).await.unwrap();
    assert_eq!(printed, EngineSlipOutcome::Printed);
    assert_eq!(
        out_equal::<_, ffi::SlipOutcome>(&printed),
        ffi::SlipOutcome::Printed
    );
    // Nothing new since the last round.
    let nothing = do_sign_table_order(&app.state, table).await.unwrap();
    assert_eq!(nothing, EngineSlipOutcome::NothingToPrint);
    assert_eq!(
        out_equal::<_, ffi::SlipOutcome>(&nothing),
        ffi::SlipOutcome::NothingToPrint
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn fdm_health_of_a_reachable_fdm_converts() {
    let app = app_on_mock(false).await;
    let health = do_fdm_check_now(&app.state).await.unwrap();
    assert!(health.connected);
    let record: ffi::FdmHealth = both_ways(&health);
    assert!(record.connected);
    assert!(record.answered);
}

#[tokio::test(flavor = "multi_thread")]
async fn fdm_health_of_an_unreachable_fdm_converts() {
    let app = app_on_mock(false).await;
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".to_string();
    let health = do_fdm_check_now(&app.state).await.unwrap();
    assert!(!health.connected);
    let record: ffi::FdmHealth = both_ways(&health);
    assert!(!record.connected);
    assert!(!record.answered);
    assert!(record.code.is_some(), "the raw transport error is kept");
}

// ───────────────────────────── hand-built values ─────────────────────────────

fn fdm_ref(label: EngineEventLabel) -> EngineFdmReference {
    EngineFdmReference {
        fdm_id: "MCK00000001".into(),
        fdm_date_time: "2026-10-08T10:15:30.000Z".into(),
        event_label: label,
        event_counter: 7,
        total_counter: 19,
    }
}

fn ticket(full: bool) -> EngineFiscalTicket {
    EngineFiscalTicket {
        restaurant_name: "Restaurant Test".into(),
        restaurant_address: "Rue de l'Exemple 1, 4000 Liege".into(),
        restaurant_vat: "BE0499999960".into(),
        restaurant_est_no: "2123456791".into(),
        restaurant_legal_name: if full {
            "Test SRL".into()
        } else {
            String::new()
        },
        pos_id: "CPOS0031234567".into(),
        pos_sw_version: "pili-pos-android/1.0.0".into(),
        terminal_id: "POS-2-ANDROID".into(),
        device_id: "EMULATOR-TEST-0001".into(),
        operator: "Gerant Test".into(),
        pos_datetime: "2026-10-08T10:15:30+02:00".into(),
        ticket_number: 42,
        event_label: EngineEventLabel::N,
        is_training: full,
        lines: vec![EngineTicketLine {
            quantity: 1.5,
            description: "Burger du chef".into(),
            unit_price: 28.0,
            total: 42.0,
            vat_code: "B".into(),
        }],
        subtotal: 37.5,
        total: 42.0,
        vat_breakdown: vec![EngineVatCalcItem {
            label: EngineVatLabel::B,
            rate: 12.0,
            taxable_amount: 37.5,
            vat_amount: 4.5,
            total_amount: 42.0,
            out_of_scope: false,
        }],
        payments: vec![EngineTicketPayment {
            method: "Cash".into(),
            amount: 42.0,
            brand: full.then(|| "Bancontact".into()),
        }],
        rounding: full.then_some(-0.01),
        fdm_ref: full.then(|| fdm_ref(EngineEventLabel::N)),
        short_signature: full.then(|| "e2c43d6bbe9be265f2ecc1578368c224ff980c26".into()),
        verification_url: full.then(|| "https://fdm.example/v?q=1".into()),
        footer: if full {
            vec!["Merci".into(), "Bedankt".into()]
        } else {
            vec![]
        },
        ticket_type_label: "TICKET".into(),
        is_refund: full,
        lang: if full {
            EngineReceiptLanguage::Nl
        } else {
            EngineReceiptLanguage::Fr
        },
        original: full.then(|| EngineOriginalControlData {
            fdm_ref: fdm_ref(EngineEventLabel::N),
            short_signature: Some("abc123".into()),
        }),
        copy_made_at: full.then(|| "2026-10-08T11:00:00+02:00".into()),
        unpaid: full,
    }
}

fn delivery_outcome(full: bool) -> EngineDeliveryOutcome {
    EngineDeliveryOutcome {
        fiscal_event_id: 12,
        delivery_id: full.then_some(3),
        medium: if full {
            EngineTicketMedium::PaperDigital
        } else {
            EngineTicketMedium::None
        },
        email: if full {
            EngineEmailStatus::Queued
        } else {
            EngineEmailStatus::NotRequested
        },
        recipient_hint: full.then(|| "j***@example.be".into()),
        paper_printed: full,
        needs_paper: !full,
    }
}

#[test]
fn a_fiscal_ticket_with_every_option_set_converts() {
    let engine = ticket(true);
    let record: ffi::FiscalTicket = both_ways(&engine);
    assert_eq!(record.rounding, Some(-0.01));
    assert_eq!(record.lang, ffi::ReceiptLanguage::Nl);
    assert_eq!(record.footer, ["Merci", "Bedankt"]);
    assert_eq!(
        record.original.unwrap().short_signature.as_deref(),
        Some("abc123")
    );
    assert_eq!(record.lines[0].quantity, 1.5);
    assert_eq!(record.vat_breakdown[0].label, ffi::VatLabel::B);
    assert_eq!(record.payments[0].method, "Cash");
}

#[test]
fn a_fiscal_ticket_with_every_option_empty_converts() {
    let record: ffi::FiscalTicket = both_ways(&ticket(false));
    assert!(record.rounding.is_none());
    assert!(record.fdm_ref.is_none());
    assert!(record.short_signature.is_none());
    assert!(record.verification_url.is_none());
    assert!(record.original.is_none());
    assert!(record.copy_made_at.is_none());
    assert_eq!(record.lang, ffi::ReceiptLanguage::Fr);
}

#[test]
fn an_old_archived_ticket_without_the_defaulted_fields_still_reads() {
    // Tickets archived before these fields existed: the engine defaults them, so must the record.
    let mut value = json_of(&ticket(false));
    let map = value.as_object_mut().unwrap();
    for key in [
        "restaurant_legal_name",
        "pos_sw_version",
        "device_id",
        "operator",
        "lang",
        "original",
        "copy_made_at",
        "unpaid",
    ] {
        map.remove(key);
    }
    let engine: EngineFiscalTicket = serde_json::from_value(value.clone()).unwrap();
    let record: ffi::FiscalTicket = serde_json::from_value(value).unwrap();
    assert_eq!(json_of(&engine), json_of(&record));
    assert_eq!(record.lang, ffi::ReceiptLanguage::Fr);
    assert!(!record.unpaid);
}

#[test]
fn a_checkout_result_with_every_option_set_converts() {
    let engine = EngineCheckoutResult {
        success: true,
        ticket: Some(ticket(true)),
        error: Some("a warning".into()),
        fdm_connected: true,
        fiscal_event_id: Some(12),
        delivery: Some(delivery_outcome(true)),
        kitchen_slip: Some(EngineSlipOutcome::Failed),
        gift_voucher_prints: Vec::new(),
    };
    let record: ffi::CheckoutResult = out_equal(&engine);
    assert_eq!(record.fiscal_event_id, Some(12));
    assert_eq!(record.kitchen_slip, Some(ffi::SlipOutcome::Failed));
    assert_eq!(record.delivery.unwrap().email, ffi::EmailStatus::Queued);
}

#[test]
fn a_checkout_result_with_every_option_empty_converts() {
    let engine = EngineCheckoutResult {
        success: false,
        ticket: None,
        error: None,
        fdm_connected: false,
        fiscal_event_id: None,
        delivery: None,
        kitchen_slip: None,
        gift_voucher_prints: Vec::new(),
    };
    let record: ffi::CheckoutResult = out_equal(&engine);
    assert!(record.ticket.is_none() && record.delivery.is_none() && record.kitchen_slip.is_none());
}

#[test]
fn a_delivery_outcome_converts_with_and_without_options() {
    let full: ffi::DeliveryOutcome = out_equal(&delivery_outcome(true));
    assert_eq!(full.delivery_id, Some(3));
    assert_eq!(full.recipient_hint.as_deref(), Some("j***@example.be"));
    assert_eq!(full.medium, ffi::TicketMedium::PaperDigital);
    let empty: ffi::DeliveryOutcome = out_equal(&delivery_outcome(false));
    assert!(empty.delivery_id.is_none() && empty.recipient_hint.is_none());
    assert_eq!(empty.medium, ffi::TicketMedium::None);
}

#[test]
#[expect(
    clippy::many_single_char_names,
    reason = "short names for the components of one ticket keep the assertions readable"
)]
fn the_component_records_of_a_ticket_convert_alone() {
    let r: ffi::FdmReference = both_ways(&fdm_ref(EngineEventLabel::C));
    assert_eq!(r.event_label, ffi::EventLabel::C);
    assert_eq!((r.event_counter, r.total_counter), (7, 19));
    let v: ffi::VatCalcItem = both_ways(&ticket(true).vat_breakdown[0]);
    assert_eq!(v.rate, 12.0);
    let l: ffi::TicketLine = both_ways(&ticket(true).lines[0]);
    assert_eq!(l.vat_code, "B");
    let p: ffi::TicketPayment = both_ways(&ticket(true).payments[0]);
    assert_eq!(p.amount, 42.0);
    let o: ffi::OriginalControlData = both_ways(&EngineOriginalControlData {
        fdm_ref: fdm_ref(EngineEventLabel::N),
        short_signature: None,
    });
    assert!(o.short_signature.is_none());
}

fn message(full: bool) -> EngineMessageItem {
    EngineMessageItem {
        message: "The FDM buffer is nearly full".into(),
        locations: full.then(|| {
            vec![EngineLocationItem {
                line: 3,
                column: 14,
            }]
        }),
        extensions: EngineExtensionItem {
            category: EngineMessageCategory::Fdm,
            code: EngineFdmWarningCode::BufferNearFull,
            data: full.then(|| {
                vec![EngineDataItem {
                    name: "percent".into(),
                    value: "93".into(),
                }]
            }),
            show_pos: EngineShowPos::Mandatory,
        },
    }
}

#[test]
fn sign_messages_convert_with_and_without_options() {
    let full = EngineSignMessages {
        mutation: "signSale".into(),
        booking_date: Some("2026-10-08".into()),
        warnings: vec![message(true)],
        informations: vec![message(false), message(true)],
    };
    let record: ffi::SignMessages = out_equal(&full);
    assert_eq!(record.booking_date.as_deref(), Some("2026-10-08"));
    assert_eq!(record.warnings[0].locations.as_ref().unwrap()[0].column, 14);
    assert_eq!(record.informations[0].locations, None);
    assert_eq!(record.informations[0].extensions.data, None);
    assert_eq!(
        record.informations[1].extensions.data.as_ref().unwrap()[0].value,
        "93"
    );
    assert_eq!(
        record.warnings[0].extensions.show_pos,
        ffi::FdmMessageDisplay::Mandatory
    );

    let empty = EngineSignMessages {
        mutation: "signReport".into(),
        booking_date: None,
        warnings: vec![],
        informations: vec![],
    };
    let record: ffi::SignMessages = out_equal(&empty);
    assert!(record.booking_date.is_none() && record.warnings.is_empty());
}

#[test]
fn the_message_records_convert_alone() {
    let m: ffi::MessageItem = both_ways(&message(true));
    assert_eq!(m.extensions.code, ffi::FdmWarningCode::BufferNearFull);
    let l: ffi::LocationItem = both_ways(&EngineLocationItem {
        line: -1,
        column: 0,
    });
    assert_eq!((l.line, l.column), (-1, 0));
    let d: ffi::DataItem = both_ways(&EngineDataItem {
        name: "n".into(),
        value: "v".into(),
    });
    assert_eq!(d.name, "n");
    let e: ffi::ExtensionItem = both_ways(&message(false).extensions);
    assert_eq!(e.data, None);
}

#[test]
fn an_fdm_code_or_category_the_spec_does_not_list_is_kept_readable() {
    // `#[serde(other)]`: an unknown code must not make a whole message unreadable.
    let code: ffi::FdmWarningCode = contract_out(&json!("BRAND_NEW_CODE")).unwrap();
    assert_eq!(code, ffi::FdmWarningCode::Unrecognized);
    let engine: EngineFdmWarningCode = serde_json::from_value(json!("BRAND_NEW_CODE")).unwrap();
    assert_eq!(engine, EngineFdmWarningCode::Unrecognized);
    let category: ffi::MessageCategory = contract_out(&json!("SOMEONE_ELSE")).unwrap();
    assert_eq!(category, ffi::MessageCategory::Other);
    // The engine's own `Unrecognized` and `Other` convert as well.
    both_ways::<_, ffi::FdmWarningCode>(&EngineFdmWarningCode::Unrecognized);
    both_ways::<_, ffi::MessageCategory>(&EngineMessageCategory::Other);
}

#[test]
fn health_session_and_presence_payloads_convert() {
    let health = EngineFdmHealth {
        connected: false,
        answered: true,
        severity: EngineFdmSeverity::Blocking,
        code: Some("BUFFER_FULL".into()),
        mandatory: true,
        checked_at: "2026-10-08T10:00:00.000Z".into(),
        next_check_at: Some("2026-10-08T10:00:30.000Z".into()),
    };
    let record: ffi::FdmHealth = both_ways(&health);
    assert_eq!(record.severity, ffi::FdmSeverity::Blocking);
    assert_eq!(
        record.next_check_at.as_deref(),
        Some("2026-10-08T10:00:30.000Z")
    );
    let empty = EngineFdmHealth {
        code: None,
        next_check_at: None,
        severity: EngineFdmSeverity::Info,
        ..health
    };
    let record: ffi::FdmHealth = both_ways(&empty);
    assert!(record.code.is_none() && record.next_check_at.is_none());

    let locked = EngineSessionLocked {
        reason: EngineLockReason::Idle,
        order_error: Some("fdm_unavailable".into()),
    };
    let record: ffi::SessionLocked = both_ways(&locked);
    assert_eq!(record.reason, ffi::LockReason::Idle);
    assert_eq!(record.order_error.as_deref(), Some("fdm_unavailable"));
    let record: ffi::SessionLocked = both_ways(&EngineSessionLocked {
        reason: EngineLockReason::Manual,
        order_error: None,
    });
    assert!(record.order_error.is_none());

    let failed = EnginePresenceFailed {
        insz: "12345678958".into(),
        clock_in: true,
        error: "FDM error".into(),
    };
    let record: ffi::PresenceFailed = both_ways(&failed);
    assert_eq!(record.insz, "12345678958");
    assert!(record.clock_in);
}

// ───────────────────────────── enums, variant by variant ─────────────────────────────

#[test]
fn vat_code_variants() {
    use EngineVatCode::*;
    enum_both::<_, ffi::VatCode>(&[A, B, C, D, X]);
}

#[test]
fn vat_label_variants() {
    use EngineVatLabel::*;
    enum_both::<_, ffi::VatLabel>(&[A, B, C, D, X]);
}

#[test]
fn event_label_variants() {
    use EngineEventLabel::*;
    enum_both::<_, ffi::EventLabel>(&[N, P, F, S, I, R, C, T]);
}

#[test]
fn payment_method_variants() {
    use EnginePaymentMethod::*;
    enum_both::<_, ffi::PaymentMethod>(&[Cash, Card, MealVoucher, App, Online]);
    assert_eq!(json_of(&MealVoucher), json!("meal_voucher"));
}

#[test]
fn requested_medium_variants() {
    use EngineRequestedMedium::*;
    enum_both::<_, ffi::RequestedMedium>(&[Paper, Digital, PaperDigital]);
    assert_eq!(ffi::RequestedMedium::default(), ffi::RequestedMedium::Paper);
}

#[test]
fn ticket_medium_variants() {
    use EngineTicketMedium::*;
    enum_both::<_, ffi::TicketMedium>(&[None, Paper, Digital, PaperDigital]);
    assert_eq!(json_of(&PaperDigital), json!("PAPER_DIGITAL"));
}

#[test]
fn receipt_language_variants() {
    use EngineReceiptLanguage::*;
    enum_both::<_, ffi::ReceiptLanguage>(&[Fr, Nl, De]);
    assert_eq!(ffi::ReceiptLanguage::default(), ffi::ReceiptLanguage::Fr);
}

#[test]
fn slip_outcome_variants() {
    use EngineSlipOutcome::*;
    enum_out::<_, ffi::SlipOutcome>(&[Printed, NothingToPrint, NoPrinter, Failed]);
    assert_eq!(json_of(&NothingToPrint), json!("nothing_to_print"));
}

#[test]
fn email_status_variants() {
    use EngineEmailStatus::*;
    enum_out::<_, ffi::EmailStatus>(&[NotRequested, Sent, Queued]);
}

#[test]
fn fdm_severity_variants() {
    use EngineFdmSeverity::*;
    enum_both::<_, ffi::FdmSeverity>(&[Blocking, Warning, Info]);
}

#[test]
fn lock_reason_variants() {
    use EngineLockReason::*;
    enum_both::<_, ffi::LockReason>(&[Manual, Idle]);
}

#[test]
fn message_display_variants() {
    use EngineShowPos::*;
    enum_both::<_, ffi::FdmMessageDisplay>(&[Mandatory, Optional, Never]);
}

#[test]
fn message_category_variants() {
    use EngineMessageCategory::*;
    enum_both::<_, ffi::MessageCategory>(&[SpfFod, Fdm, Other]);
}

#[test]
fn fdm_warning_code_variants() {
    use EngineFdmWarningCode::*;
    enum_both::<_, ffi::FdmWarningCode>(&[
        ClientCertNearExpiration,
        ClientCertExpired,
        RtcSyncFailed,
        UpdateUrlsFailed,
        UpdateTrustCertFailed,
        UpdateTaskListFailed,
        TaskFeedbackFailed,
        NopFailed,
        BufferNearFull,
        ServerCertResolveFailed,
        TransactionUploadFailed,
        InitializationFailed,
        RtcNotInitialized,
        CorruptRecordEncountered,
        UpdateParamsFailed,
        UpdateClientCertFailed,
        UpdateVatRatesFailed,
        UpdatePosAllowlistFailed,
        DuplicateRequest,
        BufferFull,
        FdmLocked,
        Unauthorized,
        InvalidRequest,
        InternalError,
        UndefinedError,
        UndefinedOther,
        FdmNotOperational,
        UnknownPos,
        UpdatePosVatno,
        UpdatePosEstno,
        Unrecognized,
    ]);
}

// ───────────────────────────── input types ─────────────────────────────

#[test]
fn a_payment_part_with_every_field_set_reaches_the_engine() {
    let record = ffi::PaymentPart {
        method: ffi::PaymentMethod::MealVoucher,
        amount_cents: Some(1250),
        eft_payment_id: None,
        gift_voucher_id: None,
    };
    let engine: EnginePaymentPart = contract_inp(&record).unwrap();
    assert_eq!(engine.method, EnginePaymentMethod::MealVoucher);
    assert_eq!(engine.amount_cents, Some(1250));
}

#[test]
fn a_cash_payment_part_has_no_amount() {
    let record = ffi::PaymentPart {
        method: ffi::PaymentMethod::Cash,
        amount_cents: None,
        eft_payment_id: None,
        gift_voucher_id: None,
    };
    let engine: EnginePaymentPart = contract_inp(&record).unwrap();
    assert_eq!(engine, EnginePaymentPart::cash());
    // And back: the engine's part reads as the record.
    let back: ffi::PaymentPart = contract_out(&json!({"method": "cash"})).unwrap();
    assert_eq!(back, record);
}

#[test]
fn every_payment_method_reaches_the_engine_in_a_part() {
    let pairs = [
        (ffi::PaymentMethod::Cash, EnginePaymentMethod::Cash),
        (ffi::PaymentMethod::Card, EnginePaymentMethod::Card),
        (
            ffi::PaymentMethod::MealVoucher,
            EnginePaymentMethod::MealVoucher,
        ),
        (ffi::PaymentMethod::App, EnginePaymentMethod::App),
        (ffi::PaymentMethod::Online, EnginePaymentMethod::Online),
    ];
    for (record, expected) in pairs {
        let engine: EnginePaymentPart = contract_inp(&ffi::PaymentPart {
            method: record,
            amount_cents: Some(1),
            eft_payment_id: None,
            gift_voucher_id: None,
        })
        .unwrap();
        assert_eq!(engine, EnginePaymentPart::of(expected, 1));
    }
}

#[test]
fn a_delivery_request_with_every_field_set_reaches_the_engine() {
    let record = ffi::TicketDeliveryRequest {
        medium: ffi::RequestedMedium::PaperDigital,
        email: Some("client@example.be".into()),
    };
    let engine: EngineDeliveryRequest = contract_inp(&record).unwrap();
    assert_eq!(engine.medium, EngineRequestedMedium::PaperDigital);
    assert_eq!(engine.email.as_deref(), Some("client@example.be"));
}

#[test]
fn a_delivery_request_with_defaults_reaches_the_engine() {
    let record = ffi::TicketDeliveryRequest::default();
    let engine: EngineDeliveryRequest = contract_inp(&record).unwrap();
    assert_eq!(engine.medium, EngineRequestedMedium::Paper);
    assert_eq!(engine.email, None);
    // The wire form is camelCase and every field has a default, like the engine's.
    let from_empty: ffi::TicketDeliveryRequest = serde_json::from_value(json!({})).unwrap();
    assert_eq!(from_empty, record);
}

#[test]
fn records_refuse_unknown_fields() {
    let part = serde_json::from_value::<ffi::PaymentPart>(json!({"method": "cash", "tip": 1}));
    assert!(part.is_err());
    let ticket = serde_json::from_value::<ffi::TicketLine>(json!({
        "quantity": 1.0, "description": "x", "unit_price": 1.0, "total": 1.0,
        "vat_code": "A", "discount": 0
    }));
    assert!(ticket.is_err());
    // A value the engine would have to be changed for is a contract failure, not a panic.
    let err = contract_out::<_, ffi::FdmHealth>(&json!({"connected": true})).unwrap_err();
    assert!(matches!(err, pos_ffi::PosError::Internal { .. }), "{err:?}");
}

// ───────────────────────────── CallbackEvent (contract: custom) ─────────────────────────────

fn callback(
    extra: Value,
    previous: Option<Value>,
    new: Option<Value>,
) -> pos_hubrise_types::CallbackEvent {
    let Value::Object(extra) = extra else {
        panic!("an object");
    };
    pos_hubrise_types::CallbackEvent {
        id: "ev_1".into(),
        resource_type: "order".into(),
        event_type: "create".into(),
        created_at: "2026-10-08T10:00:00+02:00".into(),
        previous_state: previous,
        new_state: new,
        extra,
    }
}

#[test]
fn a_callback_event_keeps_the_flattened_fields() {
    let engine = callback(
        json!({"location_id": "loc9", "order_id": "ord7", "nested": {"a": [1, 2, null]}}),
        Some(json!({"status": "new"})),
        Some(json!({"status": "accepted", "total": "12.00 EUR"})),
    );
    let record = ffi::CallbackEvent::from_engine(&engine).unwrap();
    assert_eq!(record.id, "ev_1");
    assert_eq!(record.resource_type, "order");
    assert_eq!(record.event_type, "create");
    assert_eq!(record.created_at, "2026-10-08T10:00:00+02:00");
    let extra: Value = serde_json::from_str(&record.extra).unwrap();
    assert_eq!(
        extra,
        json!({"location_id": "loc9", "order_id": "ord7", "nested": {"a": [1, 2, null]}})
    );
    let previous: Value = serde_json::from_str(record.previous_state.as_deref().unwrap()).unwrap();
    assert_eq!(previous, json!({"status": "new"}));
    let new: Value = serde_json::from_str(record.new_state.as_deref().unwrap()).unwrap();
    assert_eq!(new["total"], "12.00 EUR");

    // The engine's wire form is the record's fields with `extra` spread next to them.
    let mut flat = json_of(&engine);
    let flat = flat.as_object_mut().unwrap();
    let mut rebuilt = serde_json::Map::new();
    rebuilt.insert("id".into(), json!(record.id));
    rebuilt.insert("resource_type".into(), json!(record.resource_type));
    rebuilt.insert("event_type".into(), json!(record.event_type));
    rebuilt.insert("created_at".into(), json!(record.created_at));
    rebuilt.insert("previous_state".into(), previous);
    rebuilt.insert("new_state".into(), new);
    for (k, v) in extra.as_object().unwrap() {
        rebuilt.insert(k.clone(), v.clone());
    }
    assert_eq!(&rebuilt, flat);

    // And back.
    let back = record.to_engine().unwrap();
    assert_eq!(json_of(&back), json_of(&engine));
    assert_eq!(back.extra, engine.extra);
}

#[test]
fn a_callback_event_without_states_or_extra_fields() {
    let engine = callback(json!({}), None, None);
    let record = ffi::CallbackEvent::from_engine(&engine).unwrap();
    assert_eq!(record.previous_state, None);
    assert_eq!(record.new_state, None);
    assert_eq!(record.extra, "{}");
    let back = record.to_engine().unwrap();
    assert!(back.previous_state.is_none() && back.new_state.is_none());
    assert!(back.extra.is_empty());
    assert_eq!(json_of(&back), json_of(&engine));
}

#[test]
fn a_callback_event_from_the_wire_converts_like_one_built_by_hand() {
    // What the HubRise poller really parses: unknown keys land in `extra`, `null` states are None.
    let wire = json!({
        "id": "ev_2", "resource_type": "catalog", "event_type": "update",
        "created_at": "2026-10-08T10:00:00+02:00",
        "previous_state": null, "new_state": {"id": "cat1"},
        "location_id": "loc9", "catalog_id": "cat1"
    });
    let engine: pos_hubrise_types::CallbackEvent = serde_json::from_value(wire.clone()).unwrap();
    let record = ffi::CallbackEvent::from_engine(&engine).unwrap();
    assert_eq!(record.previous_state, None);
    assert_eq!(
        serde_json::from_str::<Value>(&record.extra).unwrap(),
        json!({"location_id": "loc9", "catalog_id": "cat1"})
    );
    assert_eq!(json_of(&record.to_engine().unwrap()), wire);
}

#[test]
fn a_callback_event_record_with_a_bad_extra_is_a_contract_error() {
    let good = ffi::CallbackEvent::from_engine(&callback(json!({}), None, None)).unwrap();
    for extra in ["[1, 2]", "\"text\"", "null", "{not json"] {
        let err = ffi::CallbackEvent {
            extra: extra.into(),
            ..good.clone()
        }
        .to_engine()
        .unwrap_err();
        let pos_ffi::PosError::Internal { detail } = err else {
            panic!("an Internal error for {extra}");
        };
        assert!(detail.starts_with("contract: CallbackEvent"), "{detail}");
    }
    let err = ffi::CallbackEvent {
        new_state: Some("{oops".into()),
        ..good
    }
    .to_engine()
    .unwrap_err();
    assert!(matches!(err, pos_ffi::PosError::Internal { .. }));
}
