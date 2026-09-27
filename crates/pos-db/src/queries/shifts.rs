//! Local index of staff shifts. The fiscal record of a clock-in or
//! clock-out is the signed S event; this table only lets the login
//! screen show who is on shift and since when.

use chrono::Utc;
use rusqlite::{Connection, params};
use serde::Serialize;

/// An open shift: the employee clocked in and has not clocked out.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenShift {
    pub user_insz: String,
    /// UTC, RFC 3339.
    pub started_at: String,
}

fn now() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Open a shift for the employee, unless one is already open.
pub fn open_shift(conn: &Connection, user_insz: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO staff_shifts (user_insz, started_at)
         SELECT ?1, ?2
         WHERE NOT EXISTS (
             SELECT 1 FROM staff_shifts WHERE user_insz = ?1 AND ended_at IS NULL
         )",
        params![user_insz, now()],
    )?;
    Ok(())
}

/// Close the employee's open shift, if any.
pub fn close_shift(conn: &Connection, user_insz: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE staff_shifts SET ended_at = ?2 WHERE user_insz = ?1 AND ended_at IS NULL",
        params![user_insz, now()],
    )?;
    Ok(())
}

pub fn list_open_shifts(conn: &Connection) -> Result<Vec<OpenShift>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT user_insz, started_at FROM staff_shifts
         WHERE ended_at IS NULL ORDER BY started_at",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(OpenShift {
            user_insz: row.get(0)?,
            started_at: row.get(1)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn a_shift_opens_once_and_closes() {
        let conn = db();
        open_shift(&conn, "85073003328").unwrap();
        open_shift(&conn, "85073003328").unwrap();
        let open = list_open_shifts(&conn).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].user_insz, "85073003328");

        close_shift(&conn, "85073003328").unwrap();
        assert!(list_open_shifts(&conn).unwrap().is_empty());

        // A new clock-in starts a new shift.
        open_shift(&conn, "85073003328").unwrap();
        assert_eq!(list_open_shifts(&conn).unwrap().len(), 1);
    }

    #[test]
    fn closing_without_a_shift_is_a_no_op() {
        let conn = db();
        close_shift(&conn, "85073003328").unwrap();
        assert!(list_open_shifts(&conn).unwrap().is_empty());
    }
}
