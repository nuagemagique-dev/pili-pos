//! Tests of the nightly export facade (`export_status`, `export_run_now`; decision X1D10).
//!
//! (a) Contract: the FFI record is the engine's `ExportStatus` field by field, with every field set
//! and with every option `None`.
//!
//! (b) Wiring: a `PosEngine` opened on a temp data dir whose bundle `pos.json` has an `export`
//! section pointing to the in-process receiver `pos_export_mock::Receiver`; a sale and a Z through
//! the FFI against the in-process FDM mock, then `export_status` until the upload is done. Without
//! an `export` section the status is `enabled: false` with no problem; a session that is not an
//! administrator gets the `forbidden` code.

use std::path::Path;
use std::time::{Duration, Instant};

use pos_engine::commands::export::ExportStatus as EngineExportStatus;
use pos_export_mock::Receiver;
use pos_ffi::records::common::{PaymentMethod, PaymentPart, contract_out};
use pos_ffi::records::export::ExportStatus;
use pos_ffi::records::orders::{AddOrderLineInput, CreateOrderInput, OrderType};
use pos_ffi::{EngineOptions, PosEngine, PosError};
use serde_json::to_value;

use crate::support::{MANAGER_INSZ, MANAGER_PIN, options_for};

const TOKEN: &str = "tok-ffi-export-test";
/// Seeded cashier "John" (not an administrator) and his PIN.
const CASHIER_INSZ: &str = "75061189702";
const CASHIER_PIN: &str = "4444";
/// Seeded product: Perrier, 2,50 EUR.
const PERRIER: i64 = 12007;

// ───────────────────────────── (a) contract ─────────────────────────────

fn engine_status(with_options: bool) -> EngineExportStatus {
    let text = |s: &str| with_options.then(|| s.to_string());
    EngineExportStatus {
        enabled: with_options,
        problem: text("export.url: an https:// URL expected"),
        endpoint: text("https://export.example.test:8443"),
        keep_days: 14,
        last_export_id: text("20261008-z000042"),
        last_prepared_at: text("2026-10-08T22:01:02+02:00"),
        last_uploaded_id: text("20261007-z000041"),
        last_uploaded_at: text("2026-10-07T22:05:00+02:00"),
        pending: 2,
        next_attempt_at: text("2026-10-08T22:10:00+02:00"),
        last_error: text("upload: HTTP 500"),
        last_error_at: text("2026-10-08T22:02:00+02:00"),
        kept: 9,
        dropped: 1,
    }
}

#[test]
fn a_status_with_every_field_set_converts_field_by_field() {
    let engine = engine_status(true);
    let record: ExportStatus = contract_out(&engine).expect("convert::out");
    assert_eq!(
        record,
        ExportStatus {
            enabled: true,
            problem: Some("export.url: an https:// URL expected".into()),
            endpoint: Some("https://export.example.test:8443".into()),
            keep_days: 14,
            last_export_id: Some("20261008-z000042".into()),
            last_prepared_at: Some("2026-10-08T22:01:02+02:00".into()),
            last_uploaded_id: Some("20261007-z000041".into()),
            last_uploaded_at: Some("2026-10-07T22:05:00+02:00".into()),
            pending: 2,
            next_attempt_at: Some("2026-10-08T22:10:00+02:00".into()),
            last_error: Some("upload: HTTP 500".into()),
            last_error_at: Some("2026-10-08T22:02:00+02:00".into()),
            kept: 9,
            dropped: 1,
        }
    );
    assert_eq!(to_value(&engine).unwrap(), to_value(&record).unwrap());
}

#[test]
fn a_status_with_every_option_none_converts() {
    let engine = engine_status(false);
    assert_eq!(engine.problem, None);
    assert_eq!(engine.last_uploaded_id, None);
    let record: ExportStatus = contract_out(&engine).expect("convert::out");
    assert!(!record.enabled);
    assert_eq!(record.problem, None);
    assert_eq!(record.endpoint, None);
    assert_eq!(record.last_export_id, None);
    assert_eq!(record.last_prepared_at, None);
    assert_eq!(record.last_uploaded_id, None);
    assert_eq!(record.last_uploaded_at, None);
    assert_eq!(record.next_attempt_at, None);
    assert_eq!(record.last_error, None);
    assert_eq!(record.last_error_at, None);
    assert_eq!(record.keep_days, 14);
    assert_eq!(record.pending, 2);
    assert_eq!(to_value(&engine).unwrap(), to_value(&record).unwrap());
}

#[test]
fn a_status_with_an_unknown_key_is_a_contract_error() {
    let mut value = to_value(engine_status(true)).unwrap();
    value["somethingNew"] = serde_json::json!(1);
    let result: Result<ExportStatus, PosError> = contract_out(&value);
    assert!(
        matches!(result, Err(PosError::Internal { .. })),
        "{result:?}"
    );
}

// ───────────────────────────── (b) wiring ─────────────────────────────

/// An engine on the seeded database with its own FDM mock, with or without an `export` section.
struct Rig {
    engine: std::sync::Arc<PosEngine>,
    receiver: Receiver,
    _dir: tempfile::TempDir,
}

async fn open(with_export: bool) -> Rig {
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

    let receiver = Receiver::start(TOKEN);
    let dir = tempfile::tempdir().expect("temp dir");
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    let token_file = dir.path().join("export.token");
    std::fs::write(&token_file, TOKEN).unwrap();

    let (_owner_secret, owner) = pos_export::crypt::generate_identity();
    let (_producer_secret, producer) = pos_export::crypt::generate_identity();
    let export = if with_export {
        format!(
            r#", "export": {{
                "url": "{}", "token_file": "{}",
                "owner_recipient": "{owner}", "producer_recipient": "{producer}",
                "keep_days": 14 }}"#,
            receiver.url(),
            token_file.display()
        )
    } else {
        String::new()
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
            "table_count": 25
            {export} }}"#
        ),
    )
    .unwrap();

    let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
        .expect("seed the database");
    let options: EngineOptions = options_for(dir.path());
    let engine = PosEngine::open(options).unwrap_or_else(|e| panic!("open: {e}"));
    Rig {
        engine,
        receiver,
        _dir: dir,
    }
}

fn coded(err: &PosError, expected: &str) -> bool {
    matches!(err, PosError::Coded { code, .. } if code == expected)
}

/// One cash sale of a Perrier by the logged-in manager.
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_sale_and_a_z_through_the_ffi_reach_the_receiver() {
    let rig = open(true).await;
    let e = &rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");

    let before = e.export_status().await.expect("export_status");
    assert!(before.enabled, "{before:?}");
    assert_eq!(before.problem, None);
    assert_eq!(before.pending, 0);
    assert_eq!(before.last_uploaded_id, None);
    assert_eq!(before.endpoint, Some(rig.receiver.url()));
    assert_eq!(before.keep_days, 14);

    sell(e).await;
    let z = e
        .generate_z_report(MANAGER_INSZ.into())
        .await
        .expect("generate_z_report");
    assert!(z.success, "Z: {:?}", z.error);

    let started = Instant::now();
    let status = loop {
        let status = e.export_status().await.expect("export_status");
        if status.pending == 0 && status.last_uploaded_id.is_some() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "timed out waiting for the upload; last status: {status:?}"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    };
    let id = status.last_uploaded_id.clone().unwrap();
    assert!(id.ends_with("-z000001"), "the first Z: {id}");
    assert_eq!(status.last_error, None);
    assert!(status.kept >= 1);

    for name in [
        pos_export::naming::backup_name(&id),
        pos_export::naming::stats_name(&id),
        pos_export::naming::manifest_name(&id),
    ] {
        let body = rig
            .receiver
            .file(&format!("{id}/{name}"))
            .unwrap_or_else(|| panic!("the receiver has no {name}"));
        assert!(!body.is_empty(), "{name} is empty");
    }

    // Nothing is pending: run-now only answers the status.
    let again = e.export_run_now().await.expect("export_run_now");
    assert_eq!(again.last_uploaded_id, Some(id));
    assert!(again.enabled);

    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_export_section_the_status_is_disabled_and_run_now_is_harmless() {
    let rig = open(false).await;
    let e = &rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");

    let status = e.export_status().await.expect("export_status");
    assert!(!status.enabled);
    assert_eq!(status.problem, None);
    assert_eq!(status.endpoint, None);
    assert_eq!(status.pending, 0);
    assert_eq!(status.last_uploaded_id, None);

    let run = e.export_run_now().await.expect("export_run_now");
    assert_eq!(run, status);
    assert!(
        !rig._dir.path().join("data").join("exports").exists(),
        "nothing is created on disk without a section"
    );
    assert_eq!(rig.receiver.put_count(), 0);

    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_that_is_not_an_administrator_is_refused() {
    let rig = open(true).await;
    let e = &rig.engine;

    // Nobody is signed in.
    let err = e.export_status().await.unwrap_err();
    assert!(coded(&err, "not_logged_in"), "{err:?}");

    e.login(CASHIER_INSZ.into(), Some(CASHIER_PIN.into()))
        .await
        .expect("cashier login");
    let err = e.export_status().await.unwrap_err();
    assert!(coded(&err, "forbidden"), "{err:?}");
    let err = e.export_run_now().await.unwrap_err();
    assert!(coded(&err, "forbidden"), "{err:?}");
    assert_eq!(rig.receiver.put_count(), 0);

    e.shutdown().await.unwrap();
}
