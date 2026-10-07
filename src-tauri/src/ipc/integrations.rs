use tauri::State;

use crate::commands::integrations::Integrations;

use super::Managed;

#[tauri::command]
pub fn get_integrations(state: State<'_, Managed>) -> Result<Integrations, String> {
    crate::commands::integrations::do_get_integrations(state.inner())
}

#[tauri::command]
pub fn set_hubrise_enabled(
    state: State<'_, Managed>,
    enabled: bool,
) -> Result<Integrations, String> {
    crate::commands::integrations::do_set_hubrise_enabled(state.inner(), enabled)
}
