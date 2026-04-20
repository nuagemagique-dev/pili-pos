use pos_core::menu::{Category, MenuItem};
use tauri::State;

use crate::state::AppState;

#[tauri::command]
pub fn get_categories(state: State<'_, AppState>, lang: Option<String>) -> Result<Vec<Category>, String> {
    let lang = lang.as_deref().unwrap_or("fr");
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::menu::get_categories(&db, lang).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_menu_items(
    state: State<'_, AppState>,
    category_id: Option<i64>,
    lang: Option<String>,
) -> Result<Vec<MenuItem>, String> {
    let lang = lang.as_deref().unwrap_or("fr");
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::menu::get_menu_items(&db, category_id, lang).map_err(|e| e.to_string())
}
