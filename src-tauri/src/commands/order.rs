use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use rusqlite::OptionalExtension;
use serde::Deserialize;
use tauri::State;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateOrderInput {
    pub order_type: OrderType,
    pub table_number: Option<i32>,
    pub user_insz: String,
}

#[tauri::command]
pub fn create_order(state: State<'_, AppState>, input: CreateOrderInput) -> Result<Order, String> {
    do_create_order(state.inner(), input)
}

/// Start an order. A dine-in order needs a table and a table has at
/// most one open order: asking again for a busy table returns its
/// order (with its lines), so an unpaid order can never hide behind a
/// second one on the tables screen.
pub fn do_create_order(state: &AppState, input: CreateOrderInput) -> Result<Order, String> {
    if !pos_core::user::validate_insz(&input.user_insz) {
        return Err(errors::OPERATOR_INVALID.into());
    }
    access::operator_named(state, &input.user_insz, Access::Staff)?;
    match (input.order_type, input.table_number) {
        (OrderType::DineIn, Some(t)) if t > 0 => {}
        (OrderType::DineIn, _) => return Err(errors::DINE_IN_NEEDS_TABLE.into()),
        (OrderType::Takeaway, None) => {}
        (OrderType::Takeaway, Some(_)) => {
            return Err(errors::TAKEAWAY_HAS_NO_TABLE.into());
        }
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(table) = input.table_number {
        let open: Option<i64> = db
            .query_row(
                "SELECT id FROM orders
                 WHERE table_number = ?1 AND status = 'open' AND order_type = 'dine_in'
                 ORDER BY id LIMIT 1",
                rusqlite::params![table],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if let Some(id) = open {
            return pos_db::queries::orders::get_order(&db, id).map_err(|e| e.to_string());
        }
    }
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
    access::operator(state, Access::Staff)?;
    if input.quantity <= 0 {
        return Err(errors::QUANTITY_POSITIVE.into());
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
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    unchanged_while_pending(&db, input.order_id)?;
    let product = pos_db::queries::menu::resolve_catalogue_line(
        &db,
        input.menu_item_id,
        order.order_type == OrderType::Takeaway,
        lang,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => errors::coded(
            errors::PRODUCT_UNAVAILABLE,
            &[("product", &input.menu_item_id)],
        ),
        other => errors::internal(other),
    })?;
    let unit_price_cents = match (product.open_price, input.unit_price_cents) {
        (true, Some(price)) if price > 0 => price,
        (true, _) => return Err(errors::PRICE_POSITIVE.into()),
        (false, None) => product.price_cents,
        (false, Some(_)) => {
            return Err(errors::coded(
                errors::FIXED_PRICE,
                &[("product", &input.menu_item_id)],
            ));
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
    do_remove_order_line(state.inner(), line_id)
}

/// Remove a line from an open order (journalled as a correction).
pub fn do_remove_order_line(state: &AppState, line_id: i64) -> Result<(), String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    unchanged_while_pending(&db, order_of_line(&db, line_id)?)?;
    pos_db::queries::orders::remove_order_line(&db, line_id).map_err(line_error)
}

#[tauri::command]
pub fn update_order_line_quantity(
    state: State<'_, AppState>,
    line_id: i64,
    quantity: i32,
) -> Result<(), String> {
    do_update_order_line_quantity(state.inner(), line_id, quantity)
}

/// Set a line's quantity on an open order. Zero or less is refused:
/// removing a line is [`do_remove_order_line`], and a sale line is
/// never negative (refunds are their own ticket).
pub fn do_update_order_line_quantity(
    state: &AppState,
    line_id: i64,
    quantity: i32,
) -> Result<(), String> {
    access::operator(state, Access::Staff)?;
    if quantity <= 0 {
        return Err(errors::QUANTITY_POSITIVE.into());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    unchanged_while_pending(&db, order_of_line(&db, line_id)?)?;
    pos_db::queries::orders::update_order_line_quantity(&db, line_id, quantity).map_err(line_error)
}

/// An order whose event waits for the FDM's answer keeps what was sent
/// (`commands::signing`).
fn unchanged_while_pending(db: &rusqlite::Connection, order_id: i64) -> Result<(), String> {
    if pos_db::queries::pending_signs::order_has_open(db, order_id).map_err(errors::internal)? {
        return Err(errors::ORDER_SIGN_PENDING.into());
    }
    Ok(())
}

fn order_of_line(db: &rusqlite::Connection, line_id: i64) -> Result<i64, String> {
    db.query_row(
        "SELECT order_id FROM order_lines WHERE id = ?1",
        [line_id],
        |r| r.get(0),
    )
    .map_err(line_error)
}

fn line_error(e: rusqlite::Error) -> String {
    match e {
        rusqlite::Error::QueryReturnedNoRows => errors::LINE_NOT_FOUND.into(),
        other => errors::internal(other),
    }
}

#[tauri::command]
pub fn get_order(state: State<'_, AppState>, order_id: i64) -> Result<Order, String> {
    do_get_order(state.inner(), order_id)
}

pub fn do_get_order(state: &AppState, order_id: i64) -> Result<Order, String> {
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
    do_list_order_lines(state.inner(), order_id)
}

pub fn do_list_order_lines(state: &AppState, order_id: i64) -> Result<Vec<OrderLine>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::get_order_lines(&db, order_id).map_err(|e| e.to_string())
}

/// The order's lines as the cart shows them: names in French and
/// Chinese, and the quantity not yet sent to the bar / kitchen.
#[tauri::command]
pub fn list_cart_lines(
    state: State<'_, AppState>,
    order_id: i64,
) -> Result<Vec<pos_db::queries::orders::CartLine>, String> {
    do_list_cart_lines(state.inner(), order_id)
}

pub fn do_list_cart_lines(
    state: &AppState,
    order_id: i64,
) -> Result<Vec<pos_db::queries::orders::CartLine>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::get_cart_lines(&db, order_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_open_orders(state: State<'_, AppState>) -> Result<Vec<Order>, String> {
    do_list_open_orders(state.inner())
}

pub fn do_list_open_orders(state: &AppState) -> Result<Vec<Order>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::orders::list_open_orders(&db).map_err(|e| e.to_string())
}
