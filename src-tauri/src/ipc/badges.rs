use tauri::State;

use pos_engine::commands::badges::{
    BadgeLink, BadgeOutcome, BadgeOverview, BadgeSettingsInput, BadgeSettingsView,
};

use super::Managed;

/// The badge reader setting. No session needed: the login screen arms
/// its reader from it.
#[tauri::command(async)]
pub fn get_badge_settings(state: State<'_, Managed>) -> Result<BadgeSettingsView, String> {
    pos_engine::commands::badges::do_get_badge_settings(state.inner())
}

/// Change the badge reader setting (managers only, logged).
#[tauri::command(async)]
pub fn set_badge_settings(
    state: State<'_, Managed>,
    input: BadgeSettingsInput,
) -> Result<BadgeSettingsView, String> {
    pos_engine::commands::badges::do_set_badge_settings(state.inner(), input)
}

/// The active badges and the latest log rows (managers only).
#[tauri::command(async)]
pub fn list_badges(state: State<'_, Managed>) -> Result<BadgeOverview, String> {
    pos_engine::commands::badges::do_list_badges(state.inner())
}

/// Link a badge to an employee (a manager, confirmed by their PIN).
#[tauri::command(async)]
pub fn link_badge(
    state: State<'_, Managed>,
    employee_insz: String,
    token: String,
    operator_pin: Option<String>,
) -> Result<BadgeLink, String> {
    pos_engine::commands::badges::do_link_badge(state.inner(), employee_insz, token, operator_pin)
}

/// Remove an employee's badge (managers only).
#[tauri::command(async)]
pub fn revoke_badge(state: State<'_, Managed>, employee_insz: String) -> Result<(), String> {
    pos_engine::commands::badges::do_revoke_badge(state.inner(), employee_insz)
}

/// A badge scan: presence, login or a PIN request, decided by the engine.
#[tauri::command]
pub async fn badge_tap(
    state: State<'_, Managed>,
    token: String,
    end_shift: bool,
) -> Result<BadgeOutcome, String> {
    pos_engine::commands::badges::do_badge_tap(state.inner(), token, end_shift).await
}
