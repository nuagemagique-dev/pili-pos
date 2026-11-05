//! Quick notes (UX audit C11): the chips of the kitchen note dialog.
//!
//! - a new installation reads the nine usual notes; any logged-in operator
//!   reads the list, only a manager changes it;
//! - each chip is a valid kitchen note (60 characters, printable), a blank
//!   chip is dropped, a repeated one keeps its first place, the list holds
//!   24 chips at most;
//! - every change is logged in the append-only `owner_settings_log`;
//! - the list is local data: it is in no FDM request, no archived event and
//!   no column other than the two owner settings ones, even after a chip was
//!   used in a note and the order was registered, pre-billed and paid.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_db::queries::owner_settings;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_set_line_note,
};
use pos_engine::commands::quick_notes::{
    DEFAULT_QUICK_NOTES, KEY_QUICK_NOTES, MAX_QUICK_NOTES, QuickNotes, do_get_quick_notes,
    do_set_quick_notes,
};
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};

const MANAGER: &str = "12345678958";
const SERVER: &str = "85010100156";
const APPRENTICE: &str = "90020200252";

fn notes(list: &[&str]) -> QuickNotes {
    QuickNotes {
        notes: list.iter().map(|n| (*n).to_string()).collect(),
    }
}

async fn app() -> (TestApp, LossyFdm) {
    let fdm = LossyFdm::start().await;
    let app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(MANAGER, UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 1, 1200, 'B', 1, 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Maki saumon');",
        )
        .unwrap();
    (app, fdm)
}

fn history_len(app: &TestApp) -> usize {
    let db = app.state.db.lock().unwrap();
    owner_settings::history(&db, KEY_QUICK_NOTES, 100)
        .unwrap()
        .len()
}

// ── The list ───────────────────────────────────────────────────────

#[tokio::test]
async fn a_new_installation_reads_the_nine_usual_notes() {
    let (app, _fdm) = app().await;
    let list = do_get_quick_notes(&app.state).unwrap();
    assert_eq!(list.notes, DEFAULT_QUICK_NOTES);
    assert_eq!(
        list.notes,
        [
            "Sans wasabi",
            "Sans sésame",
            "Sans sauce",
            "Sauce à part",
            "Bien cuit",
            "Épicé",
            "Pas épicé",
            "Sans oignon",
            "À emporter",
        ]
    );
    // Reading stores nothing.
    assert_eq!(history_len(&app), 0);
}

#[tokio::test]
async fn a_manager_changes_the_list_and_it_reads_back_in_order() {
    let (app, _fdm) = app().await;
    let stored =
        do_set_quick_notes(&app.state, notes(&["Bien cuit", "Sans wasabi", "Épicé"])).unwrap();
    assert_eq!(stored.notes, ["Bien cuit", "Sans wasabi", "Épicé"]);
    assert_eq!(do_get_quick_notes(&app.state).unwrap(), stored);

    // Reordered: the order is the stored one.
    let reordered =
        do_set_quick_notes(&app.state, notes(&["Épicé", "Bien cuit", "Sans wasabi"])).unwrap();
    assert_eq!(reordered.notes, ["Épicé", "Bien cuit", "Sans wasabi"]);
    assert_eq!(do_get_quick_notes(&app.state).unwrap(), reordered);
}

#[tokio::test]
async fn an_empty_list_is_a_choice_and_not_the_defaults() {
    let (app, _fdm) = app().await;
    do_set_quick_notes(&app.state, notes(&[])).unwrap();
    assert_eq!(do_get_quick_notes(&app.state).unwrap().notes.len(), 0);
}

#[tokio::test]
async fn any_operator_reads_and_only_a_manager_changes_the_list() {
    let (app, _fdm) = app().await;
    app.logout();
    assert_eq!(
        do_get_quick_notes(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_quick_notes(&app.state, notes(&["x"])).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    for (insz, role) in [(SERVER, UserRole::Cashier), (APPRENTICE, UserRole::Waiter)] {
        app.login_as(insz, role);
        assert_eq!(do_get_quick_notes(&app.state).unwrap().notes.len(), 9);
        assert_eq!(
            do_set_quick_notes(&app.state, notes(&["x"])).unwrap_err(),
            ERR_FORBIDDEN
        );
    }
    // Nothing was stored by the refusals.
    assert_eq!(history_len(&app), 0);
    app.login_as(MANAGER, UserRole::Admin);
    assert!(do_set_quick_notes(&app.state, notes(&["x"])).is_ok());
}

// ── Validation ─────────────────────────────────────────────────────

#[tokio::test]
async fn a_chip_is_trimmed_a_blank_one_dropped_and_a_repeat_keeps_its_first_place() {
    let (app, _fdm) = app().await;
    let stored = do_set_quick_notes(
        &app.state,
        notes(&[
            "  Sans wasabi  ",
            "",
            "   ",
            "Bien cuit",
            "SANS WASABI",
            "bien cuit",
        ]),
    )
    .unwrap();
    assert_eq!(stored.notes, ["Sans wasabi", "Bien cuit"]);
}

#[tokio::test]
async fn a_chip_is_a_valid_kitchen_note() {
    let (app, _fdm) = app().await;
    do_set_quick_notes(&app.state, notes(&["Bien cuit"])).unwrap();

    // 60 characters fit (characters, not bytes), 61 do not.
    let sixty = "é".repeat(60);
    assert_eq!(
        do_set_quick_notes(&app.state, notes(&[&sixty]))
            .unwrap()
            .notes,
        [sixty]
    );
    assert_eq!(
        do_set_quick_notes(&app.state, notes(&[&"x".repeat(61)])).unwrap_err(),
        errors::NOTE_TOO_LONG
    );
    // What the slip printer cannot print is refused.
    for bad in ["不要芥末", "line\nbreak", "sushi \u{1F363}", "a\tb"] {
        assert_eq!(
            do_set_quick_notes(&app.state, notes(&[bad])).unwrap_err(),
            errors::NOTE_UNPRINTABLE,
            "{bad:?}"
        );
    }
    // A refusal leaves the stored list alone.
    assert_eq!(
        do_get_quick_notes(&app.state).unwrap().notes,
        ["é".repeat(60)]
    );
}

#[tokio::test]
async fn the_list_holds_24_chips_at_most() {
    let (app, _fdm) = app().await;
    assert_eq!(MAX_QUICK_NOTES, 24);
    let many = |n: usize| QuickNotes {
        notes: (0..n).map(|i| format!("Note {i}")).collect(),
    };
    assert_eq!(
        do_set_quick_notes(&app.state, many(24))
            .unwrap()
            .notes
            .len(),
        24
    );
    assert_eq!(
        do_set_quick_notes(&app.state, many(25)).unwrap_err(),
        "quick_notes_too_many|max=24"
    );
    assert_eq!(do_get_quick_notes(&app.state).unwrap().notes.len(), 24);
}

// ── The change log ─────────────────────────────────────────────────

#[tokio::test]
async fn every_change_is_logged_once_with_the_manager_and_the_log_is_append_only() {
    let (app, _fdm) = app().await;
    do_set_quick_notes(&app.state, notes(&["Bien cuit"])).unwrap();
    // The same list again is not a change.
    do_set_quick_notes(&app.state, notes(&["Bien cuit"])).unwrap();
    do_set_quick_notes(&app.state, notes(&["Bien cuit", "Épicé"])).unwrap();

    let db = app.state.db.lock().unwrap();
    let log = owner_settings::history(&db, KEY_QUICK_NOTES, 10).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].changed_by.as_deref(), Some(MANAGER));
    assert_eq!(log[0].value_json, r#"{"notes":["Bien cuit","Épicé"]}"#);
    assert_eq!(log[1].value_json, r#"{"notes":["Bien cuit"]}"#);

    assert!(
        db.execute(
            "DELETE FROM owner_settings_log WHERE key = 'quick_notes'",
            []
        )
        .is_err()
    );
    assert!(
        db.execute(
            "UPDATE owner_settings_log SET value_json = '{}' WHERE key = 'quick_notes'",
            []
        )
        .is_err()
    );
}

// ── Local data, never fiscal ───────────────────────────────────────

/// Every text column of every table, as `table.column`, that holds `needle`.
fn columns_holding(app: &TestApp, needle: &str) -> Vec<String> {
    let db = app.state.db.lock().unwrap();
    let tables: Vec<String> = db
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut found = Vec::new();
    for table in tables {
        let columns: Vec<String> = db
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for column in columns {
            let hits: i64 = db
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM \"{table}\" WHERE CAST(\"{column}\" AS TEXT) LIKE ?1"
                    ),
                    [format!("%{needle}%")],
                    |r| r.get(0),
                )
                .unwrap();
            if hits > 0 {
                found.push(format!("{table}.{column}"));
            }
        }
    }
    found.sort();
    found
}

fn order_with_a_line(app: &TestApp, table: i32) -> (i64, i64) {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(table),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    let line = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: 2,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap()
    .id;
    (order, line)
}

#[tokio::test]
async fn the_list_is_in_no_fiscal_message_and_no_archived_event() {
    let (app, fdm) = app().await;
    let secret = "zzquickchip";
    do_set_quick_notes(&app.state, notes(&[secret, "Bien cuit"])).unwrap();

    // Registered (P), pre-billed (P) and paid (N) while the list exists.
    let (order, _line) = order_with_a_line(&app, 4);
    do_sign_table_order(&app.state, order).await.unwrap();
    do_sign_pre_bill(&app.state, order).await.unwrap();
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );

    let operations: Vec<String> = fdm.sent().into_iter().map(|s| s.op).collect();
    for expected in ["signOrder", "signPreBill", "signSale"] {
        assert!(
            operations.iter().any(|o| o == expected),
            "{expected} in {operations:?}"
        );
    }
    for sent in fdm.sent() {
        assert!(!sent.body.contains(secret), "{} request", sent.op);
        assert!(!sent.body.contains("Bien cuit"), "{} request", sent.op);
    }
    for event in app.fiscal_events() {
        let request = event.event_data_json.as_deref().unwrap_or_default();
        let response = event.fdm_response_json.as_deref().unwrap_or_default();
        assert!(!request.contains(secret) && !request.contains("Bien cuit"));
        assert!(!response.contains(secret) && !response.contains("Bien cuit"));
    }
    // Only the settings store and its log hold the list.
    assert_eq!(
        columns_holding(&app, secret),
        ["owner_settings.value_json", "owner_settings_log.value_json"]
    );
}

#[tokio::test]
async fn a_chip_used_in_a_note_stays_in_the_note_columns_and_out_of_every_request() {
    let (app, fdm) = app().await;
    let secret = "zzquickchip";
    do_set_quick_notes(&app.state, notes(&[secret, "Bien cuit"])).unwrap();

    // The dialog appends chips to the note, comma separated.
    let (order, line) = order_with_a_line(&app, 5);
    do_set_line_note(&app.state, line, Some(format!("{secret}, Bien cuit")))
        .await
        .unwrap();
    do_sign_table_order(&app.state, order).await.unwrap();
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );

    for sent in fdm.sent() {
        assert!(!sent.body.contains(secret), "{} request", sent.op);
        assert!(!sent.body.contains("Bien cuit"), "{} request", sent.op);
    }
    for event in app.fiscal_events() {
        let request = event.event_data_json.as_deref().unwrap_or_default();
        assert!(!request.contains(secret) && !request.contains("Bien cuit"));
    }
    assert_eq!(
        columns_holding(&app, secret),
        [
            "order_lines.kitchen_note",
            "owner_settings.value_json",
            "owner_settings_log.value_json",
        ]
    );
}
