//! Facade of the checkout scale commands (engine module `scale`): one method per Tauri command, same
//! parameter names, one engine call each. A weighing talks to the scale, so it runs on the engine's
//! runtime and stays pending until the load is stable, or until `scale_cancel_weighing`.

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::orders::OrderLine;
use crate::records::scale::{ScaleStatus, ScaleWeighInput};

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[allow(clippy::redundant_closure)]
#[uniffi::export]
impl PosEngine {
    /// How the scale is doing, for the header light.
    pub async fn get_scale_status(&self) -> Result<ScaleStatus, PosError> {
        let status = self
            .spawn(|engine| async move {
                pos_engine::commands::scale::do_get_scale_status(&engine).await
            })
            .await?;
        convert::out(&status)
    }

    /// Weigh a product sold by weight and add the line from the scale's answer.
    pub async fn scale_start_weighing(
        &self,
        input: ScaleWeighInput,
    ) -> Result<OrderLine, PosError> {
        let input: pos_engine::commands::scale::ScaleWeighInput = convert::inp(&input)?;
        let line = self
            .spawn(move |engine| async move {
                pos_engine::commands::scale::do_scale_start_weighing(&engine, input).await
            })
            .await?;
        convert::out(&line)
    }

    /// Stop the weighing in progress: it ends with `scale_cancelled`.
    pub async fn scale_cancel_weighing(&self) -> Result<(), PosError> {
        self.blocking(|state| pos_engine::commands::scale::do_scale_cancel_weighing(state))
            .await
    }
}
