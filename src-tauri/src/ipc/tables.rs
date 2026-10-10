use tauri::State;

use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::table_plan::TablePlan;
use pos_engine::commands::tables::{TableInfo, TakeawayInfo};

use super::Managed;

/// List tables with their current order status.
///
/// A table is only reported as **occupied** when its open order
/// contains at least one line. Empty open orders (typical after a
/// tap-and-walk-away) are treated as free so the TablesView doesn't
/// surface ghost "0.00 €" cards that add cognitive load for zero
/// informational value (UX Guide §2.1).
#[tauri::command]
pub fn list_tables(state: State<'_, Managed>) -> Result<Vec<TableInfo>, String> {
    pos_engine::commands::tables::do_list_configured_tables(state.inner())
}

/// The takeaways left open (nothing is registered for them until they
/// are paid): the Tables screen lists them so each one ends in a sale
/// or is closed unpaid (FAQ Q205). Empty drafts are left out.
#[tauri::command]
pub fn list_open_takeaways(state: State<'_, Managed>) -> Result<Vec<TakeawayInfo>, String> {
    pos_engine::commands::tables::do_list_open_takeaways(state.inner())
}

/// Register what was ordered at a table since the last registration
/// (event P, "vente interrompue", POS description v2.2 §1.3): the
/// order's journal entries not yet transmitted, corrections included,
/// in chronological order. A no-op when nothing changed. The UI calls
/// it when the operator leaves a table. Once signed, the new lines go
/// to the bar / kitchen slip; the result says what became of it.
#[tauri::command]
pub async fn sign_table_order(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<SlipOutcome, String> {
    pos_engine::commands::tables::do_sign_table_order(state.inner(), order_id).await
}

/// Send a signPreBill (provisional bill P event).
#[tauri::command]
pub async fn sign_pre_bill(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<SlipOutcome, String> {
    pos_engine::commands::tables::do_sign_pre_bill(state.inner(), order_id).await
}

/// The table plan (the table numbers in use) and its changes.
#[tauri::command]
pub fn get_table_plan(state: State<'_, Managed>) -> Result<TablePlan, String> {
    pos_engine::commands::table_plan::do_get_table_plan(state.inner())
}

/// Set the table plan ("1-12, 20-25"); managers only.
#[tauri::command]
pub fn set_table_plan(state: State<'_, Managed>, plan: String) -> Result<TablePlan, String> {
    pos_engine::commands::table_plan::do_set_table_plan(state.inner(), &plan)
}
