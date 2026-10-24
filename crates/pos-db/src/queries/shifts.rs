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
///
/// `by` is the INSZ of who registered the clock-in when it is not the
/// employee themself (a manager), for the audit.
pub fn open_shift(
    conn: &Connection,
    user_insz: &str,
    by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO staff_shifts (user_insz, started_at, started_by)
         SELECT ?1, ?2, ?3
         WHERE NOT EXISTS (
             SELECT 1 FROM staff_shifts WHERE user_insz = ?1 AND ended_at IS NULL
         )",
        params![user_insz, now(), by],
    )?;
    Ok(())
}

/// Close the employee's open shift, if any.
///
/// `by` is the INSZ of who registered the clock-out when it is not the
/// employee themself (a manager).
pub fn close_shift(
    conn: &Connection,
    user_insz: &str,
    by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE staff_shifts SET ended_at = ?2, ended_by = ?3
         WHERE user_insz = ?1 AND ended_at IS NULL",
        params![user_insz, now(), by],
    )?;
    Ok(())
}

/// Whether the employee has an open shift.
pub fn has_open_shift(conn: &Connection, user_insz: &str) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM staff_shifts WHERE user_insz = ?1 AND ended_at IS NULL)",
        params![user_insz],
        |row| row.get(0),
    )
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
        open_shift(&conn, "85073003328", None).unwrap();
        open_shift(&conn, "85073003328", None).unwrap();
        let open = list_open_shifts(&conn).unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].user_insz, "85073003328");

        close_shift(&conn, "85073003328", None).unwrap();
        assert!(list_open_shifts(&conn).unwrap().is_empty());

        // A new clock-in starts a new shift.
        open_shift(&conn, "85073003328", None).unwrap();
        assert_eq!(list_open_shifts(&conn).unwrap().len(), 1);
    }

    #[test]
    fn closing_without_a_shift_is_a_no_op() {
        let conn = db();
        close_shift(&conn, "85073003328", None).unwrap();
        assert!(list_open_shifts(&conn).unwrap().is_empty());
    }

    #[test]
    fn the_registering_manager_is_stored_on_both_ends() {
        let conn = db();
        open_shift(&conn, "85073003328", Some("11111111111")).unwrap();
        let (started, ended): (Option<String>, Option<String>) = conn
            .query_row("SELECT started_by, ended_by FROM staff_shifts", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(started.as_deref(), Some("11111111111"));
        assert_eq!(ended, None);

        close_shift(&conn, "85073003328", Some("22222222222")).unwrap();
        let ended: Option<String> = conn
            .query_row("SELECT ended_by FROM staff_shifts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ended.as_deref(), Some("22222222222"));
    }

    #[test]
    fn a_self_registered_shift_has_no_manager() {
        let conn = db();
        open_shift(&conn, "85073003328", None).unwrap();
        close_shift(&conn, "85073003328", None).unwrap();
        let (s, e): (Option<String>, Option<String>) = conn
            .query_row("SELECT started_by, ended_by FROM staff_shifts", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((s, e), (None, None));
    }
}
