use pos_db::queries::fiscal::RefundableEvent;
use tauri::State;

use pos_engine::commands::checkout::CheckoutResult;
use pos_engine::commands::delivery::TicketDeliveryRequest;
use pos_engine::commands::refund::RefundItem;

use super::Managed;

/// List recent closed N-events that can be refunded.
#[tauri::command]
pub fn list_refundable_events(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<Vec<RefundableEvent>, String> {
    pos_engine::commands::refund::do_list_refundable_events(state.inner(), limit)
}

/// Process a full or partial refund for a previously signed ticket.
///
/// Creates a new Event N with:
/// - negative-quantity lines (negQuantityReason = REFUND)
/// - fdmRef referencing the original ticket
/// - cash payment (refund handed to customer)
///
/// Archives the event, prints a REFUND receipt, opens the drawer.
#[tauri::command]
pub async fn refund_checkout(
    state: State<'_, Managed>,
    original_fiscal_event_id: i64,
    items: Vec<RefundItem>,
    employee_insz: String,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    pos_engine::commands::refund::do_refund_checkout_with(
        state.inner(),
        original_fiscal_event_id,
        items,
        employee_insz,
        delivery.as_ref(),
    )
    .await
}
