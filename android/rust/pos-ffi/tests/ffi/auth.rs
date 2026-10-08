//! Tests of the auth facade (`api::auth`, `records::auth`): the contract of every record and one
//! wiring test per method through `PosEngine`.
//!
//! - Contract, output types: a real engine value (a command run on a `TestApp`) or a hand-built
//!   one, with `Option` fields set and empty, goes through `convert::out`; the two serde forms must
//!   be equal. Input types: an FFI value with every field set goes through `convert::inp` and the
//!   engine fields are checked one by one.
//! - Wiring: an engine on the seeded database of `support`, logged in through the FFI with the
//!   seeded manager. Remote support answers "not available" (PD53: no support program on Android).

use std::fmt::Debug;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use pos_core::user::{User as EngineUser, UserRole as EngineUserRole};
use pos_db::queries::integrations::SwitchChange as EngineSwitchChange;
use pos_engine::commands::auth::{
    CreateUserInput as EngineCreateUserInput, do_create_user, do_list_users, do_set_pin,
};
use pos_engine::commands::integrations::{
    Integrations as EngineIntegrations, do_get_integrations, do_set_hubrise_enabled,
};
use pos_engine::commands::support::{
    SupportRunning as EngineSupportRunning, SupportStatus as EngineSupportStatus,
    do_remote_support_running, do_remote_support_status, do_start_remote_support,
    do_stop_remote_support,
};
use pos_engine_testkit::TestApp;
use pos_ffi::records::auth as ffi;
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::{PosError, PosEvent};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json, to_value};

use crate::support::{MANAGER_INSZ, MANAGER_PIN, open_seeded};

// ───────────────────────────── helpers ─────────────────────────────

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

/// Engine value to FFI record through `convert::out`; the two serde forms must be equal.
fn out_equal<E: Serialize, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = contract_out(engine).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        json_of(engine),
        json_of(&record),
        "engine vs FFI serde form"
    );
    record
}

/// `out_equal` for an enum, plus: the variants stay distinct, and the way back gives the same
/// engine value.
fn enum_both<
    E: Serialize + DeserializeOwned + PartialEq + Debug,
    F: Serialize + DeserializeOwned + PartialEq + Debug,
>(
    variants: &[E],
) {
    for (i, v) in variants.iter().enumerate() {
        let record: F = out_equal(v);
        let back: E = contract_inp(&record).unwrap_or_else(|e| panic!("convert::inp: {e}"));
        assert_eq!(&back, v);
        for (j, w) in variants.iter().enumerate() {
            let other: F = out_equal(w);
            assert_eq!(i == j, record == other, "{record:?} vs {other:?}");
        }
    }
}

fn code_of<T: Debug>(r: Result<T, PosError>) -> String {
    match r.expect_err("an error") {
        PosError::Coded { code, .. } => code,
        other => panic!("expected a coded error, got {other:?}"),
    }
}

/// A mod-97-valid INSZ no seed user has.
const NEW_INSZ: &str = "12345678958";
/// A second one.
const OTHER_INSZ: &str = "85073003328";
const SEED_CASHIER_INSZ: &str = "75061189702";
const SEED_CASHIER_PIN: &str = "4444";
const ROBOT_INSZ: &str = "00000000029";

fn engine_user(insz: &str, role: EngineUserRole, active: bool, has_pin: bool) -> EngineUser {
    EngineUser {
        insz: insz.to_string(),
        name: format!("Name {insz}"),
        role,
        active,
        has_pin,
    }
}

/// A stand-in for the support program (a shell script), as the engine's own tests use.
fn fake_program(dir: &tempfile::TempDir, script: &str) -> std::path::PathBuf {
    let path = dir.path().join("AppRun");
    std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

// ───────────────────────────── contract: users ─────────────────────────────

#[test]
fn user_roles_convert_both_ways() {
    enum_both::<EngineUserRole, ffi::UserRole>(&[
        EngineUserRole::Admin,
        EngineUserRole::Cashier,
        EngineUserRole::Waiter,
    ]);
    // The wire names are the engine's.
    assert_eq!(json_of(&ffi::UserRole::Admin), json!("admin"));
    assert_eq!(json_of(&ffi::UserRole::Cashier), json!("cashier"));
    assert_eq!(json_of(&ffi::UserRole::Waiter), json!("waiter"));
}

#[tokio::test]
async fn users_of_the_engine_convert() {
    let app = TestApp::new().await;
    // A user the engine creates, as the create_user command answers it.
    let created = do_create_user(
        &app.state,
        EngineCreateUserInput {
            insz: "99012345667".into(),
            name: "Apprenti".into(),
            role: "cashier".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let record: ffi::User = out_equal(&created);
    assert_eq!(record.role, ffi::UserRole::Cashier);
    assert!(!record.has_pin);

    // Real values: the users the engine lists, one with a PIN, one without.
    app.login_as(NEW_INSZ, EngineUserRole::Admin);
    do_set_pin(&app.state, NEW_INSZ.into(), "4321".into(), None).unwrap();
    app.login_as(OTHER_INSZ, EngineUserRole::Waiter);
    let listed = do_list_users(&app.state).unwrap();
    assert!(listed.iter().any(|u| u.insz == NEW_INSZ && u.has_pin));
    assert!(listed.iter().any(|u| u.insz == OTHER_INSZ && !u.has_pin));
    let records: Vec<ffi::User> = out_equal(&listed);
    assert_eq!(records.len(), listed.len());
    let admin = records.iter().find(|u| u.insz == NEW_INSZ).unwrap();
    assert_eq!(admin.role, ffi::UserRole::Admin);
    assert!(admin.has_pin && admin.active);
    let waiter = records.iter().find(|u| u.insz == OTHER_INSZ).unwrap();
    assert_eq!(waiter.role, ffi::UserRole::Waiter);
    assert!(!waiter.has_pin);

    // Hand-built: an inactive cashier (the engine lists active users only).
    let inactive = engine_user("75061189702", EngineUserRole::Cashier, false, true);
    let record: ffi::User = out_equal(&inactive);
    assert!(!record.active);
    assert_eq!(record.role, ffi::UserRole::Cashier);
    // And back.
    let back: EngineUser = contract_inp(&record).unwrap();
    assert_eq!(json_of(&back), json_of(&inactive));
}

#[test]
fn a_user_record_has_no_pin_hash() {
    let record: ffi::User = out_equal(&engine_user(NEW_INSZ, EngineUserRole::Admin, true, true));
    let mut keys: Vec<String> = json_of(&record)
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    keys.sort();
    assert_eq!(keys, ["active", "has_pin", "insz", "name", "role"]);
    // A value carrying a hash is refused, not ignored.
    let with_hash = json!({
        "insz": NEW_INSZ, "name": "n", "role": "admin", "active": true, "has_pin": true,
        "pin_hash": "$2y$12$x"
    });
    assert!(serde_json::from_value::<ffi::User>(with_hash).is_err());
}

#[test]
fn create_user_input_converts_field_by_field() {
    let full = ffi::CreateUserInput {
        insz: NEW_INSZ.into(),
        name: "Alice".into(),
        role: "admin".into(),
        initial_pin: Some("1234".into()),
        operator_pin: Some("5678".into()),
    };
    let engine: EngineCreateUserInput = contract_inp(&full).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(engine.insz, NEW_INSZ);
    assert_eq!(engine.name, "Alice");
    assert_eq!(engine.role, "admin");
    assert_eq!(engine.initial_pin.as_deref(), Some("1234"));
    assert_eq!(engine.operator_pin.as_deref(), Some("5678"));
    // The keys are the engine's (camelCase).
    let value = json_of(&full);
    for key in ["insz", "name", "role", "initialPin", "operatorPin"] {
        assert!(value.get(key).is_some(), "key {key} in {value}");
    }

    let bare = ffi::CreateUserInput {
        insz: OTHER_INSZ.into(),
        name: "Bob".into(),
        role: "waiter".into(),
        initial_pin: None,
        operator_pin: None,
    };
    let engine: EngineCreateUserInput = contract_inp(&bare).unwrap();
    assert_eq!(engine.role, "waiter");
    assert_eq!(engine.initial_pin, None);
    assert_eq!(engine.operator_pin, None);

    // The optional fields may be absent, like the engine accepts; an unknown key is refused.
    let absent: ffi::CreateUserInput =
        serde_json::from_value(json!({"insz": "1", "name": "n", "role": "cashier"})).unwrap();
    assert_eq!(absent.initial_pin, None);
    assert_eq!(absent.operator_pin, None);
    assert!(
        serde_json::from_value::<ffi::CreateUserInput>(
            json!({"insz": "1", "name": "n", "role": "cashier", "extra": 1})
        )
        .is_err()
    );
}

// ───────────────────────────── contract: integrations ─────────────────────────────

#[tokio::test]
async fn integrations_of_the_engine_convert() {
    let app = TestApp::new().await;
    // The state the engine starts in: the switch row exists (`pos.json` says off), and that first
    // setting is logged by the configuration, without an author.
    {
        let conn = app.state.db.lock().unwrap();
        assert!(!pos_db::queries::integrations::hubrise_at_start(&conn, false).unwrap());
    }
    let start = do_get_integrations(&app.state).unwrap();
    assert_eq!(start.hubrise_log.len(), 1);
    assert_eq!(start.hubrise_log[0].changed_by, None);
    let record: ffi::Integrations = out_equal(&start);
    assert_eq!(record.hubrise_log[0].changed_by, None);
    let record: ffi::Integrations = out_equal(&start);
    assert_eq!(record.hubrise_enabled, start.hubrise_enabled);
    assert_eq!(record.online_orders_open, 0);
    assert_eq!(record.hubrise_log.len(), start.hubrise_log.len());

    // Flipped, then flipped back: two more logged changes, the latest first.
    let first = !start.hubrise_enabled;
    let flipped = do_set_hubrise_enabled(&app.state, first).unwrap();
    let record: ffi::Integrations = out_equal(&flipped);
    assert_eq!(record.hubrise_enabled, first);
    assert_eq!(record.hubrise_log.len(), start.hubrise_log.len() + 1);
    assert_eq!(record.hubrise_log[0].enabled, first);
    assert!(record.hubrise_log[0].changed_by.is_some());
    let back = do_set_hubrise_enabled(&app.state, !first).unwrap();
    let record: ffi::Integrations = out_equal(&back);
    assert_eq!(record.hubrise_enabled, !first);
    assert_eq!(record.hubrise_log.len(), start.hubrise_log.len() + 2);
    assert_eq!(record.hubrise_log[0].enabled, !first);
    assert_eq!(record.hubrise_log[1].enabled, first);
    assert!(!record.hubrise_log[0].changed_at.is_empty());
    assert!(!record.hubrise_log[0].source.is_empty());
}

#[test]
fn switch_changes_convert_with_and_without_an_author() {
    let by_admin = EngineSwitchChange {
        enabled: true,
        source: "admin".into(),
        changed_by: Some(MANAGER_INSZ.into()),
        changed_at: "2026-10-05 18:30:00".into(),
    };
    let record: ffi::SwitchChange = out_equal(&by_admin);
    assert_eq!(record.changed_by.as_deref(), Some(MANAGER_INSZ));
    assert!(record.enabled);
    let by_config = EngineSwitchChange {
        enabled: false,
        source: "config".into(),
        changed_by: None,
        changed_at: "2026-10-05 18:31:00".into(),
    };
    let record: ffi::SwitchChange = out_equal(&by_config);
    assert_eq!(record.changed_by, None);
    assert!(!record.enabled);
    assert_eq!(record.source, "config");
    assert_eq!(record.changed_at, "2026-10-05 18:31:00");

    // A whole answer, hand-built.
    let answer = EngineIntegrations {
        hubrise_enabled: true,
        online_orders_open: 3,
        hubrise_log: vec![by_admin, by_config],
    };
    let record: ffi::Integrations = out_equal(&answer);
    assert_eq!(record.online_orders_open, 3);
    assert_eq!(record.hubrise_log.len(), 2);
}

// ───────────────────────────── contract: support ─────────────────────────────

#[tokio::test]
async fn support_status_and_running_convert() {
    let app = TestApp::new().await;
    // No program: not available, nothing runs.
    let status = do_remote_support_status(&app.state).unwrap();
    assert!(!status.available);
    let record: ffi::SupportStatus = out_equal(&status);
    assert!(!record.available && !record.running);
    assert_eq!((record.started_by, record.started_at), (None, None));
    let running = do_remote_support_running(&app.state).unwrap();
    let record: ffi::SupportRunning = out_equal(&running);
    assert!(!record.running);
    assert_eq!(record.started_at, None);

    // A program installed and a session started: every field set.
    let dir = tempfile::tempdir().unwrap();
    app.state
        .support
        .set_program(Some(fake_program(&dir, "sleep 30")));
    let operator = app.state.session.lock().unwrap().clone().unwrap();
    do_set_pin(&app.state, operator.insz.clone(), "4321".into(), None).unwrap();
    let started =
        do_start_remote_support(&app.state, operator.insz.clone(), "4321".into()).unwrap();
    let record: ffi::SupportStatus = out_equal(&started);
    assert!(record.available && record.running);
    assert_eq!(record.started_by.as_deref(), Some(operator.name.as_str()));
    assert!(record.started_at.is_some());
    let running: EngineSupportRunning = do_remote_support_running(&app.state).unwrap();
    let record: ffi::SupportRunning = out_equal(&running);
    assert!(record.running);
    assert!(record.started_at.is_some());
    let stopped: EngineSupportStatus = do_stop_remote_support(&app.state).unwrap();
    let record: ffi::SupportStatus = out_equal(&stopped);
    assert!(record.available && !record.running);
}

// ───────────────────────────── wiring ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn login_logout_and_the_current_user_are_wired() {
    let te = open_seeded().await;
    let e = &te.engine;

    assert_eq!(e.get_current_user().await.unwrap(), None);

    // The login screen: every active user, the robot excluded.
    let users = e.list_users().await.unwrap();
    assert!(users.iter().any(|u| u.insz == MANAGER_INSZ
        && u.name == "Gerant Test"
        && u.role == ffi::UserRole::Admin
        && u.has_pin));
    assert!(users.iter().any(|u| u.insz == SEED_CASHIER_INSZ
        && u.name == "John"
        && u.role == ffi::UserRole::Cashier));
    assert!(users.iter().all(|u| u.insz != ROBOT_INSZ));

    // A wrong PIN is a coded refusal; no PIN at all for a user who has one too.
    assert_eq!(
        code_of(e.login(SEED_CASHIER_INSZ.into(), Some("0000".into())).await),
        "pin_wrong"
    );
    assert_eq!(
        code_of(e.login(SEED_CASHIER_INSZ.into(), None).await),
        "pin_required"
    );
    assert_eq!(e.get_current_user().await.unwrap(), None);
    // An unknown user, the system identity and a bad INSZ.
    assert_eq!(
        code_of(e.login(OTHER_INSZ.into(), Some("1111".into())).await),
        "user_unknown"
    );
    assert_eq!(
        code_of(e.login(ROBOT_INSZ.into(), None).await),
        "system_user"
    );
    assert_eq!(code_of(e.login("123".into(), None).await), "insz_invalid");

    // The right PIN: the user, then the same one as the current user.
    let events = e.events().await.unwrap();
    let user = e
        .login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert_eq!(user.insz, MANAGER_INSZ);
    assert_eq!(user.role, ffi::UserRole::Admin);
    assert!(user.active && user.has_pin);
    assert_eq!(e.get_current_user().await.unwrap(), Some(user.clone()));

    // The login queued a clock-in: the presence worker signs an S event, or at least does not
    // report a failure.
    let mut failed = false;
    let mut signed = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline && !signed && !failed {
        match tokio::time::timeout(Duration::from_millis(200), events.next()).await {
            Ok(Some(PosEvent::PresenceFailed { .. })) => failed = true,
            Ok(Some(_)) | Err(_) => {}
            Ok(None) => panic!("the event stream ended"),
        }
        signed = e
            .list_fiscal_events(None)
            .await
            .unwrap()
            .iter()
            .any(|ev| ev.event_type == "S");
    }
    assert!(signed || !failed, "the clock-in failed: presence-failed");

    // Logout: nobody is logged in; the commands that need an operator refuse.
    e.logout().await.unwrap();
    assert_eq!(e.get_current_user().await.unwrap(), None);
    assert_eq!(code_of(e.remote_support_status().await), "not_logged_in");
    assert_eq!(
        code_of(e.deactivate_user(SEED_CASHIER_INSZ.into()).await),
        "not_logged_in"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn users_and_pins_are_wired() {
    let te = open_seeded().await;
    let e = &te.engine;

    // Nobody logged in: creating a user is refused.
    let waiter = ffi::CreateUserInput {
        insz: NEW_INSZ.into(),
        name: "Alice".into(),
        role: "waiter".into(),
        initial_pin: None,
        operator_pin: None,
    };
    assert_eq!(
        code_of(e.create_user(waiter.clone()).await),
        "not_logged_in"
    );

    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    // A waiter without a PIN.
    let created = e.create_user(waiter.clone()).await.unwrap();
    assert_eq!(
        created,
        ffi::User {
            insz: NEW_INSZ.into(),
            name: "Alice".into(),
            role: ffi::UserRole::Waiter,
            active: true,
            has_pin: false,
        }
    );
    assert!(e.list_users().await.unwrap().contains(&created));
    // Twice, and a bad role.
    assert!(e.create_user(waiter.clone()).await.is_err());
    assert_eq!(
        code_of(
            e.create_user(ffi::CreateUserInput {
                insz: OTHER_INSZ.into(),
                role: "chef".into(),
                ..waiter.clone()
            })
            .await
        ),
        "role_invalid"
    );

    // A manager needs a first PIN and the creating manager's own PIN.
    let boss = ffi::CreateUserInput {
        insz: OTHER_INSZ.into(),
        name: "Boss".into(),
        role: "admin".into(),
        initial_pin: Some("9999".into()),
        operator_pin: None,
    };
    assert_eq!(code_of(e.create_user(boss.clone()).await), "pin_required");
    let boss_user = e
        .create_user(ffi::CreateUserInput {
            operator_pin: Some(MANAGER_PIN.into()),
            ..boss
        })
        .await
        .unwrap();
    assert!(boss_user.has_pin);
    assert_eq!(boss_user.role, ffi::UserRole::Admin);

    // The new waiter chooses a PIN at the first login; a second time is refused.
    let logged = e
        .login_with_new_pin(NEW_INSZ.into(), "7777".into())
        .await
        .unwrap();
    assert!(logged.has_pin);
    assert_eq!(e.get_current_user().await.unwrap(), Some(logged));
    assert_eq!(
        code_of(e.login_with_new_pin(NEW_INSZ.into(), "1234".into()).await),
        "pin_already_set"
    );
    assert_eq!(
        code_of(
            e.login_with_new_pin(SEED_CASHIER_INSZ.into(), "12".into())
                .await
        ),
        "pin_format"
    );

    // The waiter changes their own PIN; they cannot change another user's.
    e.set_pin(NEW_INSZ.into(), "8888".into(), None)
        .await
        .unwrap();
    assert_eq!(
        code_of(
            e.set_pin(SEED_CASHIER_INSZ.into(), "1234".into(), None)
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code_of(e.set_pin(NEW_INSZ.into(), "12a4".into(), None).await),
        "pin_format"
    );
    e.logout().await.unwrap();
    assert_eq!(
        code_of(e.login(NEW_INSZ.into(), Some("7777".into())).await),
        "pin_wrong"
    );
    e.login(NEW_INSZ.into(), Some("8888".into())).await.unwrap();
    e.logout().await.unwrap();

    // A manager sets another user's PIN with their own PIN to confirm.
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert_eq!(
        code_of(e.set_pin(NEW_INSZ.into(), "5555".into(), None).await),
        "pin_required"
    );
    e.set_pin(NEW_INSZ.into(), "5555".into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    // Deactivate: never oneself; the others leave the login tiles.
    assert_eq!(
        code_of(e.deactivate_user(MANAGER_INSZ.into()).await),
        "deactivate_self"
    );
    e.deactivate_user(NEW_INSZ.into()).await.unwrap();
    assert!(
        e.list_users()
            .await
            .unwrap()
            .iter()
            .all(|u| u.insz != NEW_INSZ)
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_hubrise_switch_round_trips() {
    let te = open_seeded().await;
    let e = &te.engine;

    // Nobody logged in: the switch is refused (the read is open).
    assert_eq!(code_of(e.set_hubrise_enabled(true).await), "not_logged_in");
    let before = e.get_integrations().await.unwrap();

    // A cashier is not allowed to flip it.
    e.login(SEED_CASHIER_INSZ.into(), Some(SEED_CASHIER_PIN.into()))
        .await
        .unwrap();
    assert_eq!(
        code_of(e.set_hubrise_enabled(!before.hubrise_enabled).await),
        "forbidden"
    );
    e.logout().await.unwrap();

    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    let first = !before.hubrise_enabled;
    let flipped = e.set_hubrise_enabled(first).await.unwrap();
    assert_eq!(flipped.hubrise_enabled, first);
    assert_eq!(flipped.online_orders_open, 0);
    assert_eq!(flipped.hubrise_log.len(), before.hubrise_log.len() + 1);
    assert_eq!(flipped.hubrise_log[0].enabled, first);
    assert_eq!(
        flipped.hubrise_log[0].changed_by.as_deref(),
        Some(MANAGER_INSZ)
    );
    assert_eq!(e.get_integrations().await.unwrap(), flipped);

    let back = e.set_hubrise_enabled(!first).await.unwrap();
    assert_eq!(back.hubrise_enabled, !first);
    assert_eq!(back.hubrise_log.len(), flipped.hubrise_log.len() + 1);
    assert_eq!(back.hubrise_log[0].enabled, !first);
    assert_eq!(e.get_integrations().await.unwrap(), back);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn remote_support_is_not_available_on_android() {
    let te = open_seeded().await;
    let e = &te.engine;

    // The login screen asks without an operator.
    let running = e.remote_support_running().await.unwrap();
    assert_eq!(
        running,
        ffi::SupportRunning {
            running: false,
            started_at: None
        }
    );
    // Stopping when nothing runs is fine, logged in or not.
    let stopped = e.stop_remote_support().await.unwrap();
    assert!(!stopped.available && !stopped.running);

    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert_eq!(
        e.remote_support_status().await.unwrap(),
        ffi::SupportStatus {
            available: false,
            running: false,
            started_by: None,
            started_at: None
        }
    );
    // The PIN is checked first, then the missing program (PD53: no support program).
    assert_eq!(
        code_of(
            e.start_remote_support(MANAGER_INSZ.into(), "0000".into())
                .await
        ),
        "pin_wrong"
    );
    assert_eq!(
        code_of(
            e.start_remote_support(MANAGER_INSZ.into(), MANAGER_PIN.into())
                .await
        ),
        "support_unavailable"
    );
    let status = e.remote_support_status().await.unwrap();
    assert!(!status.available && !status.running);
    assert!(!e.remote_support_running().await.unwrap().running);
    let stopped = e.stop_remote_support().await.unwrap();
    assert!(!stopped.available && !stopped.running);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_methods_refuse_after_shutdown() {
    let te = open_seeded().await;
    te.engine.shutdown().await.unwrap();
    assert!(matches!(
        te.engine.login(MANAGER_INSZ.into(), None).await,
        Err(PosError::EngineClosed)
    ));
    assert!(matches!(
        te.engine.get_current_user().await,
        Err(PosError::EngineClosed)
    ));
    assert!(matches!(
        te.engine.get_integrations().await,
        Err(PosError::EngineClosed)
    ));
    assert!(matches!(
        te.engine.remote_support_running().await,
        Err(PosError::EngineClosed)
    ));
}
