use serde::Serialize;
use tauri::State;

use crate::commands::access::{self, Access};
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosSettings {
    pub training_mode: bool,
    pub printer_address: Option<String>,
    pub fdm_url: String,
    pub pos_id: String,
    pub terminal_id: String,
    pub restaurant_name: String,
    pub language: String,
    /// What the FDM would refuse in `pos.json` ([`PosConfig::problems`]):
    /// shown as a banner so a bad identity is fixed before opening.
    ///
    /// [`PosConfig::problems`]: pos_core::config::PosConfig::problems
    pub config_problems: Vec<String>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<PosSettings, String> {
    do_get_settings(state.inner())
}

pub fn do_get_settings(state: &AppState) -> Result<PosSettings, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(PosSettings {
        training_mode: config.training_mode,
        printer_address: config.printer_address.clone(),
        fdm_url: config.fdm_url.clone(),
        pos_id: config.pos_id.clone(),
        terminal_id: config.terminal_id.clone(),
        restaurant_name: config.restaurant_name.clone(),
        language: config.language.clone(),
        config_problems: config.problems(),
    })
}

#[tauri::command]
pub fn set_training_mode(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    do_set_training_mode(state.inner(), enabled)
}

pub const ERR_OPEN_ORDERS: &str = "Pay or empty the open orders before switching training mode";

/// Switch training mode. Managers only, and never while an order with
/// lines is open: a real table would otherwise be paid as a training
/// sale (T, never counted), or a training order paid as a real one.
pub fn do_set_training_mode(state: &AppState, enabled: bool) -> Result<bool, String> {
    access::operator(state, Access::Admin)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let busy: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM orders o
                 WHERE o.status = 'open'
                   AND EXISTS (SELECT 1 FROM order_lines l WHERE l.order_id = o.id)",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut config = state.config.lock().map_err(|e| e.to_string())?;
        if config.training_mode == enabled {
            return Ok(enabled);
        }
        if busy > 0 {
            return Err(ERR_OPEN_ORDERS.to_string());
        }
        config.training_mode = enabled;
    }
    Ok(enabled)
}
