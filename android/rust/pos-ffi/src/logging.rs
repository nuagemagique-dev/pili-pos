//! Logging and the panic hook: the only file of `pos-ffi` with `cfg(target_os)` and with
//! process-wide state.
//!
//! `init_logging` installs one global `tracing` subscriber: a daily rolling file
//! (`pili-pos.log.<date>`, 90 files kept) and, on Android, logcat under the tag `pili-pos`.

use std::path::Path;
use std::sync::{Mutex, Once, OnceLock};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::error::PosError;

/// Keeps the file writer's worker thread alive (and flushes on exit) once logging is set up.
static GUARD: OnceLock<WorkerGuard> = OnceLock::new();
/// Serialises concurrent `init_logging` calls.
static INIT: Mutex<()> = Mutex::new(());
static PANIC_HOOK: Once = Once::new();

/// The logcat tag.
#[cfg(target_os = "android")]
const TAG: &str = "pili-pos";

#[cfg(target_os = "android")]
mod logcat {
    use std::ffi::{CString, c_char, c_int};
    use std::io::Write;

    use tracing::Level;
    use tracing_subscriber::fmt::MakeWriter;

    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
    }

    /// `android.util.Log` priorities.
    const VERBOSE: c_int = 2;
    const DEBUG: c_int = 3;
    const INFO: c_int = 4;
    const WARN: c_int = 5;
    const ERROR: c_int = 6;

    /// Write one line to logcat (interior NUL bytes are dropped).
    pub(super) fn write(prio: c_int, text: &str) {
        let tag = CString::new(super::TAG).expect("tag without NUL");
        let text = CString::new(text.replace('\0', "")).expect("text without NUL");
        // SAFETY: both pointers are valid NUL-terminated strings for the duration of the call.
        unsafe {
            __android_log_write(prio, tag.as_ptr(), text.as_ptr());
        }
    }

    pub(super) const PANIC_PRIO: c_int = ERROR;

    /// Collects one formatted event and sends it to logcat when dropped.
    pub(super) struct LineWriter {
        prio: c_int,
        buf: Vec<u8>,
    }

    impl Write for LineWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.buf.extend_from_slice(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Drop for LineWriter {
        fn drop(&mut self) {
            let text = String::from_utf8_lossy(&self.buf);
            write(self.prio, text.trim_end());
        }
    }

    pub(super) struct Logcat;

    impl<'a> MakeWriter<'a> for Logcat {
        type Writer = LineWriter;

        fn make_writer(&'a self) -> Self::Writer {
            LineWriter {
                prio: INFO,
                buf: Vec::new(),
            }
        }

        fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
            let prio = match *meta.level() {
                Level::ERROR => ERROR,
                Level::WARN => WARN,
                Level::INFO => INFO,
                Level::DEBUG => DEBUG,
                Level::TRACE => VERBOSE,
            };
            LineWriter {
                prio,
                buf: Vec::new(),
            }
        }
    }
}

/// Set up logging: `log_dir` is an absolute directory (created if missing), `filter` an
/// `EnvFilter` directive string (empty means `info`). A second call returns `Ok` and changes
/// nothing: the subscriber is process-wide.
#[uniffi::export]
pub async fn init_logging(log_dir: String, filter: String) -> Result<(), PosError> {
    let _gate = INIT.lock().unwrap_or_else(|e| e.into_inner());
    if GUARD.get().is_some() {
        return Ok(());
    }
    let dir = Path::new(&log_dir);
    if !dir.is_absolute() {
        return Err(PosError::internal(format!(
            "init_logging: log_dir must be an absolute path: {log_dir}"
        )));
    }
    std::fs::create_dir_all(dir)
        .map_err(|e| PosError::internal(format!("init_logging: creating {log_dir}: {e}")))?;
    let directives = if filter.trim().is_empty() {
        "info"
    } else {
        filter.trim()
    };
    let env_filter = EnvFilter::try_new(directives)
        .map_err(|e| PosError::internal(format!("init_logging: filter {directives:?}: {e}")))?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix("pili-pos.log")
        .max_log_files(90)
        .build(dir)
        .map_err(|e| PosError::internal(format!("init_logging: log file in {log_dir}: {e}")))?;
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let registry = tracing_subscriber::registry().with(env_filter).with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(writer),
    );
    #[cfg(target_os = "android")]
    let registry = registry.with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(logcat::Logcat),
    );
    registry
        .try_init()
        .map_err(|e| PosError::internal(format!("init_logging: a logger is already set: {e}")))?;
    let _ = GUARD.set(guard);
    Ok(())
}

/// Log panics (message and location) before the previous hook runs. Idempotent. The panic itself
/// still unwinds: UniFFI turns it into an `InternalException` on the Kotlin side.
pub(crate) fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "non-string panic payload".to_string()
            };
            let location = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown location".to_string());
            tracing::error!(%message, %location, "panic");
            #[cfg(target_os = "android")]
            logcat::write(
                logcat::PANIC_PRIO,
                &format!("panic at {location}: {message}"),
            );
            previous(info);
        }));
    });
}
