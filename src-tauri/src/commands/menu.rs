use pos_core::menu::{Category, MenuItem};

use crate::state::AppState;

pub fn do_get_categories(state: &AppState, lang: Option<String>) -> Result<Vec<Category>, String> {
    let lang = lang.as_deref().unwrap_or("fr");
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::menu::get_categories(&db, lang).map_err(|e| e.to_string())
}

pub fn do_get_menu_items(
    state: &AppState,
    category_id: Option<i64>,
    lang: Option<String>,
) -> Result<Vec<MenuItem>, String> {
    let lang = lang.as_deref().unwrap_or("fr");
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::menu::get_menu_items(&db, category_id, lang).map_err(|e| e.to_string())
}
