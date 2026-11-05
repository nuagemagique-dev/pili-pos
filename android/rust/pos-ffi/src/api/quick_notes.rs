//! The quick notes facade (engine module `quick_notes`): two commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/quick_notes.rs`): the same parameter
//! names and the single engine call. Local data, never fiscal. Any logged-in operator reads the
//! list; the setting is for managers.

use pos_engine::commands::quick_notes;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::quick_notes::QuickNotes;

#[uniffi::export]
impl PosEngine {
    /// The quick notes (chips of the kitchen note dialog).
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn get_quick_notes(&self) -> Result<QuickNotes, PosError> {
        let notes = self
            .blocking(|st| quick_notes::do_get_quick_notes(st))
            .await?;
        convert::out(&notes)
    }

    /// Store the quick notes; managers only.
    pub async fn set_quick_notes(&self, input: QuickNotes) -> Result<QuickNotes, PosError> {
        let input: quick_notes::QuickNotes = convert::inp(&input)?;
        let stored = self
            .blocking(move |st| quick_notes::do_set_quick_notes(st, input))
            .await?;
        convert::out(&stored)
    }
}
