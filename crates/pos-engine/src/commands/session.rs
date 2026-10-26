//! The order the operator is working on and the screen lock.
//!
//! Two rules used to live in the Vue UI and now belong to the engine:
//!
//! - **Leaving an order registers it.** When the operator leaves the
//!   order or checkout screen, what was ordered is registered with the
//!   FDM (event P, POS description v2.2 §1.3). The router guard in
//!   `ui/src/router/index.ts` and `registerPending` in
//!   `ui/src/stores/order.ts` did it; [`do_leave_order`] replaces them.
//! - **Locking registers the active order first.** The lock button in
//!   `ui/src/components/AppHeader.vue` navigated to the login screen
//!   (so the guard signed the P while the session was still open) and
//!   then logged out. [`do_lock_session`] does both in that order, and
//!   the idle lock reaches the same code ([`lock`]).
//!
//! - **Login queues the clock-in.** A successful login queues a
//!   [`PresenceJob`] that the presence worker signs in the background
//!   (planner decision PD34); the login never waits for the FDM.
//!
//! - **The engine owns the idle lock.** The UI used to count five
//!   minutes without a touch or key press; the engine now does (planner
//!   decision PD35, `EngineTimers::idle_lock`). The shell forwards the
//!   operator's activity with [`do_user_activity`], every login restarts
//!   the time, and the idle loop of `background.rs` runs [`lock`] with
//!   [`LockReason::Idle`] when it runs out.
//!
//! The engine knows the active order through [`do_set_active_order`]:
//! the shell tells it which order the screen shows, and the lock needs
//! no help from the UI to register it.

use std::sync::Mutex;

use tokio::sync::Notify;
use tokio::sync::futures::Notified;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::Instant;

use serde::{Deserialize, Serialize};

use pos_core::order::{OrderStatus, OrderType};

use crate::commands::access::{self, Access};
use crate::commands::bar_slip::SlipOutcome;
use crate::commands::errors;
use crate::events::EngineEvent;
use crate::state::AppState;

/// Why the operator leaves an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaveReason {
    /// To another screen.
    Navigate,
    /// To another order (for instance a takeaway started from a table).
    SwitchOrder,
    /// The order is sent to the kitchen and stays on screen.
    SendToKitchen,
    /// The operator locks the screen.
    Lock,
    /// The screen locks itself after the idle delay.
    Idle,
}

/// Why the session was locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockReason {
    /// The operator pressed the lock button.
    Manual,
    /// The idle delay ran out.
    Idle,
}

/// The `session-locked` event: the screen goes to the login screen.
/// `order_error` is why the active order could not be registered, if
/// that failed; the lock happened anyway, and the entries stay pending
/// until the next registration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLocked {
    pub reason: LockReason,
    pub order_error: Option<String>,
}

/// What [`do_lock_session`] answers: the same `order_error`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LockOutcome {
    pub order_error: Option<String>,
}

/// Work for the presence worker (`background.rs`), queued by the
/// commands that open a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceJob {
    /// Clock this employee in unless they are already on shift
    /// ([`crate::commands::social::clock_in_if_off_shift`]).
    ClockIn(String),
}

/// The engine's session rules: the order on screen, the presence queue
/// and the idle clock. Later rules add their own fields here.
#[derive(Debug)]
pub struct SessionRules {
    active_order: Mutex<Option<i64>>,
    /// The last user activity (or login): the idle lock counts from it.
    last_activity: Mutex<Instant>,
    /// Woken by every [`SessionRules::touch`], so the idle loop re-arms.
    activity: Notify,
    /// Notified when a session opens or closes, so the FDM monitor loop
    /// starts or stops polling at once. A stored permit: a change while
    /// the loop is busy is not lost.
    session_changed: Notify,
    presence_tx: UnboundedSender<PresenceJob>,
    presence_rx: Mutex<Option<UnboundedReceiver<PresenceJob>>>,
}

impl Default for SessionRules {
    fn default() -> Self {
        let (presence_tx, presence_rx) = unbounded_channel();
        Self {
            active_order: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
            activity: Notify::new(),
            session_changed: Notify::new(),
            presence_tx,
            presence_rx: Mutex::new(Some(presence_rx)),
        }
    }
}

impl SessionRules {
    /// The order the screen shows, if the shell told the engine.
    pub fn active_order(&self) -> Option<i64> {
        self.active_order.lock().ok().and_then(|g| *g)
    }

    /// Record user activity now: the idle time restarts and the idle
    /// loop is woken to re-arm. Cheap: the shell calls it up to once per
    /// second.
    pub fn touch(&self) {
        if let Ok(mut last) = self.last_activity.lock() {
            *last = Instant::now();
        }
        self.activity.notify_waiters();
    }

    /// When the last activity happened (the start of the engine, if
    /// there was none).
    pub fn last_activity(&self) -> Instant {
        self.last_activity
            .lock()
            .map_or_else(|e| *e.into_inner(), |g| *g)
    }

    /// Completes at the next [`SessionRules::touch`]. The idle loop
    /// enables it before it reads the clock, so no touch is missed.
    pub(crate) fn next_activity(&self) -> Notified<'_> {
        self.activity.notified()
    }

    /// Tell the FDM monitor that a session opened or closed.
    pub(crate) fn notify_session_changed(&self) {
        self.session_changed.notify_one();
    }

    /// Completes when a session opens or closes.
    pub(crate) fn session_changed(&self) -> Notified<'_> {
        self.session_changed.notified()
    }

    /// Queue a clock-in for `insz`. Never blocks: the presence worker
    /// signs it later. A closed queue is logged and ignored.
    pub fn queue_clock_in(&self, insz: &str) {
        if self
            .presence_tx
            .send(PresenceJob::ClockIn(insz.to_string()))
            .is_err()
        {
            tracing::warn!("presence queue closed: clock-in not queued");
        }
    }

    /// The receiving end of the presence queue, once: the presence
    /// worker takes it. `None` when it was already taken.
    pub(crate) fn take_presence_jobs(&self) -> Option<UnboundedReceiver<PresenceJob>> {
        self.presence_rx.lock().ok().and_then(|mut rx| rx.take())
    }

    fn set(&self, order_id: Option<i64>) -> Result<(), String> {
        *self.active_order.lock().map_err(|e| e.to_string())? = order_id;
        Ok(())
    }

    /// Forget the active order when it is `order_id`.
    fn clear_if(&self, order_id: i64) -> Result<(), String> {
        let mut active = self.active_order.lock().map_err(|e| e.to_string())?;
        if *active == Some(order_id) {
            *active = None;
        }
        Ok(())
    }
}

/// Tell the engine which order the screen shows (`None`: none). The
/// order must exist. Needs a logged-in operator.
pub fn do_set_active_order(state: &AppState, order_id: Option<i64>) -> Result<(), String> {
    access::operator(state, Access::Staff)?;
    if let Some(id) = order_id {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        pos_db::queries::orders::get_order(&db, id).map_err(|e| e.to_string())?;
    }
    state.session_rules.set(order_id)
}

/// The operator leaves an order: register what was ordered (event P,
/// POS description v2.2 §1.3), as the router guard and `registerPending`
/// did in the UI.
///
/// A takeaway is a direct sale (FAQ Q205) and an order that is not open
/// has nothing left to register: both answer `NothingToPrint` without
/// calling the FDM. Otherwise it is `do_sign_table_order`, a no-op when
/// nothing changed. Once registered (or when there was nothing to
/// register) the order stops being the active one, unless the reason is
/// `SendToKitchen`: the order stays on screen. On an error the active
/// order is kept and the entries stay pending, retried at the next
/// registration.
pub async fn do_leave_order(
    state: &AppState,
    order_id: i64,
    reason: LeaveReason,
) -> Result<SlipOutcome, String> {
    access::operator(state, Access::Staff)?;
    let order = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        pos_db::queries::orders::get_order(&db, order_id).map_err(|e| e.to_string())?
    };
    tracing::info!(order_id, ?reason, "leaving an order");
    let outcome = if order.order_type == OrderType::Takeaway || order.status != OrderStatus::Open {
        SlipOutcome::NothingToPrint
    } else {
        crate::commands::tables::do_sign_table_order(state, order_id).await?
    };
    if reason != LeaveReason::SendToKitchen {
        // Opening a table or the takeaway tab creates the order: leaving
        // it with nothing on it must not leave an open order behind (it
        // would block the SPF export, the update and the menu refresh).
        // An order with a line, a signed event or a payment is kept.
        let voided = {
            let db = state.db.lock().map_err(errors::internal)?;
            pos_db::queries::orders::void_if_empty(&db, order_id).map_err(errors::internal)?
        };
        if voided {
            tracing::info!(order_id, "empty order voided on leaving");
        }
        state.session_rules.clear_if(order_id)?;
    }
    Ok(outcome)
}

/// Lock the session: register the active order while the operator is
/// still logged in, close the session (the operator stays on shift, no
/// clock-out) and tell the shell with `session-locked`. A failed
/// registration is reported in `order_error` and never stops the lock.
/// Nothing happens when no session is open.
pub(crate) async fn lock(state: &AppState, reason: LockReason) -> Result<LockOutcome, String> {
    let open = state.session.lock().map_err(|e| e.to_string())?.is_some();
    if !open {
        return Ok(LockOutcome { order_error: None });
    }
    let mut order_error = None;
    if let Some(id) = state.session_rules.active_order() {
        let leave = match reason {
            LockReason::Manual => LeaveReason::Lock,
            LockReason::Idle => LeaveReason::Idle,
        };
        if let Err(e) = do_leave_order(state, id, leave).await {
            tracing::warn!(order_id = id, error = %e, "active order not registered at lock");
            order_error = Some(e);
        }
    }
    state.session_rules.set(None)?;
    access::close_session(state)?;
    state.events.emit(EngineEvent::SessionLocked(SessionLocked {
        reason,
        order_error: order_error.clone(),
    }));
    Ok(LockOutcome { order_error })
}

/// The operator is active (a touch, a key press): restart the idle
/// time (PD35). Needs no session and no database; cheap enough for the
/// shell to call once per second.
pub fn do_user_activity(state: &AppState) -> Result<(), String> {
    state.session_rules.touch();
    Ok(())
}

/// The operator locks the screen (replaces the lock button's
/// "navigate to login, then log out" in `AppHeader.vue`).
pub async fn do_lock_session(state: &AppState) -> Result<LockOutcome, String> {
    lock(state, LockReason::Manual).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_ins_come_out_in_arrival_order_and_the_queue_is_taken_once() {
        let rules = SessionRules::default();
        rules.queue_clock_in("12345678901");
        rules.queue_clock_in("98765432109");

        let mut jobs = rules.take_presence_jobs().expect("first take");
        assert_eq!(
            jobs.try_recv().unwrap(),
            PresenceJob::ClockIn("12345678901".to_string())
        );
        assert_eq!(
            jobs.try_recv().unwrap(),
            PresenceJob::ClockIn("98765432109".to_string())
        );
        assert!(jobs.try_recv().is_err());
        assert!(rules.take_presence_jobs().is_none(), "taken once");
    }

    #[test]
    fn queueing_into_a_closed_queue_is_ignored() {
        let rules = SessionRules::default();
        drop(rules.take_presence_jobs());
        rules.queue_clock_in("12345678901");
    }
}
