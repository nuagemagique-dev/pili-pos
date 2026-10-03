//! Sign requests stored before they are sent (migration 10): at most
//! one is open, waiting for the FDM's answer.

use rusqlite::{Connection, OptionalExtension, params};

/// A request to store before sending it.
pub struct NewPendingSign<'a> {
    pub event_label: &'a str,
    pub first_ticket_no: i64,
    pub ticket_count: i64,
    pub mutation: &'a str,
    pub is_training: bool,
    pub request_body: &'a str,
    pub archive_json: &'a str,
    pub kind: &'a str,
    pub subject: &'a str,
    pub order_id: Option<i64>,
    pub context_json: &'a str,
    pub user_insz: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSign {
    pub id: i64,
    pub event_label: String,
    pub first_ticket_no: i64,
    pub ticket_count: i64,
    pub mutation: String,
    pub is_training: bool,
    pub request_body: String,
    pub archive_json: String,
    pub kind: String,
    pub subject: String,
    pub order_id: Option<i64>,
    pub context_json: String,
    pub user_insz: String,
    pub status: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub fiscal_event_id: Option<i64>,
}

/// How a request ended without being signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsigned {
    /// The FDM refused it.
    Refused,
    /// It never reached the FDM.
    NotSent,
}

pub fn insert_open(conn: &Connection, new: &NewPendingSign<'_>) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO pending_signs (event_label, first_ticket_no, ticket_count, mutation,
            is_training, request_body, archive_json, kind, subject, order_id, context_json,
            user_insz)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            new.event_label,
            new.first_ticket_no,
            new.ticket_count,
            new.mutation,
            new.is_training,
            new.request_body,
            new.archive_json,
            new.kind,
            new.subject,
            new.order_id,
            new.context_json,
            new.user_insz,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

const COLUMNS: &str = "id, event_label, first_ticket_no, ticket_count, mutation, is_training,
    request_body, archive_json, kind, subject, order_id, context_json, user_insz, status,
    attempts, last_error, fiscal_event_id";

fn from_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<PendingSign> {
    Ok(PendingSign {
        id: r.get(0)?,
        event_label: r.get(1)?,
        first_ticket_no: r.get(2)?,
        ticket_count: r.get(3)?,
        mutation: r.get(4)?,
        is_training: r.get(5)?,
        request_body: r.get(6)?,
        archive_json: r.get(7)?,
        kind: r.get(8)?,
        subject: r.get(9)?,
        order_id: r.get(10)?,
        context_json: r.get(11)?,
        user_insz: r.get(12)?,
        status: r.get(13)?,
        attempts: r.get(14)?,
        last_error: r.get(15)?,
        fiscal_event_id: r.get(16)?,
    })
}

/// The open request, if any.
pub fn open(conn: &Connection) -> rusqlite::Result<Option<PendingSign>> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM pending_signs WHERE status = 'open'"),
        [],
        from_row,
    )
    .optional()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<PendingSign>> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM pending_signs WHERE id = ?1"),
        [id],
        from_row,
    )
    .optional()
}

/// One more send of the open request, and what it got.
pub fn record_attempt(conn: &Connection, id: i64, error: Option<&str>) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE pending_signs SET attempts = attempts + 1, last_error = ?2
         WHERE id = ?1 AND status = 'open'",
        params![id, error],
    )?;
    Ok(())
}

/// The request was not signed: its ticket number was never used.
pub fn mark_unsigned(
    conn: &Connection,
    id: i64,
    how: Unsigned,
    error: &str,
) -> rusqlite::Result<()> {
    let status = match how {
        Unsigned::Refused => "refused",
        Unsigned::NotSent => "not_sent",
    };
    conn.execute(
        "UPDATE pending_signs SET status = ?2, last_error = ?3,
            resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1 AND status = 'open'",
        params![id, status, error],
    )?;
    Ok(())
}

/// The request was signed and its event archived (call in that
/// transaction). `context_json` replaces the stored context.
pub fn mark_signed(
    conn: &Connection,
    id: i64,
    fiscal_event_id: i64,
    context_json: &str,
) -> rusqlite::Result<()> {
    let changed = conn.execute(
        "UPDATE pending_signs SET status = 'signed', fiscal_event_id = ?2, context_json = ?3,
            last_error = NULL, resolved_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1 AND status = 'open'",
        params![id, fiscal_event_id, context_json],
    )?;
    if changed == 1 {
        Ok(())
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

/// Whether the order has a request waiting for its answer.
pub fn order_has_open(conn: &Connection, order_id: i64) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pending_signs WHERE order_id = ?1 AND status = 'open')",
        [order_id],
        |r| r.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn
    }

    fn new(subject: &str) -> NewPendingSign<'_> {
        NewPendingSign {
            event_label: "N",
            first_ticket_no: 7,
            ticket_count: 1,
            mutation: "signSale",
            is_training: false,
            request_body: "{}",
            archive_json: "[]",
            kind: "sale",
            subject,
            order_id: Some(3),
            context_json: r#"{"email":"a@b.be"}"#,
            user_insz: "12345678901",
        }
    }

    fn archived_event(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id,
                user_insz, event_data_json) VALUES ('N', 7, 'P', 'T', 'U', '{}')",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn one_request_is_open_at_a_time() {
        let conn = conn();
        let id = insert_open(&conn, &new("3")).unwrap();
        assert!(insert_open(&conn, &new("4")).is_err());
        assert_eq!(open(&conn).unwrap().unwrap().id, id);
        assert!(order_has_open(&conn, 3).unwrap());

        mark_unsigned(&conn, id, Unsigned::Refused, "BUFFER_FULL").unwrap();
        assert!(open(&conn).unwrap().is_none());
        assert!(!order_has_open(&conn, 3).unwrap());
        insert_open(&conn, &new("4")).unwrap();
    }

    #[test]
    fn what_was_sent_never_changes_nor_goes() {
        let conn = conn();
        let id = insert_open(&conn, &new("3")).unwrap();
        for sql in [
            "UPDATE pending_signs SET request_body = 'x'",
            "UPDATE pending_signs SET first_ticket_no = 8",
            "UPDATE pending_signs SET archive_json = '[1]'",
            "UPDATE pending_signs SET context_json = '{}'",
            "DELETE FROM pending_signs",
        ] {
            assert!(conn.execute(sql, []).is_err(), "{sql}");
        }
        record_attempt(&conn, id, Some("timeout")).unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(
            (row.attempts, row.last_error.as_deref()),
            (1, Some("timeout"))
        );
    }

    #[test]
    fn a_resolved_request_stays_resolved() {
        let conn = conn();
        let id = insert_open(&conn, &new("3")).unwrap();
        let event = archived_event(&conn);
        mark_signed(&conn, id, event, "{}").unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, "signed");
        assert_eq!(row.fiscal_event_id, Some(event));
        assert_eq!(row.context_json, "{}");

        assert!(mark_signed(&conn, id, event, "{}").is_err());
        assert!(
            conn.execute("UPDATE pending_signs SET status = 'open'", [])
                .is_err()
        );
        assert!(
            conn.execute("UPDATE pending_signs SET attempts = 9", [])
                .is_err()
        );
    }

    #[test]
    fn no_event_joins_a_closed_booking_period() {
        let conn = conn();
        let event = |period: &str, n: i64| {
            conn.execute(
                "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id,
                    user_insz, event_data_json) VALUES ('N', ?1, 'P', 'T', 'U', ?2)",
                params![n, format!(r#"{{"bookingPeriodId":"{period}"}}"#)],
            )
        };
        conn.execute(
            "INSERT INTO booking_periods (id, booking_date) VALUES ('bp-1', '2026-07-03')",
            [],
        )
        .unwrap();
        event("bp-1", 1).unwrap();
        let z = conn.last_insert_rowid();
        crate::queries::booking::close_period(&conn, "bp-1", z).unwrap();
        let refused = event("bp-1", 2).unwrap_err();
        assert!(refused.to_string().contains("booking period is closed"));
        event("bp-2", 2).unwrap();
    }
}
