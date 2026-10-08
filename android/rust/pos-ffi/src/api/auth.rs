//! Facade of the authentication, users, integrations and remote support commands (engine modules
//! auth, integrations and support): one async method per Tauri command, each one converting its
//! arguments, making the wrapper's single engine call and converting the answer.
//!
//! Every engine function here is synchronous, so each one runs on the blocking pool (the PIN
//! checks and hashes are bcrypt). No PIN is logged by this file.

use pos_engine::commands::{auth, integrations, support};

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::auth::{CreateUserInput, Integrations, SupportRunning, SupportStatus, User};

#[uniffi::export]
impl PosEngine {
    /// Open the operator's session with INSZ and PIN (the PIN is optional for a user who has none).
    pub async fn login(&self, insz: String, pin: Option<String>) -> Result<User, PosError> {
        let user = self
            .blocking(move |st| auth::do_login(st, insz, pin))
            .await?;
        convert::out(&user)
    }

    /// First login of a user who has no PIN yet: the chosen PIN is stored and the session opened.
    pub async fn login_with_new_pin(
        &self,
        insz: String,
        new_pin: String,
    ) -> Result<User, PosError> {
        let user = self
            .blocking(move |st| auth::do_login_with_new_pin(st, insz, new_pin))
            .await?;
        convert::out(&user)
    }

    /// End the operator's session.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn logout(&self) -> Result<(), PosError> {
        self.blocking(|st| auth::do_logout(st)).await
    }

    /// The login tiles: the active users.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn list_users(&self) -> Result<Vec<User>, PosError> {
        let users = self.blocking(|st| auth::do_list_users(st)).await?;
        convert::out(&users)
    }

    /// Create a user (managers only).
    pub async fn create_user(&self, input: CreateUserInput) -> Result<User, PosError> {
        let input: auth::CreateUserInput = convert::inp(&input)?;
        let user = self
            .blocking(move |st| auth::do_create_user(st, input))
            .await?;
        convert::out(&user)
    }

    /// Set or change a user's 4-digit PIN.
    pub async fn set_pin(
        &self,
        insz: String,
        new_pin: String,
        operator_pin: Option<String>,
    ) -> Result<(), PosError> {
        self.blocking(move |st| auth::do_set_pin(st, insz, new_pin, operator_pin))
            .await
    }

    /// Deactivate a user (managers only, never themselves).
    pub async fn deactivate_user(&self, insz: String) -> Result<(), PosError> {
        self.blocking(move |st| auth::do_deactivate_user(st, insz))
            .await
    }

    /// The logged-in operator, if any.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn get_current_user(&self) -> Result<Option<User>, PosError> {
        let user = self.blocking(|st| auth::do_get_current_user(st)).await?;
        convert::out(&user)
    }

    /// The integrations switches and their log.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn get_integrations(&self) -> Result<Integrations, PosError> {
        let value = self
            .blocking(|st| integrations::do_get_integrations(st))
            .await?;
        convert::out(&value)
    }

    /// Switch the HubRise integration on or off (managers only).
    pub async fn set_hubrise_enabled(&self, enabled: bool) -> Result<Integrations, PosError> {
        let value = self
            .blocking(move |st| integrations::do_set_hubrise_enabled(st, enabled))
            .await?;
        convert::out(&value)
    }

    /// Whether remote support is installed and whether a session runs.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn remote_support_status(&self) -> Result<SupportStatus, PosError> {
        let status = self
            .blocking(|st| support::do_remote_support_status(st))
            .await?;
        convert::out(&status)
    }

    /// Whether a session runs, for the login screen.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn remote_support_running(&self) -> Result<SupportRunning, PosError> {
        let running = self
            .blocking(|st| support::do_remote_support_running(st))
            .await?;
        convert::out(&running)
    }

    /// Start a remote support session: the logged-in manager, confirmed by their PIN.
    pub async fn start_remote_support(
        &self,
        insz: String,
        pin: String,
    ) -> Result<SupportStatus, PosError> {
        let status = self
            .blocking(move |st| support::do_start_remote_support(st, insz, pin))
            .await?;
        convert::out(&status)
    }

    /// End the remote support session.
    #[allow(clippy::redundant_closure)] // check_command_parity.py wants the engine call spelled out
    pub async fn stop_remote_support(&self) -> Result<SupportStatus, PosError> {
        let status = self
            .blocking(|st| support::do_stop_remote_support(st))
            .await?;
        convert::out(&status)
    }
}
