use pos_db::queries::owner_settings::AlertLogRow;
use pos_engine::commands::alerts::{AlertTestResult, OwnerAlerts, OwnerAlertsInput};
use tauri::State;

use super::Managed;

/// The owner alert settings (Telegram messages to the owner's phone);
/// managers only. The bot token is never returned, only whether it is
/// configured.
#[tauri::command(async)]
pub fn get_owner_alerts(state: State<'_, Managed>) -> Result<OwnerAlerts, String> {
    pos_engine::commands::alerts::do_get_owner_alerts(state.inner())
}

/// Store the owner alert settings; managers only, logged.
#[tauri::command(async)]
pub fn set_owner_alerts(
    state: State<'_, Managed>,
    input: OwnerAlertsInput,
) -> Result<OwnerAlerts, String> {
    pos_engine::commands::alerts::do_set_owner_alerts(state.inner(), input)
}

/// Send one test message to the owner's chat now; managers only.
#[tauri::command]
pub async fn send_test_alert(state: State<'_, Managed>) -> Result<AlertTestResult, String> {
    pos_engine::commands::alerts::do_send_test_alert(state.inner()).await
}

/// The latest alert messages sent or dropped, newest first; managers
/// only.
#[tauri::command(async)]
pub fn list_owner_alert_log(
    state: State<'_, Managed>,
    limit: i64,
) -> Result<Vec<AlertLogRow>, String> {
    pos_engine::commands::alerts::do_list_owner_alert_log(state.inner(), limit)
}
