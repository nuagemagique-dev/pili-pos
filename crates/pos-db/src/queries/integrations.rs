//! The `HubRise` switch (migration 25): its state and the log of its
//! changes. See `schema_hubrise::MIGRATION_025_INTEGRATION_SWITCH`.

use rusqlite::{Connection, OptionalExtension, params};

const HUBRISE: &str = "hubrise";

/// Who changed the switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Config,
    Admin,
}

impl Source {
    fn as_str(self) -> &'static str {
        match self {
            Source::Config => "config",
            Source::Admin => "admin",
        }
    }
}

/// One change of the switch.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SwitchChange {
    pub enabled: bool,
    pub source: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

/// Whether the integration is switched on: the state a log row records and
/// the value the switch is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    On,
    Off,
}

impl State {
    /// Whether this is `On`.
    #[must_use]
    pub fn is_on(self) -> bool {
        self == Self::On
    }
}

impl From<bool> for State {
    fn from(enabled: bool) -> Self {
        if enabled { Self::On } else { Self::Off }
    }
}

fn log(
    conn: &Connection,
    state: State,
    source: Source,
    by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO integration_switch_log (name, enabled, source, changed_by)
         VALUES (?1, ?2, ?3, ?4)",
        params![HUBRISE, state.is_on(), source.as_str(), by],
    )?;
    Ok(())
}

/// At start-up: the switch, given what `pos.json` says now. The first
/// start, or a new value in `pos.json`, sets it (and logs it); else it
/// keeps its last value, an admin's change included.
pub fn hubrise_at_start(conn: &Connection, config: State) -> Result<bool, rusqlite::Error> {
    let config_value = config.is_on();
    super::with_savepoint(conn, |tx| {
        let row: Option<(bool, bool)> = tx
            .query_row(
                "SELECT enabled, config_value FROM integration_switches WHERE name = ?1",
                params![HUBRISE],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let enabled = match row {
            Some((enabled, seen)) if seen == config_value => enabled,
            Some(_) => {
                tx.execute(
                    "UPDATE integration_switches
                     SET enabled = ?2, config_value = ?2, changed_at = datetime('now'),
                         changed_by = NULL
                     WHERE name = ?1",
                    params![HUBRISE, config_value],
                )?;
                log(tx, State::from(config_value), Source::Config, None)?;
                config_value
            }
            None => {
                tx.execute(
                    "INSERT INTO integration_switches (name, enabled, config_value, changed_at)
                     VALUES (?1, ?2, ?2, datetime('now'))",
                    params![HUBRISE, config_value],
                )?;
                log(tx, State::from(config_value), Source::Config, None)?;
                config_value
            }
        };
        Ok(enabled)
    })
}

/// An admin's change. Returns whether it changed anything.
pub fn set_hubrise(
    conn: &Connection,
    state: State,
    admin_insz: &str,
) -> Result<bool, rusqlite::Error> {
    let enabled = state.is_on();
    super::with_savepoint(conn, |tx| {
        let changed = tx.execute(
            "UPDATE integration_switches
             SET enabled = ?2, changed_at = datetime('now'), changed_by = ?3
             WHERE name = ?1 AND enabled != ?2",
            params![HUBRISE, enabled, admin_insz],
        )? == 1;
        if changed {
            log(tx, state, Source::Admin, Some(admin_insz))?;
        }
        Ok(changed)
    })
}

/// Every change, the latest first.
pub fn hubrise_log(conn: &Connection, limit: i64) -> Result<Vec<SwitchChange>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT enabled, source, changed_by, changed_at FROM integration_switch_log
         WHERE name = ?1 ORDER BY id DESC LIMIT ?2",
    )?;
    stmt.query_map(params![HUBRISE, limit], |r| {
        Ok(SwitchChange {
            enabled: r.get(0)?,
            source: r.get(1)?,
            changed_by: r.get(2)?,
            changed_at: r.get(3)?,
        })
    })?
    .collect()
}

/// Online orders that wait for acceptance (`fiscal_state` `pending`)
/// and arrived at or before `arrived_before` (SQLite datetime text,
/// UTC): how many per channel, in channel-name order. Names and counts
/// only, nothing about the customer or the content (owner alerts,
/// V12-D14).
pub fn waiting_online_orders(
    conn: &Connection,
    arrived_before: &str,
) -> Result<Vec<(String, i64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT channel, COUNT(*) FROM webshop_orders
         WHERE fiscal_state = 'pending' AND resolved_at <= ?1
         GROUP BY channel ORDER BY channel",
    )?;
    let rows = stmt.query_map(rusqlite::params![arrived_before], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?;
    rows.collect()
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
    fn the_first_start_takes_the_config_and_logs_it() {
        let conn = db();
        assert!(!hubrise_at_start(&conn, State::Off).unwrap());
        let log = hubrise_log(&conn, 10).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!((log[0].enabled, log[0].source.as_str()), (false, "config"));
        // The same config again: nothing new.
        assert!(!hubrise_at_start(&conn, State::Off).unwrap());
        assert_eq!(hubrise_log(&conn, 10).unwrap().len(), 1);
    }

    #[test]
    fn an_admin_change_holds_until_the_config_changes() {
        let conn = db();
        hubrise_at_start(&conn, State::Off).unwrap();
        assert!(set_hubrise(&conn, State::On, "85073003328").unwrap());
        assert!(
            !set_hubrise(&conn, State::On, "85073003328").unwrap(),
            "no change"
        );
        assert!(
            hubrise_at_start(&conn, State::Off).unwrap(),
            "the admin's value holds"
        );
        // The installer sets pos.json to true then false: the latest wins.
        assert!(hubrise_at_start(&conn, State::On).unwrap());
        assert!(!hubrise_at_start(&conn, State::Off).unwrap());
        let sources: Vec<_> = hubrise_log(&conn, 10)
            .unwrap()
            .into_iter()
            .map(|c| (c.enabled, c.source, c.changed_by))
            .collect();
        assert_eq!(
            sources,
            vec![
                (false, "config".into(), None),
                (true, "config".into(), None),
                (true, "admin".into(), Some("85073003328".into())),
                (false, "config".into(), None),
            ]
        );
    }

    #[test]
    fn the_log_cannot_be_rewritten() {
        let conn = db();
        hubrise_at_start(&conn, State::On).unwrap();
        assert!(
            conn.execute("DELETE FROM integration_switch_log", [])
                .is_err()
        );
        assert!(
            conn.execute("UPDATE integration_switch_log SET enabled = 0", [])
                .is_err()
        );
    }
}
