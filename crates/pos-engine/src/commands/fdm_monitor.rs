//! The FDM status monitor (planner decision PD36).
//!
//! The Vue store `ui/src/stores/fdm.ts` used to poll the FDM and
//! classify its answer. Both belong to the engine now:
//!
//! - [`classify`] turns the status JSON of
//!   [`crate::commands::checkout::do_check_fdm_status`] into an
//!   [`FdmHealth`], with the rules of the store's `checkStatus` and
//!   `pickPrimaryMessage` (see `docs/android/ENGINE.md` for the table).
//! - The monitor loop of `background.rs` checks while a session is open:
//!   at once when it opens, then every `EngineTimers::fdm_poll` while the
//!   FDM answers, and at `base * 2^n` up to `base * 10` while it does not
//!   ([`poll_delay`]). Each check is stored, so [`do_fdm_health`] can
//!   answer from memory, and emitted as `fdm-status`.
//! - [`do_fdm_check_now`] is the explicit check (the login screen, the
//!   retry button). It works without a session.
//!
//! A message with showPos MANDATORY must not auto-dismiss (certification
//! manual section V): that is why `mandatory` is part of the result.

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Notify;

use crate::commands::checkout;
use crate::events::EngineEvent;
use crate::state::AppState;

/// The code of a transport failure (`FDM_NOT_OPERATIONAL`), also the
/// code of an FDM that answers with an error of that name.
const NOT_OPERATIONAL: &str = "FDM_NOT_OPERATIONAL";
/// The reason named for an FDM that says it is not initialized with
/// nothing more to say.
const NOT_INITIALIZED: &str = "FDM_NOT_INITIALIZED";

/// The error codes that stop checkout (the store's `BLOCKING_CODES`).
const BLOCKING_CODES: [&str; 4] = ["BUFFER_FULL", "FDM_LOCKED", NOT_OPERATIONAL, "UNKNOWN_POS"];

/// What the UI does about a code: `blocking` is the red pill with the
/// checkout-blocking modal, `warning` the yellow pill, `info` no change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FdmSeverity {
    Blocking,
    Warning,
    Info,
}

/// The state of the FDM as the UI shows it.
///
/// `answered` is whether the FDM answered at all (an answer with an error
/// still counts); it drives the backoff. `connected` is the transport
/// working and no blocking condition. Times are RFC 3339 strings.
/// [`classify`] stamps `checked_at` with the current time and leaves
/// `next_check_at` `None`: the caller that knows whether a session is
/// open sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FdmHealth {
    pub connected: bool,
    pub answered: bool,
    pub severity: FdmSeverity,
    /// The code to show, or the raw transport error text.
    pub code: Option<String>,
    pub mandatory: bool,
    pub checked_at: String,
    /// When the monitor checks next; `None` without a session.
    pub next_check_at: Option<String>,
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn rfc3339_in(delay: Duration) -> String {
    let delay = chrono::Duration::from_std(delay).unwrap_or(chrono::Duration::MAX);
    chrono::Utc::now()
        .checked_add_signed(delay)
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// JavaScript truthiness of a JSON value (`status.error` in the store).
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// The health of an FDM that did not answer: a transport failure.
fn unanswered(code: String) -> FdmHealth {
    FdmHealth {
        connected: false,
        answered: false,
        severity: FdmSeverity::Blocking,
        code: Some(code),
        mandatory: true,
        checked_at: now_rfc3339(),
        next_check_at: None,
    }
}

/// `extensions.showPos` of a message item, when it is a string.
fn show_pos(item: &Value) -> Option<&str> {
    item.get("extensions")?.get("showPos")?.as_str()
}

/// `extensions.code` of a message item; like JavaScript truthiness, an
/// empty code counts as none.
fn code_of(item: &Value) -> Option<&str> {
    item.get("extensions")?
        .get("code")?
        .as_str()
        .filter(|c| !c.is_empty())
}

/// The first item for which `pick` holds, like JavaScript's `find`, and
/// its code. The item found is not skipped when it has no code: the
/// search does not go on to the next one.
fn find_code(items: &[Value], pick: impl Fn(&Value) -> bool) -> Option<&str> {
    items.iter().find(|i| pick(i)).and_then(code_of)
}

fn items<'a>(status: &'a Value, key: &str) -> &'a [Value] {
    status
        .get(key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The store's `pickPrimaryMessage`: errors before warnings, within each
/// the first MANDATORY item, else the first one that is not NEVER.
fn pick_primary(status: &Value) -> (FdmSeverity, Option<String>, bool) {
    let by_code = |code: &str| {
        if BLOCKING_CODES.contains(&code) {
            FdmSeverity::Blocking
        } else {
            FdmSeverity::Warning
        }
    };
    let mandatory = |i: &Value| show_pos(i) == Some("MANDATORY");
    let shown = |i: &Value| show_pos(i) != Some("NEVER");

    let errors = items(status, "errors");
    if let Some(code) = find_code(errors, mandatory) {
        return (by_code(code), Some(code.to_string()), true);
    }
    if let Some(code) = find_code(errors, shown) {
        return (by_code(code), Some(code.to_string()), false);
    }
    let warnings = items(status, "warnings");
    if let Some(code) = find_code(warnings, mandatory) {
        return (FdmSeverity::Warning, Some(code.to_string()), true);
    }
    if let Some(code) = find_code(warnings, shown) {
        return (FdmSeverity::Warning, Some(code.to_string()), false);
    }
    (FdmSeverity::Info, None, false)
}

/// Classify the answer of `do_check_fdm_status` (the store's
/// `checkStatus`). `connected: false` or a truthy `error` is a transport
/// failure: blocking and mandatory, the code is the error text (or
/// `FDM_NOT_OPERATIONAL` without one), and the FDM did not answer. A
/// value that is not an object is treated the same way. Otherwise the FDM
/// answered: see [`pick_primary`]; an `initialized: false` FDM with no
/// message is named `FDM_NOT_INITIALIZED`, and is not connected.
pub fn classify(status: &Value) -> FdmHealth {
    let Some(object) = status.as_object() else {
        return unanswered(NOT_OPERATIONAL.to_string());
    };
    let failed = object.get("connected") == Some(&Value::Bool(false))
        || object.get("error").is_some_and(truthy);
    if failed {
        let code = match object.get("error") {
            Some(Value::String(s)) if !s.is_empty() => s.clone(),
            Some(v) if truthy(v) => v.to_string(),
            _ => NOT_OPERATIONAL.to_string(),
        };
        return unanswered(code);
    }
    let (severity, code, mandatory) = pick_primary(status);
    let not_initialized = object.get("initialized") == Some(&Value::Bool(false));
    let code = code.or_else(|| not_initialized.then(|| NOT_INITIALIZED.to_string()));
    FdmHealth {
        connected: !not_initialized && severity != FdmSeverity::Blocking,
        answered: true,
        severity,
        code,
        mandatory,
        checked_at: now_rfc3339(),
        next_check_at: None,
    }
}

/// The wait before the next check: `base` while the FDM answers, then
/// `base * 2^failures` up to `base * 10` (30 s, 60, 120, 240, 300, 300 for
/// the default base). `failures` is the number of consecutive checks the
/// FDM did not answer.
pub fn poll_delay(base: Duration, failures: u32) -> Duration {
    if failures == 0 {
        return base;
    }
    let factor = 1u32.checked_shl(failures).map_or(10, |f| f.min(10));
    base.saturating_mul(factor)
}

#[derive(Debug, Default)]
struct Last {
    health: Option<FdmHealth>,
    failures: u32,
}

/// What the monitor remembers: the last health, how many checks in a
/// row the FDM did not answer, and the wake-up of the loop.
#[derive(Debug, Default)]
pub struct FdmMonitorState {
    last: Mutex<Last>,
    /// Notified by an explicit check, so the loop schedules the next one
    /// from now.
    wake: Notify,
}

impl FdmMonitorState {
    /// Consecutive checks the FDM did not answer (0 after an answer).
    pub(crate) fn failures(&self) -> u32 {
        self.last.lock().map_or(0, |l| l.failures)
    }

    /// A session opens: the backoff starts again (the store did the
    /// same when polling started).
    pub(crate) fn reset_failures(&self) {
        if let Ok(mut last) = self.last.lock() {
            last.failures = 0;
        }
    }

    /// The session closed: the stored health no longer promises a check.
    pub(crate) fn clear_next_check(&self) {
        if let Ok(mut last) = self.last.lock()
            && let Some(health) = last.health.as_mut()
        {
            health.next_check_at = None;
        }
    }

    /// Completes at the next explicit check.
    pub(crate) fn woken(&self) -> tokio::sync::futures::Notified<'_> {
        self.wake.notified()
    }
}

/// The last known health, `None` before the first check.
pub fn do_fdm_health(state: &AppState) -> Result<Option<FdmHealth>, String> {
    let last = state.fdm_monitor.last.lock().map_err(|e| e.to_string())?;
    Ok(last.health.clone())
}

/// Check the FDM now and tell everyone: the result is stored, emitted as
/// `fdm-status`, and the monitor loop schedules its next check from now.
/// Needs no session (the login screen uses it).
pub async fn do_fdm_check_now(state: &AppState) -> Result<FdmHealth, String> {
    check(state, Wake::Loop).await
}

/// Whether a check wakes the monitor loop to reschedule from now.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wake {
    Loop,
    No,
}

/// One check, shared by [`do_fdm_check_now`] and the loop (which passes
/// `Wake::No`: it reschedules by itself).
pub(crate) async fn check(state: &AppState, wake: Wake) -> Result<FdmHealth, String> {
    let mut health = match checkout::do_check_fdm_status(state).await {
        Ok(status) => classify(&status),
        Err(e) => {
            tracing::warn!(%e, "FDM status check failed");
            unanswered(NOT_OPERATIONAL.to_string())
        }
    };
    let session_open = state.session.lock().is_ok_and(|s| s.is_some());
    {
        let mut last = state.fdm_monitor.last.lock().map_err(|e| e.to_string())?;
        last.failures = if health.answered {
            0
        } else {
            last.failures.saturating_add(1)
        };
        health.next_check_at =
            session_open.then(|| rfc3339_in(poll_delay(state.timers.fdm_poll, last.failures)));
        last.health = Some(health.clone());
    }
    state.events.emit(EngineEvent::FdmStatus(health.clone()));
    if wake == Wake::Loop {
        state.fdm_monitor.wake.notify_one();
    }
    Ok(health)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(code: Option<&str>, show_pos: Option<&str>) -> Value {
        let mut extensions = serde_json::Map::new();
        if let Some(code) = code {
            extensions.insert("code".into(), json!(code));
        }
        if let Some(show_pos) = show_pos {
            extensions.insert("showPos".into(), json!(show_pos));
        }
        json!({ "message": "m", "extensions": extensions })
    }

    fn status(errors: Vec<Value>, warnings: Vec<Value>) -> Value {
        json!({ "initialized": true, "errors": errors, "warnings": warnings })
    }

    #[test]
    fn a_transport_failure_with_an_error_string_is_that_code_blocking_and_mandatory() {
        let h = classify(&json!({ "connected": false, "error": "connection refused" }));
        assert_eq!(h.code.as_deref(), Some("connection refused"));
        assert_eq!(h.severity, FdmSeverity::Blocking);
        assert!(h.mandatory);
        assert!(!h.connected);
        assert!(!h.answered);
        assert_eq!(h.next_check_at, None);
        assert!(chrono::DateTime::parse_from_rfc3339(&h.checked_at).is_ok());
    }

    #[test]
    fn an_error_string_alone_is_a_transport_failure_too() {
        let h = classify(&json!({ "error": "timeout" }));
        assert_eq!(h.code.as_deref(), Some("timeout"));
        assert!(!h.answered);
    }

    #[test]
    fn connected_false_without_an_error_is_fdm_not_operational() {
        let h = classify(&json!({ "connected": false }));
        assert_eq!(h.code.as_deref(), Some("FDM_NOT_OPERATIONAL"));
        assert_eq!(h.severity, FdmSeverity::Blocking);
        assert!(h.mandatory);
        assert!(!h.connected);
        assert!(!h.answered);
    }

    #[test]
    fn a_value_that_is_not_an_object_is_fdm_not_operational() {
        let h = classify(&Value::Null);
        assert_eq!(h.code.as_deref(), Some("FDM_NOT_OPERATIONAL"));
        assert!(!h.answered);
    }

    #[test]
    fn a_mandatory_error_with_a_blocking_code_is_blocking_and_mandatory() {
        for code in [
            "BUFFER_FULL",
            "FDM_LOCKED",
            "FDM_NOT_OPERATIONAL",
            "UNKNOWN_POS",
        ] {
            let h = classify(&status(vec![item(Some(code), Some("MANDATORY"))], vec![]));
            assert_eq!(h.code.as_deref(), Some(code));
            assert_eq!(h.severity, FdmSeverity::Blocking, "{code}");
            assert!(h.mandatory);
            assert!(!h.connected);
            assert!(h.answered);
        }
    }

    #[test]
    fn a_mandatory_error_with_another_code_is_a_warning_and_mandatory() {
        let h = classify(&status(
            vec![item(Some("SOME_OTHER_CODE"), Some("MANDATORY"))],
            vec![],
        ));
        assert_eq!(h.code.as_deref(), Some("SOME_OTHER_CODE"));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(h.mandatory);
        assert!(h.connected);
        assert!(h.answered);
    }

    #[test]
    fn a_mandatory_error_is_preferred_to_an_earlier_plain_one() {
        let h = classify(&status(
            vec![
                item(Some("FIRST"), Some("ALWAYS")),
                item(Some("BUFFER_FULL"), Some("MANDATORY")),
            ],
            vec![],
        ));
        assert_eq!(h.code.as_deref(), Some("BUFFER_FULL"));
        assert!(h.mandatory);
    }

    #[test]
    fn a_non_never_error_has_its_severity_by_code_and_is_not_mandatory() {
        let h = classify(&status(vec![item(Some("BUFFER_FULL"), None)], vec![]));
        assert_eq!(h.code.as_deref(), Some("BUFFER_FULL"));
        assert_eq!(h.severity, FdmSeverity::Blocking);
        assert!(!h.mandatory);
        assert!(!h.connected);

        let h = classify(&status(vec![item(Some("OTHER"), Some("ALWAYS"))], vec![]));
        assert_eq!(h.code.as_deref(), Some("OTHER"));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(!h.mandatory);
        assert!(h.connected);
    }

    #[test]
    fn errors_that_are_never_shown_are_ignored() {
        let h = classify(&status(
            vec![
                item(Some("BUFFER_FULL"), Some("NEVER")),
                item(Some("FDM_LOCKED"), Some("NEVER")),
            ],
            vec![],
        ));
        assert_eq!(h.severity, FdmSeverity::Info);
        assert_eq!(h.code, None);
        assert!(h.connected);
        assert!(!h.mandatory);
    }

    #[test]
    fn a_never_error_does_not_hide_a_warning() {
        let h = classify(&status(
            vec![item(Some("BUFFER_FULL"), Some("NEVER"))],
            vec![item(Some("SOME_WARNING"), None)],
        ));
        assert_eq!(h.code.as_deref(), Some("SOME_WARNING"));
        assert_eq!(h.severity, FdmSeverity::Warning);
    }

    #[test]
    fn a_mandatory_warning_is_a_warning_and_mandatory() {
        let h = classify(&status(
            vec![],
            vec![
                item(Some("PLAIN"), None),
                item(Some("KEEP_ME"), Some("MANDATORY")),
            ],
        ));
        assert_eq!(h.code.as_deref(), Some("KEEP_ME"));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(h.mandatory);
        assert!(h.connected);
    }

    #[test]
    fn a_warning_never_blocks_whatever_its_code() {
        let h = classify(&status(vec![], vec![item(Some("BUFFER_FULL"), None)]));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(h.connected);
    }

    #[test]
    fn a_plain_warning_is_a_warning_and_not_mandatory() {
        let h = classify(&status(vec![], vec![item(Some("PLAIN"), Some("ALWAYS"))]));
        assert_eq!(h.code.as_deref(), Some("PLAIN"));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(!h.mandatory);
    }

    #[test]
    fn an_error_wins_over_a_warning() {
        let h = classify(&status(
            vec![item(Some("AN_ERROR"), None)],
            vec![item(Some("A_WARNING"), Some("MANDATORY"))],
        ));
        assert_eq!(h.code.as_deref(), Some("AN_ERROR"));
        assert!(!h.mandatory);
    }

    #[test]
    fn nothing_to_say_is_info_without_a_code_and_connected() {
        let h = classify(&status(vec![], vec![]));
        assert_eq!(h.severity, FdmSeverity::Info);
        assert_eq!(h.code, None);
        assert!(h.connected);
        assert!(h.answered);
        assert!(!h.mandatory);
        // Null lists, as the FDM may send them, are empty ones.
        let h = classify(&json!({ "initialized": true, "errors": null, "warnings": null }));
        assert_eq!(h.severity, FdmSeverity::Info);
        assert_eq!(h.code, None);
        // So is a status that carries none of the fields.
        let h = classify(&json!({}));
        assert_eq!(h.severity, FdmSeverity::Info);
        assert!(h.connected);
    }

    #[test]
    fn not_initialized_and_nothing_else_is_fdm_not_initialized_not_connected_info() {
        let h = classify(&json!({ "initialized": false, "errors": [], "warnings": [] }));
        assert_eq!(h.code.as_deref(), Some("FDM_NOT_INITIALIZED"));
        assert_eq!(h.severity, FdmSeverity::Info);
        assert!(!h.connected);
        assert!(h.answered);
        assert!(!h.mandatory);
    }

    #[test]
    fn not_initialized_with_a_warning_names_that_code_and_is_not_connected() {
        let h = classify(&json!({
            "initialized": false,
            "errors": [],
            "warnings": [item(Some("SOME_WARNING"), None)]
        }));
        assert_eq!(h.code.as_deref(), Some("SOME_WARNING"));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(!h.connected);
        assert!(h.answered);
    }

    #[test]
    fn a_first_mandatory_item_without_a_code_falls_through_like_js_find() {
        // find() takes the first MANDATORY error (no code) and stops: the
        // plain error after it is then looked for by the second find(),
        // whose first match is the same code-less item, so it yields
        // nothing either, and the warnings answer.
        let h = classify(&status(
            vec![
                item(None, Some("MANDATORY")),
                item(Some("BUFFER_FULL"), Some("MANDATORY")),
            ],
            vec![item(Some("A_WARNING"), None)],
        ));
        assert_eq!(h.code.as_deref(), Some("A_WARNING"));
        assert_eq!(h.severity, FdmSeverity::Warning);
        assert!(!h.mandatory);

        // The same for an item with no code that is first and not NEVER.
        let h = classify(&status(
            vec![item(None, None), item(Some("BUFFER_FULL"), None)],
            vec![],
        ));
        assert_eq!(h.severity, FdmSeverity::Info);
        assert_eq!(h.code, None);

        // And an empty code counts as none.
        let h = classify(&status(vec![item(Some(""), Some("MANDATORY"))], vec![]));
        assert_eq!(h.code, None);
    }

    #[test]
    fn poll_delay_doubles_to_ten_times_the_base() {
        let base = Duration::from_secs(30);
        let secs: Vec<u64> = (0..=5).map(|n| poll_delay(base, n).as_secs()).collect();
        assert_eq!(secs, [30, 60, 120, 240, 300, 300]);
    }

    #[test]
    fn poll_delay_does_not_overflow() {
        let base = Duration::from_secs(30);
        for failures in [31, 32, 33, 63, 64, 65, u32::MAX] {
            assert_eq!(poll_delay(base, failures), Duration::from_secs(300));
        }
        assert_eq!(poll_delay(Duration::MAX, 64), Duration::MAX);
        assert_eq!(
            poll_delay(Duration::from_millis(150), 3),
            Duration::from_millis(1200)
        );
    }

    #[test]
    fn the_health_serializes_as_the_ui_reads_it() {
        let h = classify(&status(vec![], vec![]));
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(v["severity"], "info");
        assert_eq!(v["connected"], true);
        assert_eq!(v["answered"], true);
        assert!(v["code"].is_null());
        assert!(v["nextCheckAt"].is_null());
        assert!(v["checkedAt"].is_string());
    }
}
