//! Tests of the favourites facade (engine module `favourites`): `get_favourites_settings`,
//! `set_favourites_settings`, `set_favourite`, `list_favourites`.
//!
//! (a) Contract: the FFI records are the engine's field by field and in the same serde form; an
//! unknown key is a contract error.
//!
//! (b) Wiring on the seeded engine: the default (most sold, nothing selected, empty tab), the
//! round trip of the selection in its order, the refusals (role, unknown product, count) and the
//! tab listing the starred products.

use pos_engine::commands::favourites as engine;
use pos_ffi::PosError;
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::records::favourites::{FavouritesMode, FavouritesSettings};
use serde_json::json;

use crate::support::{MANAGER_INSZ, MANAGER_PIN};

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn settings(mode: FavouritesMode, selection: &[i64]) -> FavouritesSettings {
    FavouritesSettings {
        mode,
        selection: selection.to_vec(),
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_records_are_the_engines_field_by_field() {
    let input: engine::FavouritesSettings =
        contract_inp(&settings(FavouritesMode::Selection, &[3, 1])).unwrap();
    assert_eq!(input.mode, engine::FavouritesMode::Selection);
    assert_eq!(input.selection, [3, 1]);
    let back: FavouritesSettings = contract_out(&input).unwrap();
    assert_eq!(back, settings(FavouritesMode::Selection, &[3, 1]));
    let defaults: FavouritesSettings =
        contract_out(&engine::FavouritesSettings::default()).unwrap();
    assert_eq!(defaults, settings(FavouritesMode::TopSold, &[]));
}

#[test]
fn the_mode_has_the_engines_serde_names() {
    assert_eq!(
        serde_json::to_value(FavouritesMode::TopSold).unwrap(),
        json!("top_sold")
    );
    assert_eq!(
        serde_json::to_value(FavouritesMode::Selection).unwrap(),
        json!("selection")
    );
}

#[test]
fn an_unknown_key_is_a_contract_error() {
    let mut value = serde_json::to_value(settings(FavouritesMode::TopSold, &[])).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<FavouritesSettings>(value).is_err());
}

// ───────────────────────────── (b) wiring ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse_and_a_cashier_does_not_set() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.get_favourites_settings().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.list_favourites(None).await.unwrap_err()).0,
        "not_logged_in"
    );
    e.login("75061189702".into(), Some("4444".into()))
        .await
        .unwrap();
    assert_eq!(
        e.get_favourites_settings().await.unwrap(),
        settings(FavouritesMode::TopSold, &[])
    );
    assert!(e.list_favourites(None).await.is_ok());
    assert_eq!(
        coded(
            &e.set_favourites_settings(settings(FavouritesMode::Selection, &[1]))
                .await
                .unwrap_err()
        )
        .0,
        "forbidden"
    );
    assert_eq!(
        coded(&e.set_favourite(1, true).await.unwrap_err()).0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_selection_round_trips_and_the_tab_lists_it() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    let items = e.get_menu_items(None, None).await.unwrap();
    assert!(items.len() >= 2, "the seed holds a menu");
    let (first, second) = (items[0].id, items[1].id);

    // Most sold, no sale yet: the tab is empty.
    assert!(e.list_favourites(None).await.unwrap().is_empty());

    let stored = e.set_favourite(second, true).await.unwrap();
    assert_eq!(stored, settings(FavouritesMode::TopSold, &[second]));
    let stored = e.set_favourite(first, true).await.unwrap();
    assert_eq!(stored.selection, [second, first]);

    let stored = e
        .set_favourites_settings(settings(FavouritesMode::Selection, &[second, first]))
        .await
        .unwrap();
    assert_eq!(e.get_favourites_settings().await.unwrap(), stored);
    let tab = e.list_favourites(Some("fr".into())).await.unwrap();
    assert_eq!(
        tab.iter().map(|m| m.id).collect::<Vec<_>>(),
        [second, first]
    );
    assert_eq!(tab[0].name, items[1].name);

    let after = e.set_favourite(second, false).await.unwrap();
    assert_eq!(after.selection, [first]);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_refusals_are_coded_and_leave_the_setting_alone() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    let before = e
        .set_favourites_settings(settings(FavouritesMode::Selection, &[]))
        .await
        .unwrap();

    let err = e.set_favourite(987_654, true).await.unwrap_err();
    assert_eq!(coded(&err).0, "favourites_invalid");
    let err = e
        .set_favourites_settings(settings(FavouritesMode::Selection, &[987_654]))
        .await
        .unwrap_err();
    assert_eq!(coded(&err).0, "favourites_invalid");

    // Thirty-one products: one more than the selection holds.
    let items = e.get_menu_items(None, None).await.unwrap();
    if items.len() > 30 {
        let ids: Vec<i64> = items.iter().take(31).map(|m| m.id).collect();
        let err = e
            .set_favourites_settings(settings(FavouritesMode::Selection, &ids))
            .await
            .unwrap_err();
        let (code, params) = coded(&err);
        assert_eq!(code, "favourites_too_many");
        assert_eq!(params["max"], "30");
    }
    assert_eq!(e.get_favourites_settings().await.unwrap(), before);
    e.shutdown().await.unwrap();
}
