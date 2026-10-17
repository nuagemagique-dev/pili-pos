//! Facade of the staff badge commands (engine module `badges`): one method per Tauri command, same
//! parameter names, one engine call each.
//!
//! The synchronous engine functions run on the blocking pool (the manager's PIN is checked with
//! bcrypt); `badge_tap` signs, so it runs on the engine's runtime. No badge number is logged by
//! this file.

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::badges::{
    BadgeLink, BadgeOutcome, BadgeOverview, BadgeSettingsInput, BadgeSettingsView,
};

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[allow(clippy::redundant_closure)]
#[uniffi::export]
impl PosEngine {
    /// The badge reader setting. No session needed: the login screen arms its reader from it.
    pub async fn get_badge_settings(&self) -> Result<BadgeSettingsView, PosError> {
        let view = self
            .blocking(|state| pos_engine::commands::badges::do_get_badge_settings(state))
            .await?;
        convert::out(&view)
    }

    /// Change the badge reader setting (managers only, logged).
    pub async fn set_badge_settings(
        &self,
        input: BadgeSettingsInput,
    ) -> Result<BadgeSettingsView, PosError> {
        let input: pos_engine::commands::badges::BadgeSettingsInput = convert::inp(&input)?;
        let view = self
            .blocking(move |state| {
                pos_engine::commands::badges::do_set_badge_settings(state, input)
            })
            .await?;
        convert::out(&view)
    }

    /// The active badges and the latest log rows (managers only).
    pub async fn list_badges(&self) -> Result<BadgeOverview, PosError> {
        let overview = self
            .blocking(|state| pos_engine::commands::badges::do_list_badges(state))
            .await?;
        convert::out(&overview)
    }

    /// Link a badge to an employee: a manager, confirmed by their own PIN.
    pub async fn link_badge(
        &self,
        employee_insz: String,
        token: String,
        operator_pin: Option<String>,
    ) -> Result<BadgeLink, PosError> {
        let link = self
            .blocking(move |state| {
                pos_engine::commands::badges::do_link_badge(
                    state,
                    employee_insz,
                    token,
                    operator_pin,
                )
            })
            .await?;
        convert::out(&link)
    }

    /// Remove an employee's badge (managers only).
    pub async fn revoke_badge(&self, employee_insz: String) -> Result<(), PosError> {
        self.blocking(move |state| {
            pos_engine::commands::badges::do_revoke_badge(state, employee_insz)
        })
        .await
    }

    /// A badge scan: the engine decides (presence, login or a PIN request). `end_shift` is true
    /// while the login screen is in its end-of-shift mode.
    pub async fn badge_tap(
        &self,
        token: String,
        end_shift: bool,
    ) -> Result<BadgeOutcome, PosError> {
        let outcome = self
            .spawn(move |engine| async move {
                pos_engine::commands::badges::do_badge_tap(&engine, token, end_shift).await
            })
            .await?;
        convert::out(&outcome)
    }
}
