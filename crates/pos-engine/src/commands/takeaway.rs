//! The numbered takeaway (UX audit T2): each takeaway sale of the till
//! carries a number of the day ("E-61") and, when the operator enters
//! them, the name and phone of the customer.
//!
//! - The number is given when the takeaway order is created. It starts at
//!   the installation's first number (an owner setting, default 61 like the
//!   old register) and restarts every business day: the day is the booking
//!   date the engine already uses (`commands::business_date`, the day ends
//!   at `day_end_hour`, so a service past midnight keeps its day). A
//!   takeaway that held nothing is voided on leaving and gives its number
//!   back when it was the last of the day.
//! - The name and phone are typed from the order screen. They are personal
//!   data: the engine clears them 72 hours after the order opened, like the
//!   e-mail address of a digital ticket ([`purge_contacts`]); the number
//!   stays.
//! - The number and the name print in large type on the kitchen and bar
//!   slips ([`slip_mark`]). The pre-bill is a dine-in document (a takeaway
//!   is a direct sale, nothing is pre-billed) and the sale ticket follows
//!   the SPF example: neither carries them.
//!
//! Local data, never fiscal: the tag lives in its own table
//! (`takeaway_tags`) and reaches no FDM request, signed ticket, archived
//! event or export (`tests/takeaway_numbers.rs` proves it). An online order
//! keeps its platform reference and gets no tag.

use chrono::{Duration, SecondsFormat, Utc};
use pos_core::order::{OrderStatus, OrderType};
use pos_db::queries::takeaway as tags;
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::owner_settings;
use crate::state::AppState;

/// The key of the numbering in the owner settings store.
pub const KEY_TAKEAWAY_NUMBERING: &str = "takeaway_numbering";

/// The first number of a new installation (the old register started at 61).
pub const DEFAULT_FIRST_NUMBER: i32 = 61;

/// The highest first number a manager may choose.
pub const MAX_FIRST_NUMBER: i32 = 999;

/// The longest customer name, in characters.
pub const NAME_MAX: usize = 30;

/// The longest customer phone, in characters.
pub const PHONE_MAX: usize = 24;

/// How long a name and a phone are kept (GDPR), like a ticket's e-mail address.
pub const CONTACT_RETENTION: Duration = Duration::hours(72);

/// The numbering of this installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeawayNumbering {
    /// The first number of each day.
    pub first_number: i32,
}

impl Default for TakeawayNumbering {
    fn default() -> Self {
        Self {
            first_number: DEFAULT_FIRST_NUMBER,
        }
    }
}

/// The number, the way every screen and slip writes it: `E-61`.
pub fn label(number: i64) -> String {
    format!("E-{number}")
}

/// The tag of a takeaway order, for the screens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeawayTag {
    pub order_id: i64,
    /// The business day the number belongs to (`YYYY-MM-DD`).
    pub day: String,
    pub number: i64,
    /// `E-61`.
    pub label: String,
    pub customer_name: Option<String>,
    pub customer_phone: Option<String>,
}

impl From<tags::Tag> for TakeawayTag {
    fn from(tag: tags::Tag) -> Self {
        Self {
            order_id: tag.order_id,
            day: tag.day,
            label: label(tag.number),
            number: tag.number,
            customer_name: tag.customer_name,
            customer_phone: tag.customer_phone,
        }
    }
}

/// The numbering, for any logged-in operator.
pub fn do_get_takeaway_numbering(state: &AppState) -> Result<TakeawayNumbering, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    numbering(&db)
}

fn numbering(db: &rusqlite::Connection) -> Result<TakeawayNumbering, String> {
    owner_settings::get::<TakeawayNumbering>(db, KEY_TAKEAWAY_NUMBERING)
}

/// Store the numbering (managers only). The first number is between 1 and
/// [`MAX_FIRST_NUMBER`]. The change applies to the next number handed out:
/// a number already given never changes.
pub fn do_set_takeaway_numbering(
    state: &AppState,
    input: TakeawayNumbering,
) -> Result<TakeawayNumbering, String> {
    let operator = access::operator(state, Access::Admin)?;
    if !(1..=MAX_FIRST_NUMBER).contains(&input.first_number) {
        return Err(errors::coded(
            errors::TAKEAWAY_NUMBER_INVALID,
            &[("min", &1), ("max", &MAX_FIRST_NUMBER)],
        ));
    }
    let db = state.db.lock().map_err(errors::internal)?;
    owner_settings::set(&db, KEY_TAKEAWAY_NUMBERING, &input, Some(&operator.insz))?;
    Ok(input)
}

/// Give a takeaway order its number of today. Called by the order creation
/// with the database lock held; a failure is logged and the order goes on
/// without a number: numbering never stops a sale.
pub(crate) fn assign(
    db: &rusqlite::Connection,
    day_end_hour: u32,
    order_id: i64,
) -> Option<TakeawayTag> {
    let day = super::business_date(pos_fiscal::event::now_in_belgium(), day_end_hour);
    let result = numbering(db).and_then(|n| {
        tags::assign(db, order_id, &day, i64::from(n.first_number)).map_err(errors::internal)
    });
    match result {
        Ok(tag) => tag.map(TakeawayTag::from),
        Err(err) => {
            tracing::warn!(%err, order_id, "takeaway number not given");
            None
        }
    }
}

/// The tag of an order: `None` for an order that has none (a table, an
/// online order, a takeaway opened before the numbering existed).
pub fn do_get_takeaway_tag(state: &AppState, order_id: i64) -> Result<Option<TakeawayTag>, String> {
    access::operator(state, Access::Staff)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(tags::tag(&db, order_id)
        .map_err(errors::internal)?
        .map(TakeawayTag::from))
}

/// A customer name as stored: trimmed, `None` when empty; at most
/// [`NAME_MAX`] characters, only what the slip printer prints.
fn clean_name(name: Option<String>) -> Result<Option<String>, String> {
    let Some(name) = name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else {
        return Ok(None);
    };
    if name.chars().count() > NAME_MAX || !pos_print::escpos::is_cp858_printable(&name) {
        return Err(errors::coded(
            errors::TAKEAWAY_NAME_INVALID,
            &[("max", &NAME_MAX)],
        ));
    }
    Ok(Some(name))
}

/// A customer phone as stored: trimmed, `None` when empty. Digits with the
/// usual separators (space, `+`, `.`, `/`, `-`, parentheses), 3 to 15
/// digits, at most [`PHONE_MAX`] characters.
fn clean_phone(phone: Option<String>) -> Result<Option<String>, String> {
    let Some(phone) = phone
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
    else {
        return Ok(None);
    };
    let digits = phone.chars().filter(char::is_ascii_digit).count();
    let allowed = phone
        .chars()
        .all(|c| c.is_ascii_digit() || " +./-()".contains(c));
    if !allowed || !(3..=15).contains(&digits) || phone.chars().count() > PHONE_MAX {
        return Err(errors::coded(
            errors::TAKEAWAY_PHONE_INVALID,
            &[("max", &PHONE_MAX)],
        ));
    }
    Ok(Some(phone))
}

/// Set (or, with `None` or an empty text, clear) the name and the phone of
/// the customer of an open takeaway of the till. Both are replaced: the
/// screen sends the one it does not edit as it is. Any staff member. A
/// takeaway opened before the numbering existed gets its number now.
pub fn do_set_takeaway_customer(
    state: &AppState,
    order_id: i64,
    name: Option<String>,
    phone: Option<String>,
) -> Result<TakeawayTag, String> {
    access::operator(state, Access::Staff)?;
    let name = clean_name(name)?;
    let phone = clean_phone(phone)?;
    let day_end_hour = state.config.lock().map_err(errors::internal)?.day_end_hour;
    let db = state.db.lock().map_err(errors::internal)?;
    let order = pos_db::queries::orders::get_order(&db, order_id).map_err(errors::internal)?;
    if order.order_type != OrderType::Takeaway || is_online(&db, order_id)? {
        return Err(errors::TAKEAWAY_CUSTOMER_NOT_TAKEAWAY.into());
    }
    if order.status != OrderStatus::Open {
        return Err(errors::ORDER_NOT_OPEN.into());
    }
    if tags::tag(&db, order_id)
        .map_err(errors::internal)?
        .is_none()
    {
        assign(&db, day_end_hour, order_id);
    }
    if !tags::set_contact(&db, order_id, name.as_deref(), phone.as_deref())
        .map_err(errors::internal)?
    {
        // The day ran out of numbers: no number, so no place for a name either.
        return Err(errors::TAKEAWAY_CUSTOMER_NOT_TAKEAWAY.into());
    }
    tags::tag(&db, order_id)
        .map_err(errors::internal)?
        .map(TakeawayTag::from)
        .ok_or_else(|| errors::internal("takeaway tag vanished"))
}

fn is_online(db: &rusqlite::Connection, order_id: i64) -> Result<bool, String> {
    db.query_row(
        "SELECT hubrise_order_id IS NOT NULL FROM orders WHERE id = ?1",
        [order_id],
        |r| r.get(0),
    )
    .map_err(errors::internal)
}

/// The GDPR sweep: clear the name and phone of every takeaway opened more
/// than [`CONTACT_RETENTION`] ago. The number stays. Returns how many tags
/// were cleared. Run by the background worker next to the ticket outbox.
pub fn purge_contacts(state: &AppState) -> usize {
    purge_contacts_at(state, Utc::now())
}

/// [`purge_contacts`] at the instant `now`.
pub fn purge_contacts_at(state: &AppState, now: chrono::DateTime<Utc>) -> usize {
    let cutoff = (now - CONTACT_RETENTION).to_rfc3339_opts(SecondsFormat::Millis, true);
    let Ok(db) = state.db.lock() else { return 0 };
    match tags::clear_contacts_before(&db, &cutoff) {
        Ok(cleared) => {
            if cleared > 0 {
                tracing::info!(cleared, "takeaway customer contacts cleared (72 h)");
            }
            cleared
        }
        Err(err) => {
            tracing::warn!(%err, "takeaway customer contacts not cleared");
            0
        }
    }
}

/// What a kitchen or bar slip prints for a takeaway: its number in large
/// type and the customer name. The phone stays off the paper.
pub(crate) fn slip_mark(
    db: &rusqlite::Connection,
    order_id: i64,
) -> Option<pos_print::bar_ticket::TakeawayMark> {
    match tags::tag(db, order_id) {
        Ok(tag) => tag.map(|t| pos_print::bar_ticket::TakeawayMark {
            label: label(t.number),
            name: t.customer_name,
        }),
        Err(err) => {
            tracing::warn!(%err, order_id, "takeaway tag unreadable for the slip");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_is_e_dash_number() {
        assert_eq!(label(61), "E-61");
        assert_eq!(label(1204), "E-1204");
    }

    #[test]
    fn a_name_is_trimmed_bounded_and_printable() {
        assert_eq!(clean_name(None).unwrap(), None);
        assert_eq!(clean_name(Some("   ".into())).unwrap(), None);
        assert_eq!(
            clean_name(Some("  Dupont ".into())).unwrap().as_deref(),
            Some("Dupont")
        );
        assert_eq!(
            clean_name(Some("Hélène".into())).unwrap().as_deref(),
            Some("Hélène")
        );
        let too_long = "x".repeat(NAME_MAX + 1);
        assert_eq!(
            clean_name(Some(too_long)).unwrap_err(),
            "takeaway_name_invalid|max=30"
        );
        assert!(clean_name(Some("x".repeat(NAME_MAX))).is_ok());
        assert!(
            clean_name(Some("王".into())).is_err(),
            "the printer cannot print it"
        );
    }

    #[test]
    fn a_phone_is_digits_with_the_usual_separators() {
        assert_eq!(clean_phone(None).unwrap(), None);
        assert_eq!(clean_phone(Some(" ".into())).unwrap(), None);
        for ok in [
            "0470 12 34 56",
            "+32 4 223 45 67",
            "04/223.45.67",
            "(04) 223-45-67",
        ] {
            assert_eq!(clean_phone(Some(ok.into())).unwrap().as_deref(), Some(ok));
        }
        for bad in ["abc", "12", "0470 12 34 56 78 90 12", "tel 0470", "+++"] {
            assert!(clean_phone(Some(bad.into())).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_default_numbering_starts_at_61() {
        assert_eq!(TakeawayNumbering::default().first_number, 61);
    }
}
