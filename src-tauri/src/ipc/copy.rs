use tauri::State;

use crate::commands::copy::CopyResult;
use crate::commands::delivery::TicketDeliveryRequest;

use super::Managed;

/// Create a copy (event C) of a previous fiscal event.
/// Per SCE 2.0 spec: references the original via fdmId + eventLabel + counters.
#[tauri::command]
pub async fn copy_ticket(
    state: State<'_, Managed>,
    fiscal_event_id: i64,
    employee_insz: String,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CopyResult, String> {
    crate::commands::copy::do_copy_ticket_with(
        state.inner(),
        fiscal_event_id,
        employee_insz,
        delivery.as_ref(),
    )
    .await
}

/// Recent tickets a customer received, for the copy picker: managers
/// only, like the copy itself.
#[tauri::command]
pub fn list_copyable_tickets(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<Vec<pos_db::queries::fiscal::CopyableTicket>, String> {
    crate::commands::copy::do_list_copyable_tickets(state.inner(), limit)
}

/// List recent fiscal events.
#[tauri::command]
pub fn list_fiscal_events(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    crate::commands::copy::do_list_fiscal_events(state.inner(), limit)
}
