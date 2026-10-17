//! Tests of the allergens facade (engine module allergens, v1.2): `list_allergens`,
//! `get_allergen_sheet`, `print_allergen_sheet`, `set_allergens`.
//!
//! (a) Contract: the FFI records are the engine's field by field; an unknown key is a contract
//! error; the menu records carry the allergen fields with defaults.
//!
//! (b) Wiring on the seeded engine: the legend, the sheet, the manager's edit and its refusals,
//! the print on the printer mock.

use pos_engine::commands::allergens as engine;
use pos_ffi::records::allergens::{
    AllergenEntry, AllergenNames, AllergenSheetView, SetAllergensInput,
};
use pos_ffi::records::common::{SlipOutcome, contract_inp, contract_out};
use pos_ffi::records::orders::{MenuItem, Modifier};
use pos_ffi::{PosEngine, PosError};
use serde_json::json;

use crate::support::{MANAGER_INSZ, MANAGER_PIN};

/// Dry Martini of the seed (category 10, a drink).
const MARTINI: i64 = 10006;

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn edit(contains: &[&str], may: &[&str]) -> SetAllergensInput {
    SetAllergensInput {
        entity_type: "menu_item".into(),
        entity_id: MARTINI,
        declared: true,
        contains: contains.iter().map(|s| (*s).to_owned()).collect(),
        may_contain: may.iter().map(|s| (*s).to_owned()).collect(),
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_records_are_the_engines_field_by_field() {
    let input: engine::SetAllergensInput = contract_inp(&edit(&["milk"], &["nuts"])).unwrap();
    assert_eq!(input.entity_type, "menu_item");
    assert_eq!(input.contains, vec!["milk"]);
    assert_eq!(input.may_contain, vec!["nuts"]);

    let entry: AllergenEntry = contract_out(&engine::AllergenEntry {
        code: "milk".into(),
        number: 7,
        names: engine::AllergenNames {
            fr: "Lait".into(),
            nl: "Melk".into(),
            en: "Milk".into(),
            zh: "奶".into(),
        },
    })
    .unwrap();
    assert_eq!(entry.number, 7);
    assert_eq!(
        entry.names,
        AllergenNames {
            fr: "Lait".into(),
            nl: "Melk".into(),
            en: "Milk".into(),
            zh: "奶".into()
        }
    );

    let sheet: AllergenSheetView = contract_out(&engine::AllergenSheetView {
        lang: "fr".into(),
        products: vec![engine::SheetProductView {
            id: 1,
            name: "Maki".into(),
            name_fr: "Maki".into(),
            declared: false,
            contains: vec![],
            traces: vec![engine::AllergenRef {
                code: "fish".into(),
                number: 4,
                name: "Poissons".into(),
            }],
            options: vec![engine::SheetOptionView {
                group: "Sauce".into(),
                name: "Soja".into(),
                declared: true,
                contains: vec![],
                traces: vec![],
            }],
        }],
    })
    .unwrap();
    assert_eq!(sheet.products[0].traces[0].number, 4);
    assert!(sheet.products[0].options[0].declared);
}

#[test]
fn an_unknown_key_is_a_contract_error() {
    let mut value = serde_json::to_value(edit(&[], &[])).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<SetAllergensInput>(value).is_err());
}

#[test]
fn the_menu_records_default_the_allergen_fields() {
    let item: MenuItem = serde_json::from_value(json!({
        "id": 1, "category_id": 1, "name": "x", "price_cents": 100, "vat_code": "B",
        "kitchen_printer": true, "sort_order": 0, "active": true
    }))
    .unwrap();
    assert!(item.allergens.is_empty() && item.traces.is_empty() && !item.allergens_declared);
    let option: Modifier = serde_json::from_value(json!({
        "id": 1, "name": "x", "price_cents": 0
    }))
    .unwrap();
    assert!(option.allergens.is_empty() && !option.allergens_declared);
}

// ───────────────────────────── (b) wiring ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(coded(&e.list_allergens().await.unwrap_err()).0, "not_logged_in");
    assert_eq!(
        coded(&e.get_allergen_sheet(None, None).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.print_allergen_sheet(None, None).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_allergens(edit(&[], &[])).await.unwrap_err()).0,
        "not_logged_in"
    );
    // A cashier reads but does not edit.
    e.login("75061189702".into(), Some("4444".into())).await.unwrap();
    assert_eq!(e.list_allergens().await.unwrap().len(), 14);
    assert_eq!(
        coded(&e.set_allergens(edit(&[], &[])).await.unwrap_err()).0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

async fn manager(e: &PosEngine) {
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_legend_lists_the_14_in_annex_order() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;
    let legend = e.list_allergens().await.unwrap();
    assert_eq!(legend.len(), 14);
    assert_eq!(legend[0].code, "gluten");
    assert_eq!(legend[0].number, 1);
    assert_eq!(legend[13].code, "molluscs");
    assert_eq!(legend[13].number, 14);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manager_edit_shows_on_the_sheet_and_unknown_codes_are_refused() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;

    let before = e.get_allergen_sheet(Some(MARTINI), None).await.unwrap();
    assert_eq!(before.products.len(), 1);
    assert!(!before.products[0].declared, "no information yet");

    assert!(e.set_allergens(edit(&["sulphites"], &["nuts"])).await.unwrap());
    assert!(
        !e.set_allergens(edit(&["sulphites"], &["nuts"])).await.unwrap(),
        "the same list changes nothing"
    );
    let after = e
        .get_allergen_sheet(Some(MARTINI), Some("en".into()))
        .await
        .unwrap();
    assert_eq!(after.lang, "en");
    let p = &after.products[0];
    assert!(p.declared);
    assert_eq!(p.contains[0].code, "sulphites");
    assert_eq!(p.contains[0].number, 12);
    assert_eq!(p.contains[0].name, "Sulphites");
    assert_eq!(p.traces[0].name, "Nuts");

    let err = e.set_allergens(edit(&["glutten"], &[])).await.unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "allergen_unknown");
    assert_eq!(params["code"], "glutten");

    let mut missing = edit(&[], &[]);
    missing.entity_id = 987_654;
    assert_eq!(
        coded(&e.set_allergens(missing).await.unwrap_err()).0,
        "allergen_target_not_found"
    );
    assert_eq!(
        coded(&e.get_allergen_sheet(Some(987_654), None).await.unwrap_err()).0,
        "allergen_target_not_found"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_sheet_prints_on_the_printer() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;
    e.set_allergens(edit(&["fish"], &[])).await.unwrap();
    assert_eq!(
        e.print_allergen_sheet(Some(MARTINI), None).await.unwrap(),
        SlipOutcome::Printed
    );
    assert_eq!(
        e.print_allergen_sheet(None, Some("nl".into())).await.unwrap(),
        SlipOutcome::Printed
    );
    e.shutdown().await.unwrap();
}
