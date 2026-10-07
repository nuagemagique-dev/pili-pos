use pos_core::order::OrderLine;
use pos_db::queries::menu::ModifierGroup;
use tauri::State;

use crate::commands::modifiers::AddWithModifiersInput;

use super::Managed;

#[tauri::command]
pub fn get_item_modifiers(
    state: State<'_, Managed>,
    menu_item_id: i64,
    lang: Option<String>,
) -> Result<Vec<ModifierGroup>, String> {
    crate::commands::modifiers::do_get_item_modifiers_localized(
        state.inner(),
        menu_item_id,
        lang.as_deref(),
    )
}

#[tauri::command]
pub fn add_order_line_with_modifiers(
    state: State<'_, Managed>,
    input: AddWithModifiersInput,
) -> Result<OrderLine, String> {
    crate::commands::modifiers::do_add_order_line_with_modifiers(state.inner(), &input)
}
