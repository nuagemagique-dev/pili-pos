//! Record of the quick notes (engine module `quick_notes`): the chips of the kitchen note dialog.
//! Mirrors the engine type (PD56): same field name, same serde form, `deny_unknown_fields`.

/// `pos_engine::commands::quick_notes::QuickNotes`: the quick notes of this installation, in the
/// order the dialog shows them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct QuickNotes {
    /// The chips; an empty list is a choice (no chip), not "unset".
    pub notes: Vec<String>,
}
