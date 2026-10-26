//! The owner settings store (migration 60): one JSON document per key
//! (`alerts`, later `courses`, `gift_vouchers`) and the append-only
//! log of their changes, plus the owner alert log.
//!
//! This module only moves text. Validation and typing live in the
//! engine (`pos_engine::commands::owner_settings`), which owns the
//! `serde(deny_unknown_fields)` structures. An absent key means the
//! feature's defaults (off).

use rusqlite::{Connection, OptionalExtension, params};

/// The stored document of `key`, `None` when it was never set.
pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT value_json FROM owner_settings WHERE key = ?1",
        params![key],
        |r| r.get(0),
    )
    .optional()
}

/// Store `value_json` under `key` and log it, in one transaction.
/// Returns whether the stored value changed (an identical value writes
/// nothing, so the log lists real changes only).
pub fn set(
    conn: &Connection,
    key: &str,
    value_json: &str,
    changed_by: Option<&str>,
) -> Result<bool, rusqlite::Error> {
    super::with_savepoint(conn, |tx| {
        if get(&tx, key)?.as_deref() == Some(value_json) {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO owner_settings (key, value_json, changed_at, changed_by)
             VALUES (?1, ?2, datetime('now'), ?3)
             ON CONFLICT(key) DO UPDATE SET
                 value_json = excluded.value_json,
                 changed_at = excluded.changed_at,
                 changed_by = excluded.changed_by",
            params![key, value_json, changed_by],
        )?;
        tx.execute(
            "INSERT INTO owner_settings_log (key, value_json, changed_by) VALUES (?1, ?2, ?3)",
            params![key, value_json, changed_by],
        )?;
        Ok(true)
    })
}

/// One change of a setting, newest first in [`history`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SettingChange {
    pub key: String,
    pub value_json: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

/// The latest changes of `key`, newest first.
pub fn history(
    conn: &Connection,
    key: &str,
    limit: i64,
) -> Result<Vec<SettingChange>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT key, value_json, changed_by, changed_at FROM owner_settings_log
         WHERE key = ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![key, limit], |r| {
        Ok(SettingChange {
            key: r.get(0)?,
            value_json: r.get(1)?,
            changed_by: r.get(2)?,
            changed_at: r.get(3)?,
        })
    })?;
    rows.collect()
}

/// One row of the owner alert log.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AlertLogRow {
    pub id: i64,
    pub kind: String,
    pub transition: String,
    pub detail: Option<String>,
    pub outcome: String,
    pub http_status: Option<i64>,
    pub created_at: String,
}

/// Append a row to the alert log, stamped `created_at` (SQLite
/// datetime text, UTC: the worker passes its own clock so a restart
/// and the hourly cap read the time the worker decided on).
pub fn log_alert(
    conn: &Connection,
    kind: &str,
    transition: &str,
    detail: Option<&str>,
    outcome: &str,
    http_status: Option<i64>,
    created_at: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO owner_alert_log (kind, transition, detail, outcome, http_status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![kind, transition, detail, outcome, http_status, created_at],
    )?;
    Ok(())
}

/// The latest rows of the alert log, newest first.
pub fn list_alert_log(conn: &Connection, limit: i64) -> Result<Vec<AlertLogRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, transition, detail, outcome, http_status, created_at
         FROM owner_alert_log ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], alert_row)?;
    rows.collect()
}

fn alert_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<AlertLogRow> {
    Ok(AlertLogRow {
        id: r.get(0)?,
        kind: r.get(1)?,
        transition: r.get(2)?,
        detail: r.get(3)?,
        outcome: r.get(4)?,
        http_status: r.get(5)?,
        created_at: r.get(6)?,
    })
}

/// The last row of `kind` that is a raise, a reminder or a resolve
/// (suppressed and failed rows count too: they say what the worker
/// decided), or `None`. A restart reads it to know an alert is open.
pub fn last_alert_row(
    conn: &Connection,
    kind: &str,
) -> Result<Option<AlertLogRow>, rusqlite::Error> {
    conn.query_row(
        "SELECT id, kind, transition, detail, outcome, http_status, created_at
         FROM owner_alert_log WHERE kind = ?1 AND transition <> 'test'
         ORDER BY id DESC LIMIT 1",
        params![kind],
        alert_row,
    )
    .optional()
}

/// The times of the messages actually sent since `since` (SQLite
/// datetime text, UTC), oldest first, test messages left out: the
/// global hourly cap counts them, also after a restart.
pub fn sent_times_since(conn: &Connection, since: &str) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT created_at FROM owner_alert_log
         WHERE outcome = 'sent' AND transition <> 'test' AND created_at >= ?1
         ORDER BY created_at, id",
    )?;
    let rows = stmt.query_map(params![since], |r| r.get(0))?;
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
    fn set_works_inside_a_callers_transaction_and_follows_its_rollback() {
        let conn = db();
        conn.execute_batch("BEGIN").unwrap();
        assert!(set(&conn, "alerts", "{\"a\":1}", None).unwrap());
        conn.execute_batch("ROLLBACK").unwrap();
        assert_eq!(get(&conn, "alerts").unwrap(), None);
    }

    #[test]
    fn an_absent_key_reads_none() {
        assert_eq!(get(&db(), "alerts").unwrap(), None);
    }

    #[test]
    fn set_stores_and_logs_in_one_step_and_skips_an_identical_value() {
        let conn = db();
        assert!(set(&conn, "alerts", r#"{"a":1}"#, Some("00000000029")).unwrap());
        assert_eq!(get(&conn, "alerts").unwrap().as_deref(), Some(r#"{"a":1}"#));
        assert!(!set(&conn, "alerts", r#"{"a":1}"#, Some("00000000029")).unwrap());
        assert!(set(&conn, "alerts", r#"{"a":2}"#, None).unwrap());
        let h = history(&conn, "alerts", 10).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].value_json, r#"{"a":2}"#);
        assert_eq!(h[1].changed_by.as_deref(), Some("00000000029"));
        assert!(history(&conn, "courses", 10).unwrap().is_empty());
    }

    #[test]
    fn the_alert_log_reads_back_newest_first() {
        let conn = db();
        let t = "2026-10-17 10:00:00";
        log_alert(&conn, "fdm", "raised", None, "sent", Some(200), t).unwrap();
        log_alert(&conn, "test", "test", Some("x"), "failed", None, t).unwrap();
        log_alert(&conn, "fdm", "resolved", None, "sent", Some(200), t).unwrap();
        let rows = list_alert_log(&conn, 2).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].transition, "resolved");
        assert_eq!(rows[1].kind, "test");
        let last = last_alert_row(&conn, "fdm").unwrap().unwrap();
        assert_eq!(last.transition, "resolved");
        assert!(last_alert_row(&conn, "printer").unwrap().is_none());
        // A test row never counts as the state of a kind.
        assert!(last_alert_row(&conn, "test").unwrap().is_none());
        assert_eq!(
            sent_times_since(&conn, "2000-01-01 00:00:00").unwrap(),
            vec![t, t]
        );
        assert!(
            sent_times_since(&conn, "2999-01-01 00:00:00")
                .unwrap()
                .is_empty()
        );
    }
}
