//! Facade of the nightly export commands (engine module `export`): one method per Tauri command,
//! same parameter names, one engine call each. Both are administrator-only in the engine.

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::export::ExportStatus;

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[uniffi::export]
impl PosEngine {
    /// The state of the nightly export (administrators only).
    pub async fn export_status(&self) -> Result<ExportStatus, PosError> {
        let status = self
            .spawn(move |engine| async move {
                pos_engine::commands::export::do_export_status(&engine).await
            })
            .await?;
        convert::out(&status)
    }

    /// Make the pending exports due at once and wake the export loop; returns the status without
    /// waiting for the pass (administrators only).
    pub async fn export_run_now(&self) -> Result<ExportStatus, PosError> {
        let status = self
            .spawn(move |engine| async move {
                pos_engine::commands::export::do_export_run_now(&engine).await
            })
            .await?;
        convert::out(&status)
    }
}
