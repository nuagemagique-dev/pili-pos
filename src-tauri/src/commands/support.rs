//! Remote support session (RustDesk), started from the POS by a manager.
//!
//! The kiosk runs no remote-access service. When the manager needs the
//! technician, they start a session from Settings: their PIN is asked
//! again, RustDesk is started (`/opt/moyun-pos/rustdesk/AppRun`,
//! installed by `deploy/provision/provision.sh`), and its window shows
//! the ID and one-time password the manager reads to the technician by
//! phone. The session ends when the manager stops it (or RustDesk
//! exits). Every screen shows a banner while it runs, so the staff know
//! someone is connected. Who started and stopped it is logged.
//!
//! Only one session at a time. The program is started with a private
//! `HOME` under the service's runtime directory, so nothing it writes
//! survives a restart.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::Serialize;
use tauri::State;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// Where the image installs RustDesk (`deploy/provision/provision.sh`).
pub const REMOTE_SUPPORT_PROGRAM: &str = "/opt/moyun-pos/rustdesk/AppRun";

struct Session {
    child: Child,
    started_by: String,
    started_at: DateTime<Utc>,
}

/// The remote support program and its running session, if any.
pub struct SupportState {
    program: Mutex<Option<PathBuf>>,
    session: Mutex<Option<Session>>,
}

impl SupportState {
    /// RustDesk as installed on the kiosk, when it is there.
    pub fn installed() -> Self {
        let program = Path::new(REMOTE_SUPPORT_PROGRAM);
        Self::with_program(program.exists().then(|| program.to_path_buf()))
    }

    /// No remote support on this terminal.
    pub fn disabled() -> Self {
        Self::with_program(None)
    }

    pub fn with_program(program: Option<PathBuf>) -> Self {
        Self {
            program: Mutex::new(program),
            session: Mutex::new(None),
        }
    }

    /// Replace the program (tests).
    pub fn set_program(&self, program: Option<PathBuf>) {
        if let Ok(mut current) = self.program.lock() {
            *current = program;
        }
    }
}

impl Drop for SupportState {
    fn drop(&mut self) {
        if let Ok(session) = self.session.get_mut()
            && let Some(mut session) = session.take()
        {
            let _ = session.child.kill();
            let _ = session.child.wait();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportStatus {
    /// RustDesk is installed on this terminal.
    pub available: bool,
    pub running: bool,
    /// Name of the manager who started the running session.
    pub started_by: Option<String>,
    /// RFC 3339, UTC.
    pub started_at: Option<String>,
}

/// Directory given to RustDesk as `HOME`: under the service's runtime
/// directory (`RuntimeDirectory=moyun-pos`, wiped at stop), else the
/// system temporary directory.
fn support_home() -> PathBuf {
    std::env::var_os("RUNTIME_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("moyun-pos-support")
}

/// Forget a session whose program has exited.
fn reap(session: &mut Option<Session>) {
    let exited = session
        .as_mut()
        .is_some_and(|s| !matches!(s.child.try_wait(), Ok(None)));
    if exited && let Some(s) = session.take() {
        tracing::info!(started_by = %s.started_by, "remote support ended by the program");
    }
}

fn status_of(support: &SupportState, session: &Option<Session>) -> SupportStatus {
    let available = support.program.lock().map(|p| p.is_some()).unwrap_or(false);
    SupportStatus {
        available,
        running: session.is_some(),
        started_by: session.as_ref().map(|s| s.started_by.clone()),
        started_at: session.as_ref().map(|s| s.started_at.to_rfc3339()),
    }
}

#[tauri::command]
pub fn remote_support_status(state: State<'_, AppState>) -> Result<SupportStatus, String> {
    do_remote_support_status(state.inner())
}

/// Anyone logged in may see whether a session runs (the banner).
pub fn do_remote_support_status(state: &AppState) -> Result<SupportStatus, String> {
    access::operator(state, Access::Staff)?;
    let mut session = state.support.session.lock().map_err(errors::internal)?;
    reap(&mut session);
    Ok(status_of(&state.support, &session))
}

#[tauri::command]
pub fn start_remote_support(
    state: State<'_, AppState>,
    insz: String,
    pin: String,
) -> Result<SupportStatus, String> {
    do_start_remote_support(state.inner(), insz, pin)
}

/// Start a session: the logged-in manager, confirmed by their PIN.
pub fn do_start_remote_support(
    state: &AppState,
    insz: String,
    pin: String,
) -> Result<SupportStatus, String> {
    let operator = access::operator_named(state, &insz, Access::Admin)?;
    {
        let db = state.db.lock().map_err(errors::internal)?;
        let hash = pos_db::queries::users::get_pin_hash(&db, &insz)
            .map_err(errors::internal)?
            .ok_or_else(|| errors::PIN_SETUP_REQUIRED.to_string())?;
        if !bcrypt::verify(&pin, &hash).unwrap_or(false) {
            return Err(errors::PIN_WRONG.to_string());
        }
    }
    let program = state
        .support
        .program
        .lock()
        .map_err(errors::internal)?
        .clone()
        .ok_or_else(|| errors::SUPPORT_UNAVAILABLE.to_string())?;

    let mut session = state.support.session.lock().map_err(errors::internal)?;
    reap(&mut session);
    if session.is_some() {
        return Err(errors::SUPPORT_RUNNING.to_string());
    }
    let home = support_home();
    std::fs::create_dir_all(&home)
        .map_err(|e| errors::coded(errors::SUPPORT_START_FAILED, &[("detail", &e)]))?;
    let child = Command::new(&program)
        .env("HOME", &home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            tracing::error!(%e, program = %program.display(), "remote support did not start");
            errors::coded(errors::SUPPORT_START_FAILED, &[("detail", &e)])
        })?;
    tracing::warn!(
        operator = %operator.insz,
        name = %operator.name,
        pid = child.id(),
        "remote support session started"
    );
    *session = Some(Session {
        child,
        started_by: operator.name.clone(),
        started_at: Utc::now(),
    });
    Ok(status_of(&state.support, &session))
}

#[tauri::command]
pub fn stop_remote_support(state: State<'_, AppState>) -> Result<SupportStatus, String> {
    do_stop_remote_support(state.inner())
}

/// End the session (managers). Stopping when none runs is not an error.
pub fn do_stop_remote_support(state: &AppState) -> Result<SupportStatus, String> {
    let operator = access::operator(state, Access::Admin)?;
    let mut session = state.support.session.lock().map_err(errors::internal)?;
    if let Some(mut s) = session.take() {
        let _ = s.child.kill();
        let _ = s.child.wait();
        tracing::warn!(
            operator = %operator.insz,
            started_by = %s.started_by,
            "remote support session stopped"
        );
    }
    Ok(status_of(&state.support, &session))
}
