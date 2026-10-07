//! Remote support session (RustDesk), started from the POS by a manager.
//!
//! The kiosk runs no remote-access service. When the manager needs the
//! technician, they start a session from Settings: their PIN is asked
//! again, RustDesk is started (`/opt/pili-pos/rustdesk/AppRun`,
//! installed by `deploy/provision/provision.sh`), and its window shows
//! the ID and one-time password the manager reads to the technician by
//! phone. The session outlives a logout, so the technician can keep
//! working at the login screen. It ends when anyone stops it (no login
//! needed), when RustDesk exits, or after [`MAX_SESSION`]. Every screen,
//! the login screen included, shows a banner while it runs, so the
//! staff know someone is connected. The log records the role of who
//! stopped it, never a name or an INSZ.
//!
//! Only one session at a time. The program is started with a private
//! `HOME` under the service's runtime directory, so nothing it writes
//! survives a restart.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;

use pos_core::user::UserRole;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// Where the image installs RustDesk (`deploy/provision/provision.sh`).
pub const REMOTE_SUPPORT_PROGRAM: &str = "/opt/pili-pos/rustdesk/AppRun";

/// A session is ended this long after it started.
pub const MAX_SESSION: TimeDelta = TimeDelta::hours(2);

struct Session {
    child: Child,
    started_by: String,
    started_at: DateTime<Utc>,
}

impl Session {
    fn end(mut self, reason: &str, by: Option<UserRole>) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        tracing::warn!(reason, by = ?by, "remote support session stopped");
    }
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

    /// Move the running session's start back by `by` (tests).
    pub fn age_session(&self, by: TimeDelta) {
        if let Ok(mut session) = self.session.lock()
            && let Some(s) = session.as_mut()
        {
            s.started_at -= by;
        }
    }
}

impl Drop for SupportState {
    fn drop(&mut self) {
        if let Ok(session) = self.session.get_mut()
            && let Some(session) = session.take()
        {
            session.end("shutdown", None);
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
/// directory (`RuntimeDirectory=pili-pos`, wiped at stop), else the
/// system temporary directory.
fn support_home() -> PathBuf {
    std::env::var_os("RUNTIME_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("pili-pos-support")
}

/// Forget a session whose program has exited; end one past
/// [`MAX_SESSION`].
fn reap(session: &mut Option<Session>) {
    let Some(s) = session.as_mut() else { return };
    let exited = !matches!(s.child.try_wait(), Ok(None));
    let expired = Utc::now() - s.started_at >= MAX_SESSION;
    if !(exited || expired) {
        return;
    }
    if let Some(s) = session.take() {
        if exited {
            tracing::info!("remote support ended by the program");
        } else {
            s.end("time limit", None);
        }
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

/// Anyone logged in may see whether a session runs (the banner).
pub fn do_remote_support_status(state: &AppState) -> Result<SupportStatus, String> {
    access::operator(state, Access::Staff)?;
    let mut session = state.support.session.lock().map_err(errors::internal)?;
    reap(&mut session);
    Ok(status_of(&state.support, &session))
}

/// Whether a session runs, without who started it: for the login
/// screen, where nobody is logged in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportRunning {
    pub running: bool,
    /// RFC 3339, UTC.
    pub started_at: Option<String>,
}

pub fn do_remote_support_running(state: &AppState) -> Result<SupportRunning, String> {
    let mut session = state.support.session.lock().map_err(errors::internal)?;
    reap(&mut session);
    Ok(SupportRunning {
        running: session.is_some(),
        started_at: session.as_ref().map(|s| s.started_at.to_rfc3339()),
    })
}

/// Start a session: the logged-in manager, confirmed by their PIN.
pub fn do_start_remote_support(
    state: &AppState,
    insz: String,
    pin: String,
) -> Result<SupportStatus, String> {
    let operator = access::confirmed_operator(state, &insz, &pin, Access::Admin)?;
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
        role = ?operator.role,
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

/// End the session. Anyone may, logged in or not: stopping is always
/// safe. Stopping when none runs is not an error.
pub fn do_stop_remote_support(state: &AppState) -> Result<SupportStatus, String> {
    let operator = access::operator(state, Access::Staff).ok();
    let mut session = state.support.session.lock().map_err(errors::internal)?;
    if let Some(s) = session.take() {
        s.end("stopped", operator.as_ref().map(|o| o.role));
    }
    Ok(status_of(&state.support, &session))
}
