//! The takeaway tags (migration 90, UX audit T2): the number of the day of a
//! takeaway sale and its optional customer name and phone. See
//! `schema_takeaway`. Local data, never fiscal.

use rusqlite::{Connection, OptionalExtension, params};

/// The highest number a day hands out ("E-9999" still fits the slip).
pub const MAX_NUMBER: i64 = 9999;

/// A takeaway order's tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub order_id: i64,
    /// The business day the number belongs to (`YYYY-MM-DD`).
    pub day: String,
    pub number: i64,
    pub customer_name: Option<String>,
    pub customer_phone: Option<String>,
}

fn tag_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Tag> {
    Ok(Tag {
        order_id: r.get(0)?,
        day: r.get(1)?,
        number: r.get(2)?,
        customer_name: r.get(3)?,
        customer_phone: r.get(4)?,
    })
}

/// The tag of an order; `None` for an order without one (a dine-in order,
/// an online order, a takeaway opened before the numbering existed).
pub fn tag(conn: &Connection, order_id: i64) -> Result<Option<Tag>, rusqlite::Error> {
    conn.query_row(
        "SELECT order_id, day, number, customer_name, customer_phone
         FROM takeaway_tags WHERE order_id = ?1",
        params![order_id],
        tag_row,
    )
    .optional()
}

/// Give the takeaway `order_id` its number of `day`: one more than the
/// highest number the day handed out, and at least `first_number`. An order
/// that already has a tag keeps it. `None` when the day ran out of numbers
/// ([`MAX_NUMBER`]): the order then goes without a number.
pub fn assign(
    conn: &Connection,
    order_id: i64,
    day: &str,
    first_number: i64,
) -> Result<Option<Tag>, rusqlite::Error> {
    super::with_savepoint(conn, |tx| {
        if let Some(existing) = tag(tx, order_id)? {
            return Ok(Some(existing));
        }
        let highest: i64 = tx.query_row(
            "SELECT COALESCE(MAX(number), 0) FROM takeaway_tags WHERE day = ?1",
            params![day],
            |r| r.get(0),
        )?;
        let number = highest.max(first_number - 1) + 1;
        if number > MAX_NUMBER {
            return Ok(None);
        }
        tx.execute(
            "INSERT INTO takeaway_tags (order_id, day, number) VALUES (?1, ?2, ?3)",
            params![order_id, day, number],
        )?;
        tag(tx, order_id)
    })
}

/// Replace the customer name and phone of a tagged order. Returns whether
/// the order has a tag.
pub fn set_contact(
    conn: &Connection,
    order_id: i64,
    name: Option<&str>,
    phone: Option<&str>,
) -> Result<bool, rusqlite::Error> {
    let changed = conn.execute(
        "UPDATE takeaway_tags SET customer_name = ?2, customer_phone = ?3, contact_cleared_at = NULL
         WHERE order_id = ?1",
        params![order_id, name, phone],
    )?;
    Ok(changed == 1)
}

/// The GDPR sweep: clear the name and phone of every tag created before
/// `cutoff` (`YYYY-MM-DDTHH:MM:SS.fffZ`) that still holds one. The number
/// stays. Returns how many tags were cleared.
pub fn clear_contacts_before(conn: &Connection, cutoff: &str) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "UPDATE takeaway_tags
         SET customer_name = NULL, customer_phone = NULL,
             contact_cleared_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE created_at < ?1 AND (customer_name IS NOT NULL OR customer_phone IS NOT NULL)",
        params![cutoff],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn order(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO orders (order_type, status, user_insz, subtotal_cents, total_cents)
             VALUES ('takeaway', 'open', '85010100156', 0, 0)",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn the_first_number_of_a_day_is_the_one_asked_and_the_next_follow() {
        let conn = db();
        let (a, b) = (order(&conn), order(&conn));
        assert_eq!(
            assign(&conn, a, "2026-11-06", 61).unwrap().unwrap().number,
            61
        );
        assert_eq!(
            assign(&conn, b, "2026-11-06", 61).unwrap().unwrap().number,
            62
        );
    }

    #[test]
    fn an_order_keeps_its_number_when_asked_again() {
        let conn = db();
        let a = order(&conn);
        let first = assign(&conn, a, "2026-11-06", 61).unwrap().unwrap();
        let again = assign(&conn, a, "2026-11-06", 100).unwrap().unwrap();
        assert_eq!(first, again);
    }

    #[test]
    fn each_day_restarts_at_the_first_number() {
        let conn = db();
        let (a, b, c) = (order(&conn), order(&conn), order(&conn));
        assign(&conn, a, "2026-11-06", 61).unwrap();
        assign(&conn, b, "2026-11-06", 61).unwrap();
        assert_eq!(
            assign(&conn, c, "2026-11-07", 61).unwrap().unwrap().number,
            61
        );
    }

    #[test]
    fn raising_the_first_number_jumps_and_lowering_it_never_repeats_a_number() {
        let conn = db();
        let (a, b, c) = (order(&conn), order(&conn), order(&conn));
        assign(&conn, a, "2026-11-06", 61).unwrap();
        assert_eq!(
            assign(&conn, b, "2026-11-06", 200).unwrap().unwrap().number,
            200
        );
        assert_eq!(
            assign(&conn, c, "2026-11-06", 1).unwrap().unwrap().number,
            201
        );
    }

    #[test]
    fn a_day_that_ran_out_of_numbers_gives_none() {
        let conn = db();
        let (a, b) = (order(&conn), order(&conn));
        assert_eq!(
            assign(&conn, a, "2026-11-06", MAX_NUMBER)
                .unwrap()
                .unwrap()
                .number,
            MAX_NUMBER
        );
        assert_eq!(assign(&conn, b, "2026-11-06", 1).unwrap(), None);
    }

    #[test]
    fn the_contact_is_replaced_as_a_whole_and_cleared_by_the_sweep() {
        let conn = db();
        let a = order(&conn);
        assert!(
            !set_contact(&conn, a, Some("Dupont"), None).unwrap(),
            "no tag yet"
        );
        assign(&conn, a, "2026-11-06", 61).unwrap();
        assert!(set_contact(&conn, a, Some("Dupont"), Some("0470 12 34 56")).unwrap());
        let t = tag(&conn, a).unwrap().unwrap();
        assert_eq!(t.customer_name.as_deref(), Some("Dupont"));
        assert_eq!(t.customer_phone.as_deref(), Some("0470 12 34 56"));

        assert_eq!(
            clear_contacts_before(&conn, "2000-01-01T00:00:00.000Z").unwrap(),
            0
        );
        assert_eq!(
            clear_contacts_before(&conn, "2999-01-01T00:00:00.000Z").unwrap(),
            1
        );
        let t = tag(&conn, a).unwrap().unwrap();
        assert_eq!(
            (t.customer_name, t.customer_phone, t.number),
            (None, None, 61)
        );
        assert_eq!(
            clear_contacts_before(&conn, "2999-01-01T00:00:00.000Z").unwrap(),
            0
        );
    }
}
