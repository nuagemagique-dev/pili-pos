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
    let db = state.db.lock().map_err(errors::internal)?;
    let hash = pos_db::queries::users::get_pin_hash(&db, insz)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::PIN_SETUP_REQUIRED.to_string())?;
    if !bcrypt::verify(pin, &hash).unwrap_or(false) {
        return Err(errors::PIN_WRONG.to_string());
    }
    Ok(user)
}

/// Open a session for `user` (after `login` checked them).
pub fn open_session(state: &AppState, user: &User) -> Result<(), String> {
    *state.session.lock().map_err(|e| e.to_string())? = Some(user.clone());
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
