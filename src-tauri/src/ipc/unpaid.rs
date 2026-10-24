use tauri::State;

use pos_engine::commands::checkout::CheckoutResult;

use super::Managed;

#[tauri::command]
pub async fn close_unpaid(
    state: State<'_, Managed>,
    order_id: i64,
    insz: String,
    pin: String,
) -> Result<CheckoutResult, String> {
    pos_engine::commands::unpaid::do_close_unpaid(state.inner(), order_id, &insz, &pin).await
}
