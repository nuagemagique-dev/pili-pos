use pos_engine::commands::quick_notes::QuickNotes;
use tauri::State;

use super::Managed;

/// The quick notes (chips of the kitchen note dialog), for any logged-in operator.
#[tauri::command(async)]
pub fn get_quick_notes(state: State<'_, Managed>) -> Result<QuickNotes, String> {
    pos_engine::commands::quick_notes::do_get_quick_notes(state.inner())
}

/// Store the quick notes; managers only.
#[tauri::command(async)]
pub fn set_quick_notes(state: State<'_, Managed>, input: QuickNotes) -> Result<QuickNotes, String> {
    pos_engine::commands::quick_notes::do_set_quick_notes(state.inner(), input)
}
