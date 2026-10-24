use tauri::State;

use pos_engine::commands::discount::{DiscountInput, DiscountResult};

use super::Managed;

/// Give a discount on an open till order: an order discount (percent,
/// amount or "Offert") or one free line. A manager confirms with the
/// PIN; a table that was already registered gets its P order signed at
/// once. The result says what became of the kitchen slip and of that P.
#[tauri::command]
pub async fn apply_discount(
    state: State<'_, Managed>,
    order_id: i64,
    input: DiscountInput,
    insz: String,
    pin: String,
) -> Result<DiscountResult, String> {
    pos_engine::commands::discount::do_apply_discount(state.inner(), order_id, input, insz, pin)
        .await
}

/// Remove the order discount (`line_id` absent), or the free line
/// `line_id`. A manager confirms with the PIN.
#[tauri::command]
pub async fn remove_discount(
    state: State<'_, Managed>,
    order_id: i64,
    line_id: Option<i64>,
    insz: String,
    pin: String,
) -> Result<DiscountResult, String> {
    pos_engine::commands::discount::do_remove_discount(state.inner(), order_id, line_id, insz, pin)
        .await
}
