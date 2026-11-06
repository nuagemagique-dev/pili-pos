//! One manager PIN per order for discounts (UX audit P8).
//!
//! A manager who gave a discount or an offered item or line on an order,
//! confirmed by their PIN, may give the next ones on the same order without
//! typing the PIN again for [`DEFAULT_LIFETIME`] (two minutes, counted from
//! the PIN, never extended by a reuse). The engine keeps the grant, in
//! memory only: a restart forgets it, which is the safe side.
//!
//! The grant is bound to three things: the logged-in user, the order, and an
//! expiry. It is dropped by:
//! - a lock (manual or idle), a logout, the end of a shift and every login
//!   ([`crate::commands::access::open_session`] and `close_session`);
//! - an order switch ([`crate::commands::session::do_set_active_order`] to
//!   another order or to none, and a [`LeaveReason`] other than
//!   `SendToKitchen`);
//! - the sale of the order (`checkout::sell`).
//!
//! Only the discount commands (`discount::do_apply_discount` and
//! `do_remove_discount`) ever ask the grant. "Client parti sans payer"
//! (`unpaid`), the payment correction, a refund, the remote support and the
//! settings keep asking the PIN every time: they call
//! `access::confirmed_operator` and never see the grant. Every discount that
//! used the grant is written to the audit trail `till_discounts` with
//! `pin_reused` set.
//!
//! [`LeaveReason`]: crate::commands::session::LeaveReason

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::time::Instant;

/// How long a PIN confirmation covers the next discounts of the order.
pub const DEFAULT_LIFETIME: Duration = Duration::from_mins(2);

/// What a PIN confirmation covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinGrant {
    insz: String,
    order_id: i64,
    expires_at: Instant,
}

impl PinGrant {
    /// A grant for `insz` on `order_id` that runs out `lifetime` after `now`.
    pub fn new(insz: &str, order_id: i64, now: Instant, lifetime: Duration) -> Self {
        Self {
            insz: insz.to_string(),
            order_id,
            expires_at: now + lifetime,
        }
    }

    /// Whether it covers `insz` on `order_id` at `now`: the same user, the
    /// same order, not yet run out (the very instant of the expiry is out).
    pub fn covers(&self, insz: &str, order_id: i64, now: Instant) -> bool {
        self.insz == insz && self.order_id == order_id && now < self.expires_at
    }

    /// The time left at `now`, `None` once it ran out.
    pub fn remaining(&self, now: Instant) -> Option<Duration> {
        self.expires_at
            .checked_duration_since(now)
            .filter(|d| !d.is_zero())
    }
}

/// The grant of the engine: at most one, the last PIN confirmation.
#[derive(Debug, Default)]
pub struct PinGrants(Mutex<Option<PinGrant>>);

impl PinGrants {
    /// A poisoned lock still answers: the grant is only ever replaced or
    /// dropped, never half written.
    fn slot(&self) -> MutexGuard<'_, Option<PinGrant>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Remember `grant`, replacing any earlier one.
    pub fn give(&self, grant: PinGrant) {
        *self.slot() = Some(grant);
    }

    /// Whether the grant covers `insz` on `order_id` at `now`. An expired
    /// grant is dropped on the way.
    pub fn covers(&self, insz: &str, order_id: i64, now: Instant) -> bool {
        let mut slot = self.slot();
        match slot.as_ref() {
            Some(grant) if grant.covers(insz, order_id, now) => true,
            Some(grant) if grant.remaining(now).is_none() => {
                *slot = None;
                false
            }
            _ => false,
        }
    }

    /// The time the grant has left for `insz` on `order_id` at `now`.
    pub fn remaining(&self, insz: &str, order_id: i64, now: Instant) -> Option<Duration> {
        self.slot()
            .as_ref()
            .filter(|g| g.covers(insz, order_id, now))
            .and_then(|g| g.remaining(now))
    }

    /// Drop the grant, whoever and whatever order it covers.
    pub fn clear(&self) {
        *self.slot() = None;
    }

    /// Drop the grant unless it covers `order_id` (`None`: drop it): the
    /// screen now shows another order, or none.
    pub fn clear_unless_order(&self, order_id: Option<i64>) {
        let mut slot = self.slot();
        if slot.as_ref().is_some_and(|g| Some(g.order_id) != order_id) {
            *slot = None;
        }
    }

    /// Drop the grant when it covers `order_id` (the order was left or sold).
    pub fn clear_for_order(&self, order_id: i64) {
        let mut slot = self.slot();
        if slot.as_ref().is_some_and(|g| g.order_id == order_id) {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANAGER: &str = "12345678958";
    const OTHER: &str = "84022899837";

    /// A fake clock: instants are plain values, `t0 + n` is "n later".
    fn grant(t0: Instant) -> PinGrant {
        PinGrant::new(MANAGER, 7, t0, DEFAULT_LIFETIME)
    }

    #[test]
    fn it_lasts_two_minutes_from_the_pin_and_not_a_moment_more() {
        let t0 = Instant::now();
        let g = grant(t0);
        assert!(g.covers(MANAGER, 7, t0));
        assert!(g.covers(MANAGER, 7, t0 + Duration::from_secs(119)));
        assert!(g.covers(MANAGER, 7, t0 + Duration::from_millis(119_999)));
        assert!(!g.covers(MANAGER, 7, t0 + Duration::from_secs(120)));
        assert!(!g.covers(MANAGER, 7, t0 + Duration::from_secs(121)));
        assert_eq!(
            g.remaining(t0 + Duration::from_secs(90)),
            Some(Duration::from_secs(30))
        );
        assert_eq!(g.remaining(t0 + Duration::from_secs(120)), None);
    }

    #[test]
    fn it_covers_one_user_on_one_order() {
        let t0 = Instant::now();
        let g = grant(t0);
        assert!(!g.covers(OTHER, 7, t0), "another user");
        assert!(!g.covers(MANAGER, 8, t0), "another order");
    }

    #[test]
    fn an_expired_grant_is_dropped_when_asked() {
        let t0 = Instant::now();
        let grants = PinGrants::default();
        grants.give(grant(t0));
        assert!(grants.covers(MANAGER, 7, t0 + Duration::from_secs(60)));
        assert!(!grants.covers(MANAGER, 7, t0 + Duration::from_secs(121)));
        // It is gone, not only out of date: asking again at an earlier
        // instant (a clock that went back) still finds nothing.
        assert!(!grants.covers(MANAGER, 7, t0));
    }

    #[test]
    fn a_new_confirmation_replaces_the_grant() {
        let t0 = Instant::now();
        let grants = PinGrants::default();
        grants.give(grant(t0));
        grants.give(PinGrant::new(MANAGER, 8, t0, DEFAULT_LIFETIME));
        assert!(!grants.covers(MANAGER, 7, t0), "the first order lost it");
        assert!(grants.covers(MANAGER, 8, t0));
    }

    #[test]
    fn an_order_switch_keeps_the_grant_only_for_the_same_order() {
        let t0 = Instant::now();
        let grants = PinGrants::default();
        grants.give(grant(t0));
        grants.clear_unless_order(Some(7));
        assert!(grants.covers(MANAGER, 7, t0), "the same order stays");
        grants.clear_unless_order(Some(8));
        assert!(!grants.covers(MANAGER, 7, t0), "another order drops it");
        grants.give(grant(t0));
        grants.clear_unless_order(None);
        assert!(!grants.covers(MANAGER, 7, t0), "no order drops it");
    }

    #[test]
    fn leaving_or_selling_an_order_drops_only_its_grant() {
        let t0 = Instant::now();
        let grants = PinGrants::default();
        grants.give(grant(t0));
        grants.clear_for_order(8);
        assert!(grants.covers(MANAGER, 7, t0));
        grants.clear_for_order(7);
        assert!(!grants.covers(MANAGER, 7, t0));
    }

    #[test]
    fn clear_drops_any_grant() {
        let t0 = Instant::now();
        let grants = PinGrants::default();
        grants.give(grant(t0));
        grants.clear();
        assert!(grants.remaining(MANAGER, 7, t0).is_none());
    }
}
