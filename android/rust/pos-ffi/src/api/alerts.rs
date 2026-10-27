//! The owner alerts facade (engine module alerts): four cert commands as async methods of
//! [`PosEngine`].
//!
//! Each method mirrors its Tauri wrapper (`src-tauri/src/ipc/alerts.rs`): the same parameter names
//! and the single engine call. The settings and the log are managers only (the engine refuses the
//! others); the bot token never crosses this boundary.

use pos_engine::commands::alerts;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::alerts::{AlertLogRow, AlertTestResult, OwnerAlerts, OwnerAlertsInput};

#[uniffi::export]
impl PosEngine {
    /// The owner alert settings (Telegram messages to the owner's phone); managers only.
    #[expect(
        clippy::redundant_closure,
        reason = "check_command_parity.py wants the engine call spelled out"
    )]
    pub async fn get_owner_alerts(&self) -> Result<OwnerAlerts, PosError> {
        let current = self.blocking(|st| alerts::do_get_owner_alerts(st)).await?;
        convert::out(&current)
    }

    /// Store the owner alert settings; managers only, logged.
    pub async fn set_owner_alerts(&self, input: OwnerAlertsInput) -> Result<OwnerAlerts, PosError> {
        let input: alerts::OwnerAlertsInput = convert::inp(&input)?;
        let current = self
            .blocking(move |st| alerts::do_set_owner_alerts(st, input))
            .await?;
        convert::out(&current)
    }

    /// Send one test message to the owner's chat now; managers only.
    pub async fn send_test_alert(&self) -> Result<AlertTestResult, PosError> {
        let result: alerts::AlertTestResult = self
            .spawn(|engine| async move { alerts::do_send_test_alert(&engine).await })
            .await?;
        convert::out(&result)
    }

    /// The latest alert messages sent or dropped, newest first; managers only.
    pub async fn list_owner_alert_log(&self, limit: i64) -> Result<Vec<AlertLogRow>, PosError> {
        let rows = self
            .blocking(move |st| alerts::do_list_owner_alert_log(st, limit))
            .await?;
        convert::out(&rows)
    }
}
