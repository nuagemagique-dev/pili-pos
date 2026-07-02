use pos_core::order::{Order, OrderLine, OrderType};
use pos_core::vat::VatCode;
use serde::Deserialize;
use tauri::State;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateOrderInput {
    pub order_type: OrderType,
    pub table_number: Option<i32>,
    pub user_insz: String,
}

#[tauri::command]
pub fn create_order(state: State<'_, AppState>, input: CreateOrderInput) -> Result<Order, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::create_order(
        &db,
        input.order_type,
        input.table_number,
        &input.user_insz,
    )
    .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct AddOrderLineInput {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub name: String,
    pub modifiers_json: Option<String>,
}

#[tauri::command]
pub fn add_order_line(
    state: State<'_, AppState>,
    input: AddOrderLineInput,
) -> Result<OrderLine, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::add_order_line(
        &db,
        pos_db::queries::orders::NewOrderLine {
            order_id: input.order_id,
            menu_item_id: input.menu_item_id,
            quantity: input.quantity,
            unit_price_cents: input.unit_price_cents,
            vat_code: input.vat_code,
            name: &input.name,
            modifiers_json: input.modifiers_json.as_deref(),
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_order_line(state: State<'_, AppState>, line_id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::remove_order_line(&db, line_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_order_line_quantity(
    state: State<'_, AppState>,
    line_id: i64,
    quantity: i32,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::update_order_line_quantity(&db, line_id, quantity)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_order(state: State<'_, AppState>, order_id: i64) -> Result<Order, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())
}

/// Return every line of an existing order, ordered by id. Used when
/// the operator re-opens a previously-started dine-in order (by
/// tapping the table again after having switched away) so the
/// in-memory cart is rehydrated from SQLite.
#[tauri::command]
pub fn list_order_lines(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<Vec<OrderLine>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_open_orders(state: State<'_, AppState>) -> Result<Vec<Order>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::list_open_orders(&db).map_err(|e| e.to_string())
}
