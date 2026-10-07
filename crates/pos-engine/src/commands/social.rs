use serde::{Deserialize, Serialize};

use pos_fiscal::event::build_work_in_out;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::auth;
use crate::commands::errors;
use crate::commands::signing::{self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared};
use crate::events::EngineEvent;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Register employee clock-in (ONSS presence start).
/// Sends event S (signWorkIn) to the FDM.
pub async fn do_work_in(state: &AppState, employee_insz: String) -> Result<SocialResult, String> {
    do_work_event(state, employee_insz, true).await
}

/// Register employee clock-out (ONSS presence end).
/// Sends event S (signWorkOut) to the FDM.
pub async fn do_work_out(state: &AppState, employee_insz: String) -> Result<SocialResult, String> {
    do_work_event(state, employee_insz, false).await
}

async fn do_work_event(
    state: &AppState,
    employee_insz: String,
    is_in: bool,
) -> Result<SocialResult, String> {
    access::operator_named(state, &employee_insz, Access::Staff)?;
    let kind = if is_in { Kind::WorkIn } else { Kind::WorkOut };
    let op = Op::new(kind, &employee_insz);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // The same clock-in or out, answered late: done.
        Ok(Begin::AlreadyDone(_)) => return Ok(SocialResult::done()),
        Err(e) => return Ok(SocialResult::failed(e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state, &guard)?;
    let label = EventLabel::S.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, &guard, label)?;

    let input = build_work_in_out(
        &config,
        &state.identity,
        ticket_number,
        &employee_insz,
        &bp_id,
    );
    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;
    let mutation = if is_in {
        SignMutation::WorkIn
    } else {
        SignMutation::WorkOut
    };
    let request =
        SignRequest::new(mutation, &input, config.training_mode).map_err(errors::internal)?;

    let prepared = Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: None,
            user_insz: employee_insz.clone(),
            request_json,
            sent_entries: None,
        }],
        order_id: None,
        user_insz: employee_insz.clone(),
        context: PendingContext::Work {
            insz: employee_insz,
            is_in,
        },
    };
    match signing::sign(state, &guard, &config, prepared).await {
        Ok(_) => Ok(SocialResult::done()),
        Err(failure) => Ok(SocialResult::failed(failure.error)),
    }
}

impl SocialResult {
    pub(crate) fn done() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    pub(crate) fn failed(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
        }
    }
}

/// Employees currently on shift (clocked in, not yet clocked out),
/// shown on the login screen.
pub fn do_list_open_shifts(
    state: &AppState,
) -> Result<Vec<pos_db::queries::shifts::OpenShift>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::shifts::list_open_shifts(&db).map_err(|e| e.to_string())
}

/// End the shift: check the PIN exactly like a login, sign the
/// employee's clock-out and close the session in every case.
///
/// The login checks are [`auth::authenticate`] (a refused PIN is an
/// `Err` with the login's code, and nothing is signed or opened). The
/// clock-out is [`do_work_out`]: a failure (an error, or a result that
/// is not a success) comes back as `success: false` with the error,
/// never as `presence-failed`. It queues no clock-in and leaves the
/// active order alone: the shift ends, the screen goes back to the
/// login, whatever the FDM answered.
pub async fn do_end_shift(
    state: &AppState,
    insz: String,
    pin: Option<String>,
) -> Result<SocialResult, String> {
    let user = auth::authenticate(state, &insz, pin)?;
    access::open_session(state, &user)?;
    let result = match do_work_out(state, insz).await {
        Ok(result) => result,
        Err(e) => SocialResult::failed(e),
    };
    access::close_session(state)?;
    Ok(result)
}

/// The `presence-failed` event: a clock-in the engine tried at login
/// was not registered (the UI shows the `presence_failed` toast).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceFailed {
    pub insz: String,
    /// Always true for now: a clock-out is answered in the result of
    /// `end_shift`.
    pub clock_in: bool,
    pub error: String,
}

/// Clock `insz` in unless they are already on shift: the rule of the
/// UI's `clockInIfOffShift`, run by the presence worker after a login.
///
/// - when the lookup of open shifts fails, clock in anyway (a second
///   WorkIn is harmless);
/// - when the employee is already on shift (the screen was only
///   locked), do nothing;
/// - a failure (an error, or a result that is not a success) emits
///   `presence-failed`; it never reaches the login.
pub(crate) async fn clock_in_if_off_shift(state: &AppState, insz: &str) {
    let on_shift = match do_list_open_shifts(state) {
        Ok(shifts) => shifts.iter().any(|s| s.user_insz == insz),
        Err(e) => {
            tracing::warn!(%e, "open shifts unknown: clocking in anyway");
            false
        }
    };
    if on_shift {
        return;
    }
    let error = match do_work_in(state, insz.to_string()).await {
        Ok(SocialResult { success: true, .. }) => return,
        Ok(SocialResult { error, .. }) => error.unwrap_or_else(|| "FDM error".to_string()),
        Err(e) => e,
    };
    tracing::warn!(%error, "clock-in at login not registered");
    state
        .events
        .emit(EngineEvent::PresenceFailed(PresenceFailed {
            insz: insz.to_string(),
            clock_in: true,
            error,
        }));
}
