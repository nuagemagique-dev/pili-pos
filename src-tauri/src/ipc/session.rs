use tauri::State;

use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::session::{LeaveReason, LockOutcome};

use super::Managed;

/// Tell the engine which order the screen shows (`null`: none), so the
/// lock can register it. The order must exist.
#[tauri::command]
pub fn set_active_order(state: State<'_, Managed>, order_id: Option<i64>) -> Result<(), String> {
    pos_engine::commands::session::do_set_active_order(state.inner(), order_id)
}

/// The operator leaves an order: register what was ordered (event P)
/// unless it is a takeaway or no longer open. `reason` is `navigate`,
/// `switch_order`, `send_to_kitchen`, `lock` or `idle`; the order stays
/// the active one only for `send_to_kitchen`.
#[tauri::command]
pub async fn leave_order(
    state: State<'_, Managed>,
    order_id: i64,
    reason: LeaveReason,
) -> Result<SlipOutcome, String> {
    pos_engine::commands::session::do_leave_order(state.inner(), order_id, reason).await
}

/// Lock the screen: register the active order, then close the session
/// (the operator stays on shift). The engine also emits `session-locked`.
#[tauri::command]
pub async fn lock_session(state: State<'_, Managed>) -> Result<LockOutcome, String> {
    pos_engine::commands::session::do_lock_session(state.inner()).await
}
