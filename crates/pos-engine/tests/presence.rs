//! Presence belongs to the engine:
//!
//! - a successful `login` or `login_with_new_pin` queues a clock-in that
//!   the presence worker signs (event S, signWorkIn) without making the
//!   login wait; a refused login queues nothing;
//! - an employee already on shift (the screen was only locked) is not
//!   clocked in again;
//! - a clock-in the FDM refuses raises `presence-failed`;
//! - `end_shift` checks the PIN like `login`, signs one signWorkOut (and
//!   no signWorkIn) and closes the session in every case.
//!
//! The worker runs through `Engine::start_background`, the FDM is a
//! wiremock server that counts what reaches it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_engine_testkit as common;

use common::*;

use serde_json::json;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use pos_engine::commands::auth::{do_login, do_login_with_new_pin};
use pos_engine::commands::errors;
use pos_engine::commands::social::{PresenceFailed, do_end_shift};
use pos_engine::{
    ConfigSource, Engine, EngineConfig, EngineEvent, EnginePaths, EngineTimers, EventSink,
};

const FIRST: &str = "12345678958";
const SECOND: &str = "84022899837";
const THIRD: &str = "94060242576";
const PIN: &str = "4321";

#[derive(Default)]
struct Recorder(Mutex<Vec<EngineEvent>>);

impl EventSink for Recorder {
    fn emit(&self, event: EngineEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl Recorder {
    fn presence_failed(&self) -> Vec<PresenceFailed> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                EngineEvent::PresenceFailed(p) => Some(p.clone()),
                _ => None,
            })
            .collect()
    }
}

/// An engine with its presence worker running, against a wiremock FDM.
struct Rig {
    engine: Engine,
    fdm: MockServer,
    events: Arc<Recorder>,
    _dir: tempfile::TempDir,
}

impl Rig {
    /// `fdm_signs`: the FDM signs every event (the M140 and M141 envelopes,
    /// echoed back); otherwise it answers every request with an error.
    async fn start(fdm_signs: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let events = Arc::new(Recorder::default());
        let hour = Duration::from_secs(3600);
        let engine = Engine::open(EngineConfig {
            paths: EnginePaths::under(dir.path().to_path_buf()),
            config: ConfigSource::DevelopmentDefaults,
            events: events.clone(),
            timers: EngineTimers {
                auto_z: hour,
                mail_outbox: hour,
                ..EngineTimers::default()
            },
            identity: pos_engine::SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
            http_tls: pos_engine::HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        })
        .expect("open the engine");

        let fdm = MockServer::start().await;
        if fdm_signs {
            for (uc, mutation) in [("M140", "signWorkIn"), ("M141", "signWorkOut")] {
                let envelope = read_response_envelope(&load_uc(&basic_set_root(), uc));
                Mock::given(method("POST"))
                    .and(path("/graphql"))
                    .and(body_string_contains(mutation))
                    .respond_with(EchoRequest(envelope))
                    .mount(&fdm)
                    .await;
            }
        } else {
            Mock::given(method("POST"))
                .and(path("/graphql"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "errors": [{ "message": "internal FDM error (synthetic, from the test suite)" }]
                })))
                .mount(&fdm)
                .await;
        }
        engine.config.lock().unwrap().fdm_url = format!("{}/graphql", fdm.uri());

        engine.start_background(&tokio::runtime::Handle::current());
        Self {
            engine,
            fdm,
            events,
            _dir: dir,
        }
    }

    /// An active cashier, with a PIN when `pin` is given.
    fn user(&self, insz: &str, pin: Option<&str>) {
        let db = self.engine.db.lock().unwrap();
        db.execute(
            "INSERT INTO users (insz, name, role, active) VALUES (?1, ?2, 'cashier', 1)",
            rusqlite::params![insz, format!("Test {insz}")],
        )
        .unwrap();
        if let Some(pin) = pin {
            let hash = bcrypt::hash(pin, 4).unwrap();
            pos_db::queries::users::set_pin_hash(&db, insz, &hash).unwrap();
        }
    }

    /// The bodies of the sign mutations that reached the FDM.
    async fn sent(&self, mutation: &str) -> Vec<String> {
        self.fdm
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .map(|r| String::from_utf8_lossy(&r.body).to_string())
            .filter(|body| body.contains(mutation))
            .collect()
    }

    /// Wait until `n` of `mutation` reached the FDM, then give a stray
    /// extra one the time to show.
    async fn wait_for(&self, mutation: &str, n: usize) -> Vec<String> {
        let waited = tokio::time::timeout(Duration::from_secs(10), async {
            while self.sent(mutation).await.len() < n {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        assert!(waited.is_ok(), "{n} {mutation} never reached the FDM");
        self.settle().await;
        self.sent(mutation).await
    }

    async fn settle(&self) {
        tokio::time::sleep(Duration::from_millis(300)).await;
    }

    /// How many requests reached the FDM, not counting the status
    /// queries of the FDM monitor (it checks when a session opens).
    async fn requests(&self) -> usize {
        self.fdm
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| !String::from_utf8_lossy(&r.body).contains("status(language"))
            .count()
    }

    fn session_is_open(&self) -> bool {
        self.engine.session.lock().unwrap().is_some()
    }

    async fn stop(self) {
        self.engine.shutdown();
        for task in self.engine.take_background_tasks() {
            task.await.expect("a loop panicked");
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_login_and_a_login_with_a_new_pin_each_queue_one_clock_in() {
    let rig = Rig::start(true).await;
    rig.user(FIRST, None);
    rig.user(SECOND, None);

    do_login(&rig.engine, FIRST.into(), None).expect("login");
    let sent = rig.wait_for("signWorkIn", 1).await;
    assert_eq!(sent.len(), 1, "one clock-in for the login");
    assert!(sent[0].contains(FIRST));

    do_login_with_new_pin(&rig.engine, SECOND.into(), PIN.into()).expect("first PIN login");
    let sent = rig.wait_for("signWorkIn", 2).await;
    assert_eq!(sent.len(), 2, "one more for the first-PIN login");
    assert!(sent[1].contains(SECOND));
    assert!(rig.events.presence_failed().is_empty());
    assert_eq!(rig.sent("signWorkOut").await.len(), 0);
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_login_queues_nothing() {
    let rig = Rig::start(true).await;
    rig.user(FIRST, Some(PIN));
    rig.user(SECOND, None);

    let refused = do_login(&rig.engine, FIRST.into(), Some("0000".into())).unwrap_err();
    assert_eq!(refused, errors::PIN_WRONG);
    let refused = do_login(&rig.engine, FIRST.into(), None).unwrap_err();
    assert_eq!(refused, errors::PIN_REQUIRED);
    let refused = do_login(&rig.engine, "00000000000".into(), None).unwrap_err();
    assert_eq!(refused, errors::INSZ_INVALID);
    assert!(!rig.session_is_open());

    // A job queued by a refused login would run with no session (or with
    // the wrong one) and raise presence-failed before any FDM request.
    rig.settle().await;
    assert!(
        rig.events.presence_failed().is_empty(),
        "a refused login queued a clock-in"
    );
    assert_eq!(rig.requests().await, 0, "nothing reached the FDM");

    // The worker is in order: the first clock-in is the accepted login's.
    do_login(&rig.engine, SECOND.into(), None).expect("login");
    let sent = rig.wait_for("signWorkIn", 1).await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains(SECOND));
    rig.settle().await;
    assert_eq!(rig.requests().await, 1);
    assert!(rig.events.presence_failed().is_empty());
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_employee_off_shift_is_clocked_in_once() {
    let rig = Rig::start(true).await;
    rig.user(FIRST, None);

    do_login(&rig.engine, FIRST.into(), None).expect("login");
    let sent = rig.wait_for("signWorkIn", 1).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(rig.requests().await, 1, "nothing else reached the FDM");
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_employee_already_on_shift_is_not_clocked_in_again() {
    let rig = Rig::start(true).await;
    rig.user(FIRST, None);
    rig.user(SECOND, None);
    pos_db::queries::shifts::open_shift(&rig.engine.db.lock().unwrap(), FIRST).unwrap();

    do_login(&rig.engine, FIRST.into(), None).expect("login");
    rig.settle().await;
    assert_eq!(rig.requests().await, 0, "the screen was only locked");

    // The worker has gone through the first job by now: the next one is
    // still served.
    do_login(&rig.engine, SECOND.into(), None).expect("login");
    let sent = rig.wait_for("signWorkIn", 1).await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0].contains(SECOND));
    assert_eq!(rig.requests().await, 1);
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_clock_in_raises_presence_failed_and_the_login_stays() {
    let rig = Rig::start(false).await;
    rig.user(FIRST, None);

    let user = do_login(&rig.engine, FIRST.into(), None).expect("login never waits for the FDM");
    assert_eq!(user.insz, FIRST);

    let waited = tokio::time::timeout(Duration::from_secs(10), async {
        while rig.events.presence_failed().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert!(waited.is_ok(), "no presence-failed event");
    rig.settle().await;
    let failed = rig.events.presence_failed();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].insz, FIRST);
    assert!(failed[0].clock_in);
    assert!(!failed[0].error.is_empty());
    assert!(rig.session_is_open(), "the session stays open");
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_shift_with_a_wrong_pin_is_refused_like_login_and_calls_no_fdm() {
    let rig = Rig::start(true).await;
    rig.user(THIRD, Some(PIN));

    let on_login = do_login(&rig.engine, THIRD.into(), Some("0000".into())).unwrap_err();
    let on_end = do_end_shift(&rig.engine, THIRD.into(), Some("0000".into()))
        .await
        .unwrap_err();
    assert_eq!(on_end, on_login);
    assert_eq!(on_end, errors::PIN_WRONG);
    let no_pin = do_end_shift(&rig.engine, THIRD.into(), None)
        .await
        .unwrap_err();
    assert_eq!(no_pin, errors::PIN_REQUIRED);

    rig.settle().await;
    assert_eq!(rig.requests().await, 0, "no FDM request");
    assert!(!rig.session_is_open(), "no session");
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_shift_signs_one_clock_out_and_no_clock_in_then_closes_the_session() {
    let rig = Rig::start(true).await;
    rig.user(THIRD, Some(PIN));

    let result = do_end_shift(&rig.engine, THIRD.into(), Some(PIN.into()))
        .await
        .expect("end shift");
    assert!(result.success, "{:?}", result.error);
    assert!(result.error.is_none());

    rig.settle().await;
    let out = rig.sent("signWorkOut").await;
    assert_eq!(out.len(), 1, "one clock-out");
    assert!(out[0].contains(THIRD));
    assert_eq!(rig.sent("signWorkIn").await.len(), 0, "no clock-in");
    assert_eq!(rig.requests().await, 1);
    assert!(!rig.session_is_open());
    assert!(rig.events.presence_failed().is_empty());
    rig.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn end_shift_with_the_fdm_failing_reports_it_and_still_closes_the_session() {
    let rig = Rig::start(false).await;
    rig.user(THIRD, Some(PIN));

    let result = do_end_shift(&rig.engine, THIRD.into(), Some(PIN.into()))
        .await
        .expect("the failure is in the result");
    assert!(!result.success);
    assert!(result.error.as_deref().is_some_and(|e| !e.is_empty()));

    rig.settle().await;
    assert_eq!(rig.sent("signWorkIn").await.len(), 0, "no clock-in");
    assert!(!rig.session_is_open(), "the session is closed anyway");
    assert!(
        rig.events.presence_failed().is_empty(),
        "the result carries the failure, not an event"
    );
    rig.stop().await;
}
