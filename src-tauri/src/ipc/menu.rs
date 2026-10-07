use pos_core::menu::{Category, MenuItem};
use tauri::State;

use super::Managed;

#[tauri::command]
pub fn get_categories(
    state: State<'_, Managed>,
    lang: Option<String>,
) -> Result<Vec<Category>, String> {
    crate::commands::menu::do_get_categories(state.inner(), lang)
}

#[tauri::command]
pub fn get_menu_items(
    state: State<'_, Managed>,
    category_id: Option<i64>,
    lang: Option<String>,
) -> Result<Vec<MenuItem>, String> {
    crate::commands::menu::do_get_menu_items(state.inner(), category_id, lang)
}
