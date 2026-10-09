use tauri::State;

use pos_engine::commands::export::ExportStatus;

use super::Managed;

/// The state of the nightly export (administrators only).
#[tauri::command]
pub async fn export_status(state: State<'_, Managed>) -> Result<ExportStatus, String> {
    pos_engine::commands::export::do_export_status(state.inner()).await
}

/// Make the pending exports due at once and wake the export loop; returns
/// the status without waiting for the pass (administrators only).
#[tauri::command]
pub async fn export_run_now(state: State<'_, Managed>) -> Result<ExportStatus, String> {
    pos_engine::commands::export::do_export_run_now(state.inner()).await
}
