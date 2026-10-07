//! Who may do what (`PROFILS_UTILISATEURS_FR.md` §2–3).
//!
//! The logged-in operator is held by the backend (`AppState::session`,
//! set by `login`, cleared by `logout`): the UI cannot act for anyone
//! else. Every command that signs a fiscal event, changes a setting or
//! manages users checks the operator's role here before doing anything,
//! so a refused command touches neither the counters nor the database.
//!
//! | Access    | Roles                  | Commands                                        |
//! |-----------|------------------------|-------------------------------------------------|
//! | `Staff`   | admin, cashier, waiter | clock in/out, orders and lines, table order (P) |
//! | `Cashier` | admin, cashier         | pre-bill (P), sale (N), missing original print  |
//! | `Admin`   | admin                  | refund, copy (C), cash (F), reports (R), training mode, users, backup |

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pos_core::user::{User, UserRole};

use crate::commands::errors;
use crate::state::AppState;

pub const ERR_NOT_LOGGED_IN: &str = errors::NOT_LOGGED_IN;
pub const ERR_FORBIDDEN: &str = errors::FORBIDDEN;
pub const ERR_NOT_THE_OPERATOR: &str = errors::NOT_THE_OPERATOR;

/// The least a command requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Staff,
    Cashier,
    Admin,
}

impl Access {
    pub fn allows(self, role: UserRole) -> bool {
        match self {
            Access::Staff => true,
            Access::Cashier => matches!(role, UserRole::Admin | UserRole::Cashier),
            Access::Admin => role == UserRole::Admin,
        }
    }
}

/// The logged-in operator, if their role allows `access`.
pub fn operator(state: &AppState, access: Access) -> Result<User, String> {
    let user = state
        .session
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or_else(|| ERR_NOT_LOGGED_IN.to_string())?;
    if !access.allows(user.role) {
        return Err(ERR_FORBIDDEN.to_string());
    }
    Ok(user)
}

/// [`operator`], for a command that names its operator: `insz` must
/// be the logged-in one (a UI can never sign in someone else's name).
pub fn operator_named(state: &AppState, insz: &str, access: Access) -> Result<User, String> {
    let user = operator(state, access)?;
    if user.insz != insz {
        return Err(ERR_NOT_THE_OPERATOR.to_string());
    }
    Ok(user)
}

/// [`operator_named`], confirmed by the operator's PIN: for an action
/// a tap alone must not take (remote support, a sale closed unpaid).
pub fn confirmed_operator(
    state: &AppState,
    insz: &str,
    pin: &str,
    access: Access,
) -> Result<User, String> {
    let user = operator_named(state, insz, access)?;
    let hash = {
        let db = state.db.lock().map_err(errors::internal)?;
        pos_db::queries::users::get_pin_hash(&db, insz)
            .map_err(errors::internal)?
            .ok_or_else(|| errors::PIN_SETUP_REQUIRED.to_string())?
    };
    verify_pin(state, insz, &hash, pin)?;
    Ok(user)
}

/// Wrong PINs before the first lockout.
const FREE_FAILURES: u32 = 5;
const FIRST_LOCKOUT: Duration = Duration::from_secs(30);
const MAX_LOCKOUT: Duration = Duration::from_secs(15 * 60);

/// Wrong PIN attempts per user, in memory (a restart clears them).
#[derive(Default)]
pub struct PinThrottle(Mutex<HashMap<String, Attempts>>);

#[derive(Debug, Clone, Copy, Default)]
struct Attempts {
    failures: u32,
    locked_until: Option<Instant>,
}

/// Seconds left on a lockout, `None` when a try is allowed.
fn seconds_locked(locked_until: Option<Instant>, now: Instant) -> Option<u64> {
    let left = locked_until?.checked_duration_since(now)?;
    (!left.is_zero()).then(|| left.as_secs() + u64::from(left.subsec_nanos() > 0))
}

/// The lockout after `failures` wrong PINs in a row: none for the
/// first five, then 30 s doubling up to 15 min.
fn lockout_after(failures: u32) -> Option<Duration> {
    let extra = failures.checked_sub(FREE_FAILURES)?;
    let factor = 1u32.checked_shl(extra).unwrap_or(u32::MAX);
    Some(FIRST_LOCKOUT.saturating_mul(factor).min(MAX_LOCKOUT))
}

/// Check `pin` against `hash` for `insz`, refusing any try (even the
/// right PIN) while the user is locked out. Call without the DB lock.
pub fn verify_pin(state: &AppState, insz: &str, hash: &str, pin: &str) -> Result<(), String> {
    let locked = |seconds: u64| errors::coded(errors::PIN_LOCKED, &[("seconds", &seconds)]);
    {
        let table = state.pin_throttle.0.lock().map_err(errors::internal)?;
        let until = table.get(insz).and_then(|a| a.locked_until);
        if let Some(seconds) = seconds_locked(until, Instant::now()) {
            return Err(locked(seconds));
        }
    }

    let right = bcrypt::verify(pin, hash).unwrap_or(false);

    let mut table = state.pin_throttle.0.lock().map_err(errors::internal)?;
    if right {
        table.remove(insz);
        return Ok(());
    }
    let attempts = table.entry(insz.to_string()).or_default();
    attempts.failures = attempts.failures.saturating_add(1);
    let now = Instant::now();
    attempts.locked_until = lockout_after(attempts.failures).map(|d| now + d);
    Err(match seconds_locked(attempts.locked_until, now) {
        Some(seconds) => locked(seconds),
        None => errors::PIN_WRONG.to_string(),
    })
}

/// Open a session for `user` (after `login` checked them). A login
/// restarts the idle time (PD35).
pub fn open_session(state: &AppState, user: &User) -> Result<(), String> {
    *state.session.lock().map_err(|e| e.to_string())? = Some(user.clone());
    state.session_rules.touch();
    Ok(())
}

/// End the session: every command needing an operator is refused
/// until the next login.
pub fn close_session(state: &AppState) -> Result<(), String> {
    *state.session.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockouts_start_after_five_failures_and_double_to_a_cap() {
        for failures in 0..5 {
            assert_eq!(lockout_after(failures), None);
        }
        assert_eq!(lockout_after(5), Some(Duration::from_secs(30)));
        assert_eq!(lockout_after(6), Some(Duration::from_secs(60)));
        assert_eq!(lockout_after(9), Some(Duration::from_secs(480)));
        assert_eq!(lockout_after(10), Some(MAX_LOCKOUT));
        assert_eq!(lockout_after(u32::MAX), Some(MAX_LOCKOUT));
    }

    #[test]
    fn seconds_locked_rounds_up_and_ends() {
        let now = Instant::now();
        assert_eq!(seconds_locked(None, now), None);
        assert_eq!(seconds_locked(Some(now), now), None);
        assert_eq!(
            seconds_locked(Some(now + Duration::from_millis(1)), now),
            Some(1)
        );
        assert_eq!(
            seconds_locked(Some(now + Duration::from_secs(30)), now),
            Some(30)
        );
        assert_eq!(
            seconds_locked(Some(now), now + Duration::from_secs(1)),
            None
        );
    }

    #[test]
    fn the_role_matrix() {
        use UserRole::*;
        let expect = [
            (Access::Staff, [true, true, true]),
            (Access::Cashier, [true, true, false]),
            (Access::Admin, [true, false, false]),
        ];
        for (access, allowed) in expect {
            for (role, ok) in [Admin, Cashier, Waiter].into_iter().zip(allowed) {
                assert_eq!(access.allows(role), ok, "{access:?} / {role:?}");
            }
        }
    }
}
