//! Tests of the courses facade (engine module courses): `get_course_settings`,
//! `set_course_settings`, `set_line_course`, `list_held_courses`, `send_next_course`,
//! `reprint_course_slip`.
//!
//! (a) Contract: the FFI records are the engine's field by field; an unknown key is a contract
//! error; the cart and table records carry the course fields.
//!
//! (b) Wiring on the seeded engine: the setting round trip and its refusals, a main dish held
//! at the first slip, released by "Envoyer la suite", reprinted.

use pos_engine::commands::courses as engine;
use pos_ffi::records::common::{SlipOutcome, contract_inp, contract_out};
use pos_ffi::records::courses::{CategoryCourse, CourseSettings, HeldCourse, NextCourse};
use pos_ffi::records::orders::{AddOrderLineInput, CreateOrderInput, OrderType};
use pos_ffi::{PosEngine, PosError};
use serde_json::json;

use crate::support::{MANAGER_INSZ, MANAGER_PIN};

/// Carpaccio Beef of the seed (category 20, starters) and Burger of the Chef (category 22,
/// mains); both go to the kitchen. Dry Martini (category 10) does not.
const CARPACCIO: i64 = 20001;
const BURGER: i64 = 22001;

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn settings(enabled: bool) -> CourseSettings {
    CourseSettings {
        enabled,
        category_courses: vec![
            CategoryCourse {
                category_id: 20,
                course: 1,
            },
            CategoryCourse {
                category_id: 22,
                course: 2,
            },
        ],
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_records_are_the_engines_field_by_field() {
    let input: engine::CourseSettings = contract_inp(&settings(true)).unwrap();
    assert!(input.enabled);
    assert_eq!(input.category_courses[1].category_id, 22);
    assert_eq!(input.category_courses[1].course, 2);

    let back: CourseSettings = contract_out(&input).unwrap();
    assert_eq!(back, settings(true));

    let held: HeldCourse = contract_out(&engine::HeldCourseView {
        course: 3,
        lines: 2,
        quantity: 5,
    })
    .unwrap();
    assert_eq!(
        held,
        HeldCourse {
            course: 3,
            lines: 2,
            quantity: 5
        }
    );

    let next: NextCourse = contract_out(&engine::NextCourse {
        course: Some(2),
        signed: Some(pos_engine::commands::bar_slip::SlipOutcome::NothingToPrint),
        slip: pos_engine::commands::bar_slip::SlipOutcome::Printed,
    })
    .unwrap();
    assert_eq!(next.course, Some(2));
    assert_eq!(next.signed, Some(SlipOutcome::NothingToPrint));
    assert_eq!(next.slip, SlipOutcome::Printed);
}

#[test]
fn an_unknown_key_is_a_contract_error() {
    let mut value = serde_json::to_value(settings(true)).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<CourseSettings>(value).is_err());
    let off: CourseSettings = serde_json::from_value(json!({})).unwrap();
    assert!(!off.enabled && off.category_courses.is_empty());
}

// ───────────────────────────── (b) wiring ─────────────────────────────

async fn manager(e: &PosEngine) {
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse_and_a_cashier_does_not_set() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.get_course_settings().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_course_settings(settings(true)).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_line_course(1, 1).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.list_held_courses(1).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.send_next_course(1).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.reprint_course_slip(1, 1).await.unwrap_err()).0,
        "not_logged_in"
    );
    e.login("75061189702".into(), Some("4444".into()))
        .await
        .unwrap();
    assert!(!e.get_course_settings().await.unwrap().enabled);
    assert_eq!(
        coded(&e.set_course_settings(settings(true)).await.unwrap_err()).0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_setting_round_trips_and_its_refusals_are_coded() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;
    let stored = e.set_course_settings(settings(true)).await.unwrap();
    assert_eq!(stored, settings(true));
    assert_eq!(e.get_course_settings().await.unwrap(), settings(true));

    let mut bad = settings(true);
    bad.category_courses[0].course = 4;
    let err = e.set_course_settings(bad).await.unwrap_err();
    assert_eq!(coded(&err).0, "course_invalid");
    let mut unknown = settings(true);
    unknown.category_courses[0].category_id = 987_654;
    let err = e.set_course_settings(unknown).await.unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "course_invalid");
    assert_eq!(params["category"], "987654");
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_main_dish_is_held_released_and_reprinted() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;
    assert_eq!(
        coded(&e.set_line_course(1, 1).await.unwrap_err()).0,
        "courses_disabled"
    );
    e.set_course_settings(settings(true)).await.unwrap();

    let order = e
        .create_order(CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(3),
            user_insz: MANAGER_INSZ.into(),
        })
        .await
        .unwrap();
    let add = |item: i64| AddOrderLineInput {
        order_id: order.id,
        menu_item_id: item,
        quantity: 1,
        modifiers_json: None,
        unit_price_cents: None,
    };
    let starter = e.add_order_line(add(CARPACCIO)).await.unwrap();
    let main = e.add_order_line(add(BURGER)).await.unwrap();

    let cart = e.list_cart_lines(order.id).await.unwrap();
    assert_eq!(
        (cart[0].course, cart[0].kitchen_state.as_str()),
        (1, "unsent")
    );
    assert_eq!(
        (cart[1].course, cart[1].kitchen_state.as_str()),
        (2, "held")
    );
    assert_eq!(
        e.list_held_courses(order.id).await.unwrap(),
        vec![HeldCourse {
            course: 2,
            lines: 1,
            quantity: 1
        }]
    );
    let tables = e.list_tables().await.unwrap();
    assert_eq!(tables[2].held_course, Some(2));

    assert_eq!(
        e.sign_table_order(order.id).await.unwrap(),
        SlipOutcome::Printed
    );
    assert_eq!(
        coded(&e.set_line_course(starter.id, 3).await.unwrap_err()).0,
        "course_already_sent"
    );
    assert_eq!(
        coded(&e.set_line_course(main.id, 9).await.unwrap_err()).0,
        "course_invalid"
    );
    assert_eq!(
        coded(&e.reprint_course_slip(order.id, 2).await.unwrap_err()).0,
        "nothing_printed"
    );

    let next = e.send_next_course(order.id).await.unwrap();
    assert_eq!(
        next,
        NextCourse {
            course: Some(2),
            signed: None,
            slip: SlipOutcome::Printed
        }
    );
    assert!(e.list_held_courses(order.id).await.unwrap().is_empty());
    assert_eq!(e.list_tables().await.unwrap()[2].held_course, None);
    assert_eq!(
        coded(&e.send_next_course(order.id).await.unwrap_err()).0,
        "no_course_held"
    );
    assert_eq!(
        e.reprint_course_slip(order.id, 2).await.unwrap(),
        SlipOutcome::Printed
    );
    e.shutdown().await.unwrap();
}
