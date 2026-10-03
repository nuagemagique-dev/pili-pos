pub mod access;
pub mod auth;
pub mod bar_slip;
pub mod checkout;
pub mod copy;
pub mod delivery;
pub mod errors;
pub mod financial;
#[cfg(feature = "hubrise")]
pub mod hubrise;
#[cfg(feature = "hubrise")]
pub mod hubrise_push;
#[cfg(feature = "peppol")]
pub mod invoice;
pub mod menu;
#[cfg(feature = "hubrise")]
pub mod modifiers;
pub mod order;
pub mod print;
pub mod refund;
pub mod report;
pub mod settings;
pub mod social;
pub mod support;
pub mod tables;
pub mod unpaid;
#[cfg(feature = "hubrise")]
pub mod webshop;

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

/// The business day `now` belongs to: a service that runs past
/// midnight stays on its day until `day_end_hour` (Belgian time).
pub(crate) fn business_date(now: chrono::DateTime<chrono_tz::Tz>, day_end_hour: u32) -> String {
    (now - chrono::Duration::hours(i64::from(day_end_hour)))
        .date_naive()
        .to_string()
}

/// Whether `period` belongs to a business day that is over: its Z was
/// not made, and the POS makes it (producers FAQ Q501c).
pub(crate) fn period_overdue(
    period: &pos_db::queries::booking::BookingPeriod,
    now: chrono::DateTime<chrono_tz::Tz>,
    day_end_hour: u32,
) -> bool {
    period.booking_date < business_date(now, day_end_hour)
}

/// The open booking period (POS spec v2.2 §2.2.2), opened by the first
/// event after a Z report. Its booking date is the business day in
/// Belgium when it opened, whatever the system time zone.
pub(crate) fn booking_period(
    db: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
) -> Result<pos_db::queries::booking::BookingPeriod, String> {
    let today = business_date(pos_fiscal::event::now_in_belgium(), config.day_end_hour);
    pos_db::queries::booking::current_period(db, &uuid::Uuid::new_v4().to_string(), &today)
        .map_err(|e| e.to_string())
}

/// The open booking period, as carried by every fiscal event. No event
/// goes into a business day that is over: its automatic Z comes first
/// (`report::close_overdue_day`, run in the background). Training
/// events take no Z, so they are let through.
pub(crate) fn booking(
    db: &rusqlite::Connection,
    config: &pos_core::config::PosConfig,
) -> Result<pos_fiscal::event::Booking, String> {
    let period = booking_period(db, config)?;
    if !config.training_mode
        && period_overdue(
            &period,
            pos_fiscal::event::now_in_belgium(),
            config.day_end_hour,
        )
    {
        return Err(errors::PREVIOUS_DAY_OPEN.into());
    }
    Ok(pos_fiscal::event::Booking {
        period_id: period.id,
        date: period.booking_date,
    })
}

/// `booking`, for commands that don't hold the database lock.
pub(crate) fn current_booking(state: &AppState) -> Result<pos_fiscal::event::Booking, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    booking(&db, &config)
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
    commit_signed_event_then(state, config, event, |_, _| Ok(())).map(|(id, ())| id)
}

/// [`commit_signed_event`], running `then` (given the new event's id)
/// inside the same transaction - e.g. to queue its email delivery.
pub(crate) fn commit_signed_event_then<T>(
    state: &AppState,
    config: &pos_core::config::PosConfig,
    event: SignedEvent<'_>,
    then: impl FnOnce(&rusqlite::Connection, i64) -> Result<T, String>,
) -> Result<(i64, T), String> {
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
    // command for the rest of the process's life - far worse than one
    // rejected request. The transaction rolls back on `Err` (dropped
    // without `commit()`), so the counter reverts and no gap results;
    // the loser can safely retry and will peek the next free number.
    if committed != event.ticket_number {
        return Err(errors::concurrent(format_args!(
            "sign {:?}: peeked {}, committed {}",
            event.label, event.ticket_number, committed
        )));
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
    let extra = then(&tx, id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok((id, extra))
}
