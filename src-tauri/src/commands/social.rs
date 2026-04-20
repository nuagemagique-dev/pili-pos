use serde::Serialize;
use tauri::State;

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_work_in_out;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::types::EventLabel;

use crate::state::AppState;

#[derive(Serialize)]
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
    do_work_event(state, employee_insz, true).await
}

/// Register employee clock-out (ONSS presence end).
/// Sends event S (signWorkOut) to the FDM.
#[tauri::command]
pub async fn work_out(
    state: State<'_, AppState>,
    employee_insz: String,
) -> Result<SocialResult, String> {
    do_work_event(state, employee_insz, false).await
}

async fn do_work_event(
    state: State<'_, AppState>,
    employee_insz: String,
    is_in: bool,
) -> Result<SocialResult, String> {
    let (config, bp_id, ticket_number) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state.booking_period_id.lock().map_err(|e| e.to_string())?.clone();
        let tn = FiscalCounter::next(&db, EventLabel::S).map_err(|e| e.to_string())?;
        (config, bp, tn)
    };

    let input = build_work_in_out(&config, ticket_number, &employee_insz, &bp_id);
    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = if is_in {
        fdm.sign_work_in(input, config.training_mode).await.ok()
    } else {
        fdm.sign_work_out(input, config.training_mode).await.ok()
    };

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        archive::archive_event(&db, ArchiveParams {
            event_type: EventLabel::S,
            sequence_number: ticket_number,
            order_id: None,
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: &employee_insz,
            request_json: &request_json,
            sign_result: sign_result.as_ref(),
        }).map_err(|e| e.to_string())?;
    }

    Ok(SocialResult {
        success: true,
        error: None,
    })
}
