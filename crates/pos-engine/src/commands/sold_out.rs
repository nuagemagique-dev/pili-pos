//! Sold-out products ("Épuisé", V11-D14, V11-D15): a cashier marks a product sold out until the
//! end of the business day or until re-enabled; the till then refuses to add it to an order
//! (`product_sold_out|product=<id>`). Decreasing a quantity, removing a line, lines already on
//! an order and online orders are not affected. There is no fiscal effect: no event, no counter.
//!
//! Every set, clear and expiry is appended to `sold_out_log`. Expiry is by time (the end of the
//! business day in Europe/Brussels, from `day_end_hour`), not by Z; its log row is written
//! lazily, on the next read. With HubRise on and a product that has a SKU ref, the change is
//! queued for the inventory push (`pos_hubrise::inventory`).

use chrono::{Duration, NaiveDate, NaiveTime, TimeZone};
use pos_db::queries::sold_out as q;
pub use pos_db::queries::sold_out::SoldOutItem;

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// The end of the business day `now` belongs to: the next `day_end_hour` o'clock in Belgium, as
/// RFC 3339 text with the Brussels offset (`2026-10-18T04:00:00+02:00`).
pub(crate) fn end_of_business_day(
    now: chrono::DateTime<chrono_tz::Tz>,
    day_end_hour: u32,
) -> String {
    let business_day: NaiveDate = super::business_date(now, day_end_hour)
        .parse()
        .unwrap_or_else(|_| now.date_naive());
    let local = (business_day + Duration::days(1))
        .and_time(NaiveTime::from_hms_opt(day_end_hour.min(23), 0, 0).unwrap_or(NaiveTime::MIN));
    let tz = now.timezone();
    // A local time that does not exist (spring-forward gap) is read one hour later.
    let end = tz
        .from_local_datetime(&local)
        .earliest()
        .or_else(|| {
            tz.from_local_datetime(&(local + Duration::hours(1)))
                .earliest()
        })
        .unwrap_or(now);
    end.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
}

fn lang_of(lang: Option<String>) -> String {
    lang.unwrap_or_else(|| "fr".to_string())
}

/// Record the products whose end of day is past (their `expired` log row) and drop their rows.
pub(crate) fn expire_due(db: &rusqlite::Connection) {
    if let Err(e) = q::expire_due(db) {
        tracing::warn!(%e, "sold-out expiry pass failed");
    }
}

/// The error of an order line for a sold-out product, `None` when it can be sold.
pub(crate) fn refuse_if_sold_out(
    db: &rusqlite::Connection,
    menu_item_id: i64,
) -> Result<(), String> {
    if q::is_sold_out(db, menu_item_id).map_err(errors::internal)? {
        return Err(errors::coded(
            errors::PRODUCT_SOLD_OUT,
            &[("product", &menu_item_id)],
        ));
    }
    Ok(())
}

/// Queue the push of the product's state to the HubRise inventory: only when HubRise is on and
/// the product has a SKU ref. Pure SQLite, no network.
#[cfg(feature = "hubrise")]
fn queue_inventory(
    state: &AppState,
    db: &rusqlite::Connection,
    menu_item_id: i64,
) -> Result<(), String> {
    if !state.hubrise_switch.is_on() {
        return Ok(());
    }
    if let Some(sku) = q::sku_ref(db, menu_item_id).map_err(errors::internal)? {
        q::queue_inventory_sync(db, menu_item_id, &sku).map_err(errors::internal)?;
        state.hubrise_outbox_wake.notify_one();
    }
    Ok(())
}

#[cfg(not(feature = "hubrise"))]
fn queue_inventory(_: &AppState, _: &rusqlite::Connection, _: i64) -> Result<(), String> {
    Ok(())
}

fn exists(db: &rusqlite::Connection, menu_item_id: i64) -> Result<(), String> {
    let found: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM menu_items WHERE id = ?1)",
            [menu_item_id],
            |r| r.get(0),
        )
        .map_err(errors::internal)?;
    if found {
        Ok(())
    } else {
        Err(errors::coded(
            errors::PRODUCT_UNAVAILABLE,
            &[("product", &menu_item_id)],
        ))
    }
}

/// The products sold out now. Cashier and manager.
pub fn do_list_sold_out(
    state: &AppState,
    lang: Option<String>,
) -> Result<Vec<SoldOutItem>, String> {
    access::operator(state, Access::Cashier)?;
    let db = state.db.lock().map_err(errors::internal)?;
    expire_due(&db);
    q::list(&db, &lang_of(lang)).map_err(errors::internal)
}

/// Mark a product sold out: until the end of the business day, or until re-enabled. Cashier and
/// manager. Returns the list of sold-out products.
pub fn do_set_sold_out(
    state: &AppState,
    menu_item_id: i64,
    until_end_of_day: bool,
    lang: Option<String>,
) -> Result<Vec<SoldOutItem>, String> {
    let user = access::operator(state, Access::Cashier)?;
    let day_end_hour = state.config.lock().map_err(errors::internal)?.day_end_hour;
    let until = until_end_of_day
        .then(|| end_of_business_day(pos_fiscal::event::now_in_belgium(), day_end_hour));
    let db = state.db.lock().map_err(errors::internal)?;
    exists(&db, menu_item_id)?;
    expire_due(&db);
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    q::set(&tx, menu_item_id, until.as_deref(), &user.insz).map_err(errors::internal)?;
    queue_inventory(state, &tx, menu_item_id)?;
    tx.commit().map_err(errors::internal)?;
    tracing::info!(menu_item_id, ?until, "product sold out");
    q::list(&db, &lang_of(lang)).map_err(errors::internal)
}

/// Make a product available again. Cashier and manager. Returns the list of sold-out products.
pub fn do_clear_sold_out(
    state: &AppState,
    menu_item_id: i64,
    lang: Option<String>,
) -> Result<Vec<SoldOutItem>, String> {
    let user = access::operator(state, Access::Cashier)?;
    let db = state.db.lock().map_err(errors::internal)?;
    exists(&db, menu_item_id)?;
    expire_due(&db);
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    if q::clear(&tx, menu_item_id, &user.insz).map_err(errors::internal)? {
        queue_inventory(state, &tx, menu_item_id)?;
        tracing::info!(menu_item_id, "product available again");
    }
    tx.commit().map_err(errors::internal)?;
    q::list(&db, &lang_of(lang)).map_err(errors::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::Europe::Brussels;

    fn at(s: &str) -> chrono::DateTime<chrono_tz::Tz> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&Brussels)
    }

    #[test]
    fn the_end_of_the_day_is_the_next_day_end_hour() {
        // Evening: the day ends tomorrow at 04:00 (summer time, +02:00).
        assert_eq!(
            end_of_business_day(at("2026-10-17T20:30:00+02:00"), 4),
            "2026-10-18T04:00:00+02:00"
        );
        // After midnight but before day_end_hour: still the same business day.
        assert_eq!(
            end_of_business_day(at("2026-10-18T01:00:00+02:00"), 4),
            "2026-10-18T04:00:00+02:00"
        );
        // Exactly at day_end_hour a new business day starts.
        assert_eq!(
            end_of_business_day(at("2026-10-18T04:00:00+02:00"), 4),
            "2026-10-19T04:00:00+02:00"
        );
    }

    #[test]
    fn the_offset_follows_winter_and_summer_time() {
        // Summer time ends on 2026-10-25: the day that starts on the 24th ends at +01:00.
        assert_eq!(
            end_of_business_day(at("2026-10-24T20:00:00+02:00"), 4),
            "2026-10-25T04:00:00+01:00"
        );
        assert_eq!(
            end_of_business_day(at("2026-01-10T12:00:00+01:00"), 5),
            "2026-01-11T05:00:00+01:00"
        );
    }

    #[test]
    fn day_end_hour_zero_is_midnight() {
        assert_eq!(
            end_of_business_day(at("2026-10-17T20:30:00+02:00"), 0),
            "2026-10-18T00:00:00+02:00"
        );
    }

    #[test]
    fn a_missing_local_hour_is_read_one_hour_later() {
        // Spring forward 2026-03-29: 02:00 does not exist.
        assert_eq!(
            end_of_business_day(at("2026-03-28T20:00:00+01:00"), 2),
            "2026-03-29T03:00:00+02:00"
        );
    }
}
