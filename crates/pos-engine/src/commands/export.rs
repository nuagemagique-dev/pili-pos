//! The nightly export in the engine (phase X1, decisions X1D5, X1D6, X1D10).
//!
//! After each real Z report the `pos-export` crate builds three files (an
//! encrypted backup, a statistics database, a manifest) and uploads them to
//! the endpoint of the `export` section of `pos.json`. This module is the
//! engine side of it:
//!
//! - [`ExportState`]: the settings and the HTTPS client, built once at
//!   [`crate::engine::Engine::open`]. An absent section means no export; an
//!   invalid one is logged once and shown by [`do_export_status`], and never
//!   stops the engine from opening.
//! - [`run_pass`]: one pass of the background loop (`crate::background`):
//!   prepare the exports of the Z reports newer than the cursor, apply the
//!   retention, upload what is due. It runs on the pass's own blocking
//!   threads and its own read-only database connections: it never takes the
//!   database lock of the commands, the configuration lock for longer than a
//!   clone, or the signing lock, so a slow endpoint can never delay a sale or
//!   a Z.
//! - [`wake_after_z`]: called once the printouts of a real Z are done, so the
//!   export starts at once instead of at the next tick.
//! - [`do_export_status`] and [`do_export_run_now`]: the two admin commands.
//!
//! Every failure ends up in `exports/status.json` (through `pos-export`) and
//! in the log; nothing here panics.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use pos_export::outbox::{self, ExportEnv};
use pos_export::settings::ExportSettings;
use pos_export::upload::Uploader;

use crate::commands::access::{self, Access};
use crate::state::AppState;

/// The status the operator sees (decision X1D10); the shells name this type.
pub use pos_export::status::ExportStatus;

/// Timeout of each HTTPS request of the export (decision X1D8).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// The export settings, the HTTPS client and the wake-up of the background
/// loop.
pub struct ExportState {
    /// `Ok(None)`: no `export` section. `Err`: the section is unusable, the
    /// text says why.
    settings: Result<Option<ExportSettings>, String>,
    /// Wakes the background loop for an early pass.
    wake: tokio::sync::Notify,
    /// Present exactly when the settings are `Ok(Some(_))`.
    client: Option<reqwest::Client>,
}

impl ExportState {
    /// No export on this terminal (also the state of the test harness).
    pub fn disabled() -> Self {
        Self {
            settings: Ok(None),
            wake: tokio::sync::Notify::new(),
            client: None,
        }
    }

    /// Check the `export` section of `config` and build the HTTPS client from
    /// `http_tls`. A section that cannot be used (a bad value, a client that
    /// cannot be built) is logged once and kept as the problem
    /// [`do_export_status`] reports; it is never an error of the engine.
    pub fn from_config(
        config: &pos_core::config::PosConfig,
        http_tls: &pos_fiscal::http_tls::HttpTls,
    ) -> Self {
        let Some(section) = config.export.as_ref() else {
            return Self::disabled();
        };
        let built = ExportSettings::from_config(section).and_then(|settings| {
            let client = http_tls
                .client_builder()
                .map_err(|e| format!("export: cannot build the HTTPS client: {e}"))?
                .timeout(REQUEST_TIMEOUT)
                .build()
                .map_err(|e| format!("export: cannot build the HTTPS client: {e}"))?;
            Ok((settings, client))
        });
        match built {
            Ok((settings, client)) => Self {
                settings: Ok(Some(settings)),
                wake: tokio::sync::Notify::new(),
                client: Some(client),
            },
            Err(problem) => {
                tracing::error!(%problem, "pos.json: the nightly export is off");
                Self {
                    settings: Err(problem),
                    wake: tokio::sync::Notify::new(),
                    client: None,
                }
            }
        }
    }

    /// The settings and the client when the export is configured and usable.
    fn active(&self) -> Option<(&ExportSettings, &reqwest::Client)> {
        match (&self.settings, &self.client) {
            (Ok(Some(settings)), Some(client)) => Some((settings, client)),
            _ => None,
        }
    }

    /// Whether the export is configured and usable.
    pub fn is_enabled(&self) -> bool {
        self.active().is_some()
    }

    /// Completes when [`wake_after_z`] or [`do_export_run_now`] asks for a
    /// pass. A wake-up that comes while no one waits is kept for the next
    /// wait.
    pub(crate) fn woken(&self) -> tokio::sync::futures::Notified<'_> {
        self.wake.notified()
    }
}

/// `<data_dir>/exports`: where the outbox lives.
pub(crate) fn exports_dir(state: &AppState) -> PathBuf {
    state.paths.data_dir.join("exports")
}

/// What the outbox needs for one blocking step, owned so it can move to a
/// blocking thread.
struct OwnedEnv {
    db_path: PathBuf,
    exports_dir: PathBuf,
    settings: ExportSettings,
    pos_id: String,
    terminal_id: String,
    pos_sw_version: String,
}

impl OwnedEnv {
    /// The configuration lock is held for a clone of two strings only.
    fn new(state: &AppState, settings: &ExportSettings) -> Result<Self, String> {
        let (pos_id, terminal_id) = {
            let config = state.config.lock().map_err(|e| e.to_string())?;
            (config.pos_id.clone(), config.terminal_id.clone())
        };
        Ok(Self {
            db_path: state.paths.db.clone(),
            exports_dir: exports_dir(state),
            settings: settings.clone(),
            pos_id,
            terminal_id,
            pos_sw_version: state.identity.as_str().to_string(),
        })
    }

    fn env(&self) -> ExportEnv<'_> {
        ExportEnv {
            db_path: &self.db_path,
            exports_dir: &self.exports_dir,
            settings: &self.settings,
            pos_id: &self.pos_id,
            terminal_id: &self.terminal_id,
            pos_sw_version: &self.pos_sw_version,
        }
    }
}

fn status_of(
    exports_dir: &Path,
    settings: &Result<Option<ExportSettings>, String>,
) -> ExportStatus {
    match settings {
        Ok(settings) => outbox::status(exports_dir, settings.as_ref(), None),
        Err(problem) => outbox::status(exports_dir, None, Some(problem.clone())),
    }
}

/// The status of the nightly export. Administrators only. Without an
/// `export` section: `enabled` false, no problem, and nothing is created on
/// disk. With an unusable one: `enabled` false and the problem.
pub async fn do_export_status(state: &AppState) -> Result<ExportStatus, String> {
    access::operator(state, Access::Admin)?;
    current_status(state).await
}

async fn current_status(state: &AppState) -> Result<ExportStatus, String> {
    let dir = exports_dir(state);
    let settings = state.export.settings.clone();
    tokio::task::spawn_blocking(move || status_of(&dir, &settings))
        .await
        .map_err(|e| format!("export status: {e}"))
}

/// Make every export that is waiting for a retry due at once and wake the
/// background loop; return the status at once, without waiting for the pass.
/// Administrators only. Without a usable export it only returns the status.
pub async fn do_export_run_now(state: &AppState) -> Result<ExportStatus, String> {
    access::operator(state, Access::Admin)?;
    if let Some((settings, _)) = state.export.active() {
        let owned = OwnedEnv::new(state, settings)?;
        let retried =
            tokio::task::spawn_blocking(move || outbox::retry_now(&owned.env(), Utc::now()))
                .await
                .map_err(|e| format!("export: {e}"))?;
        if let Err(e) = retried {
            tracing::warn!(%e, "export: cannot make the pending exports due");
        }
        state.export.wake.notify_one();
    }
    current_status(state).await
}

/// A real Z report is archived and printed: start the export now. Never
/// waits and never fails; a terminal without export ignores it.
pub(crate) fn wake_after_z(state: &AppState) {
    if state.export.is_enabled() {
        state.export.wake.notify_one();
    }
}

/// One pass of the export: prepare the exports of the Z reports newer than
/// the cursor, delete what is older than `keep_days`, upload what is due.
/// A no-op without a usable export. Every error is logged, and recorded in
/// `status.json` by `pos-export`.
pub async fn run_pass(state: &AppState) {
    pass(state, false).await;
}

/// The first pass after the engine started: like [`run_pass`], and the
/// exports that were waiting for a retry when the previous run ended are due
/// at once (a restart is a reason to try again, not to wait out a backoff).
pub(crate) async fn run_startup_pass(state: &AppState) {
    pass(state, true).await;
}

async fn pass(state: &AppState, startup: bool) {
    let Some((settings, client)) = state.export.active() else {
        return;
    };
    let owned = match OwnedEnv::new(state, settings) {
        Ok(owned) => std::sync::Arc::new(owned),
        Err(e) => {
            tracing::warn!(%e, "export: pass skipped");
            return;
        }
    };

    let blocking = owned.clone();
    let prepared = tokio::task::spawn_blocking(move || {
        let env = blocking.env();
        let now = Utc::now();
        if startup && let Err(e) = outbox::retry_now(&env, now) {
            tracing::warn!(%e, "export: cannot make the pending exports due");
        }
        // prepare_due logs and records its own failure.
        let _ = outbox::prepare_due(&env, now);
        if let Err(e) = outbox::retention(&env, now) {
            tracing::warn!(%e, "export: retention failed");
        }
    })
    .await;
    if let Err(e) = prepared {
        tracing::warn!(%e, "export: preparation task failed");
        return;
    }

    let uploader = Uploader::new(client.clone(), &owned.settings.url);
    outbox::upload_due(&owned.env(), &uploader, Utc::now()).await;
}
