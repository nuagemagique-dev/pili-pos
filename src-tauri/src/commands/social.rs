use serde::Serialize;
use tauri::State;

use pos_fiscal::event::build_work_in_out;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::types::EventLabel;

use crate::commands::access::{self, Access};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialResult {
    pub success: bool,
    pub error: Option<String>,
}

/// Register employee clock-in (ONSS presence start).
/// Sends event S (signWorkIn) to the FDM.
#[tauri::command]
pub async fn work_in(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<SocialResult, String> {
    do_work_in(state.inner(), employee_insz).await
}

pub async fn do_work_in(state: &AppState, employee_insz: String) -> Result<SocialResult, String> {
    do_work_event(state, employee_insz, true).await
}

/// Register employee clock-out (ONSS presence end).
/// Sends event S (signWorkOut) to the FDM.
#[tauri::command]
pub async fn work_out(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<SocialResult, String> {
    do_work_out(state.inner(), employee_insz).await
}

pub async fn do_work_out(state: &AppState, employee_insz: String) -> Result<SocialResult, String> {
    do_work_event(state, employee_insz, false).await
}

async fn do_work_event(
    state: &AppState,
    employee_insz: String,
    is_in: bool,
) -> Result<SocialResult, String> {
    access::operator_named(state, &employee_insz, Access::Staff)?;
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state)?;
    let label = EventLabel::S.in_mode(config.training_mode);
    let ticket_number = super::peek_ticket_number(state, label)?;

    let input = build_work_in_out(&config, ticket_number, &employee_insz, &bp_id);
    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::from_config(&config);
    let signed = if is_in {
        fdm.sign_work_in(input, config.training_mode).await
    } else {
        fdm.sign_work_out(input, config.training_mode).await
    };
    let sign_result = match signed {
        Ok(result) => result,
        Err(e) => {
            return Ok(SocialResult {
                success: false,
                error: Some(format!("FDM error: {e}")),
            });
        }
    };

    super::commit_signed_event(
        state,
        &config,
        super::SignedEvent {
            label,
            ticket_number,
            order_id: None,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: &sign_result,
            sent_entries: None,
        },
    )?;

    // The screen lock keeps a shift open; only a signed clock-out ends it.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let recorded = if is_in {
            pos_db::queries::shifts::open_shift(&db, &employee_insz)
        } else {
            pos_db::queries::shifts::close_shift(&db, &employee_insz)
        };
        recorded.map_err(|e| e.to_string())?;
    }

    Ok(SocialResult {
        success: true,
        error: None,
    })
}

/// Employees currently on shift (clocked in, not yet clocked out),
/// shown on the login screen.
#[tauri::command]
pub fn list_open_shifts(
    state: State<'_, AppState>,
) -> Result<Vec<pos_db::queries::shifts::OpenShift>, String> {
    do_list_open_shifts(state.inner())
}

pub fn do_list_open_shifts(
    state: &AppState,
) -> Result<Vec<pos_db::queries::shifts::OpenShift>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::shifts::list_open_shifts(&db).map_err(|e| e.to_string())
}
