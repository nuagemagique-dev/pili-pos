//! Records of the badges facade (PD56): the staff badge reader setting, the manager's overview of
//! the linked badges, and the outcome of a scan.
//!
//! Owner: the badges domain (engine module `badges`). Each type mirrors an engine type: same field
//! names, same serde attributes, `deny_unknown_fields` on records. The wedge timing values are
//! `i64` (hygiene H6: no unsigned integers). No record carries a badge number or its hash: the
//! engine never returns one.
//!
//! `BadgeOutcome.user` reuses [`super::auth::User`].

use super::auth::User;

/// `pos_engine::commands::badges::BadgeSettingsView`: the reader setting. `reader` is `off`,
/// `nfc` or `usb`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeSettingsView {
    pub reader: String,
    pub login_enabled: bool,
    pub wedge_max_gap_ms: i64,
    pub wedge_min_len: i64,
    pub wedge_max_len: i64,
    /// True after the badge key changed, until a badge is linked again.
    pub relink_needed: bool,
}

/// `pos_engine::commands::badges::BadgeSettingsInput`: a new setting, from a manager.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeSettingsInput {
    pub reader: String,
    pub login_enabled: bool,
    pub wedge_max_gap_ms: i64,
    pub wedge_min_len: i64,
    pub wedge_max_len: i64,
}

/// `pos_db::queries::badges::ActiveBadge`: an active badge in the manager's list.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveBadge {
    pub insz: String,
    pub name: String,
    /// UTC, `YYYY-MM-DD HH:MM:SS`.
    pub linked_at: String,
    pub linked_by: String,
    pub linked_by_name: Option<String>,
}

/// `pos_db::queries::badges::LogEntry`: one row of the badge audit log. `action` is `link`,
/// `revoke` or `settings`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeLogEntry {
    pub action: String,
    pub insz: Option<String>,
    pub name: Option<String>,
    pub by: Option<String>,
    pub by_name: Option<String>,
    /// UTC, `YYYY-MM-DD HH:MM:SS`.
    pub at: String,
    /// Revoke: the reason; settings: the new values as JSON text.
    pub detail: Option<String>,
}

/// `pos_engine::commands::badges::BadgeOverview`: the active badges and the latest log rows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeOverview {
    pub badges: Vec<ActiveBadge>,
    pub log: Vec<BadgeLogEntry>,
}

/// `pos_engine::commands::badges::BadgeLink`: a badge just linked.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeLink {
    pub insz: String,
    pub name: String,
    pub linked_at: String,
}

/// `pos_engine::commands::badges::BadgeOutcomeKind`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, uniffi::Enum,
)]
#[serde(rename_all = "snake_case")]
pub enum BadgeOutcomeKind {
    /// A repeat of a scan just handled: nothing to show.
    Ignored,
    ClockIn,
    ClockOut,
    /// The FDM did not register the presence (`error` says why).
    PresenceFailed,
    /// A session was opened (`user`).
    LoggedIn,
    /// A manager's badge: the PIN pad opens for them.
    PinRequired,
}

/// `pos_engine::commands::badges::BadgeOutcome`: what a scan did. Refusals are coded errors.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BadgeOutcome {
    pub kind: BadgeOutcomeKind,
    pub insz: Option<String>,
    pub name: Option<String>,
    /// RFC 3339 UTC time of a registered presence.
    pub at: Option<String>,
    /// The FDM text of `presence_failed`.
    pub error: Option<String>,
    pub user: Option<User>,
}
