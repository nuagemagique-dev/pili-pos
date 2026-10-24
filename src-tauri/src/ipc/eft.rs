use tauri::State;

use pos_engine::commands::eft::{EftPayment, PaymentTerminalStatus, UnsettledCardPayment};

use super::Managed;

/// Ask the linked card terminal to charge `amount_cents` for an order.
#[tauri::command]
pub async fn eft_pay(
    state: State<'_, Managed>,
    order_id: i64,
    amount_cents: i64,
) -> Result<EftPayment, String> {
    pos_engine::commands::eft::do_eft_pay(state.inner(), order_id, amount_cents).await
}

/// Stop the card payment in progress on the terminal, if any.
#[tauri::command]
pub async fn eft_abort(state: State<'_, Managed>) -> Result<(), String> {
    pos_engine::commands::eft::do_eft_abort(state.inner()).await
}

/// Ask the terminal where a payment that has no final answer stands.
#[tauri::command]
pub async fn eft_check(state: State<'_, Managed>, id: i64) -> Result<EftPayment, String> {
    pos_engine::commands::eft::do_eft_check(state.inner(), id).await
}

/// An order's card payments to settle or to check, oldest first.
#[tauri::command]
pub fn list_eft_payments(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<Vec<EftPayment>, String> {
    pos_engine::commands::eft::do_list_eft_payments(state.inner(), order_id)
}

/// Whether the linked card terminal answers. Charges nothing; while a
/// payment runs it says "busy" without probing.
#[tauri::command]
pub async fn check_payment_terminal(
    state: State<'_, Managed>,
) -> Result<PaymentTerminalStatus, String> {
    pos_engine::commands::eft::do_check_payment_terminal(state.inner()).await
}

/// The approved card payments no sale has settled yet, of every order:
/// the Z warns about them and the order lists mark them.
#[tauri::command]
pub fn list_unsettled_card_payments(
    state: State<'_, Managed>,
) -> Result<Vec<UnsettledCardPayment>, String> {
    pos_engine::commands::eft::do_list_unsettled_card_payments(state.inner())
}
