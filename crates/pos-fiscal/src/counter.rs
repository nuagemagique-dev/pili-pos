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
    pub fn next(
        conn: &rusqlite::Connection,
        event_label: EventLabel,
    ) -> Result<i64, rusqlite::Error> {
        let label = serde_json::to_string(&event_label)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        // Ensure the counter table exists
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fiscal_counters (
                event_label TEXT PRIMARY KEY,
                current_value INTEGER NOT NULL DEFAULT 0
            )",
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

    /// What `next()` *would* return without mutating the DB.
    ///
    /// Used by command handlers to reserve a ticket number for the
    /// FDM request **before** committing the increment. If the FDM
    /// call fails, the counter stays untouched and a retry sees the
    /// same value - which is the only way to honour the SCE 2.0
    /// "strictly sequential, no gaps" requirement when the network
    /// is unreliable.
    ///
    /// The actual commit (via `next()`) happens inside the same
    /// transaction as the `fiscal_events` archive - that pair is
    /// what ensures atomicity. See
    /// `docs/SUBMISSION_ARTIFACTS/COUNTER_STRATEGY.md`.
    pub fn peek_next(
        conn: &rusqlite::Connection,
        event_label: EventLabel,
    ) -> Result<i64, rusqlite::Error> {
        let cur = Self::current(conn, event_label)?;
        Ok(if cur >= MAX_COUNTER { 1 } else { cur + 1 })
    }

    /// Get the current counter value without incrementing.
    pub fn current(
        conn: &rusqlite::Connection,
        event_label: EventLabel,
    ) -> Result<i64, rusqlite::Error> {
        let label = serde_json::to_string(&event_label)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fiscal_counters (
                event_label TEXT PRIMARY KEY,
                current_value INTEGER NOT NULL DEFAULT 0
            )",
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
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .unwrap();
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

    #[test]
    fn peek_next_does_not_mutate() {
        let conn = setup();
        // Initial: peek returns 1, doesn't advance.
        assert_eq!(FiscalCounter::peek_next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 0);
        assert_eq!(FiscalCounter::peek_next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 0);
        // After next(), peek shows the FOLLOWING value.
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::peek_next(&conn, EventLabel::N).unwrap(), 2);
        assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 1);
    }

    #[test]
    fn peek_next_matches_next_when_committed() {
        // Property the command handlers rely on: a peek followed by
        // a next() (with nothing else in between) must produce the
        // same value.
        let conn = setup();
        for _ in 0..5 {
            let speculative = FiscalCounter::peek_next(&conn, EventLabel::N).unwrap();
            let actual = FiscalCounter::next(&conn, EventLabel::N).unwrap();
            assert_eq!(speculative, actual);
        }
    }

    #[test]
    fn counter_is_strictly_sequential_with_no_gaps() {
        let conn = setup();
        let mut seen = Vec::new();
        for _ in 0..50 {
            seen.push(FiscalCounter::next(&conn, EventLabel::N).unwrap());
        }
        let expected: Vec<i64> = (1..=50).collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn wraps_to_one_after_max_counter() {
        // SCE 2.0 spec: 999_999_999 is the last value before wrapping
        // back to 1. We seed the row directly instead of looping a
        // billion times.
        let conn = setup();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS fiscal_counters (
                event_label TEXT PRIMARY KEY,
                current_value INTEGER NOT NULL DEFAULT 0
            )",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO fiscal_counters (event_label, current_value) VALUES ('N', 999999999)",
            [],
        )
        .unwrap();

        assert_eq!(FiscalCounter::peek_next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 1);
        assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 2);
    }

    #[test]
    fn every_event_label_has_its_own_independent_sequence() {
        let conn = setup();
        let labels = [
            EventLabel::N,
            EventLabel::P,
            EventLabel::T,
            EventLabel::C,
            EventLabel::I,
            EventLabel::F,
            EventLabel::S,
            EventLabel::R,
        ];
        for label in labels {
            assert_eq!(FiscalCounter::next(&conn, label).unwrap(), 1, "{label:?}");
        }
        // Advancing N further must not affect any other label's value.
        FiscalCounter::next(&conn, EventLabel::N).unwrap();
        FiscalCounter::next(&conn, EventLabel::N).unwrap();
        assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 3);
        for label in labels.iter().filter(|l| **l != EventLabel::N) {
            assert_eq!(
                FiscalCounter::current(&conn, *label).unwrap(),
                1,
                "{label:?}"
            );
        }
    }

    #[test]
    fn counter_persists_across_connections_to_the_same_file() {
        // Crash-safety requirement (CLAUDE.md: "Sequence number
        // management - persist in SQLite, crash-safe"): the counter
        // must survive a process restart, i.e. a fresh `Connection`
        // opened against the same on-disk database file.
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "pili-pos-counter-test-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("pos.db");

        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
            assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 1);
            assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 2);
            // conn dropped here - simulates the process exiting.
        }
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            assert_eq!(FiscalCounter::current(&conn, EventLabel::N).unwrap(), 2);
            assert_eq!(FiscalCounter::next(&conn, EventLabel::N).unwrap(), 3);
        }

        // Cleanup - WAL mode may leave -wal/-shm sidecar files.
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", db_path.display()));
        }
        let _ = std::fs::remove_dir(&dir);
    }
}
