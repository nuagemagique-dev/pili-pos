use tauri::State;

use super::Managed;

/// Check if the printer is reachable.
#[tauri::command]
pub async fn check_printer(state: State<'_, Managed>) -> Result<bool, String> {
    crate::commands::print::do_check_printer(state.inner()).await
}

/// Check if the bar / kitchen printer is reachable. `None` when the
/// terminal has no separate kitchen printer (slips go to the ticket
/// printer), so the header can grey out its light.
#[tauri::command]
pub async fn check_kitchen_printer(state: State<'_, Managed>) -> Result<Option<bool>, String> {
    crate::commands::print::do_check_kitchen_printer(state.inner()).await
}

/// Print a kitchen ticket (items + options + notes, non-fiscal).
#[cfg(feature = "hubrise")]
#[tauri::command]
pub async fn print_webshop_kitchen(state: State<'_, Managed>, order_id: i64) -> Result<(), String> {
    crate::commands::print::do_print_webshop_kitchen(state.inner(), order_id).await
}

/// Print a customer slip (items + total + customer, non-fiscal).
#[cfg(feature = "hubrise")]
#[tauri::command]
pub async fn print_webshop_customer(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<(), String> {
    crate::commands::print::do_print_webshop_customer(state.inner(), order_id).await
}
