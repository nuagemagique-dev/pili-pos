use tauri::State;

use crate::commands::support::{SupportRunning, SupportStatus};

use super::Managed;

#[tauri::command]
pub fn remote_support_status(state: State<'_, Managed>) -> Result<SupportStatus, String> {
    crate::commands::support::do_remote_support_status(state.inner())
}

#[tauri::command]
pub fn remote_support_running(state: State<'_, Managed>) -> Result<SupportRunning, String> {
    crate::commands::support::do_remote_support_running(state.inner())
}

#[tauri::command]
pub fn start_remote_support(
    state: State<'_, Managed>,
    insz: String,
    pin: String,
) -> Result<SupportStatus, String> {
    crate::commands::support::do_start_remote_support(state.inner(), insz, pin)
}

#[tauri::command]
pub fn stop_remote_support(state: State<'_, Managed>) -> Result<SupportStatus, String> {
    crate::commands::support::do_stop_remote_support(state.inner())
}
