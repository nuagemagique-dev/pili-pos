use tauri::State;

use pos_engine::commands::settings::PosSettings;

use super::Managed;

#[tauri::command(async)]
pub fn get_settings(state: State<'_, Managed>) -> Result<PosSettings, String> {
    pos_engine::commands::settings::do_get_settings(state.inner())
}

#[tauri::command(async)]
pub fn set_training_mode(state: State<'_, Managed>, enabled: bool) -> Result<bool, String> {
    pos_engine::commands::settings::do_set_training_mode(state.inner(), enabled)
}
