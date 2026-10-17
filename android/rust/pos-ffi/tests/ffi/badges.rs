//! Tests of the badges facade (`api::badges`, `records::badges`): the contract of every record and
//! one wiring test per method through `PosEngine`.
//!
//! - Contract: an engine value goes through `convert::out`, the two serde forms must be equal; the
//!   input record goes through `convert::inp` and the engine fields are checked one by one.
//! - Wiring: an engine on the seeded database of `support` (the manager of the seed, an employee
//!   of the seed to hold the badge). The reader is off by default and every badge command says so.

use std::fmt::Debug;

use pos_core::user::{User as EngineUser, UserRole as EngineUserRole};
use pos_db::queries::badges::{ActiveBadge as EngineActiveBadge, LogEntry as EngineLogEntry};
use pos_engine::commands::badges::{
    BadgeLink as EngineBadgeLink, BadgeOutcome as EngineBadgeOutcome,
    BadgeOutcomeKind as EngineKind, BadgeOverview as EngineOverview,
    BadgeSettingsInput as EngineInput, BadgeSettingsView as EngineView,
};
use pos_ffi::PosError;
use pos_ffi::records::badges as ffi;
use pos_ffi::records::common::{contract_inp, contract_out};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json, to_value};

use crate::support::{MANAGER_INSZ, MANAGER_PIN, open_seeded};

/// "John", a cashier of the seed.
const EMPLOYEE: &str = "75061189702";
const TOKEN: &str = "04A1B2C3D4E5F6";

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

fn out_equal<E: Serialize, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = contract_out(engine).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        json_of(engine),
        json_of(&record),
        "engine vs FFI serde form"
    );
    record
}

fn coded(err: &PosError, expected: &str) -> bool {
    matches!(err, PosError::Coded { code, .. } if code == expected)
}

fn code_of<T: Debug>(r: Result<T, PosError>) -> String {
    match r.expect_err("an error") {
        PosError::Coded { code, .. } => code,
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn engine_user() -> EngineUser {
    EngineUser {
        insz: EMPLOYEE.into(),
        name: "John".into(),
        role: EngineUserRole::Cashier,
        active: true,
        has_pin: true,
        presence_only: false,
    }
}

// ───────────────────────────── contract ─────────────────────────────

#[test]
fn the_settings_view_converts() {
    let view = EngineView {
        reader: "usb".into(),
        login_enabled: true,
        wedge_max_gap_ms: 80,
        wedge_min_len: 8,
        wedge_max_len: 20,
        relink_needed: true,
    };
    let record: ffi::BadgeSettingsView = out_equal(&view);
    assert_eq!(record.reader, "usb");
    assert!(record.login_enabled && record.relink_needed);
    assert_eq!(
        (
            record.wedge_max_gap_ms,
            record.wedge_min_len,
            record.wedge_max_len
        ),
        (80, 8, 20)
    );
    let keys: Vec<String> = {
        let mut k: Vec<String> = json_of(&record)
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        k.sort();
        k
    };
    assert_eq!(
        keys,
        [
            "loginEnabled",
            "reader",
            "relinkNeeded",
            "wedgeMaxGapMs",
            "wedgeMaxLen",
            "wedgeMinLen"
        ]
    );
}

#[test]
fn the_settings_input_converts_field_by_field() {
    let full = ffi::BadgeSettingsInput {
        reader: "nfc".into(),
        login_enabled: true,
        wedge_max_gap_ms: 120,
        wedge_min_len: 5,
        wedge_max_len: 40,
    };
    let engine: EngineInput = contract_inp(&full).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(engine.reader, "nfc");
    assert!(engine.login_enabled);
    assert_eq!(engine.wedge_max_gap_ms, 120);
    assert_eq!(engine.wedge_min_len, 5);
    assert_eq!(engine.wedge_max_len, 40);
    // An unknown field is refused, not ignored.
    let extra = json!({
        "reader": "usb", "loginEnabled": false, "wedgeMaxGapMs": 50, "wedgeMinLen": 6,
        "wedgeMaxLen": 32, "token": "x"
    });
    assert!(serde_json::from_value::<ffi::BadgeSettingsInput>(extra).is_err());
}

#[test]
fn the_overview_converts_with_and_without_optional_fields() {
    let overview = EngineOverview {
        badges: vec![
            EngineActiveBadge {
                insz: EMPLOYEE.into(),
                name: "John".into(),
                linked_at: "2026-10-17 12:00:00".into(),
                linked_by: MANAGER_INSZ.into(),
                linked_by_name: Some("Gerant Test".into()),
            },
            EngineActiveBadge {
                insz: "12345678958".into(),
                name: "Anna".into(),
                linked_at: "2026-10-17 12:01:00".into(),
                linked_by: "00000000000".into(),
                linked_by_name: None,
            },
        ],
        log: vec![
            EngineLogEntry {
                action: "link".into(),
                insz: Some(EMPLOYEE.into()),
                name: Some("John".into()),
                by: Some(MANAGER_INSZ.into()),
                by_name: Some("Gerant Test".into()),
                at: "2026-10-17 12:00:00".into(),
                detail: None,
            },
            EngineLogEntry {
                action: "revoke".into(),
                insz: Some(EMPLOYEE.into()),
                name: None,
                by: None,
                by_name: None,
                at: "2026-10-17 12:02:00".into(),
                detail: Some("key_changed".into()),
            },
        ],
    };
    let record: ffi::BadgeOverview = out_equal(&overview);
    assert_eq!(record.badges.len(), 2);
    assert_eq!(record.badges[1].linked_by_name, None);
    assert_eq!(record.log[1].detail.as_deref(), Some("key_changed"));
    assert_eq!(record.log[1].by, None);
}

#[test]
fn a_link_converts() {
    let link = EngineBadgeLink {
        insz: EMPLOYEE.into(),
        name: "John".into(),
        linked_at: "2026-10-17 12:00:00".into(),
    };
    let record: ffi::BadgeLink = out_equal(&link);
    assert_eq!(record.name, "John");
}

#[test]
fn every_outcome_kind_converts_and_keeps_its_wire_name() {
    for (kind, wire) in [
        (EngineKind::Ignored, "ignored"),
        (EngineKind::ClockIn, "clock_in"),
        (EngineKind::ClockOut, "clock_out"),
        (EngineKind::PresenceFailed, "presence_failed"),
        (EngineKind::LoggedIn, "logged_in"),
        (EngineKind::PinRequired, "pin_required"),
    ] {
        let outcome = EngineBadgeOutcome {
            kind,
            insz: None,
            name: None,
            at: None,
            error: None,
            user: None,
        };
        let record: ffi::BadgeOutcome = out_equal(&outcome);
        assert_eq!(json_of(&record.kind), json!(wire));
    }
}

#[test]
fn an_outcome_converts_with_its_fields_and_its_user() {
    let outcome = EngineBadgeOutcome {
        kind: EngineKind::LoggedIn,
        insz: Some(EMPLOYEE.into()),
        name: Some("John".into()),
        at: Some("2026-10-17T12:00:00Z".into()),
        error: Some("FDM refused".into()),
        user: Some(engine_user()),
    };
    let record: ffi::BadgeOutcome = out_equal(&outcome);
    assert_eq!(record.kind, ffi::BadgeOutcomeKind::LoggedIn);
    assert_eq!(record.at.as_deref(), Some("2026-10-17T12:00:00Z"));
    let user = record.user.expect("the session user");
    assert_eq!(user.insz, EMPLOYEE);
    assert!(user.has_pin);
    // Never a badge number or a hash in an outcome.
    let keys: Vec<String> = json_of(&record_without_user())
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert!(
        keys.iter()
            .all(|k| !k.contains("token") && !k.contains("hash"))
    );
}

fn record_without_user() -> ffi::BadgeOutcome {
    ffi::BadgeOutcome {
        kind: ffi::BadgeOutcomeKind::ClockIn,
        insz: None,
        name: None,
        at: None,
        error: None,
        user: None,
    }
}

// ───────────────────────────── wiring ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn the_reader_is_off_by_default_and_every_badge_command_says_so() {
    let t = open_seeded().await;
    let e = &t.engine;

    // The setting needs no session.
    let s = e.get_badge_settings().await.unwrap();
    assert_eq!(s.reader, "off");
    assert!(!s.login_enabled && !s.relink_needed);
    assert_eq!(
        (s.wedge_max_gap_ms, s.wedge_min_len, s.wedge_max_len),
        (50, 6, 32)
    );

    // A scan is refused with the reader off, with or without a session.
    assert!(coded(
        &e.badge_tap(TOKEN.into(), false).await.unwrap_err(),
        "badge_reader_off"
    ));
    // Everything else needs a manager.
    for err in [
        e.list_badges().await.unwrap_err(),
        e.revoke_badge(EMPLOYEE.into()).await.unwrap_err(),
        e.link_badge(EMPLOYEE.into(), TOKEN.into(), Some(MANAGER_PIN.into()))
            .await
            .unwrap_err(),
        e.set_badge_settings(ffi::BadgeSettingsInput {
            reader: "usb".into(),
            login_enabled: false,
            wedge_max_gap_ms: 50,
            wedge_min_len: 6,
            wedge_max_len: 32,
        })
        .await
        .unwrap_err(),
    ] {
        assert!(coded(&err, "not_logged_in"), "{err:?}");
    }
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    let err = e
        .link_badge(EMPLOYEE.into(), TOKEN.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap_err();
    assert!(coded(&err, "badge_reader_off"), "{err:?}");
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn settings_enrolment_scan_and_revocation_go_through_the_facade() {
    let t = open_seeded().await;
    let e = &t.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    // set_badge_settings
    let input = ffi::BadgeSettingsInput {
        reader: "usb".into(),
        login_enabled: true,
        wedge_max_gap_ms: 70,
        wedge_min_len: 6,
        wedge_max_len: 32,
    };
    let s = e.set_badge_settings(input.clone()).await.unwrap();
    assert_eq!(s.reader, "usb");
    assert!(s.login_enabled);
    assert_eq!(s.wedge_max_gap_ms, 70);
    assert_eq!(e.get_badge_settings().await.unwrap(), s);
    let bad = ffi::BadgeSettingsInput {
        wedge_max_gap_ms: 5,
        ..input
    };
    assert_eq!(
        code_of(e.set_badge_settings(bad).await),
        "badge_settings_invalid"
    );

    // link_badge: the manager's own PIN is checked.
    assert_eq!(
        code_of(e.link_badge(EMPLOYEE.into(), TOKEN.into(), None).await),
        "pin_required"
    );
    assert_eq!(
        code_of(
            e.link_badge(EMPLOYEE.into(), TOKEN.into(), Some("9999".into()))
                .await
        ),
        "pin_wrong"
    );
    assert_eq!(
        code_of(
            e.link_badge(EMPLOYEE.into(), "zz".into(), Some(MANAGER_PIN.into()))
                .await
        ),
        "badge_token_invalid"
    );
    let link = e
        .link_badge(EMPLOYEE.into(), TOKEN.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    assert_eq!(link.insz, EMPLOYEE);
    assert_eq!(
        code_of(
            e.link_badge(
                EMPLOYEE.into(),
                "04A1B2C3D4E5F7".into(),
                Some(MANAGER_PIN.into())
            )
            .await
        ),
        "badge_already_linked"
    );

    // list_badges
    let overview = e.list_badges().await.unwrap();
    assert_eq!(overview.badges.len(), 1);
    assert_eq!(overview.badges[0].insz, EMPLOYEE);
    assert_eq!(overview.badges[0].linked_by, MANAGER_INSZ);
    let actions: Vec<_> = overview.log.iter().map(|l| l.action.as_str()).collect();
    assert_eq!(actions, ["link", "settings"]);

    // badge_tap: the manager is logged in, so John's badge is a presence toggle.
    let outcome = e.badge_tap(TOKEN.to_lowercase(), false).await.unwrap();
    assert_eq!(outcome.kind, ffi::BadgeOutcomeKind::ClockIn);
    assert_eq!(outcome.insz.as_deref(), Some(EMPLOYEE));
    assert!(outcome.at.is_some());
    assert!(
        e.list_open_shifts()
            .await
            .unwrap()
            .iter()
            .any(|s| s.user_insz == EMPLOYEE)
    );
    // The repeat window ignores a second tap.
    let again = e.badge_tap(TOKEN.into(), false).await.unwrap();
    assert_eq!(again.kind, ffi::BadgeOutcomeKind::Ignored);
    // An unknown badge is a coded refusal.
    assert_eq!(
        code_of(e.badge_tap("DEADBEEF".into(), false).await),
        "badge_unknown"
    );

    // revoke_badge
    e.revoke_badge(EMPLOYEE.into()).await.unwrap();
    assert_eq!(code_of(e.revoke_badge(EMPLOYEE.into()).await), "badge_none");
    assert!(e.list_badges().await.unwrap().badges.is_empty());
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_manager_badge_on_the_login_screen_asks_for_the_pin() {
    let t = open_seeded().await;
    let e = &t.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    e.set_badge_settings(ffi::BadgeSettingsInput {
        reader: "usb".into(),
        login_enabled: true,
        wedge_max_gap_ms: 50,
        wedge_min_len: 6,
        wedge_max_len: 32,
    })
    .await
    .unwrap();
    e.link_badge(MANAGER_INSZ.into(), TOKEN.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
    e.logout().await.unwrap();

    let outcome = e.badge_tap(TOKEN.into(), false).await.unwrap();
    assert_eq!(outcome.kind, ffi::BadgeOutcomeKind::PinRequired);
    assert_eq!(outcome.insz.as_deref(), Some(MANAGER_INSZ));
    assert!(outcome.user.is_none());
    // No session was opened.
    assert!(e.get_current_user().await.unwrap().is_none());
    e.shutdown().await.unwrap();
}
