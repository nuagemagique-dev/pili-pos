//! The engine monitors the FDM status (planner decision PD36): while a
//! session is open, never without one. A session that opens is checked at
//! once; then the checks come every `timers.fdm_poll` while the FDM
//! answers, and further and further apart while it does not. Every check
//! is emitted as `fdm-status`; `fdm_check_now` is the explicit one and
//! needs no session.
//!
//! The engine reuses a status answer for 20 s, which would hide the
//! polling from a counter of FDM requests. Where a test counts requests,
//! a helper task empties that cache every few milliseconds, so each check
//! of the monitor reaches the FDM. Waits are generous and negative
//! assertions wait several times the interval, so a slow machine cannot
//! make a test flaky.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use pos_engine_testkit as common;

use common::*;

use serde_json::json;
use tokio::task::JoinHandle;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use pos_core::user::{User, UserRole};
use pos_engine::background::run_fdm_monitor;
use pos_engine::commands::access::{close_session, open_session};
use pos_engine::commands::fdm_monitor::{FdmHealth, FdmSeverity, do_fdm_check_now, do_fdm_health};
use pos_engine::state::AppState;
use pos_engine::{EngineEvent, EventSink};

const DEADLINE: Duration = Duration::from_secs(15);
/// Long enough to prove that nothing happens.
const QUIET: Duration = Duration::from_millis(1000);
/// A port nothing listens on: the connection is refused at once.
const CLOSED_PORT_FDM: &str = "http://127.0.0.1:1/graphql";

/// Keeps the payload of every `fdm-status` event and the topic of all.
#[derive(Default)]
struct Recorder {
    health: Mutex<Vec<FdmHealth>>,
    topics: Mutex<Vec<String>>,
}

impl EventSink for Recorder {
    fn emit(&self, event: EngineEvent) {
        if let EngineEvent::FdmStatus(h) = &event {
            self.health.lock().unwrap().push(h.clone());
        }
        self.topics.lock().unwrap().push(event.topic().to_string());
    }
}

impl Recorder {
    fn health(&self) -> Vec<FdmHealth> {
        self.health.lock().unwrap().clone()
    }

    fn topics(&self) -> Vec<String> {
        self.topics.lock().unwrap().clone()
    }
}

struct Monitor {
    state: Arc<AppState>,
    fdm: MockServer,
    recorder: Arc<Recorder>,
    loop_task: JoinHandle<()>,
    clearer: JoinHandle<()>,
    _data_dir: tempfile::TempDir,
}

fn operator() -> User {
    User {
        insz: "12345678901".to_string(),
        name: "Test".to_string(),
        role: UserRole::Cashier,
        active: true,
        has_pin: false,
        presence_only: false,
    }
}

fn healthy() -> serde_json::Value {
    json!({ "data": { "fdmStatus": {
        "device": {
            "fdmId": "FDM02000425",
            "fdmSwVersion": "2.2.0.5",
            "fdmDateTime": "2026-09-27T14:40:00.000Z",
            "bufferCapacityUsed": 0.0
        },
        "initialized": true,
        "informations": [],
        "warnings": [],
        "errors": []
    } } })
}

/// An app with no session, its FDM answering with a healthy status, the
/// recorder as its sink and `fdm_poll` as the poll interval. The monitor
/// loop is not started yet.
async fn app_without_session(
    fdm_poll: Duration,
) -> (AppState, MockServer, Arc<Recorder>, tempfile::TempDir) {
    let app = TestApp::new().await;
    app.logout();
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(healthy()))
        .mount(&app.fdm)
        .await;
    let recorder = Arc::new(Recorder::default());
    let TestApp {
        mut state,
        data_dir,
        fdm,
        ..
    } = app;
    let events: Arc<dyn pos_engine::EventSink> = Arc::<Recorder>::clone(&recorder);
    state.events = events;
    state.timers.fdm_poll = fdm_poll;
    (state, fdm, recorder, data_dir)
}

/// Run the monitor loop on `state`, with the cache emptier.
fn start(
    state: AppState,
    fdm: MockServer,
    recorder: Arc<Recorder>,
    data_dir: tempfile::TempDir,
) -> Monitor {
    let state = Arc::new(state);
    let loop_task = tokio::spawn(run_fdm_monitor(Arc::clone(&state)));
    let clearer = {
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            loop {
                *state.fdm_status_cache.lock().unwrap() = None;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
    };
    Monitor {
        state,
        fdm,
        recorder,
        loop_task,
        clearer,
        _data_dir: data_dir,
    }
}

impl Monitor {
    /// Status queries that reached the FDM.
    async fn status_requests(&self) -> usize {
        self.fdm
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| String::from_utf8_lossy(&r.body).contains("status(language"))
            .count()
    }

    async fn wait_for_events(&self, count: usize) {
        let waited = tokio::time::timeout(DEADLINE, async {
            while self.recorder.health().len() < count {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        assert!(
            waited.is_ok(),
            "{count} fdm-status event(s) expected, got {}",
            self.recorder.health().len()
        );
    }

    async fn stop(self) {
        self.clearer.abort();
        self.state.shutdown_token.cancel();
        tokio::time::timeout(Duration::from_secs(5), self.loop_task)
            .await
            .expect("the monitor outlived shutdown")
            .expect("the monitor panicked");
    }
}

fn ms(t: &str) -> chrono::DateTime<chrono::FixedOffset> {
    chrono::DateTime::parse_from_rfc3339(t).expect("an RFC 3339 time")
}

/// The wait the event promises before the next check.
fn promised_wait(h: &FdmHealth) -> chrono::Duration {
    ms(h.next_check_at.as_deref().expect("a next check")) - ms(&h.checked_at)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_session_no_status_request_is_sent() {
    let (state, fdm, recorder, dir) = app_without_session(Duration::from_millis(150)).await;
    let monitor = start(state, fdm, recorder, dir);

    tokio::time::sleep(QUIET).await;
    assert_eq!(
        monitor.status_requests().await,
        0,
        "a status request without a session"
    );
    assert_eq!(monitor.recorder.health().len(), 0);
    assert!(do_fdm_health(&monitor.state).unwrap().is_none());
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_that_opens_is_checked_at_once() {
    // A poll interval far longer than the wait: the first request cannot
    // be a poll.
    let poll = Duration::from_secs(5);
    let (state, fdm, recorder, dir) = app_without_session(poll).await;
    let monitor = start(state, fdm, recorder, dir);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(monitor.status_requests().await, 0);

    let opened = Instant::now();
    open_session(&monitor.state, &operator()).unwrap();
    monitor.wait_for_events(1).await;
    assert!(
        opened.elapsed() < Duration::from_secs(3),
        "not checked at once"
    );
    assert_eq!(monitor.status_requests().await, 1);

    let health = monitor.recorder.health();
    assert_eq!(health[0].severity, FdmSeverity::Info);
    assert!(health[0].connected && health[0].answered);
    assert_eq!(health[0].code, None);
    // The session is open, so the event says when the next check comes.
    let wait = promised_wait(&health[0]);
    assert!(
        wait >= chrono::Duration::milliseconds(4900) && wait <= chrono::Duration::seconds(6),
        "the next check is promised {wait} later"
    );
    assert_eq!(monitor.recorder.topics(), vec!["fdm-status".to_string()]);
    // The last health is the one of the event.
    assert_eq!(
        do_fdm_health(&monitor.state).unwrap(),
        Some(health[0].clone())
    );
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_answering_fdm_is_checked_every_poll_interval() {
    let (state, fdm, recorder, dir) = app_without_session(Duration::from_millis(150)).await;
    let monitor = start(state, fdm, recorder, dir);
    open_session(&monitor.state, &operator()).unwrap();

    monitor.wait_for_events(4).await;
    for health in monitor.recorder.health() {
        assert!(health.answered && health.connected);
        let wait = promised_wait(&health);
        assert!(
            wait >= chrono::Duration::milliseconds(140)
                && wait < chrono::Duration::milliseconds(400),
            "the wait stays at the base while the FDM answers, got {wait}"
        );
    }
    assert!(monitor.status_requests().await >= 4);
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn polling_stops_when_the_session_closes() {
    let (state, fdm, recorder, dir) = app_without_session(Duration::from_millis(150)).await;
    let monitor = start(state, fdm, recorder, dir);
    open_session(&monitor.state, &operator()).unwrap();
    monitor.wait_for_events(3).await;

    close_session(&monitor.state).unwrap();
    // A check in flight at the close may still finish.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let requests = monitor.status_requests().await;
    let events = monitor.recorder.health().len();
    assert!(requests >= 3);

    tokio::time::sleep(QUIET * 2).await;
    assert_eq!(monitor.status_requests().await, requests, "polling went on");
    assert_eq!(monitor.recorder.health().len(), events, "events went on");
    // The stored health no longer promises a check.
    assert_eq!(
        do_fdm_health(&monitor.state)
            .unwrap()
            .unwrap()
            .next_check_at,
        None
    );

    // A new session starts the checks again, at once.
    open_session(&monitor.state, &operator()).unwrap();
    monitor.wait_for_events(events + 1).await;
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unreachable_fdm_is_checked_further_and_further_apart() {
    let (state, fdm, recorder, dir) = app_without_session(Duration::from_millis(150)).await;
    state.config.lock().unwrap().fdm_url = CLOSED_PORT_FDM.to_string();
    let monitor = start(state, fdm, recorder, dir);
    open_session(&monitor.state, &operator()).unwrap();

    monitor.wait_for_events(3).await;
    let health = monitor.recorder.health();
    for h in &health {
        assert!(!h.answered && !h.connected);
        assert_eq!(h.severity, FdmSeverity::Blocking);
        assert!(h.mandatory);
        assert!(h.code.is_some());
    }
    // 150 ms * 2^n: 300 ms, 600 ms, 1200 ms after the 1st, 2nd, 3rd failure.
    let waits: Vec<chrono::Duration> = health.iter().take(3).map(promised_wait).collect();
    assert!(waits[0] >= chrono::Duration::milliseconds(290), "{waits:?}");
    assert!(
        waits[1] >= waits[0] + chrono::Duration::milliseconds(200),
        "{waits:?}"
    );
    assert!(
        waits[2] >= waits[1] + chrono::Duration::milliseconds(400),
        "{waits:?}"
    );
    // And the checks really are that far apart.
    let gap = |a: usize, b: usize| ms(&health[b].checked_at) - ms(&health[a].checked_at);
    assert!(
        gap(0, 1) >= chrono::Duration::milliseconds(250),
        "{:?}",
        gap(0, 1)
    );
    assert!(
        gap(1, 2) >= chrono::Duration::milliseconds(500),
        "{:?}",
        gap(1, 2)
    );
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_wait_stops_growing_at_ten_times_the_interval() {
    // Five failures at 50 ms: 100, 200, 400, then 500 ms twice.
    let (state, fdm, recorder, dir) = app_without_session(Duration::from_millis(50)).await;
    state.config.lock().unwrap().fdm_url = CLOSED_PORT_FDM.to_string();
    let monitor = start(state, fdm, recorder, dir);
    open_session(&monitor.state, &operator()).unwrap();

    monitor.wait_for_events(6).await;
    let waits: Vec<i64> = monitor
        .recorder
        .health()
        .iter()
        .take(6)
        .map(|h| promised_wait(h).num_milliseconds())
        .collect();
    assert!(waits[4] >= 450 && waits[5] >= 450, "{waits:?}");
    assert!(
        waits[4] <= 800 && waits[5] <= 800,
        "the cap is 10 x, got {waits:?}"
    );
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_answer_ends_the_backoff() {
    let (state, fdm, recorder, dir) = app_without_session(Duration::from_millis(100)).await;
    state.config.lock().unwrap().fdm_url = CLOSED_PORT_FDM.to_string();
    let good_url = format!("{}/graphql", fdm.uri());
    let monitor = start(state, fdm, recorder, dir);
    open_session(&monitor.state, &operator()).unwrap();
    monitor.wait_for_events(2).await;

    // The FDM comes back.
    monitor.state.config.lock().unwrap().fdm_url = good_url;
    let seen = monitor.recorder.health().len();
    monitor.wait_for_events(seen + 2).await;
    let health = monitor.recorder.health();
    let first_answer = health
        .iter()
        .position(|h| h.answered)
        .expect("an answered check");
    let after = &health[first_answer];
    assert!(after.connected);
    let wait = promised_wait(after);
    assert!(
        wait < chrono::Duration::milliseconds(250),
        "the wait goes back to the base once the FDM answers, got {wait}"
    );
    monitor.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_check_now_without_a_session_answers_and_emits_once() {
    let (state, _fdm, recorder, _dir) = app_without_session(Duration::from_millis(150)).await;
    assert!(do_fdm_health(&state).unwrap().is_none());

    let health = do_fdm_check_now(&state).await.unwrap();
    assert!(health.answered && health.connected);
    assert_eq!(health.severity, FdmSeverity::Info);
    assert_eq!(health.next_check_at, None, "no session, no next check");
    assert_eq!(recorder.health(), vec![health.clone()]);
    assert_eq!(recorder.topics(), vec!["fdm-status".to_string()]);
    assert_eq!(do_fdm_health(&state).unwrap(), Some(health));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_check_now_with_an_unreachable_fdm_is_not_operational() {
    let (state, _fdm, recorder, _dir) = app_without_session(Duration::from_millis(150)).await;
    state.config.lock().unwrap().fdm_url = CLOSED_PORT_FDM.to_string();

    let health = do_fdm_check_now(&state).await.unwrap();
    assert!(!health.answered && !health.connected);
    assert_eq!(health.severity, FdmSeverity::Blocking);
    assert!(health.mandatory);
    assert_eq!(recorder.health().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_check_now_with_a_session_promises_the_next_check() {
    let (state, _fdm, recorder, _dir) = app_without_session(Duration::from_secs(30)).await;
    open_session(&state, &operator()).unwrap();

    let health = do_fdm_check_now(&state).await.unwrap();
    let wait = promised_wait(&health);
    assert!(
        wait >= chrono::Duration::seconds(29) && wait <= chrono::Duration::seconds(31),
        "{wait}"
    );
    assert_eq!(recorder.health().len(), 1);
}

/// The path of the FDM mock the captures run against: its warnings reach
/// the engine monitor with their `showPos`, so a mandatory code gives a
/// mandatory reading and an optional one does not.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_mock_marks_mandatory_and_optional_warnings_as_the_monitor_reads_them() {
    let (state, _fdm, _recorder, _dir) = app_without_session(Duration::from_secs(30)).await;
    for (code, mandatory) in [
        ("BUFFER_NEAR_FULL", true),
        ("CLIENT_CERT_EXPIRED", true),
        ("UPDATE_POS_VATNO", true),
        ("UPDATE_POS_ESTNO", true),
        ("CLIENT_CERT_NEAR_EXPIRATION", false),
        ("RTC_SYNC_FAILED", false),
        ("UPDATE_URLS_FAILED", false),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let mock = pos_fdm_mock::app_with(pos_fdm_mock::Control {
            warnings: vec![code.to_string()],
            ..Default::default()
        });
        tokio::spawn(async move {
            axum::serve(listener, mock).await.unwrap();
        });
        state.config.lock().unwrap().fdm_url = format!("http://{addr}/graphql");
        *state.fdm_status_cache.lock().unwrap() = None;

        let health = do_fdm_check_now(&state).await.unwrap();
        assert_eq!(health.severity, FdmSeverity::Warning, "{code}");
        assert_eq!(health.code.as_deref(), Some(code), "{code}");
        assert_eq!(health.mandatory, mandatory, "{code}");
    }
}
