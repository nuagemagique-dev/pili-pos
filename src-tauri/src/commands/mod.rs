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

/// Ticket number the next event of `label` will use, without consuming
/// it. The counter only advances in `commit_signed_event`, once the FDM
/// has signed the event, so a refused or unreachable FDM leaves no gap
/// (same pattern as checkout.rs).
pub(crate) fn peek_ticket_number(
    state: &AppState,
    label: pos_fiscal::types::EventLabel,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    pos_fiscal::counter::FiscalCounter::peek_next(&db, label).map_err(|e| e.to_string())
}

/// A fiscal event the FDM has signed, ready to be archived.
pub(crate) struct SignedEvent<'a> {
    pub label: pos_fiscal::types::EventLabel,
    pub ticket_number: i64,
    pub order_id: Option<i64>,
    pub user_insz: &'a str,
    pub request_json: &'a str,
    pub sign_result: &'a pos_fiscal::response::SignResult,
    /// `(order_id, last_entry_id)`: the order entries this event
    /// transmitted, marked as sent in the same transaction.
    pub sent_entries: Option<(i64, i64)>,
}

/// Advance the event's counter and archive it in one transaction.
/// Events are never archived without the FDM's signature: callers
/// return an error to the operator when signing fails.
pub(crate) fn commit_signed_event(
    state: &AppState,
    config: &pos_core::config::PosConfig,
    event: SignedEvent<'_>,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    let committed =
        pos_fiscal::counter::FiscalCounter::next(&tx, event.label).map_err(|e| e.to_string())?;
    // Under a genuine race (e.g. a UI double-tap slipping past its
    // loading guard, or two commands sharing a label firing at once)
    // two callers can peek the same ticket number before either
    // commits. This must be a graceful error, not `debug_assert_eq!`:
    // a panic here would unwind while `db` (the MutexGuard) is still
    // held, poisoning `state.db` and bricking every subsequent
    // command for the rest of the process's life — far worse than one
    // rejected request. The transaction rolls back on `Err` (dropped
    // without `commit()`), so the counter reverts and no gap results;
    // the loser can safely retry and will peek the next free number.
    if committed != event.ticket_number {
        return Err(format!(
            "counter drifted between peek and commit for {:?} (peeked {}, committed {}) — \
             likely a concurrent sign of the same event; retry",
            event.label, event.ticket_number, committed
        ));
    }
    let id = pos_fiscal::archive::archive_event(
        &tx,
        pos_fiscal::archive::ArchiveParams {
            event_type: event.label,
            sequence_number: event.ticket_number,
            order_id: event.order_id,
            pos_id: &config.pos_id,
            terminal_id: &config.terminal_id,
            user_insz: event.user_insz,
            request_json: event.request_json,
            sign_result: Some(event.sign_result),
        },
    )
    .map_err(|e| e.to_string())?;
    if let Some((order_id, last_entry_id)) = event.sent_entries {
        pos_db::queries::orders::mark_order_entries_sent(&tx, order_id, last_entry_id, id)
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(id)
}
