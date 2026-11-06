//! The favourites tab (UX audit C12).
//!
//! - "most sold": the 18 best sellers of the last 30 booking days, read from
//!   the archived signed sales (refunds count negative, training is left
//!   out), skipping products that are inactive or sold out; cached, and
//!   computed again after an hour or after a Z report;
//! - "selection": the products a manager starred, in the order they were
//!   added, a sold-out one stays in the list;
//! - any logged-in operator reads, only a manager changes the setting, every
//!   change is logged in the append-only `owner_settings_log`;
//! - local data: the setting and the cache are in no FDM request, no archived
//!   event and no table other than their own.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::fmt::Write as _;

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_db::queries::owner_settings;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::favourites::{
    FavouritesMode, FavouritesSettings, KEY_FAVOURITES, MAX_SELECTION, TOP_SOLD_COUNT,
    do_get_favourites_settings, do_list_favourites, do_set_favourite, do_set_favourites_settings,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::do_generate_z_report;
use pos_engine::commands::sold_out::do_set_sold_out;

const MANAGER: &str = "12345678958";
const SERVER: &str = "85010100156";
const APPRENTICE: &str = "90020200252";

/// Twenty-two products, ids 1 to 22, plus the inactive 23.
async fn app() -> (TestApp, LossyFdm) {
    let fdm = LossyFdm::start().await;
    let app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(MANAGER, UserRole::Admin);
    let mut sql =
        String::from("INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);\n");
    for id in 1..=23 {
        let active = i32::from(id != 23);
        let price = 500 + id * 10;
        writeln!(
            sql,
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES ({id}, 1, {price}, 'B', 1, {id}, {active});\n\
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES ('menu_item', {id}, 'name', 'fr', 'Produit {id}');"
        )
        .unwrap();
    }
    app.state.db.lock().unwrap().execute_batch(&sql).unwrap();
    (app, fdm)
}

fn ids(items: &[pos_core::menu::MenuItem]) -> Vec<i64> {
    items.iter().map(|m| m.id).collect()
}

fn list(app: &TestApp) -> Vec<i64> {
    ids(&do_list_favourites(&app.state, None).unwrap())
}

/// Sell `quantity` of `item` in one takeaway sale paid in cash; returns the sale's event id.
async fn sell(app: &TestApp, item: i64, quantity: i32) -> i64 {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: item,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    let result = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    result.fiscal_event_id.unwrap()
}

/// Make the cached ranking look `minutes` old.
fn age_cache(app: &TestApp, minutes: i64) {
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE favourites_cache_meta SET computed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now', ?1)",
            [format!("-{minutes} minutes")],
        )
        .unwrap();
}

fn selection(mode: FavouritesMode, ids: &[i64]) -> FavouritesSettings {
    FavouritesSettings {
        mode,
        selection: ids.to_vec(),
    }
}

fn history_len(app: &TestApp) -> usize {
    let db = app.state.db.lock().unwrap();
    owner_settings::history(&db, KEY_FAVOURITES, 100)
        .unwrap()
        .len()
}

// ── The setting ────────────────────────────────────────────────────

#[tokio::test]
async fn a_new_installation_shows_the_most_sold_and_has_no_selection() {
    let (app, _fdm) = app().await;
    let settings = do_get_favourites_settings(&app.state).unwrap();
    assert_eq!(settings.mode, FavouritesMode::TopSold);
    assert_eq!(settings.selection, Vec::<i64>::new());
    // Nothing sold yet: the tab is empty, so the UIs hide it.
    assert_eq!(list(&app), Vec::<i64>::new());
    // Reading stores nothing.
    assert_eq!(history_len(&app), 0);
}

#[tokio::test]
async fn any_operator_reads_and_only_a_manager_changes_the_setting() {
    let (app, _fdm) = app().await;
    app.logout();
    assert_eq!(
        do_get_favourites_settings(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_list_favourites(&app.state, None).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    for (insz, role) in [(SERVER, UserRole::Cashier), (APPRENTICE, UserRole::Waiter)] {
        app.login_as(insz, role);
        assert!(do_get_favourites_settings(&app.state).is_ok());
        assert!(do_list_favourites(&app.state, None).is_ok());
        assert_eq!(
            do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[1]))
                .unwrap_err(),
            ERR_FORBIDDEN
        );
        assert_eq!(
            do_set_favourite(&app.state, 1, true).unwrap_err(),
            ERR_FORBIDDEN
        );
    }
    assert_eq!(history_len(&app), 0);
}

#[tokio::test]
async fn starring_adds_at_the_end_in_the_order_of_addition_and_unstarring_removes() {
    let (app, _fdm) = app().await;
    do_set_favourite(&app.state, 5, true).unwrap();
    do_set_favourite(&app.state, 2, true).unwrap();
    let after = do_set_favourite(&app.state, 9, true).unwrap();
    assert_eq!(after.selection, [5, 2, 9]);
    // Starring again changes nothing (not even the order).
    assert_eq!(
        do_set_favourite(&app.state, 5, true).unwrap().selection,
        [5, 2, 9]
    );
    assert_eq!(
        do_set_favourite(&app.state, 2, false).unwrap().selection,
        [5, 9]
    );
    // A product starred again goes to the end.
    assert_eq!(
        do_set_favourite(&app.state, 2, true).unwrap().selection,
        [5, 9, 2]
    );
    // Unstarring a product that is not there is not an error.
    assert_eq!(
        do_set_favourite(&app.state, 20, false).unwrap().selection,
        [5, 9, 2]
    );
    assert_eq!(
        do_get_favourites_settings(&app.state).unwrap().selection,
        [5, 9, 2]
    );
}

#[tokio::test]
async fn the_selection_is_checked_and_kept_while_the_mode_is_most_sold() {
    let (app, _fdm) = app().await;
    // A repeat keeps its first place.
    let stored = do_set_favourites_settings(
        &app.state,
        selection(FavouritesMode::TopSold, &[4, 1, 4, 3, 1]),
    )
    .unwrap();
    assert_eq!(stored.selection, [4, 1, 3]);
    assert_eq!(stored.mode, FavouritesMode::TopSold);
    // An unknown product, and the system gift voucher product, are refused.
    for bad in [9999, 990_001, -1] {
        assert_eq!(
            do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[1, bad]))
                .unwrap_err(),
            errors::FAVOURITES_INVALID
        );
        assert_eq!(
            do_set_favourite(&app.state, bad, true).unwrap_err(),
            errors::FAVOURITES_INVALID
        );
    }
    // A refusal leaves the stored setting alone.
    assert_eq!(do_get_favourites_settings(&app.state).unwrap(), stored);
    // The mode switches without losing the list.
    do_set_favourites_settings(
        &app.state,
        selection(FavouritesMode::Selection, &stored.selection),
    )
    .unwrap();
    assert_eq!(list(&app), [4, 1, 3]);
    do_set_favourites_settings(
        &app.state,
        selection(FavouritesMode::TopSold, &stored.selection),
    )
    .unwrap();
    assert_eq!(
        do_get_favourites_settings(&app.state).unwrap().selection,
        [4, 1, 3]
    );
}

#[tokio::test]
async fn the_selection_holds_thirty_products_at_most() {
    let (app, _fdm) = app().await;
    assert_eq!(MAX_SELECTION, 30);
    // Only 22 active products plus 1 inactive exist: add 7 more so 31 are known.
    let mut sql = String::new();
    for id in 24..=40 {
        writeln!(
            sql,
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES ({id}, 1, 700, 'B', 1, {id}, 1);"
        )
        .unwrap();
    }
    app.state.db.lock().unwrap().execute_batch(&sql).unwrap();
    let thirty: Vec<i64> = (1..=30).collect();
    assert_eq!(
        do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &thirty))
            .unwrap()
            .selection
            .len(),
        30
    );
    let thirty_one: Vec<i64> = (1..=31).collect();
    assert_eq!(
        do_set_favourites_settings(
            &app.state,
            selection(FavouritesMode::Selection, &thirty_one)
        )
        .unwrap_err(),
        "favourites_too_many|max=30"
    );
    assert_eq!(
        do_set_favourite(&app.state, 31, true).unwrap_err(),
        "favourites_too_many|max=30"
    );
    assert_eq!(
        do_get_favourites_settings(&app.state)
            .unwrap()
            .selection
            .len(),
        30
    );
}

#[tokio::test]
async fn every_change_is_logged_with_the_manager_and_the_log_is_append_only() {
    let (app, _fdm) = app().await;
    do_set_favourite(&app.state, 3, true).unwrap();
    // The same star again is not a change.
    do_set_favourite(&app.state, 3, true).unwrap();
    do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[3])).unwrap();

    let db = app.state.db.lock().unwrap();
    let log = owner_settings::history(&db, KEY_FAVOURITES, 10).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].changed_by.as_deref(), Some(MANAGER));
    assert_eq!(log[0].value_json, r#"{"mode":"selection","selection":[3]}"#);
    assert_eq!(log[1].value_json, r#"{"mode":"top_sold","selection":[3]}"#);
    assert!(
        db.execute(
            "DELETE FROM owner_settings_log WHERE key = 'favourites'",
            []
        )
        .is_err()
    );
}

// ── Most sold ──────────────────────────────────────────────────────

#[tokio::test]
async fn the_most_sold_are_ranked_by_units_best_first_and_ties_by_id() {
    let (app, _fdm) = app().await;
    sell(&app, 7, 3).await;
    sell(&app, 2, 1).await;
    sell(&app, 4, 3).await;
    sell(&app, 9, 5).await;
    sell(&app, 2, 1).await;
    // 9: 5, 4: 3, 7: 3, 2: 2.
    assert_eq!(list(&app), [9, 4, 7, 2]);
    let items = do_list_favourites(&app.state, Some("fr".into())).unwrap();
    assert_eq!(items[0].name, "Produit 9");
    assert_eq!(items[0].price_cents, 590);
}

#[tokio::test]
async fn the_tab_holds_eighteen_products() {
    let (app, _fdm) = app().await;
    assert_eq!(TOP_SOLD_COUNT, 18);
    // 22 products, product n sold n times in one sale: the best are 22 down to 5.
    for id in 1..=22 {
        sell(&app, id, i32::try_from(id).unwrap()).await;
    }
    let shown = list(&app);
    assert_eq!(shown.len(), 18);
    assert_eq!(shown, (5..=22).rev().collect::<Vec<i64>>());
}

#[tokio::test]
async fn an_inactive_or_sold_out_product_gives_its_place_to_the_next_one() {
    let (app, _fdm) = app().await;
    sell(&app, 1, 5).await;
    sell(&app, 2, 4).await;
    sell(&app, 3, 3).await;
    assert_eq!(list(&app), [1, 2, 3]);

    // Sold out right now: skipped in the most sold mode.
    do_set_sold_out(&app.state, 1, false, None).unwrap();
    age_cache(&app, 0);
    assert_eq!(list(&app), [2, 3]);

    // Deactivated in the menu: skipped too (the ranking is not even recomputed).
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE menu_items SET active = 0 WHERE id = 2", [])
        .unwrap();
    assert_eq!(list(&app), [3]);
}

#[tokio::test]
async fn a_refund_takes_its_units_back_and_training_sales_do_not_count() {
    let (app, _fdm) = app().await;
    let event = sell(&app, 1, 2).await;
    sell(&app, 2, 3).await;
    assert_eq!(list(&app), [2, 1]);

    let refund = do_refund_checkout(
        &app.state,
        event,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 2,
            line_id: None,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    age_cache(&app, 61);
    // Product 1 is back at zero net units: not a best seller at all.
    assert_eq!(list(&app), [2]);

    // A training sale is not a signed sale of the day.
    app.state.config.lock().unwrap().training_mode = true;
    sell(&app, 5, 9).await;
    app.state.config.lock().unwrap().training_mode = false;
    age_cache(&app, 61);
    assert_eq!(list(&app), [2]);
}

#[tokio::test]
async fn the_ranking_is_kept_for_an_hour_and_computed_again_after() {
    let (app, _fdm) = app().await;
    sell(&app, 1, 1).await;
    assert_eq!(list(&app), [1]);

    // A better seller appears, but the cache is fresh: the tab does not move.
    sell(&app, 2, 4).await;
    assert_eq!(list(&app), [1]);
    age_cache(&app, 59);
    assert_eq!(list(&app), [1]);

    // An hour old: computed again.
    age_cache(&app, 61);
    assert_eq!(list(&app), [2, 1]);
    // And fresh again.
    sell(&app, 3, 9).await;
    assert_eq!(list(&app), [2, 1]);
}

#[tokio::test]
async fn a_z_report_makes_the_ranking_current_at_once() {
    let (app, _fdm) = app().await;
    sell(&app, 1, 1).await;
    assert_eq!(list(&app), [1]);
    sell(&app, 2, 4).await;
    assert_eq!(list(&app), [1]);

    let z = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);
    // Fresh cache, but a Z was made since it was computed.
    assert_eq!(list(&app), [2, 1]);
}

#[tokio::test]
async fn only_the_last_thirty_booking_days_count() {
    let (app, _fdm) = app().await;
    sell(&app, 1, 5).await;
    let shift = |days: i64| {
        let db = app.state.db.lock().unwrap();
        // The open period is the only one that can still be edited; the archive stays untouched.
        db.execute(
            "UPDATE booking_periods SET booking_date = date(booking_date, ?1) WHERE closed_at IS NULL",
            [format!("-{days} days")],
        )
        .unwrap();
    };
    // Today and 29 days back are inside the window (30 booking days).
    age_cache(&app, 61);
    assert_eq!(list(&app), [1]);
    shift(29);
    age_cache(&app, 61);
    assert_eq!(list(&app), [1]);
    // One more day back and the sale leaves the window.
    shift(1);
    age_cache(&app, 61);
    assert_eq!(list(&app), Vec::<i64>::new());
}

// ── Ma sélection ───────────────────────────────────────────────────

#[tokio::test]
async fn the_selection_shows_in_its_order_and_keeps_a_sold_out_product_marked() {
    let (app, _fdm) = app().await;
    sell(&app, 1, 9).await;
    do_set_favourites_settings(
        &app.state,
        selection(FavouritesMode::Selection, &[8, 1, 12]),
    )
    .unwrap();
    assert_eq!(list(&app), [8, 1, 12]);

    do_set_sold_out(&app.state, 1, false, None).unwrap();
    let items = do_list_favourites(&app.state, None).unwrap();
    assert_eq!(ids(&items), [8, 1, 12]);
    assert!(items[1].sold_out);
    assert!(!items[0].sold_out);

    // The most sold has nothing to do with the selection: the only seller (product 1)
    // is sold out, so the tab is empty.
    do_set_favourites_settings(&app.state, selection(FavouritesMode::TopSold, &[8, 1, 12]))
        .unwrap();
    assert_eq!(list(&app), Vec::<i64>::new());
}

#[tokio::test]
async fn an_empty_selection_hides_the_tab() {
    let (app, _fdm) = app().await;
    sell(&app, 1, 9).await;
    do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[])).unwrap();
    assert_eq!(list(&app), Vec::<i64>::new());
}

#[tokio::test]
async fn a_product_that_left_the_menu_drops_out_of_the_selection_view() {
    let (app, _fdm) = app().await;
    do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[23, 4])).unwrap();
    // 23 is inactive: not on sale, not shown (the setting still holds it).
    assert_eq!(list(&app), [4]);
    assert_eq!(
        do_get_favourites_settings(&app.state).unwrap().selection,
        [23, 4]
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

#[tokio::test]
async fn the_setting_and_the_cache_are_in_no_fiscal_message_and_no_archived_event() {
    let (app, fdm) = app().await;
    do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[3, 6])).unwrap();
    // Sell from the favourites list, register a Z while both exist.
    sell(&app, 3, 2).await;
    do_set_favourites_settings(&app.state, selection(FavouritesMode::TopSold, &[3, 6])).unwrap();
    assert_eq!(list(&app), [3]);
    let z = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);

    let operations: Vec<String> = fdm.sent().into_iter().map(|s| s.op).collect();
    assert!(operations.iter().any(|o| o == "signSale"), "{operations:?}");
    for sent in fdm.sent() {
        for word in ["favourite", "top_sold", "selection"] {
            assert!(
                !sent.body.contains(word),
                "{word} in the {} request",
                sent.op
            );
        }
    }
    for event in app.fiscal_events() {
        let request = event.event_data_json.as_deref().unwrap_or_default();
        let response = event.fdm_response_json.as_deref().unwrap_or_default();
        for word in ["favourite", "top_sold", "\"selection\""] {
            assert!(
                !request.contains(word) && !response.contains(word),
                "{word}"
            );
        }
    }
    // Only the settings store and its log hold the setting.
    assert_eq!(
        columns_holding(&app, "top_sold"),
        ["owner_settings.value_json", "owner_settings_log.value_json"]
    );
}

#[tokio::test]
async fn a_sale_signs_the_same_lines_whether_or_not_a_favourites_setting_exists() {
    let (app, _fdm) = app().await;
    sell(&app, 4, 2).await;
    do_set_favourites_settings(&app.state, selection(FavouritesMode::Selection, &[4, 5])).unwrap();
    // The product is added from the favourites list exactly as from its category.
    let from_tab = do_list_favourites(&app.state, None).unwrap();
    assert_eq!(ids(&from_tab), [4, 5]);
    sell(&app, from_tab[0].id, 2).await;

    let lines: Vec<serde_json::Value> = app
        .fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "N")
        .map(|e| {
            let request: serde_json::Value =
                serde_json::from_str(e.event_data_json.as_deref().unwrap()).unwrap();
            request["transaction"]["transactionLines"].clone()
        })
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], lines[1]);
    assert_eq!(lines[0][0]["mainProduct"]["productName"], "Produit 4");
}
