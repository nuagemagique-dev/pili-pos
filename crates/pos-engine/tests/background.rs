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
