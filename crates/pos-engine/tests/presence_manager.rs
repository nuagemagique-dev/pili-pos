//! A manager registers the presence of another employee (kitchen staff
//! who never use the till, or anyone who forgot): `clock_in_employee`
//! and `clock_out_employee`.
//!
//! - the S event is the employee's (`employeeId`, archived `user_insz`),
//!   the manager is kept in `staff_shifts.started_by` / `ended_by`;
//! - every refusal signs nothing and moves no counter;
//! - an FDM failure archives nothing and leaves the local index;
//! - training: the manager commands are refused, and a training (T)
//!   event never touches `staff_shifts`;
//! - a presence-only employee cannot log in, choose a PIN, end a shift
//!   or have a PIN set; an employee on shift cannot be deactivated;
//! - the user X report lists the presence-only employee's presence.

use pos_engine_testkit as common;

use common::*;

use pos_core::user::UserRole;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN};
use pos_engine::commands::auth::{
    CreateUserInput, do_create_user, do_deactivate_user, do_list_users, do_login,
    do_login_with_new_pin, do_set_pin,
};
use pos_engine::commands::errors;
use pos_engine::commands::report::do_generate_user_x_report;
use pos_engine::commands::social::{
    SocialResult, do_clock_in_employee, do_clock_out_employee, do_end_shift, do_list_open_shifts,
    do_work_in, do_work_out,
};
use pos_fiscal::types::EventLabel;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// The test manager (`TEST_OPERATOR`, an admin).
const MANAGER: &str = TEST_OPERATOR;
/// The kitchen employee (mod-97 valid).
const KITCHEN: &str = "12345678958";
const CASHIER: &str = "84022899837";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    do_create_user(
        &app.state,
        CreateUserInput {
            insz: KITCHEN.into(),
            name: "Cuisine".into(),
            role: "waiter".into(),
            presence_only: true,
            ..Default::default()
        },
    )
    .expect("create the presence-only employee");
    app
}

/// One-shot S answers from the published M140 / M141 fixtures.
async fn mock_in(app: &TestApp) {
    mock_from_fixture(app, &load_uc(&basic_set_root(), "M140")).await;
}
async fn mock_out(app: &TestApp) {
    mock_from_fixture(app, &load_uc(&basic_set_root(), "M141")).await;
}

/// The FDM answers every request with an error.
async fn mock_fdm_error(app: &TestApp) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{ "message": "internal FDM error (synthetic, from the test suite)" }]
        })))
        .mount(&app.fdm)
        .await;
}

/// The work-in / work-out requests that reached the FDM.
async fn work_requests(app: &TestApp) -> Vec<Value> {
    app.fdm
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter_map(|r| serde_json::from_slice::<Value>(&r.body).ok())
        .filter(|body| {
            body["query"]
                .as_str()
                .is_some_and(|q| q.contains("signWork"))
        })
        .collect()
}

fn shifts(app: &TestApp) -> Vec<(String, Option<String>, Option<String>, bool)> {
    let conn = app.state.db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT user_insz, started_by, ended_by, ended_at IS NULL
             FROM staff_shifts ORDER BY id",
        )
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn s_events(app: &TestApp) -> Vec<FiscalEventRow> {
    app.fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "S")
        .collect()
}

/// The manager command in the given direction.
async fn clock(app: &TestApp, is_in: bool, insz: &str) -> Result<SocialResult, String> {
    if is_in {
        do_clock_in_employee(&app.state, insz.into()).await
    } else {
        do_clock_out_employee(&app.state, insz.into()).await
    }
}

// ── The happy path ─────────────────────────────────────────────

#[tokio::test]
async fn a_manager_clocks_a_presence_only_employee_in_and_out() {
    let app = app().await;
    mock_in(&app).await;

    let r = do_clock_in_employee(&app.state, KITCHEN.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    // One signWorkIn, signed in the employee's name.
    let sent = work_requests(&app).await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0]["query"].as_str().unwrap().contains("signWorkIn"));
    assert_eq!(sent[0]["variables"]["data"]["employeeId"], KITCHEN);

    let events = s_events(&app);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].user_insz, KITCHEN);
    assert_eq!(app.counter_for(EventLabel::S), 1);
    let open = do_list_open_shifts(&app.state).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].user_insz, KITCHEN);
    assert_eq!(
        shifts(&app),
        vec![(KITCHEN.into(), Some(MANAGER.into()), None, true)]
    );

    mock_out(&app).await;
    let r = do_clock_out_employee(&app.state, KITCHEN.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let sent = work_requests(&app).await;
    assert_eq!(sent.len(), 2);
    assert!(sent[1]["query"].as_str().unwrap().contains("signWorkOut"));
    assert_eq!(sent[1]["variables"]["data"]["employeeId"], KITCHEN);
    assert_eq!(s_events(&app).len(), 2);
    assert!(do_list_open_shifts(&app.state).unwrap().is_empty());
    assert_eq!(
        shifts(&app),
        vec![(
            KITCHEN.into(),
            Some(MANAGER.into()),
            Some(MANAGER.into()),
            false
        )]
    );
}

#[tokio::test]
async fn a_regular_employee_can_be_clocked_in_by_a_manager_too() {
    let app = app().await;
    app.login_as(CASHIER, UserRole::Cashier);
    app.login_as(MANAGER, UserRole::Admin);
    mock_in(&app).await;
    let r = do_clock_in_employee(&app.state, CASHIER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(
        shifts(&app),
        vec![(CASHIER.into(), Some(MANAGER.into()), None, true)]
    );
}

#[tokio::test]
async fn a_self_registered_shift_has_no_manager_on_record() {
    let app = app().await;
    mock_in(&app).await;
    do_work_in(&app.state, MANAGER.into()).await.unwrap();
    mock_out(&app).await;
    do_work_out(&app.state, MANAGER.into()).await.unwrap();
    assert_eq!(
        shifts(&app),
        vec![(MANAGER.into(), None, None, false)],
        "the employee registered it themself"
    );
}

// ── Refusals sign nothing ──────────────────────────────────────

#[tokio::test]
async fn every_refusal_signs_nothing_and_moves_no_counter() {
    let app = app().await;
    let counter = app.counter_for(EventLabel::S);
    let archived = app.fiscal_events().len();

    // Not a manager.
    app.login_as(CASHIER, UserRole::Cashier);
    for result in [
        do_clock_in_employee(&app.state, KITCHEN.into()).await,
        do_clock_out_employee(&app.state, KITCHEN.into()).await,
    ] {
        assert_eq!(result.unwrap_err(), ERR_FORBIDDEN);
    }
    app.logout();
    assert_eq!(
        do_clock_in_employee(&app.state, KITCHEN.into())
            .await
            .unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    app.login_as(MANAGER, UserRole::Admin);

    // The system identity, an unknown employee, themself.
    for is_in in [true, false] {
        assert_eq!(
            clock(&app, is_in, "00000000029").await.unwrap_err(),
            errors::SYSTEM_USER,
            "in={is_in}"
        );
        assert_eq!(
            clock(&app, is_in, "99012345667").await.unwrap_err(),
            errors::USER_UNKNOWN,
            "in={is_in}"
        );
        assert_eq!(
            clock(&app, is_in, MANAGER).await.unwrap_err(),
            errors::PRESENCE_SELF,
            "in={is_in}"
        );
    }

    // An inactive employee is unknown.
    app.login_as(CASHIER, UserRole::Cashier);
    app.login_as(MANAGER, UserRole::Admin);
    do_deactivate_user(&app.state, CASHIER.into()).unwrap();
    assert_eq!(
        do_clock_in_employee(&app.state, CASHIER.into())
            .await
            .unwrap_err(),
        errors::USER_UNKNOWN
    );

    // Clock-out of someone who is not on shift.
    assert_eq!(
        do_clock_out_employee(&app.state, KITCHEN.into())
            .await
            .unwrap_err(),
        errors::PRESENCE_NOT_IN
    );
    // Clock-in of someone already on shift.
    pos_db::queries::shifts::open_shift(&app.state.db.lock().unwrap(), KITCHEN, None).unwrap();
    assert_eq!(
        do_clock_in_employee(&app.state, KITCHEN.into())
            .await
            .unwrap_err(),
        errors::PRESENCE_ALREADY_IN
    );

    assert!(
        work_requests(&app).await.is_empty(),
        "nothing reached the FDM"
    );
    assert_eq!(app.counter_for(EventLabel::S), counter);
    assert_eq!(app.fiscal_events().len(), archived);
    assert!(app.open_pending().is_none());
}

#[tokio::test]
async fn the_refusals_come_in_the_documented_order() {
    let app = app().await;
    // Training first, even for the system identity and the manager.
    app.state.config.lock().unwrap().training_mode = true;
    for insz in ["00000000029", "99012345667", MANAGER, KITCHEN] {
        assert_eq!(
            do_clock_in_employee(&app.state, insz.into())
                .await
                .unwrap_err(),
            errors::PRESENCE_TRAINING,
            "{insz}"
        );
        assert_eq!(
            do_clock_out_employee(&app.state, insz.into())
                .await
                .unwrap_err(),
            errors::PRESENCE_TRAINING,
            "{insz}"
        );
    }
    // Then unknown before self, and self before the shift state.
    app.state.config.lock().unwrap().training_mode = false;
    pos_db::queries::shifts::open_shift(&app.state.db.lock().unwrap(), MANAGER, None).unwrap();
    assert_eq!(
        do_clock_in_employee(&app.state, MANAGER.into())
            .await
            .unwrap_err(),
        errors::PRESENCE_SELF,
        "self is reported before already-in"
    );
    assert!(work_requests(&app).await.is_empty());
}

// ── FDM failure ────────────────────────────────────────────────

#[tokio::test]
async fn an_fdm_failure_archives_nothing_and_leaves_the_index() {
    let app = app().await;
    mock_fdm_error(&app).await;
    let counter = app.counter_for(EventLabel::S);

    let r = do_clock_in_employee(&app.state, KITCHEN.into())
        .await
        .expect("an FDM failure is a result, not an error");
    assert!(!r.success);
    let error = r.error.expect("the FDM error");
    assert!(error.starts_with("FDM"), "{error}");
    assert!(s_events(&app).is_empty(), "nothing archived");
    assert!(do_list_open_shifts(&app.state).unwrap().is_empty());
    assert!(shifts(&app).is_empty());
    assert_eq!(app.counter_for(EventLabel::S), counter);

    // The same for a clock-out of someone really on shift.
    pos_db::queries::shifts::open_shift(&app.state.db.lock().unwrap(), KITCHEN, None).unwrap();
    let r = do_clock_out_employee(&app.state, KITCHEN.into())
        .await
        .unwrap();
    assert!(!r.success);
    assert!(s_events(&app).is_empty());
    assert_eq!(
        do_list_open_shifts(&app.state).unwrap().len(),
        1,
        "still on shift"
    );
    assert_eq!(app.counter_for(EventLabel::S), counter);
}

// ── Training ───────────────────────────────────────────────────

#[tokio::test]
async fn a_training_work_in_does_not_open_a_shift_and_the_next_real_one_does() {
    let app = app().await;
    app.state.config.lock().unwrap().training_mode = true;
    mock_response(
        &app,
        in_training(read_response_envelope(&load_uc(&basic_set_root(), "M170"))),
    )
    .await;
    let r = do_work_in(&app.state, MANAGER.into()).await.unwrap();
    assert!(r.success, "{:?}", r.error);
    let events = app.fiscal_events();
    assert_eq!(
        events.iter().filter(|e| e.event_type == "T").count(),
        1,
        "a T event is archived"
    );
    assert!(
        do_list_open_shifts(&app.state).unwrap().is_empty(),
        "a training event registers no presence"
    );
    assert!(shifts(&app).is_empty());

    // A training clock-out leaves a real shift open.
    pos_db::queries::shifts::open_shift(&app.state.db.lock().unwrap(), MANAGER, None).unwrap();
    mock_response(
        &app,
        in_training(read_response_envelope(&load_uc(&basic_set_root(), "M141"))),
    )
    .await;
    let r = do_work_out(&app.state, MANAGER.into()).await.unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(do_list_open_shifts(&app.state).unwrap().len(), 1);

    // Training off: a real clock-in is signed (an S event) for a real shift.
    app.state.config.lock().unwrap().training_mode = false;
    pos_db::queries::shifts::close_shift(&app.state.db.lock().unwrap(), MANAGER, None).unwrap();
    mock_in(&app).await;
    let r = do_work_in(&app.state, MANAGER.into()).await.unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(s_events(&app).len(), 1);
    assert_eq!(do_list_open_shifts(&app.state).unwrap().len(), 1);
}

// ── Presence-only employees ────────────────────────────────────

#[tokio::test]
async fn a_presence_only_employee_is_created_with_no_pin_and_the_waiter_role() {
    let app = app().await;
    let listed = do_list_users(&app.state).unwrap();
    let kitchen = listed.iter().find(|u| u.insz == KITCHEN).unwrap();
    assert!(kitchen.presence_only);
    assert!(!kitchen.has_pin);
    assert_eq!(kitchen.role, UserRole::Waiter);

    // The flag needs the waiter role.
    for role in ["cashier", "admin", "chef"] {
        let err = do_create_user(
            &app.state,
            CreateUserInput {
                insz: CASHIER.into(),
                name: "X".into(),
                role: role.into(),
                presence_only: true,
                initial_pin: Some("1234".into()),
                operator_pin: Some("4321".into()),
            },
        )
        .unwrap_err();
        assert_eq!(err, errors::ROLE_INVALID, "{role}");
    }
    // A PIN passed with the flag is ignored.
    let created = do_create_user(
        &app.state,
        CreateUserInput {
            insz: CASHIER.into(),
            name: "Plonge".into(),
            role: "waiter".into(),
            initial_pin: Some("1234".into()),
            presence_only: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(created.presence_only && !created.has_pin);
    let db = app.state.db.lock().unwrap();
    assert!(
        pos_db::queries::users::get_pin_hash(&db, CASHIER)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn a_presence_only_employee_cannot_log_in_choose_a_pin_or_end_a_shift() {
    let app = app().await;
    app.logout();
    for pin in [None, Some("1234".to_string())] {
        assert_eq!(
            do_login(&app.state, KITCHEN.into(), pin.clone()).unwrap_err(),
            errors::PRESENCE_ONLY_USER
        );
        assert_eq!(
            do_end_shift(&app.state, KITCHEN.into(), pin)
                .await
                .unwrap_err(),
            errors::PRESENCE_ONLY_USER
        );
    }
    assert_eq!(
        do_login_with_new_pin(&app.state, KITCHEN.into(), "1234".into()).unwrap_err(),
        errors::PRESENCE_ONLY_USER
    );
    assert!(app.state.session.lock().unwrap().is_none());
    {
        let db = app.state.db.lock().unwrap();
        assert!(
            pos_db::queries::users::get_pin_hash(&db, KITCHEN)
                .unwrap()
                .is_none(),
            "no PIN was stored"
        );
    }
    assert!(work_requests(&app).await.is_empty());

    // A manager cannot give them a PIN either.
    app.login_as(MANAGER, UserRole::Admin);
    assert_eq!(
        do_set_pin(
            &app.state,
            KITCHEN.into(),
            "1234".into(),
            Some("0000".into())
        )
        .unwrap_err(),
        errors::PRESENCE_ONLY_USER
    );
    let db = app.state.db.lock().unwrap();
    assert!(
        pos_db::queries::users::get_pin_hash(&db, KITCHEN)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn re_creating_the_same_insz_without_the_flag_makes_a_till_user() {
    let app = app().await;
    do_deactivate_user(&app.state, KITCHEN.into()).unwrap();
    let user = do_create_user(
        &app.state,
        CreateUserInput {
            insz: KITCHEN.into(),
            name: "Cuisine".into(),
            role: "cashier".into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!user.presence_only);
    app.logout();
    assert!(do_login(&app.state, KITCHEN.into(), None).is_ok());
}

// ── Deactivation ───────────────────────────────────────────────

#[tokio::test]
async fn an_employee_on_shift_cannot_be_deactivated() {
    let app = app().await;
    mock_in(&app).await;
    do_clock_in_employee(&app.state, KITCHEN.into())
        .await
        .unwrap();
    assert_eq!(
        do_deactivate_user(&app.state, KITCHEN.into()).unwrap_err(),
        errors::DEACTIVATE_ON_SHIFT
    );
    assert!(
        do_list_users(&app.state)
            .unwrap()
            .iter()
            .any(|u| u.insz == KITCHEN)
    );

    mock_out(&app).await;
    do_clock_out_employee(&app.state, KITCHEN.into())
        .await
        .unwrap();
    do_deactivate_user(&app.state, KITCHEN.into()).unwrap();
}

// ── Reports ────────────────────────────────────────────────────

#[tokio::test]
async fn the_user_x_report_lists_the_presence_only_employee() {
    let app = app().await;
    mock_in(&app).await;
    do_clock_in_employee(&app.state, KITCHEN.into())
        .await
        .unwrap();
    mock_from_fixture(&app, &load_uc(&reports_root(), "M182")).await;

    let result = do_generate_user_x_report(&app.state, MANAGER.to_string())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let events = app.fiscal_events();
    let r = events
        .iter()
        .find(|e| e.event_type == "R")
        .expect("R event");
    let request: Value = serde_json::from_str(r.event_data_json.as_ref().unwrap()).unwrap();
    let users = request["users"].as_array().expect("users");
    let kitchen = users
        .iter()
        .find(|u| u["employeeId"] == KITCHEN)
        .expect("the presence-only employee is listed");
    assert_eq!(kitchen["socialEvents"][0]["inOut"], "IN");
    assert_eq!(kitchen["totalAmount"], 0.0);
}
