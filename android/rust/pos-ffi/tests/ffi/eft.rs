//! Tests of the card payment terminal facade (engine module eft): `eft_pay`, `eft_abort`,
//! `eft_check`, `list_eft_payments`, `check_payment_terminal`.
//!
//! (a) Contract: the FFI `EftPayment` is the engine's field by field, with every option set and
//! with every option `None`; an unknown key is a contract error; `PaymentPart` carries the
//! optional `eft_payment_id` (absent from the wire form when `None`).
//!
//! (b) Wiring without a terminal: every method answers `eft_not_configured`, the list is empty.
//!
//! (c) Wiring with a terminal: an engine whose `pos.json` has a `payment_terminal` pointing to the
//! in-process terminal mock (`pos_eft_mock`) and the FDM mock; the full path from `eft_pay` to a
//! checkout that settles the approved payment, a decline, an abort, an unknown answer then
//! `eft_check`, and a call dropped by the caller that still ends in the database.

use std::sync::Arc;
use std::time::Duration;

use pos_eft_mock::{Behaviour, Mock, Next};
use pos_engine::commands::eft::EftPayment as EngineEftPayment;
use pos_ffi::records::common::{PaymentMethod, PaymentPart, contract_inp, contract_out};
use pos_ffi::records::eft::{EftPayment, PaymentTerminalStatus, UnsettledCardPayment};
use pos_ffi::records::orders::{AddOrderLineInput, CreateOrderInput, OrderType};
use pos_ffi::{EngineOptions, PosEngine, PosError};
use serde_json::{json, to_value};

use crate::support::{MANAGER_INSZ, MANAGER_PIN, options_for};

/// Seeded product: Perrier, 2,50 EUR.
const PERRIER: i64 = 12007;

fn coded(err: &PosError) -> &str {
    match err {
        PosError::Coded { code, .. } => code,
        other => panic!("expected a coded error, got {other:?}"),
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

fn engine_payment(full: bool) -> EngineEftPayment {
    // The engine type has no public constructor: build it through its serde form.
    let text = |s: &str| if full { json!(s) } else { json!(null) };
    serde_json::from_value::<EngineEftPaymentShape>(json!({
        "id": 7, "order_id": 3, "amount_cents": 1250, "status": "approved",
        "card_type": text("debit"), "brand": text("Bancontact"),
        "terminal_reference": text("T-1"), "authorisation_code": text("A1B2C3"),
        "masked_pan": text("************1234"), "detail": text("none"),
        "created_at": "2026-10-12T10:00:00+02:00", "used": full
    }))
    .unwrap()
    .0
}

/// `EftPayment` of the engine only serializes: parse the fields back into a record equal to it
/// for the comparison. The wire form is what the contract is about.
struct EngineEftPaymentShape(EngineEftPayment);

impl<'de> serde::Deserialize<'de> for EngineEftPaymentShape {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let record = EftPayment::deserialize(d)?;
        Ok(Self(EngineEftPayment {
            id: record.id,
            order_id: record.order_id,
            amount_cents: record.amount_cents,
            status: record.status,
            card_type: record.card_type,
            brand: record.brand,
            terminal_reference: record.terminal_reference,
            authorisation_code: record.authorisation_code,
            masked_pan: record.masked_pan,
            detail: record.detail,
            created_at: record.created_at,
            used: record.used,
        }))
    }
}

#[test]
fn a_payment_with_every_option_set_converts_field_by_field() {
    let engine = engine_payment(true);
    let record: EftPayment = contract_out(&engine).expect("convert::out");
    assert_eq!(record.brand.as_deref(), Some("Bancontact"));
    assert_eq!(record.masked_pan.as_deref(), Some("************1234"));
    assert!(record.used);
    assert_eq!(to_value(&engine).unwrap(), to_value(&record).unwrap());
}

#[test]
fn a_payment_with_every_option_none_converts() {
    let engine = engine_payment(false);
    let record: EftPayment = contract_out(&engine).expect("convert::out");
    assert_eq!(record.card_type, None);
    assert_eq!(record.brand, None);
    assert_eq!(record.detail, None);
    assert!(!record.used);
    assert_eq!(to_value(&engine).unwrap(), to_value(&record).unwrap());
}

#[test]
fn a_payment_with_an_unknown_key_is_a_contract_error() {
    let mut value = to_value(engine_payment(true)).unwrap();
    value["somethingNew"] = json!(1);
    let result: Result<EftPayment, PosError> = contract_out(&value);
    assert!(
        matches!(result, Err(PosError::Internal { .. })),
        "{result:?}"
    );
}

#[test]
fn a_terminal_status_converts_and_refuses_unknown_keys() {
    let wire = json!({
        "configured": true, "kind": "ccv_opi", "connected": false, "busy": false,
        "detail": "connection refused"
    });
    let record: PaymentTerminalStatus = contract_out(&wire).expect("convert::out");
    assert_eq!(record.kind.as_deref(), Some("ccv_opi"));
    assert_eq!(record.detail.as_deref(), Some("connection refused"));
    assert_eq!(to_value(&record).unwrap(), wire);
    let mut extra = wire;
    extra["somethingNew"] = json!(1);
    let result: Result<PaymentTerminalStatus, PosError> = contract_out(&extra);
    assert!(
        matches!(result, Err(PosError::Internal { .. })),
        "{result:?}"
    );
}

#[test]
fn an_unsettled_payment_converts_and_refuses_unknown_keys() {
    let wire = json!({
        "payment_id": 7, "order_id": 3, "order_type": "dine_in", "table_number": 12,
        "amount_cents": 1250, "brand": "Bancontact", "created_at": "2026-10-12T10:00:00+02:00"
    });
    let record: UnsettledCardPayment = contract_out(&wire).expect("convert::out");
    assert_eq!(record.table_number, Some(12));
    assert_eq!(to_value(&record).unwrap(), wire);
    let takeaway = json!({
        "payment_id": 8, "order_id": 4, "order_type": "takeaway", "table_number": null,
        "amount_cents": 500, "brand": null, "created_at": "2026-10-12T10:05:00+02:00"
    });
    let record: UnsettledCardPayment = contract_out(&takeaway).expect("convert::out");
    assert_eq!((record.table_number, record.brand), (None, None));
    let mut extra = wire;
    extra["somethingNew"] = json!(1);
    let result: Result<UnsettledCardPayment, PosError> = contract_out(&extra);
    assert!(
        matches!(result, Err(PosError::Internal { .. })),
        "{result:?}"
    );
}

#[test]
fn a_payment_part_carries_the_terminal_payment_id() {
    let part = PaymentPart {
        method: PaymentMethod::Card,
        amount_cents: Some(1250),
        eft_payment_id: Some(7),
        gift_voucher_id: None,
    };
    let engine: pos_fiscal::payment::PaymentPart = contract_inp(&part).expect("convert::inp");
    assert_eq!(engine.eft_payment_id, Some(7));
    assert_eq!(engine.amount_cents, Some(1250));
    // A hand-typed card has no id and no key on the wire.
    let typed = PaymentPart {
        eft_payment_id: None,
        ..part
    };
    assert!(to_value(typed).unwrap().get("eft_payment_id").is_none());
    assert_eq!(to_value(part).unwrap()["eft_payment_id"], json!(7));
}

// ───────────────────────────── (b) without a terminal ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn without_a_terminal_every_method_is_eft_not_configured() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");

    assert_eq!(
        coded(&e.eft_pay(1, 100).await.unwrap_err()),
        "eft_not_configured"
    );
    assert_eq!(
        coded(&e.eft_abort().await.unwrap_err()),
        "eft_not_configured"
    );
    // A payment that does not exist is not usable (the terminal is never asked).
    assert_eq!(
        coded(&e.eft_check(99).await.unwrap_err()),
        "eft_payment_not_usable"
    );
    assert_eq!(e.list_eft_payments(1).await.unwrap(), vec![]);
    assert_eq!(e.list_unsettled_card_payments().await.unwrap(), vec![]);
    // Nothing linked: the header shows no light, nothing is probed.
    assert_eq!(
        e.check_payment_terminal().await.unwrap(),
        PaymentTerminalStatus {
            configured: false,
            kind: None,
            connected: false,
            busy: false,
            detail: None,
        }
    );
    assert_eq!(e.get_settings().await.unwrap().payment_terminal, None);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.eft_pay(1, 100).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(coded(&e.eft_abort().await.unwrap_err()), "not_logged_in");
    assert_eq!(coded(&e.eft_check(1).await.unwrap_err()), "not_logged_in");
    assert_eq!(
        coded(&e.list_eft_payments(1).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.list_unsettled_card_payments().await.unwrap_err()),
        "not_logged_in"
    );
    e.shutdown().await.unwrap();
}

// ───────────────────────────── (c) with the terminal mock ─────────────────────────────

struct Rig {
    engine: Arc<PosEngine>,
    mock: Mock,
    _dir: tempfile::TempDir,
}

async fn open_with_terminal() -> Rig {
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind the FDM mock");
    let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(fdm, pos_fdm_mock::app())
            .await
            .expect("serve the FDM mock");
    });
    let printer_mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
    let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
        .await
        .expect("bind the printer mock")
        .to_string();
    let mock = Mock::start();
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
            "table_count": 25,
            "payment_terminal": {{
                "kind": "worldline_terminal_api", "base_url": "{}",
                "merchant_id": "DEMO-UMID", "terminal_id": "DEMO-UTID",
                "integrator_id": "pili-ffi-test", "timeout_seconds": 30 }}
            }}"#,
            mock.url()
        ),
    )
    .unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
        .expect("seed the database");
    let options: EngineOptions = options_for(dir.path());
    let engine = PosEngine::open(options).unwrap_or_else(|e| panic!("open: {e}"));
    engine
        .login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");
    // The login queued a clock-in; while its sign waits for the FDM no card is charged.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !engine
        .list_fiscal_events(None)
        .await
        .unwrap()
        .iter()
        .any(|ev| ev.event_type == "S")
    {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the clock-in was never signed"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Rig {
        engine,
        mock,
        _dir: dir,
    }
}

/// An open takeaway order of one Perrier (2,50 EUR).
async fn order(engine: &PosEngine) -> i64 {
    let order = engine
        .create_order(CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER_INSZ.into(),
        })
        .await
        .expect("create_order");
    engine
        .add_order_line(AddOrderLineInput {
            order_id: order.id,
            menu_item_id: PERRIER,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        })
        .await
        .expect("add_order_line");
    order.id
}

fn next(result: Behaviour, delay_ms: u64) -> Next {
    Next {
        result,
        delay_ms,
        ..Next::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_approved_payment_is_listed_then_settled_by_the_checkout() {
    let rig = open_with_terminal().await;
    let e = &rig.engine;
    assert_eq!(
        e.get_settings().await.unwrap().payment_terminal.as_deref(),
        Some("worldline_terminal_api")
    );
    let status = e.check_payment_terminal().await.expect("status");
    assert!(status.configured && !status.busy, "{status:?}");
    assert_eq!(status.kind.as_deref(), Some("worldline_terminal_api"));
    assert_eq!(status.connected, status.detail.is_none());
    let order_id = order(e).await;

    let payment = e.eft_pay(order_id, 250).await.expect("eft_pay");
    assert_eq!(payment.status, "approved", "{payment:?}");
    assert_eq!(payment.amount_cents, 250);
    assert_eq!(payment.order_id, order_id);
    assert!(!payment.used);

    // The crash-recovery list: approved and unused.
    assert_eq!(
        e.list_eft_payments(order_id).await.unwrap(),
        vec![payment.clone()]
    );
    // The Z warns about the money no ticket records yet.
    let unsettled = e.list_unsettled_card_payments().await.unwrap();
    assert_eq!(unsettled.len(), 1, "{unsettled:?}");
    assert_eq!(unsettled[0].payment_id, payment.id);
    assert_eq!(unsettled[0].order_id, order_id);
    assert_eq!(unsettled[0].amount_cents, 250);
    // An approved payment answers eft_check as it is.
    assert_eq!(e.eft_check(payment.id).await.unwrap(), payment);
    // Nothing runs: abort is Ok.
    e.eft_abort()
        .await
        .expect("eft_abort with nothing in flight");

    let sale = e
        .checkout(
            order_id,
            vec![PaymentPart {
                method: PaymentMethod::Card,
                amount_cents: Some(250),
                eft_payment_id: Some(payment.id),
                gift_voucher_id: None,
            }],
            None,
        )
        .await
        .expect("checkout");
    assert!(sale.success, "{:?}", sale.error);
    // Settled: no longer listed.
    assert_eq!(e.list_eft_payments(order_id).await.unwrap(), vec![]);
    assert_eq!(e.list_unsettled_card_payments().await.unwrap(), vec![]);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_declined_payment_is_answered_and_the_card_can_be_tried_again() {
    let rig = open_with_terminal().await;
    let e = &rig.engine;
    let order_id = order(e).await;

    rig.mock.next(next(Behaviour::Decline, 0));
    let payment = e.eft_pay(order_id, 250).await.expect("eft_pay");
    assert_eq!(payment.status, "declined", "{payment:?}");
    assert!(payment.detail.is_some());
    // Not listed: nothing to settle or check.
    assert_eq!(e.list_eft_payments(order_id).await.unwrap(), vec![]);

    rig.mock.next(next(Behaviour::Approve, 0));
    let again = e.eft_pay(order_id, 250).await.expect("eft_pay again");
    assert_eq!(again.status, "approved", "{again:?}");
    assert_ne!(again.id, payment.id);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_amount_over_the_order_is_refused() {
    let rig = open_with_terminal().await;
    let e = &rig.engine;
    let order_id = order(e).await;
    assert_eq!(
        coded(&e.eft_pay(order_id, 251).await.unwrap_err()),
        "eft_amount_invalid"
    );
    assert_eq!(
        coded(&e.eft_pay(order_id, 0).await.unwrap_err()),
        "eft_amount_invalid"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_abort_in_flight_ends_the_payment_cancelled() {
    let rig = open_with_terminal().await;
    let e = &rig.engine;
    let order_id = order(e).await;

    rig.mock.next(next(Behaviour::Approve, 20_000));
    let caller = e.clone();
    let pay = tokio::spawn(async move { caller.eft_pay(order_id, 250).await });
    // The payment is stored before the terminal is called: wait for it to be in flight.
    tokio::time::sleep(Duration::from_millis(1500)).await;
    e.eft_abort().await.expect("eft_abort");
    let payment = tokio::time::timeout(Duration::from_secs(15), pay)
        .await
        .expect("the payment ends after the abort")
        .unwrap()
        .expect("eft_pay");
    assert_eq!(payment.status, "cancelled", "{payment:?}");
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_dropped_caller_does_not_abort_the_payment() {
    let rig = open_with_terminal().await;
    let e = &rig.engine;
    let order_id = order(e).await;

    rig.mock.next(next(Behaviour::Approve, 1_500));
    let caller = e.clone();
    let pay = tokio::spawn(async move { caller.eft_pay(order_id, 250).await });
    tokio::time::sleep(Duration::from_millis(500)).await;
    // The Kotlin coroutine is cancelled: its future is dropped.
    pay.abort();
    assert!(pay.await.unwrap_err().is_cancelled());

    // The engine still stores the terminal's answer: the payment shows up approved.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let listed = e.list_eft_payments(order_id).await.unwrap();
        if let Some(p) = listed.first()
            && p.status == "approved"
        {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "never approved: {listed:?}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unknown_answer_blocks_until_it_is_checked() {
    let rig = open_with_terminal().await;
    let e = &rig.engine;
    let order_id = order(e).await;

    rig.mock.next(next(Behaviour::Timeout, 0));
    let payment = e.eft_pay(order_id, 250).await.expect("eft_pay");
    assert_eq!(payment.status, "unknown", "{payment:?}");
    // Listed for the checkout screen, and nothing else can be asked meanwhile.
    assert_eq!(e.list_eft_payments(order_id).await.unwrap().len(), 1);
    assert_eq!(
        coded(&e.eft_pay(order_id, 250).await.unwrap_err()),
        "eft_pending"
    );
    // The terminal says what happened: the lost answer was an approval.
    let checked = e.eft_check(payment.id).await.expect("eft_check");
    assert_eq!(checked.status, "approved", "{checked:?}");
    assert_eq!(checked.id, payment.id);
    e.shutdown().await.unwrap();
}
