//! The engine object: lifecycle (open, shutdown), the runtime, and the helpers every facade uses
//! to run an engine command off the caller's thread.
//!
//! Every method is async and awaits a `JoinHandle` from the engine's own runtime (not
//! `async_runtime = "tokio"`): when the Kotlin coroutine is cancelled the `UniFFI` future is dropped,
//! the handle is detached and the task, a sign in flight for instance, still finishes.

use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use pos_engine::{
    ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers, HttpTls, SoftwareIdentity,
};
use tokio::runtime::Runtime;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::error::{self, PosError};
use crate::events::{EventStream, FfiSink, PosEvent};

/// The `posSwVersion` of every event and ticket of this app (36 characters at most).
const SW_VERSION: &str = concat!("pili-pos-android/", env!("CARGO_PKG_VERSION"));
/// Ring size of the event channel; a slower subscriber gets `Resync`.
const EVENT_CAPACITY: usize = 256;
/// How long `shutdown` waits for the background tasks and for in-flight calls.
const SHUTDOWN_BOUND: Duration = Duration::from_secs(30);
/// DLE EOT answers often arrive after 500 ms on the Android network path (spike S4, PD37).
const PRINTER_STATUS_READ: Duration = Duration::from_secs(2);

/// Intervals of the engine's background loops, in milliseconds. All must be positive.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct EngineTimersMs {
    pub auto_z_ms: i64,
    pub mail_outbox_ms: i64,
    pub printer_status_read_ms: i64,
    pub idle_lock_ms: i64,
    pub fdm_poll_ms: i64,
}

/// What the app gives [`PosEngine::open`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct EngineOptions {
    /// Absolute path of the app's files directory: database, backups, `engine.lock`.
    pub data_dir: String,
    /// Absolute path of `pos.json` in the provisioning bundle.
    pub config_bundle: String,
    /// The handset identifier (replaces `device_id` of `pos.json`).
    pub device_id: Option<String>,
    /// `None`: the engine's defaults, with a 2 s printer status read.
    pub timers: Option<EngineTimersMs>,
}

/// The `posSwVersion` this build signs with, for example `pili-pos-android/1.0.0`.
#[cfg(not(feature = "abi-mismatch"))]
#[uniffi::export]
pub async fn pos_sw_version() -> String {
    SW_VERSION.to_string()
}

/// Same exported name, different signature: bindings generated from the normal build no longer
/// match. The Rust function has another name so that `pos_sw_version()` (the normal signature)
/// still exists for the integration tests, which build with this feature too.
#[cfg(feature = "abi-mismatch")]
#[uniffi::export(name = "pos_sw_version")]
pub async fn pos_sw_version_mismatch(extra: i64) -> String {
    format!("{SW_VERSION}+{extra}")
}

/// The normal signature, not exported, so the tests build with `--features abi-mismatch`.
#[cfg(feature = "abi-mismatch")]
pub async fn pos_sw_version() -> String {
    SW_VERSION.to_string()
}

/// What the drain of a shutdown ends with: `Ok` once the engine is dropped, or the engine itself
/// when the bound ran out.
type Drained = Result<(), Arc<Engine>>;

/// The engine and its runtime.
#[derive(uniffi::Object)]
pub struct PosEngine {
    // Option so Drop can use shutdown_background(): dropping a Runtime from inside an async
    // context (the last Arc dropped on a runtime thread) would panic.
    rt: Option<Runtime>,
    engine: Mutex<Option<Arc<Engine>>>,
    closing: AtomicBool,
    events_tx: Mutex<Option<broadcast::Sender<PosEvent>>>,
    /// One shutdown at a time. Holds the drain a shutdown started and did not see through (its
    /// caller was cancelled while waiting), so the next shutdown waits for that same drain.
    shutdown_gate: tokio::sync::Mutex<Option<JoinHandle<Drained>>>,
}

/// Cancel the engine's tasks and wait for them and for the calls in flight, at most
/// [`SHUTDOWN_BOUND`]; then drop the engine.
async fn drain(engine: Arc<Engine>) -> Drained {
    engine.shutdown();
    let tasks = engine.take_background_tasks();
    let drained = tokio::time::timeout(SHUTDOWN_BOUND, async {
        for task in tasks {
            let _ = task.await;
        }
        // Polling is deliberate: the calls in flight each hold a clone of the Arc and there is no
        // event to await when the last one drops. The loop is bounded by SHUTDOWN_BOUND, runs only
        // at shutdown and costs one counter read every 20 ms.
        while Arc::strong_count(&engine) > 1 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    match drained {
        Ok(()) => {
            drop(engine);
            Ok(())
        }
        Err(_) => Err(engine),
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn positive_ms(name: &str, value: i64) -> Result<Duration, PosError> {
    u64::try_from(value)
        .ok()
        .filter(|ms| *ms > 0)
        .map(Duration::from_millis)
        .ok_or_else(|| PosError::internal(format!("open: timers.{name} must be > 0, got {value}")))
}

fn timers_from(options: Option<&EngineTimersMs>) -> Result<EngineTimers, PosError> {
    let Some(t) = options else {
        return Ok(EngineTimers {
            printer_status_read: PRINTER_STATUS_READ,
            ..EngineTimers::default()
        });
    };
    Ok(EngineTimers {
        auto_z: positive_ms("auto_z_ms", t.auto_z_ms)?,
        mail_outbox: positive_ms("mail_outbox_ms", t.mail_outbox_ms)?,
        printer_status_read: positive_ms("printer_status_read_ms", t.printer_status_read_ms)?,
        idle_lock: positive_ms("idle_lock_ms", t.idle_lock_ms)?,
        fdm_poll: positive_ms("fdm_poll_ms", t.fdm_poll_ms)?,
        export_outbox: EngineTimers::default().export_outbox,
        badge_repeat: EngineTimers::default().badge_repeat,
        peppol_outbox: EngineTimers::default().peppol_outbox,
        terminal_probe: EngineTimers::default().terminal_probe,
        support_reap: EngineTimers::default().support_reap,
    })
}

fn absolute(name: &str, value: &str) -> Result<std::path::PathBuf, PosError> {
    let path = Path::new(value);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Err(PosError::internal(format!(
            "open: {name} must be an absolute path: {value}"
        )))
    }
}

/// A join error: a panic is re-raised (`UniFFI` reports it as `InternalException`), a cancelled
/// task means the runtime is going away.
fn joined<T>(result: Result<T, tokio::task::JoinError>) -> Result<T, PosError> {
    match result {
        Ok(v) => Ok(v),
        Err(e) if e.is_panic() => std::panic::resume_unwind(e.into_panic()),
        Err(_) => Err(PosError::EngineClosed),
    }
}

#[uniffi::export]
impl PosEngine {
    /// Open the engine on the app's data directory and start its background loops. Synchronous:
    /// Kotlin calls it on `Dispatchers.IO`.
    #[uniffi::constructor]
    pub fn open(options: EngineOptions) -> Result<Arc<Self>, PosError> {
        crate::logging::install_panic_hook();
        let data_dir = absolute("data_dir", &options.data_dir)?;
        let bundle = absolute("config_bundle", &options.config_bundle)?;
        let timers = timers_from(options.timers.as_ref())?;
        let identity = SoftwareIdentity::new(SW_VERSION)
            .map_err(|e| PosError::internal(format!("open: identity: {e}")))?;

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("pos-engine")
            .enable_all()
            .build()
            .map_err(|e| PosError::internal(format!("open: runtime: {e}")))?;
        let (tx, _rx) = broadcast::channel(EVENT_CAPACITY);
        let config = EngineConfig {
            paths: EnginePaths::under(data_dir),
            config: ConfigSource::Bundle(bundle),
            events: Arc::new(FfiSink { tx: tx.clone() }),
            timers,
            identity,
            http_tls: HttpTls::RootsOnly(
                webpki_root_certs::TLS_SERVER_ROOT_CERTS
                    .iter()
                    .map(|c| c.to_vec())
                    .collect(),
            ),
            device_id_override: options.device_id,
            support_program: None,
        };
        let engine = match Engine::open(config) {
            Ok(engine) => Arc::new(engine),
            Err(e) => {
                rt.shutdown_background();
                return Err(PosError::internal(format!("open: {e:#}")));
            }
        };
        engine.start_background(rt.handle());
        Ok(Arc::new(Self {
            rt: Some(rt),
            engine: Mutex::new(Some(engine)),
            closing: AtomicBool::new(false),
            events_tx: Mutex::new(Some(tx)),
            shutdown_gate: tokio::sync::Mutex::new(None),
        }))
    }

    /// Stop the engine: cancel the background loops, wait for them and for the calls in flight
    /// (30 s at most), then close the database and release `engine.lock`, so the same data
    /// directory can be opened again, and end every `EventStream`. A second call is `Ok`. On
    /// timeout the engine stays (closed to new calls) and the error is `Internal`.
    ///
    /// The drain runs on the engine's runtime, so a `shutdown` whose caller is cancelled still
    /// completes in the background, and the next `shutdown` waits for that same drain: `Ok`
    /// always means the engine is dropped, `engine.lock` is free and the event streams have ended.
    pub async fn shutdown(&self) -> Result<(), PosError> {
        let mut pending = self.shutdown_gate.lock().await;
        self.closing.store(true, Ordering::SeqCst);
        if pending.is_none() {
            let Some(engine) = lock(&self.engine).take() else {
                // Shut down before: the engine is gone, so the streams must be over too.
                lock(&self.events_tx).take();
                return Ok(());
            };
            // The wait sleeps, so it runs on the engine's runtime, not on the foreign executor.
            *pending = Some(self.rt().spawn(drain(engine)));
        }
        // The handle stays in the gate while it is awaited: if this future is dropped here, the
        // next shutdown polls the same handle.
        let outcome = match pending.as_mut() {
            Some(handle) => handle.await,
            None => return Ok(()),
        };
        *pending = None;
        if outcome.is_err() {
            // The drain panicked or was cancelled with the runtime: the engine is lost.
            lock(&self.events_tx).take();
        }
        match joined(outcome) {
            Ok(Ok(())) => {
                lock(&self.events_tx).take();
                Ok(())
            }
            Ok(Err(engine)) => {
                tracing::error!("shutdown: timed out waiting for the engine's tasks");
                *lock(&self.engine) = Some(engine);
                Err(PosError::internal("shutdown: timed out"))
            }
            Err(e) => Err(e),
        }
    }

    /// A new subscription to the engine's events; `EngineClosed` once shut down.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "UniFFI exports it as a Kotlin suspend fun: the signature is the contract"
    )]
    pub async fn events(&self) -> Result<Arc<EventStream>, PosError> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(PosError::EngineClosed);
        }
        let tx = lock(&self.events_tx)
            .as_ref()
            .map(tokio::sync::broadcast::Sender::subscribe)
            .ok_or(PosError::EngineClosed)?;
        Ok(EventStream::new(tx))
    }
}

impl PosEngine {
    fn rt(&self) -> &Runtime {
        self.rt.as_ref().expect("runtime present until drop")
    }

    /// The open engine, or `EngineClosed`.
    fn engine(&self) -> Result<Arc<Engine>, PosError> {
        if self.closing.load(Ordering::SeqCst) {
            return Err(PosError::EngineClosed);
        }
        lock(&self.engine).clone().ok_or(PosError::EngineClosed)
    }

    /// Run a synchronous engine command (rusqlite, bcrypt) on the blocking pool. The engine
    /// command's `String` error is mapped by [`error::from_engine`].
    pub(crate) async fn blocking<T, F>(&self, f: F) -> Result<T, PosError>
    where
        F: FnOnce(&pos_engine::state::AppState) -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        let engine = self.engine()?;
        let handle = self.rt().spawn_blocking(move || f(&engine));
        joined(handle.await)?.map_err(error::from_engine)
    }

    /// Run an async engine command on the engine's runtime.
    pub(crate) async fn spawn<T, F, Fut>(&self, f: F) -> Result<T, PosError>
    where
        F: FnOnce(Arc<Engine>) -> Fut,
        Fut: Future<Output = Result<T, String>> + Send + 'static,
        T: Send + 'static,
    {
        let engine = self.engine()?;
        let handle = self.rt().spawn(f(engine));
        joined(handle.await)?.map_err(error::from_engine)
    }

    #[cfg(test)]
    pub(crate) fn events_tx_for_tests(&self) -> Option<broadcast::Sender<PosEvent>> {
        lock(&self.events_tx).clone()
    }
}

impl std::fmt::Debug for PosEngine {
    /// The engine, its runtime and the channels hold no state worth printing.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PosEngine")
            .field("closing", &self.closing.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl Drop for PosEngine {
    fn drop(&mut self) {
        if let Some(engine) = lock(&self.engine).take() {
            tracing::error!("PosEngine dropped without shutdown(): cancelling the engine");
            engine.shutdown();
        }
        if let Some(rt) = self.rt.take() {
            rt.shutdown_background();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use crate::events::publish_for_tests;

    const POS_JSON: &str = r#"{
        "pos_id": "CPOS0031234567", "terminal_id": "POS-2-ANDROID", "device_id": "unit-test",
        "restaurant_name": "R", "restaurant_address": "A", "restaurant_vat": "BE0499999960",
        "restaurant_est_no": "2123456791", "fdm_url": "http://127.0.0.1:9/graphql"
    }"#;

    /// Open an engine on a fresh data directory with a bundle next to it.
    fn open(dir: &Path) -> Arc<PosEngine> {
        let bundle = dir.join("bundle");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::write(bundle.join("pos.json"), POS_JSON).unwrap();
        PosEngine::open(EngineOptions {
            data_dir: dir.join("data").to_string_lossy().into_owned(),
            config_bundle: bundle.join("pos.json").to_string_lossy().into_owned(),
            device_id: Some("UNIT-0001".into()),
            timers: None,
        })
        .unwrap_or_else(|e| panic!("open: {e}"))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn blocking_returns_the_value_and_maps_a_coded_error() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        assert_eq!(engine.blocking(|_| Ok::<_, String>(5)).await.unwrap(), 5);
        let err = engine
            .blocking(|_| Err::<(), _>("order_no_lines".to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, PosError::Coded { code, params } if code == "order_no_lines" && params.is_empty()),
            "{err:?}"
        );
        let err = engine
            .spawn(|_| async { Err::<(), _>("pin_locked|seconds=30".to_string()) })
            .await
            .unwrap_err();
        assert!(
            matches!(&err, PosError::Coded { code, params } if code == "pin_locked" && params["seconds"] == "30"),
            "{err:?}"
        );
        engine.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_panic_in_a_blocking_closure_surfaces_as_a_panic_of_the_awaited_future() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        let caller = Arc::clone(&engine);
        let join = tokio::spawn(async move {
            caller
                .blocking(|_| -> Result<(), String> { panic!("deliberate blocking panic") })
                .await
        });
        let err = join.await.unwrap_err();
        assert!(err.is_panic());
        engine.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_panic_in_a_spawned_future_surfaces_as_a_panic_of_the_awaited_future() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        let caller = Arc::clone(&engine);
        let join = tokio::spawn(async move {
            caller
                .spawn(|_| async {
                    assert!(!std::hint::black_box(true), "deliberate spawned panic");
                    Ok::<(), String>(())
                })
                .await
        });
        let err = join.await.unwrap_err();
        assert!(err.is_panic());
        // The engine is still usable after a panic in one call.
        assert_eq!(engine.blocking(|_| Ok::<_, String>(1)).await.unwrap(), 1);
        engine.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn helpers_answer_engine_closed_after_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        engine.shutdown().await.unwrap();
        assert!(matches!(
            engine.blocking(|_| Ok::<_, String>(())).await,
            Err(PosError::EngineClosed)
        ));
        assert!(matches!(
            engine.spawn(|_| async { Ok::<_, String>(()) }).await,
            Err(PosError::EngineClosed)
        ));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shutdown_waits_for_a_call_in_flight() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        let caller = Arc::clone(&engine);
        let started = Instant::now();
        let call = tokio::spawn(async move {
            caller
                .blocking(|_| {
                    std::thread::sleep(Duration::from_millis(400));
                    Ok::<_, String>(7)
                })
                .await
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        engine.shutdown().await.unwrap();
        assert!(
            started.elapsed() >= Duration::from_millis(380),
            "shutdown returned before the call finished: {:?}",
            started.elapsed()
        );
        assert_eq!(call.await.unwrap().unwrap(), 7);
        // The lock is released: the same directory opens again.
        let again = open(dir.path());
        again.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_event_stream_delivers_resync_and_ends_with_the_engine() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        let stream = engine.events().await.unwrap();
        publish_for_tests(&engine, PosEvent::Resync { missed: 7 });
        assert!(matches!(
            stream.next().await,
            Some(PosEvent::Resync { missed: 7 })
        ));
        engine.shutdown().await.unwrap();
        assert!(stream.next().await.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_slow_subscriber_gets_a_resync_with_the_missed_count() {
        let dir = tempfile::tempdir().unwrap();
        let engine = open(dir.path());
        let stream = engine.events().await.unwrap();
        for _ in 0..(EVENT_CAPACITY + 44) {
            publish_for_tests(&engine, PosEvent::Resync { missed: 0 });
        }
        assert!(matches!(
            stream.next().await,
            Some(PosEvent::Resync { missed: 44 })
        ));
        engine.shutdown().await.unwrap();
    }

    #[test]
    fn timers_are_validated() {
        let ok = EngineTimersMs {
            auto_z_ms: 1,
            mail_outbox_ms: 2,
            printer_status_read_ms: 3,
            idle_lock_ms: 4,
            fdm_poll_ms: 5,
        };
        let t = timers_from(Some(&ok)).unwrap();
        assert_eq!(t.fdm_poll, Duration::from_millis(5));
        assert_eq!(t.printer_status_read, Duration::from_millis(3));
        let default = timers_from(None).unwrap();
        assert_eq!(default.printer_status_read, Duration::from_secs(2));
        assert_eq!(default.idle_lock, EngineTimers::default().idle_lock);
        for bad in [0, -5] {
            let t = EngineTimersMs {
                idle_lock_ms: bad,
                ..ok.clone()
            };
            let err = timers_from(Some(&t)).unwrap_err();
            assert!(
                matches!(&err, PosError::Internal { detail } if detail.contains("idle_lock_ms")),
                "{err:?}"
            );
        }
    }

    #[test]
    fn the_software_identity_fits_the_fdm_limit() {
        assert_eq!(SW_VERSION, "pili-pos-android/1.0.0");
        assert!(SoftwareIdentity::new(SW_VERSION).is_ok());
    }
}
