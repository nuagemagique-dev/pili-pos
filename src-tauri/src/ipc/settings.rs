use tauri::State;

use crate::commands::settings::PosSettings;

use super::Managed;

#[tauri::command]
pub fn get_settings(state: State<'_, Managed>) -> Result<PosSettings, String> {
    crate::commands::settings::do_get_settings(state.inner())
}

#[tauri::command]
pub fn set_training_mode(state: State<'_, Managed>, enabled: bool) -> Result<bool, String> {
    crate::commands::settings::do_set_training_mode(state.inner(), enabled)
}
