use tauri::State;

use crate::commands::checkout::CheckoutResult;
use crate::commands::webshop::{AcceptRequest, AcceptResult, HandoverRequest, RefundRequest};

use super::Managed;

#[tauri::command]
pub async fn webshop_accept_order(
    state: State<'_, Managed>,
    req: AcceptRequest,
) -> Result<AcceptResult, String> {
    crate::commands::webshop::do_accept_webshop_order(
        state.inner(),
        req.order_id,
        req.confirmed_time.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn webshop_handover_order(
    state: State<'_, Managed>,
    req: HandoverRequest,
) -> Result<CheckoutResult, String> {
    crate::commands::webshop::do_handover_webshop_order(
        state.inner(),
        req.order_id,
        &req.payments,
        req.delivery.as_ref(),
    )
    .await
}

#[tauri::command]
pub fn webshop_reject_order(state: State<'_, Managed>, order_id: i64) -> Result<(), String> {
    crate::commands::webshop::do_reject_webshop_order(state.inner(), order_id)
}

#[tauri::command]
pub fn webshop_complete_order(state: State<'_, Managed>, order_id: i64) -> Result<(), String> {
    crate::commands::webshop::do_complete_webshop_order(state.inner(), order_id)
}

#[tauri::command]
pub async fn webshop_refund_order(
    state: State<'_, Managed>,
    req: RefundRequest,
) -> Result<CheckoutResult, String> {
    crate::commands::webshop::do_refund_webshop_order(
        state.inner(),
        req.order_id,
        &req.employee_insz,
        req.delivery.as_ref(),
    )
    .await
}

#[tauri::command]
pub async fn webshop_close_cancelled_order(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<CheckoutResult, String> {
    crate::commands::webshop::do_close_cancelled_webshop_order(state.inner(), order_id).await
}
