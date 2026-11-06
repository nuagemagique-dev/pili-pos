use pos_core::menu::MenuItem;
use pos_engine::commands::favourites::FavouritesSettings;
use tauri::State;

use super::Managed;

/// The favourites setting (mode and manager's selection), for any logged-in operator.
#[tauri::command(async)]
pub fn get_favourites_settings(state: State<'_, Managed>) -> Result<FavouritesSettings, String> {
    pos_engine::commands::favourites::do_get_favourites_settings(state.inner())
}

/// Store the favourites setting; managers only.
#[tauri::command(async)]
pub fn set_favourites_settings(
    state: State<'_, Managed>,
    input: FavouritesSettings,
) -> Result<FavouritesSettings, String> {
    pos_engine::commands::favourites::do_set_favourites_settings(state.inner(), input)
}

/// Star or unstar a product (the menu editor's toggle); managers only.
#[tauri::command(async)]
pub fn set_favourite(
    state: State<'_, Managed>,
    menu_item_id: i64,
    favourite: bool,
) -> Result<FavouritesSettings, String> {
    pos_engine::commands::favourites::do_set_favourite(state.inner(), menu_item_id, favourite)
}

/// The products of the "Favoris" tab, names in `lang`; empty hides the tab.
#[tauri::command(async)]
pub fn list_favourites(
    state: State<'_, Managed>,
    lang: Option<String>,
) -> Result<Vec<MenuItem>, String> {
    pos_engine::commands::favourites::do_list_favourites(state.inner(), lang)
}
