use tauri::State;

use super::Managed;

#[tauri::command]
pub async fn sign_thing(state: State<'_, Managed>, order_id: i64) -> Result<(), String> {
    crate::commands::inv::do_sign_thing(state.inner(), order_id).await
}
