//! Staff badges (lane 5): the setting, enrolment, revocation and the
//! scan (`badge_tap`).
//!
//! - the reader is off by default and then nothing reads or links badges;
//! - only a keyed hash of a badge is stored: the badge number is nowhere
//!   in the database file, the key lives in `badge.key` outside it;
//! - a lost key revokes every badge, an unreadable key revokes nothing;
//! - a manager links a badge with their own PIN, one active badge per
//!   employee and one employee per badge;
//! - a scan registers the badge holder's presence (the S event is the
//!   employee's) without logging the operator out, logs a cashier in on
//!   the login screen, only asks a manager for their PIN, and ends a
//!   shift in "Fin de service" mode;
//! - a repeat of a scan is ignored for `EngineTimers::badge_repeat`;
//! - every refusal signs nothing.
//!
//! The FDM is a wiremock server that answers S events with the M140 and
//! M141 envelopes, echoed back.

use std::sync::Arc;
use std::time::Duration;

use pos_engine_testkit as common;

use common::*;

use pos_core::user::UserRole;
use pos_engine::commands::access::{ERR_FORBIDDEN, ERR_NOT_LOGGED_IN};
use pos_engine::commands::auth::{CreateUserInput, do_create_user, do_deactivate_user, do_login};
use pos_engine::commands::badges::{
    BadgeOutcome, BadgeOutcomeKind, BadgeSettingsInput, do_badge_tap, do_get_badge_settings,
    do_link_badge, do_list_badges, do_revoke_badge, do_set_badge_settings,
};
use pos_engine::commands::errors;
use pos_engine::state::AppState;
use pos_engine::{ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers};
use pos_fiscal::types::EventLabel;
use serde_json::{Value, json};
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The test manager (`TEST_OPERATOR`, an admin) and their PIN.
const MANAGER: &str = TEST_OPERATOR;
const PIN: &str = "4321";
/// A waiter, a cashier and a presence-only kitchen employee.
const ANNA: &str = "12345678958";
const BOB: &str = "84022899837";
const KITCHEN: &str = "94060242576";

const T_ANNA: &str = "04A1B2C3";
const T_BOB: &str = "04A1B2C4";
const T_KITCHEN: &str = "04A1B2C5";
const T_MANAGER: &str = "04A1B2C6";

/// Short enough to wait out in a test, long enough for two quick taps.
const REPEAT: Duration = Duration::from_millis(300);

fn user(state: &AppState, insz: &str, name: &str, role: &str, presence_only: bool) {
    do_create_user(
        state,
        CreateUserInput {
            insz: insz.into(),
            name: name.into(),
            role: role.into(),
            presence_only,
            ..Default::default()
        },
    )
    .expect("create the user");
}

/// The manager has a PIN and the employees exist.
fn seed(state: &AppState) {
    {
        let db = state.db.lock().unwrap();
        let hash = bcrypt::hash(PIN, 4).unwrap();
        pos_db::queries::users::set_pin_hash(&db, MANAGER, &hash).unwrap();
    }
    user(state, ANNA, "Anna", "waiter", false);
    user(state, BOB, "Bob", "cashier", false);
    user(state, KITCHEN, "Cuisine", "waiter", true);
}

/// A manager with a valid INSZ (the login checks the checksum) and the
/// test PIN, logged in. `TEST_OPERATOR` is not a valid INSZ.
const REAL_MANAGER: &str = "85073003328";

fn real_manager(app: &TestApp) {
    app.login_as(REAL_MANAGER, UserRole::Admin);
    let db = app.state.db.lock().unwrap();
    let hash = bcrypt::hash(PIN, 4).unwrap();
    pos_db::queries::users::set_pin_hash(&db, REAL_MANAGER, &hash).unwrap();
}

fn set_reader(state: &AppState, reader: &str, login_enabled: bool) {
    do_set_badge_settings(
        state,
        BadgeSettingsInput {
            reader: reader.into(),
            login_enabled,
            wedge_max_gap_ms: 50,
            wedge_min_len: 6,
            wedge_max_len: 32,
        },
    )
    .expect("set the badge settings");
}

fn link(state: &AppState, insz: &str, token: &str) {
    do_link_badge(state, insz.into(), token.into(), Some(PIN.into()))
        .unwrap_or_else(|e| panic!("link {insz}: {e}"));
}

/// The S answers of the published fixtures, for every sign request.
async fn fdm_signs(fdm: &MockServer, delay: Option<Duration>) {
    for (uc, mutation) in [("M140", "signWorkIn"), ("M141", "signWorkOut")] {
        let envelope = read_response_envelope(&load_uc(&basic_set_root(), uc));
        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(body_string_contains(mutation))
            .respond_with(Slow(EchoRequest(envelope), delay))
            .mount(fdm)
            .await;
    }
}

/// `EchoRequest`, after an optional delay (a slow FDM).
struct Slow(EchoRequest, Option<Duration>);

impl wiremock::Respond for Slow {
    fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
        let answer = self.0.respond(request);
        match self.1 {
            Some(delay) => answer.set_delay(delay),
            None => answer,
        }
    }
}

async fn fdm_errors(fdm: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "errors": [{ "message": "internal FDM error (synthetic, from the test suite)" }]
        })))
        .mount(fdm)
        .await;
}

/// A manager logged in, the reader on (USB), badge login off, the FDM
/// signing S events.
async fn app() -> TestApp {
    let mut app = TestApp::new().await;
    fdm_signs(&app.fdm, None).await;
    seed(&app.state);
    app.state.timers.badge_repeat = REPEAT;
    set_reader(&app.state, "usb", false);
    app
}

async fn tap(app: &TestApp, token: &str) -> Result<BadgeOutcome, String> {
    do_badge_tap(&app.state, token.into(), false).await
}

async fn tap_end_shift(app: &TestApp, token: &str) -> Result<BadgeOutcome, String> {
    do_badge_tap(&app.state, token.into(), true).await
}

fn kind(outcome: Result<BadgeOutcome, String>) -> BadgeOutcomeKind {
    outcome.expect("a scan outcome").kind
}

async fn wait_out_the_repeat_window() {
    tokio::time::sleep(REPEAT + Duration::from_millis(100)).await;
}

/// The work-in / work-out requests that reached the FDM.
async fn work_requests(fdm: &MockServer) -> Vec<Value> {
    fdm.received_requests()
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

fn s_events(app: &TestApp) -> Vec<FiscalEventRow> {
    app.fiscal_events()
        .into_iter()
        .filter(|e| e.event_type == "S")
        .collect()
}

type Scan = (String, Option<String>, String, Option<String>);

/// `badge_scans`: outcome, employee, badge reference, detail.
fn scans(state: &AppState) -> Vec<Scan> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT outcome, user_insz, badge_ref, detail FROM badge_scans ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// `badge_log` rows: action, employee, actor, detail.
type LogRow = (String, Option<String>, Option<String>, Option<String>);

fn log_rows(state: &AppState) -> Vec<LogRow> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT action, user_insz, changed_by, detail FROM badge_log ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn active_badges(state: &AppState) -> usize {
    let db = state.db.lock().unwrap();
    db.query_row(
        "SELECT COUNT(*) FROM staff_badges WHERE revoked_at IS NULL",
        [],
        |r| r.get::<_, i64>(0),
    )
    .unwrap() as usize
}

type ShiftRow = (String, Option<String>, Option<String>, bool);

fn shifts(state: &AppState) -> Vec<ShiftRow> {
    let db = state.db.lock().unwrap();
    let mut stmt = db
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

fn session_insz(state: &AppState) -> Option<String> {
    state
        .session
        .lock()
        .unwrap()
        .as_ref()
        .map(|u| u.insz.clone())
}

fn key_path(app: &TestApp) -> std::path::PathBuf {
    app.state
        .paths
        .data_dir
        .join(pos_engine::badge_key::KEY_FILE)
}

// ── The setting ───────────────────────────────────────────────

#[tokio::test]
async fn a_fresh_install_has_the_reader_off_and_refuses_every_scan() {
    let app = TestApp::new().await;
    fdm_signs(&app.fdm, None).await;
    seed(&app.state);
    app.logout();

    // No session needed to read it (the login screen arms its reader).
    let s = do_get_badge_settings(&app.state).unwrap();
    assert_eq!(s.reader, "off");
    assert!(!s.login_enabled);
    assert_eq!(
        (s.wedge_max_gap_ms, s.wedge_min_len, s.wedge_max_len),
        (50, 6, 32)
    );
    assert!(!s.relink_needed);

    for end_shift in [false, true] {
        assert_eq!(
            do_badge_tap(&app.state, T_ANNA.into(), end_shift)
                .await
                .unwrap_err(),
            errors::BADGE_READER_OFF
        );
    }
    app.login_as(MANAGER, UserRole::Admin);
    assert_eq!(
        do_link_badge(&app.state, ANNA.into(), T_ANNA.into(), Some(PIN.into())).unwrap_err(),
        errors::BADGE_READER_OFF
    );
    assert!(scans(&app.state).is_empty(), "nothing logged");
    assert!(!key_path(&app).exists(), "no key is created for nothing");
    assert!(work_requests(&app.fdm).await.is_empty());
}

#[tokio::test]
async fn invalid_settings_are_refused_and_not_logged() {
    let app = app().await;
    let before = log_rows(&app.state).len();
    let input = |f: &dyn Fn(&mut BadgeSettingsInput)| {
        let mut i = BadgeSettingsInput {
            reader: "usb".into(),
            login_enabled: true,
            wedge_max_gap_ms: 50,
            wedge_min_len: 6,
            wedge_max_len: 32,
        };
        f(&mut i);
        i
    };
    for bad in [
        input(&|i| i.reader = "bluetooth".into()),
        input(&|i| i.reader = "".into()),
        input(&|i| i.wedge_max_gap_ms = 9),
        input(&|i| i.wedge_max_gap_ms = 201),
        input(&|i| i.wedge_min_len = 3),
        input(&|i| i.wedge_min_len = 33),
        input(&|i| i.wedge_max_len = 65),
        input(&|i| {
            i.wedge_min_len = 20;
            i.wedge_max_len = 10;
        }),
    ] {
        assert_eq!(
            do_set_badge_settings(&app.state, bad).unwrap_err(),
            errors::BADGE_SETTINGS_INVALID
        );
    }
    assert_eq!(log_rows(&app.state).len(), before, "nothing logged");
    let s = do_get_badge_settings(&app.state).unwrap();
    assert_eq!((s.reader.as_str(), s.login_enabled), ("usb", false));
}

#[tokio::test]
async fn only_a_manager_changes_the_badge_settings_and_each_change_is_logged() {
    let app = app().await;
    let input = BadgeSettingsInput {
        reader: "nfc".into(),
        login_enabled: true,
        wedge_max_gap_ms: 80,
        wedge_min_len: 8,
        wedge_max_len: 20,
    };

    // A cashier, and nobody, may not.
    app.login_as(BOB, UserRole::Cashier);
    assert_eq!(
        do_set_badge_settings(&app.state, input.clone()).unwrap_err(),
        ERR_FORBIDDEN
    );
    app.logout();
    assert_eq!(
        do_set_badge_settings(&app.state, input.clone()).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    assert_eq!(do_get_badge_settings(&app.state).unwrap().reader, "usb");

    app.login_as(MANAGER, UserRole::Admin);
    let s = do_set_badge_settings(&app.state, input.clone()).unwrap();
    assert_eq!(s.reader, "nfc");
    assert!(s.login_enabled);
    assert_eq!(
        (s.wedge_max_gap_ms, s.wedge_min_len, s.wedge_max_len),
        (80, 8, 20)
    );
    // The same values again change nothing and log nothing.
    let rows = log_rows(&app.state).len();
    do_set_badge_settings(&app.state, input).unwrap();
    assert_eq!(log_rows(&app.state).len(), rows);

    let last = log_rows(&app.state).pop().unwrap();
    assert_eq!(last.0, "settings");
    assert_eq!(last.2.as_deref(), Some(MANAGER));
    let detail: Value = serde_json::from_str(last.3.as_deref().unwrap()).unwrap();
    assert_eq!(detail["reader"], "nfc");
    assert_eq!(detail["wedge_max_gap_ms"], 80);
}

// ── Hash only, key outside the database ───────────────────────

#[tokio::test]
async fn the_raw_badge_number_is_nowhere_in_the_database_file() {
    let (mut app, db_dir) = TestApp::with_shared_db().await;
    fdm_signs(&app.fdm, None).await;
    seed(&app.state);
    app.state.timers.badge_repeat = REPEAT;
    set_reader(&app.state, "usb", false);
    link(&app.state, ANNA, T_ANNA);
    tap(&app, T_ANNA).await.unwrap();
    // A refused scan is journaled too.
    tap(&app, "DEADBEEF").await.unwrap_err();

    // Write the WAL into the file, then read every byte of both.
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .unwrap();
    let mut bytes = std::fs::read(db_dir.path().join("pos.db")).unwrap();
    for suffix in ["-wal", "-shm"] {
        if let Ok(more) = std::fs::read(db_dir.path().join(format!("pos.db{suffix}"))) {
            bytes.extend(more);
        }
    }
    let contains = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);

    let number = u32::from_str_radix(T_ANNA, 16).unwrap();
    for needle in [
        T_ANNA.to_string(),
        T_ANNA.to_lowercase(),
        number.to_string(),
        number
            .to_be_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        "DEADBEEF".to_string(),
    ] {
        assert!(
            !contains(needle.as_bytes()),
            "{needle} found in the database"
        );
    }
    assert!(
        contains(b"staff_badges"),
        "sanity: the schema is in the file"
    );

    // The key is a file of the data directory, not part of the database.
    let key = std::fs::read(key_path(&app)).unwrap();
    assert_eq!(key.len(), 32);
    assert!(!contains(&key), "the key is inside pos.db");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(key_path(&app))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
    // What is stored is the keyed hash, 64 hex characters.
    let hash: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT badge_hash FROM staff_badges", [], |r| r.get(0))
        .unwrap();
    let key: [u8; 32] = key.try_into().unwrap();
    assert_eq!(hash, pos_engine::badge_key::hash_token(&key, T_ANNA));
    // The journal carries a short reference, never the number.
    let reference = &scans(&app.state)[0].2;
    assert_eq!(reference.as_str(), &hash[..12]);
}

#[tokio::test]
async fn the_token_is_normalized_before_it_is_hashed() {
    let app = app().await;
    link(&app.state, ANNA, "  04a1b2c3\r\n");
    // The same badge, typed or read in another case or with whitespace.
    assert_eq!(kind(tap(&app, "04A1B2C3").await), BadgeOutcomeKind::ClockIn);
    wait_out_the_repeat_window().await;
    assert_eq!(
        kind(tap(&app, " 04a1b2c3 ").await),
        BadgeOutcomeKind::ClockOut
    );
    for bad in ["", "   ", "ABC", "04G1B2C3", "04A1 B2C3", &"A".repeat(65)] {
        assert_eq!(
            tap(&app, bad).await.unwrap_err(),
            errors::BADGE_TOKEN_INVALID,
            "{bad:?}"
        );
    }
    // An unreadable token is not journaled.
    assert_eq!(scans(&app.state).len(), 2);
}

// ── Lost and unreadable keys ──────────────────────────────────

#[tokio::test]
async fn a_new_key_revokes_every_active_badge_with_key_changed() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    link(&app.state, BOB, T_BOB);
    assert_eq!(active_badges(&app.state), 2);
    assert!(!do_get_badge_settings(&app.state).unwrap().relink_needed);

    // The database restored on new hardware: no key file there.
    std::fs::remove_file(key_path(&app)).unwrap();
    assert_eq!(
        tap(&app, T_ANNA).await.unwrap_err(),
        errors::BADGE_UNKNOWN,
        "the old badge is unknown under the new key"
    );
    assert_eq!(active_badges(&app.state), 0);
    let revokes: Vec<_> = log_rows(&app.state)
        .into_iter()
        .filter(|r| r.0 == "revoke")
        .collect();
    assert_eq!(revokes.len(), 2);
    assert!(
        revokes.iter().all(|r| r.2.is_none()),
        "no actor: the engine"
    );
    assert!(
        revokes
            .iter()
            .all(|r| r.3.as_deref() == Some("key_changed"))
    );
    assert!(key_path(&app).exists(), "a new key was created");
    assert!(do_get_badge_settings(&app.state).unwrap().relink_needed);

    // Linking again, under the new key, clears the banner.
    link(&app.state, ANNA, T_ANNA);
    assert!(!do_get_badge_settings(&app.state).unwrap().relink_needed);
    wait_out_the_repeat_window().await;
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
}

#[tokio::test]
async fn an_unreadable_key_refuses_and_revokes_nothing() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    let good = std::fs::read(key_path(&app)).unwrap();

    std::fs::write(key_path(&app), b"not a key").unwrap();
    assert_eq!(
        tap(&app, T_ANNA).await.unwrap_err(),
        errors::BADGE_KEY_UNAVAILABLE
    );
    assert_eq!(
        do_link_badge(&app.state, BOB.into(), T_BOB.into(), Some(PIN.into())).unwrap_err(),
        errors::BADGE_KEY_UNAVAILABLE
    );
    assert_eq!(active_badges(&app.state), 1, "nothing revoked");
    assert_eq!(
        std::fs::read(key_path(&app)).unwrap(),
        b"not a key",
        "the file is left alone"
    );
    assert!(scans(&app.state).is_empty(), "nothing logged");
    assert!(work_requests(&app.fdm).await.is_empty());

    // The key readable again: the badge works.
    std::fs::write(key_path(&app), good).unwrap();
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
}

#[tokio::test]
async fn opening_the_engine_without_the_key_revokes_the_badges() {
    let dir = tempfile::tempdir().unwrap();
    let open = || {
        Engine::open(EngineConfig {
            paths: EnginePaths::under(dir.path().to_path_buf()),
            config: ConfigSource::DevelopmentDefaults,
            events: Arc::new(pos_engine::NullSink),
            timers: EngineTimers::default(),
            identity: pos_engine::SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
            http_tls: pos_engine::HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        })
        .expect("open the engine")
    };
    {
        let engine = open();
        let db = engine.db.lock().unwrap();
        db.execute_batch(
            "INSERT INTO users (insz, name, role) VALUES ('12345678901', 'M', 'admin');
             INSERT INTO users (insz, name, role) VALUES ('12345678958', 'A', 'waiter');",
        )
        .unwrap();
        let key = pos_engine::badge_key::load_or_create(dir.path()).unwrap();
        pos_db::queries::badges::set_key_check(&db, &pos_engine::badge_key::key_check(&key))
            .unwrap();
        pos_db::queries::badges::link(
            &db,
            &pos_engine::badge_key::hash_token(&key, T_ANNA),
            ANNA,
            MANAGER,
        )
        .unwrap();
    }
    // Reopened with the same key: the badge stays.
    {
        let engine = open();
        let db = engine.db.lock().unwrap();
        assert_eq!(pos_db::queries::badges::list_active(&db).unwrap().len(), 1);
    }
    // Reopened after the key is gone: revoked, and a new key exists.
    std::fs::remove_file(dir.path().join(pos_engine::badge_key::KEY_FILE)).unwrap();
    let engine = open();
    let db = engine.db.lock().unwrap();
    assert!(
        pos_db::queries::badges::list_active(&db)
            .unwrap()
            .is_empty()
    );
    assert!(pos_db::queries::badges::relink_needed(&db).unwrap());
    assert!(dir.path().join(pos_engine::badge_key::KEY_FILE).exists());
}

// ── Enrolment ─────────────────────────────────────────────────

#[tokio::test]
async fn a_manager_links_a_badge_to_a_presence_only_employee() {
    let app = app().await;
    let linked = do_link_badge(
        &app.state,
        KITCHEN.into(),
        T_KITCHEN.into(),
        Some(PIN.into()),
    )
    .unwrap();
    assert_eq!(linked.insz, KITCHEN);
    assert_eq!(linked.name, "Cuisine");
    assert!(!linked.linked_at.is_empty());

    let overview = do_list_badges(&app.state).unwrap();
    assert_eq!(overview.badges.len(), 1);
    assert_eq!(overview.badges[0].insz, KITCHEN);
    assert_eq!(overview.badges[0].linked_by, MANAGER);
    // The overview never carries a hash.
    let json = serde_json::to_string(&overview).unwrap();
    let hash: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT badge_hash FROM staff_badges", [], |r| r.get(0))
        .unwrap();
    assert!(!json.contains(&hash));
    // The key check is stored with the first link.
    assert!(
        pos_db::queries::badges::key_check(&app.state.db.lock().unwrap())
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn link_refusals_come_in_the_documented_order() {
    let app = app().await;

    // forbidden (even with the reader off), then not logged in.
    app.login_as(BOB, UserRole::Cashier);
    assert_eq!(
        do_link_badge(&app.state, ANNA.into(), T_ANNA.into(), Some(PIN.into())).unwrap_err(),
        ERR_FORBIDDEN
    );
    app.logout();
    assert_eq!(
        do_link_badge(&app.state, ANNA.into(), T_ANNA.into(), Some(PIN.into())).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    app.login_as(MANAGER, UserRole::Admin);

    // The reader off beats a bad token and the system identity.
    set_reader(&app.state, "off", false);
    assert_eq!(
        do_link_badge(&app.state, "00000000029".into(), "zz".into(), None).unwrap_err(),
        errors::BADGE_READER_OFF
    );
    set_reader(&app.state, "usb", false);

    // Then the token, then the key, then the employee.
    assert_eq!(
        do_link_badge(&app.state, "00000000029".into(), "zz".into(), None).unwrap_err(),
        errors::BADGE_TOKEN_INVALID
    );
    std::fs::write(key_path(&app), b"bad").unwrap();
    assert_eq!(
        do_link_badge(&app.state, "00000000029".into(), T_ANNA.into(), None).unwrap_err(),
        errors::BADGE_KEY_UNAVAILABLE
    );
    std::fs::remove_file(key_path(&app)).unwrap();
    assert_eq!(
        do_link_badge(&app.state, "00000000029".into(), T_ANNA.into(), None).unwrap_err(),
        errors::SYSTEM_USER
    );
    assert_eq!(
        do_link_badge(&app.state, "99012345667".into(), T_ANNA.into(), None).unwrap_err(),
        errors::USER_UNKNOWN
    );
    // An inactive employee is unknown.
    app.login_as(BOB, UserRole::Cashier);
    app.login_as(MANAGER, UserRole::Admin);
    user(&app.state, "85073003328", "X", "waiter", false);
    do_deactivate_user(&app.state, "85073003328".into()).unwrap();
    assert_eq!(
        do_link_badge(&app.state, "85073003328".into(), T_ANNA.into(), None).unwrap_err(),
        errors::USER_UNKNOWN
    );

    // Anna and Bob have a badge each.
    link(&app.state, ANNA, T_ANNA);
    link(&app.state, BOB, T_BOB);
    // Anna again with Bob's badge: she already has one (before in-use).
    assert_eq!(
        do_link_badge(&app.state, ANNA.into(), T_BOB.into(), None).unwrap_err(),
        errors::BADGE_ALREADY_LINKED
    );
    // Someone else with Bob's badge: in use (before the PIN).
    assert_eq!(
        do_link_badge(&app.state, KITCHEN.into(), T_BOB.into(), None).unwrap_err(),
        errors::BADGE_IN_USE
    );
    // Only then the manager's PIN.
    assert_eq!(
        do_link_badge(&app.state, KITCHEN.into(), T_KITCHEN.into(), None).unwrap_err(),
        errors::PIN_REQUIRED
    );
    assert_eq!(
        do_link_badge(
            &app.state,
            KITCHEN.into(),
            T_KITCHEN.into(),
            Some("0000".into())
        )
        .unwrap_err(),
        errors::PIN_WRONG
    );
    assert_eq!(active_badges(&app.state), 2, "nothing was written");
}

#[tokio::test]
async fn a_wrong_manager_pin_links_nothing() {
    let app = app().await;
    let logged = log_rows(&app.state).len();
    for pin in ["0000", "1111"] {
        assert_eq!(
            do_link_badge(&app.state, ANNA.into(), T_ANNA.into(), Some(pin.into())).unwrap_err(),
            errors::PIN_WRONG
        );
    }
    assert_eq!(active_badges(&app.state), 0);
    assert_eq!(log_rows(&app.state).len(), logged);
    assert!(
        pos_db::queries::badges::key_check(&app.state.db.lock().unwrap())
            .unwrap()
            .is_none(),
        "no key check without a link"
    );
    // The PIN of someone else's account is not the operator's.
    assert_eq!(tap(&app, T_ANNA).await.unwrap_err(), errors::BADGE_UNKNOWN);
}

#[tokio::test]
async fn an_employee_cannot_hold_two_badges() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    assert_eq!(
        do_link_badge(&app.state, ANNA.into(), T_BOB.into(), Some(PIN.into())).unwrap_err(),
        errors::BADGE_ALREADY_LINKED
    );
    assert_eq!(active_badges(&app.state), 1);
}

#[tokio::test]
async fn a_badge_cannot_be_linked_to_two_employees() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    assert_eq!(
        do_link_badge(&app.state, BOB.into(), T_ANNA.into(), Some(PIN.into())).unwrap_err(),
        errors::BADGE_IN_USE
    );
    assert_eq!(active_badges(&app.state), 1);
}

// ── Revocation ────────────────────────────────────────────────

#[tokio::test]
async fn a_revoked_badge_is_refused_as_revoked_and_can_be_linked_again() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    do_revoke_badge(&app.state, ANNA.into()).unwrap();
    assert_eq!(active_badges(&app.state), 0);
    assert_eq!(tap(&app, T_ANNA).await.unwrap_err(), errors::BADGE_REVOKED);
    let scan = scans(&app.state);
    assert_eq!(scan.len(), 1);
    assert_eq!(scan[0].0, "revoked");
    assert_eq!(scan[0].1.as_deref(), Some(ANNA));

    // The same card can be issued again, to the same or another person.
    link(&app.state, BOB, T_ANNA);
    wait_out_the_repeat_window().await;
    let outcome = tap(&app, T_ANNA).await.unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::ClockIn);
    assert_eq!(outcome.insz.as_deref(), Some(BOB));
}

#[tokio::test]
async fn revoke_without_badge_is_badge_none() {
    let app = app().await;
    assert_eq!(
        do_revoke_badge(&app.state, ANNA.into()).unwrap_err(),
        errors::BADGE_NONE
    );
    link(&app.state, ANNA, T_ANNA);
    do_revoke_badge(&app.state, ANNA.into()).unwrap();
    assert_eq!(
        do_revoke_badge(&app.state, ANNA.into()).unwrap_err(),
        errors::BADGE_NONE
    );
    // Managers only; removing needs no PIN and works with the reader off.
    app.login_as(BOB, UserRole::Cashier);
    assert_eq!(
        do_revoke_badge(&app.state, ANNA.into()).unwrap_err(),
        ERR_FORBIDDEN
    );
    assert_eq!(do_list_badges(&app.state).unwrap_err(), ERR_FORBIDDEN);
    app.logout();
    assert_eq!(
        do_revoke_badge(&app.state, ANNA.into()).unwrap_err(),
        ERR_NOT_LOGGED_IN
    );
    app.login_as(MANAGER, UserRole::Admin);
    link(&app.state, ANNA, T_ANNA);
    set_reader(&app.state, "off", false);
    do_revoke_badge(&app.state, ANNA.into()).unwrap();
    assert!(do_list_badges(&app.state).unwrap().badges.is_empty());
}

#[tokio::test]
async fn deactivating_an_employee_revokes_their_badge() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    do_deactivate_user(&app.state, ANNA.into()).unwrap();
    assert_eq!(active_badges(&app.state), 0);
    let revoke = log_rows(&app.state)
        .into_iter()
        .find(|r| r.0 == "revoke")
        .expect("a revoke row");
    assert_eq!(revoke.1.as_deref(), Some(ANNA));
    assert_eq!(revoke.2.as_deref(), Some(MANAGER));
    assert_eq!(revoke.3.as_deref(), Some("deactivated"));
    assert_eq!(tap(&app, T_ANNA).await.unwrap_err(), errors::BADGE_REVOKED);
    // Someone with no badge deactivates as before.
    do_deactivate_user(&app.state, BOB.into()).unwrap();

    // An employee on shift is not deactivated, and keeps their badge.
    link(&app.state, KITCHEN, T_KITCHEN);
    pos_db::queries::shifts::open_shift(&app.state.db.lock().unwrap(), KITCHEN, None).unwrap();
    assert_eq!(
        do_deactivate_user(&app.state, KITCHEN.into()).unwrap_err(),
        errors::DEACTIVATE_ON_SHIFT
    );
    assert_eq!(active_badges(&app.state), 1);
}

#[tokio::test]
async fn every_link_and_revocation_is_logged_with_the_manager() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    do_revoke_badge(&app.state, ANNA.into()).unwrap();
    link(&app.state, ANNA, T_BOB);

    let rows = log_rows(&app.state);
    let actions: Vec<_> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(actions, ["settings", "link", "revoke", "link"]);
    for row in &rows {
        assert_eq!(row.2.as_deref(), Some(MANAGER), "{row:?}");
    }
    assert_eq!(rows[2].3.as_deref(), Some("manager"));

    // The overview shows them newest first, with names.
    let overview = do_list_badges(&app.state).unwrap();
    assert_eq!(overview.log.len(), 4);
    assert_eq!(overview.log[0].action, "link");
    assert_eq!(overview.log[0].name.as_deref(), Some("Anna"));
    assert_eq!(overview.log[0].by.as_deref(), Some(MANAGER));
}

// ── Presence by badge ─────────────────────────────────────────

#[tokio::test]
async fn a_scan_while_a_cashier_is_logged_in_clocks_the_badge_holder_in_and_keeps_the_session() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    app.login_as(BOB, UserRole::Cashier);
    let counter = app.counter_for(EventLabel::S);

    let outcome = tap(&app, T_ANNA).await.unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::ClockIn);
    assert_eq!(outcome.insz.as_deref(), Some(ANNA));
    assert_eq!(outcome.name.as_deref(), Some("Anna"));
    let at = outcome.at.expect("the time of the registration");
    assert!(at.ends_with('Z') && at.len() == 20, "{at}");
    assert!(outcome.error.is_none() && outcome.user.is_none());

    // The cashier is still logged in.
    assert_eq!(session_insz(&app.state).as_deref(), Some(BOB));

    // One signWorkIn in the badge holder's name.
    let sent = work_requests(&app.fdm).await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0]["query"].as_str().unwrap().contains("signWorkIn"));
    assert_eq!(sent[0]["variables"]["data"]["employeeId"], ANNA);
    let events = s_events(&app);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].user_insz, ANNA);
    assert_eq!(app.counter_for(EventLabel::S), counter + 1);
    // Self-registered: no manager on record.
    assert_eq!(shifts(&app.state), vec![(ANNA.into(), None, None, true)]);

    let scan = scans(&app.state);
    assert_eq!(scan.len(), 1);
    assert_eq!(scan[0].0, "clock_in");
    assert_eq!(scan[0].1.as_deref(), Some(ANNA));
    assert_eq!(scan[0].2.len(), 12);
    assert_eq!(scan[0].3, None);
}

#[tokio::test]
async fn a_second_scan_after_the_window_clocks_out() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
    wait_out_the_repeat_window().await;
    let out = tap(&app, T_ANNA).await.unwrap();
    assert_eq!(out.kind, BadgeOutcomeKind::ClockOut);
    assert_eq!(out.insz.as_deref(), Some(ANNA));

    let sent = work_requests(&app.fdm).await;
    assert_eq!(sent.len(), 2);
    assert!(sent[1]["query"].as_str().unwrap().contains("signWorkOut"));
    assert_eq!(sent[1]["variables"]["data"]["employeeId"], ANNA);
    assert_eq!(s_events(&app).len(), 2);
    assert_eq!(
        shifts(&app.state),
        vec![(ANNA.into(), None, None, false)],
        "the shift is closed, by the employee"
    );
    let outcomes: Vec<_> = scans(&app.state).into_iter().map(|s| s.0).collect();
    assert_eq!(outcomes, ["clock_in", "clock_out"]);
}

#[tokio::test]
async fn the_operators_own_badge_is_refused_as_presence_self() {
    let app = app().await;
    link(&app.state, MANAGER, T_MANAGER);
    assert_eq!(
        tap(&app, T_MANAGER).await.unwrap_err(),
        errors::PRESENCE_SELF
    );
    assert!(work_requests(&app.fdm).await.is_empty());
    assert!(s_events(&app).is_empty());
    let scan = scans(&app.state);
    assert_eq!(scan.len(), 1);
    assert_eq!(scan[0].0, "refused");
    assert_eq!(scan[0].3.as_deref(), Some(errors::PRESENCE_SELF));
}

#[tokio::test]
async fn an_fdm_failure_on_a_badge_scan_archives_nothing() {
    let app = TestApp::new().await;
    fdm_errors(&app.fdm).await;
    seed(&app.state);
    set_reader(&app.state, "usb", false);
    link(&app.state, ANNA, T_ANNA);
    let counter = app.counter_for(EventLabel::S);

    let outcome = tap(&app, T_ANNA).await.unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::PresenceFailed);
    assert_eq!(outcome.insz.as_deref(), Some(ANNA));
    let error = outcome.error.expect("the FDM text");
    assert!(error.starts_with("FDM"), "{error}");
    assert!(outcome.at.is_none());

    assert!(s_events(&app).is_empty(), "nothing archived");
    assert!(shifts(&app.state).is_empty(), "the index is unchanged");
    assert_eq!(app.counter_for(EventLabel::S), counter);
    let scan = scans(&app.state);
    assert_eq!(scan[0].0, "presence_failed");
    assert_eq!(scan[0].3.as_deref(), Some(error.as_str()));
}

#[tokio::test]
async fn in_training_a_badge_scan_is_refused_and_signs_nothing() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    app.state.config.lock().unwrap().training_mode = true;
    assert_eq!(
        tap(&app, T_ANNA).await.unwrap_err(),
        errors::PRESENCE_TRAINING
    );
    wait_out_the_repeat_window().await;
    // Also in "Fin de service" mode, on the login screen.
    app.logout();
    assert_eq!(
        tap_end_shift(&app, T_ANNA).await.unwrap_err(),
        errors::PRESENCE_TRAINING
    );
    assert!(work_requests(&app.fdm).await.is_empty());
    assert!(app.fiscal_events().is_empty());
    assert!(shifts(&app.state).is_empty());
    assert!(scans(&app.state).iter().all(|s| s.0 == "refused"));
}

// ── Repeats ───────────────────────────────────────────────────

#[tokio::test]
async fn a_double_tap_never_clocks_in_and_out() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
    // The card is still on the reader: ignored, however often.
    for _ in 0..3 {
        let again = tap(&app, T_ANNA).await.unwrap();
        assert_eq!(again.kind, BadgeOutcomeKind::Ignored);
        assert!(again.insz.is_none() && again.name.is_none() && again.at.is_none());
    }
    assert_eq!(work_requests(&app.fdm).await.len(), 1);
    assert_eq!(s_events(&app).len(), 1);
    assert_eq!(
        scans(&app.state).len(),
        1,
        "an ignored repeat is not logged"
    );
}

#[tokio::test]
async fn a_scan_during_a_slow_fdm_answer_is_ignored() {
    let mut app = TestApp::new().await;
    // The FDM takes longer than the repeat window to answer.
    fdm_signs(&app.fdm, Some(Duration::from_millis(600))).await;
    seed(&app.state);
    app.state.timers.badge_repeat = Duration::from_millis(400);
    set_reader(&app.state, "usb", false);
    link(&app.state, ANNA, T_ANNA);

    let (first, second) = tokio::join!(tap(&app, T_ANNA), async {
        tokio::time::sleep(Duration::from_millis(100)).await;
        tap(&app, T_ANNA).await
    });
    assert_eq!(first.unwrap().kind, BadgeOutcomeKind::ClockIn);
    assert_eq!(second.unwrap().kind, BadgeOutcomeKind::Ignored);
    assert_eq!(work_requests(&app.fdm).await.len(), 1);

    // The window runs from the END of the first scan: a tap right after
    // the slow answer is still ignored, and one after the window is not.
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::Ignored);
    tokio::time::sleep(Duration::from_millis(450)).await;
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockOut);
    assert_eq!(work_requests(&app.fdm).await.len(), 2);
}

#[tokio::test]
async fn two_badges_are_not_debounced_together() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    link(&app.state, BOB, T_BOB);
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
    assert_eq!(kind(tap(&app, T_BOB).await), BadgeOutcomeKind::ClockIn);
    assert_eq!(work_requests(&app.fdm).await.len(), 2);
    assert_eq!(
        shifts(&app.state).len(),
        2,
        "both are on shift: {:?}",
        shifts(&app.state)
    );
}

#[tokio::test]
async fn refusals_are_debounced_too() {
    let app = app().await;
    assert_eq!(
        tap(&app, "DEADBEEF").await.unwrap_err(),
        errors::BADGE_UNKNOWN
    );
    assert_eq!(kind(tap(&app, "DEADBEEF").await), BadgeOutcomeKind::Ignored);
    assert_eq!(scans(&app.state).len(), 1);
    wait_out_the_repeat_window().await;
    assert_eq!(
        tap(&app, "DEADBEEF").await.unwrap_err(),
        errors::BADGE_UNKNOWN
    );
    assert_eq!(scans(&app.state).len(), 2);
}

#[tokio::test]
async fn unknown_and_revoked_badges_are_refused_and_logged() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    do_revoke_badge(&app.state, ANNA.into()).unwrap();
    assert_eq!(
        tap(&app, "DEADBEEF").await.unwrap_err(),
        errors::BADGE_UNKNOWN
    );
    assert_eq!(tap(&app, T_ANNA).await.unwrap_err(), errors::BADGE_REVOKED);
    let scan = scans(&app.state);
    let outcomes: Vec<_> = scan
        .iter()
        .map(|s| (s.0.as_str(), s.1.as_deref()))
        .collect();
    assert_eq!(outcomes, [("unknown", None), ("revoked", Some(ANNA))]);
    assert_ne!(scan[0].2, scan[1].2, "a reference per badge");
    assert!(work_requests(&app.fdm).await.is_empty());
    assert!(app.fiscal_events().is_empty());
}

// ── The login screen ──────────────────────────────────────────

#[tokio::test]
async fn a_presence_only_badge_on_the_login_screen_toggles_presence() {
    let app = app().await;
    set_reader(&app.state, "usb", true);
    link(&app.state, KITCHEN, T_KITCHEN);
    app.logout();
    let outcome = tap(&app, T_KITCHEN).await.unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::ClockIn);
    assert_eq!(session_insz(&app.state), None, "never a session");
    assert_eq!(work_requests(&app.fdm).await.len(), 1);
    assert_eq!(s_events(&app)[0].user_insz, KITCHEN);
}

#[tokio::test]
async fn with_badge_login_off_the_login_screen_toggles_presence() {
    let app = app().await;
    link(&app.state, BOB, T_BOB);
    app.logout();
    let outcome = tap(&app, T_BOB).await.unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::ClockIn);
    assert_eq!(session_insz(&app.state), None);
    assert!(outcome.user.is_none());
}

#[tokio::test]
async fn a_manager_badge_opens_no_session_and_manager_commands_stay_refused() {
    let app = app().await;
    real_manager(&app);
    set_reader(&app.state, "usb", true);
    link(&app.state, REAL_MANAGER, T_MANAGER);
    app.logout();

    let outcome = tap(&app, T_MANAGER).await.unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::PinRequired);
    assert_eq!(outcome.insz.as_deref(), Some(REAL_MANAGER));
    assert!(outcome.user.is_none());
    assert_eq!(session_insz(&app.state), None);
    assert!(work_requests(&app.fdm).await.is_empty(), "nothing signed");
    assert_eq!(scans(&app.state)[0].0, "pin_required");

    // No manager command works on the strength of a badge.
    assert_eq!(do_list_badges(&app.state).unwrap_err(), ERR_NOT_LOGGED_IN);
    assert_eq!(
        do_set_badge_settings(
            &app.state,
            BadgeSettingsInput {
                reader: "off".into(),
                login_enabled: false,
                wedge_max_gap_ms: 50,
                wedge_min_len: 6,
                wedge_max_len: 32,
            }
        )
        .unwrap_err(),
        ERR_NOT_LOGGED_IN
    );

    // The session comes only from the PIN.
    let user = do_login(&app.state, REAL_MANAGER.into(), Some(PIN.into())).unwrap();
    assert_eq!(user.role, UserRole::Admin);
    assert!(do_list_badges(&app.state).is_ok());
}

#[tokio::test]
async fn the_pin_login_after_pin_required_opens_the_manager_session() {
    let app = app().await;
    real_manager(&app);
    set_reader(&app.state, "usb", true);
    link(&app.state, REAL_MANAGER, T_MANAGER);
    app.logout();
    assert_eq!(
        kind(tap(&app, T_MANAGER).await),
        BadgeOutcomeKind::PinRequired
    );
    assert_eq!(
        do_login(&app.state, REAL_MANAGER.into(), Some("0000".into())).unwrap_err(),
        errors::PIN_WRONG
    );
    assert_eq!(session_insz(&app.state), None);
    do_login(&app.state, REAL_MANAGER.into(), Some(PIN.into())).unwrap();
    assert_eq!(session_insz(&app.state).as_deref(), Some(REAL_MANAGER));
}

#[tokio::test]
async fn end_shift_mode_clocks_out_without_a_session() {
    let app = app().await;
    link(&app.state, ANNA, T_ANNA);
    link(&app.state, BOB, T_BOB);
    link(&app.state, MANAGER, T_MANAGER);
    // Anna clocks in first (a session open), then the screen is locked.
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
    app.logout();
    wait_out_the_repeat_window().await;

    let out = tap_end_shift(&app, T_ANNA).await.unwrap();
    assert_eq!(out.kind, BadgeOutcomeKind::ClockOut);
    assert_eq!(session_insz(&app.state), None, "no session opened");
    let sent = work_requests(&app.fdm).await;
    assert_eq!(sent.len(), 2);
    assert!(sent[1]["query"].as_str().unwrap().contains("signWorkOut"));
    assert_eq!(sent[1]["variables"]["data"]["employeeId"], ANNA);

    // Not on shift: presence_not_in, nothing signed, not a clock-in.
    assert_eq!(
        tap_end_shift(&app, T_BOB).await.unwrap_err(),
        errors::PRESENCE_NOT_IN
    );
    assert_eq!(work_requests(&app.fdm).await.len(), 2);

    // A manager ends their shift by badge too: no PIN, no session.
    pos_db::queries::shifts::open_shift(&app.state.db.lock().unwrap(), MANAGER, None).unwrap();
    assert_eq!(
        kind(tap_end_shift(&app, T_MANAGER).await),
        BadgeOutcomeKind::ClockOut
    );
    assert_eq!(session_insz(&app.state), None);
}

#[tokio::test]
async fn a_presence_scan_does_not_restart_the_idle_timer() {
    let app = app().await;
    set_reader(&app.state, "usb", true);
    link(&app.state, ANNA, T_ANNA);
    link(&app.state, BOB, T_BOB);
    app.state.session_rules.touch();
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Someone else's badge on the order screen: not the operator's activity.
    assert_eq!(kind(tap(&app, T_ANNA).await), BadgeOutcomeKind::ClockIn);
    let idle = app.state.session_rules.last_activity().elapsed();
    assert!(idle >= Duration::from_millis(150), "{idle:?}");

    // A badge login opens a session: that restarts the idle time.
    app.logout();
    assert_eq!(kind(tap(&app, T_BOB).await), BadgeOutcomeKind::LoggedIn);
    let idle = app.state.session_rules.last_activity().elapsed();
    assert!(idle < Duration::from_millis(150), "{idle:?}");
}

// ── Badge login through the engine, with its presence worker ──

/// An engine with its presence worker running against a wiremock FDM.
struct Rig {
    engine: Engine,
    fdm: MockServer,
    _dir: tempfile::TempDir,
}

impl Rig {
    async fn start(training: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let hour = Duration::from_secs(3600);
        let engine = Engine::open(EngineConfig {
            paths: EnginePaths::under(dir.path().to_path_buf()),
            config: ConfigSource::DevelopmentDefaults,
            events: Arc::new(pos_engine::NullSink),
            timers: EngineTimers {
                auto_z: hour,
                mail_outbox: hour,
                badge_repeat: REPEAT,
                ..EngineTimers::default()
            },
            identity: pos_engine::SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
            http_tls: pos_engine::HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        })
        .expect("open the engine");

        let fdm = MockServer::start().await;
        for (uc, mutation) in [("M140", "signWorkIn"), ("M141", "signWorkOut")] {
            let mut envelope = read_response_envelope(&load_uc(&basic_set_root(), uc));
            if training {
                envelope = in_training(envelope);
            }
            Mock::given(method("POST"))
                .and(path("/graphql"))
                .and(body_string_contains(mutation))
                .respond_with(EchoRequest(envelope))
                .mount(&fdm)
                .await;
        }
        {
            let mut config = engine.config.lock().unwrap();
            config.fdm_url = format!("{}/graphql", fdm.uri());
            config.training_mode = training;
        }
        engine.start_background(&tokio::runtime::Handle::current());
        Self {
            engine,
            fdm,
            _dir: dir,
        }
    }

    /// The employees of the tests, a manager with a PIN to link with.
    fn seed(&self) {
        {
            let db = self.engine.db.lock().unwrap();
            db.execute(
                "INSERT INTO users (insz, name, role, active) VALUES (?1, 'Manager', 'admin', 1)",
                [MANAGER],
            )
            .unwrap();
            let hash = bcrypt::hash(PIN, 4).unwrap();
            pos_db::queries::users::set_pin_hash(&db, MANAGER, &hash).unwrap();
        }
        // Log the manager in without the presence worker clocking them in.
        *self.engine.session.lock().unwrap() =
            pos_db::queries::users::get_user(&self.engine.db.lock().unwrap(), MANAGER).unwrap();
        user(&self.engine, BOB, "Bob", "cashier", false);
        set_reader(&self.engine, "usb", true);
        link(&self.engine, BOB, T_BOB);
        *self.engine.session.lock().unwrap() = None;
    }

    async fn requests(&self, mutation: &str) -> Vec<Value> {
        self.fdm
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter_map(|r| serde_json::from_slice::<Value>(&r.body).ok())
            .filter(|b| b["query"].as_str().is_some_and(|q| q.contains(mutation)))
            .collect()
    }

    async fn wait_for(&self, mutation: &str, n: usize) -> Vec<Value> {
        let waited = tokio::time::timeout(Duration::from_secs(10), async {
            while self.requests(mutation).await.len() < n {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(waited.is_ok(), "{n} {mutation} never reached the FDM");
        tokio::time::sleep(Duration::from_millis(300)).await;
        self.requests(mutation).await
    }

    async fn stop(self) {
        self.engine.shutdown();
        for task in self.engine.take_background_tasks() {
            task.await.expect("a loop panicked");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cashier_logs_in_with_a_badge_and_is_clocked_in() {
    let rig = Rig::start(false).await;
    rig.seed();
    let outcome = do_badge_tap(&rig.engine, T_BOB.into(), false)
        .await
        .unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::LoggedIn);
    assert_eq!(outcome.insz.as_deref(), Some(BOB));
    assert_eq!(outcome.user.as_ref().map(|u| u.insz.as_str()), Some(BOB));
    assert_eq!(session_insz(&rig.engine).as_deref(), Some(BOB));

    // The presence worker clocks them in, as after a PIN login.
    let sent = rig.wait_for("signWorkIn", 1).await;
    assert_eq!(sent.len(), 1, "one clock-in for the login");
    assert_eq!(sent[0]["variables"]["data"]["employeeId"], BOB);
    assert_eq!(scans(&rig.engine)[0].0, "login");
    assert_eq!(rig.requests("signWorkOut").await.len(), 0);
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn in_training_a_badge_login_opens_no_shift() {
    let rig = Rig::start(true).await;
    rig.seed();
    let outcome = do_badge_tap(&rig.engine, T_BOB.into(), false)
        .await
        .unwrap();
    assert_eq!(outcome.kind, BadgeOutcomeKind::LoggedIn, "login is allowed");
    assert_eq!(session_insz(&rig.engine).as_deref(), Some(BOB));
    // The worker sends a T WorkIn, which registers no presence.
    let sent = rig.wait_for("signWorkIn", 1).await;
    assert_eq!(sent.len(), 1);
    assert!(shifts(&rig.engine).is_empty());
    assert!(
        pos_engine::commands::social::do_list_open_shifts(&rig.engine)
            .unwrap()
            .is_empty()
    );
    rig.stop().await;
}
