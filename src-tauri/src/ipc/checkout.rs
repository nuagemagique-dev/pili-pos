use pos_fiscal::payment::PaymentPart;
use tauri::State;

use crate::commands::checkout::CheckoutResult;
use crate::commands::delivery::TicketDeliveryRequest;

use super::Managed;

#[tauri::command]
pub async fn checkout(
    state: State<'_, Managed>,
    order_id: i64,
    payments: Vec<PaymentPart>,
    delivery: Option<TicketDeliveryRequest>,
) -> Result<CheckoutResult, String> {
    crate::commands::checkout::do_checkout_split(
        state.inner(),
        order_id,
        &payments,
        delivery.as_ref(),
    )
    .await
}

/// Check FDM connection status.
#[tauri::command]
pub async fn check_fdm_status(state: State<'_, Managed>) -> Result<serde_json::Value, String> {
    crate::commands::checkout::do_check_fdm_status(state.inner()).await
}
