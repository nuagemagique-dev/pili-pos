//! Staff badges (migration 70): the setting, the links between a keyed
//! hash of a badge and an employee, their audit log and the scan
//! journal. See `schema_badges`.
//!
//! This module never sees a badge number: callers pass the keyed hash
//! (`pos_engine::badge_key::hash_token`). Functions that write several
//! rows take a plain `&Connection`; call them inside a transaction when
//! they must stay together.

use pos_core::badge::{BadgeReader, BadgeSettings};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

/// Why a badge was revoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevokeReason {
    /// A manager removed it.
    Manager,
    /// Its employee was deactivated.
    Deactivated,
    /// The installation's badge key is not the one the badge was linked
    /// under (a restore on new hardware): the hash would never match.
    KeyChanged,
}

impl RevokeReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manager => "manager",
            Self::Deactivated => "deactivated",
            Self::KeyChanged => "key_changed",
        }
    }
}

/// Run `f` atomically: all its writes or none. A savepoint, so it also
/// works inside a transaction the caller started (the engine groups a
/// link with its key check, a deactivation with its revocation).
pub fn atomic<T>(
    conn: &Connection,
    f: impl FnOnce(&Connection) -> Result<T, rusqlite::Error>,
) -> Result<T, rusqlite::Error> {
    super::with_savepoint(conn, f)
}

fn conversion(e: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::from(e))
}

/// The setting row (always present from migration 70).
pub fn settings(conn: &Connection) -> Result<BadgeSettings, rusqlite::Error> {
    conn.query_row(
        "SELECT reader, login_enabled, wedge_max_gap_ms, wedge_min_len, wedge_max_len
         FROM badge_settings WHERE id = 1",
        [],
        |r| {
            let reader: String = r.get(0)?;
            Ok(BadgeSettings {
                reader: reader.parse::<BadgeReader>().map_err(conversion)?,
                login_enabled: r.get::<_, i64>(1)? != 0,
                wedge_max_gap_ms: r.get(2)?,
                wedge_min_len: r.get(3)?,
                wedge_max_len: r.get(4)?,
            })
        },
    )
}

/// Store the setting; when it differs from the stored one, log the new
/// values (JSON, action `settings`). Returns whether anything changed.
/// The values must already be valid (`BadgeSettings::validate`); the
/// table CHECKs refuse the rest.
pub fn set_settings(
    conn: &Connection,
    new: &BadgeSettings,
    by: &str,
) -> Result<bool, rusqlite::Error> {
    atomic(conn, |tx| {
        if settings(tx)? == *new {
            return Ok(false);
        }
        tx.execute(
            "UPDATE badge_settings
         SET reader = ?1, login_enabled = ?2, wedge_max_gap_ms = ?3,
             wedge_min_len = ?4, wedge_max_len = ?5,
             changed_at = datetime('now'), changed_by = ?6
         WHERE id = 1",
            params![
                new.reader.as_str(),
                i64::from(new.login_enabled),
                new.wedge_max_gap_ms,
                new.wedge_min_len,
                new.wedge_max_len,
                by
            ],
        )?;
        let detail = serde_json::to_string(new).map_err(|e| conversion(e.to_string()))?;
        log(tx, "settings", None, None, Some(&detail), Some(by))?;
        Ok(true)
    })
}

/// The stored key check (`None` until a key exists).
pub fn key_check(conn: &Connection) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT key_check FROM badge_settings WHERE id = 1",
        [],
        |r| r.get(0),
    )
}

pub fn set_key_check(conn: &Connection, check: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE badge_settings SET key_check = ?1 WHERE id = 1",
        params![check],
    )?;
    Ok(())
}

/// A link between a badge hash and an employee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub id: i64,
    pub user_insz: String,
    pub revoked: bool,
}

fn link_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Link> {
    Ok(Link {
        id: r.get(0)?,
        user_insz: r.get(1)?,
        revoked: r.get::<_, Option<String>>(2)?.is_some(),
    })
}

/// The employee an active badge belongs to.
pub fn active_link_by_hash(conn: &Connection, hash: &str) -> Result<Option<Link>, rusqlite::Error> {
    conn.query_row(
        "SELECT id, user_insz, revoked_at FROM staff_badges
         WHERE badge_hash = ?1 AND revoked_at IS NULL",
        params![hash],
        link_row,
    )
    .optional()
}

/// The link of a badge hash, active first, else the latest revoked one:
/// how a scan tells an unknown badge from a removed one.
pub fn any_link_by_hash(conn: &Connection, hash: &str) -> Result<Option<Link>, rusqlite::Error> {
    conn.query_row(
        "SELECT id, user_insz, revoked_at FROM staff_badges
         WHERE badge_hash = ?1
         ORDER BY revoked_at IS NULL DESC, id DESC LIMIT 1",
        params![hash],
        link_row,
    )
    .optional()
}

/// The active badge of an employee.
pub fn active_link_by_user(conn: &Connection, insz: &str) -> Result<Option<Link>, rusqlite::Error> {
    conn.query_row(
        "SELECT id, user_insz, revoked_at FROM staff_badges
         WHERE user_insz = ?1 AND revoked_at IS NULL",
        params![insz],
        link_row,
    )
    .optional()
}

/// Link a badge hash to an employee and log it. Returns the badge id.
/// The partial unique indexes refuse a second active badge for the
/// employee or for the hash; the engine checks both first.
pub fn link(conn: &Connection, hash: &str, insz: &str, by: &str) -> Result<i64, rusqlite::Error> {
    atomic(conn, |tx| {
        tx.execute(
            "INSERT INTO staff_badges (badge_hash, user_insz, linked_by) VALUES (?1, ?2, ?3)",
            params![hash, insz, by],
        )?;
        let id = tx.last_insert_rowid();
        log(tx, "link", Some(id), Some(insz), None, Some(by))?;
        Ok(id)
    })
}

/// Revoke the active badge of an employee and log it (the reason is the
/// log detail). `by` is the manager, `None` for the engine. Returns
/// whether the employee had an active badge.
pub fn revoke(
    conn: &Connection,
    insz: &str,
    reason: RevokeReason,
    by: Option<&str>,
) -> Result<bool, rusqlite::Error> {
    atomic(conn, |tx| revoke_in(tx, insz, reason, by))
}

fn revoke_in(
    conn: &Connection,
    insz: &str,
    reason: RevokeReason,
    by: Option<&str>,
) -> Result<bool, rusqlite::Error> {
    let Some(link) = active_link_by_user(conn, insz)? else {
        return Ok(false);
    };
    conn.execute(
        "UPDATE staff_badges
         SET revoked_at = datetime('now'), revoked_by = ?2, revoke_reason = ?3
         WHERE id = ?1",
        params![link.id, by, reason.as_str()],
    )?;
    log(
        conn,
        "revoke",
        Some(link.id),
        Some(insz),
        Some(reason.as_str()),
        by,
    )?;
    Ok(true)
}

/// Revoke every active badge (one log row each). Returns how many.
pub fn revoke_all_active(
    conn: &Connection,
    reason: RevokeReason,
    by: Option<&str>,
) -> Result<usize, rusqlite::Error> {
    atomic(conn, |tx| {
        let users: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT user_insz FROM staff_badges WHERE revoked_at IS NULL ORDER BY id",
            )?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        for insz in &users {
            revoke_in(tx, insz, reason, by)?;
        }
        Ok(users.len())
    })
}

/// An active badge, as the manager's list shows it (never the hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveBadge {
    pub insz: String,
    pub name: String,
    pub linked_at: String,
    pub linked_by: String,
    pub linked_by_name: Option<String>,
}

pub fn list_active(conn: &Connection) -> Result<Vec<ActiveBadge>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT b.user_insz, u.name, b.linked_at, b.linked_by, m.name
         FROM staff_badges b
         JOIN users u ON u.insz = b.user_insz
         LEFT JOIN users m ON m.insz = b.linked_by
         WHERE b.revoked_at IS NULL
         ORDER BY u.name, b.id",
    )?;
    stmt.query_map([], |r| {
        Ok(ActiveBadge {
            insz: r.get(0)?,
            name: r.get(1)?,
            linked_at: r.get(2)?,
            linked_by: r.get(3)?,
            linked_by_name: r.get(4)?,
        })
    })?
    .collect()
}

/// One row of the audit log, with names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub action: String,
    pub insz: Option<String>,
    pub name: Option<String>,
    pub by: Option<String>,
    pub by_name: Option<String>,
    pub at: String,
    pub detail: Option<String>,
}

fn log(
    conn: &Connection,
    action: &str,
    badge_id: Option<i64>,
    insz: Option<&str>,
    detail: Option<&str>,
    by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO badge_log (action, badge_id, user_insz, detail, changed_by)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![action, badge_id, insz, detail, by],
    )?;
    Ok(())
}

/// The latest log rows, the newest first.
pub fn history(conn: &Connection, limit: i64) -> Result<Vec<LogEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT l.action, l.user_insz, u.name, l.changed_by, m.name, l.changed_at, l.detail
         FROM badge_log l
         LEFT JOIN users u ON u.insz = l.user_insz
         LEFT JOIN users m ON m.insz = l.changed_by
         ORDER BY l.id DESC LIMIT ?1",
    )?;
    stmt.query_map(params![limit], |r| {
        Ok(LogEntry {
            action: r.get(0)?,
            insz: r.get(1)?,
            name: r.get(2)?,
            by: r.get(3)?,
            by_name: r.get(4)?,
            at: r.get(5)?,
            detail: r.get(6)?,
        })
    })?
    .collect()
}

/// True after the engine revoked badges for `key_changed` and until a
/// badge is linked again: Settings then asks to link the badges again.
pub fn relink_needed(conn: &Connection) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(
             (SELECT MAX(id) FROM badge_log WHERE action = 'revoke' AND detail = 'key_changed'), 0)
         > COALESCE((SELECT MAX(id) FROM badge_log WHERE action = 'link'), 0)",
        [],
        |r| r.get(0),
    )
}

/// Append a scan to the journal. `badge_ref` is a short reference of
/// the badge hash (never the badge number); `detail` the refusal code
/// or the FDM error.
pub fn log_scan(
    conn: &Connection,
    outcome: &str,
    insz: Option<&str>,
    badge_ref: &str,
    detail: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO badge_scans (outcome, user_insz, badge_ref, detail)
         VALUES (?1, ?2, ?3, ?4)",
        params![outcome, insz, badge_ref, detail],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const MGR: &str = "11111111111";
    const ANNA: &str = "22222222222";
    const BOB: &str = "33333333333";

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO users (insz, name, role) VALUES ('11111111111', 'Manager', 'admin');
             INSERT INTO users (insz, name, role) VALUES ('22222222222', 'Anna', 'waiter');
             INSERT INTO users (insz, name, role) VALUES ('33333333333', 'Bob', 'cashier');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn settings_default_to_off_and_changes_are_logged_once() {
        let conn = db();
        assert_eq!(settings(&conn).unwrap(), BadgeSettings::default());
        assert_eq!(key_check(&conn).unwrap(), None);

        let mut s = BadgeSettings {
            reader: BadgeReader::Usb,
            login_enabled: true,
            ..BadgeSettings::default()
        };
        assert!(set_settings(&conn, &s, MGR).unwrap());
        assert_eq!(settings(&conn).unwrap(), s);
        assert!(
            !set_settings(&conn, &s, MGR).unwrap(),
            "same values: no change, no log"
        );
        s.wedge_max_gap_ms = 80;
        assert!(set_settings(&conn, &s, MGR).unwrap());

        let log = history(&conn, 10).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].action, "settings");
        assert_eq!(log[0].by.as_deref(), Some(MGR));
        assert_eq!(log[0].by_name.as_deref(), Some("Manager"));
        let json: serde_json::Value =
            serde_json::from_str(log[0].detail.as_deref().unwrap()).unwrap();
        assert_eq!(json["reader"], "usb");
        assert_eq!(json["wedge_max_gap_ms"], 80);
        assert_eq!(json["login_enabled"], true);
    }

    #[test]
    fn invalid_settings_are_refused_by_the_table_and_log_nothing() {
        let conn = db();
        let bad = BadgeSettings {
            wedge_max_gap_ms: 5,
            ..BadgeSettings::default()
        };
        assert!(set_settings(&conn, &bad, MGR).is_err());
        assert_eq!(settings(&conn).unwrap(), BadgeSettings::default());
        assert_eq!(history(&conn, 10).unwrap().len(), 0);
    }

    #[test]
    fn the_key_check_is_stored() {
        let conn = db();
        set_key_check(&conn, "0123456789abcdef").unwrap();
        assert_eq!(
            key_check(&conn).unwrap().as_deref(),
            Some("0123456789abcdef")
        );
    }

    #[test]
    fn a_link_is_found_by_hash_and_by_user_and_logged() {
        let conn = db();
        let id = link(&conn, A, ANNA, MGR).unwrap();
        let by_hash = active_link_by_hash(&conn, A).unwrap().unwrap();
        assert_eq!(by_hash.id, id);
        assert_eq!(by_hash.user_insz, ANNA);
        assert!(!by_hash.revoked);
        assert_eq!(active_link_by_user(&conn, ANNA).unwrap().unwrap().id, id);
        assert!(active_link_by_hash(&conn, B).unwrap().is_none());
        assert!(active_link_by_user(&conn, BOB).unwrap().is_none());

        let log = history(&conn, 10).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].action, "link");
        assert_eq!(log[0].insz.as_deref(), Some(ANNA));
        assert_eq!(log[0].name.as_deref(), Some("Anna"));
        assert_eq!(log[0].by_name.as_deref(), Some("Manager"));
    }

    #[test]
    fn a_second_active_link_is_refused_and_leaves_no_log_row() {
        let conn = db();
        link(&conn, A, ANNA, MGR).unwrap();
        assert!(link(&conn, B, ANNA, MGR).is_err());
        assert!(link(&conn, A, BOB, MGR).is_err());
        assert_eq!(history(&conn, 10).unwrap().len(), 1);
    }

    #[test]
    fn revoking_keeps_the_row_and_tells_revoked_from_unknown() {
        let conn = db();
        link(&conn, A, ANNA, MGR).unwrap();
        assert!(revoke(&conn, ANNA, RevokeReason::Manager, Some(MGR)).unwrap());
        assert!(active_link_by_hash(&conn, A).unwrap().is_none());
        let any = any_link_by_hash(&conn, A).unwrap().unwrap();
        assert!(any.revoked);
        assert_eq!(any.user_insz, ANNA);
        assert!(any_link_by_hash(&conn, B).unwrap().is_none());
        assert!(
            !revoke(&conn, ANNA, RevokeReason::Manager, Some(MGR)).unwrap(),
            "nothing left to revoke"
        );
        let log = history(&conn, 10).unwrap();
        assert_eq!(log[0].action, "revoke");
        assert_eq!(log[0].detail.as_deref(), Some("manager"));
    }

    #[test]
    fn a_revoked_badge_can_be_linked_again_and_the_active_one_wins() {
        let conn = db();
        link(&conn, A, ANNA, MGR).unwrap();
        revoke(&conn, ANNA, RevokeReason::Manager, Some(MGR)).unwrap();
        let again = link(&conn, A, BOB, MGR).unwrap();
        let any = any_link_by_hash(&conn, A).unwrap().unwrap();
        assert_eq!(any.id, again);
        assert!(!any.revoked);
        assert_eq!(any.user_insz, BOB);
        // Re-issue to the same employee: revoke then link.
        revoke(&conn, BOB, RevokeReason::Manager, Some(MGR)).unwrap();
        link(&conn, B, BOB, MGR).unwrap();
    }

    #[test]
    fn revoke_all_active_logs_each_with_the_reason_and_no_actor() {
        let conn = db();
        link(&conn, A, ANNA, MGR).unwrap();
        link(&conn, B, BOB, MGR).unwrap();
        assert_eq!(
            revoke_all_active(&conn, RevokeReason::KeyChanged, None).unwrap(),
            2
        );
        assert_eq!(list_active(&conn).unwrap().len(), 0);
        let log = history(&conn, 10).unwrap();
        let revokes: Vec<_> = log.iter().filter(|l| l.action == "revoke").collect();
        assert_eq!(revokes.len(), 2);
        assert!(revokes.iter().all(|l| l.by.is_none()));
        assert!(
            revokes
                .iter()
                .all(|l| l.detail.as_deref() == Some("key_changed"))
        );
        let by: Option<String> = conn
            .query_row("SELECT revoked_by FROM staff_badges LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(by, None);
        assert_eq!(
            revoke_all_active(&conn, RevokeReason::KeyChanged, None).unwrap(),
            0
        );
    }

    #[test]
    fn list_active_names_the_employee_and_the_manager_and_hides_the_hash() {
        let conn = db();
        link(&conn, B, BOB, MGR).unwrap();
        link(&conn, A, ANNA, MGR).unwrap();
        revoke(&conn, BOB, RevokeReason::Deactivated, Some(MGR)).unwrap();
        let list = list_active(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].insz, ANNA);
        assert_eq!(list[0].name, "Anna");
        assert_eq!(list[0].linked_by, MGR);
        assert_eq!(list[0].linked_by_name.as_deref(), Some("Manager"));
        let json = serde_json::to_string(&list).unwrap();
        assert!(!json.contains(A) && !json.contains(B));
    }

    #[test]
    fn history_is_newest_first_and_limited() {
        let conn = db();
        link(&conn, A, ANNA, MGR).unwrap();
        revoke(&conn, ANNA, RevokeReason::Manager, Some(MGR)).unwrap();
        link(&conn, B, ANNA, MGR).unwrap();
        let log = history(&conn, 2).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].action, "link");
        assert_eq!(log[1].action, "revoke");
    }

    #[test]
    fn relink_is_needed_from_a_key_change_until_the_next_link() {
        let conn = db();
        assert!(!relink_needed(&conn).unwrap());
        link(&conn, A, ANNA, MGR).unwrap();
        assert!(!relink_needed(&conn).unwrap());
        revoke(&conn, ANNA, RevokeReason::Manager, Some(MGR)).unwrap();
        assert!(
            !relink_needed(&conn).unwrap(),
            "a manager revocation is not a key change"
        );
        link(&conn, A, ANNA, MGR).unwrap();
        revoke_all_active(&conn, RevokeReason::KeyChanged, None).unwrap();
        assert!(relink_needed(&conn).unwrap());
        link(&conn, B, BOB, MGR).unwrap();
        assert!(!relink_needed(&conn).unwrap());
    }

    #[test]
    fn scans_are_journaled_and_append_only() {
        let conn = db();
        log_scan(&conn, "clock_in", Some(ANNA), "aaaaaaaaaaaa", None).unwrap();
        log_scan(
            &conn,
            "refused",
            None,
            "bbbbbbbbbbbb",
            Some("presence_self"),
        )
        .unwrap();
        let rows: Vec<(String, Option<String>, String, Option<String>)> = conn
            .prepare("SELECT outcome, user_insz, badge_ref, detail FROM badge_scans ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].3.as_deref(), Some("presence_self"));
        assert!(log_scan(&conn, "bogus", None, "x", None).is_err());
        assert!(conn.execute("DELETE FROM badge_scans", []).is_err());
    }
}
