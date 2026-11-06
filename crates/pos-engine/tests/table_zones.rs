//! Table zones and covers (UX audit T1 and T5).
//!
//! - the plan accepts zones written with letters ("A 1-20, B 21-40") and a
//!   manager can name them; a table is then written "A-19" on the Tables
//!   screen data and on the kitchen, note and transfer slips, a plan without
//!   letters is unchanged;
//! - the table of every signed request and of the archive is still its number
//!   (`T19`), and neither a zone, a zone name nor the covers reaches the FDM,
//!   the archive or any other table than their own;
//! - an occupied table card carries the initial of the user who opened the
//!   order and the covers the operator set (1 to 30, local data).

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::covers::{do_get_order_covers, do_set_order_covers};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::table_plan::{do_get_table_plan, do_set_table_plan};
use pos_engine::commands::table_transfer::do_transfer_table;
use pos_engine::commands::tables::{
    do_list_configured_tables, do_sign_pre_bill, do_sign_table_order,
};

const MANAGER: &str = "12345678958";
const SERVER: &str = "94060242576";

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
                ('menu_item', 2, 'name', 'fr', 'Maki saumon');
             UPDATE users SET name = 'Zoe Martin' WHERE insz = '12345678958';",
        )
        .unwrap();
    (app, fdm)
}

fn plan(app: &TestApp, text: &str, names: &str) {
    do_set_table_plan(&app.state, text, names).unwrap();
}

fn order_as(app: &TestApp, user: &str, order_type: OrderType, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type,
            table_number: table,
            user_insz: user.into(),
        },
    )
    .unwrap()
    .id
}

fn table_order(app: &TestApp, table: i32) -> i64 {
    order_as(app, MANAGER, OrderType::DineIn, Some(table))
}

fn add(app: &TestApp, order: i64, item: i64, quantity: i32) {
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
}

fn rendered(slips: &[String]) -> Vec<String> {
    slips
        .iter()
        .map(|s| pos_print::render::render_plain_text(s.as_bytes()))
        .collect()
}

// ── The plan ───────────────────────────────────────────────────────

#[tokio::test]
async fn a_manager_sets_zones_and_names_and_the_plan_reads_them_back() {
    let (app, _fdm) = app().await;
    plan(&app, "b 21-23, a1-3", "B : Terrasse, A \u{b7} Salle");
    let read = do_get_table_plan(&app.state).unwrap();
    assert_eq!(read.plan, "A 1-3, B 21-23", "canonical form");
    assert_eq!(read.zone_names, "A \u{b7} Salle, B \u{b7} Terrasse");
    assert_eq!(read.table_count, 6);
    assert_eq!(read.zones.len(), 2);
    assert_eq!(
        (read.zones[0].letter.as_str(), read.zones[0].name.as_deref()),
        ("A", Some("Salle"))
    );
    assert_eq!(read.zones[0].numbers, vec![1, 2, 3]);
    assert_eq!(read.zones[1].numbers, vec![21, 22, 23]);
    // The log keeps the plan and the names, with the manager.
    assert_eq!(read.log[0].plan, "A 1-3, B 21-23");
    assert_eq!(read.log[0].zone_names, "A \u{b7} Salle, B \u{b7} Terrasse");
    assert_eq!(read.log[0].changed_by.as_deref(), Some(MANAGER));
}

#[tokio::test]
async fn the_tables_carry_their_zone_their_name_and_the_label_a_19() {
    let (app, _fdm) = app().await;
    plan(&app, "A 1-20, B 21-40", "A \u{b7} Salle, B \u{b7} Terrasse");
    let tables = do_list_configured_tables(&app.state).unwrap();
    assert_eq!(tables.len(), 40);
    let t19 = tables.iter().find(|t| t.table_number == 19).unwrap();
    assert_eq!(t19.label, "A-19");
    assert_eq!(t19.zone.as_deref(), Some("A"));
    assert_eq!(t19.zone_name.as_deref(), Some("Salle"));
    let t30 = tables.iter().find(|t| t.table_number == 30).unwrap();
    assert_eq!(t30.label, "B-30");
    assert_eq!(t30.zone_name.as_deref(), Some("Terrasse"));
    // Numbers are unique across zones: 19 and 30 are two different tables.
    assert_ne!(t19.table_number, t30.table_number);
}

#[tokio::test]
async fn a_zone_without_a_name_has_none_and_a_plan_without_letters_is_unchanged() {
    let (app, _fdm) = app().await;
    plan(&app, "A 1-3, B 4-6", "A \u{b7} Salle");
    let tables = do_list_configured_tables(&app.state).unwrap();
    assert_eq!(tables[4].label, "B-5");
    assert_eq!(tables[4].zone_name, None);

    plan(&app, "1-6", "");
    let tables = do_list_configured_tables(&app.state).unwrap();
    for t in &tables {
        assert_eq!(t.label, t.table_number.to_string());
        assert_eq!((t.zone.as_deref(), t.zone_name.as_deref()), (None, None));
    }
    let read = do_get_table_plan(&app.state).unwrap();
    assert_eq!(read.zones.len(), 0);
    assert_eq!(read.zone_names, "");
}

#[tokio::test]
async fn a_wrong_zone_or_name_is_refused_and_the_old_plan_kept() {
    let (app, _fdm) = app().await;
    plan(&app, "A 1-4", "A \u{b7} Salle");
    for (text, names) in [
        ("A 1-10, B 10-20", ""),
        ("1-5, A 5", ""),
        ("ABC 1-4", ""),
        ("A 1-4", "B \u{b7} Terrasse"),
        ("A 1-4", "Salle"),
        ("A 1-4", "A \u{b7} aaaaaaaaaaaaaaaaaaaaa"),
        ("A 1-4", "A \u{b7} Salle, A \u{b7} Bar"),
    ] {
        assert_eq!(
            do_set_table_plan(&app.state, text, names).unwrap_err(),
            errors::TABLE_PLAN_INVALID,
            "{text:?} {names:?}"
        );
    }
    let read = do_get_table_plan(&app.state).unwrap();
    assert_eq!(read.plan, "A 1-4");
    assert_eq!(read.zone_names, "A \u{b7} Salle");
}

#[tokio::test]
async fn only_a_manager_changes_the_zones() {
    let (app, _fdm) = app().await;
    app.login_as(SERVER, UserRole::Waiter);
    assert!(do_set_table_plan(&app.state, "A 1-4", "").is_err());
    assert!(do_get_table_plan(&app.state).is_ok(), "everyone reads it");
}

#[tokio::test]
async fn a_table_left_out_of_the_plan_keeps_its_open_order_and_has_no_zone() {
    let (app, _fdm) = app().await;
    let o = table_order(&app, 9);
    add(&app, o, 1, 1);
    plan(&app, "A 1-3", "");
    let tables = do_list_configured_tables(&app.state).unwrap();
    let nine = tables.iter().find(|t| t.table_number == 9).unwrap();
    assert_eq!(nine.order_id, Some(o));
    assert_eq!((nine.label.as_str(), nine.zone.as_deref()), ("9", None));
}

// ── The slips ──────────────────────────────────────────────────────

#[tokio::test]
async fn the_kitchen_slip_and_the_transfer_slip_write_the_zone() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    plan(&app, "A 1-20, B 21-40", "");
    let o = table_order(&app, 19);
    add(&app, o, 2, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_transfer_table(&app.state, o, 30).await.unwrap();

    let slips = rendered(&printer.settle_slips(2).await);
    assert!(slips[0].contains("Table A-19"), "{}", slips[0]);
    assert!(slips[0].contains("1 x Maki saumon"));
    assert!(slips[1].contains("TRANSFERT DE TABLE"), "{}", slips[1]);
    assert!(
        slips[1].contains("Table A-19 -> Table B-30"),
        "{}",
        slips[1]
    );
}

#[tokio::test]
async fn the_note_slip_of_a_sent_line_writes_the_zone() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    plan(&app, "A 1-20", "");
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    let line = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT id FROM order_lines WHERE order_id = ?1", [o], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    pos_engine::commands::order::do_set_line_note(&app.state, line, Some("sans wasabi".into()))
        .await
        .unwrap();
    let slips = rendered(&printer.settle_slips(2).await);
    assert!(
        slips[1].contains("Table A-4") && slips[1].contains("sans wasabi"),
        "{}",
        slips[1]
    );
}

#[tokio::test]
async fn a_plan_without_letters_prints_the_plain_number_as_before() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_transfer_table(&app.state, o, 8).await.unwrap();
    let slips = rendered(&printer.settle_slips(2).await);
    assert!(slips[0].contains("Table 4") && !slips[0].contains("Table 4-"));
    assert!(slips[1].contains("Table 4 -> Table 8"), "{}", slips[1]);
}

// ── Fiscal: the table is still its number ──────────────────────────

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
async fn the_zone_the_name_and_the_covers_are_in_no_fiscal_message_and_no_archived_event() {
    let (app, fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    plan(
        &app,
        "A 1-20, B 21-40",
        "A \u{b7} Zzsalle, B \u{b7} Zzterrasse",
    );

    // A table order in zone A, covers set, registered, pre-billed, moved to
    // zone B, paid: an order P, a pre-bill P, a cost center change P and a sale N.
    let o = table_order(&app, 19);
    do_set_order_covers(&app.state, o, Some(29)).unwrap();
    add(&app, o, 2, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_sign_pre_bill(&app.state, o).await.unwrap();
    do_transfer_table(&app.state, o, 30).await.unwrap();
    assert!(
        do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );

    let operations: Vec<String> = fdm.sent().into_iter().map(|s| s.op).collect();
    for expected in [
        "signOrder",
        "signPreBill",
        "signCostCenterChange",
        "signSale",
    ] {
        assert!(
            operations.iter().any(|op| op == expected),
            "{expected} in {operations:?}"
        );
    }
    let secrets = [
        "A-19",
        "B-30",
        "Zzsalle",
        "Zzterrasse",
        "covers",
        "couvert",
        "\"zone",
    ];
    for sent in fdm.sent() {
        for secret in secrets {
            assert!(
                !sent.body.contains(secret),
                "{secret} in the {} request",
                sent.op
            );
        }
    }
    // The cost center is the table number, unchanged.
    let requests: Vec<serde_json::Value> = app
        .fiscal_events()
        .iter()
        .filter_map(|e| e.event_data_json.as_deref())
        .map(|j| serde_json::from_str(j).unwrap())
        .collect();
    assert_eq!(requests[0]["costCenter"]["id"], "T19");
    assert_eq!(
        requests[2]["transfer"]["from"][0]["costCenter"]["id"],
        "T19"
    );
    assert_eq!(requests[2]["transfer"]["to"][0]["costCenter"]["id"], "T30");
    assert_eq!(requests[3]["costCenter"]["id"], "T30");
    for event in app.fiscal_events() {
        let request = event.event_data_json.as_deref().unwrap_or_default();
        let response = event.fdm_response_json.as_deref().unwrap_or_default();
        for secret in secrets {
            assert!(
                !request.contains(secret) && !response.contains(secret),
                "{secret}"
            );
        }
    }

    // The names are only in the plan and its log; the covers only in their own table.
    assert_eq!(
        columns_holding(&app, "Zzsalle"),
        ["table_plan.zone_names", "table_plan_log.zone_names"]
    );
    let covers_columns: Vec<String> = {
        let db = app.state.db.lock().unwrap();
        db.prepare(
            "SELECT m.name FROM sqlite_master m, pragma_table_info(m.name) c
             WHERE m.type = 'table' AND c.name = 'covers'",
        )
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
    };
    assert_eq!(covers_columns, ["order_covers"]);

    // The paper: no ticket or pre-bill carries the zone or the covers.
    for document in printer.rendered() {
        if !document.contains("BON DE COMMANDE") && !document.contains("TRANSFERT") {
            assert!(
                !document.contains("A-19")
                    && !document.contains("B-30")
                    && !document.contains("couv"),
                "{document}"
            );
        }
    }
}

// ── The server and the covers ──────────────────────────────────────

#[tokio::test]
async fn an_occupied_table_carries_the_initial_of_the_user_who_opened_the_order() {
    let (app, _fdm) = app().await;
    app.login_as(SERVER, UserRole::Waiter);
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE users SET name = 'anna dupont' WHERE insz = ?1",
            [SERVER],
        )
        .unwrap();
    let mine = order_as(&app, SERVER, OrderType::DineIn, Some(2));
    add(&app, mine, 1, 1);
    // An empty table order is a free table: no badge.
    let _empty = order_as(&app, SERVER, OrderType::DineIn, Some(4));
    app.login_as(MANAGER, UserRole::Admin);
    let manager = order_as(&app, MANAGER, OrderType::DineIn, Some(3));
    add(&app, manager, 1, 1);

    let tables = do_list_configured_tables(&app.state).unwrap();
    let by = |n: i32| tables.iter().find(|t| t.table_number == n).unwrap();
    assert_eq!(by(2).opened_by_initial.as_deref(), Some("A"), "upper case");
    assert_eq!(by(3).opened_by_initial.as_deref(), Some("Z"));
    assert_eq!(by(4).opened_by_initial, None);
    assert_eq!(by(5).opened_by_initial, None);
}

#[tokio::test]
async fn the_covers_are_set_changed_cleared_and_shown_on_the_table_card() {
    let (app, _fdm) = app().await;
    let o = table_order(&app, 6);
    add(&app, o, 1, 1);
    assert_eq!(do_get_order_covers(&app.state, o).unwrap(), None);
    let card = |app: &TestApp| {
        do_list_configured_tables(&app.state)
            .unwrap()
            .into_iter()
            .find(|t| t.table_number == 6)
            .unwrap()
            .covers
    };
    assert_eq!(card(&app), None);

    assert_eq!(
        do_set_order_covers(&app.state, o, Some(4)).unwrap(),
        Some(4)
    );
    assert_eq!(do_get_order_covers(&app.state, o).unwrap(), Some(4));
    assert_eq!(card(&app), Some(4));
    assert_eq!(
        do_set_order_covers(&app.state, o, Some(30)).unwrap(),
        Some(30)
    );
    assert_eq!(card(&app), Some(30));
    assert_eq!(do_set_order_covers(&app.state, o, None).unwrap(), None);
    assert_eq!(card(&app), None);
}

#[tokio::test]
async fn the_covers_stay_between_1_and_30_on_an_open_table_order() {
    let (app, _fdm) = app().await;
    let o = table_order(&app, 6);
    for bad in [0, -3, 31, 100] {
        assert_eq!(
            do_set_order_covers(&app.state, o, Some(bad)).unwrap_err(),
            "covers_invalid|max=30",
            "{bad}"
        );
    }
    assert_eq!(do_get_order_covers(&app.state, o).unwrap(), None);

    // A takeaway has no covers; an order that is not open takes none.
    let takeaway = order_as(&app, MANAGER, OrderType::Takeaway, None);
    assert_eq!(
        do_set_order_covers(&app.state, takeaway, Some(2)).unwrap_err(),
        errors::TAKEAWAY_HAS_NO_TABLE
    );
    assert_eq!(
        do_set_order_covers(&app.state, 9999, Some(2)).unwrap_err(),
        errors::ORDER_NOT_OPEN
    );
    add(&app, o, 1, 1);
    do_set_order_covers(&app.state, o, Some(2)).unwrap();
    assert!(
        do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        do_set_order_covers(&app.state, o, Some(5)).unwrap_err(),
        errors::ORDER_NOT_OPEN
    );
    // Logged out nobody sets or reads them.
    app.logout();
    assert!(do_get_order_covers(&app.state, o).is_err());
    assert!(do_set_order_covers(&app.state, o, Some(2)).is_err());
}

#[tokio::test]
async fn the_covers_follow_the_order_to_the_table_it_moves_to() {
    let (app, _fdm) = app().await;
    let o = table_order(&app, 6);
    add(&app, o, 1, 1);
    do_set_order_covers(&app.state, o, Some(5)).unwrap();
    do_transfer_table(&app.state, o, 7).await.unwrap();
    let tables = do_list_configured_tables(&app.state).unwrap();
    let find = |n: i32| tables.iter().find(|t| t.table_number == n).unwrap();
    assert_eq!(find(7).covers, Some(5));
    assert_eq!(find(6).covers, None, "the old table is free");
}
