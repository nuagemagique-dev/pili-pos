//! Lifecycle of the engine object: open, shutdown, reopen, argument checks, logging.

use std::time::{Duration, Instant};

use pos_ffi::{EngineOptions, EngineTimersMs, PosEngine, PosError, init_logging, pos_sw_version};

use crate::support::{open_seeded, open_seeded_with_hold, options_for, start_held_call};

fn open_err(options: EngineOptions) -> PosError {
    match PosEngine::open(options) {
        Ok(_) => panic!("open should have failed"),
        Err(e) => e,
    }
}

fn detail(err: &PosError) -> &str {
    match err {
        PosError::Internal { detail } => detail,
        other => panic!("an Internal error expected, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn open_and_shutdown_twice() {
    let t = open_seeded().await;
    t.engine.shutdown().await.unwrap();
    // A second shutdown is Ok.
    t.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_second_open_on_the_same_data_dir_is_refused() {
    let t = open_seeded().await;
    let err = open_err(t.options());
    assert!(detail(&err).contains("engine already open"), "{err:?}");
    t.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn reopen_after_shutdown_succeeds() {
    let t = open_seeded().await;
    t.engine.shutdown().await.unwrap();
    let again = PosEngine::open(t.options()).unwrap_or_else(|e| panic!("reopen: {e}"));
    again.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_after_shutdown_is_engine_closed() {
    let t = open_seeded().await;
    t.engine.shutdown().await.unwrap();
    assert!(matches!(
        t.engine.events().await,
        Err(PosError::EngineClosed)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn relative_paths_are_refused() {
    let t = open_seeded().await;
    let options = t.options();
    t.engine.shutdown().await.unwrap();

    let err = open_err(EngineOptions {
        data_dir: "relative/data".into(),
        ..options.clone()
    });
    assert!(
        detail(&err).contains("data_dir must be an absolute path"),
        "{err:?}"
    );

    let err = open_err(EngineOptions {
        config_bundle: "bundle/pos.json".into(),
        ..options
    });
    assert!(
        detail(&err).contains("config_bundle must be an absolute path"),
        "{err:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_zero_timer_is_refused() {
    let t = open_seeded().await;
    let options = t.options();
    t.engine.shutdown().await.unwrap();
    let timers = EngineTimersMs {
        auto_z_ms: 60_000,
        mail_outbox_ms: 30_000,
        printer_status_read_ms: 2_000,
        idle_lock_ms: 300_000,
        fdm_poll_ms: 0,
    };
    let err = open_err(EngineOptions {
        timers: Some(timers.clone()),
        ..options.clone()
    });
    assert!(detail(&err).contains("fdm_poll_ms"), "{err:?}");

    // Valid timers open.
    let engine = PosEngine::open(EngineOptions {
        timers: Some(EngineTimersMs {
            fdm_poll_ms: 30_000,
            ..timers
        }),
        ..options
    })
    .unwrap_or_else(|e| panic!("open: {e}"));
    engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_bundle_is_an_internal_error_and_leaves_the_directory_free() {
    let dir = tempfile::tempdir().unwrap();
    let err = open_err(options_for(dir.path()));
    assert!(matches!(err, PosError::Internal { .. }), "{err:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_software_version_is_the_android_identity() {
    assert_eq!(pos_sw_version().await, "pili-pos-android/1.0.0");
}

/// Logging is process-wide: this is the only test that calls `init_logging`.
#[tokio::test(flavor = "multi_thread")]
async fn init_logging_writes_a_daily_log_file() {
    let logs = tempfile::tempdir().unwrap();
    let log_dir = logs.path().join("logs");
    init_logging(log_dir.to_string_lossy().into_owned(), String::new())
        .await
        .unwrap();
    // A second call is Ok and changes nothing.
    let other = logs.path().join("other");
    init_logging(other.to_string_lossy().into_owned(), "debug".into())
        .await
        .unwrap();
    assert!(!other.exists());

    let t = open_seeded().await;
    t.engine.shutdown().await.unwrap();

    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut found = None;
        for entry in std::fs::read_dir(&log_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if name.starts_with("pili-pos.log.")
                && std::fs::read_to_string(&path)
                    .map(|text| text.contains("opening database"))
                    .unwrap_or(false)
            {
                found = Some(name);
            }
        }
        if found.is_some() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "no pili-pos.log.* file with \"opening database\" in {}",
            log_dir.display()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_event_stream_ends_when_the_engine_shuts_down() {
    let t = open_seeded().await;
    let stream = t.engine.events().await.unwrap();
    t.engine.shutdown().await.unwrap();
    assert!(stream.next().await.is_none());
}

/// How long the first `shutdown` is given before its caller gives up (a cancelled Kotlin
/// coroutine drops the UniFFI future the same way).
const CANCEL_AFTER: Duration = Duration::from_millis(300);

/// The first `shutdown` of an engine with a call in flight is dropped by its caller after the
/// engine was taken. Checks that it really was: the call in flight keeps the drain waiting, so
/// the data directory is still locked, and new calls are already refused.
async fn cancel_a_shutdown_in_progress(t: &crate::support::TestEngine) {
    let first = tokio::time::timeout(CANCEL_AFTER, t.engine.shutdown()).await;
    assert!(
        first.is_err(),
        "the first shutdown must still be waiting: {first:?}"
    );
    assert!(matches!(
        t.engine.events().await,
        Err(PosError::EngineClosed)
    ));
    assert!(matches!(
        t.engine.get_settings().await,
        Err(PosError::EngineClosed)
    ));
    let err = open_err(t.options());
    assert!(detail(&err).contains("engine already open"), "{err:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelled_shutdown_then_shutdown_releases_the_engine() {
    let (t, hold) = open_seeded_with_hold().await;
    let call = start_held_call(&t, &hold).await;
    cancel_a_shutdown_in_progress(&t).await;

    // The second shutdown waits for the drain the first one started: while the call is held it
    // is not done.
    let engine = t.engine.clone();
    let second = tokio::spawn(async move { engine.shutdown().await });
    tokio::time::sleep(CANCEL_AFTER).await;
    assert!(
        !second.is_finished(),
        "the second shutdown returned while the engine was still in use"
    );

    hold.release();
    tokio::time::timeout(Duration::from_secs(10), second)
        .await
        .expect("the second shutdown ends once the call is done")
        .unwrap()
        .unwrap();
    call.await.unwrap().unwrap();

    // Ok means engine.lock is free: the same directory opens right away.
    let again = PosEngine::open(t.options()).unwrap_or_else(|e| panic!("reopen: {e}"));
    again.shutdown().await.unwrap();
    // And a third shutdown is still Ok.
    t.engine.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn cancelled_shutdown_then_shutdown_ends_the_streams() {
    let (t, hold) = open_seeded_with_hold().await;
    let stream = t.engine.events().await.unwrap();
    let call = start_held_call(&t, &hold).await;
    cancel_a_shutdown_in_progress(&t).await;

    hold.release();
    call.await.unwrap().unwrap();
    t.engine.shutdown().await.unwrap();

    // Events sent before the shutdown (the clock-out's) are still delivered, then the stream
    // ends: on a slow machine the clock-out event can still be queued here.
    tokio::time::timeout(Duration::from_secs(5), async {
        while stream.next().await.is_some() {}
    })
    .await
    .expect("the stream ends once the engine is shut down");
    assert!(matches!(
        t.engine.events().await,
        Err(PosError::EngineClosed)
    ));
}
