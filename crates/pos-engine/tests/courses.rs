//! Courses and "Envoyer la suite" (V12-D22 and V12-D23).
//!
//! Kitchen printing only:
//! - switched off (the default), every slip is the one the till always
//!   printed, byte for byte;
//! - switched on, a P prints course 0 and the courses up to the released
//!   one (1 at the start), grouped with headers, and holds the rest;
//! - "Envoyer la suite" signs the unsent lines first, releases the lowest
//!   held course and prints it with "SUITE : <course>"; with the FDM down
//!   and unsent lines nothing is signed, released or printed;
//! - corrections: a printed line prints at once, a held line is netted at
//!   release, a net zero prints nothing;
//! - the course never reaches a signed request, a product name, a
//!   department, the VAT ticket or any archived text.

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::courses::{
    CategoryCourse, CourseSettings, do_get_course_settings, do_list_held_courses,
    do_reprint_course_slip, do_send_next_course, do_set_course_settings, do_set_line_course,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_list_cart_lines,
    do_remove_order_line, do_update_order_line_quantity,
};
use pos_engine::commands::tables::{do_list_tables, do_sign_pre_bill, do_sign_table_order};

const OPERATOR: &str = "12345678958";

const SAKE: i64 = 1;
const EDAMAME: i64 = 2;
const MAKI: i64 = 3;
const MOCHI: i64 = 4;
const WATER: i64 = 5;
const GYOZA: i64 = 6;

/// Categories: 1 drinks (course 0), 2 starters (1), 3 mains (2), 4
/// desserts (3). Water is served at the counter, off every slip.
async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1), (3, 2, 1), (4, 3, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
            (1, 1, 650, 'A', 1, 0, 1),
            (2, 2, 450, 'B', 1, 1, 1),
            (3, 3, 1200, 'B', 1, 2, 1),
            (4, 4, 500, 'B', 1, 3, 1),
            (5, 1, 300, 'A', 0, 4, 1),
            (6, 3, 900, 'B', 1, 5, 1);
         INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
            ('menu_item', 1, 'name', 'fr', 'Sake'),
            ('menu_item', 2, 'name', 'fr', 'Edamame'),
            ('menu_item', 3, 'name', 'fr', 'Maki saumon'),
            ('menu_item', 4, 'name', 'fr', 'Mochi'),
            ('menu_item', 5, 'name', 'fr', 'Eau plate'),
            ('menu_item', 6, 'name', 'fr', 'Gyoza');",
    )
    .unwrap();
    drop(conn);
    app
}

fn enable(app: &TestApp) {
    do_set_course_settings(
        &app.state,
        CourseSettings {
            enabled: true,
            category_courses: vec![
                CategoryCourse {
                    category_id: 2,
                    course: 1,
                },
                CategoryCourse {
                    category_id: 3,
                    course: 2,
                },
                CategoryCourse {
                    category_id: 4,
                    course: 3,
                },
            ],
        },
    )
    .unwrap();
}

fn table_order(app: &TestApp, table: i32) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(table),
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id
}

fn takeaway(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: OPERATOR.into(),
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

fn fdm_down(app: &TestApp) -> String {
    let mut config = app.state.config.lock().unwrap();
    std::mem::replace(&mut config.fdm_url, "http://127.0.0.1:9/graphql".into())
}

fn fdm_up(app: &TestApp, url: String) {
    app.state.config.lock().unwrap().fdm_url = url;
}

fn recorded(app: &TestApp, order: i64) -> i64 {
    let conn = app.state.db.lock().unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM kitchen_slip_entries k JOIN order_entries e ON e.id = k.entry_id
         WHERE e.order_id = ?1",
        [order],
        |r| r.get(0),
    )
    .unwrap()
}

/// The first `dd/mm/yyyy hh:mm:ss` of a slip.
fn slip_date(slip: &str) -> String {
    let b = slip.as_bytes();
    let shape = b"dd/dd/dddd dd:dd:dd";
    (0..b.len().saturating_sub(shape.len()))
        .find(|&i| {
            shape.iter().enumerate().all(|(k, &s)| {
                if s == b'd' {
                    b[i + k].is_ascii_digit()
                } else {
                    b[i + k] == s
                }
            })
        })
        .map(|i| slip[i..i + shape.len()].to_string())
        .expect("a date on the slip")
}

fn legacy_item(quantity: i32, name: &str) -> pos_print::bar_ticket::BarItem {
    pos_print::bar_ticket::BarItem {
        quantity,
        name: name.into(),
        course: None,
    }
}

// ── Settings ───────────────────────────────────────────────────

#[tokio::test]
async fn courses_are_off_by_default_and_settings_are_validated_and_logged() {
    let app = app().await;
    let defaults = do_get_course_settings(&app.state).unwrap();
    assert!(!defaults.enabled && defaults.category_courses.is_empty());

    let set = |courses: Vec<(i64, i32)>| {
        do_set_course_settings(
            &app.state,
            CourseSettings {
                enabled: true,
                category_courses: courses
                    .into_iter()
                    .map(|(category_id, course)| CategoryCourse {
                        category_id,
                        course,
                    })
                    .collect(),
            },
        )
    };
    assert!(set(vec![(2, 4)]).unwrap_err().starts_with("course_invalid"));
    assert!(
        set(vec![(2, -1)])
            .unwrap_err()
            .starts_with("course_invalid")
    );
    assert_eq!(
        set(vec![(99, 1)]).unwrap_err(),
        "course_invalid|category=99"
    );
    assert_eq!(
        set(vec![(2, 1), (2, 2)]).unwrap_err(),
        "course_invalid|category=2"
    );
    assert_eq!(do_get_course_settings(&app.state).unwrap(), defaults);

    // Course 0 is the default of an unmapped category: it is not stored.
    let stored = set(vec![(4, 3), (1, 0), (2, 1)]).unwrap();
    assert!(stored.enabled);
    assert_eq!(
        stored
            .category_courses
            .iter()
            .map(|c| (c.category_id, c.course))
            .collect::<Vec<_>>(),
        vec![(2, 1), (4, 3)]
    );
    assert_eq!(do_get_course_settings(&app.state).unwrap(), stored);
    // The same document again writes no log row.
    set(vec![(2, 1), (4, 3)]).unwrap();
    let conn = app.state.db.lock().unwrap();
    let logged: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM owner_settings_log WHERE key = 'courses'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(logged, 1);
}

#[tokio::test]
async fn a_line_takes_the_course_of_its_category_and_none_when_off() {
    let app = app().await;
    let o = table_order(&app, 1);
    let before = add(&app, o, MAKI, 1);
    enable(&app);
    let sake = add(&app, o, SAKE, 1);
    let edamame = add(&app, o, EDAMAME, 1);
    let maki = add(&app, o, MAKI, 1);
    let mochi = add(&app, o, MOCHI, 1);
    let course = |id: i64| {
        do_list_cart_lines(&app.state, o)
            .unwrap()
            .into_iter()
            .find(|l| l.line.id == id)
            .map(|l| (l.course, l.kitchen_state))
            .unwrap()
    };
    // A line added while the switch was off is "Direct".
    assert_eq!(course(before), (0, "unsent".to_string()));
    assert_eq!(course(sake), (0, "unsent".to_string()));
    assert_eq!(course(edamame), (1, "unsent".to_string()));
    assert_eq!(course(maki), (2, "held".to_string()));
    assert_eq!(course(mochi), (3, "held".to_string()));

    // Switched off again: the screen shows no course and nothing held.
    do_set_course_settings(&app.state, CourseSettings::default()).unwrap();
    assert_eq!(course(mochi), (0, "unsent".to_string()));
    assert!(do_list_held_courses(&app.state, o).unwrap().is_empty());
}

// ── Off: the slip of v1.0.0 ────────────────────────────────────

#[tokio::test]
async fn off_the_slip_is_byte_identical_to_the_one_before_courses() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 4);
    add(&app, o, SAKE, 2);
    add(&app, o, MAKI, 1);
    add(&app, o, MOCHI, 1);
    add(&app, o, WATER, 1);
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    assert_eq!(slips.len(), 1);
    let expected = pos_print::bar_ticket::format_bar_ticket(&pos_print::bar_ticket::BarTicket {
        date_time: slip_date(&slips[0]),
        operator: format!("Test {OPERATOR}"),
        table: Some(4),
        items: vec![
            legacy_item(2, "Sake"),
            legacy_item(1, "Maki saumon"),
            legacy_item(1, "Mochi"),
        ],
        lang: pos_fiscal::ticket::ReceiptLanguage::Fr,
        is_training: false,
        release: None,
        duplicate: false,
    });
    assert!(
        slips[0].starts_with(&*String::from_utf8_lossy(&expected)),
        "{}",
        slips[0]
    );
    assert!(!slips[0].contains("-- ") && !slips[0].contains("SUITE"));
    // The entries are recorded all the same, for a later switch-on.
    assert_eq!(recorded(&app, o), 3);

    // A failed slip is not retried while the switch is off (as before).
    let o2 = table_order(&app, 5);
    app.set_printer(DEAD_PRINTER);
    add(&app, o2, SAKE, 1);
    assert_eq!(
        do_sign_table_order(&app.state, o2).await.unwrap(),
        SlipOutcome::Failed
    );
    app.set_printer(&printer.addr);
    assert_eq!(
        do_sign_table_order(&app.state, o2).await.unwrap(),
        SlipOutcome::NothingToPrint
    );
    assert_eq!(recorded(&app, o2), 0);
    assert!(
        do_send_next_course(&app.state, o2)
            .await
            .unwrap_err()
            .starts_with("courses_disabled")
    );
}

#[tokio::test]
async fn on_with_only_direct_lines_the_slip_is_the_plain_one() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o2 = table_order(&app, 6);
    add(&app, o2, SAKE, 1);
    add(&app, o2, WATER, 1);
    do_sign_table_order(&app.state, o2).await.unwrap();
    let slips = printer.settle_slips(1).await;
    let last = &slips[0];
    let expected = pos_print::bar_ticket::format_bar_ticket(&pos_print::bar_ticket::BarTicket {
        date_time: slip_date(last),
        operator: format!("Test {OPERATOR}"),
        table: Some(6),
        items: vec![legacy_item(1, "Sake")],
        lang: pos_fiscal::ticket::ReceiptLanguage::Fr,
        is_training: false,
        release: None,
        duplicate: false,
    });
    assert!(
        last.starts_with(&*String::from_utf8_lossy(&expected)),
        "{last}"
    );
}

// ── The first slip and the release ─────────────────────────────

#[tokio::test]
async fn a_p_prints_course_0_and_1_and_holds_2_and_3_until_the_release() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 3);
    add(&app, o, SAKE, 2);
    add(&app, o, EDAMAME, 1);
    add(&app, o, MAKI, 3);
    add(&app, o, MOCHI, 1);
    add(&app, o, WATER, 1);

    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    let first = &slips[0];
    for expected in [
        "Table 3",
        "-- Direct --",
        "2 x Sake",
        "-- Entrees --",
        "1 x Edamame",
        "CECI N'EST PAS UN TICKET DE CAISSE",
    ] {
        assert!(first.contains(expected), "{expected:?} in {first}");
    }
    for absent in ["Maki", "Mochi", "-- Plats --", "SUITE", "Eau", "DUPLICATA"] {
        assert!(!first.contains(absent), "{absent:?} in {first}");
    }
    let d = first.find("-- Direct --").unwrap();
    assert!(d < first.find("2 x Sake").unwrap());
    assert!(first.find("2 x Sake").unwrap() < first.find("-- Entrees --").unwrap());

    // What waits, signed.
    let held = do_list_held_courses(&app.state, o).unwrap();
    assert_eq!(
        held.iter()
            .map(|h| (h.course, h.lines, h.quantity))
            .collect::<Vec<_>>(),
        vec![(2, 1, 3), (3, 1, 1)]
    );
    let states: Vec<_> = do_list_cart_lines(&app.state, o)
        .unwrap()
        .iter()
        .map(|l| (l.line.menu_item_id, l.kitchen_state.clone()))
        .collect();
    assert_eq!(
        states,
        vec![
            (SAKE, "printed".into()),
            (EDAMAME, "printed".into()),
            (MAKI, "held".into()),
            (MOCHI, "held".into()),
            (WATER, "unsent".into()),
        ]
    );
    let table = do_list_tables(&app.state, 3).unwrap();
    assert_eq!(table[2].held_course, Some(2));
    assert_eq!(recorded(&app, o), 2);

    // Release course 2, then 3.
    let events = app.fiscal_events().len();
    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(
        (next.course, next.signed, next.slip),
        (Some(2), None, SlipOutcome::Printed)
    );
    assert_eq!(
        app.fiscal_events().len(),
        events,
        "nothing to sign: no new event"
    );
    let slips = printer.settle_slips(2).await;
    let second = &slips[1];
    for expected in [
        "BON DE COMMANDE",
        "Table 3",
        "SUITE : Plats",
        "-- Plats --",
        "3 x Maki saumon",
        "CECI N'EST PAS UN TICKET DE CAISSE",
    ] {
        assert!(second.contains(expected), "{expected:?} in {second}");
    }
    assert!(
        !second.contains("Mochi") && !second.contains("Sake"),
        "{second}"
    );
    assert!(!second.contains('€') && !second.contains("EUR"), "{second}");
    assert_eq!(
        do_list_tables(&app.state, 3).unwrap()[2].held_course,
        Some(3)
    );

    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(next.course, Some(3));
    let slips = printer.settle_slips(3).await;
    assert!(
        slips[2].contains("SUITE : Desserts") && slips[2].contains("1 x Mochi"),
        "{}",
        slips[2]
    );
    assert!(do_list_held_courses(&app.state, o).unwrap().is_empty());
    assert_eq!(do_list_tables(&app.state, 3).unwrap()[2].held_course, None);
    assert_eq!(
        do_send_next_course(&app.state, o).await.unwrap_err(),
        "no_course_held"
    );
    assert_eq!(recorded(&app, o), 4);
}

#[tokio::test]
async fn a_release_signs_the_unsent_lines_first() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, SAKE, 1);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    // New lines, not signed: a main and a drink.
    add(&app, o, GYOZA, 2);
    add(&app, o, SAKE, 1);
    let events = app.fiscal_events().len();

    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(next.course, Some(2));
    // The P printed the new drink; the main waited for the release.
    assert_eq!(next.signed, Some(SlipOutcome::Printed));
    assert_eq!(app.fiscal_events().len(), events + 1);
    assert_eq!(app.fiscal_events().last().unwrap().event_type, "P");
    let slips = printer.settle_slips(3).await;
    assert_eq!(slips.len(), 3, "{slips:?}");
    assert!(
        slips[1].contains("1 x Sake") && !slips[1].contains("Gyoza"),
        "{}",
        slips[1]
    );
    assert!(slips[2].contains("SUITE : Plats"));
    assert!(
        slips[2].contains("1 x Maki saumon") && slips[2].contains("2 x Gyoza"),
        "{}",
        slips[2]
    );
    // Nothing remains unsent.
    assert!(
        do_list_cart_lines(&app.state, o)
            .unwrap()
            .iter()
            .all(|l| l.unsent_quantity == 0)
    );
}

#[tokio::test]
async fn with_the_fdm_down_a_release_with_unsent_lines_signs_prints_and_releases_nothing() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    add(&app, o, GYOZA, 1);
    let events = app.fiscal_events().len();
    let url = fdm_down(&app);

    assert!(do_send_next_course(&app.state, o).await.is_err());
    assert_eq!(app.fiscal_events().len(), events);
    assert_eq!(printer.settle_slips(1).await.len(), 0, "nothing printed");
    assert_eq!(do_list_held_courses(&app.state, o).unwrap()[0].course, 2);
    assert_eq!(recorded(&app, o), 0);

    fdm_up(&app, url);
    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(next.course, Some(2));
    let slips = printer.settle_slips(1).await;
    let release = slips.iter().find(|s| s.contains("SUITE : Plats")).unwrap();
    assert!(
        release.contains("Maki saumon") && release.contains("Gyoza"),
        "{release}"
    );
}

#[tokio::test]
async fn an_already_signed_course_prints_without_the_fdm() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, MAKI, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    let events = app.fiscal_events().len();
    let _url = fdm_down(&app);

    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(
        (next.course, next.signed, next.slip),
        (Some(2), None, SlipOutcome::Printed)
    );
    assert_eq!(app.fiscal_events().len(), events);
    let slips = printer.settle_slips(1).await;
    assert!(
        slips[0].contains("SUITE : Plats") && slips[0].contains("2 x Maki saumon"),
        "{}",
        slips[0]
    );
}

// ── Corrections ────────────────────────────────────────────────

#[tokio::test]
async fn a_correction_of_a_printed_line_prints_at_once() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    let sake = add(&app, o, SAKE, 2);
    let edamame = add(&app, o, EDAMAME, 2);
    do_sign_table_order(&app.state, o).await.unwrap();

    do_update_order_line_quantity(&app.state, sake, 1).unwrap();
    do_remove_order_line(&app.state, edamame).unwrap();
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(2).await;
    assert!(
        slips[1].contains("-1 x Sake") && slips[1].contains("-2 x Edamame"),
        "{}",
        slips[1]
    );
}

#[tokio::test]
async fn a_correction_of_a_held_line_is_netted_at_the_release() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, SAKE, 1);
    let maki = add(&app, o, MAKI, 3);
    let gyoza = add(&app, o, GYOZA, 1);
    let mochi = add(&app, o, MOCHI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    // Maki 3 -> 1, Gyoza removed, Mochi removed: held lines, no slip.
    do_update_order_line_quantity(&app.state, maki, 1).unwrap();
    do_remove_order_line(&app.state, gyoza).unwrap();
    do_remove_order_line(&app.state, mochi).unwrap();
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::NothingToPrint
    );
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!((next.course, next.slip), (Some(2), SlipOutcome::Printed));
    let slips = printer.settle_slips(2).await;
    let release = &slips[1];
    assert!(release.contains("1 x Maki saumon"), "{release}");
    assert!(
        !release.contains("3 x") && !release.contains("-2 x"),
        "{release}"
    );
    assert!(
        !release.contains("Gyoza") && !release.contains("Mochi"),
        "{release}"
    );
    // The cancelled lines are done with: nothing is held, nothing to retry.
    assert!(do_list_held_courses(&app.state, o).unwrap().is_empty());
    assert_eq!(
        do_send_next_course(&app.state, o).await.unwrap_err(),
        "no_course_held"
    );
}

#[tokio::test]
async fn a_course_that_nets_to_zero_prints_nothing() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, SAKE, 1);
    let maki = add(&app, o, MAKI, 2);
    add(&app, o, MOCHI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_remove_order_line(&app.state, maki).unwrap();
    do_sign_table_order(&app.state, o).await.unwrap();

    // Course 2 has no line left: the next held course is the dessert.
    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(next.course, Some(3));
    let slips = printer.settle_slips(2).await;
    assert!(!slips[1].contains("Maki"), "{}", slips[1]);
    assert!(slips[1].contains("SUITE : Desserts") && slips[1].contains("1 x Mochi"));
    // The pair of entries of the removed line was consumed, never printed.
    assert_eq!(recorded(&app, o), 4);
}

// ── Changing a course ──────────────────────────────────────────

#[tokio::test]
async fn a_course_changes_until_a_slip_carried_the_line() {
    let app = app().await;
    let o = table_order(&app, 2);
    let sake = add(&app, o, SAKE, 1);
    assert_eq!(
        do_set_line_course(&app.state, sake, 1).unwrap_err(),
        "courses_disabled"
    );
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let maki = add(&app, o, MAKI, 1);
    let edamame = add(&app, o, EDAMAME, 1);

    assert!(
        do_set_line_course(&app.state, maki, 4)
            .unwrap_err()
            .starts_with("course_invalid")
    );
    assert!(do_set_line_course(&app.state, 9999, 1).is_err());
    do_set_line_course(&app.state, maki, 3).unwrap();
    do_set_line_course(&app.state, maki, 2).unwrap();
    do_sign_table_order(&app.state, o).await.unwrap();

    // Edamame (course 1) went out; Maki (course 2) is held.
    assert_eq!(
        do_set_line_course(&app.state, edamame, 2).unwrap_err(),
        "course_already_sent"
    );
    // A held, signed line may still move: to course 1 it prints at the
    // next registration, even with nothing new to sign.
    do_set_line_course(&app.state, maki, 1).unwrap();
    let events = app.fiscal_events().len();
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    assert_eq!(app.fiscal_events().len(), events, "no new P");
    let slips = printer.settle_slips(2).await;
    assert!(
        slips[1].contains("1 x Maki saumon") && !slips[1].contains("SUITE"),
        "{}",
        slips[1]
    );
    assert_eq!(
        do_set_line_course(&app.state, maki, 3).unwrap_err(),
        "course_already_sent"
    );
}

// ── Direct sale, checkout ──────────────────────────────────────

#[tokio::test]
async fn a_direct_sale_prints_every_course_at_once_with_headers() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = takeaway(&app);
    add(&app, o, SAKE, 1);
    add(&app, o, EDAMAME, 1);
    add(&app, o, MAKI, 1);
    add(&app, o, MOCHI, 1);
    // A takeaway holds nothing.
    assert!(do_list_held_courses(&app.state, o).unwrap().is_empty());
    assert!(
        do_list_cart_lines(&app.state, o)
            .unwrap()
            .iter()
            .all(|l| l.kitchen_state == "unsent")
    );
    assert_eq!(
        do_send_next_course(&app.state, o).await.unwrap_err(),
        "no_course_held"
    );

    let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(r.kitchen_slip, Some(SlipOutcome::Printed));
    let slips = printer.settle_slips(1).await;
    let slip = &slips[0];
    for expected in [
        "Vente directe",
        "-- Direct --",
        "-- Entrees --",
        "-- Plats --",
        "-- Desserts --",
        "1 x Mochi",
    ] {
        assert!(slip.contains(expected), "{expected:?} in {slip}");
    }
    assert!(!slip.contains("SUITE"), "{slip}");
    assert_eq!(recorded(&app, o), 4);
}

#[tokio::test]
async fn a_sale_that_closes_a_table_does_not_print_the_held_courses() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 7);
    add(&app, o, SAKE, 1);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    // The UIs warn from this list before the checkout.
    assert_eq!(do_list_held_courses(&app.state, o).unwrap()[0].course, 2);
    add(&app, o, EDAMAME, 1);
    let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let slips = printer.settle_slips(2).await;
    assert!(
        slips[1].contains("1 x Edamame") && !slips[1].contains("Maki"),
        "{}",
        slips[1]
    );
    assert_eq!(recorded(&app, o), 2, "the held Maki never printed");
}

// ── Printer trouble, reprint, training ─────────────────────────

#[tokio::test]
async fn a_failed_printer_leaves_the_entries_unprinted_and_a_retry_prints_them() {
    let app = app().await;
    enable(&app);
    app.set_printer(DEAD_PRINTER);
    let o = table_order(&app, 2);
    add(&app, o, SAKE, 1);
    add(&app, o, MAKI, 1);
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Failed
    );
    assert_eq!(recorded(&app, o), 0);
    // The line did not go out: its course can still be changed.
    let sake = do_list_cart_lines(&app.state, o).unwrap()[0].line.id;
    do_set_line_course(&app.state, sake, 1).unwrap();
    do_set_line_course(&app.state, sake, 0).unwrap();

    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    assert!(
        slips[0].contains("1 x Sake") && !slips[0].contains("Maki"),
        "{}",
        slips[0]
    );
    assert_eq!(recorded(&app, o), 1);

    // A release whose slip failed is retried by "Envoyer la suite".
    app.set_printer(DEAD_PRINTER);
    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!((next.course, next.slip), (Some(2), SlipOutcome::Failed));
    assert!(do_list_held_courses(&app.state, o).unwrap().is_empty());
    app.set_printer(&printer.addr);
    let retry = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!(
        (retry.course, retry.signed, retry.slip),
        (None, None, SlipOutcome::Printed)
    );
    let slips = printer.settle_slips(2).await;
    assert!(slips[1].contains("1 x Maki saumon"), "{}", slips[1]);
    assert_eq!(
        do_send_next_course(&app.state, o).await.unwrap_err(),
        "no_course_held"
    );
}

#[tokio::test]
async fn a_reprint_says_duplicata_and_records_nothing() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, EDAMAME, 2);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();

    assert_eq!(
        do_reprint_course_slip(&app.state, o, 2).await.unwrap_err(),
        "nothing_printed"
    );
    assert!(
        do_reprint_course_slip(&app.state, o, 7)
            .await
            .unwrap_err()
            .starts_with("course_invalid")
    );
    let before = recorded(&app, o);
    assert_eq!(
        do_reprint_course_slip(&app.state, o, 1).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(2).await;
    assert!(
        slips[1].contains("DUPLICATA") && slips[1].contains("2 x Edamame"),
        "{}",
        slips[1]
    );
    assert!(!slips[1].contains("SUITE"), "{}", slips[1]);

    do_send_next_course(&app.state, o).await.unwrap();
    do_reprint_course_slip(&app.state, o, 2).await.unwrap();
    let slips = printer.settle_slips(4).await;
    assert!(
        slips[3].contains("DUPLICATA")
            && slips[3].contains("SUITE : Plats")
            && slips[3].contains("1 x Maki saumon"),
        "{}",
        slips[3]
    );
    assert_eq!(recorded(&app, o), before + 1);
}

#[tokio::test]
async fn training_slips_keep_the_training_mark() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    app.state.config.lock().unwrap().training_mode = true;
    let o = table_order(&app, 2);
    add(&app, o, EDAMAME, 1);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_send_next_course(&app.state, o).await.unwrap();
    let slips = printer.settle_slips(2).await;
    assert!(slips.iter().all(|s| s.contains("TRAINING")), "{slips:?}");
    assert!(slips[1].contains("SUITE : Plats"));
}

#[tokio::test]
async fn the_slip_follows_the_ticket_language() {
    let app = app().await;
    enable(&app);
    app.state.config.lock().unwrap().language = "nl".into();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 2);
    add(&app, o, EDAMAME, 1);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_send_next_course(&app.state, o).await.unwrap();
    let slips = printer.settle_slips(2).await;
    assert!(slips[0].contains("-- Voorgerechten --"), "{}", slips[0]);
    assert!(
        slips[1].contains("VOLGENDE : Hoofdgerechten"),
        "{}",
        slips[1]
    );
}

// ── The course stays out of the fiscal data ────────────────────

/// Every text the till archives or prints for fiscal purposes, as one
/// string: the signed requests and answers, the pending requests, the
/// ticket deliveries, the order lines and entries, the VAT tickets.
fn fiscal_text(app: &TestApp, printer: &PrinterSink) -> String {
    let conn = app.state.db.lock().unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let kitchen_only = [
        "order_line_courses",
        "order_course_releases",
        "kitchen_slip_entries",
        "owner_settings",
        "owner_settings_log",
    ];
    let mut text = String::new();
    for table in tables
        .iter()
        .filter(|t| !t.contains("migration"))
        .filter(|t| !kitchen_only.contains(&t.as_str()))
    {
        let columns: Vec<String> = conn
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let select = columns
            .iter()
            .map(|c| format!("COALESCE(CAST(\"{c}\" AS TEXT), '')"))
            .collect::<Vec<_>>()
            .join(" || ' ' || ");
        let mut stmt = conn
            .prepare(&format!("SELECT {select} FROM \"{table}\""))
            .unwrap();
        for row in stmt.query_map([], |r| r.get::<_, String>(0)).unwrap() {
            text.push_str(&row.unwrap());
            text.push('\n');
        }
    }
    for receipt in printer.receipts() {
        text.push_str(&receipt);
    }
    text
}

#[tokio::test]
async fn the_course_never_reaches_a_signed_request_a_name_or_the_vat_ticket() {
    let app = app().await;
    enable(&app);
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    // A P, a pre-bill, a release, a table sale, a direct sale, a training P.
    let o = table_order(&app, 2);
    add(&app, o, SAKE, 1);
    add(&app, o, EDAMAME, 1);
    add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    do_sign_pre_bill(&app.state, o).await.unwrap();
    do_send_next_course(&app.state, o).await.unwrap();
    add(&app, o, MOCHI, 1);
    let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let d = takeaway(&app);
    add(&app, d, MAKI, 1);
    add(&app, d, MOCHI, 1);
    let r = do_checkout(&app.state, d, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    app.state.config.lock().unwrap().training_mode = true;
    let t = table_order(&app, 3);
    add(&app, t, EDAMAME, 1);
    do_sign_table_order(&app.state, t).await.unwrap();
    let r = do_checkout(&app.state, t, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    let kinds: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    for needed in ["P", "N", "T"] {
        assert!(kinds.iter().any(|k| k == needed), "{kinds:?}");
    }
    let text = fiscal_text(&app, &printer).to_lowercase();
    for needle in [
        "course",
        "suite",
        "entrees",
        "entrées",
        "desserts",
        "plats",
        "direct",
        "held",
        "release",
        "duplicata",
    ] {
        let at = text.find(needle);
        assert!(
            at.is_none(),
            "{needle:?} found in the archived text: {:?}",
            at.map(|i| &text[i.saturating_sub(60)..(i + 60).min(text.len())])
        );
    }
    // The kitchen tables are the only place.
    let conn = app.state.db.lock().unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM order_line_courses", [], |r| r.get(0))
        .unwrap();
    assert!(rows > 0);
}

#[tokio::test]
async fn the_signed_requests_are_the_same_with_courses_on_or_off() {
    async fn run(on: bool) -> Vec<(String, String)> {
        let app = app().await;
        if on {
            enable(&app);
        }
        let printer = PrinterSink::start().await;
        app.set_printer(&printer.addr);
        let o = table_order(&app, 2);
        add(&app, o, SAKE, 2);
        add(&app, o, EDAMAME, 1);
        add(&app, o, MAKI, 1);
        do_sign_table_order(&app.state, o).await.unwrap();
        if on {
            do_send_next_course(&app.state, o).await.unwrap();
        }
        let r = do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
            .await
            .unwrap();
        assert!(r.success, "{:?}", r.error);
        app.fiscal_events()
            .into_iter()
            .map(|e| {
                (
                    e.event_type,
                    strip_volatile(e.event_data_json.as_deref().unwrap_or("{}")),
                )
            })
            .collect()
    }
    // Same shape, event by event, whatever the switch says.
    assert_eq!(run(false).await, run(true).await);
}

/// The request without what changes from one run to the next.
fn strip_volatile(json: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
    fn walk(v: &mut serde_json::Value) {
        match v {
            serde_json::Value::Object(map) => {
                map.retain(|k, _| {
                    !matches!(
                        k.as_str(),
                        "dateTime"
                            | "date"
                            | "time"
                            | "createdAt"
                            | "posDateTime"
                            | "bookingPeriodId"
                    )
                });
                map.values_mut().for_each(walk);
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(walk),
            _ => {}
        }
    }
    walk(&mut value);
    value.to_string()
}
