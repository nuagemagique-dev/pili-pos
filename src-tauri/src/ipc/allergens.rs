use pos_engine::commands::allergens::{AllergenEntry, AllergenSheetView, SetAllergensInput};
use pos_engine::commands::bar_slip::SlipOutcome;
use tauri::State;

use super::Managed;

/// The 14 EU allergens in Annex II order, for the legend.
#[tauri::command(async)]
pub fn list_allergens(state: State<'_, Managed>) -> Result<Vec<AllergenEntry>, String> {
    pos_engine::commands::allergens::do_list_allergens(state.inner())
}

/// The allergen sheet of one product, or of the whole active menu.
#[tauri::command(async)]
pub fn get_allergen_sheet(
    state: State<'_, Managed>,
    menu_item_id: Option<i64>,
    lang: Option<String>,
) -> Result<AllergenSheetView, String> {
    pos_engine::commands::allergens::do_get_allergen_sheet(state.inner(), menu_item_id, lang)
}

/// Print the allergen sheet (one product or the whole menu).
#[tauri::command]
pub async fn print_allergen_sheet(
    state: State<'_, Managed>,
    menu_item_id: Option<i64>,
    lang: Option<String>,
) -> Result<SlipOutcome, String> {
    pos_engine::commands::allergens::do_print_allergen_sheet(state.inner(), menu_item_id, lang)
        .await
}

/// Set the allergens of a product or an option by hand; managers only,
/// refused while a catalog source manages the menu.
#[tauri::command(async)]
pub fn set_allergens(state: State<'_, Managed>, input: SetAllergensInput) -> Result<bool, String> {
    pos_engine::commands::allergens::do_set_allergens(state.inner(), input)
}
