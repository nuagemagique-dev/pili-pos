//! Staff badges (NFC card or key fob, read by an NFC or USB reader).
//!
//! A manager links a badge to an employee; a scan then registers that
//! employee's presence from any screen, or, when the login screen shows
//! and the setting allows it, logs a cashier or waiter in. All of it
//! sits behind `badge_settings.reader`, `off` by default: with the
//! reader off nothing reads badges and every badge command is refused.
//!
//! The engine only ever sees a normalized token
//! ([`pos_core::badge::normalize_token`]) and stores a keyed hash of it
//! ([`crate::badge_key`]). A scan is the identification of its employee
//! (POS description v2.2 §6.4); the S event it signs is byte for byte the
//! one of the manager's clock-in (`social::sign_work_event`). A badge
//! never grants manager rights: a manager's badge only asks for their
//! PIN (L5-D6).

use std::time::Instant;

use pos_core::badge::{BadgeReader, BadgeSettings, normalize_token};
use pos_core::user::{SYSTEM_INSZ, User, UserRole};
use pos_db::queries::badges::{self, ActiveBadge, LogEntry, RevokeReason};
use serde::{Deserialize, Serialize};

use crate::badge_key;
use crate::commands::access::{self, Access};
use crate::commands::auth;
use crate::commands::errors;
use crate::commands::social;
use crate::state::{AppState, Repeat};

/// Log rows the manager's overview carries.
const LOG_LIMIT: i64 = 30;

/// The setting, as the screens read it (no personal data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeSettingsView {
    /// `off`, `nfc` or `usb`.
    pub reader: String,
    pub login_enabled: bool,
    pub wedge_max_gap_ms: i64,
    pub wedge_min_len: i64,
    pub wedge_max_len: i64,
    /// True after the engine revoked badges for a changed key, until a
    /// badge is linked again.
    pub relink_needed: bool,
}

/// A new setting, from a manager.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeSettingsInput {
    pub reader: String,
    pub login_enabled: bool,
    pub wedge_max_gap_ms: i64,
    pub wedge_min_len: i64,
    pub wedge_max_len: i64,
}

/// The manager's overview: active badges and the latest log rows. Never
/// a hash.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeOverview {
    pub badges: Vec<ActiveBadge>,
    pub log: Vec<LogEntry>,
}

/// A badge just linked.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeLink {
    pub insz: String,
    pub name: String,
    /// UTC, `YYYY-MM-DD HH:MM:SS`.
    pub linked_at: String,
}

/// What a scan did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BadgeOutcomeKind {
    /// A repeat of a scan just handled: nothing shown, nothing done.
    Ignored,
    ClockIn,
    ClockOut,
    /// The FDM did not register the presence (`error` says why).
    PresenceFailed,
    /// A session was opened (`user`).
    LoggedIn,
    /// A manager's badge: nothing happened, the PIN pad opens for them.
    PinRequired,
}

/// The result of a scan, flat for TypeScript and UniFFI. Refusals are
/// `Err(code)`, not an outcome.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgeOutcome {
    pub kind: BadgeOutcomeKind,
    pub insz: Option<String>,
    pub name: Option<String>,
    /// RFC 3339 UTC time of a registered presence (the toast time).
    pub at: Option<String>,
    /// The FDM text of `presence_failed`.
    pub error: Option<String>,
    pub user: Option<User>,
}

impl BadgeOutcome {
    fn of(kind: BadgeOutcomeKind) -> Self {
        Self {
            kind,
            insz: None,
            name: None,
            at: None,
            error: None,
            user: None,
        }
    }

    fn for_user(kind: BadgeOutcomeKind, user: &User) -> Self {
        Self {
            insz: Some(user.insz.clone()),
            name: Some(user.name.clone()),
            ..Self::of(kind)
        }
    }
}

fn db_err(e: rusqlite::Error) -> String {
    errors::internal(e)
}

fn view(state: &AppState) -> Result<BadgeSettingsView, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let s = badges::settings(&db).map_err(db_err)?;
    let relink_needed = badges::relink_needed(&db).map_err(db_err)?;
    Ok(BadgeSettingsView {
        reader: s.reader.as_str().to_string(),
        login_enabled: s.login_enabled,
        wedge_max_gap_ms: s.wedge_max_gap_ms,
        wedge_min_len: s.wedge_min_len,
        wedge_max_len: s.wedge_max_len,
        relink_needed,
    })
}

/// The setting. No session needed: the login screen arms its reader
/// from it.
pub fn do_get_badge_settings(state: &AppState) -> Result<BadgeSettingsView, String> {
    view(state)
}

/// Change the setting. Managers only; a change is logged. Any value out
/// of range, or an unknown reader, is `badge_settings_invalid` and
/// nothing is stored.
pub fn do_set_badge_settings(
    state: &AppState,
    input: BadgeSettingsInput,
) -> Result<BadgeSettingsView, String> {
    let operator = access::operator(state, Access::Admin)?;
    let invalid = || errors::BADGE_SETTINGS_INVALID.to_string();
    let new = BadgeSettings {
        reader: input.reader.parse::<BadgeReader>().map_err(|_| invalid())?,
        login_enabled: input.login_enabled,
        wedge_max_gap_ms: input.wedge_max_gap_ms,
        wedge_min_len: input.wedge_min_len,
        wedge_max_len: input.wedge_max_len,
    };
    new.validate().map_err(|_| invalid())?;
    {
        let db = state.db.lock().map_err(errors::internal)?;
        badges::set_settings(&db, &new, &operator.insz).map_err(db_err)?;
    }
    view(state)
}

/// The active badges and the latest log rows. Managers only; works with
/// the reader off (a manager can still see and remove badges).
pub fn do_list_badges(state: &AppState) -> Result<BadgeOverview, String> {
    access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(BadgeOverview {
        badges: badges::list_active(&db).map_err(db_err)?,
        log: badges::history(&db, LOG_LIMIT).map_err(db_err)?,
    })
}

/// The setting, refused as `badge_reader_off` when the reader is off.
fn active_settings(state: &AppState) -> Result<BadgeSettings, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    let s = badges::settings(&db).map_err(db_err)?;
    if s.reader == BadgeReader::Off {
        return Err(errors::BADGE_READER_OFF.to_string());
    }
    Ok(s)
}

/// The hash of a raw reader output: normalize, then key it. The raw
/// token goes no further than this function.
fn hash_of(state: &AppState, raw: &str) -> Result<String, String> {
    let token = normalize_token(raw).map_err(|_| errors::BADGE_TOKEN_INVALID.to_string())?;
    let key = {
        let db = state.db.lock().map_err(errors::internal)?;
        badge_key::key_for_use(&db, &state.paths.data_dir)
    }
    .map_err(|e| {
        tracing::warn!(%e, "badge key unavailable");
        errors::BADGE_KEY_UNAVAILABLE.to_string()
    })?;
    Ok(badge_key::hash_token(&key, &token))
}

/// Link a badge to an employee. A manager, confirmed by their own PIN
/// (a badge is a login credential). Refusals, in this order, nothing
/// written: `forbidden`, `badge_reader_off`, `badge_token_invalid`,
/// `badge_key_unavailable`, `system_user`, `user_unknown` (unknown or
/// inactive), `badge_already_linked` (the employee has an active
/// badge), `badge_in_use` (the badge is active for anyone), then the PIN
/// (`pin_required`, `pin_wrong`, `pin_locked`).
pub fn do_link_badge(
    state: &AppState,
    employee_insz: String,
    token: String,
    operator_pin: Option<String>,
) -> Result<BadgeLink, String> {
    let operator = access::operator(state, Access::Admin)?;
    active_settings(state)?;
    let hash = hash_of(state, &token)?;
    if employee_insz == SYSTEM_INSZ {
        return Err(errors::SYSTEM_USER.to_string());
    }
    let check = |db: &rusqlite::Connection| -> Result<User, String> {
        // `get_user` only finds active users; presence-only ones are fine.
        let user = pos_db::queries::users::get_user(db, &employee_insz)
            .map_err(db_err)?
            .ok_or_else(|| errors::USER_UNKNOWN.to_string())?;
        if badges::active_link_by_user(db, &employee_insz)
            .map_err(db_err)?
            .is_some()
        {
            return Err(errors::BADGE_ALREADY_LINKED.to_string());
        }
        if badges::active_link_by_hash(db, &hash)
            .map_err(db_err)?
            .is_some()
        {
            return Err(errors::BADGE_IN_USE.to_string());
        }
        Ok(user)
    };
    {
        let db = state.db.lock().map_err(errors::internal)?;
        check(&db)?;
    }
    let pin = operator_pin.ok_or_else(|| errors::PIN_REQUIRED.to_string())?;
    access::confirmed_operator(state, &operator.insz, &pin, Access::Admin)?;

    let db = state.db.lock().map_err(errors::internal)?;
    // The PIN check ran without the lock: look again before writing.
    let user = check(&db)?;
    badges::atomic(&db, |tx| {
        badges::link(tx, &hash, &employee_insz, &operator.insz)?;
        if badges::key_check(tx)?.is_none() {
            let key = badge_key::load_or_create(&state.paths.data_dir)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
            badges::set_key_check(tx, &badge_key::key_check(&key))?;
        }
        Ok(())
    })
    .map_err(|e| match e {
        rusqlite::Error::ToSqlConversionFailure(_) => errors::BADGE_KEY_UNAVAILABLE.to_string(),
        other => db_err(other),
    })?;
    let linked_at = badges::list_active(&db)
        .map_err(db_err)?
        .into_iter()
        .find(|b| b.insz == employee_insz)
        .map(|b| b.linked_at)
        .unwrap_or_default();
    Ok(BadgeLink {
        insz: user.insz,
        name: user.name,
        linked_at,
    })
}

/// Remove an employee's badge. Managers only, no PIN (it only removes
/// access). `badge_none` when the employee has no active badge.
pub fn do_revoke_badge(state: &AppState, employee_insz: String) -> Result<(), String> {
    let operator = access::operator(state, Access::Admin)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let revoked = badges::revoke(
        &db,
        &employee_insz,
        RevokeReason::Manager,
        Some(&operator.insz),
    )
    .map_err(db_err)?;
    if revoked {
        Ok(())
    } else {
        Err(errors::BADGE_NONE.to_string())
    }
}

/// Marks a badge's scan as finished when dropped, on every exit path,
/// so the repeat window runs from the END of the scan (L5-D10).
struct RepeatGuard<'a> {
    state: &'a AppState,
    hash: String,
}

impl Drop for RepeatGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut repeats) = self.state.badge_repeats.lock() {
            repeats.insert(self.hash.clone(), Repeat::Done(Instant::now()));
        }
    }
}

/// Take the badge's scan, or `None` when it is a repeat (the previous
/// scan is still being handled, or ended less than `badge_repeat` ago).
fn begin_scan<'a>(state: &'a AppState, hash: &str) -> Result<Option<RepeatGuard<'a>>, String> {
    let window = state.timers.badge_repeat;
    let now = Instant::now();
    let mut repeats = state.badge_repeats.lock().map_err(errors::internal)?;
    repeats.retain(|_, r| match r {
        Repeat::InFlight => true,
        Repeat::Done(at) => now.saturating_duration_since(*at) < window,
    });
    if repeats.contains_key(hash) {
        return Ok(None);
    }
    repeats.insert(hash.to_string(), Repeat::InFlight);
    Ok(Some(RepeatGuard {
        state,
        hash: hash.to_string(),
    }))
}

/// The scan journal (`badge_scans`) of one scan: the badge is named by
/// its short reference, never by its number.
struct Journal<'a> {
    state: &'a AppState,
    badge_ref: String,
}

impl Journal<'_> {
    fn record(&self, outcome: &str, insz: Option<&str>, detail: Option<&str>) {
        let logged = self
            .state
            .db
            .lock()
            .map_err(|e| e.to_string())
            .and_then(|db| {
                badges::log_scan(&db, outcome, insz, &self.badge_ref, detail)
                    .map_err(|e| e.to_string())
            });
        if let Err(e) = logged {
            tracing::error!(%e, "badge scan not journaled");
        }
    }

    /// Journal a refusal and hand its code back as the command's error.
    fn refuse(&self, code: &str, insz: Option<&str>) -> String {
        self.record("refused", insz, Some(code));
        code.to_string()
    }
}

/// A scan. `token` is the reader's raw output, `end_shift` is true while
/// the login screen is in its "Fin de service" mode.
///
/// After the reader, token, key and repeat checks (`badge_reader_off`,
/// `badge_token_invalid`, `badge_key_unavailable`: nothing logged;
/// a repeat: [`BadgeOutcomeKind::Ignored`]), in this order (L5-D7):
///
/// 1. unknown or removed badge: `badge_unknown` / `badge_revoked`;
/// 2. a session is open: the presence of the badge's employee toggles;
///    the logged-in operator's own badge is `presence_self`;
/// 3. no session, `end_shift`: clock-out only, `presence_not_in` when
///    not on shift, no session opened;
/// 4. no session, badge login on, employee not presence-only: a manager
///    gets `pin_required`, anyone else is logged in like a PIN login
///    minus the PIN;
/// 5. otherwise the presence toggles.
///
/// A presence toggle is refused with `presence_training` in training
/// mode. Every scan but a repeat is written to `badge_scans`.
pub async fn do_badge_tap(
    state: &AppState,
    token: String,
    end_shift: bool,
) -> Result<BadgeOutcome, String> {
    let settings = active_settings(state)?;
    let hash = hash_of(state, &token)?;
    drop(token);
    let Some(_guard) = begin_scan(state, &hash)? else {
        return Ok(BadgeOutcome::of(BadgeOutcomeKind::Ignored));
    };
    let journal = Journal {
        state,
        badge_ref: badge_key::badge_ref(&hash).to_string(),
    };

    // 1. Whose badge is it?
    let (link, user) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let link = badges::any_link_by_hash(&db, &hash).map_err(db_err)?;
        let user = match &link {
            Some(l) if !l.revoked => {
                pos_db::queries::users::get_user(&db, &l.user_insz).map_err(db_err)?
            }
            _ => None,
        };
        (link, user)
    };
    let Some(link) = link else {
        journal.record("unknown", None, None);
        return Err(errors::BADGE_UNKNOWN.to_string());
    };
    if link.revoked {
        journal.record("revoked", Some(&link.user_insz), None);
        return Err(errors::BADGE_REVOKED.to_string());
    }
    let Some(user) = user else {
        return Err(journal.refuse(errors::USER_UNKNOWN, Some(&link.user_insz)));
    };

    let session = state.session.lock().map_err(errors::internal)?.clone();

    // 2. A session is open: the badge registers its holder's presence.
    if let Some(operator) = &session {
        if operator.insz == user.insz {
            return Err(journal.refuse(errors::PRESENCE_SELF, Some(&user.insz)));
        }
        return toggle_presence(state, &user, None, &journal).await;
    }

    // 3. The login screen in "Fin de service": clock out, no session.
    if end_shift {
        return toggle_presence(state, &user, Some(false), &journal).await;
    }

    // 4. Badge login, for those who log in.
    if settings.login_enabled && !user.presence_only {
        if user.role == UserRole::Admin {
            journal.record("pin_required", Some(&user.insz), None);
            return Ok(BadgeOutcome::for_user(BadgeOutcomeKind::PinRequired, &user));
        }
        auth::open_login_session(state, &user)?;
        journal.record("login", Some(&user.insz), None);
        return Ok(BadgeOutcome {
            user: Some(user.clone()),
            ..BadgeOutcome::for_user(BadgeOutcomeKind::LoggedIn, &user)
        });
    }

    // 5. Presence only, or badge login off.
    toggle_presence(state, &user, None, &journal).await
}

/// Register the arrival or departure of `user`. `force` is `Some(false)`
/// for a clock-out only (end of shift); `None` lets the local shift
/// index decide.
async fn toggle_presence(
    state: &AppState,
    user: &User,
    force: Option<bool>,
    journal: &Journal<'_>,
) -> Result<BadgeOutcome, String> {
    // A T event registers no presence: toggling in training would show
    // "Arrivée" every time (L1F-A4, L5-D9).
    if state.config.lock().map_err(errors::internal)?.training_mode {
        return Err(journal.refuse(errors::PRESENCE_TRAINING, Some(&user.insz)));
    }
    let on_shift = {
        let db = state.db.lock().map_err(errors::internal)?;
        pos_db::queries::shifts::has_open_shift(&db, &user.insz).map_err(db_err)?
    };
    let is_in = match force {
        Some(is_in) => is_in,
        None => !on_shift,
    };
    if !is_in && !on_shift {
        return Err(journal.refuse(errors::PRESENCE_NOT_IN, Some(&user.insz)));
    }
    let result = match social::sign_work_event(state, user.insz.clone(), is_in, None).await {
        Ok(result) => result,
        Err(code) => return Err(journal.refuse(&code, Some(&user.insz))),
    };
    if result.success {
        let kind = if is_in {
            BadgeOutcomeKind::ClockIn
        } else {
            BadgeOutcomeKind::ClockOut
        };
        journal.record(
            if is_in { "clock_in" } else { "clock_out" },
            Some(&user.insz),
            None,
        );
        return Ok(BadgeOutcome {
            at: Some(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            ..BadgeOutcome::for_user(kind, user)
        });
    }
    let error = result.error.unwrap_or_else(|| "FDM error".to_string());
    journal.record("presence_failed", Some(&user.insz), Some(&error));
    Ok(BadgeOutcome {
        error: Some(error),
        ..BadgeOutcome::for_user(BadgeOutcomeKind::PresenceFailed, user)
    })
}
