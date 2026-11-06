//! The numbered takeaway (UX audit T2): the number of the day of a
//! takeaway sale ("E-61") and the optional name and phone of the customer.
//!
//! - a takeaway of the till gets a number when it is created: the first
//!   number is a manager's setting (default 61), the numbers follow, they
//!   restart each business day, and a takeaway that held nothing gives its
//!   number back;
//! - the name and phone are validated, replaced as a whole, kept for the
//!   open takeaway only, and cleared 72 hours after the order opened (the
//!   number stays);
//! - the number and the name print in large type on the kitchen and bar
//!   slip (never the phone), a table slip does not change;
//! - the tag is local data: it is in no FDM request, no archived event, no
//!   ticket and no column other than its own table, and the list of the
//!   open takeaways carries it.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use chrono::Duration;
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_db::queries::owner_settings;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_set_line_note,
};
use pos_engine::commands::session::{LeaveReason, do_leave_order};
use pos_engine::commands::tables::{do_list_open_takeaways, do_sign_pre_bill};
use pos_engine::commands::takeaway::{
    DEFAULT_FIRST_NUMBER, KEY_TAKEAWAY_NUMBERING, MAX_FIRST_NUMBER, TakeawayNumbering,
    do_get_takeaway_numbering, do_get_takeaway_tag, do_set_takeaway_customer,
    do_set_takeaway_numbering, purge_contacts_at,
};

const MANAGER: &str = "12345678958";
const SERVER: &str = "85010100156";

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

fn takeaway(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id
}

fn table_order(app: &TestApp, table: i32) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(table),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id
}

fn add(app: &TestApp, order: i64, item: i64, quantity: i32) -> i64 {
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
    .unwrap()
    .id
}

fn label(app: &TestApp, order: i64) -> Option<String> {
    do_get_takeaway_tag(&app.state, order)
        .unwrap()
        .map(|t| t.label)
}

fn numbering(first: i32) -> TakeawayNumbering {
    TakeawayNumbering {
        first_number: first,
    }
}

fn rendered(slips: &[String]) -> Vec<String> {
    slips
        .iter()
        .map(|s| pos_print::render::render_plain_text(s.as_bytes()))
        .collect()
}

// ── The number ─────────────────────────────────────────────────────

#[tokio::test]
async fn the_first_takeaway_of_a_new_installation_is_e_61_and_the_next_follow() {
    let (app, _fdm) = app().await;
    assert_eq!(DEFAULT_FIRST_NUMBER, 61);
    assert_eq!(
        do_get_takeaway_numbering(&app.state).unwrap(),
        numbering(61)
    );
    let (a, b, c) = (takeaway(&app), takeaway(&app), takeaway(&app));
    assert_eq!(label(&app, a).as_deref(), Some("E-61"));
    assert_eq!(label(&app, b).as_deref(), Some("E-62"));
    assert_eq!(label(&app, c).as_deref(), Some("E-63"));
    let tag = do_get_takeaway_tag(&app.state, a).unwrap().unwrap();
    assert_eq!((tag.number, tag.order_id), (61, a));
    assert_eq!((tag.customer_name, tag.customer_phone), (None, None));
}

#[tokio::test]
async fn the_day_of_a_number_is_the_business_day_the_engine_uses() {
    let (app, _fdm) = app().await;
    let order = takeaway(&app);
    let tag = do_get_takeaway_tag(&app.state, order).unwrap().unwrap();
    let day_end_hour = app.state.config.lock().unwrap().day_end_hour;
    let expected = (pos_fiscal::event::now_in_belgium() - Duration::hours(i64::from(day_end_hour)))
        .date_naive()
        .to_string();
    assert_eq!(tag.day, expected);
}

#[tokio::test]
async fn the_numbers_restart_each_business_day() {
    let (app, _fdm) = app().await;
    let (a, b) = (takeaway(&app), takeaway(&app));
    assert_eq!(label(&app, b).as_deref(), Some("E-62"));
    // The day changes: yesterday's tags are of an earlier day.
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE takeaway_tags SET day = '2020-01-01'", [])
        .unwrap();
    let c = takeaway(&app);
    assert_eq!(label(&app, c).as_deref(), Some("E-61"));
    // Yesterday's numbers did not change.
    assert_eq!(label(&app, a).as_deref(), Some("E-61"));
    assert_eq!(label(&app, b).as_deref(), Some("E-62"));
}

#[tokio::test]
async fn a_table_has_no_number() {
    let (app, _fdm) = app().await;
    let table = table_order(&app, 4);
    assert_eq!(label(&app, table), None);
    // The next takeaway is still the first.
    assert_eq!(label(&app, takeaway(&app)).as_deref(), Some("E-61"));
    // And a name cannot be put on a table.
    assert_eq!(
        do_set_takeaway_customer(&app.state, table, Some("Dupont".into()), None).unwrap_err(),
        errors::TAKEAWAY_CUSTOMER_NOT_TAKEAWAY
    );
}

#[tokio::test]
async fn a_takeaway_that_held_nothing_gives_its_number_back_when_it_was_the_last() {
    let (app, _fdm) = app().await;
    let kept = takeaway(&app);
    add(&app, kept, 1, 1);
    // Opened and left empty: voided, its number returns.
    let empty = takeaway(&app);
    assert_eq!(label(&app, empty).as_deref(), Some("E-62"));
    do_set_takeaway_customer(&app.state, empty, Some("Fantôme".into()), None).unwrap();
    do_leave_order(&app.state, empty, LeaveReason::Navigate)
        .await
        .unwrap();
    assert_eq!(label(&app, empty), None, "its tag went with the order");

    let next = takeaway(&app);
    assert_eq!(label(&app, next).as_deref(), Some("E-62"));
    // The one that held a line keeps its number when left.
    do_leave_order(&app.state, kept, LeaveReason::Navigate)
        .await
        .unwrap();
    assert_eq!(label(&app, kept).as_deref(), Some("E-61"));
    assert_eq!(columns_holding(&app, "Fantôme"), Vec::<String>::new());
}

// ── The setting ────────────────────────────────────────────────────

#[tokio::test]
async fn a_manager_sets_the_first_number_and_it_applies_to_the_next_one() {
    let (app, _fdm) = app().await;
    let first = takeaway(&app);
    assert_eq!(
        do_set_takeaway_numbering(&app.state, numbering(100)).unwrap(),
        numbering(100)
    );
    assert_eq!(
        do_get_takeaway_numbering(&app.state).unwrap(),
        numbering(100)
    );
    let next = takeaway(&app);
    assert_eq!(
        label(&app, first).as_deref(),
        Some("E-61"),
        "never renumbered"
    );
    assert_eq!(label(&app, next).as_deref(), Some("E-100"));
    // Going back down never repeats a number of the day.
    do_set_takeaway_numbering(&app.state, numbering(1)).unwrap();
    assert_eq!(label(&app, takeaway(&app)).as_deref(), Some("E-101"));
    // A new day starts at the first number.
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE takeaway_tags SET day = '2020-01-01'", [])
        .unwrap();
    assert_eq!(label(&app, takeaway(&app)).as_deref(), Some("E-1"));
}

#[tokio::test]
async fn the_first_number_is_between_1_and_999() {
    let (app, _fdm) = app().await;
    for bad in [-5, 0, MAX_FIRST_NUMBER + 1, 10_000] {
        assert_eq!(
            do_set_takeaway_numbering(&app.state, numbering(bad)).unwrap_err(),
            "takeaway_number_invalid|min=1|max=999"
        );
    }
    do_set_takeaway_numbering(&app.state, numbering(MAX_FIRST_NUMBER)).unwrap();
    do_set_takeaway_numbering(&app.state, numbering(1)).unwrap();
}

#[tokio::test]
async fn any_operator_reads_and_only_a_manager_changes_the_first_number() {
    let (app, _fdm) = app().await;
    app.logout();
    assert_eq!(
        do_get_takeaway_numbering(&app.state).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_takeaway_numbering(&app.state, numbering(70)).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as(SERVER, role);
        assert_eq!(
            do_get_takeaway_numbering(&app.state).unwrap(),
            numbering(61)
        );
        assert_eq!(
            do_set_takeaway_numbering(&app.state, numbering(70)).unwrap_err(),
            ERR_FORBIDDEN
        );
    }
}

#[tokio::test]
async fn every_change_of_the_first_number_is_logged_with_the_manager() {
    let (app, _fdm) = app().await;
    do_set_takeaway_numbering(&app.state, numbering(70)).unwrap();
    do_set_takeaway_numbering(&app.state, numbering(70)).unwrap();
    do_set_takeaway_numbering(&app.state, numbering(80)).unwrap();
    let db = app.state.db.lock().unwrap();
    let log = owner_settings::history(&db, KEY_TAKEAWAY_NUMBERING, 10).unwrap();
    assert_eq!(log.len(), 2, "an unchanged value is not logged again");
    assert_eq!(log[0].value_json, r#"{"firstNumber":80}"#);
    assert_eq!(log[0].changed_by.as_deref(), Some(MANAGER));
    assert!(
        db.execute("DELETE FROM owner_settings_log", []).is_err(),
        "append-only"
    );
}

// ── The customer ───────────────────────────────────────────────────

#[tokio::test]
async fn the_name_and_phone_are_set_replaced_as_a_whole_and_cleared() {
    let (app, _fdm) = app().await;
    let order = takeaway(&app);
    let tag = do_set_takeaway_customer(
        &app.state,
        order,
        Some("  Dupont ".into()),
        Some("0470 12 34 56".into()),
    )
    .unwrap();
    assert_eq!(tag.customer_name.as_deref(), Some("Dupont"));
    assert_eq!(tag.customer_phone.as_deref(), Some("0470 12 34 56"));
    assert_eq!(tag.label, "E-61");
    assert_eq!(
        do_get_takeaway_tag(&app.state, order).unwrap().unwrap(),
        tag
    );

    // Both are replaced: the screen sends the one it does not edit.
    let renamed = do_set_takeaway_customer(
        &app.state,
        order,
        Some("Martin".into()),
        Some("0470 12 34 56".into()),
    )
    .unwrap();
    assert_eq!(renamed.customer_name.as_deref(), Some("Martin"));
    assert_eq!(renamed.customer_phone.as_deref(), Some("0470 12 34 56"));
    let no_phone =
        do_set_takeaway_customer(&app.state, order, Some("Martin".into()), None).unwrap();
    assert_eq!(no_phone.customer_phone, None);
    let cleared = do_set_takeaway_customer(&app.state, order, Some("  ".into()), None).unwrap();
    assert_eq!(
        (cleared.customer_name, cleared.customer_phone),
        (None, None)
    );
    assert_eq!(cleared.label, "E-61", "the number stays");
}

#[tokio::test]
async fn a_name_and_a_phone_are_validated() {
    let (app, _fdm) = app().await;
    let order = takeaway(&app);
    assert_eq!(
        do_set_takeaway_customer(&app.state, order, Some("x".repeat(31)), None).unwrap_err(),
        "takeaway_name_invalid|max=30"
    );
    assert_eq!(
        do_set_takeaway_customer(&app.state, order, Some("王小明".into()), None).unwrap_err(),
        "takeaway_name_invalid|max=30"
    );
    assert_eq!(
        do_set_takeaway_customer(&app.state, order, None, Some("appelez-moi".into())).unwrap_err(),
        "takeaway_phone_invalid|max=24"
    );
    assert_eq!(
        do_set_takeaway_customer(&app.state, order, None, Some("12".into())).unwrap_err(),
        "takeaway_phone_invalid|max=24"
    );
    do_set_takeaway_customer(&app.state, order, Some("Hélène Gérard".into()), None).unwrap();
    // A refusal stores nothing.
    let tag = do_get_takeaway_tag(&app.state, order).unwrap().unwrap();
    assert_eq!(tag.customer_name.as_deref(), Some("Hélène Gérard"));
    assert_eq!(tag.customer_phone, None);
}

#[tokio::test]
async fn any_staff_member_sets_the_customer_but_only_while_the_takeaway_is_open() {
    let (app, _fdm) = app().await;
    let order = takeaway(&app);
    app.logout();
    assert_eq!(
        do_set_takeaway_customer(&app.state, order, Some("Dupont".into()), None).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    app.login_as(SERVER, UserRole::Waiter);
    do_set_takeaway_customer(&app.state, order, Some("Dupont".into()), None).unwrap();

    add(&app, order, 1, 1);
    app.login_as(SERVER, UserRole::Cashier);
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        do_set_takeaway_customer(&app.state, order, Some("Martin".into()), None).unwrap_err(),
        errors::ORDER_NOT_OPEN
    );
    // The paid takeaway keeps its tag.
    let tag = do_get_takeaway_tag(&app.state, order).unwrap().unwrap();
    assert_eq!(tag.customer_name.as_deref(), Some("Dupont"));
}

#[tokio::test]
async fn a_takeaway_opened_before_the_numbering_gets_its_number_with_its_name() {
    let (app, _fdm) = app().await;
    let order = takeaway(&app);
    app.state
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM takeaway_tags", [])
        .unwrap();
    assert_eq!(label(&app, order), None);
    let tag = do_set_takeaway_customer(&app.state, order, Some("Dupont".into()), None).unwrap();
    assert_eq!(tag.label, "E-61");
}

#[tokio::test]
async fn an_online_order_keeps_its_platform_reference_and_takes_no_number() {
    let (app, _fdm) = app().await;
    let online = takeaway(&app);
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE orders SET hubrise_order_id = 'HR-1', source = 'hubrise' WHERE id = ?1",
            [online],
        )
        .unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM takeaway_tags", [])
        .unwrap();
    assert_eq!(
        do_set_takeaway_customer(&app.state, online, Some("Dupont".into()), None).unwrap_err(),
        errors::TAKEAWAY_CUSTOMER_NOT_TAKEAWAY
    );
    assert_eq!(label(&app, online), None);
}

// ── Retention ──────────────────────────────────────────────────────

#[tokio::test]
async fn the_name_and_phone_are_cleared_after_72_hours_and_the_number_stays() {
    let (app, _fdm) = app().await;
    let order = takeaway(&app);
    do_set_takeaway_customer(
        &app.state,
        order,
        Some("Dupont".into()),
        Some("0470 12 34 56".into()),
    )
    .unwrap();
    let now = chrono::Utc::now();

    // Fresh: kept.
    assert_eq!(purge_contacts_at(&app.state, now + Duration::hours(71)), 0);
    assert_eq!(
        do_get_takeaway_tag(&app.state, order)
            .unwrap()
            .unwrap()
            .customer_name
            .as_deref(),
        Some("Dupont")
    );
    // Past 72 hours: cleared, once.
    assert_eq!(purge_contacts_at(&app.state, now + Duration::hours(73)), 1);
    assert_eq!(purge_contacts_at(&app.state, now + Duration::hours(74)), 0);
    let tag = do_get_takeaway_tag(&app.state, order).unwrap().unwrap();
    assert_eq!((tag.customer_name, tag.customer_phone), (None, None));
    assert_eq!(tag.label, "E-61");
    assert_eq!(columns_holding(&app, "Dupont"), Vec::<String>::new());
    assert_eq!(columns_holding(&app, "0470 12 34 56"), Vec::<String>::new());
}

// ── The list of the open takeaways ─────────────────────────────────

#[tokio::test]
async fn the_open_takeaways_carry_their_number_and_name() {
    let (app, _fdm) = app().await;
    let (a, b) = (takeaway(&app), takeaway(&app));
    add(&app, a, 1, 1);
    add(&app, b, 2, 2);
    do_set_takeaway_customer(
        &app.state,
        b,
        Some("Dupont".into()),
        Some("0470 12 34 56".into()),
    )
    .unwrap();
    let open = do_list_open_takeaways(&app.state).unwrap();
    assert_eq!(open.len(), 2);
    assert_eq!(open[0].order_id, a);
    assert_eq!(open[0].number, Some(61));
    assert_eq!(open[0].label.as_deref(), Some("E-61"));
    assert_eq!(open[0].customer_name, None);
    assert_eq!(open[1].label.as_deref(), Some("E-62"));
    assert_eq!(open[1].customer_name.as_deref(), Some("Dupont"));
}

// ── The slip ───────────────────────────────────────────────────────

#[tokio::test]
async fn the_kitchen_slip_of_a_takeaway_carries_its_number_and_name_but_not_the_phone() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let order = takeaway(&app);
    add(&app, order, 2, 3);
    do_set_takeaway_customer(
        &app.state,
        order,
        Some("Dupont".into()),
        Some("0470 12 34 56".into()),
    )
    .unwrap();
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    let slips = rendered(&printer.settle_slips(1).await);
    assert_eq!(slips.len(), 1, "{slips:?}");
    for expected in ["Vente directe", "E-61", "Dupont", "3 x Maki saumon"] {
        assert!(slips[0].contains(expected), "{expected:?} in\n{}", slips[0]);
    }
    assert!(
        !slips[0].contains("0470"),
        "no phone on paper: {}",
        slips[0]
    );
    // The number is in large type: the last size command before it is GS ! 0x11.
    let raw = printer.bar_slips().remove(0);
    let at = raw.find("E-61").unwrap();
    let size = raw[..at].rfind("\u{1d}!").unwrap();
    assert_eq!(
        &raw[size..size + 3],
        "\u{1d}!\u{11}",
        "double width and height"
    );
}

#[tokio::test]
async fn a_slip_of_a_takeaway_without_a_name_has_the_number_alone() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let _first = takeaway(&app);
    let order = takeaway(&app);
    add(&app, order, 1, 1);
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    let slips = rendered(&printer.settle_slips(1).await);
    assert!(
        slips[0].contains("Vente directe") && slips[0].contains("E-62"),
        "{}",
        slips[0]
    );
}

#[tokio::test]
async fn the_note_slip_of_a_sent_takeaway_line_names_the_takeaway() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let order = takeaway(&app);
    let line = add(&app, order, 1, 1);
    do_set_takeaway_customer(&app.state, order, Some("Dupont".into()), None).unwrap();
    // A takeaway is not sent before it is paid: the note changes the slip only after.
    do_set_line_note(&app.state, line, Some("sans wasabi".into()))
        .await
        .unwrap();
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    let slips = rendered(&printer.settle_slips(1).await);
    assert!(
        slips[0].contains("E-61") && slips[0].contains("sans wasabi"),
        "{}",
        slips[0]
    );
}

#[tokio::test]
async fn a_table_slip_does_not_change() {
    let (app, _fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let _takeaway = takeaway(&app);
    let table = table_order(&app, 4);
    add(&app, table, 1, 1);
    pos_engine::commands::tables::do_sign_table_order(&app.state, table)
        .await
        .unwrap();
    let slips = rendered(&printer.settle_slips(1).await);
    assert!(slips[0].contains("Table 4"), "{}", slips[0]);
    assert!(!slips[0].contains("E-61") && !slips[0].contains("Vente directe"));
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
async fn the_number_the_name_and_the_phone_are_in_no_fiscal_message_and_no_archived_event() {
    let (app, fdm) = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let (name, phone) = ("Zzdupont", "0499 88 77 66");

    // A takeaway with a name and phone, paid (N), and a table registered,
    // pre-billed and paid while takeaways exist.
    let order = takeaway(&app);
    add(&app, order, 1, 2);
    do_set_takeaway_customer(&app.state, order, Some(name.into()), Some(phone.into())).unwrap();
    assert!(
        do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    let table = table_order(&app, 6);
    add(&app, table, 2, 1);
    pos_engine::commands::tables::do_sign_table_order(&app.state, table)
        .await
        .unwrap();
    do_sign_pre_bill(&app.state, table).await.unwrap();
    assert!(
        do_checkout(&app.state, table, CheckoutPaymentMethod::Cash)
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
        for secret in [name, "0499", "E-61"] {
            assert!(
                !sent.body.contains(secret),
                "{secret} in the {} request",
                sent.op
            );
        }
    }
    for event in app.fiscal_events() {
        let request = event.event_data_json.as_deref().unwrap_or_default();
        let response = event.fdm_response_json.as_deref().unwrap_or_default();
        for secret in [name, "0499", "E-61"] {
            assert!(
                !request.contains(secret) && !response.contains(secret),
                "{secret}"
            );
        }
    }
    // Only the tag table holds the name and the phone.
    assert_eq!(columns_holding(&app, name), ["takeaway_tags.customer_name"]);
    assert_eq!(
        columns_holding(&app, "0499"),
        ["takeaway_tags.customer_phone"]
    );

    // The paper: the slip carries the name, no receipt or pre-bill does.
    for document in printer.rendered() {
        if !document.contains("BON DE COMMANDE") {
            assert!(
                !document.contains(name) && !document.contains("E-61"),
                "{document}"
            );
        }
    }
}
