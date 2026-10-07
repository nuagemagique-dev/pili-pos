use pos_core::user::{SYSTEM_INSZ, User, UserRole};
use serde::Deserialize;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// Login with INSZ + optional PIN. If the user has a PIN set (pin_hash
/// is not NULL), the caller MUST provide the correct PIN or login is
/// rejected. A manager without a PIN is refused: on first boot (no
/// manager has a PIN yet) with [`ERR_PIN_SETUP_REQUIRED`], and they
/// choose one through [`do_login_with_new_pin`]; afterwards with
/// [`ERR_FIRST_PIN_NEEDS_MANAGER`], since another manager sets it.
/// Other roles without a PIN log in, and the frontend warns them to set
/// one. The system identity never logs in.
pub fn do_login(state: &AppState, insz: String, pin: Option<String>) -> Result<User, String> {
    if !pos_core::user::validate_insz(&insz) {
        return Err(errors::INSZ_INVALID.to_string());
    }
    if insz == SYSTEM_INSZ {
        return Err(ERR_SYSTEM_USER.to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = pos_db::queries::users::get_user(&db, &insz)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| errors::USER_UNKNOWN.to_string())?;

    if user.role == UserRole::Admin && !user.has_pin {
        let claimed = pos_db::queries::users::any_admin_has_pin(&db).map_err(|e| e.to_string())?;
        return Err(if claimed {
            ERR_FIRST_PIN_NEEDS_MANAGER
        } else {
            ERR_PIN_SETUP_REQUIRED
        }
        .to_string());
    }

    // PIN verification: only when the user has a stored hash.
    let stored_hash = if user.has_pin {
        Some(
            pos_db::queries::users::get_pin_hash(&db, &insz)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| errors::internal("PIN hash missing"))?,
        )
    } else {
        None
    };
    drop(db);
    if let Some(hash) = stored_hash {
        let provided = pin.ok_or_else(|| errors::PIN_REQUIRED.to_string())?;
        access::verify_pin(state, &insz, &hash, &provided)?;
    }

    access::open_session(state, &user)?;
    Ok(user)
}

/// The system identity (`SYSTEM_INSZ`) is not an operator.
pub const ERR_SYSTEM_USER: &str = errors::SYSTEM_USER;
/// A manager has no PIN yet: they must choose one to log in.
pub const ERR_PIN_SETUP_REQUIRED: &str = errors::PIN_SETUP_REQUIRED;
/// `login_with_new_pin` for a user who already has a PIN.
pub const ERR_PIN_ALREADY_SET: &str = errors::PIN_ALREADY_SET;
/// `login_with_new_pin` on a manager tile once a manager has a PIN.
pub const ERR_FIRST_PIN_NEEDS_MANAGER: &str = errors::FIRST_PIN_NEEDS_MANAGER;
pub const ERR_PIN_FORMAT: &str = errors::PIN_FORMAT;

fn valid_pin(pin: &str) -> bool {
    pin.len() == 4 && pin.chars().all(|c| c.is_ascii_digit())
}

/// First login of a user who has no PIN yet (required for a manager):
/// the chosen PIN is stored and the session opened in one step. Refused
/// once a PIN exists, and for a manager tile once any manager has a
/// PIN: later managers get theirs from a manager ([`create_user`],
/// [`set_pin`]).
pub fn do_login_with_new_pin(
    state: &AppState,
    insz: String,
    new_pin: String,
) -> Result<User, String> {
    if !pos_core::user::validate_insz(&insz) {
        return Err(errors::INSZ_INVALID.to_string());
    }
    if insz == SYSTEM_INSZ {
        return Err(ERR_SYSTEM_USER.to_string());
    }
    if !valid_pin(&new_pin) {
        return Err(ERR_PIN_FORMAT.to_string());
    }
    let hash = bcrypt::hash(&new_pin, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = pos_db::queries::users::get_user(&db, &insz)
        .map_err(|e| e.to_string())?
        .filter(|u| u.active)
        .ok_or_else(|| errors::USER_UNKNOWN.to_string())?;
    if !pos_db::queries::users::set_first_pin_hash(&db, &insz, &hash).map_err(user_error)? {
        let manager_tile = user.role == UserRole::Admin && !user.has_pin;
        return Err(if manager_tile {
            ERR_FIRST_PIN_NEEDS_MANAGER
        } else {
            ERR_PIN_ALREADY_SET
        }
        .to_string());
    }
    drop(db);

    let user = User {
        has_pin: true,
        ..user
    };
    access::open_session(state, &user)?;
    Ok(user)
}

/// End the operator's session (after their clock-out): every command
/// needing an operator is refused until the next login.
pub fn do_logout(state: &AppState) -> Result<(), String> {
    access::close_session(state)
}

/// The login tiles: active users, the system identity excluded.
pub fn do_list_users(state: &AppState) -> Result<Vec<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::users::list_users(&db).map_err(|e| e.to_string())
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub insz: String,
    pub name: String,
    pub role: String,
    /// Required for role admin, ignored otherwise.
    #[serde(default)]
    pub initial_pin: Option<String>,
    /// The creating manager's own PIN.
    #[serde(default)]
    pub operator_pin: Option<String>,
}

/// Create a user. Managers only; a new manager gets their first PIN
/// here, confirmed by the creating manager's own PIN.
pub fn do_create_user(state: &AppState, input: CreateUserInput) -> Result<User, String> {
    let operator = access::operator(state, Access::Admin)?;
    if input.insz == SYSTEM_INSZ {
        return Err(ERR_SYSTEM_USER.to_string());
    }
    if input.name.trim().is_empty() {
        return Err(errors::NAME_REQUIRED.to_string());
    }
    if !pos_core::user::validate_insz(&input.insz) {
        return Err(errors::INSZ_INVALID.to_string());
    }

    let role: UserRole = input
        .role
        .parse()
        .map_err(|_: String| errors::ROLE_INVALID.to_string())?;
    let first_pin_hash = if role == UserRole::Admin {
        let pin = input.initial_pin.as_deref().unwrap_or_default();
        if !valid_pin(pin) {
            return Err(ERR_PIN_FORMAT.to_string());
        }
        let operator_pin = input
            .operator_pin
            .ok_or_else(|| errors::PIN_REQUIRED.to_string())?;
        access::confirmed_operator(state, &operator.insz, &operator_pin, Access::Admin)?;
        Some(bcrypt::hash(pin, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?)
    } else {
        None
    };

    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let mut user = pos_db::queries::users::create_user(&tx, &input.insz, &input.name, role)
        .map_err(|e| e.to_string())?;
    if let Some(hash) = first_pin_hash {
        pos_db::queries::users::set_pin_hash(&tx, &input.insz, &hash).map_err(user_error)?;
        user.has_pin = true;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(user)
}

/// Set or change a user's 4-digit PIN. The PIN is bcrypt-hashed
/// before storage - the plaintext never hits the DB.
///
/// Set a PIN: the operator's own, or anyone's for a manager who
/// confirms with their own PIN (`operator_pin`).
pub fn do_set_pin(
    state: &AppState,
    insz: String,
    new_pin: String,
    operator_pin: Option<String>,
) -> Result<(), String> {
    let operator = access::operator(state, Access::Staff)?;
    let other = operator.insz != insz;
    if other && !Access::Admin.allows(operator.role) {
        return Err(access::ERR_FORBIDDEN.to_string());
    }
    if insz == SYSTEM_INSZ {
        return Err(ERR_SYSTEM_USER.to_string());
    }
    if !valid_pin(&new_pin) {
        return Err(ERR_PIN_FORMAT.to_string());
    }
    if other {
        let operator_pin = operator_pin.ok_or_else(|| errors::PIN_REQUIRED.to_string())?;
        access::confirmed_operator(state, &operator.insz, &operator_pin, Access::Admin)?;
    }
    let hash = bcrypt::hash(&new_pin, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::users::set_pin_hash(&db, &insz, &hash).map_err(user_error)
}

pub const ERR_DEACTIVATE_SELF: &str = errors::DEACTIVATE_SELF;

/// Deactivate a user (an employee who left). Managers only, never
/// themselves: the terminal always keeps the manager who did it.
pub fn do_deactivate_user(state: &AppState, insz: String) -> Result<(), String> {
    let operator = access::operator(state, Access::Admin)?;
    if operator.insz == insz {
        return Err(ERR_DEACTIVATE_SELF.to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::users::deactivate_user(&db, &insz).map_err(user_error)
}

fn user_error(e: rusqlite::Error) -> String {
    match e {
        rusqlite::Error::QueryReturnedNoRows => errors::USER_UNKNOWN.to_string(),
        other => errors::internal(other),
    }
}

/// The logged-in operator, if any.
pub fn do_get_current_user(state: &AppState) -> Result<Option<User>, String> {
    Ok(state.session.lock().map_err(|e| e.to_string())?.clone())
}
