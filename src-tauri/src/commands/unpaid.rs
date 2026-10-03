//! A sale closed without payment: the customer left without paying
//! (Advanced Set UC4109). What was served is still sold, so the sale is
//! signed (N, or T in training) with no payment line and its VAT ticket
//! printed; no money moves: no drawer, no payment row, and it can never
//! be refunded. A manager confirms it with their PIN.

use tauri::State;

use crate::commands::access::{self, Access};
use crate::commands::checkout::{self, CheckoutResult, Payment};
use crate::state::AppState;

#[tauri::command]
pub async fn close_unpaid(
    state: State<'_, AppState>,
    order_id: i64,
    insz: String,
    pin: String,
) -> Result<CheckoutResult, String> {
    do_close_unpaid(state.inner(), order_id, &insz, &pin).await
}

/// Close `order_id` unpaid in the name of the logged-in manager
/// `insz`, confirmed by `pin`. Allowed in training (it signs a T):
/// training cannot be left while an order is open, so an abandoned
/// training table must be closable too.
pub async fn do_close_unpaid(
    state: &AppState,
    order_id: i64,
    insz: &str,
    pin: &str,
) -> Result<CheckoutResult, String> {
    let manager = match access::confirmed_operator(state, insz, pin, Access::Admin) {
        Ok(user) => user,
        Err(e) => return Ok(CheckoutResult::refused(e)),
    };
    tracing::warn!(order_id, role = ?manager.role, "sale closed unpaid");
    checkout::sell(
        state,
        manager,
        order_id,
        Payment::Unpaid,
        None,
        checkout::SaleOptions::default(),
    )
    .await
}
