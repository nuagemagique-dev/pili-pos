use tauri::State;

use crate::commands::financial::{FinancialResult, MoneyDirection};

use super::Managed;

/// Record a cash financial event (Event F): money put in or taken
/// out (cash float, withdrawal, ...), or the count of the drawer.
#[tauri::command]
pub async fn money_in_out(
    state: State<'_, Managed>,
    employee_insz: String,
    amount_cents: i64,
    direction: MoneyDirection,
    reason: String,
) -> Result<FinancialResult, String> {
    crate::commands::financial::do_money_in_out(
        state.inner(),
        employee_insz,
        amount_cents,
        direction,
        reason,
    )
    .await
}

/// Record a fiscal drawer open event (Event F).
/// Tracks when the cash drawer is opened outside of a sale.
#[tauri::command]
pub async fn open_drawer_fiscal(
    state: State<'_, Managed>,
    employee_insz: String,
) -> Result<FinancialResult, String> {
    crate::commands::financial::do_open_drawer_fiscal(state.inner(), employee_insz).await
}
