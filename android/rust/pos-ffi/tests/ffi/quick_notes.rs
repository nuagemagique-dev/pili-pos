//! Tests of the quick notes facade (engine module `quick_notes`): `get_quick_notes`,
//! `set_quick_notes`.
//!
//! (a) Contract: the FFI record is the engine's field by field; an unknown key is a contract error.
//!
//! (b) Wiring on the seeded engine: the nine usual notes of a new installation, the round trip, the
//! refusals (role, length, count) and that a chip is only a text for the note of a line.

use pos_engine::commands::quick_notes as engine;
use pos_ffi::PosError;
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::records::quick_notes::QuickNotes;
use serde_json::json;

use crate::support::{MANAGER_INSZ, MANAGER_PIN};

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn notes(list: &[&str]) -> QuickNotes {
    QuickNotes {
        notes: list.iter().map(|n| (*n).to_string()).collect(),
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_record_is_the_engines_field_by_field() {
    let input: engine::QuickNotes = contract_inp(&notes(&["Bien cuit", "Épicé"])).unwrap();
    assert_eq!(input.notes, ["Bien cuit", "Épicé"]);
    let back: QuickNotes = contract_out(&input).unwrap();
    assert_eq!(back, notes(&["Bien cuit", "Épicé"]));
    let defaults: QuickNotes = contract_out(&engine::QuickNotes::default()).unwrap();
    assert_eq!(defaults.notes.len(), 9);
}

#[test]
fn an_unknown_key_is_a_contract_error() {
    let mut value = serde_json::to_value(notes(&["x"])).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<QuickNotes>(value).is_err());
}

// ───────────────────────────── (b) wiring ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse_and_a_cashier_does_not_set() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.get_quick_notes().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_quick_notes(notes(&["x"])).await.unwrap_err()).0,
        "not_logged_in"
    );
    e.login("75061189702".into(), Some("4444".into()))
        .await
        .unwrap();
    assert_eq!(e.get_quick_notes().await.unwrap().notes.len(), 9);
    assert_eq!(
        coded(&e.set_quick_notes(notes(&["x"])).await.unwrap_err()).0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_list_round_trips_and_its_refusals_are_coded() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert_eq!(e.get_quick_notes().await.unwrap().notes[0], "Sans wasabi");

    let stored = e
        .set_quick_notes(notes(&["  Bien cuit ", "", "Épicé"]))
        .await
        .unwrap();
    assert_eq!(stored, notes(&["Bien cuit", "Épicé"]));
    assert_eq!(e.get_quick_notes().await.unwrap(), stored);

    let err = e
        .set_quick_notes(notes(&[&"x".repeat(61)]))
        .await
        .unwrap_err();
    assert_eq!(coded(&err).0, "note_too_long");
    let err = e.set_quick_notes(notes(&["不要"])).await.unwrap_err();
    assert_eq!(coded(&err).0, "note_unprintable");
    let many: Vec<String> = (0..25).map(|i| format!("Note {i}")).collect();
    let err = e
        .set_quick_notes(QuickNotes { notes: many })
        .await
        .unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "quick_notes_too_many");
    assert_eq!(params["max"], "24");
    // The refusals left the list alone.
    assert_eq!(e.get_quick_notes().await.unwrap(), stored);
    e.shutdown().await.unwrap();
}
