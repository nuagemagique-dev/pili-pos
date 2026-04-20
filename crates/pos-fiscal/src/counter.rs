use crate::types::EventLabel;

/// Maximum value before wrapping back to 1 (SCE 2.0 spec: 999999999).
const MAX_COUNTER: i64 = 999_999_999;

/// Manages sequential fiscal ticket numbering.
///
/// The counter is persisted in SQLite via the `fiscal_counters` table.
/// Each event type has its own independent counter per the spec:
/// "Continuous numbering per event type, per terminal, or general across all"
///
/// We use per-event-type counters for maximum traceability.
pub struct FiscalCounter;

impl FiscalCounter {
    /// Get the next sequence number for a given event label.
    /// This MUST be called inside a transaction to ensure crash safety.
    pub fn next(conn: &rusqlite::Connection, event_label: EventLabel) -> Result<i64, rusqlite::Error> {
        let label = serde_json::to_string(&event_label)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        // Ensure the counter table exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fiscal_counters (
                event_label TEXT PRIMARY KEY,
                current_value INTEGER NOT NULL DEFAULT 0
            )"
        )?;

        // Upsert: increment or initialize
        conn.execute(
            "INSERT INTO fiscal_counters (event_label, current_value)
             VALUES (?1, 1)
             ON CONFLICT(event_label)
             DO UPDATE SET current_value =
                CASE WHEN current_value >= ?2 THEN 1
                     ELSE current_value + 1
                END",
            rusqlite::params![label, MAX_COUNTER],
        )?;

        let value: i64 = conn.query_row(
            "SELECT current_value FROM fiscal_counters WHERE event_label = ?1",
            rusqlite::params![label],
            |row| row.get(0),
        )?;

        Ok(value)
    }

    /// Get the current counter value without incrementing.
    pub fn current(conn: &rusqlite::Connection, event_label: EventLabel) -> Result<i64, rusqlite::Error> {
        let label = serde_json::to_string(&event_label)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fiscal_counters (
                event_label TEXT PRIMARY KEY,
                current_value INTEGER NOT NULL DEFAULT 0
            )"
        )?;

        let value: i64 = conn
            .query_row(
                "SELECT COALESCE(current_value, 0) FROM fiscal_counters WHERE event_label = ?1",
                rusqlite::params![label],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    #[test]
    fn counter_starts_at_one() {
        let conn = setup();
        let n = FiscalCounter::next(&conn, EventLabel::N).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn counter_increments() {
        let conn = setup();
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 2);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 3);
    }

    #[test]
    fn counters_are_per_event_type() {
        let conn = setup();
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::P).unwrap(), 1);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 2);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::P).unwrap(), 2);
    }

    #[test]
    fn current_returns_zero_for_new() {
        let conn = setup();
        assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 0);
    }

    #[test]
    fn current_reflects_last_next() {
        let conn = setup();
        FiscalCounter::next(&conn, EventLabel::N).unwrap();
        FiscalCounter::next(&conn, EventLabel::N).unwrap();
        assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 2);
    }
}
