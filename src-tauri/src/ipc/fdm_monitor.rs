use tauri::State;

use pos_engine::commands::fdm_monitor::FdmHealth;

use super::Managed;

/// The last FDM health the engine's monitor knows (`null` before the
/// first check). The same value the `fdm-status` event carries.
#[tauri::command(async)]
pub fn fdm_health(state: State<'_, Managed>) -> Result<Option<FdmHealth>, String> {
    pos_engine::commands::fdm_monitor::do_fdm_health(state.inner())
}

/// Check the FDM now (the login screen, the retry button): the health is
/// stored, emitted as `fdm-status`, and the monitor schedules from now.
/// Works without a session.
#[tauri::command]
pub async fn fdm_check_now(state: State<'_, Managed>) -> Result<FdmHealth, String> {
    pos_engine::commands::fdm_monitor::do_fdm_check_now(state.inner()).await
}
