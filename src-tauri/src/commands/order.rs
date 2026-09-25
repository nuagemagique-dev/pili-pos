use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
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

/// A product added to an order. Only the product and quantity come
/// from the frontend: price, VAT label, name and department are read
/// from the catalogue (`resolve_catalogue_line`), so the fiscal record
/// never depends on what the UI displayed. The one exception is the
/// unit price of a free-price product, entered by the operator.
#[derive(Deserialize)]
pub struct AddOrderLineInput {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub modifiers_json: Option<String>,
    /// Required for free-price products, refused for the others.
    #[serde(default)]
    pub unit_price_cents: Option<i64>,
}

#[tauri::command]
pub fn add_order_line(
    state: State<'_, AppState>,
    input: AddOrderLineInput,
) -> Result<OrderLine, String> {
    do_add_order_line(state.inner(), input)
}

pub fn do_add_order_line(state: &AppState, input: AddOrderLineInput) -> Result<OrderLine, String> {
    if input.quantity <= 0 {
        return Err("Quantity must be greater than zero".into());
    }
    // Product and department names are fiscal data: always in the
    // establishment's national language, whatever the operator's UI
    // language is.
    let lang = pos_fiscal::ticket::ReceiptLanguage::from_code(
        &state.config.lock().map_err(|e| e.to_string())?.language,
    )
    .code();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let order =
        pos_db::queries::orders::get_order(&db, input.order_id).map_err(|e| e.to_string())?;
    if order.status != OrderStatus::Open {
        return Err("Order is not open".into());
    }
    let product = pos_db::queries::menu::resolve_catalogue_line(
        &db,
        input.menu_item_id,
        order.order_type == OrderType::Takeaway,
        lang,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            format!("Product {} is not available", input.menu_item_id)
        }
        other => other.to_string(),
    })?;
    let unit_price_cents = match (product.open_price, input.unit_price_cents) {
        (true, Some(price)) if price > 0 => price,
        (true, _) => return Err("Enter a price greater than zero".into()),
        (false, None) => product.price_cents,
        (false, Some(_)) => {
            return Err(format!("Product {} has a fixed price", input.menu_item_id));
        }
    };
    pos_db::queries::orders::add_order_line(
        &db,
        pos_db::queries::orders::NewOrderLine {
            order_id: input.order_id,
            menu_item_id: input.menu_item_id,
            quantity: input.quantity,
            unit_price_cents,
            vat_code: product.vat_code,
            name: &product.name,
            modifiers_json: input.modifiers_json.as_deref(),
            department_id: &product.department_id,
            department_name: &product.department_name,
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
