use tauri::State;

use pos_core::order::OrderLine;
use pos_engine::commands::scale::{ScaleStatus, ScaleWeighInput};

use super::Managed;

/// Whether the checkout scale answers.
#[tauri::command]
pub async fn get_scale_status(state: State<'_, Managed>) -> Result<ScaleStatus, String> {
    pos_engine::commands::scale::do_get_scale_status(state.inner()).await
}

/// Weigh a product sold by weight and add its line to the order.
#[tauri::command]
pub async fn scale_start_weighing(
    state: State<'_, Managed>,
    input: ScaleWeighInput,
) -> Result<OrderLine, String> {
    pos_engine::commands::scale::do_scale_start_weighing(state.inner(), input).await
}

/// End the weighing in progress.
#[tauri::command]
pub fn scale_cancel_weighing(state: State<'_, Managed>) -> Result<(), String> {
    pos_engine::commands::scale::do_scale_cancel_weighing(state.inner())
}
