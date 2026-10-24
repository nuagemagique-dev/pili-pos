//! Records of the auth facade (PD56): the operator and the users, the HubRise switch and the remote
//! support status.
//!
//! Owner: the auth domain (engine modules auth, integrations and support). Each type mirrors an
//! engine type: same field names, same serde attributes, `deny_unknown_fields` on records. No
//! shared type is needed, and every type these commands reach is in the T4 table, so there is no
//! extra type to list.
//!
//! `User` has no PIN hash: the engine type carries none either (`has_pin` says whether one is
//! stored), so no PIN, plain or hashed, ever crosses the FFI in an answer.

/// `pos_core::user::UserRole`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Cashier,
    Waiter,
}

/// `pos_core::user::User`: an operator of the till.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct User {
    pub insz: String,
    pub name: String,
    pub role: UserRole,
    pub active: bool,
    /// True when a PIN hash is stored: the login screen shows the numpad.
    pub has_pin: bool,
    /// A kitchen employee who never uses the till: no login, no PIN, a manager clocks them in
    /// and out.
    #[serde(default)]
    #[uniffi(default = false)]
    pub presence_only: bool,
}

/// `pos_engine::commands::auth::CreateUserInput`. `role` is the role name as text (`admin`,
/// `cashier`, `waiter`), like the engine reads it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateUserInput {
    pub insz: String,
    pub name: String,
    pub role: String,
    /// Required for role admin, ignored otherwise.
    #[serde(default)]
    pub initial_pin: Option<String>,
    /// The creating manager's own PIN.
    #[serde(default)]
    pub operator_pin: Option<String>,
    /// A kitchen employee who never uses the till (role `waiter`, no PIN).
    #[serde(default)]
    #[uniffi(default = false)]
    pub presence_only: bool,
}

/// `pos_db::queries::integrations::SwitchChange`: one change of the HubRise switch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct SwitchChange {
    pub enabled: bool,
    pub source: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

/// `pos_engine::commands::integrations::Integrations`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct Integrations {
    pub hubrise_enabled: bool,
    /// Online orders still to settle: the board stays reachable for them even when the
    /// integration is off.
    pub online_orders_open: i64,
    /// The latest changes first.
    pub hubrise_log: Vec<SwitchChange>,
}

/// `pos_engine::commands::support::SupportStatus`. Times are RFC 3339 text, UTC.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupportStatus {
    /// Remote support is installed on this terminal (never on Android, PD53).
    pub available: bool,
    pub running: bool,
    /// Name of the manager who started the running session.
    pub started_by: Option<String>,
    pub started_at: Option<String>,
}

/// `pos_engine::commands::support::SupportRunning`: whether a session runs, for the login screen.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SupportRunning {
    pub running: bool,
    pub started_at: Option<String>,
}
