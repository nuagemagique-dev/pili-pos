use tauri::State;

use pos_engine::commands::financial::{FinancialResult, MoneyDirection};
use pos_engine::commands::payment_correction::SalePayments;
use pos_fiscal::payment::PaymentMethod;

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
    pos_engine::commands::financial::do_money_in_out(
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
    pos_engine::commands::financial::do_open_drawer_fiscal(state.inner(), employee_insz).await
}

/// The payment lines of a signed sale and whether (and why not) one
/// can be corrected. Managers only.
#[tauri::command]
pub fn list_sale_payments(
    state: State<'_, Managed>,
    fiscal_event_id: i64,
) -> Result<SalePayments, String> {
    pos_engine::commands::payment_correction::do_list_sale_payments(state.inner(), fiscal_event_id)
}

/// Correct the payment method of one payment line of a sale signed
/// today (event F, payment correction). The manager confirms with the
/// PIN; a refusal comes back as `success: false` with its error code.
#[tauri::command]
pub async fn correct_payment(
    state: State<'_, Managed>,
    fiscal_event_id: i64,
    line_index: i32,
    method: PaymentMethod,
    insz: String,
    pin: String,
) -> Result<FinancialResult, String> {
    pos_engine::commands::payment_correction::do_correct_payment(
        state.inner(),
        fiscal_event_id,
        line_index,
        method,
        insz,
        pin,
    )
    .await
}
