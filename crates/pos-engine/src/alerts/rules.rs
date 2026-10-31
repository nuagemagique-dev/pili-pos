//! The owner alert rules: a pure state machine, no clock, no I/O.
//!
//! One alert per [`Kind`] is open at a time. A condition that holds is
//! raised once (after its debounce), reminded every `reminder_minutes`
//! while it lasts and resolved once when it clears. A global cap keeps
//! the phone from being flooded whatever happens: at most
//! [`GLOBAL_CAP`] messages in any rolling hour, the rest dropped (the
//! worker logs them `suppressed`). The caller passes the time of each
//! call, so the tests run on a fixed clock.
//!
//! A decision updates the state at once, whether the message then
//! arrives or not: a message that fails after its attempts is dropped
//! and the next reminder tries again, the loop never hammers a dead
//! endpoint on every tick.

use std::collections::VecDeque;

use chrono::{DateTime, Duration, Utc};

/// The most messages sent in any rolling hour (tests excluded).
pub const GLOBAL_CAP: usize = 20;
/// The window of [`GLOBAL_CAP`].
pub const CAP_WINDOW_MINUTES: i64 = 60;
/// How long an open alert from before a restart still counts as open
/// when reminders are off.
const RESTORE_WINDOW_NO_REMINDER_MINUTES: i64 = 24 * 60;

/// What can go wrong, one open alert each.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The FDM does not answer or is blocked.
    Fdm,
    /// A printer is offline, out of paper or has its cover open.
    Printer,
    /// The linked card terminal does not answer.
    Terminal,
    /// The checkout scale does not answer or refused the software check.
    Scale,
    /// The booking period that should have closed is still open.
    ZMissing,
    /// An online order waits for acceptance.
    OnlineWaiting,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Fdm,
        Kind::Printer,
        Kind::Terminal,
        Kind::Scale,
        Kind::ZMissing,
        Kind::OnlineWaiting,
    ];

    /// The spelling of the log (`owner_alert_log.kind`).
    pub fn code(self) -> &'static str {
        match self {
            Kind::Fdm => "fdm",
            Kind::Printer => "printer",
            Kind::Terminal => "terminal",
            Kind::Scale => "scale",
            Kind::ZMissing => "z_missing",
            Kind::OnlineWaiting => "online_waiting",
        }
    }

    pub fn from_code(code: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.code() == code)
    }

    fn index(self) -> usize {
        Kind::ALL
            .iter()
            .position(|k| *k == self)
            .expect("every kind is in ALL")
    }

    /// How long a condition must hold, and how many consecutive
    /// observations must see it, before it is raised. The FDM monitor
    /// observes every poll, so the FDM waits 2 minutes; the probes of
    /// the printers and the terminal are 5 minutes apart, so 2
    /// consecutive probes already mean 5 minutes of trouble; the other
    /// two conditions carry their own threshold.
    pub fn debounce(self) -> (Duration, u32) {
        match self {
            Kind::Fdm => (Duration::minutes(2), 1),
            Kind::Printer | Kind::Terminal | Kind::Scale => (Duration::zero(), 2),
            Kind::ZMissing | Kind::OnlineWaiting => (Duration::zero(), 1),
        }
    }
}

/// The transitions that send a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Raised,
    Reminder,
    Resolved,
}

impl Transition {
    /// The spelling of the log (`owner_alert_log.transition`).
    pub fn code(self) -> &'static str {
        match self {
            Transition::Raised => "raised",
            Transition::Reminder => "reminder",
            Transition::Resolved => "resolved",
        }
    }

    pub fn from_code(code: &str) -> Option<Transition> {
        [
            Transition::Raised,
            Transition::Reminder,
            Transition::Resolved,
        ]
        .into_iter()
        .find(|t| t.code() == code)
    }
}

/// Which printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Printer {
    Ticket,
    Kitchen,
}

/// What is wrong with a printer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterProblem {
    Offline,
    PaperOut,
    CoverOpen,
    Error,
}

/// The facts a message may carry: counts, channel names, a booking
/// date, the printers in trouble. Never a person, an amount or an order
/// content (V12-D14); the type has no field for them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Detail {
    /// `online_waiting`: how many orders wait.
    pub count: Option<i64>,
    /// `online_waiting`: the channels they come from.
    pub channels: Vec<String>,
    /// `online_waiting`: the wait after which they count, in minutes.
    pub wait_minutes: Option<i64>,
    /// `z_missing`: the booking date still open (`YYYY-MM-DD`).
    pub booking_date: Option<String>,
    /// `z_missing`: the hour by which the Z was due.
    pub deadline_hour: Option<i64>,
    /// `printer`: which printer has which problem.
    pub printers: Vec<(Printer, PrinterProblem)>,
    /// `scale`: it answers but refused the software check.
    pub scale_check_refused: bool,
}

impl Detail {
    /// The short machine text kept in the log (`owner_alert_log.detail`).
    pub fn to_log(&self) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(n) = self.count {
            parts.push(format!("count={n}"));
        }
        if self.scale_check_refused {
            parts.push("check=refused".to_string());
        }
        if !self.channels.is_empty() {
            parts.push(format!("channels={}", self.channels.join(",")));
        }
        if let Some(d) = &self.booking_date {
            parts.push(format!("booking_date={d}"));
        }
        for (printer, problem) in &self.printers {
            let printer = match printer {
                Printer::Ticket => "ticket",
                Printer::Kitchen => "kitchen",
            };
            let problem = match problem {
                PrinterProblem::Offline => "offline",
                PrinterProblem::PaperOut => "paper",
                PrinterProblem::CoverOpen => "cover",
                PrinterProblem::Error => "error",
            };
            parts.push(format!("{printer}={problem}"));
        }
        (!parts.is_empty()).then(|| parts.join(";"))
    }
}

/// A message to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub kind: Kind,
    pub transition: Transition,
    /// When the condition was first seen (the message says "since").
    pub since: DateTime<Utc>,
    pub detail: Detail,
}

/// What the log says about a kind, to pick up after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastRow {
    pub transition: Transition,
    /// Whether the message was actually sent.
    pub sent: bool,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct Open {
    since: DateTime<Utc>,
    last_notified: DateTime<Utc>,
    /// The owner was told (a raise or a reminder arrived); a resolve is
    /// only sent for an alert the owner knows about.
    announced: bool,
    detail: Detail,
}

#[derive(Debug, Clone, Default)]
struct KindState {
    pending_since: Option<DateTime<Utc>>,
    observations: u32,
    open: Option<Open>,
}

/// The state of every kind and the hourly window.
#[derive(Debug, Default)]
pub struct AlertBook {
    kinds: [KindState; 6],
    sent: VecDeque<DateTime<Utc>>,
}

impl AlertBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pick up an alert that was open before a restart: when the last
    /// row of the kind is a raise or a reminder younger than the
    /// reminder period (a day when reminders are off), the alert is
    /// open, so the restart does not raise it a second time. A resolve,
    /// or an older row, leaves the kind closed.
    pub fn restore(
        &mut self,
        kind: Kind,
        last: Option<LastRow>,
        reminder_minutes: i64,
        now: DateTime<Utc>,
    ) {
        let Some(last) = last else { return };
        if last.transition == Transition::Resolved {
            return;
        }
        let period = if reminder_minutes > 0 {
            reminder_minutes
        } else {
            RESTORE_WINDOW_NO_REMINDER_MINUTES
        };
        if now - last.at >= Duration::minutes(period) {
            return;
        }
        self.kinds[kind.index()].open = Some(Open {
            since: last.at,
            last_notified: last.at,
            announced: last.sent,
            detail: Detail::default(),
        });
    }

    /// Seed the hourly window with the messages sent before a restart.
    pub fn seed_sent(&mut self, times: impl IntoIterator<Item = DateTime<Utc>>) {
        self.sent.extend(times);
        self.sent.make_contiguous().sort();
    }

    /// Forget a kind that is switched off (or whose source went away):
    /// no resolve is sent for it.
    pub fn clear(&mut self, kind: Kind) {
        self.kinds[kind.index()] = KindState::default();
    }

    /// Whether `kind` has an alert open.
    pub fn is_open(&self, kind: Kind) -> bool {
        self.kinds[kind.index()].open.is_some()
    }

    /// One observation of `kind`: `Some(detail)` when the condition
    /// holds now, `None` when it does not. Returns the message to send,
    /// if any.
    pub fn observe(
        &mut self,
        kind: Kind,
        observed: Option<Detail>,
        reminder_minutes: i64,
        now: DateTime<Utc>,
    ) -> Option<Decision> {
        let state = &mut self.kinds[kind.index()];
        let Some(detail) = observed else {
            state.pending_since = None;
            state.observations = 0;
            let open = state.open.take()?;
            return open.announced.then_some(Decision {
                kind,
                transition: Transition::Resolved,
                since: open.since,
                detail: open.detail,
            });
        };

        let since = *state.pending_since.get_or_insert(now);
        state.observations = state.observations.saturating_add(1);

        if let Some(open) = &mut state.open {
            open.detail = detail.clone();
            if reminder_minutes > 0
                && now - open.last_notified >= Duration::minutes(reminder_minutes)
            {
                open.last_notified = now;
                open.announced = true;
                return Some(Decision {
                    kind,
                    transition: Transition::Reminder,
                    since: open.since,
                    detail,
                });
            }
            return None;
        }

        let (wait, observations) = kind.debounce();
        if now - since >= wait && state.observations >= observations {
            state.open = Some(Open {
                since,
                last_notified: now,
                announced: true,
                detail: detail.clone(),
            });
            return Some(Decision {
                kind,
                transition: Transition::Raised,
                since,
                detail,
            });
        }
        None
    }

    /// Whether one more message may be sent at `now` (the global cap);
    /// when it may, the message is counted.
    pub fn permit(&mut self, now: DateTime<Utc>) -> bool {
        let window = Duration::minutes(CAP_WINDOW_MINUTES);
        while self.sent.front().is_some_and(|t| now - *t >= window) {
            self.sent.pop_front();
        }
        if self.sent.len() >= GLOBAL_CAP {
            return false;
        }
        self.sent.push_back(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(minute: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 17, 12, 0, 0).unwrap() + Duration::minutes(minute)
    }

    fn at(minute: i64, second: i64) -> DateTime<Utc> {
        t(minute) + Duration::seconds(second)
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "observe takes an Option: on() reads better than Some(Detail::default()) in every test"
    )]
    fn on() -> Option<Detail> {
        Some(Detail::default())
    }

    #[test]
    fn the_fdm_is_raised_after_two_minutes_not_before() {
        let mut book = AlertBook::new();
        assert_eq!(book.observe(Kind::Fdm, on(), 60, t(0)), None);
        assert_eq!(book.observe(Kind::Fdm, on(), 60, at(1, 59)), None);
        let d = book.observe(Kind::Fdm, on(), 60, t(2)).unwrap();
        assert_eq!(d.transition, Transition::Raised);
        assert_eq!(d.kind, Kind::Fdm);
        assert_eq!(d.since, t(0), "since is when it was first seen");
        // Raised once only.
        assert_eq!(book.observe(Kind::Fdm, on(), 60, at(2, 30)), None);
        assert_eq!(book.observe(Kind::Fdm, on(), 60, t(3)), None);
    }

    #[test]
    fn a_blip_shorter_than_the_debounce_never_raises() {
        let mut book = AlertBook::new();
        assert_eq!(book.observe(Kind::Fdm, on(), 60, t(0)), None);
        assert_eq!(book.observe(Kind::Fdm, None, 60, at(1, 30)), None);
        // The clock starts again.
        assert_eq!(book.observe(Kind::Fdm, on(), 60, t(2)), None);
        assert_eq!(book.observe(Kind::Fdm, on(), 60, at(3, 59)), None);
        assert!(book.observe(Kind::Fdm, on(), 60, t(4)).is_some());
    }

    #[test]
    fn a_probe_kind_needs_two_consecutive_observations() {
        for kind in [Kind::Printer, Kind::Terminal, Kind::Scale] {
            let mut book = AlertBook::new();
            assert_eq!(book.observe(kind, on(), 60, t(0)), None, "{kind:?}");
            // One good probe in between starts the count again.
            assert_eq!(book.observe(kind, None, 60, t(5)), None);
            assert_eq!(book.observe(kind, on(), 60, t(10)), None);
            let d = book.observe(kind, on(), 60, t(15)).unwrap();
            assert_eq!(d.transition, Transition::Raised);
            assert_eq!(d.since, t(10));
        }
    }

    #[test]
    fn the_other_kinds_raise_at_the_first_observation() {
        for kind in [Kind::ZMissing, Kind::OnlineWaiting] {
            let mut book = AlertBook::new();
            let d = book.observe(kind, on(), 60, t(0)).unwrap();
            assert_eq!(d.transition, Transition::Raised, "{kind:?}");
        }
    }

    #[test]
    fn a_reminder_comes_every_period_while_it_lasts() {
        let mut book = AlertBook::new();
        book.observe(Kind::ZMissing, on(), 60, t(0)).unwrap();
        assert_eq!(book.observe(Kind::ZMissing, on(), 60, t(59)), None);
        let r = book.observe(Kind::ZMissing, on(), 60, t(60)).unwrap();
        assert_eq!(r.transition, Transition::Reminder);
        assert_eq!(r.since, t(0), "the reminder keeps the original since");
        assert_eq!(book.observe(Kind::ZMissing, on(), 60, t(119)), None);
        assert_eq!(
            book.observe(Kind::ZMissing, on(), 60, t(120))
                .unwrap()
                .transition,
            Transition::Reminder
        );
    }

    #[test]
    fn reminders_off_means_never() {
        let mut book = AlertBook::new();
        book.observe(Kind::ZMissing, on(), 0, t(0)).unwrap();
        assert_eq!(book.observe(Kind::ZMissing, on(), 0, t(100_000)), None);
    }

    #[test]
    fn a_resolve_is_sent_once_when_the_condition_clears() {
        let mut book = AlertBook::new();
        book.observe(Kind::OnlineWaiting, on(), 60, t(0)).unwrap();
        let d = book.observe(Kind::OnlineWaiting, None, 60, t(7)).unwrap();
        assert_eq!(d.transition, Transition::Resolved);
        assert_eq!(d.since, t(0));
        assert!(!book.is_open(Kind::OnlineWaiting));
        assert_eq!(book.observe(Kind::OnlineWaiting, None, 60, t(8)), None);
        // And it can be raised again.
        assert_eq!(
            book.observe(Kind::OnlineWaiting, on(), 60, t(9))
                .unwrap()
                .transition,
            Transition::Raised
        );
    }

    #[test]
    fn the_kinds_do_not_disturb_each_other() {
        let mut book = AlertBook::new();
        book.observe(Kind::ZMissing, on(), 60, t(0)).unwrap();
        assert_eq!(book.observe(Kind::OnlineWaiting, None, 60, t(1)), None);
        assert!(book.is_open(Kind::ZMissing));
        assert!(!book.is_open(Kind::OnlineWaiting));
    }

    #[test]
    fn the_detail_follows_the_condition() {
        let mut book = AlertBook::new();
        let two = Detail {
            count: Some(2),
            channels: vec!["webshop".into()],
            ..Detail::default()
        };
        book.observe(Kind::OnlineWaiting, Some(two), 60, t(0))
            .unwrap();
        let three = Detail {
            count: Some(3),
            channels: vec!["webshop".into(), "deliveroo".into()],
            ..Detail::default()
        };
        let d = book
            .observe(Kind::OnlineWaiting, Some(three.clone()), 60, t(60))
            .unwrap();
        assert_eq!(d.detail, three);
        let end = book.observe(Kind::OnlineWaiting, None, 60, t(61)).unwrap();
        assert_eq!(end.detail, three, "the resolve carries the last facts");
    }

    #[test]
    fn clear_forgets_a_kind_without_a_resolve() {
        let mut book = AlertBook::new();
        book.observe(Kind::ZMissing, on(), 60, t(0)).unwrap();
        book.clear(Kind::ZMissing);
        assert!(!book.is_open(Kind::ZMissing));
        assert_eq!(book.observe(Kind::ZMissing, None, 60, t(1)), None);
    }

    #[test]
    fn the_global_cap_is_twenty_per_rolling_hour() {
        let mut book = AlertBook::new();
        for i in 0..i64::try_from(GLOBAL_CAP).unwrap() {
            assert!(book.permit(at(0, i)), "message {i}");
        }
        assert!(!book.permit(at(0, 30)), "the 21st is dropped");
        assert!(!book.permit(at(59, 59)));
        // The first message leaves the window after one hour.
        assert!(book.permit(t(60)));
        assert!(!book.permit(t(60)), "the window is full again");
    }

    #[test]
    fn a_dropped_message_is_not_counted() {
        let mut book = AlertBook::new();
        for _ in 0..GLOBAL_CAP {
            book.permit(t(0));
        }
        for _ in 0..50 {
            assert!(!book.permit(t(1)));
        }
        assert!(book.permit(t(60)), "the dropped ones did not extend it");
    }

    #[test]
    fn the_window_is_seeded_after_a_restart() {
        let mut book = AlertBook::new();
        book.seed_sent((0..i64::try_from(GLOBAL_CAP).unwrap()).map(|i| at(0, i)));
        assert!(!book.permit(t(10)));
        assert!(book.permit(t(61)));
    }

    #[test]
    fn a_restart_does_not_raise_an_open_alert_twice() {
        let mut book = AlertBook::new();
        let last = LastRow {
            transition: Transition::Raised,
            sent: true,
            at: t(0),
        };
        book.restore(Kind::ZMissing, Some(last), 60, t(30));
        assert!(book.is_open(Kind::ZMissing));
        assert_eq!(book.observe(Kind::ZMissing, on(), 60, t(31)), None);
        // The reminder is due an hour after the raise.
        assert_eq!(
            book.observe(Kind::ZMissing, on(), 60, t(60))
                .unwrap()
                .transition,
            Transition::Reminder
        );
    }

    #[test]
    fn a_restored_alert_is_resolved_when_it_clears() {
        let mut book = AlertBook::new();
        let last = LastRow {
            transition: Transition::Reminder,
            sent: true,
            at: t(0),
        };
        book.restore(Kind::Fdm, Some(last), 60, t(10));
        let d = book.observe(Kind::Fdm, None, 60, t(11)).unwrap();
        assert_eq!(d.transition, Transition::Resolved);
    }

    #[test]
    fn an_old_or_resolved_row_does_not_reopen() {
        let mut book = AlertBook::new();
        let old = LastRow {
            transition: Transition::Raised,
            sent: true,
            at: t(0),
        };
        book.restore(Kind::Fdm, Some(old), 60, t(60));
        assert!(!book.is_open(Kind::Fdm), "older than the reminder period");
        let resolved = LastRow {
            transition: Transition::Resolved,
            sent: true,
            at: t(0),
        };
        book.restore(Kind::Printer, Some(resolved), 60, t(1));
        assert!(!book.is_open(Kind::Printer));
        book.restore(Kind::Terminal, None, 60, t(1));
        assert!(!book.is_open(Kind::Terminal));
    }

    #[test]
    fn a_restored_raise_that_never_arrived_is_closed_silently() {
        let mut book = AlertBook::new();
        let failed = LastRow {
            transition: Transition::Raised,
            sent: false,
            at: t(0),
        };
        book.restore(Kind::Fdm, Some(failed), 60, t(5));
        assert!(book.is_open(Kind::Fdm));
        assert_eq!(
            book.observe(Kind::Fdm, None, 60, t(6)),
            None,
            "the owner never heard of it, nothing to resolve"
        );
    }

    #[test]
    fn with_reminders_off_a_restore_still_remembers_a_day() {
        let mut book = AlertBook::new();
        let last = LastRow {
            transition: Transition::Raised,
            sent: true,
            at: t(0),
        };
        book.restore(Kind::Fdm, Some(last), 0, t(23 * 60));
        assert!(book.is_open(Kind::Fdm));
        let mut later = AlertBook::new();
        later.restore(Kind::Fdm, Some(last), 0, t(25 * 60));
        assert!(!later.is_open(Kind::Fdm));
    }

    #[test]
    fn kinds_and_transitions_round_trip_through_their_codes() {
        for kind in Kind::ALL {
            assert_eq!(Kind::from_code(kind.code()), Some(kind));
        }
        for tr in [
            Transition::Raised,
            Transition::Reminder,
            Transition::Resolved,
        ] {
            assert_eq!(Transition::from_code(tr.code()), Some(tr));
        }
        assert_eq!(Kind::from_code("test"), None);
        assert_eq!(Transition::from_code("test"), None);
    }

    #[test]
    fn the_log_detail_has_counts_channels_dates_and_devices_only() {
        let d = Detail {
            count: Some(2),
            channels: vec!["webshop".into(), "deliveroo".into()],
            booking_date: Some("2026-10-16".into()),
            printers: vec![(Printer::Kitchen, PrinterProblem::PaperOut)],
            ..Detail::default()
        };
        assert_eq!(
            d.to_log().as_deref(),
            Some("count=2;channels=webshop,deliveroo;booking_date=2026-10-16;kitchen=paper")
        );
        assert_eq!(Detail::default().to_log(), None);
    }
}
