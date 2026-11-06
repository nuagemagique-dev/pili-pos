//! The table plan (migration 11): the table numbers in use and the log
//! of their changes. See `schema::MIGRATION_011_TABLE_PLAN`.
//!
//! The plans are stored in their canonical form
//! (`pos_core::table_plan::format_zoned`); this module does not read them.
//! The names of the zone letters (migration 91) are stored with the plan,
//! also in their canonical form (`format_zone_names`).

use rusqlite::{Connection, OptionalExtension, params};

/// One change of the plan.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PlanChange {
    pub plan: String,
    /// The zone names the plan had then ("A · Salle, B · Terrasse"), empty for none.
    pub zone_names: String,
    /// `config` (pos.json) or `admin` (a manager).
    pub source: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

fn log(
    conn: &Connection,
    plan: &str,
    zone_names: &str,
    source: &str,
    by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO table_plan_log (plan, zone_names, source, changed_by) VALUES (?1, ?2, ?3, ?4)",
        params![plan, zone_names, source, by],
    )?;
    Ok(())
}

/// At start-up: the plan, given the one `pos.json` gives now. The first
/// start, or a new plan in `pos.json`, sets it (and logs it); else it
/// keeps its last value, a manager's change included.
pub fn at_start(conn: &Connection, config_plan: &str) -> Result<String, rusqlite::Error> {
    super::with_savepoint(conn, |tx| {
        let row: Option<(String, String)> = tx
            .query_row(
                "SELECT plan, config_plan FROM table_plan WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let plan = match row {
            Some((plan, seen)) if seen == config_plan => plan,
            Some(_) => {
                tx.execute(
                    "UPDATE table_plan
                     SET plan = ?1, config_plan = ?1, changed_at = datetime('now'), changed_by = NULL
                     WHERE id = 1",
                    params![config_plan],
                )?;
                let names = zone_names(tx)?.unwrap_or_default();
                log(tx, config_plan, &names, "config", None)?;
                config_plan.to_string()
            }
            None => {
                tx.execute(
                    "INSERT INTO table_plan (id, plan, config_plan, changed_at)
                     VALUES (1, ?1, ?1, datetime('now'))",
                    params![config_plan],
                )?;
                log(tx, config_plan, "", "config", None)?;
                config_plan.to_string()
            }
        };
        Ok(plan)
    })
}

/// The plan in use; None before the first start-up recorded one.
pub fn current(conn: &Connection) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row("SELECT plan FROM table_plan WHERE id = 1", [], |r| r.get(0))
        .optional()
}

/// The names of the zone letters of the plan in use ("A · Salle, B · Terrasse");
/// None before the first start-up recorded a plan.
pub fn zone_names(conn: &Connection) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row("SELECT zone_names FROM table_plan WHERE id = 1", [], |r| {
        r.get(0)
    })
    .optional()
}

/// A manager's change of the plan and of its zone names. Returns whether
/// it changed anything.
pub fn set(
    conn: &Connection,
    plan: &str,
    zone_names_text: &str,
    admin_insz: &str,
) -> Result<bool, rusqlite::Error> {
    super::with_savepoint(conn, |tx| {
        let changed = match current(tx)? {
            Some(old) if old == plan && zone_names(tx)?.as_deref() == Some(zone_names_text) => {
                false
            }
            Some(_) => {
                tx.execute(
                    "UPDATE table_plan
                     SET plan = ?1, zone_names = ?2, changed_at = datetime('now'), changed_by = ?3
                     WHERE id = 1",
                    params![plan, zone_names_text, admin_insz],
                )?;
                true
            }
            // No start-up recorded the pos.json plan (a bare database): the
            // manager's plan stands for both until pos.json says otherwise.
            None => {
                tx.execute(
                    "INSERT INTO table_plan (id, plan, zone_names, config_plan, changed_at, changed_by)
                     VALUES (1, ?1, ?2, ?1, datetime('now'), ?3)",
                    params![plan, zone_names_text, admin_insz],
                )?;
                true
            }
        };
        if changed {
            log(tx, plan, zone_names_text, "admin", Some(admin_insz))?;
        }
        Ok(changed)
    })
}

/// The changes, the latest first.
pub fn history(conn: &Connection, limit: i64) -> Result<Vec<PlanChange>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT plan, zone_names, source, changed_by, changed_at FROM table_plan_log
         ORDER BY id DESC LIMIT ?1",
    )?;
    stmt.query_map(params![limit], |r| {
        Ok(PlanChange {
            plan: r.get(0)?,
            zone_names: r.get(1)?,
            source: r.get(2)?,
            changed_by: r.get(3)?,
            changed_at: r.get(4)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn the_first_start_takes_the_config_plan_and_logs_it() {
        let conn = db();
        assert_eq!(current(&conn).unwrap(), None);
        assert_eq!(at_start(&conn, "1-20").unwrap(), "1-20");
        assert_eq!(current(&conn).unwrap().as_deref(), Some("1-20"));
        let log = history(&conn, 10).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(
            (log[0].plan.as_str(), log[0].source.as_str()),
            ("1-20", "config")
        );
    }

    #[test]
    fn a_manager_change_survives_a_restart_until_pos_json_changes() {
        let conn = db();
        at_start(&conn, "1-20").unwrap();
        assert!(set(&conn, "1-12, 20-25", "", "94060242576").unwrap());
        assert!(
            !set(&conn, "1-12, 20-25", "", "94060242576").unwrap(),
            "same plan: no change"
        );
        assert_eq!(
            at_start(&conn, "1-20").unwrap(),
            "1-12, 20-25",
            "same pos.json: kept"
        );
        assert_eq!(
            at_start(&conn, "1-30").unwrap(),
            "1-30",
            "new pos.json plan wins"
        );
        let log = history(&conn, 10).unwrap();
        let sources: Vec<_> = log
            .iter()
            .map(|c| (c.plan.as_str(), c.source.as_str()))
            .collect();
        assert_eq!(
            sources,
            vec![
                ("1-30", "config"),
                ("1-12, 20-25", "admin"),
                ("1-20", "config")
            ]
        );
        assert_eq!(log[1].changed_by.as_deref(), Some("94060242576"));
    }

    #[test]
    fn a_change_on_a_bare_database_is_kept_and_logged() {
        let conn = db();
        assert!(set(&conn, "5-8", "", "94060242576").unwrap());
        assert_eq!(current(&conn).unwrap().as_deref(), Some("5-8"));
        assert_eq!(history(&conn, 10).unwrap().len(), 1);
    }

    #[test]
    fn the_log_is_append_only() {
        let conn = db();
        at_start(&conn, "1-20").unwrap();
        assert!(
            conn.execute("UPDATE table_plan_log SET plan = 'x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM table_plan_log", []).is_err());
    }

    #[test]
    fn the_zone_names_are_stored_with_the_plan_and_logged_with_it() {
        let conn = db();
        at_start(&conn, "1-20").unwrap();
        assert_eq!(zone_names(&conn).unwrap().as_deref(), Some(""));
        assert!(set(&conn, "A 1-10, B 11-20", "A \u{b7} Salle", "94060242576").unwrap());
        assert_eq!(
            zone_names(&conn).unwrap().as_deref(),
            Some("A \u{b7} Salle")
        );
        // A new name alone is a change; the same plan and names again is not.
        assert!(
            set(
                &conn,
                "A 1-10, B 11-20",
                "A \u{b7} Salle, B \u{b7} Terrasse",
                "94060242576"
            )
            .unwrap()
        );
        assert!(
            !set(
                &conn,
                "A 1-10, B 11-20",
                "A \u{b7} Salle, B \u{b7} Terrasse",
                "94060242576"
            )
            .unwrap()
        );
        // The same pos.json plan at the next start keeps the names.
        assert_eq!(at_start(&conn, "1-20").unwrap(), "A 1-10, B 11-20");
        let log = history(&conn, 10).unwrap();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].zone_names, "A \u{b7} Salle, B \u{b7} Terrasse");
        assert_eq!(log[1].zone_names, "A \u{b7} Salle");
        assert_eq!(log[2].zone_names, "");
    }
}
