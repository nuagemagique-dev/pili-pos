//! Schema of the staff badges (NFC or USB reader), from migration 70.
//!
//! Part of the certified binary on every installation; the tables are
//! written only when a manager turns the badge reader on in Settings
//! (`badge_settings.reader`, off by default).

/// v70 - staff badges.
///
/// `badge_settings` holds the one setting row. `staff_badges` links a
/// keyed hash of a badge number (never the number) to an employee; a
/// row is never deleted and its only change is one revocation.
/// `badge_log` (links, revocations, settings changes) and `badge_scans`
/// (every scan that was not an ignored repeat) are append-only.
pub(crate) const MIGRATION_070_STAFF_BADGES: &str = r#"
CREATE TABLE IF NOT EXISTS badge_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    reader TEXT NOT NULL DEFAULT 'off' CHECK (reader IN ('off', 'nfc', 'usb')),
    login_enabled INTEGER NOT NULL DEFAULT 0 CHECK (login_enabled IN (0, 1)),
    wedge_max_gap_ms INTEGER NOT NULL DEFAULT 50 CHECK (wedge_max_gap_ms BETWEEN 10 AND 200),
    wedge_min_len INTEGER NOT NULL DEFAULT 6 CHECK (wedge_min_len BETWEEN 4 AND 32),
    wedge_max_len INTEGER NOT NULL DEFAULT 32 CHECK (wedge_max_len BETWEEN 4 AND 64),
    -- First 16 hex chars of HMAC(badge key, "pili-badge-check/1"); NULL until a key exists.
    key_check TEXT,
    changed_at TEXT NOT NULL DEFAULT (datetime('now')),
    changed_by TEXT,
    CHECK (wedge_min_len <= wedge_max_len)
);
INSERT OR IGNORE INTO badge_settings (id) VALUES (1);

CREATE TABLE IF NOT EXISTS staff_badges (
    id INTEGER PRIMARY KEY,
    badge_hash TEXT NOT NULL CHECK (length(badge_hash) = 64),
    user_insz TEXT NOT NULL REFERENCES users(insz),
    linked_at TEXT NOT NULL DEFAULT (datetime('now')),
    linked_by TEXT NOT NULL,
    revoked_at TEXT,
    -- NULL with the reason key_changed (the engine revoked it).
    revoked_by TEXT,
    revoke_reason TEXT CHECK (revoke_reason IS NULL OR revoke_reason IN ('manager', 'deactivated', 'key_changed'))
);
CREATE UNIQUE INDEX IF NOT EXISTS staff_badges_active_badge
    ON staff_badges(badge_hash) WHERE revoked_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS staff_badges_active_user
    ON staff_badges(user_insz) WHERE revoked_at IS NULL;

-- Never deleted; the only change ever made is one revocation.
CREATE TRIGGER IF NOT EXISTS staff_badges_no_delete
BEFORE DELETE ON staff_badges
BEGIN
    SELECT RAISE(ABORT, 'staff_badges rows are never deleted');
END;

CREATE TRIGGER IF NOT EXISTS staff_badges_revoke_only
BEFORE UPDATE ON staff_badges
WHEN OLD.revoked_at IS NOT NULL OR NEW.revoked_at IS NULL OR NEW.revoke_reason IS NULL
  OR NEW.badge_hash IS NOT OLD.badge_hash OR NEW.user_insz IS NOT OLD.user_insz
  OR NEW.linked_at IS NOT OLD.linked_at OR NEW.linked_by IS NOT OLD.linked_by
BEGIN
    SELECT RAISE(ABORT, 'staff_badges: only one revocation may change a row');
END;

-- Append-only audit of links, revocations and settings changes.
CREATE TABLE IF NOT EXISTS badge_log (
    id INTEGER PRIMARY KEY,
    action TEXT NOT NULL CHECK (action IN ('link', 'revoke', 'settings')),
    badge_id INTEGER REFERENCES staff_badges(id),
    user_insz TEXT,
    -- revoke: the reason; settings: JSON of the new values.
    detail TEXT,
    -- Manager INSZ; NULL for the engine (key_changed).
    changed_by TEXT,
    changed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS badge_log_no_update
BEFORE UPDATE ON badge_log
BEGIN
    SELECT RAISE(ABORT, 'badge_log is append-only');
END;

CREATE TRIGGER IF NOT EXISTS badge_log_no_delete
BEFORE DELETE ON badge_log
BEGIN
    SELECT RAISE(ABORT, 'badge_log is append-only');
END;

-- Append-only: every scan that was not an ignored repeat.
CREATE TABLE IF NOT EXISTS badge_scans (
    id INTEGER PRIMARY KEY,
    outcome TEXT NOT NULL CHECK (outcome IN
        ('clock_in', 'clock_out', 'presence_failed', 'login', 'pin_required', 'unknown', 'revoked', 'refused')),
    user_insz TEXT,
    -- First 12 hex chars of the badge hash, never the badge number.
    badge_ref TEXT NOT NULL,
    -- The refusal code or the FDM error.
    detail TEXT,
    scanned_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TRIGGER IF NOT EXISTS badge_scans_no_update
BEFORE UPDATE ON badge_scans
BEGIN
    SELECT RAISE(ABORT, 'badge_scans is append-only');
END;

CREATE TRIGGER IF NOT EXISTS badge_scans_no_delete
BEFORE DELETE ON badge_scans
BEGIN
    SELECT RAISE(ABORT, 'badge_scans is append-only');
END;
"#;

#[cfg(test)]
mod tests {
    use crate::migrate::run_migrations;
    use rusqlite::Connection;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO users (insz, name, role) VALUES ('11111111111', 'A', 'waiter');
             INSERT INTO users (insz, name, role) VALUES ('22222222222', 'B', 'waiter');",
        )
        .unwrap();
        conn
    }

    fn link(conn: &Connection, hash: &str, insz: &str) -> rusqlite::Result<usize> {
        conn.execute(
            "INSERT INTO staff_badges (badge_hash, user_insz, linked_by) VALUES (?1, ?2, 'm')",
            [hash, insz],
        )
    }

    #[test]
    fn staff_badges_migration_has_one_default_settings_row() {
        let conn = db();
        let row: (String, i64, i64, i64, i64, Option<String>) = conn
            .query_row(
                "SELECT reader, login_enabled, wedge_max_gap_ms, wedge_min_len, wedge_max_len, key_check
                 FROM badge_settings",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .unwrap();
        assert_eq!(row, ("off".to_string(), 0, 50, 6, 32, None));
        // Exactly one row, and no second one.
        assert!(
            conn.execute("INSERT INTO badge_settings (id) VALUES (2)", [])
                .is_err()
        );
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM badge_settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn staff_badges_migration_checks_the_settings_values() {
        let conn = db();
        for bad in [
            "reader = 'bluetooth'",
            "login_enabled = 2",
            "wedge_max_gap_ms = 9",
            "wedge_max_gap_ms = 201",
            "wedge_min_len = 3",
            "wedge_min_len = 33",
            "wedge_max_len = 3",
            "wedge_max_len = 65",
            // min above max
            "wedge_min_len = 20, wedge_max_len = 10",
        ] {
            assert!(
                conn.execute(&format!("UPDATE badge_settings SET {bad}"), [])
                    .is_err(),
                "{bad}"
            );
        }
        for good in [
            "reader = 'usb'",
            "reader = 'nfc'",
            "wedge_max_gap_ms = 10",
            "wedge_max_gap_ms = 200",
            "wedge_min_len = 4, wedge_max_len = 4",
            "wedge_min_len = 32, wedge_max_len = 64",
        ] {
            conn.execute(&format!("UPDATE badge_settings SET {good}"), [])
                .unwrap_or_else(|e| panic!("{good}: {e}"));
        }
    }

    #[test]
    fn staff_badges_migration_checks_hash_length_and_reason() {
        let conn = db();
        assert!(link(&conn, "abc", "11111111111").is_err(), "short hash");
        link(&conn, HASH_A, "11111111111").unwrap();
        assert!(
            conn.execute(
                "UPDATE staff_badges SET revoked_at = 'x', revoke_reason = 'other'",
                []
            )
            .is_err()
        );
        assert!(
            conn.execute(
                "UPDATE staff_badges SET revoked_at = 'x', revoke_reason = NULL",
                []
            )
            .is_err(),
            "a revocation names its reason"
        );
        for reason in ["manager", "deactivated", "key_changed"] {
            let conn = db();
            link(&conn, HASH_A, "11111111111").unwrap();
            conn.execute(
                "UPDATE staff_badges SET revoked_at = 'x', revoke_reason = ?1",
                [reason],
            )
            .unwrap();
        }
    }

    #[test]
    fn staff_badges_migration_allows_one_active_badge_per_user_and_per_hash() {
        let conn = db();
        link(&conn, HASH_A, "11111111111").unwrap();
        assert!(
            link(&conn, HASH_B, "11111111111").is_err(),
            "a second active badge for the same employee"
        );
        assert!(
            link(&conn, HASH_A, "22222222222").is_err(),
            "the same badge active for two employees"
        );
        link(&conn, HASH_B, "22222222222").unwrap();

        // After a revocation the badge and the employee are free again.
        conn.execute(
            "UPDATE staff_badges SET revoked_at = 'x', revoked_by = 'm', revoke_reason = 'manager'
             WHERE user_insz = '11111111111'",
            [],
        )
        .unwrap();
        link(&conn, HASH_A, "11111111111").unwrap();
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM staff_badges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 3, "the revoked row is kept");
    }

    #[test]
    fn staff_badges_migration_never_deletes_and_revokes_once() {
        let conn = db();
        link(&conn, HASH_A, "11111111111").unwrap();
        assert!(conn.execute("DELETE FROM staff_badges", []).is_err());
        // Changing anything but the revocation is refused.
        for change in [
            "badge_hash = 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc'",
            "user_insz = '22222222222'",
            "linked_by = 'z'",
            "linked_at = 'z'",
            "revoked_at = NULL",
        ] {
            assert!(
                conn.execute(&format!("UPDATE staff_badges SET {change}"), [])
                    .is_err(),
                "{change}"
            );
        }
        conn.execute(
            "UPDATE staff_badges SET revoked_at = 'x', revoked_by = NULL, revoke_reason = 'key_changed'",
            [],
        )
        .unwrap();
        assert!(
            conn.execute(
                "UPDATE staff_badges SET revoked_at = 'y', revoke_reason = 'manager'",
                []
            )
            .is_err(),
            "a second revocation"
        );
        assert!(conn.execute("DELETE FROM staff_badges", []).is_err());
    }

    #[test]
    fn staff_badges_migration_logs_are_append_only() {
        let conn = db();
        conn.execute(
            "INSERT INTO badge_log (action, user_insz, detail, changed_by) VALUES ('settings', NULL, '{}', 'm')",
            [],
        )
        .unwrap();
        assert!(
            conn.execute("INSERT INTO badge_log (action) VALUES ('other')", [])
                .is_err()
        );
        assert!(
            conn.execute("UPDATE badge_log SET detail = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM badge_log", []).is_err());

        conn.execute(
            "INSERT INTO badge_scans (outcome, badge_ref) VALUES ('unknown', 'aaaaaaaaaaaa')",
            [],
        )
        .unwrap();
        assert!(
            conn.execute(
                "INSERT INTO badge_scans (outcome, badge_ref) VALUES ('ignored', 'aaaaaaaaaaaa')",
                []
            )
            .is_err(),
            "ignored repeats are never stored"
        );
        assert!(
            conn.execute("UPDATE badge_scans SET detail = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM badge_scans", []).is_err());
    }

    #[test]
    fn staff_badges_migration_is_not_rerun_on_a_reopened_database() {
        let conn = db();
        conn.execute("UPDATE badge_settings SET reader = 'usb'", [])
            .unwrap();
        run_migrations(&conn).unwrap();
        let reader: String = conn
            .query_row("SELECT reader FROM badge_settings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(reader, "usb", "the settings row is untouched");
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations_applied WHERE version = 70",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);
    }
}
