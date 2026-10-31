use pos_core::menu::{Category, MenuItem};
use pos_engine::commands::sold_out::SoldOutItem;
use tauri::State;

use super::Managed;

#[tauri::command(async)]
pub fn get_categories(
    state: State<'_, Managed>,
    lang: Option<String>,
) -> Result<Vec<Category>, String> {
    pos_engine::commands::menu::do_get_categories(state.inner(), lang)
}

#[tauri::command(async)]
pub fn get_menu_items(
    state: State<'_, Managed>,
    category_id: Option<i64>,
    lang: Option<String>,
) -> Result<Vec<MenuItem>, String> {
    pos_engine::commands::menu::do_get_menu_items(state.inner(), category_id, lang)
}

/// The products sold out now ("Épuisé"). Cashier and manager.
#[tauri::command(async)]
pub fn list_sold_out(
    state: State<'_, Managed>,
    lang: Option<String>,
) -> Result<Vec<SoldOutItem>, String> {
    pos_engine::commands::sold_out::do_list_sold_out(state.inner(), lang)
}

/// Mark a product sold out until the end of the business day, or until re-enabled.
#[tauri::command(async)]
pub fn set_sold_out(
    state: State<'_, Managed>,
    menu_item_id: i64,
    until_end_of_day: bool,
    lang: Option<String>,
) -> Result<Vec<SoldOutItem>, String> {
    pos_engine::commands::sold_out::do_set_sold_out(
        state.inner(),
        menu_item_id,
        until_end_of_day,
        lang,
    )
}

/// Make a sold-out product available again.
#[tauri::command(async)]
pub fn clear_sold_out(
    state: State<'_, Managed>,
    menu_item_id: i64,
    lang: Option<String>,
) -> Result<Vec<SoldOutItem>, String> {
    pos_engine::commands::sold_out::do_clear_sold_out(state.inner(), menu_item_id, lang)
}
