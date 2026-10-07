use serde::Serialize;

use pos_fiscal::event::build_work_in_out;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::signing::{self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared};
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

    let input = build_work_in_out(&config, ticket_number, &employee_insz, &bp_id);
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
    fn done() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    fn failed(error: String) -> Self {
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
