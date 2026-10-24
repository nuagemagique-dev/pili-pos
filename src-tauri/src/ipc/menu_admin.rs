//! The on-screen menu editor (manager only; saves refused while a catalog manages the menu).

use pos_engine::commands::menu_admin::{
    MenuAdmin, MenuChange, MenuSaved, SaveCategoryInput, SaveModifierGroupInput, SaveModifierInput,
    SaveProductInput,
};
use tauri::State;

use super::Managed;

/// The whole menu with its names per language, hidden entries included.
#[tauri::command]
pub fn menu_admin_get(state: State<'_, Managed>) -> Result<MenuAdmin, String> {
    pos_engine::commands::menu_admin::do_menu_admin_get(state.inner())
}

/// Create (no id) or update a category.
#[tauri::command]
pub fn menu_save_category(
    state: State<'_, Managed>,
    input: SaveCategoryInput,
) -> Result<MenuSaved, String> {
    pos_engine::commands::menu_admin::do_menu_save_category(state.inner(), input)
}

/// Create (no id) or update a product.
#[tauri::command]
pub fn menu_save_product(
    state: State<'_, Managed>,
    input: SaveProductInput,
) -> Result<MenuSaved, String> {
    pos_engine::commands::menu_admin::do_menu_save_product(state.inner(), input)
}

/// Create (no id) or update an option group of a product.
#[tauri::command]
pub fn menu_save_modifier_group(
    state: State<'_, Managed>,
    input: SaveModifierGroupInput,
) -> Result<MenuSaved, String> {
    pos_engine::commands::menu_admin::do_menu_save_modifier_group(state.inner(), input)
}

/// Create (no id) or update an option of a group.
#[tauri::command]
pub fn menu_save_modifier(
    state: State<'_, Managed>,
    input: SaveModifierInput,
) -> Result<MenuSaved, String> {
    pos_engine::commands::menu_admin::do_menu_save_modifier(state.inner(), input)
}

/// The change log of the menu, newest first.
#[tauri::command]
pub fn list_menu_changes(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<Vec<MenuChange>, String> {
    pos_engine::commands::menu_admin::do_list_menu_changes(state.inner(), limit)
}
