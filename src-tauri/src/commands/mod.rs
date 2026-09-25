pub mod auth;
pub mod checkout;
pub mod copy;
pub mod financial;
#[cfg(feature = "hubrise")]
pub mod hubrise;
#[cfg(feature = "hubrise")]
pub mod hubrise_push;
#[cfg(feature = "peppol")]
pub mod invoice;
pub mod menu;
pub mod order;
pub mod print;
pub mod refund;
pub mod report;
pub mod settings;
pub mod social;
pub mod tables;

use crate::state::AppState;

/// Name printed as the ticket's user identification (manual §VI row
/// VIII). The INSZ itself is never printed on customer tickets; it is
/// only used as a last resort when the user row is missing.
pub(crate) fn operator_name(state: &AppState, insz: &str) -> String {
    state
        .db
        .lock()
        .ok()
        .and_then(|db| pos_db::queries::users::get_user(&db, insz).ok().flatten())
        .map(|u| u.name)
        .unwrap_or_else(|| insz.to_string())
}
