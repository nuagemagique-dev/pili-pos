use pos_core::user::User;
use tauri::State;

use pos_engine::commands::auth::CreateUserInput;

use super::Managed;

/// Login with INSZ + optional PIN. If the user has a PIN set (pin_hash
/// is not NULL), the caller MUST provide the correct PIN or login is
/// rejected. A manager without a PIN is refused: on first boot (no
/// manager has a PIN yet) with [`ERR_PIN_SETUP_REQUIRED`], and they
/// choose one through [`login_with_new_pin`]; afterwards with
/// [`ERR_FIRST_PIN_NEEDS_MANAGER`], since another manager sets it.
/// Other roles without a PIN log in, and the frontend warns them to set
/// one. The system identity never logs in.
#[tauri::command(async)]
pub fn login(state: State<'_, Managed>, insz: String, pin: Option<String>) -> Result<User, String> {
    pos_engine::commands::auth::do_login(state.inner(), insz, pin)
}

/// First login of a user who has no PIN yet (required for a manager):
/// the chosen PIN is stored and the session opened in one step. Refused
/// once a PIN exists, and for a manager tile once any manager has a
/// PIN: later managers get theirs from a manager ([`create_user`],
/// [`set_pin`]).
#[tauri::command(async)]
pub fn login_with_new_pin(
    state: State<'_, Managed>,
    insz: String,
    new_pin: String,
) -> Result<User, String> {
    pos_engine::commands::auth::do_login_with_new_pin(state.inner(), insz, new_pin)
}

/// End the operator's session (after their clock-out): every command
/// needing an operator is refused until the next login.
#[tauri::command(async)]
pub fn logout(state: State<'_, Managed>) -> Result<(), String> {
    pos_engine::commands::auth::do_logout(state.inner())
}

#[tauri::command(async)]
pub fn list_users(state: State<'_, Managed>) -> Result<Vec<User>, String> {
    pos_engine::commands::auth::do_list_users(state.inner())
}

#[tauri::command(async)]
pub fn create_user(state: State<'_, Managed>, input: CreateUserInput) -> Result<User, String> {
    pos_engine::commands::auth::do_create_user(state.inner(), input)
}

/// Set or change a user's 4-digit PIN. The PIN is bcrypt-hashed
/// before storage - the plaintext never hits the DB.
#[tauri::command(async)]
pub fn set_pin(
    state: State<'_, Managed>,
    insz: String,
    new_pin: String,
    operator_pin: Option<String>,
) -> Result<(), String> {
    pos_engine::commands::auth::do_set_pin(state.inner(), insz, new_pin, operator_pin)
}

/// Deactivate a user (an employee who left). Managers only, never
/// themselves: the terminal always keeps the manager who did it.
#[tauri::command(async)]
pub fn deactivate_user(state: State<'_, Managed>, insz: String) -> Result<(), String> {
    pos_engine::commands::auth::do_deactivate_user(state.inner(), insz)
}

/// The logged-in operator, if any.
#[tauri::command(async)]
pub fn get_current_user(state: State<'_, Managed>) -> Result<Option<User>, String> {
    pos_engine::commands::auth::do_get_current_user(state.inner())
}
