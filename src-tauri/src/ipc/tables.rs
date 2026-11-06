use tauri::State;

use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::table_plan::TablePlan;
use pos_engine::commands::table_transfer::TableTransferResult;
use pos_engine::commands::tables::{TableInfo, TakeawayInfo};

use super::Managed;

/// List tables with their current order status.
///
/// A table is only reported as **occupied** when its open order
/// contains at least one line. Empty open orders (typical after a
/// tap-and-walk-away) are treated as free so the TablesView doesn't
/// surface ghost "0.00 €" cards that add cognitive load for zero
/// informational value (UX Guide §2.1).
#[tauri::command(async)]
pub fn list_tables(state: State<'_, Managed>) -> Result<Vec<TableInfo>, String> {
    pos_engine::commands::tables::do_list_configured_tables(state.inner())
}

/// The takeaways left open (nothing is registered for them until they
/// are paid): the Tables screen lists them so each one ends in a sale
/// or is closed unpaid (FAQ Q205). Empty drafts are left out.
#[tauri::command(async)]
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

/// Move the open order of a table to a free table (event P, cost
/// center change). Entries not yet registered are sent first; the
/// result says what became of the kitchen slip and the transfer notice.
#[tauri::command]
pub async fn transfer_table(
    state: State<'_, Managed>,
    order_id: i64,
    to_table: i32,
) -> Result<TableTransferResult, String> {
    pos_engine::commands::table_transfer::do_transfer_table(state.inner(), order_id, to_table).await
}

/// The table plan (the table numbers in use) and its changes.
#[tauri::command(async)]
pub fn get_table_plan(state: State<'_, Managed>) -> Result<TablePlan, String> {
    pos_engine::commands::table_plan::do_get_table_plan(state.inner())
}

/// Set the table plan ("1-12, 20-25", "A 1-20, B 21-40") and the names of its
/// zones ("A · Salle, B · Terrasse", empty for none); managers only.
#[tauri::command(async)]
pub fn set_table_plan(
    state: State<'_, Managed>,
    plan: String,
    zone_names: String,
) -> Result<TablePlan, String> {
    pos_engine::commands::table_plan::do_set_table_plan(state.inner(), &plan, &zone_names)
}

/// The covers (guests) set on an order, for any logged-in operator.
#[tauri::command(async)]
pub fn get_order_covers(state: State<'_, Managed>, order_id: i64) -> Result<Option<i32>, String> {
    pos_engine::commands::covers::do_get_order_covers(state.inner(), order_id)
}

/// Set (1 to 30) or clear the covers of an open table order.
#[tauri::command(async)]
pub fn set_order_covers(
    state: State<'_, Managed>,
    order_id: i64,
    covers: Option<i32>,
) -> Result<Option<i32>, String> {
    pos_engine::commands::covers::do_set_order_covers(state.inner(), order_id, covers)
}
