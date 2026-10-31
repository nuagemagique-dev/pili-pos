//! Tests of the scale facade (`api::scale`, `records::scale`): the contract of every record, the
//! weighing fields of the shared records, the event, and one wiring test per method. The weighing
//! itself is `crates/pos-engine/tests/scale.rs`.

use pos_engine::commands::scale::{
    ScaleProgress as EngineProgress, ScaleStatus as EngineStatus, ScaleWeighInput as EngineInput,
};
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::records::orders;
use pos_ffi::records::scale as ffi;
use serde_json::to_value;

use crate::support::{MANAGER_INSZ, MANAGER_PIN, open_seeded};

#[test]
fn the_status_and_the_progress_keep_their_wire_form() {
    let engine = EngineStatus {
        configured: true,
        connected: true,
        busy: false,
        check: "ok".into(),
    };
    let record: ffi::ScaleStatus = contract_out(&engine).unwrap();
    assert_eq!(to_value(&record).unwrap(), to_value(&engine).unwrap());

    let engine = EngineProgress {
        stage: "stabilizing".into(),
    };
    let record: ffi::ScaleProgress = contract_out(&engine).unwrap();
    assert_eq!(to_value(&record).unwrap(), to_value(&engine).unwrap());
}

#[test]
fn the_weighing_input_reaches_the_engine() {
    let record = ffi::ScaleWeighInput {
        order_id: 7,
        menu_item_id: 12,
    };
    let engine: EngineInput = contract_inp(&record).unwrap();
    assert_eq!((engine.order_id, engine.menu_item_id), (7, 12));
}

#[test]
fn a_weighed_line_keeps_its_weight_and_its_amount() {
    let engine = pos_core::order::OrderLine {
        id: 1,
        order_id: 2,
        menu_item_id: 3,
        quantity: 1,
        unit_price_cents: 4500,
        vat_code: pos_core::vat::VatCode::B,
        name: "Saumon".into(),
        modifiers_json: None,
        created_at: chrono::Utc::now(),
        department_id: "1".into(),
        department_name: "Sushi".into(),
        weight_grams: Some(278),
        weighed_amount_cents: Some(1251),
    };
    let record: orders::OrderLine = contract_out(&engine).unwrap();
    assert_eq!(record.weight_grams, Some(278));
    assert_eq!(record.weighed_amount_cents, Some(1251));
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_section_nothing_is_weighed() {
    let t = open_seeded().await;
    let engine = &t.engine;
    engine
        .login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    let status = engine.get_scale_status().await.unwrap();
    assert!(!status.configured);
    assert_eq!(status.check, "unknown");

    let refused = engine
        .scale_start_weighing(ffi::ScaleWeighInput {
            order_id: 1,
            menu_item_id: 1,
        })
        .await
        .unwrap_err();
    assert!(
        format!("{refused:?}").contains("scale_not_configured"),
        "{refused:?}"
    );
    engine.scale_cancel_weighing().await.unwrap();
    engine.shutdown().await.unwrap();
}
