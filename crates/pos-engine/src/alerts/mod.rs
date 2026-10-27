//! The owner alerts (V12-D11 to D16): a Telegram message to the
//! owner's phone when the FDM is unreachable or blocked, a printer is
//! out of paper or offline, the linked card terminal does not answer,
//! the day has no Z by a set hour or an online order waits unaccepted.
//!
//! Not fiscal, off by default, and never in the path of a command: the
//! worker is its own loop ([`run_owner_alerts`], started by
//! `background.rs`). It
//!
//! - reads its settings on every tick, so switching off stops it at
//!   once and, off, it does nothing but that one read (no probe, no
//!   HTTP request);
//! - holds neither `state.db` nor the signing lock while an HTTP request
//!   is in flight: it reads under the lock, then sends;
//! - never probes a printer while a sale is being signed, nor the card
//!   terminal while a payment holds `eft_line`;
//! - judges the FDM only while a session is open (the FDM monitor only
//!   polls then), the printers and the terminal every five minutes,
//!   also only while a session is open.
//!
//! [`rules`] decides what to send (debounce, reminder, resolve, the
//! hourly cap), [`telegram`] sends it, [`text`] words it. [`run_pass`]
//! is one pass with the time given, so the tests drive it on a chosen
//! clock.

pub mod rules;
pub mod telegram;
pub mod text;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe::Brussels;
use pos_db::queries::booking::BookingPeriod;
use pos_db::queries::owner_settings as store;
use pos_print::transport::TcpPrinter;

use crate::commands::alerts::{self, AlertsSettings};
use crate::commands::{fdm_monitor, signing};
use crate::state::AppState;
use rules::{AlertBook, Decision, Detail, Kind, LastRow, Printer, PrinterProblem, Transition};
use telegram::MAX_ATTEMPTS;
use text::Lang;

/// How often the worker looks (the settings are read on every tick).
pub const TICK: Duration = Duration::from_secs(30);
/// How often the printers and the terminal are probed.
pub const PROBE_INTERVAL: chrono::Duration = chrono::Duration::minutes(5);

/// The longest the terminal probe may hold the card terminal's line. A
/// payment never waits behind a probe for longer than this (see
/// `EngineTimers::terminal_probe`, which defaults to it). The slowest honest check is
/// Worldline's: two requests (v2, then v1 on a 404) of `CHECK_TIMEOUT` each, 10 s. The
/// probe timeout is 12 s so that a reachable terminal that answers slowly is never judged
/// "unknown" by the clock (INT-D25).
pub const TERMINAL_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(12);

// The probe timeout always leaves room for the slowest terminal check, plus a margin.
const _: () = assert!(
    TERMINAL_PROBE_TIMEOUT.as_millis() >= 2 * pos_eft::worldline::CHECK_TIMEOUT.as_millis() + 1_000
);

#[derive(Debug)]
struct Inner {
    book: AlertBook,
    /// The book was filled from the log since the alerts were last
    /// switched on.
    restored: bool,
    last_probe: Option<DateTime<Utc>>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            book: AlertBook::new(),
            restored: false,
            last_probe: None,
        }
    }
}

/// What the worker remembers between two passes. Held by `AppState`;
/// the lock is never kept across an await.
#[derive(Debug)]
pub struct AlertRuntime {
    inner: Mutex<Inner>,
    /// The wait between the attempts of one message (tests shorten it).
    backoff: Mutex<Duration>,
}

impl Default for AlertRuntime {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Inner::default()),
            backoff: Mutex::new(telegram::DEFAULT_BACKOFF),
        }
    }
}

impl AlertRuntime {
    /// Wait `backoff` between the attempts of one message (tests).
    pub fn set_backoff(&self, backoff: Duration) {
        if let Ok(mut b) = self.backoff.lock() {
            *b = backoff;
        }
    }

    fn backoff(&self) -> Duration {
        self.backoff
            .lock()
            .map_or(telegram::DEFAULT_BACKOFF, |b| *b)
    }

    /// Forget everything: the alerts are off, or cannot be sent.
    fn reset(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            *inner = Inner::default();
        }
    }
}

/// What one pass did with one message, for the tests and the logs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub kind: Kind,
    pub transition: Transition,
    /// `sent`, `failed` or `suppressed`, as logged.
    pub outcome: &'static str,
}

/// What one observation of a kind found.
enum Seen {
    /// Nothing can be said now (no session, a payment in progress):
    /// the state of the kind stays as it is.
    Unknown,
    /// This kind does not apply (switched off, not configured): its
    /// state is forgotten, nothing is resolved.
    NotApplicable,
    /// All is well.
    Fine,
    /// The condition holds.
    Trouble(Detail),
}

/// The loop: one pass per tick until the shutdown token is cancelled.
/// A pass in flight is dropped at shutdown.
pub async fn run_owner_alerts(state: Arc<AppState>) {
    let mut tick = tokio::time::interval(TICK);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            () = state.shutdown_token.cancelled() => return,
            _ = tick.tick() => {}
        }
        tokio::select! {
            () = state.shutdown_token.cancelled() => return,
            _ = run_pass(&state, Utc::now()) => {}
        }
    }
}

fn sqlite_time(at: DateTime<Utc>) -> String {
    alerts::sqlite_time(at)
}

fn parse_sqlite_time(text: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .ok()
        .map(|n| n.and_utc())
}

/// One pass at `now`: observe every watched kind, send what the rules
/// decide. Does nothing (and sends nothing) while the alerts are off or
/// the bot token is missing.
pub async fn run_pass(state: &AppState, now: DateTime<Utc>) -> Vec<Sent> {
    let settings = match alerts::load_settings(state) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(%e, "owner alerts: cannot read the settings");
            return Vec::new();
        }
    };
    if !settings.enabled || settings.chat_id.is_empty() {
        state.owner_alerts.reset();
        return Vec::new();
    }
    let channel = if let Ok(c) = alerts::channel_of(state) {
        c.with_backoff(state.owner_alerts.backoff())
    } else {
        // No token: nothing can be sent, and nothing is probed for it.
        state.owner_alerts.reset();
        return Vec::new();
    };

    // A failed restore means a blank book: observing now would raise
    // every open alert a second time and leave the rate window unseeded.
    if !restore_once(state, &settings, now) {
        return Vec::new();
    }

    let seen = observe_all(state, &settings, now).await;

    let decisions: Vec<Decision> = {
        let Ok(mut inner) = state.owner_alerts.inner.lock() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (kind, seen) in seen {
            let observed = match seen {
                Seen::Unknown => continue,
                Seen::NotApplicable => {
                    inner.book.clear(kind);
                    continue;
                }
                Seen::Fine => None,
                Seen::Trouble(detail) => Some(detail),
            };
            out.extend(
                inner
                    .book
                    .observe(kind, observed, settings.reminder_minutes, now),
            );
        }
        out
    };

    let label = alerts::label_of(state, &settings).unwrap_or_default();
    let lang = Lang::parse(&settings.language);
    let mut done = Vec::new();
    for decision in decisions {
        let permitted = state
            .owner_alerts
            .inner
            .lock()
            .is_ok_and(|mut inner| inner.book.permit(now));
        let (outcome, http_status) = if permitted {
            let body = text::message(&decision, &label, lang, now);
            // No lock is held while the endpoint answers.
            let result = channel.send(&settings.chat_id, &body, MAX_ATTEMPTS).await;
            if !result.sent {
                tracing::warn!(
                    kind = decision.kind.code(),
                    detail = %result.detail,
                    "owner alert dropped"
                );
            }
            (
                if result.sent { "sent" } else { "failed" },
                result.http_status,
            )
        } else {
            tracing::warn!(
                kind = decision.kind.code(),
                "owner alert suppressed by the hourly cap"
            );
            ("suppressed", None)
        };
        match state.db.lock() {
            Ok(db) => {
                if let Err(e) = store::log_alert(
                    &db,
                    decision.kind.code(),
                    decision.transition.code(),
                    decision.detail.to_log().as_deref(),
                    outcome,
                    http_status,
                    &sqlite_time(now),
                ) {
                    tracing::error!(%e, "owner alerts: cannot write the log");
                }
            }
            Err(e) => tracing::error!(%e, "owner alerts: database lock poisoned"),
        }
        done.push(Sent {
            kind: decision.kind,
            transition: decision.transition,
            outcome,
        });
    }
    done
}

/// Fill the book from the log once since the alerts were switched on:
/// the alerts that were open, the messages of the last hour. Returns
/// whether the book is restored (now or before); `false` means the pass
/// must not observe or send.
fn restore_once(state: &AppState, settings: &AlertsSettings, now: DateTime<Utc>) -> bool {
    let Ok(mut inner) = state.owner_alerts.inner.lock() else {
        return false;
    };
    if inner.restored {
        return true;
    }
    let Ok(db) = state.db.lock() else {
        return false;
    };
    // Read everything first: a failed read leaves the book untouched and
    // the next pass tries again, instead of starting from a blank book
    // (which would raise every open alert a second time).
    let mut lasts = Vec::new();
    for kind in Kind::ALL {
        let row = match store::last_alert_row(&db, kind.code()) {
            Ok(row) => row,
            Err(e) => {
                tracing::error!(%e, "owner alerts: cannot restore the book, will retry");
                return false;
            }
        };
        let last = row.and_then(|row| {
            Some(LastRow {
                transition: Transition::from_code(&row.transition)?,
                sent: row.outcome == "sent",
                at: parse_sqlite_time(&row.created_at)?,
            })
        });
        lasts.push((kind, last));
    }
    let since = sqlite_time(now - chrono::Duration::minutes(rules::CAP_WINDOW_MINUTES));
    let times = match store::sent_times_since(&db, &since) {
        Ok(times) => times,
        Err(e) => {
            tracing::error!(%e, "owner alerts: cannot read the recent messages, will retry");
            return false;
        }
    };
    drop(db);
    for (kind, last) in lasts {
        inner
            .book
            .restore(kind, last, settings.reminder_minutes, now);
    }
    inner
        .book
        .seed_sent(times.iter().filter_map(|t| parse_sqlite_time(t)));
    inner.restored = true;
    true
}

async fn observe_all(
    state: &AppState,
    settings: &AlertsSettings,
    now: DateTime<Utc>,
) -> Vec<(Kind, Seen)> {
    let session_open = state.session.lock().is_ok_and(|s| s.is_some());
    let mut seen = Vec::new();

    seen.push((
        Kind::Fdm,
        if !settings.kinds.fdm {
            Seen::NotApplicable
        } else if session_open {
            observe_fdm(state)
        } else {
            Seen::Unknown
        },
    ));

    let probe_due = session_open
        && state
            .owner_alerts
            .inner
            .lock()
            .is_ok_and(|inner| inner.last_probe.is_none_or(|p| now - p >= PROBE_INTERVAL));
    let wants_probe = settings.kinds.printer || settings.kinds.terminal;
    let mut probed = false;
    let (printer_seen, terminal_seen) = if probe_due && wants_probe {
        // A sale being signed is not disturbed: no probe while the
        // signing lock is taken (the guard is dropped at once, the lock
        // is never held across the probe).
        let free = signing::try_acquire(state).is_some();
        if free {
            probed = true;
            let printer = if settings.kinds.printer {
                observe_printers(state).await
            } else {
                Seen::NotApplicable
            };
            let terminal = if settings.kinds.terminal {
                observe_terminal(state).await
            } else {
                Seen::NotApplicable
            };
            (printer, terminal)
        } else {
            (Seen::Unknown, Seen::Unknown)
        }
    } else {
        (
            if settings.kinds.printer {
                Seen::Unknown
            } else {
                Seen::NotApplicable
            },
            if settings.kinds.terminal {
                Seen::Unknown
            } else {
                Seen::NotApplicable
            },
        )
    };
    if probed && let Ok(mut inner) = state.owner_alerts.inner.lock() {
        inner.last_probe = Some(now);
    }
    seen.push((Kind::Printer, printer_seen));
    seen.push((Kind::Terminal, terminal_seen));

    seen.push((
        Kind::ZMissing,
        if settings.kinds.z_missing {
            observe_z(state, settings, now)
        } else {
            Seen::NotApplicable
        },
    ));
    seen.push((
        Kind::OnlineWaiting,
        if settings.kinds.online_waiting {
            observe_online(state, settings, now)
        } else {
            Seen::NotApplicable
        },
    ));
    seen
}

/// What the FDM monitor last saw; the caller checked a session is open.
fn observe_fdm(state: &AppState) -> Seen {
    match fdm_monitor::do_fdm_health(state) {
        Ok(Some(health)) if health.connected => Seen::Fine,
        Ok(Some(_)) => Seen::Trouble(Detail::default()),
        _ => Seen::Unknown,
    }
}

/// What is wrong with the printer at `address`, `None` when it is fine.
async fn printer_problem(state: &AppState, address: &str) -> Option<PrinterProblem> {
    let printer = TcpPrinter::new(address).with_status_timeout(state.timers.printer_status_read);
    // Nothing to print: connect and ask the real-time status only.
    match printer.send(&[]).await {
        Err(_) => Some(PrinterProblem::Offline),
        Ok(status) if status.ready() => None,
        Ok(status) if !status.paper_ok => Some(PrinterProblem::PaperOut),
        Ok(status) if !status.cover_closed => Some(PrinterProblem::CoverOpen),
        Ok(_) => Some(PrinterProblem::Error),
    }
}

async fn observe_printers(state: &AppState) -> Seen {
    let (ticket, kitchen) = match state.config.lock() {
        Ok(c) => (c.printer_address.clone(), c.kitchen_printer_address.clone()),
        Err(_) => return Seen::Unknown,
    };
    let kitchen = kitchen.filter(|k| Some(k) != ticket.as_ref());
    if ticket.is_none() && kitchen.is_none() {
        return Seen::NotApplicable;
    }
    let mut problems = Vec::new();
    if let Some(address) = &ticket
        && let Some(problem) = printer_problem(state, address).await
    {
        problems.push((Printer::Ticket, problem));
    }
    if let Some(address) = &kitchen
        && let Some(problem) = printer_problem(state, address).await
    {
        problems.push((Printer::Kitchen, problem));
    }
    if problems.is_empty() {
        Seen::Fine
    } else {
        Seen::Trouble(Detail {
            printers: problems,
            ..Detail::default()
        })
    }
}

async fn observe_terminal(state: &AppState) -> Seen {
    // The line is taken with `try_lock` inside the check (a busy line is
    // skipped, never waited for) and the check is dropped, the line with
    // it, after the short probe timeout: a payment never waits behind a
    // hanging terminal.
    let probe = crate::commands::eft::do_check_payment_terminal(state);
    let Ok(answer) = tokio::time::timeout(state.timers.terminal_probe, probe).await else {
        return Seen::Unknown;
    };
    match answer {
        Ok(status) if !status.configured => Seen::NotApplicable,
        // A payment holds the line: the terminal is busy, not gone.
        Ok(status) if status.busy => Seen::Unknown,
        Ok(status) if status.connected => Seen::Fine,
        Ok(_) => Seen::Trouble(Detail::default()),
        Err(_) => Seen::Unknown,
    }
}

/// The Z is missing when a booking period is still open at the hour
/// the Z of its day was due: `deadline_hour` o'clock Belgian time on
/// the day after its booking date.
pub(crate) fn z_missing_detail(
    open: Option<&BookingPeriod>,
    now: DateTime<Utc>,
    deadline_hour: i64,
) -> Option<Detail> {
    let period = open?;
    let date = NaiveDate::parse_from_str(&period.booking_date, "%Y-%m-%d").ok()?;
    let hour = u32::try_from(deadline_hour).ok()?;
    let local = date.succ_opt()?.and_hms_opt(hour, 0, 0)?;
    let deadline = match Brussels.from_local_datetime(&local) {
        chrono::LocalResult::Single(d) => d,
        chrono::LocalResult::Ambiguous(first, _) => first,
        // The hour that does not exist on the day clocks go forward.
        chrono::LocalResult::None => Brussels
            .from_local_datetime(&(local + chrono::Duration::hours(1)))
            .earliest()?,
    };
    (now >= deadline.with_timezone(&Utc)).then(|| Detail {
        booking_date: Some(period.booking_date.clone()),
        deadline_hour: Some(deadline_hour),
        ..Detail::default()
    })
}

fn observe_z(state: &AppState, settings: &AlertsSettings, now: DateTime<Utc>) -> Seen {
    let (training, day_end_hour) = match state.config.lock() {
        Ok(c) => (c.training_mode, c.day_end_hour),
        Err(_) => return Seen::Unknown,
    };
    if training {
        // Training closes no booking period.
        return Seen::NotApplicable;
    }
    let open = match state.db.lock() {
        Ok(db) => match pos_db::queries::booking::open_period(&db) {
            Ok(open) => open,
            Err(e) => {
                tracing::warn!(%e, "owner alerts: cannot read the booking period");
                return Seen::Unknown;
            }
        },
        Err(_) => return Seen::Unknown,
    };
    match z_missing_detail(open.as_ref(), now, settings.deadline_hour(day_end_hour)) {
        Some(detail) => Seen::Trouble(detail),
        None => Seen::Fine,
    }
}

#[cfg(feature = "hubrise")]
fn observe_online(state: &AppState, settings: &AlertsSettings, now: DateTime<Utc>) -> Seen {
    // Only with the HubRise switch on; off, there is nothing to wait for.
    if !state.hubrise_switch.is_on() {
        return Seen::NotApplicable;
    }
    let before = sqlite_time(now - chrono::Duration::minutes(settings.online_wait_minutes));
    let rows = match state.db.lock() {
        Ok(db) => match pos_db::queries::integrations::waiting_online_orders(&db, &before) {
            Ok(rows) => rows,
            Err(e) => {
                tracing::warn!(%e, "owner alerts: cannot read the waiting orders");
                return Seen::Unknown;
            }
        },
        Err(_) => return Seen::Unknown,
    };
    if rows.is_empty() {
        return Seen::Fine;
    }
    Seen::Trouble(Detail {
        count: Some(rows.iter().map(|(_, n)| n).sum()),
        channels: rows
            .iter()
            .map(|(channel, _)| text::clean_channel(channel))
            .collect(),
        wait_minutes: Some(settings.online_wait_minutes),
        ..Detail::default()
    })
}

#[cfg(not(feature = "hubrise"))]
fn observe_online(_state: &AppState, _settings: &AlertsSettings, _now: DateTime<Utc>) -> Seen {
    Seen::NotApplicable
}

/// Whether the worker would do anything: the alerts are switched on.
pub fn is_enabled(state: &AppState) -> bool {
    alerts::load_settings(state).is_ok_and(|s| s.enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn period(date: &str) -> BookingPeriod {
        BookingPeriod {
            id: "p".into(),
            booking_date: date.into(),
        }
    }

    fn utc(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn the_z_is_due_at_the_deadline_hour_of_the_next_day() {
        let open = period("2026-10-16");
        // Deadline 06:00 Brussels on the 17th (UTC+2 in October) = 04:00 UTC.
        assert_eq!(
            z_missing_detail(Some(&open), utc(2026, 10, 17, 3, 59), 6),
            None
        );
        let d = z_missing_detail(Some(&open), utc(2026, 10, 17, 4, 0), 6).unwrap();
        assert_eq!(d.booking_date.as_deref(), Some("2026-10-16"));
        assert_eq!(d.deadline_hour, Some(6));
        // A period of today is not late.
        let today = period("2026-10-17");
        assert_eq!(
            z_missing_detail(Some(&today), utc(2026, 10, 17, 22, 0), 6),
            None
        );
        assert_eq!(z_missing_detail(None, utc(2026, 10, 17, 22, 0), 6), None);
    }

    #[test]
    fn the_deadline_follows_winter_time() {
        let open = period("2026-12-10");
        // UTC+1 in December: 06:00 Brussels = 05:00 UTC.
        assert_eq!(
            z_missing_detail(Some(&open), utc(2026, 12, 11, 4, 59), 6),
            None
        );
        assert!(z_missing_detail(Some(&open), utc(2026, 12, 11, 5, 0), 6).is_some());
    }

    #[test]
    fn a_deadline_inside_the_skipped_hour_is_not_lost() {
        // 2027-03-28: clocks go from 02:00 to 03:00 (CET to CEST).
        let open = period("2027-03-27");
        let d = z_missing_detail(Some(&open), utc(2027, 3, 28, 2, 0), 2);
        assert!(d.is_some(), "02:00 does not exist, 03:00 CEST = 01:00 UTC");
        assert_eq!(
            z_missing_detail(Some(&open), utc(2027, 3, 28, 0, 59), 2),
            None
        );
    }

    #[test]
    fn a_bad_booking_date_is_never_late() {
        let open = period("not a date");
        assert_eq!(
            z_missing_detail(Some(&open), utc(2030, 1, 1, 0, 0), 6),
            None
        );
    }

    #[test]
    fn sqlite_times_round_trip() {
        let t = utc(2026, 10, 17, 12, 30);
        assert_eq!(parse_sqlite_time(&sqlite_time(t)), Some(t));
        assert_eq!(parse_sqlite_time("garbage"), None);
    }
}
