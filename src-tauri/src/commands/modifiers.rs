//! Products with options (feature `hubrise`): the order screen offers
//! the option groups a mirrored catalog gives a product, and adds the
//! line with the options picked. The backend prices the options and
//! names the line ("Maki (Soja sucrée)"), as an online order's line is
//! named; the UI only says which options were picked.

use pos_core::order::{OrderLine, OrderStatus, OrderType};
use pos_db::queries::menu::{ModifierGroup, get_modifier_groups};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

#[tauri::command]
pub fn get_item_modifiers(
    state: State<'_, AppState>,
    menu_item_id: i64,
    lang: Option<String>,
) -> Result<Vec<ModifierGroup>, String> {
    do_get_item_modifiers(state.inner(), menu_item_id, lang.as_deref().unwrap_or("fr"))
}

pub fn do_get_item_modifiers(
    state: &AppState,
    menu_item_id: i64,
    lang: &str,
) -> Result<Vec<ModifierGroup>, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    get_modifier_groups(&db, menu_item_id, lang).map_err(errors::internal)
}

#[derive(Debug, Deserialize)]
pub struct AddWithModifiersInput {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub modifier_ids: Vec<i64>,
}

/// An option as the line records it (`modifiers_json`).
#[derive(Debug, Serialize)]
struct Picked<'a> {
    id: i64,
    name: &'a str,
    price_cents: i64,
}

#[tauri::command]
pub fn add_order_line_with_modifiers(
    state: State<'_, AppState>,
    input: AddWithModifiersInput,
) -> Result<OrderLine, String> {
    do_add_order_line_with_modifiers(state.inner(), &input)
}

/// Add a product with its options: each group's choice count within
/// its minimum and maximum, every option one of the product's active
/// ones. The unit price is the product's plus its options'.
pub fn do_add_order_line_with_modifiers(
    state: &AppState,
    input: &AddWithModifiersInput,
) -> Result<OrderLine, String> {
    access::operator(state, Access::Staff)?;
    if input.quantity <= 0 {
        return Err(errors::QUANTITY_POSITIVE.into());
    }
    // Fiscal names are in the establishment's language (see
    // `do_add_order_line`).
    let lang = pos_fiscal::ticket::ReceiptLanguage::from_code(
        &state.config.lock().map_err(errors::internal)?.language,
    )
    .code();
    let db = state.db.lock().map_err(errors::internal)?;
    let order =
        pos_db::queries::orders::get_order(&db, input.order_id).map_err(errors::internal)?;
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
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
    if product.open_price {
        return Err(errors::hubrise::MODIFIER_OPEN_PRICE.into());
    }
    let groups = get_modifier_groups(&db, input.menu_item_id, lang).map_err(errors::internal)?;
    let picked = pick(&groups, &input.modifier_ids)?;

    let extra: i64 = picked.iter().map(|p| p.price_cents).sum();
    let name = if picked.is_empty() {
        product.name.clone()
    } else {
        let names: Vec<&str> = picked.iter().map(|p| p.name).collect();
        format!("{} ({})", product.name, names.join(", "))
    };
    let modifiers_json = (!picked.is_empty())
        .then(|| serde_json::to_string(&picked))
        .transpose()
        .map_err(errors::internal)?;
    pos_db::queries::orders::add_order_line(
        &db,
        pos_db::queries::orders::NewOrderLine {
            order_id: input.order_id,
            menu_item_id: input.menu_item_id,
            quantity: input.quantity,
            unit_price_cents: product.price_cents + extra,
            vat_code: product.vat_code,
            name: &name,
            modifiers_json: modifiers_json.as_deref(),
            department_id: &product.department_id,
            department_name: &product.department_name,
        },
    )
    .map_err(|e| e.to_string())
}

/// The options `ids` names, in the menu's order, checked against the
/// product's groups.
fn pick<'a>(groups: &'a [ModifierGroup], ids: &[i64]) -> Result<Vec<Picked<'a>>, String> {
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        let known = groups.iter().any(|g| g.options.iter().any(|o| o.id == *id));
        if !known || !seen.insert(*id) {
            return Err(errors::coded(
                errors::hubrise::MODIFIER_UNKNOWN,
                &[("modifier", id)],
            ));
        }
    }
    let mut picked = Vec::new();
    for group in groups {
        let chosen: Vec<_> = group
            .options
            .iter()
            .filter(|o| seen.contains(&o.id))
            .collect();
        let count = chosen.len() as i64;
        if count < group.min || group.max.is_some_and(|max| count > max) {
            let max = group.max.map_or_else(|| "-".to_string(), |m| m.to_string());
            return Err(errors::coded(
                errors::hubrise::MODIFIER_COUNT,
                &[("group", &group.name), ("min", &group.min), ("max", &max)],
            ));
        }
        picked.extend(chosen.into_iter().map(|o| Picked {
            id: o.id,
            name: &o.name,
            price_cents: o.price_cents,
        }));
    }
    Ok(picked)
}
