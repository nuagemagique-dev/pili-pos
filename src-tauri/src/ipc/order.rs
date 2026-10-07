use pos_core::order::{Order, OrderLine};
use tauri::State;

use pos_engine::commands::order::{AddOrderLineInput, CreateOrderInput};

use super::Managed;

#[tauri::command]
pub fn create_order(state: State<'_, Managed>, input: CreateOrderInput) -> Result<Order, String> {
    pos_engine::commands::order::do_create_order(state.inner(), input)
}

#[tauri::command]
pub fn add_order_line(
    state: State<'_, Managed>,
    input: AddOrderLineInput,
) -> Result<OrderLine, String> {
    pos_engine::commands::order::do_add_order_line(state.inner(), input)
}

#[tauri::command]
pub fn remove_order_line(state: State<'_, Managed>, line_id: i64) -> Result<(), String> {
    pos_engine::commands::order::do_remove_order_line(state.inner(), line_id)
}

#[tauri::command]
pub fn update_order_line_quantity(
    state: State<'_, Managed>,
    line_id: i64,
    quantity: i32,
) -> Result<(), String> {
    pos_engine::commands::order::do_update_order_line_quantity(state.inner(), line_id, quantity)
}

#[tauri::command]
pub fn get_order(state: State<'_, Managed>, order_id: i64) -> Result<Order, String> {
    pos_engine::commands::order::do_get_order(state.inner(), order_id)
}

/// Return every line of an existing order, ordered by id. Used when
/// the operator re-opens a previously-started dine-in order (by
/// tapping the table again after having switched away) so the
/// in-memory cart is rehydrated from SQLite.
#[tauri::command]
pub fn list_order_lines(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<Vec<OrderLine>, String> {
    pos_engine::commands::order::do_list_order_lines(state.inner(), order_id)
}

/// The order's lines as the cart shows them: names in French and
/// Chinese, and the quantity not yet sent to the bar / kitchen.
#[tauri::command]
pub fn list_cart_lines(
    state: State<'_, Managed>,
    order_id: i64,
) -> Result<Vec<pos_db::queries::orders::CartLine>, String> {
    pos_engine::commands::order::do_list_cart_lines(state.inner(), order_id)
}

#[tauri::command]
pub fn list_open_orders(state: State<'_, Managed>) -> Result<Vec<Order>, String> {
    pos_engine::commands::order::do_list_open_orders(state.inner())
}
