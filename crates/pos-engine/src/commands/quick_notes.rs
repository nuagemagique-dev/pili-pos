//! The quick notes: the chips of the kitchen note dialog (UX audit C11).
//!
//! A list kept per installation in the owner settings store (key
//! `quick_notes`, so every change is logged in the append-only
//! `owner_settings_log`): "Sans wasabi", "Sauce à part"... A tap on a chip
//! appends its text to the note of a cart line. Any logged-in operator reads
//! the list, only a manager changes it.
//!
//! Local data, never fiscal: a quick note is just text a waiter may pick
//! instead of typing. The note that results goes through `do_set_line_note`,
//! which already keeps it out of every FDM message, product name and ticket
//! (`tests/kitchen_notes.rs`); the list itself is stored nowhere else than in
//! `owner_settings` and its log (`tests/quick_notes.rs` proves both).

use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::order::clean_note;
use crate::commands::owner_settings;
use crate::state::AppState;

/// The key of the quick notes in the owner settings store.
pub const KEY_QUICK_NOTES: &str = "quick_notes";

/// The most chips the list holds (the dialog shows them wrapped above the keypad).
pub const MAX_QUICK_NOTES: usize = 24;

/// The list of a new installation, in the order the dialog shows it.
pub const DEFAULT_QUICK_NOTES: [&str; 9] = [
    "Sans wasabi",
    "Sans sésame",
    "Sans sauce",
    "Sauce à part",
    "Bien cuit",
    "Épicé",
    "Pas épicé",
    "Sans oignon",
    "À emporter",
];

/// The quick notes of this installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuickNotes {
    /// The chips, in display order. An empty list is a choice (no chips), not "unset".
    pub notes: Vec<String>,
}

impl Default for QuickNotes {
    fn default() -> Self {
        Self {
            notes: DEFAULT_QUICK_NOTES
                .iter()
                .map(|n| (*n).to_string())
                .collect(),
        }
    }
}

/// The list, for any logged-in operator (the note dialog reads it).
pub fn do_get_quick_notes(state: &AppState) -> Result<QuickNotes, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    owner_settings::get::<QuickNotes>(&db, KEY_QUICK_NOTES)
}

/// Store the list (managers only).
///
/// Each chip is a kitchen note: trimmed, at most 60 characters, only what
/// the slip printer prints (`note_too_long`, `note_unprintable`), so a tap
/// never produces a note the engine would refuse. A blank chip is dropped, a
/// repeated one (same text ignoring case) keeps its first place, and the
/// list holds at most [`MAX_QUICK_NOTES`] chips (`quick_notes_too_many`).
/// Returns the list as stored.
pub fn do_set_quick_notes(state: &AppState, input: QuickNotes) -> Result<QuickNotes, String> {
    let operator = access::operator(state, Access::Admin)?;
    let mut notes: Vec<String> = Vec::new();
    for raw in input.notes {
        let Some(note) = clean_note(Some(raw))? else {
            continue;
        };
        let key = note.to_lowercase();
        if !notes.iter().any(|n| n.to_lowercase() == key) {
            notes.push(note);
        }
    }
    if notes.len() > MAX_QUICK_NOTES {
        return Err(errors::coded(
            errors::QUICK_NOTES_TOO_MANY,
            &[("max", &MAX_QUICK_NOTES)],
        ));
    }
    let stored = QuickNotes { notes };
    let db = state.db.lock().map_err(errors::internal)?;
    owner_settings::set(&db, KEY_QUICK_NOTES, &stored, Some(&operator.insz))?;
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_nine_usual_notes() {
        let defaults = QuickNotes::default();
        assert_eq!(defaults.notes.len(), 9);
        assert_eq!(defaults.notes[0], "Sans wasabi");
        assert_eq!(defaults.notes[8], "À emporter");
    }

    #[test]
    fn every_default_is_a_valid_kitchen_note() {
        for note in DEFAULT_QUICK_NOTES {
            assert_eq!(
                clean_note(Some(note.to_string())).unwrap().as_deref(),
                Some(note)
            );
        }
        assert!(DEFAULT_QUICK_NOTES.len() <= MAX_QUICK_NOTES);
    }
}
