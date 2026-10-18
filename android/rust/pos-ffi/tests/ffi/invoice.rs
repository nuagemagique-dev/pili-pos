//! Tests of the B2B invoice facade (V11-D1).
//!
//! (a) Contract: every FFI record is the engine's type field by field, with every option set
//! and with every option `None`.
//!
//! (b) Wiring: a `PosEngine` on the seeded database with the in-process FDM mock and a `peppol`
//! section on the `logging` provider: a sale, a customer, the preview, the signed invoice, the
//! list, the Peppol panel until the delivery is accepted; a cashier cannot open the panel.

use std::time::{Duration, Instant};

use pos_engine::commands::invoice as inv;
use pos_ffi::records::common::{PaymentMethod, PaymentPart, contract_out};
use pos_ffi::records::invoice as rec;
use pos_ffi::records::orders::{AddOrderLineInput, CreateOrderInput, OrderType};
use pos_ffi::{EngineOptions, PosEngine, PosError};
use serde_json::to_value;

use crate::support::{MANAGER_INSZ, MANAGER_PIN, options_for};

const CASHIER_INSZ: &str = "75061189702";
const CASHIER_PIN: &str = "4444";
/// Seeded product: Perrier, 2,50 EUR.
const PERRIER: i64 = 12007;

// ───────────────────────────── (a) contract ─────────────────────────────

fn customer(with_options: bool) -> inv::Customer {
    inv::Customer {
        id: 7,
        vat_number: "BE0308357159".into(),
        name: "Client Test SRL".into(),
        street: "Rue du Test 1".into(),
        postal_code: "4000".into(),
        city: "Liege".into(),
        country_code: "BE".into(),
        peppol_status: "registered".into(),
        peppol_checked_at: with_options.then(|| "2026-10-17T10:00:00Z".to_string()),
    }
}

fn delivery(with_options: bool) -> inv::Delivery {
    let text = |s: &str| with_options.then(|| s.to_string());
    inv::Delivery {
        id: 3,
        fiscal_event_id: with_options.then_some(12),
        invoice_no: "20260001".into(),
        customer_endpoint_id: "0208:0308357159".into(),
        ap_vendor: "storecove".into(),
        ap_message_id: text("msg-1"),
        status: "failed".into(),
        error_msg: text("recipient is not registered on Peppol"),
        attempts: 2,
        next_attempt_at: text("2026-10-17T10:05:00Z"),
        last_attempt_at: text("2026-10-17T10:00:00Z"),
        parked_at: text("2026-10-17T10:00:00Z"),
        accepted_at: text("2026-10-17T10:00:01Z"),
        created_at: "2026-10-17T09:59:00Z".into(),
        lookup_status: "unknown".into(),
    }
}

fn same<E: serde::Serialize, F: serde::Serialize>(engine: &E, record: &F) {
    assert_eq!(to_value(engine).unwrap(), to_value(record).unwrap());
}

#[test]
fn a_customer_converts_with_and_without_options() {
    for with in [true, false] {
        let engine = customer(with);
        let record: rec::Customer = contract_out(&engine).expect("convert::out");
        assert_eq!(record.id, 7);
        assert_eq!(record.vat_number, "BE0308357159");
        assert_eq!(record.peppol_checked_at.is_some(), with);
        same(&engine, &record);
    }
}

#[test]
fn a_delivery_converts_with_and_without_options() {
    for with in [true, false] {
        let engine = delivery(with);
        let record: rec::Delivery = contract_out(&engine).expect("convert::out");
        assert_eq!(record.attempts, 2);
        assert_eq!(record.fiscal_event_id.is_some(), with);
        assert_eq!(record.ap_message_id.is_some(), with);
        same(&engine, &record);
    }
}

#[test]
fn the_panel_a_lookup_and_a_summary_convert() {
    let panel = inv::PeppolDeliveries {
        configured: true,
        provider: Some("storecove".into()),
        sender_id: Some("0208:0499999960".into()),
        queued: 1,
        accepted: 2,
        failed: 3,
        deliveries: vec![delivery(true), delivery(false)],
    };
    let record: rec::PeppolDeliveries = contract_out(&panel).expect("convert::out");
    assert_eq!(record.deliveries.len(), 2);
    assert_eq!((record.queued, record.accepted, record.failed), (1, 2, 3));
    same(&panel, &record);

    let lookup = inv::CustomerLookup {
        vat_number: "BE0308357159".into(),
        country_code: "BE".into(),
        customer: Some(customer(true)),
        peppol_status: "unknown".into(),
        peppol_checked: false,
        deliverable: true,
    };
    let record: rec::CustomerLookup = contract_out(&lookup).expect("convert::out");
    assert!(record.customer.is_some() && record.deliverable);
    same(&lookup, &record);

    let summary = inv::InvoiceSummary {
        id: 1,
        invoice_no: "20260001".into(),
        fiscal_event_id: 9,
        issue_date: "2026-10-17".into(),
        customer_name: "Client Test SRL".into(),
        customer_vat_number: "BE0308357159".into(),
        total_cents: 1210,
        sale_event_ids: vec![3, 4],
        delivery_status: Some("queued".into()),
        delivery_id: Some(5),
    };
    let record: rec::InvoiceSummary = contract_out(&summary).expect("convert::out");
    assert_eq!(record.sale_event_ids, vec![3, 4]);
    same(&summary, &record);
}

#[test]
fn a_preview_a_sale_and_a_result_convert() {
    let sale = inv::InvoiceableSale {
        fiscal_event_id: 3,
        ticket_number: 12,
        pos_date_time: "20261017103000".into(),
        total_cents: -250,
        fdm_id: "FDM0001".into(),
        event_counter: 4,
        total_counter: 9,
        is_refund: true,
        invoiced_by: Some("20260001".into()),
    };
    let record: rec::InvoiceableSale = contract_out(&sale).expect("convert::out");
    assert!(record.is_refund && record.total_cents == -250);
    same(&sale, &record);

    let preview = inv::InvoicePreview {
        sales: vec![sale],
        lines: vec![inv::PreviewLine {
            description: "Perrier".into(),
            quantity_milli: 1000,
            total_cents: 250,
            vat_label: "A".into(),
        }],
        vat: vec![inv::PreviewVat {
            label: "A".into(),
            rate_hundredths: 2100,
            base_cents: 207,
            vat_cents: 43,
            total_cents: 250,
        }],
        total_cents: 250,
        problems: vec!["invoice_no_sales".into()],
    };
    let record: rec::InvoicePreview = contract_out(&preview).expect("convert::out");
    assert_eq!(record.vat[0].rate_hundredths, 2100);
    same(&preview, &record);

    for with in [true, false] {
        let result = inv::InvoiceResult {
            success: with,
            fiscal_event_id: with.then_some(5),
            invoice_no: with.then(|| "20260001".to_string()),
            printed: with,
            delivery_status: with.then(|| "queued".to_string()),
            error: (!with).then(|| "fdm_not_connected".to_string()),
        };
        let record: rec::InvoiceResult = contract_out(&result).expect("convert::out");
        same(&result, &record);
    }
}

#[test]
fn the_customer_form_converts_both_ways() {
    let form = rec::B2bCustomerInput {
        id: None,
        vat_number: "BE0308357159".into(),
        name: "Client".into(),
        street: "Rue 1".into(),
        postal_code: "4000".into(),
        city: "Liege".into(),
        country_code: "BE".into(),
    };
    let engine: inv::B2bCustomerInput =
        serde_json::from_value(to_value(&form).unwrap()).expect("engine form");
    assert_eq!(engine.vat_number, "BE0308357159");
    assert_eq!(engine.id, None);
    same(&engine, &form);
}

#[test]
fn an_unknown_key_is_a_contract_error() {
    let mut value = to_value(customer(true)).unwrap();
    value["somethingNew"] = serde_json::json!(1);
    let result: Result<rec::Customer, PosError> = contract_out(&value);
    assert!(
        matches!(result, Err(PosError::Internal { .. })),
        "{result:?}"
    );
}

// ───────────────────────────── (b) wiring ─────────────────────────────

struct Rig {
    engine: std::sync::Arc<PosEngine>,
    _dir: tempfile::TempDir,
}

async fn open() -> Rig {
    open_with(None).await
}

/// The same, with the Storecove mock behind the `storecove` provider when `storecove` is given
/// (the `logging` provider accepts everything, so it cannot park a delivery).
async fn open_with(storecove: Option<pos_fdm_mock::storecove::StorecoveMock>) -> Rig {
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind the FDM mock");
    let addr = fdm.local_addr().unwrap();
    let fdm_url = format!("http://{addr}/graphql");
    let router = match &storecove {
        Some(mock) => {
            pos_fdm_mock::app_with_storecove(pos_fdm_mock::Control::default(), mock.clone())
        }
        None => pos_fdm_mock::app(),
    };
    tokio::spawn(async move {
        axum::serve(fdm, router).await.expect("serve the FDM mock");
    });
    let printer_mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
    let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
        .await
        .expect("bind the printer mock")
        .to_string();

    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    let key_file = dir.path().join("storecove.key");
    std::fs::write(&key_file, "test-key\n").unwrap();
    let peppol = if storecove.is_some() {
        format!(
            r#""provider": "storecove",
                "base_url": "http://{addr}/storecove",
                "api_key_file": "{}",
                "legal_entity_id": 4242,"#,
            key_file.display()
        )
    } else {
        r#""provider": "logging","#.to_string()
    };
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
            "peppol": {{
                {peppol}
                "sender_id": "0208:0499999960",
                "supplier_street": "Rue de l'Exemple 1",
                "supplier_postal_code": "4000",
                "supplier_city": "Liege"
            }} }}"#
        ),
    )
    .unwrap();

    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
        .expect("seed the database");
    let options: EngineOptions = options_for(dir.path());
    let engine = PosEngine::open(options).unwrap_or_else(|e| panic!("open: {e}"));
    Rig { engine, _dir: dir }
}

fn coded(err: &PosError, expected: &str) -> bool {
    matches!(err, PosError::Coded { code, .. } if code == expected)
}

async fn sell(engine: &PosEngine) {
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
    let sale = engine
        .checkout(
            order.id,
            vec![PaymentPart {
                method: PaymentMethod::Cash,
                amount_cents: None,
                eft_payment_id: None,
            }],
            None,
        )
        .await
        .expect("checkout");
    assert!(sale.success, "checkout: {:?}", sale.error);
}

fn form() -> rec::B2bCustomerInput {
    rec::B2bCustomerInput {
        id: None,
        vat_number: "BE 0308.357.159".into(),
        name: "Client Test SRL".into(),
        street: "Rue du Test 1".into(),
        postal_code: "4000".into(),
        city: "Liege".into(),
        country_code: String::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_sale_becomes_an_invoice_delivered_over_the_logging_provider() {
    let rig = open().await;
    let e = &rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");

    sell(e).await;
    let sales = e.list_invoiceable_sales(None, None).await.expect("sales");
    assert_eq!(sales.len(), 1, "{sales:?}");
    assert_eq!(sales[0].total_cents, 250);
    assert_eq!(sales[0].invoiced_by, None);

    let customer = e.save_b2b_customer(form()).await.expect("save customer");
    assert_eq!(customer.vat_number, "BE0308357159");
    assert_eq!(customer.country_code, "BE");
    let found = e
        .search_b2b_customers("BE0308".into())
        .await
        .expect("search");
    assert_eq!(found.len(), 1);

    let ids = vec![sales[0].fiscal_event_id];
    let preview = e.invoice_preview(ids.clone()).await.expect("preview");
    assert!(preview.problems.is_empty(), "{:?}", preview.problems);
    assert_eq!(preview.total_cents, 250);
    assert_eq!(preview.lines.len(), 1);

    let result = e
        .sign_invoice_fiscal(ids.clone(), customer.id, MANAGER_INSZ.into())
        .await
        .expect("sign_invoice_fiscal");
    assert!(result.success, "{:?}", result.error);
    let no = result.invoice_no.clone().expect("invoice number");
    assert!(no.ends_with("0001"), "{no}");
    assert!(result.printed);
    assert_eq!(result.delivery_status.as_deref(), Some("queued"));

    // The sale is invoiced now: a second invoice is refused.
    let again = e.invoice_preview(ids).await.expect("preview");
    assert!(
        again
            .problems
            .iter()
            .any(|p| p.starts_with("invoice_sale_already_invoiced")),
        "{:?}",
        again.problems
    );

    let invoices = e.list_invoices(None).await.expect("list_invoices");
    assert_eq!(invoices.len(), 1);
    assert_eq!(invoices[0].invoice_no, no);
    assert_eq!(invoices[0].customer_name, "Client Test SRL");

    let started = Instant::now();
    let panel = loop {
        let panel = e.list_peppol_deliveries(None).await.expect("panel");
        if panel.accepted == 1 {
            break panel;
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "the delivery was not accepted: {panel:?}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert!(panel.configured);
    assert_eq!(panel.provider.as_deref(), Some("logging"));
    assert_eq!(panel.deliveries[0].status, "accepted");
    assert_eq!(panel.deliveries[0].customer_endpoint_id, "0208:0308357159");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_peppol_panel_is_for_administrators() {
    let rig = open().await;
    let e = &rig.engine;
    e.login(CASHIER_INSZ.into(), Some(CASHIER_PIN.into()))
        .await
        .expect("login");
    let err = e.list_peppol_deliveries(None).await.unwrap_err();
    assert!(coded(&err, "forbidden"), "{err:?}");
    let err = e.retry_peppol_delivery(1).await.unwrap_err();
    assert!(coded(&err, "forbidden"), "{err:?}");
    // A cashier may save a customer.
    e.save_b2b_customer(form()).await.expect("cashier saves");
    let bad = rec::B2bCustomerInput {
        vat_number: "BE0308357158".into(),
        ..form()
    };
    let err = e.save_b2b_customer(bad).await.unwrap_err();
    assert!(coded(&err, "customer_vat_invalid"), "{err:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cashier_looks_a_vat_number_up_through_the_facade() {
    let rig = open().await;
    let e = &rig.engine;
    e.login(CASHIER_INSZ.into(), Some(CASHIER_PIN.into()))
        .await
        .expect("login");

    // Written the way it is typed: the engine normalises it.
    let lookup = e
        .lookup_b2b_customer("be 0308.357.159".into())
        .await
        .expect("lookup");
    assert_eq!(lookup.vat_number, "BE0308357159");
    assert_eq!(lookup.country_code, "BE");
    assert!(lookup.deliverable);
    assert!(lookup.customer.is_none());
    assert!(
        ["registered", "unknown"].contains(&lookup.peppol_status.as_str()),
        "{lookup:?}"
    );

    // A known customer comes back with the lookup.
    let saved = e.save_b2b_customer(form()).await.expect("save");
    let known = e
        .lookup_b2b_customer("BE0308357159".into())
        .await
        .expect("lookup");
    assert_eq!(known.customer.map(|c| c.id), Some(saved.id));

    // A foreign number is checked offline and is not delivered over Peppol.
    let foreign = e
        .lookup_b2b_customer("FR12345678901".into())
        .await
        .expect("lookup");
    assert_eq!(foreign.country_code, "FR");
    assert!(!foreign.deliverable);

    // A wrong check digit is refused with its code.
    let err = e
        .lookup_b2b_customer("BE0308357158".into())
        .await
        .unwrap_err();
    assert!(coded(&err, "customer_vat_invalid"), "{err:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_parked_delivery_is_retried_by_a_manager_and_only_a_parked_one() {
    let mock = pos_fdm_mock::storecove::StorecoveMock::new();
    // The access point refuses the first submission (parked), then fails transiently, so the
    // retried delivery waits in the queue instead of racing to "accepted".
    mock.script_submissions(&[422, 503]);
    let rig = open_with(Some(mock)).await;
    let e = &rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");

    sell(e).await;
    let sale = e.list_invoiceable_sales(None, None).await.expect("sales")[0].fiscal_event_id;
    let customer = e.save_b2b_customer(form()).await.expect("customer");
    let result = e
        .sign_invoice_fiscal(vec![sale], customer.id, MANAGER_INSZ.into())
        .await
        .expect("sign");
    assert!(result.success, "{:?}", result.error);

    let wait_for = |want_failed: i64, want_queued: i64| {
        let e = rig.engine.clone();
        async move {
            let started = Instant::now();
            loop {
                let panel = e.list_peppol_deliveries(None).await.expect("panel");
                if panel.failed == want_failed && panel.queued == want_queued {
                    return panel;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(15),
                    "expected failed={want_failed} queued={want_queued}: {panel:?}"
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    };
    let parked = wait_for(1, 0).await;
    let id = parked.deliveries[0].id;
    assert_eq!(parked.deliveries[0].status, "failed");
    assert!(parked.deliveries[0].parked_at.is_some());

    e.retry_peppol_delivery(id).await.expect("retry");
    let back = wait_for(0, 1).await;
    assert_eq!(back.deliveries[0].id, id);
    assert_eq!(back.deliveries[0].status, "queued");
    assert!(back.deliveries[0].parked_at.is_none());

    // It is not parked any more.
    let err = e.retry_peppol_delivery(id).await.unwrap_err();
    assert!(coded(&err, "peppol_delivery_not_parked"), "{err:?}");
    let err = e.retry_peppol_delivery(9999).await.unwrap_err();
    assert!(coded(&err, "peppol_delivery_not_found"), "{err:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_accepted_delivery_cannot_be_retried() {
    let rig = open().await;
    let e = &rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");
    sell(e).await;
    let sale = e.list_invoiceable_sales(None, None).await.expect("sales")[0].fiscal_event_id;
    let customer = e.save_b2b_customer(form()).await.expect("customer");
    let result = e
        .sign_invoice_fiscal(vec![sale], customer.id, MANAGER_INSZ.into())
        .await
        .expect("sign");
    assert!(result.success, "{:?}", result.error);

    let started = Instant::now();
    let id = loop {
        let panel = e.list_peppol_deliveries(None).await.expect("panel");
        if panel.accepted == 1 {
            break panel.deliveries[0].id;
        }
        assert!(started.elapsed() < Duration::from_secs(15), "{panel:?}");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    let err = e.retry_peppol_delivery(id).await.unwrap_err();
    assert!(coded(&err, "peppol_delivery_not_parked"), "{err:?}");
}
