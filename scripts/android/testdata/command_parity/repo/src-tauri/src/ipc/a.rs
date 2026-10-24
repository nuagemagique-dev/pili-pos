use tauri::State;

use super::Managed;

#[tauri::command]
pub fn get_alpha(state: State<'_, Managed>) -> Result<String, String> {
    crate::commands::alpha::do_get_alpha(state.inner())
}

#[tauri::command]
pub fn set_beta(
    state: State<'_, Managed>,
    item_id: i64,
    label: String,
) -> Result<(), String> {
    crate::commands::alpha::do_set_beta(state.inner(), item_id, label)
}

#[tauri::command]
pub async fn check_gamma(state: State<'_, Managed>, order_id: i64) -> Result<bool, String> {
    crate::commands::gamma::do_check_gamma(state.inner(), order_id).await
}
