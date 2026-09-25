use serde::Serialize;
use tauri::State;

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
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<PosSettings, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(PosSettings {
        training_mode: config.training_mode,
        printer_address: config.printer_address.clone(),
        fdm_url: config.fdm_url.clone(),
        pos_id: config.pos_id.clone(),
        terminal_id: config.terminal_id.clone(),
        restaurant_name: config.restaurant_name.clone(),
        language: config.language.clone(),
    })
}

#[tauri::command]
pub fn set_training_mode(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;
    config.training_mode = enabled;
    Ok(enabled)
}
