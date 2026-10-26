//! The engine's background loops: they run on the runtime the shell
//! hands over, tick at the configured intervals, and stop on shutdown.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_engine::{
    ConfigSource, Engine, EngineConfig, EngineEvent, EnginePaths, EngineTimers, EventSink,
};

#[derive(Default)]
struct Recorder(Mutex<Vec<&'static str>>);

impl EventSink for Recorder {
    fn emit(&self, event: EngineEvent) {
        self.0.lock().unwrap().push(event.topic());
    }
}

fn open(dir: &std::path::Path, sink: Arc<Recorder>) -> Engine {
    Engine::open(EngineConfig {
        paths: EnginePaths::under(dir.to_path_buf()),
        config: ConfigSource::DevelopmentDefaults,
        events: sink,
        timers: EngineTimers {
            auto_z: Duration::from_millis(10),
            mail_outbox: Duration::from_millis(10),
            ..EngineTimers::default()
        },
        identity: pos_engine::SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
        http_tls: pos_engine::HttpTls::Platform,
        device_id_override: None,
        support_program: None,
    })
    .expect("open the engine")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_loops_tick_and_stop_on_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let engine = open(dir.path(), Arc::new(Recorder::default()));
    assert_eq!(engine.auto_z_passes(), 0);
    assert_eq!(engine.mail_outbox_passes(), 0);

    engine.start_background(&tokio::runtime::Handle::current());

    // The automatic Z and the mail outbox each run at least once.
    let waited = tokio::time::timeout(Duration::from_secs(10), async {
        while engine.auto_z_passes() < 2 || engine.mail_outbox_passes() < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    assert!(waited.is_ok(), "the loops did not tick");

    engine.shutdown();
    let tasks = engine.take_background_tasks();
    // Automatic Z, mail outbox, presence worker, idle lock, FDM
    // monitor, owner alerts, remote support reaper, HubRise poller,
    // HubRise outbox.
    assert_eq!(tasks.len(), 9);
    let finished = tokio::time::timeout(Duration::from_secs(2), async {
        for task in tasks {
            task.await.expect("a loop panicked");
        }
    })
    .await;
    assert!(finished.is_ok(), "a background task outlived shutdown");

    // Nothing runs after the tasks are done.
    let (z, mail) = (engine.auto_z_passes(), engine.mail_outbox_passes());
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(engine.auto_z_passes(), z);
    assert_eq!(engine.mail_outbox_passes(), mail);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_before_start_stops_every_loop_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let engine = open(dir.path(), Arc::new(Recorder::default()));
    engine.shutdown();
    engine.start_background(&tokio::runtime::Handle::current());
    let finished = tokio::time::timeout(Duration::from_secs(2), async {
        for task in engine.take_background_tasks() {
            task.await.expect("a loop panicked");
        }
    })
    .await;
    assert!(finished.is_ok(), "a background task outlived shutdown");
}

#[tokio::test]
async fn dropping_the_engine_stops_the_loops_without_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let engine = open(dir.path(), Arc::new(Recorder::default()));
    engine.start_background(&tokio::runtime::Handle::current());
    let token = engine.shutdown_token.clone();
    let tasks = engine.take_background_tasks();
    assert!(!token.is_cancelled());
    drop(engine);
    assert!(token.is_cancelled(), "Drop cancels the shutdown token");
    for task in tasks {
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("loop stops")
            .unwrap();
    }
}

/// The reaper ends an expired remote support session on its own tick,
/// without anyone asking for the status.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_support_reaper_ends_an_expired_session_on_its_own() {
    use std::os::unix::fs::PermissionsExt;

    const MANAGER: &str = "12345678958";
    const PIN: &str = "4321";
    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("AppRun");
    std::fs::write(&program, "#!/bin/sh\nsleep 30\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let engine = Engine::open(EngineConfig {
        paths: EnginePaths::under(dir.path().join("data")),
        config: ConfigSource::DevelopmentDefaults,
        events: Arc::new(Recorder::default()),
        timers: EngineTimers {
            support_reap: Duration::from_millis(10),
            ..EngineTimers::default()
        },
        identity: pos_engine::SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
        http_tls: pos_engine::HttpTls::Platform,
        device_id_override: None,
        support_program: Some(program),
    })
    .expect("open the engine");
    let state = engine.state();
    state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO users (insz, name, role, active) VALUES (?1, 'Manager', 'admin', 1)",
            [MANAGER],
        )
        .unwrap();
    *state.session.lock().unwrap() = Some(pos_core::user::User {
        insz: MANAGER.into(),
        name: "Manager".into(),
        role: pos_core::user::UserRole::Admin,
        active: true,
        has_pin: false,
        presence_only: false,
    });
    pos_engine::commands::auth::do_set_pin(state, MANAGER.into(), PIN.into(), None).unwrap();
    pos_engine::commands::support::do_start_remote_support(state, MANAGER.into(), PIN.into())
        .unwrap();
    state
        .support
        .age_session(pos_engine::commands::support::MAX_SESSION);
    assert!(state.support.holds_session(), "nobody asked yet");

    engine.start_background(&tokio::runtime::Handle::current());
    let reaped = tokio::time::timeout(Duration::from_secs(10), async {
        while state.support.holds_session() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    assert!(reaped.is_ok(), "the reaper did not end the session");
    engine.shutdown();
}
