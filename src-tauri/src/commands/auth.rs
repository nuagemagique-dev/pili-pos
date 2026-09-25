use pos_core::user::{User, UserRole};
use serde::Deserialize;
use tauri::State;

use crate::state::AppState;

/// Login with INSZ + optional PIN. If the user has a PIN set (pin_hash
/// is not NULL), the caller MUST provide the correct PIN or login is
/// rejected. If no PIN is set, login succeeds without one — but the
/// frontend should warn the operator to configure a PIN via Settings.
#[tauri::command]
pub fn login(
    state: State<'_, AppState>,
    insz: String,
    pin: Option<String>,
) -> Result<User, String> {
    do_login(state.inner(), insz, pin)
}

/// Inner implementation of [`login`], decoupled from Tauri's `State`
/// wrapper so integration tests can drive it directly (same pattern as
/// every other command module's `do_*` functions).
pub fn do_login(state: &AppState, insz: String, pin: Option<String>) -> Result<User, String> {
    if !pos_core::user::validate_insz(&insz) {
        return Err("INSZ invalide".to_string());
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = pos_db::queries::users::get_user(&db, &insz)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Utilisateur inconnu".to_string())?;

    // PIN verification: only when the user has a stored hash.
    if user.has_pin {
        let stored_hash = pos_db::queries::users::get_pin_hash(&db, &insz)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "PIN hash missing".to_string())?;
        let provided = pin.ok_or_else(|| "pin_required".to_string())?;
        if !bcrypt::verify(&provided, &stored_hash).unwrap_or(false) {
            return Err("pin_wrong".to_string());
        }
    }

    Ok(user)
}

#[tauri::command]
pub fn list_users(state: State<'_, AppState>) -> Result<Vec<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::users::list_users(&db).map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct CreateUserInput {
    pub insz: String,
    pub name: String,
    pub role: String,
}

#[tauri::command]
pub fn create_user(state: State<'_, AppState>, input: CreateUserInput) -> Result<User, String> {
    do_create_user(state.inner(), input)
}

pub fn do_create_user(state: &AppState, input: CreateUserInput) -> Result<User, String> {
    if !pos_core::user::validate_insz(&input.insz) {
        return Err("INSZ invalide".to_string());
    }

    let role: UserRole = input.role.parse().map_err(|e: String| e)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::users::create_user(&db, &input.insz, &input.name, role)
        .map_err(|e| e.to_string())
}

/// Set or change a user's 4-digit PIN. The PIN is bcrypt-hashed
/// before storage — the plaintext never hits the DB.
#[tauri::command]
pub fn set_pin(state: State<'_, AppState>, insz: String, new_pin: String) -> Result<(), String> {
    do_set_pin(state.inner(), insz, new_pin)
}

pub fn do_set_pin(state: &AppState, insz: String, new_pin: String) -> Result<(), String> {
    if new_pin.len() != 4 || !new_pin.chars().all(|c| c.is_ascii_digit()) {
        return Err("PIN must be exactly 4 digits".to_string());
    }
    let hash = bcrypt::hash(&new_pin, bcrypt::DEFAULT_COST).map_err(|e| e.to_string())?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_db::queries::users::set_pin_hash(&db, &insz, &hash).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_current_user() -> Result<Option<User>, String> {
    Ok(None)
}
