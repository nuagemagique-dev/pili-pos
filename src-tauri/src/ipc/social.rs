use tauri::State;

use crate::commands::social::SocialResult;

use super::Managed;

/// Register employee clock-in (ONSS presence start).
/// Sends event S (signWorkIn) to the FDM.
#[tauri::command]
pub async fn work_in(
    state: State<'_, Managed>,
    employee_insz: String,
) -> Result<SocialResult, String> {
    crate::commands::social::do_work_in(state.inner(), employee_insz).await
}

/// Register employee clock-out (ONSS presence end).
/// Sends event S (signWorkOut) to the FDM.
#[tauri::command]
pub async fn work_out(
    state: State<'_, Managed>,
    employee_insz: String,
) -> Result<SocialResult, String> {
    crate::commands::social::do_work_out(state.inner(), employee_insz).await
}

/// Employees currently on shift (clocked in, not yet clocked out),
/// shown on the login screen.
#[tauri::command]
pub fn list_open_shifts(
    state: State<'_, Managed>,
) -> Result<Vec<pos_db::queries::shifts::OpenShift>, String> {
    crate::commands::social::do_list_open_shifts(state.inner())
}
