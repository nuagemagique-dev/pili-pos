//! Records of the owner alerts (engine module `alerts`, v1.2): the settings the owner edits, the
//! test result and the log rows. Mirrors the engine types (PD56): same field names, same serde
//! form, `deny_unknown_fields`. The bot token is never part of a record.

/// `pos_engine::commands::alerts::AlertKinds`: one switch per kind of alert.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AlertKinds {
    pub fdm: bool,
    pub printer: bool,
    pub terminal: bool,
    pub z_missing: bool,
    pub online_waiting: bool,
}

/// `pos_engine::commands::alerts::OwnerAlerts`: the settings as the screens read them.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct OwnerAlerts {
    pub enabled: bool,
    pub chat_id: String,
    pub label: String,
    /// `fr`, `zh` or `en`.
    pub language: String,
    pub kinds: AlertKinds,
    pub z_deadline_hour: i64,
    pub online_wait_minutes: i64,
    pub reminder_minutes: i64,
    /// Whether the bot token can be read; the token itself never leaves the engine.
    pub token_configured: bool,
}

/// `pos_engine::commands::alerts::OwnerAlertsInput`: what the owner edits.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct OwnerAlertsInput {
    pub enabled: bool,
    pub chat_id: String,
    pub label: String,
    pub language: String,
    pub kinds: AlertKinds,
    pub z_deadline_hour: i64,
    pub online_wait_minutes: i64,
    pub reminder_minutes: i64,
}

/// `pos_engine::commands::alerts::AlertTestResult`: how the test message ended.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AlertTestResult {
    pub sent: bool,
    pub detail: String,
}

/// `pos_db::queries::owner_settings::AlertLogRow`: one message sent or dropped.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct AlertLogRow {
    pub id: i64,
    /// `fdm`, `printer`, `terminal`, `z_missing`, `online_waiting` or `test`.
    pub kind: String,
    /// `raised`, `reminder`, `resolved` or `test`.
    pub transition: String,
    /// Counts, a booking date or channel names; never personal data.
    pub detail: Option<String>,
    /// `sent`, `failed` or `suppressed`.
    pub outcome: String,
    pub http_status: Option<i64>,
    pub created_at: String,
}
