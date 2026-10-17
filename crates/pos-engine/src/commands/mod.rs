pub mod access;
pub mod auth;
pub mod bar_slip;
pub mod checkout;
pub mod copy;
pub mod delivery;
pub mod eft;
pub mod errors;
pub mod export;
pub mod fdm_monitor;
pub mod financial;
#[cfg(feature = "hubrise")]
pub mod hubrise;
#[cfg(feature = "hubrise")]
pub mod hubrise_push;
#[cfg(feature = "hubrise")]
pub mod integrations;
#[cfg(feature = "peppol")]
pub mod invoice;
pub mod menu;
#[cfg(feature = "hubrise")]
pub mod modifiers;
pub mod order;
pub mod payment_correction;
#[cfg(feature = "hubrise")]
pub(crate) mod price_changes;
pub mod print;
pub mod refund;
pub mod report;
pub mod session;
pub mod settings;
pub mod signing;
pub mod social;
pub mod support;
pub mod table_plan;
pub mod table_transfer;
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
    _guard: &signing::SignGuard<'_>,
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
    guard: &signing::SignGuard<'_>,
    config: &pos_core::config::PosConfig,
) -> Result<pos_fiscal::event::Booking, String> {
    let period = booking_period(db, guard, config)?;
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
pub(crate) fn current_booking(
    state: &AppState,
    guard: &signing::SignGuard<'_>,
) -> Result<pos_fiscal::event::Booking, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    booking(&db, guard, &config)
}

/// Ticket number the next event of `label` will use, without consuming
/// it: the counter advances only when the signed event is archived
/// (`signing::sign`), so a refused request leaves no gap.
pub(crate) fn peek_ticket_number(
    state: &AppState,
    guard: &signing::SignGuard<'_>,
    label: pos_fiscal::types::EventLabel,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    signing::peek(&db, guard, label)
}
