//! The owner alerts: a Telegram message when the FDM, a printer,
//! the card terminal or the scale is down, the day has no Z, or an online order
//! waits. Everything runs against a local mock of the Bot API, never
//! the real Telegram.
//!
//! The worker is driven pass by pass with a chosen clock
//! (`alerts::run_pass`), so the debounce, the reminder and the hourly
//! cap are tested without waiting.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, TimeZone, Utc};
use pos_engine_testkit as common;
use serde_json::{Value, json};

use common::*;
use pos_core::config::{AlertsConfig, ScaleCheckValue, ScaleConfig};
use pos_core::user::UserRole;
use pos_engine::alerts::rules::{Kind, Transition};
use pos_engine::alerts::run_pass;
use pos_engine::commands::alerts::{
    AlertKinds, OwnerAlertsInput, do_get_owner_alerts, do_list_owner_alert_log, do_send_test_alert,
    do_set_owner_alerts,
};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::fdm_monitor::do_fdm_check_now;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::report::do_generate_z_report;
use pos_scale_mock::MockScale;

const TOKEN: &str = "123456:SECRET-bot-token_xyz";
const CHAT: &str = "-1001234567890";
const MANAGER: &str = "85073003328";
const CLOSED_PORT_FDM: &str = "http://127.0.0.1:1/graphql";

// ── The mock Bot API ──────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Mode {
    Ok,
    Fail(u16),
    Hang,
}

#[derive(Clone)]
struct Hit {
    path: String,
    body: Value,
}

#[derive(Clone)]
struct Telegram {
    base: String,
    hits: Arc<Mutex<Vec<Hit>>>,
    mode: Arc<Mutex<Mode>>,
}

impl Telegram {
    async fn start() -> Telegram {
        let hits = Arc::new(Mutex::new(Vec::new()));
        let mode = Arc::new(Mutex::new(Mode::Ok));
        let app = Router::new()
            .route("/{bot}/sendMessage", post(handle))
            .with_state((Arc::clone(&hits), Arc::clone(&mode)));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Telegram { base, hits, mode }
    }

    fn set(&self, mode: Mode) {
        *self.mode.lock().unwrap() = mode;
    }

    fn hits(&self) -> Vec<Hit> {
        self.hits.lock().unwrap().clone()
    }

    /// The text of every message, oldest first.
    fn texts(&self) -> Vec<String> {
        self.hits()
            .iter()
            .map(|h| h.body["text"].as_str().unwrap_or_default().to_string())
            .collect()
    }
}

/// What the mock Bot API shares with the test: the requests seen and the mode.
type MockState = (Arc<Mutex<Vec<Hit>>>, Arc<Mutex<Mode>>);

async fn handle(
    State((hits, mode)): State<MockState>,
    Path(bot): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    hits.lock().unwrap().push(Hit {
        path: format!("/{bot}/sendMessage"),
        body,
    });
    let mode = *mode.lock().unwrap();
    match mode {
        Mode::Ok => Json(json!({ "ok": true, "result": { "message_id": 1 } })).into_response(),
        Mode::Fail(code) => (
            StatusCode::from_u16(code).unwrap(),
            Json(json!({ "ok": false, "description": "no" })),
        )
            .into_response(),
        Mode::Hang => {
            tokio::time::sleep(Duration::from_secs(30)).await;
            Json(json!({ "ok": true })).into_response()
        }
    }
}

// ── The app ───────────────────────────────────────────────────────

/// 2026-10-17 12:00 UTC (14:00 in Brussels).
fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 10, 17, 12, 0, 0).unwrap()
}

fn minutes(n: i64) -> DateTime<Utc> {
    t0() + chrono::Duration::minutes(n)
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "test helper: the flag names what the call switches"
)]
fn input(enabled: bool) -> OwnerAlertsInput {
    OwnerAlertsInput {
        enabled,
        chat_id: CHAT.into(),
        label: "Salle".into(),
        language: "fr".into(),
        kinds: AlertKinds::default(),
        z_deadline_hour: 6,
        online_wait_minutes: 5,
        reminder_minutes: 60,
    }
}

/// A logged-in app (a manager) whose `pos.json` has the alerts section
/// pointing at `tg`, with the token file written. The alerts are still
/// off.
async fn app(tg: &Telegram) -> (TestApp, tempfile::TempDir) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("telegram.token");
    std::fs::write(&file, format!("{TOKEN}\n")).unwrap();
    app.state.config.lock().unwrap().alerts = Some(AlertsConfig {
        telegram_bot_token_file: Some(file.to_string_lossy().into_owned()),
        telegram_api_base: Some(tg.base.clone()),
    });
    app.state.owner_alerts.set_backoff(Duration::from_millis(5));
    app.login_as(MANAGER, UserRole::Admin);
    app.state.session.lock().unwrap().as_mut().unwrap().name = "Jean Dupont".into();
    (app, dir)
}

fn switch_on(app: &TestApp, mut tweak: impl FnMut(&mut OwnerAlertsInput)) {
    let mut i = input(true);
    tweak(&mut i);
    do_set_owner_alerts(&app.state, i).unwrap();
}

/// The FDM stops answering (a port nothing listens on) and the monitor's
/// state is refreshed.
async fn fdm_down(app: &TestApp) {
    app.state.config.lock().unwrap().fdm_url = CLOSED_PORT_FDM.to_string();
    let health = do_fdm_check_now(&app.state).await.unwrap();
    assert!(!health.connected);
}

async fn fdm_up(app: &TestApp) {
    app.use_admission_mock().await;
    let health = do_fdm_check_now(&app.state).await.unwrap();
    assert!(health.connected, "{health:?}");
}

fn log(app: &TestApp) -> Vec<(String, String, String)> {
    let mut rows: Vec<_> = do_list_owner_alert_log(&app.state, 200)
        .unwrap()
        .into_iter()
        .map(|r| (r.kind, r.transition, r.outcome))
        .collect();
    rows.reverse();
    rows
}

fn open_period(app: &TestApp, date: &str) {
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO booking_periods (id, booking_date) VALUES ('p-1', ?1)",
            [date],
        )
        .unwrap();
}

fn kinds_only(f: impl Fn(&mut AlertKinds)) -> AlertKinds {
    let mut k = AlertKinds {
        fdm: false,
        printer: false,
        terminal: false,
        scale: false,
        z_missing: false,
        online_waiting: false,
    };
    f(&mut k);
    k
}

// ── Off by default ────────────────────────────────────────────────

#[tokio::test]
async fn nothing_is_sent_while_the_alerts_are_off() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    // Off by default, whatever is wrong.
    let settings = do_get_owner_alerts(&app.state).unwrap();
    assert!(!settings.enabled);
    assert!(settings.token_configured);
    fdm_down(&app).await;
    open_period(&app, "2026-10-10");
    for i in 0..5 {
        assert_eq!(run_pass(&app.state, minutes(i * 10)).await.len(), 0);
    }
    // Configured but switched off again: still nothing.
    switch_on(&app, |_| {});
    do_set_owner_alerts(&app.state, input(false)).unwrap();
    assert_eq!(run_pass(&app.state, minutes(100)).await.len(), 0);
    assert!(tg.hits().is_empty());
    assert_eq!(do_list_owner_alert_log(&app.state, 10).unwrap().len(), 0);
}

#[tokio::test]
async fn nothing_is_sent_without_a_token() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    app.state.config.lock().unwrap().alerts = None;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
    // A file that is not there is no token either.
    app.state.config.lock().unwrap().alerts = Some(AlertsConfig {
        telegram_bot_token_file: Some("/nonexistent/token".into()),
        telegram_api_base: Some(tg.base.clone()),
    });
    assert!(!do_get_owner_alerts(&app.state).unwrap().token_configured);
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
    assert!(tg.hits().is_empty());
}

#[tokio::test]
async fn a_switched_off_kind_is_never_watched() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.fdm = true));
    open_period(&app, "2026-10-10");
    // The Z is a month late, but only the FDM is watched.
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
    assert!(tg.hits().is_empty());
}

// ── The FDM: debounce, reminder, resolve ──────────────────────────

#[tokio::test]
async fn the_fdm_is_raised_after_two_minutes_reminded_and_resolved() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.fdm = true));
    fdm_down(&app).await;

    assert!(run_pass(&app.state, t0()).await.is_empty(), "first sight");
    let at = |s: i64| t0() + chrono::Duration::seconds(s);
    assert_eq!(run_pass(&app.state, at(119)).await.len(), 0);
    let sent = run_pass(&app.state, at(121)).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].kind, Kind::Fdm);
    assert_eq!(sent[0].transition, Transition::Raised);
    assert_eq!(sent[0].outcome, "sent");

    // The request is the Bot API's: token in the path, chat id, text.
    let hits = tg.hits();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, format!("/bot{TOKEN}/sendMessage"));
    assert_eq!(hits[0].body["chat_id"], CHAT);
    assert_eq!(hits[0].body["disable_web_page_preview"], true);
    let text = hits[0].body["text"].as_str().unwrap();
    assert!(text.starts_with("ALERTE [Salle]"), "{text}");
    assert!(text.contains("module fiscal"), "{text}");

    // Raised once only.
    assert_eq!(run_pass(&app.state, at(300)).await.len(), 0);
    assert_eq!(run_pass(&app.state, minutes(59)).await.len(), 0);
    // A reminder an hour after the raise.
    let again = run_pass(&app.state, at(121 + 3600)).await;
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].transition, Transition::Reminder);
    assert!(tg.texts()[1].starts_with("RAPPEL [Salle]"));

    // The FDM comes back: resolved once.
    fdm_up(&app).await;
    let resolved = run_pass(&app.state, at(121 + 3700)).await;
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].transition, Transition::Resolved);
    assert!(tg.texts()[2].starts_with("RÉSOLU [Salle]"));
    assert_eq!(run_pass(&app.state, at(121 + 3800)).await.len(), 0);
    assert_eq!(tg.hits().len(), 3);

    assert_eq!(
        log(&app),
        vec![
            ("fdm".into(), "raised".into(), "sent".into()),
            ("fdm".into(), "reminder".into(), "sent".into()),
            ("fdm".into(), "resolved".into(), "sent".into()),
        ]
    );
}

#[tokio::test]
async fn a_short_blip_of_the_fdm_sends_nothing() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.fdm = true));
    fdm_down(&app).await;
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
    fdm_up(&app).await;
    assert_eq!(run_pass(&app.state, minutes(1)).await.len(), 0);
    fdm_down(&app).await;
    assert_eq!(run_pass(&app.state, minutes(2)).await.len(), 0);
    assert_eq!(run_pass(&app.state, minutes(3)).await.len(), 0);
    assert!(tg.hits().is_empty());
}

#[tokio::test]
async fn the_fdm_is_only_judged_while_a_session_is_open() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.fdm = true));
    fdm_down(&app).await;
    app.logout();
    for i in 0..5 {
        assert_eq!(run_pass(&app.state, minutes(i * 5)).await.len(), 0);
    }
    assert!(tg.hits().is_empty());
    app.login_as(MANAGER, UserRole::Admin);
    assert_eq!(run_pass(&app.state, minutes(30)).await.len(), 0);
    assert_eq!(run_pass(&app.state, minutes(33)).await.len(), 1);
}

// ── A restart, the cap, a failing endpoint ────────────────────────

#[tokio::test]
async fn a_restart_does_not_raise_an_open_alert_a_second_time() {
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");
    assert_eq!(run_pass(&app.state, t0()).await.len(), 1);

    // The till restarts: the worker's memory is gone, the log is not.
    app.state.owner_alerts = pos_engine::alerts::AlertRuntime::default();
    app.state.owner_alerts.set_backoff(Duration::from_millis(5));
    assert_eq!(run_pass(&app.state, minutes(10)).await.len(), 0);
    assert_eq!(tg.hits().len(), 1, "no second raise");

    // The reminder keeps its schedule across the restart.
    let reminder = run_pass(&app.state, minutes(61)).await;
    assert_eq!(reminder.len(), 1);
    assert_eq!(reminder[0].transition, Transition::Reminder);

    // And a Z made after the restart resolves it.
    {
        let db = app.state.db.lock().unwrap();
        db.execute_batch(
            "DROP TRIGGER booking_periods_no_delete;
             DELETE FROM booking_periods;",
        )
        .unwrap();
    }
    app.state.owner_alerts = pos_engine::alerts::AlertRuntime::default();
    app.state.owner_alerts.set_backoff(Duration::from_millis(5));
    let resolved = run_pass(&app.state, minutes(62)).await;
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].transition, Transition::Resolved);
}

/// A restart whose log cannot be read for one pass: that pass sends
/// nothing (a blank book would raise the open alert a second time), and
/// the next one restores the book and carries on as if nothing happened.
#[tokio::test]
async fn an_unreadable_log_after_a_restart_skips_the_pass() {
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");
    assert_eq!(run_pass(&app.state, t0()).await.len(), 1);

    app.state.owner_alerts = pos_engine::alerts::AlertRuntime::default();
    app.state.owner_alerts.set_backoff(Duration::from_millis(5));
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch("ALTER TABLE owner_alert_log RENAME TO owner_alert_log_away")
        .unwrap();
    assert_eq!(run_pass(&app.state, minutes(10)).await.len(), 0);
    assert_eq!(tg.hits().len(), 1, "nothing sent with a blank book");

    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch("ALTER TABLE owner_alert_log_away RENAME TO owner_alert_log")
        .unwrap();
    assert_eq!(run_pass(&app.state, minutes(11)).await.len(), 0);
    assert_eq!(tg.hits().len(), 1, "no second raise once restored");
    let reminder = run_pass(&app.state, minutes(61)).await;
    assert_eq!(reminder.len(), 1);
    assert_eq!(reminder[0].transition, Transition::Reminder);
}

#[tokio::test]
async fn an_old_open_alert_is_raised_again_after_a_restart() {
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");
    assert_eq!(run_pass(&app.state, t0()).await.len(), 1);
    app.state.owner_alerts = pos_engine::alerts::AlertRuntime::default();
    app.state.owner_alerts.set_backoff(Duration::from_millis(5));
    // A day later (older than the reminder period): a fresh raise.
    let sent = run_pass(&app.state, minutes(24 * 60)).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].transition, Transition::Raised);
}

#[tokio::test]
async fn the_hourly_cap_drops_the_twenty_first_message_and_logs_it() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    // A reminder every minute is the quickest way to many messages.
    switch_on(&app, |i| {
        i.kinds = kinds_only(|k| k.z_missing = true);
        i.reminder_minutes = 1;
    });
    open_period(&app, "2026-10-10");
    let mut outcomes = Vec::new();
    for minute in 0..22 {
        for sent in run_pass(&app.state, minutes(minute)).await {
            outcomes.push(sent.outcome);
        }
    }
    assert_eq!(outcomes.len(), 22);
    assert!(outcomes[..20].iter().all(|o| *o == "sent"), "{outcomes:?}");
    assert_eq!(&outcomes[20..], ["suppressed", "suppressed"]);
    assert_eq!(tg.hits().len(), 20, "the cap keeps the phone quiet");
    let rows = log(&app);
    assert_eq!(rows.iter().filter(|r| r.2 == "suppressed").count(), 2);

    // After a restart the window is not forgotten.
    let mut app = app;
    app.state.owner_alerts = pos_engine::alerts::AlertRuntime::default();
    app.state.owner_alerts.set_backoff(Duration::from_millis(5));
    let sent = run_pass(&app.state, minutes(23)).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].outcome, "suppressed");
    // An hour later it is open again.
    let later = run_pass(&app.state, minutes(65)).await;
    assert_eq!(later.last().unwrap().outcome, "sent");
}

#[tokio::test]
async fn a_failing_endpoint_is_retried_three_times_then_dropped() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");
    tg.set(Mode::Fail(502));
    let sent = run_pass(&app.state, t0()).await;
    assert_eq!(sent[0].outcome, "failed");
    assert_eq!(tg.hits().len(), 3, "three attempts");
    let rows = do_list_owner_alert_log(&app.state, 10).unwrap();
    assert_eq!(rows[0].outcome, "failed");
    assert_eq!(rows[0].http_status, Some(502));
    // The next pass does not hammer it; the reminder tries again.
    assert_eq!(run_pass(&app.state, minutes(5)).await.len(), 0);
    tg.set(Mode::Ok);
    let reminder = run_pass(&app.state, minutes(60)).await;
    assert_eq!(reminder[0].transition, Transition::Reminder);
    assert_eq!(reminder[0].outcome, "sent");
}

#[tokio::test]
async fn a_refusal_that_repeating_cannot_cure_is_not_retried() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");
    tg.set(Mode::Fail(401));
    let sent = run_pass(&app.state, t0()).await;
    assert_eq!(sent[0].outcome, "failed");
    assert_eq!(tg.hits().len(), 1);
}

// ── The other kinds ───────────────────────────────────────────────

#[tokio::test]
async fn the_z_is_missing_after_the_deadline_hour() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-16");
    // 06:00 in Brussels on the 17th is 04:00 UTC.
    let before = Utc.with_ymd_and_hms(2026, 10, 17, 3, 59, 0).unwrap();
    let after = Utc.with_ymd_and_hms(2026, 10, 17, 4, 0, 0).unwrap();
    assert_eq!(run_pass(&app.state, before).await.len(), 0);
    assert_eq!(run_pass(&app.state, after).await.len(), 1);
    let text = &tg.texts()[0];
    assert!(text.contains("Le Z du 16.10.2026 n'est pas fait"), "{text}");
    assert!(text.contains("avant 6 h"), "{text}");
}

#[tokio::test]
async fn the_default_deadline_is_one_hour_after_the_day_end() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    // day_end_hour is 5 in the test identity: the Z is due at 06:00.
    let saved = do_get_owner_alerts(&app.state).unwrap();
    assert_eq!(saved.z_deadline_hour, 6);
    app.state.config.lock().unwrap().day_end_hour = 3;
    assert_eq!(do_get_owner_alerts(&app.state).unwrap().z_deadline_hour, 4);
}

#[tokio::test]
async fn a_training_till_has_no_z_to_miss() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    app.state.config.lock().unwrap().training_mode = true;
    open_period(&app, "2026-10-10");
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
}

async fn printer_mock(
    control: pos_printer_mock::Control,
) -> (String, Arc<pos_printer_mock::Printer>) {
    let printer = pos_printer_mock::Printer::new(control, None);
    let addr = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), Arc::clone(&printer))
        .await
        .unwrap();
    (addr.to_string(), printer)
}

#[tokio::test]
async fn a_printer_out_of_paper_is_raised_on_the_second_probe() {
    use pos_printer_mock::{Control, Paper};
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let (addr, printer) = printer_mock(Control::default()).await;
    app.state.config.lock().unwrap().printer_address = Some(addr);
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.printer = true));

    assert!(run_pass(&app.state, t0()).await.is_empty(), "paper is fine");
    printer.set_control(Control {
        paper: Paper::End,
        ..Control::default()
    });
    // The probes are five minutes apart; passes between them look at nothing.
    assert!(
        run_pass(&app.state, minutes(5)).await.is_empty(),
        "first sight"
    );
    assert_eq!(run_pass(&app.state, minutes(6)).await.len(), 0);
    let sent = run_pass(&app.state, minutes(10)).await;
    assert_eq!(sent.len(), 1, "second probe");
    assert_eq!(sent[0].kind, Kind::Printer);
    let text = &tg.texts()[0];
    assert!(text.contains("tickets : plus de papier"), "{text}");

    printer.set_control(Control::default());
    assert!(
        run_pass(&app.state, minutes(12)).await.is_empty(),
        "no probe yet"
    );
    let resolved = run_pass(&app.state, minutes(15)).await;
    assert_eq!(resolved[0].transition, Transition::Resolved);
}

#[tokio::test]
async fn an_unplugged_printer_and_an_open_cover_are_told_apart() {
    use pos_printer_mock::Control;
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let (ticket, _ticket_printer) = printer_mock(Control {
        cover_open: true,
        ..Control::default()
    })
    .await;
    {
        let mut c = app.state.config.lock().unwrap();
        c.printer_address = Some(ticket);
        // The kitchen printer is off: nothing listens there.
        c.kitchen_printer_address = Some("127.0.0.1:1".into());
    }
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.printer = true));
    run_pass(&app.state, t0()).await;
    let sent = run_pass(&app.state, minutes(5)).await;
    assert_eq!(sent.len(), 1);
    let text = &tg.texts()[0];
    assert!(text.contains("tickets : capot ouvert"), "{text}");
    assert!(text.contains("cuisine : hors ligne"), "{text}");
}

#[tokio::test]
async fn a_printer_is_not_probed_while_a_sale_is_signing() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    // A listener that counts the connections it gets.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let connections = Arc::new(Mutex::new(0usize));
    let counted = Arc::clone(&connections);
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        while let Ok((_stream, _)) = listener.accept().await {
            *counted.lock().unwrap() += 1;
        }
    });
    app.state.config.lock().unwrap().printer_address = Some(addr);
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.printer = true));
    // Hold the signing lock, as a sale being signed does.
    let guard = pos_engine::commands::signing::try_acquire(&app.state).expect("free");
    run_pass(&app.state, t0()).await;
    run_pass(&app.state, minutes(5)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(*connections.lock().unwrap(), 0, "no probe during a sale");
    drop(guard);
    // The sale is over: the next pass probes.
    run_pass(&app.state, minutes(10)).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(*connections.lock().unwrap(), 1);
}

#[tokio::test]
async fn the_terminal_is_raised_on_the_second_failed_probe_and_never_probed_while_paying() {
    use pos_eft::EftError;
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    let script = app.link_terminal();
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.terminal = true));
    script.on_check(Err(EftError::Unreachable("down".into())));
    script.on_check(Err(EftError::Unreachable("down".into())));
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
    let sent = run_pass(&app.state, minutes(5)).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].kind, Kind::Terminal);
    assert!(tg.texts()[0].contains("terminal de carte"));

    // A payment holds the line: the probe asks nothing and judges nothing.
    let calls_before = script.calls().len();
    let line = app.state.eft_line.lock().await;
    assert_eq!(run_pass(&app.state, minutes(10)).await.len(), 0);
    assert_eq!(
        script.calls().len(),
        calls_before,
        "no probe on a busy line"
    );
    drop(line);
    // The terminal is back (the script answers Ok by default).
    let resolved = run_pass(&app.state, minutes(15)).await;
    assert_eq!(resolved[0].transition, Transition::Resolved);
}

// ── The scale ──────────────────────────────────────────────────────

fn link_scale(app: &TestApp, address: String, checksum: &str) {
    app.state.config.lock().unwrap().scale = Some(ScaleConfig {
        tcp_address: address,
        price_digits: 5,
        check_values: vec![ScaleCheckValue {
            checksum: checksum.into(),
            correction: "F336".into(),
        }],
        weigh_timeout_ms: 2000,
    });
}

#[tokio::test]
async fn the_scale_is_raised_on_the_second_failed_probe_and_resolved_when_it_answers() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let scale = MockScale::new("4711F336");
    let address = scale.spawn().await;
    // Nobody listens on a port that was just freed.
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let closed = free.local_addr().unwrap().to_string();
    drop(free);
    link_scale(&app, closed, "4711");
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.scale = true));

    assert_eq!(
        run_pass(&app.state, t0()).await,
        Vec::<pos_engine::alerts::Sent>::new()
    );
    let sent = run_pass(&app.state, minutes(5)).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].kind, Kind::Scale);
    assert!(tg.texts()[0].contains("La balance ne répond plus"));

    link_scale(&app, address.to_string(), "4711");
    let resolved = run_pass(&app.state, minutes(10)).await;
    assert_eq!(resolved[0].transition, Transition::Resolved);
    assert!(tg.texts()[1].contains("Tout est rentré dans l'ordre"));
}

#[tokio::test]
async fn a_scale_that_refuses_the_check_is_raised_with_its_own_sentence() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let scale = MockScale::new("4711F336");
    let address = scale.spawn().await;
    link_scale(&app, address.to_string(), "9999");
    scale.require_check();
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.scale = true));

    assert_eq!(
        run_pass(&app.state, t0()).await,
        Vec::<pos_engine::alerts::Sent>::new()
    );
    let sent = run_pass(&app.state, minutes(5)).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].kind, Kind::Scale);
    assert!(tg.texts()[0].contains("refuse le contrôle du logiciel"));
    let logged = do_list_owner_alert_log(&app.state, 1).unwrap();
    assert_eq!(logged[0].detail.as_deref(), Some("check=refused"));

    // The right values are installed: the next probe checks again.
    link_scale(&app, address.to_string(), "4711");
    scale.require_check();
    let resolved = run_pass(&app.state, minutes(10)).await;
    assert_eq!(resolved[0].transition, Transition::Resolved);
}

#[tokio::test]
async fn no_scale_means_no_scale_alert() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.scale = true));
    for n in [0, 5, 10] {
        assert_eq!(
            run_pass(&app.state, minutes(n)).await,
            Vec::<pos_engine::alerts::Sent>::new()
        );
    }
    assert_eq!(tg.texts(), Vec::<String>::new());
}

#[tokio::test]
async fn a_scale_alert_switched_off_sends_nothing() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let closed = free.local_addr().unwrap().to_string();
    drop(free);
    link_scale(&app, closed, "4711");
    switch_on(&app, |i| i.kinds = kinds_only(|_| {}));
    for n in [0, 5, 10] {
        assert_eq!(
            run_pass(&app.state, minutes(n)).await,
            Vec::<pos_engine::alerts::Sent>::new()
        );
    }
    assert_eq!(tg.texts(), Vec::<String>::new());
}

#[tokio::test]
async fn a_probe_on_a_held_line_is_skipped_at_once_and_sends_nothing() {
    use pos_eft::EftError;
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    let script = app.link_terminal();
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.terminal = true));
    // Even a terminal that would hang is not asked: the line is taken.
    script.set_check_delay(Duration::from_secs(30));
    script.on_check(Err(EftError::Unreachable("down".into())));
    let line = app.state.eft_line.lock().await;
    let started = Instant::now();
    for n in 0..3 {
        assert_eq!(run_pass(&app.state, minutes(5 * n)).await.len(), 0);
    }
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "a busy line is skipped, never waited for"
    );
    drop(line);
    assert!(script.calls().is_empty(), "the terminal was never asked");
    assert!(tg.texts().is_empty(), "nothing was sent");
}

#[tokio::test]
async fn a_hanging_terminal_is_dropped_after_the_short_probe_timeout() {
    use pos_eft::EftError;
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    let script = app.link_terminal();
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.terminal = true));
    assert!(
        app.state.timers.terminal_probe <= Duration::from_secs(12),
        "the default probe timeout is short"
    );
    app.state.timers.terminal_probe = Duration::from_millis(150);
    script.set_check_delay(Duration::from_secs(30));
    script.on_check(Err(EftError::Unreachable("down".into())));
    let started = Instant::now();
    // A probe that times out judges nothing, even repeated.
    for n in 0..3 {
        assert_eq!(run_pass(&app.state, minutes(5 * n)).await.len(), 0);
    }
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(tg.texts().is_empty(), "a timeout is not a failed probe");
    // The line was released by the dropped probe.
    assert!(app.state.eft_line.try_lock().is_ok());
}

#[tokio::test]
async fn a_payment_started_during_a_probe_waits_at_most_the_probe_timeout() {
    use pos_engine::commands::eft::do_eft_pay;
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    let script = app.link_terminal();
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.terminal = true));
    let timeout = Duration::from_millis(300);
    app.state.timers.terminal_probe = timeout;
    script.set_check_delay(Duration::from_secs(30));
    let order = app.order_of(1500);
    script.on_pay(Ok(approval(1500, "Bancontact", pos_eft::CardType::Debit)));

    let probe = run_pass(&app.state, t0());
    let pay = async {
        // Let the probe take the line first.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let started = Instant::now();
        let payment = do_eft_pay(&app.state, order, 1500).await.unwrap();
        (payment, started.elapsed())
    };
    let (_, (payment, waited)) = tokio::join!(probe, pay);
    assert_eq!(payment.status, "approved");
    assert!(
        waited < timeout + Duration::from_secs(2),
        "the payment waited {waited:?} behind a hanging probe"
    );
    // The probe really held the line first: it asked the terminal before the payment did, and
    // the payment only got through once the probe timeout cut the probe short.
    let calls = script.calls();
    assert_eq!(
        calls.first(),
        Some(&pos_eft::scripted::Call::Check),
        "{calls:?}"
    );
    assert!(
        matches!(calls.get(1), Some(pos_eft::scripted::Call::Pay { .. })),
        "{calls:?}"
    );
    assert!(
        waited >= timeout.saturating_sub(Duration::from_millis(200)),
        "the payment did not wait for the probe: {waited:?}"
    );
}

/// Worldline's check asks v2 and, on a 404, v1: two requests of `CHECK_TIMEOUT`. A terminal that
/// answers within that worst case must never be judged "unknown" by the probe clock (INT-D25).
#[test]
fn the_default_probe_timeout_outlasts_the_slowest_terminal_check() {
    let worst_case = 2 * pos_eft::worldline::CHECK_TIMEOUT;
    let timers = pos_engine::EngineTimers::default();
    assert_eq!(
        timers.terminal_probe,
        pos_engine::alerts::TERMINAL_PROBE_TIMEOUT
    );
    assert!(
        timers.terminal_probe >= worst_case + Duration::from_secs(1),
        "probe timeout {:?} vs worst case {worst_case:?}",
        timers.terminal_probe
    );
}

/// An online order that arrived at `arrived` (SQLite time), waiting.
fn webshop_order(app: &TestApp, hubrise_id: &str, channel: &str, arrived: &str) {
    let db = app.state.db.lock().unwrap();
    db.execute(
        "INSERT INTO orders (order_type, status, user_insz, hubrise_order_id, source, raw_hubrise_json)
         VALUES ('takeaway', 'open', '00000000029', ?1, 'hubrise', ?2)",
        rusqlite::params![
            hubrise_id,
            json!({ "customer": { "first_name": "Marie", "last_name": "Curie",
                                   "phone": "0470123456" }, "total": "47.80 EUR" }).to_string()
        ],
    )
    .unwrap();
    let order_id = db.last_insert_rowid();
    db.execute(
        "INSERT INTO webshop_orders (order_id, hubrise_order_id, channel, channel_kind, resolved_at)
         VALUES (?1, ?2, ?3, 'webshop', ?4)",
        rusqlite::params![order_id, hubrise_id, channel, arrived],
    )
    .unwrap();
}

#[tokio::test]
async fn an_online_order_waiting_too_long_is_raised_with_counts_and_channels_only() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.online_waiting = true));
    webshop_order(&app, "ho-1", "webshop", "2026-10-17 11:50:00");
    webshop_order(&app, "ho-2", "Uber Eats", "2026-10-17 11:55:00");
    webshop_order(&app, "ho-3", "webshop", "2026-10-17 11:59:00");

    // The switch is off: HubRise is not in use, nothing to wait for.
    app.state.hubrise_switch.set(false);
    assert_eq!(run_pass(&app.state, t0()).await.len(), 0);
    app.state.hubrise_switch.set(true);

    // At 12:00 two orders have waited 5 minutes or more.
    let sent = run_pass(&app.state, t0()).await;
    assert_eq!(sent.len(), 1);
    let text = &tg.texts()[0];
    assert_eq!(
        text,
        "ALERTE [Salle]\n2 commande(s) en ligne attendent depuis plus de 5 min (Uber Eats, webshop)."
    );
    for private in ["Marie", "Curie", "0470", "47.80", "47,80"] {
        assert!(!text.contains(private), "{text}");
    }

    // They are accepted: resolved.
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE webshop_orders SET fiscal_state = 'signed_p'", [])
        .unwrap();
    let resolved = run_pass(&app.state, minutes(2)).await;
    assert_eq!(resolved[0].transition, Transition::Resolved);
}

// ── The test message, secrets, privacy ────────────────────────────

#[tokio::test]
async fn the_test_message_is_sent_and_logged() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    do_set_owner_alerts(&app.state, input(false)).unwrap();
    // Allowed with the alerts switched off: that is how a chat is checked.
    let result = do_send_test_alert(&app.state).await.unwrap();
    assert!(result.sent);
    assert!(tg.texts()[0].starts_with("TEST [Salle]"));
    let rows = do_list_owner_alert_log(&app.state, 10).unwrap();
    assert_eq!(
        (rows[0].kind.as_str(), rows[0].transition.as_str()),
        ("test", "test")
    );
    assert_eq!(rows[0].outcome, "sent");
    assert_eq!(rows[0].http_status, Some(200));
}

#[tokio::test]
async fn the_test_message_reports_each_way_it_can_fail() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    // No chat id yet.
    assert_eq!(
        do_send_test_alert(&app.state).await.unwrap_err(),
        "alerts_invalid|field=chat_id"
    );
    do_set_owner_alerts(&app.state, input(false)).unwrap();

    // The endpoint refuses.
    tg.set(Mode::Fail(400));
    let err = do_send_test_alert(&app.state).await.unwrap_err();
    assert_eq!(err, "alerts_send_failed|detail=HTTP 400");
    let rows = do_list_owner_alert_log(&app.state, 10).unwrap();
    assert_eq!(rows[0].outcome, "failed");
    assert_eq!(rows[0].http_status, Some(400));
    assert_eq!(tg.hits().len(), 1, "a test is tried once");

    // Nothing answers.
    app.state
        .config
        .lock()
        .unwrap()
        .alerts
        .as_mut()
        .unwrap()
        .telegram_api_base = Some("http://127.0.0.1:1".into());
    let err = do_send_test_alert(&app.state).await.unwrap_err();
    assert!(
        err.starts_with("alerts_send_failed|detail=connection failed"),
        "{err}"
    );
    assert!(!err.contains(TOKEN), "{err}");

    // No token.
    app.state.config.lock().unwrap().alerts = None;
    assert_eq!(
        do_send_test_alert(&app.state).await.unwrap_err(),
        errors::ALERTS_TOKEN_MISSING
    );
}

#[tokio::test]
async fn only_a_manager_reads_or_changes_the_alerts() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as("12345678958", role);
        assert_eq!(do_get_owner_alerts(&app.state).unwrap_err(), "forbidden");
        assert_eq!(
            do_set_owner_alerts(&app.state, input(true)).unwrap_err(),
            "forbidden"
        );
        assert_eq!(
            do_send_test_alert(&app.state).await.unwrap_err(),
            "forbidden"
        );
        assert_eq!(
            do_list_owner_alert_log(&app.state, 10).unwrap_err(),
            "forbidden"
        );
    }
    app.logout();
    assert_eq!(
        do_get_owner_alerts(&app.state).unwrap_err(),
        "not_logged_in"
    );
}

#[tokio::test]
async fn a_change_is_logged_with_the_manager_and_the_token_is_never_returned() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let saved = do_set_owner_alerts(&app.state, input(true)).unwrap();
    assert!(saved.token_configured);
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains(TOKEN), "{json}");
    assert!(!json.contains("SECRET"), "{json}");
    {
        let db = app.state.db.lock().unwrap();
        let by: String = db
            .query_row(
                "SELECT changed_by FROM owner_settings_log WHERE key = 'alerts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(by, MANAGER);
        let stored: String = db
            .query_row(
                "SELECT value_json FROM owner_settings WHERE key = 'alerts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!stored.contains("SECRET"));
    }
    // An identical save writes no second log row.
    do_set_owner_alerts(&app.state, input(true)).unwrap();
    let count: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM owner_settings_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    // A bad value is refused with the field and stores nothing.
    let mut bad = input(true);
    bad.chat_id = "abc".into();
    assert_eq!(
        do_set_owner_alerts(&app.state, bad).unwrap_err(),
        "alerts_invalid|field=chat_id"
    );
}

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn the_token_reaches_no_log_line_no_error_and_no_log_row() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    let sink = Sink::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer({
            let sink = sink.clone();
            move || sink.clone()
        })
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let mut errors_seen = Vec::new();
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    open_period(&app, "2026-10-10");

    // A failing endpoint, a dead one, a hung-up one: every way to fail.
    tg.set(Mode::Fail(500));
    run_pass(&app.state, t0()).await;
    errors_seen.push(do_send_test_alert(&app.state).await.unwrap_err());
    app.state
        .config
        .lock()
        .unwrap()
        .alerts
        .as_mut()
        .unwrap()
        .telegram_api_base = Some("http://127.0.0.1:1".into());
    run_pass(&app.state, minutes(60)).await;
    errors_seen.push(do_send_test_alert(&app.state).await.unwrap_err());

    let trace = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
    assert!(
        trace.contains("owner alert"),
        "the failure is logged: {trace}"
    );
    assert!(!trace.contains(TOKEN), "{trace}");
    assert!(!trace.contains("SECRET"), "{trace}");
    for e in &errors_seen {
        assert!(!e.contains("SECRET"), "{e}");
    }
    let rows = serde_json::to_string(&do_list_owner_alert_log(&app.state, 200).unwrap()).unwrap();
    assert!(!rows.contains("SECRET"), "{rows}");
}

#[tokio::test]
async fn a_message_names_no_person_no_amount_and_no_order() {
    let tg = Telegram::start().await;
    let (mut app, _dir) = app(&tg).await;
    // A sale by a named manager, 12,34 euros, leaves its booking period
    // open; a month later its Z is missing.
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
                VALUES (1, 1, 1234, 'B', 0, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                VALUES ('menu_item', 1, 'name', 'fr', 'Burger Secret');",
        )
        .unwrap();
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: pos_core::order::OrderType::Takeaway,
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
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    let sale = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);

    for (n, language) in ["fr", "en", "zh"].into_iter().enumerate() {
        let tg2 = Telegram::start().await;
        app.state
            .config
            .lock()
            .unwrap()
            .alerts
            .as_mut()
            .unwrap()
            .telegram_api_base = Some(tg2.base.clone());
        app.state.owner_alerts = pos_engine::alerts::AlertRuntime::default();
        app.state.owner_alerts.set_backoff(Duration::from_millis(5));
        switch_on(&app, |i| {
            i.language = language.into();
            i.kinds = kinds_only(|k| k.z_missing = true);
        });
        // Days apart, so each language raises afresh (the log remembers).
        let far = Utc::now() + chrono::Duration::days(30 + 2 * i64::try_from(n).unwrap());
        assert_eq!(run_pass(&app.state, far).await.len(), 1, "{language}");
        let text = tg2.texts().remove(0);
        for private in [
            MANAGER, "Dupont", "Jean", "12,34", "12.34", "1234", "Burger", "Secret",
        ] {
            assert!(!text.contains(private), "{language}: {text}");
        }
        // Not even a number that looks like an INSZ or an amount.
        assert!(
            !text
                .chars()
                .collect::<Vec<_>>()
                .windows(5)
                .any(|w| w.iter().all(char::is_ascii_digit)),
            "{language}: {text}"
        );
    }
}

// ── A hung endpoint never delays the till ─────────────────────────

#[tokio::test]
async fn a_hung_endpoint_delays_neither_a_sale_nor_a_z() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
                VALUES (1, 1, 1000, 'B', 0, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                VALUES ('menu_item', 1, 'name', 'fr', 'Burger');",
        )
        .unwrap();
    #[expect(
        clippy::items_after_statements,
        reason = "a local helper beside the one test that uses it"
    )]
    async fn sale(app: &TestApp) {
        let order = do_create_order(
            &app.state,
            CreateOrderInput {
                order_type: pos_core::order::OrderType::Takeaway,
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
                menu_item_id: 1,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
        let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap();
        assert!(r.success, "{:?}", r.error);
    }
    sale(&app).await;
    switch_on(&app, |i| i.kinds = kinds_only(|k| k.z_missing = true));
    tg.set(Mode::Hang);

    // The send hangs (30 s); a sale and a Z run meanwhile.
    let far = Utc::now() + chrono::Duration::days(3);
    let pass = run_pass(&app.state, far);
    tokio::pin!(pass);
    let till = async {
        // Let the pass reach its send first.
        tokio::time::sleep(Duration::from_millis(300)).await;
        let started = Instant::now();
        sale(&app).await;
        let z = do_generate_z_report(&app.state, MANAGER.into())
            .await
            .unwrap();
        assert!(z.success, "{:?}", z.error);
        started.elapsed()
    };
    let took = tokio::select! {
        _ = &mut pass => panic!("the hung send cannot have finished"),
        took = till => took,
    };
    assert_eq!(tg.hits().len(), 1, "the send was in flight during the sale");
    assert!(took < Duration::from_secs(5), "the till waited {took:?}");
}

// ── The rest of the worker ────────────────────────────────────────

#[tokio::test]
async fn the_worker_loop_is_idle_while_off_and_stops_at_shutdown() {
    let tg = Telegram::start().await;
    let (app, _dir) = app(&tg).await;
    assert!(!pos_engine::alerts::is_enabled(&app.state));
    switch_on(&app, |_| {});
    assert!(pos_engine::alerts::is_enabled(&app.state));
    let TestApp {
        state,
        data_dir,
        fdm,
        ..
    } = app;
    let state = Arc::new(state);
    let task = tokio::spawn(pos_engine::alerts::run_owner_alerts(Arc::clone(&state)));
    state.shutdown_token.cancel();
    tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("the loop stops")
        .unwrap();
    drop((data_dir, fdm));
}
