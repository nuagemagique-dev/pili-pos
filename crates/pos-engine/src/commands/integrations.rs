//! The installation's switch for the `HubRise` integration. The code
//! ships in the certified binary; the switch says whether it runs.
//!
//! Off: the poller and the outbox idle, no till sale is sent, and the
//! commands that talk to `HubRise` or take a new online order refuse
//! ([`require_hubrise`]). What settles an online order already signed
//! (collecting it, refunding it, closing it) never needs the switch: a
//! fiscal obligation is never stranded.
//!
//! The starting value is `hubrise_enabled` in `pos.json`; an admin can
//! change it here. Every change, either way, is logged
//! (`integration_switch_log`, append-only).

use pos_db::queries::integrations::{self, SwitchChange};
use serde::Serialize;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// Refuse when the integration is switched off.
pub fn require_hubrise(state: &AppState) -> Result<(), String> {
    if state.hubrise_switch.is_on() {
        Ok(())
    } else {
        Err(errors::hubrise::DISABLED.to_string())
    }
}

/// Online orders not settled yet: waiting for an answer, being signed,
/// awaiting payment, or cancelled with a refund or a closing to make.
pub(crate) fn unsettled(state: &AppState) -> Result<i64, String> {
    let db = state.db.lock().map_err(errors::internal)?;
    unsettled_on(&db)
}

/// [`unsettled`] on a connection the caller already holds, so the check
/// and the change it guards happen under one lock.
pub(crate) fn unsettled_on(db: &rusqlite::Connection) -> Result<i64, String> {
    db.query_row(
        "SELECT COUNT(*) FROM webshop_orders
         WHERE fiscal_state IN ('pending', 'signing', 'signed_p', 'refund_pending', 'cancel_pending')",
        [],
        |r| r.get(0),
    )
    .map_err(errors::internal)
}

/// Run `f` inside `BEGIN IMMEDIATE`: the SQLite write lock is taken
/// first, so the `HubRise` poller (its own connection, `poller_db`) cannot
/// insert an order between the count `f` makes and the change it guards.
/// Rolled back when `f` fails.
pub(crate) fn with_write_lock<T>(
    db: &rusqlite::Connection,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    db.execute_batch("BEGIN IMMEDIATE")
        .map_err(errors::internal)?;
    match f(db) {
        Ok(v) => match db.execute_batch("COMMIT") {
            Ok(()) => Ok(v),
            Err(e) => {
                rollback(db);
                Err(errors::internal(e))
            }
        },
        Err(e) => {
            rollback(db);
            Err(e)
        }
    }
}

/// Roll back after a failure; the failure is what the caller returns, so
/// a rollback that fails too is only logged.
fn rollback(db: &rusqlite::Connection) {
    if let Err(e) = db.execute_batch("ROLLBACK") {
        tracing::warn!(%e, "rollback after a failed write failed");
    }
}

#[derive(Debug, Serialize)]
pub struct Integrations {
    pub hubrise_enabled: bool,
    /// Online orders still to settle: the board stays reachable for
    /// them even when the integration is off.
    pub online_orders_open: i64,
    /// The latest changes first.
    pub hubrise_log: Vec<SwitchChange>,
}

pub fn do_get_integrations(state: &AppState) -> Result<Integrations, String> {
    let online_orders_open = unsettled(state)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(Integrations {
        hubrise_enabled: state.hubrise_switch.is_on(),
        online_orders_open,
        hubrise_log: integrations::hubrise_log(&db, 20).map_err(errors::internal)?,
    })
}

/// Switch the integration on or off. Admins only; off waits until every
/// online order is settled.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "IPC/FFI contract: mirrors the bool argument of the Tauri command and the uniffi export"
)]
pub fn do_set_hubrise_enabled(state: &AppState, enabled: bool) -> Result<Integrations, String> {
    let admin = access::operator(state, Access::Admin)?;
    {
        // Count and switch under the SQLite write lock: no order can
        // open in between, not even one the poller inserts.
        let db = state.db.lock().map_err(errors::internal)?;
        with_write_lock(&db, |db| {
            if !enabled {
                let open = unsettled_on(db)?;
                if open > 0 {
                    return Err(errors::coded(
                        errors::hubrise::ONLINE_ORDERS_OPEN,
                        &[("count", &open)],
                    ));
                }
            }
            integrations::set_hubrise(
                db,
                pos_db::queries::integrations::State::from(enabled),
                &admin.insz,
            )
            .map_err(errors::internal)
        })?;
    }
    state.hubrise_switch.set(enabled);
    if enabled {
        // The poller starts over (callback, catalog, what arrived).
        state.hubrise_poll_now.notify_one();
    }
    do_get_integrations(state)
}
