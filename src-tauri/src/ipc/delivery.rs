use tauri::State;

use pos_engine::commands::delivery::{DeliveryView, MailStatus};

use super::Managed;

#[tauri::command]
pub async fn check_mail_status(state: State<'_, Managed>) -> Result<MailStatus, String> {
    pos_engine::commands::delivery::do_check_mail_status(state.inner()).await
}

#[tauri::command]
pub fn list_ticket_deliveries(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<Vec<DeliveryView>, String> {
    pos_engine::commands::delivery::do_list_ticket_deliveries(state.inner(), limit)
}

/// Print an original that was never printed: the printer failed at
/// checkout, or a DIGITAL ticket's email didn't go out. The ticket is
/// rebuilt from the archive (a sale or a copy only). Refused once the
/// original was printed (a second print must be a copy, event C) and
/// for a DIGITAL ticket whose email was sent.
#[tauri::command]
pub async fn print_missing_original(
    state: State<'_, Managed>,
    fiscal_event_id: i64,
) -> Result<(), String> {
    pos_engine::commands::delivery::do_print_missing_original(state.inner(), fiscal_event_id).await
}
