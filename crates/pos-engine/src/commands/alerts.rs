//! The owner alerts' commands (V12-D11 to D16): the settings the
//! owner edits, the test message and the log. The worker that watches
//! and sends is `crate::alerts`.
//!
//! The switches, the chat id and the thresholds are owner settings
//! (`owner_settings`, key `alerts`, V12-D2); the bot token is a file
//! named by `pos.json` (`alerts.telegram_bot_token_file`) and never
//! leaves this process: the screens only learn whether it is there.

use pos_db::queries::owner_settings::{self as store, AlertLogRow};
use serde::{Deserialize, Serialize};

use crate::alerts::telegram::TelegramChannel;
use crate::alerts::text::{self, Lang};
use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::owner_settings::{self, KEY_ALERTS};
use crate::state::AppState;

/// One switch per kind of alert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AlertKinds {
    pub fdm: bool,
    pub printer: bool,
    pub terminal: bool,
    pub scale: bool,
    pub z_missing: bool,
    pub online_waiting: bool,
}

impl Default for AlertKinds {
    /// Every kind watched once the owner turns the alerts on.
    fn default() -> Self {
        Self {
            fdm: true,
            printer: true,
            terminal: true,
            scale: true,
            z_missing: true,
            online_waiting: true,
        }
    }
}

pub const DEFAULT_ONLINE_WAIT_MINUTES: i64 = 5;
pub const DEFAULT_REMINDER_MINUTES: i64 = 60;

/// The stored settings. An absent document is [`Default`]: off.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AlertsSettings {
    pub enabled: bool,
    /// The Telegram chat: optional `-` and 1 to 20 digits.
    pub chat_id: String,
    /// The installation label at the head of each message; empty reads
    /// as the terminal id.
    pub label: String,
    /// `fr`, `zh` or `en`.
    pub language: String,
    pub kinds: AlertKinds,
    /// The hour (0 to 12, Belgian time, of the day after the booking
    /// date) by which the Z is due; `None` reads as `day_end_hour + 1`.
    pub z_deadline_hour: Option<i64>,
    /// An online order waits this long (1 to 120 minutes) before it
    /// counts.
    pub online_wait_minutes: i64,
    /// A reminder every so many minutes while an alert lasts (0 = none,
    /// up to 1440).
    pub reminder_minutes: i64,
}

impl Default for AlertsSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            chat_id: String::new(),
            label: String::new(),
            language: "fr".to_string(),
            kinds: AlertKinds::default(),
            z_deadline_hour: None,
            online_wait_minutes: DEFAULT_ONLINE_WAIT_MINUTES,
            reminder_minutes: DEFAULT_REMINDER_MINUTES,
        }
    }
}

impl AlertsSettings {
    /// The hour the Z is due, given the day end of `pos.json`.
    pub fn deadline_hour(&self, day_end_hour: u32) -> i64 {
        self.z_deadline_hour
            .unwrap_or_else(|| (i64::from(day_end_hour) + 1).min(12))
    }
}

/// What the owner edits, with the hour spelled out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// The settings as the screens read them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OwnerAlerts {
    pub enabled: bool,
    pub chat_id: String,
    pub label: String,
    pub language: String,
    pub kinds: AlertKinds,
    pub z_deadline_hour: i64,
    pub online_wait_minutes: i64,
    pub reminder_minutes: i64,
    /// Whether the bot token can be read; the token itself never leaves
    /// the engine.
    pub token_configured: bool,
}

/// How the test message ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AlertTestResult {
    pub sent: bool,
    pub detail: String,
}

/// The stored settings (the defaults when none are stored).
pub fn load_settings(state: &AppState) -> Result<AlertsSettings, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    owner_settings::get(&db, KEY_ALERTS)
}

fn invalid(field: &str) -> String {
    errors::coded(errors::ALERTS_INVALID, &[("field", &field)])
}

/// The chat id: optional `-`, then 1 to 20 digits.
fn chat_id_ok(chat_id: &str) -> bool {
    let digits = chat_id.strip_prefix('-').unwrap_or(chat_id);
    (1..=20).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit())
}

/// Check `input` and turn it into the settings to store.
pub fn validate(input: &OwnerAlertsInput) -> Result<AlertsSettings, String> {
    let chat_id = input.chat_id.trim().to_string();
    if chat_id.is_empty() {
        if input.enabled {
            return Err(invalid("chat_id"));
        }
    } else if !chat_id_ok(&chat_id) {
        return Err(invalid("chat_id"));
    }
    let label = input.label.trim().to_string();
    if label.chars().count() > 40 || label.chars().any(char::is_control) {
        return Err(invalid("label"));
    }
    if !["fr", "zh", "en"].contains(&input.language.as_str()) {
        return Err(invalid("language"));
    }
    if !(0..=12).contains(&input.z_deadline_hour) {
        return Err(invalid("z_deadline_hour"));
    }
    if !(1..=120).contains(&input.online_wait_minutes) {
        return Err(invalid("online_wait_minutes"));
    }
    if !(0..=1440).contains(&input.reminder_minutes) {
        return Err(invalid("reminder_minutes"));
    }
    Ok(AlertsSettings {
        enabled: input.enabled,
        chat_id,
        label,
        language: input.language.clone(),
        kinds: input.kinds.clone(),
        z_deadline_hour: Some(input.z_deadline_hour),
        online_wait_minutes: input.online_wait_minutes,
        reminder_minutes: input.reminder_minutes,
    })
}

fn token_configured(state: &AppState) -> Result<bool, String> {
    let config = state.config.lock().map_err(errors::internal)?;
    Ok(config
        .alerts
        .as_ref()
        .is_some_and(|a| matches!(a.bot_token(), Ok(Some(_)))))
}

fn present(state: &AppState, settings: AlertsSettings) -> Result<OwnerAlerts, String> {
    let day_end_hour = state.config.lock().map_err(errors::internal)?.day_end_hour;
    Ok(OwnerAlerts {
        enabled: settings.enabled,
        chat_id: settings.chat_id.clone(),
        label: settings.label.clone(),
        language: settings.language.clone(),
        kinds: settings.kinds.clone(),
        z_deadline_hour: settings.deadline_hour(day_end_hour),
        online_wait_minutes: settings.online_wait_minutes,
        reminder_minutes: settings.reminder_minutes,
        token_configured: token_configured(state)?,
    })
}

/// The owner alert settings. Managers only.
pub fn do_get_owner_alerts(state: &AppState) -> Result<OwnerAlerts, String> {
    access::operator(state, Access::Admin)?;
    present(state, load_settings(state)?)
}

/// Store the owner alert settings. Managers only; the change is logged
/// (`owner_settings_log`) with the manager's INSZ.
pub fn do_set_owner_alerts(
    state: &AppState,
    input: OwnerAlertsInput,
) -> Result<OwnerAlerts, String> {
    let admin = access::operator(state, Access::Admin)?;
    let settings = validate(&input)?;
    {
        let db = state.db.lock().map_err(errors::internal)?;
        if owner_settings::set(&db, KEY_ALERTS, &settings, Some(&admin.insz))? {
            tracing::info!(enabled = settings.enabled, "owner alert settings changed");
        }
    }
    present(state, settings)
}

/// The bot of `pos.json`, ready to send.
pub(crate) fn channel_of(state: &AppState) -> Result<TelegramChannel, String> {
    let (token, base) = {
        let config = state.config.lock().map_err(errors::internal)?;
        let Some(alerts) = &config.alerts else {
            return Err(errors::ALERTS_TOKEN_MISSING.to_string());
        };
        let Ok(Some(token)) = alerts.bot_token() else {
            return Err(errors::ALERTS_TOKEN_MISSING.to_string());
        };
        (token, alerts.api_base())
    };
    TelegramChannel::new(&state.http_tls, &base, token).map_err(errors::internal)
}

/// The label heading each message: the owner's, else the terminal id.
pub(crate) fn label_of(state: &AppState, settings: &AlertsSettings) -> Result<String, String> {
    if !settings.label.is_empty() {
        return Ok(settings.label.clone());
    }
    Ok(state
        .config
        .lock()
        .map_err(errors::internal)?
        .terminal_id
        .clone())
}

/// The SQLite spelling of a UTC time, as the log stores it.
pub(crate) fn sqlite_time(at: chrono::DateTime<chrono::Utc>) -> String {
    at.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Send one test message now, whatever the switches say, so the owner
/// can check the chat before turning the alerts on. Managers only. The
/// result is logged as a `test`.
pub async fn do_send_test_alert(state: &AppState) -> Result<AlertTestResult, String> {
    access::operator(state, Access::Admin)?;
    let settings = load_settings(state)?;
    if !chat_id_ok(&settings.chat_id) {
        return Err(invalid("chat_id"));
    }
    let channel = channel_of(state)?;
    let label = label_of(state, &settings)?;
    let body = text::test_message(&label, Lang::parse(&settings.language));
    // No lock is held while the endpoint answers.
    let outcome = channel.send(&settings.chat_id, &body, 1).await;
    {
        let db = state.db.lock().map_err(errors::internal)?;
        store::log_alert(
            &db,
            "test",
            "test",
            None,
            if outcome.sent { "sent" } else { "failed" },
            outcome.http_status,
            &sqlite_time(chrono::Utc::now()),
        )
        .map_err(errors::internal)?;
    }
    if outcome.sent {
        Ok(AlertTestResult {
            sent: true,
            detail: outcome.detail,
        })
    } else {
        tracing::warn!(detail = %outcome.detail, "owner alert test failed");
        Err(errors::coded(
            errors::ALERTS_SEND_FAILED,
            &[("detail", &outcome.detail)],
        ))
    }
}

/// The latest rows of the alert log, newest first (at most 200).
/// Managers only.
pub fn do_list_owner_alert_log(state: &AppState, limit: i64) -> Result<Vec<AlertLogRow>, String> {
    access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    store::list_alert_log(&db, limit.clamp(1, 200)).map_err(errors::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> OwnerAlertsInput {
        OwnerAlertsInput {
            enabled: true,
            chat_id: "-1001234567890".into(),
            label: "Salle".into(),
            language: "fr".into(),
            kinds: AlertKinds::default(),
            z_deadline_hour: 6,
            online_wait_minutes: 5,
            reminder_minutes: 60,
        }
    }

    fn field_of(err: &str) -> String {
        err.strip_prefix("alerts_invalid|field=")
            .unwrap_or(err)
            .to_string()
    }

    #[test]
    fn a_complete_input_is_accepted() {
        let s = validate(&input()).unwrap();
        assert!(s.enabled);
        assert_eq!(s.chat_id, "-1001234567890");
        assert_eq!(s.z_deadline_hour, Some(6));
    }

    #[test]
    fn the_defaults_are_off_with_every_kind_ready() {
        let d = AlertsSettings::default();
        assert!(!d.enabled);
        assert!(d.kinds.fdm && d.kinds.printer && d.kinds.terminal);
        assert!(d.kinds.z_missing && d.kinds.online_waiting);
        assert_eq!(d.deadline_hour(5), 6);
        assert_eq!(d.deadline_hour(12), 12);
        assert_eq!(d.reminder_minutes, 60);
        assert_eq!(d.online_wait_minutes, 5);
    }

    #[test]
    fn every_field_is_range_checked() {
        let mut bad = input();
        bad.chat_id = "12a".into();
        assert_eq!(field_of(&validate(&bad).unwrap_err()), "chat_id");
        bad.chat_id = "1".repeat(21);
        assert_eq!(field_of(&validate(&bad).unwrap_err()), "chat_id");
        bad.chat_id = "-".into();
        assert_eq!(field_of(&validate(&bad).unwrap_err()), "chat_id");
        bad.chat_id = String::new();
        assert_eq!(
            field_of(&validate(&bad).unwrap_err()),
            "chat_id",
            "an enabled alert needs a chat"
        );
        bad.enabled = false;
        assert!(
            validate(&bad).is_ok(),
            "switched off, an empty chat is fine"
        );

        let mut bad = input();
        bad.label = "x".repeat(41);
        assert_eq!(field_of(&validate(&bad).unwrap_err()), "label");
        let mut bad = input();
        bad.label = "a\nb".into();
        assert_eq!(field_of(&validate(&bad).unwrap_err()), "label");
        let mut bad = input();
        bad.language = "nl".into();
        assert_eq!(field_of(&validate(&bad).unwrap_err()), "language");
        for hour in [-1, 13] {
            let mut bad = input();
            bad.z_deadline_hour = hour;
            assert_eq!(field_of(&validate(&bad).unwrap_err()), "z_deadline_hour");
        }
        for minutes in [0, 121] {
            let mut bad = input();
            bad.online_wait_minutes = minutes;
            assert_eq!(
                field_of(&validate(&bad).unwrap_err()),
                "online_wait_minutes"
            );
        }
        for minutes in [-1, 1441] {
            let mut bad = input();
            bad.reminder_minutes = minutes;
            assert_eq!(field_of(&validate(&bad).unwrap_err()), "reminder_minutes");
        }
        let mut edge = input();
        edge.reminder_minutes = 0;
        edge.z_deadline_hour = 0;
        edge.online_wait_minutes = 120;
        assert!(validate(&edge).is_ok());
    }

    #[test]
    fn the_stored_document_refuses_an_unknown_field() {
        let json = r#"{"enabled":true,"surprise":1}"#;
        assert!(serde_json::from_str::<AlertsSettings>(json).is_err());
        let json = r#"{"enabled":true,"kinds":{"fdm":false}}"#;
        let s: AlertsSettings = serde_json::from_str(json).unwrap();
        assert!(!s.kinds.fdm && s.kinds.printer, "a missing kind reads on");
        assert_eq!(s.language, "fr");
    }
}
