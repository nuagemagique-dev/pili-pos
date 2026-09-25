//! Offline command-line entry points of the `moyun-pos` binary.
//!
//! Without arguments the binary starts the kiosk UI. The commands below
//! run without a display and exit; they are used by the image build and
//! by `deploy/install.sh`:
//!
//! - `moyun-pos --version`
//! - `moyun-pos --init-db [--seed <file.sql>]` — create or migrate the
//!   database at `paths::db_path()` and optionally load a catalogue seed.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail, ensure};
use pos_db::schema;
use rusqlite::Connection;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    RunUi,
    Version,
    InitDb { seed: Option<PathBuf> },
}

pub fn parse(args: &[String]) -> anyhow::Result<Command> {
    match args {
        [] => Ok(Command::RunUi),
        [flag] if flag == "--version" => Ok(Command::Version),
        [flag] if flag == "--init-db" => Ok(Command::InitDb { seed: None }),
        [flag, seed_flag, seed] if flag == "--init-db" && seed_flag == "--seed" => {
            Ok(Command::InitDb {
                seed: Some(PathBuf::from(seed)),
            })
        }
        _ => bail!(
            "usage: moyun-pos [--version | --init-db [--seed <file.sql>]] (got: {})",
            args.join(" ")
        ),
    }
}

/// Create/migrate the database and optionally apply a seed.
///
/// Seeding is refused once the database holds fiscal events: seed files
/// reset catalogue tables and fiscal counters, which must never happen
/// on a terminal that has already signed events.
pub fn init_db(db_path: &Path, seed: Option<&Path>) -> anyhow::Result<()> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let conn =
        Connection::open(db_path).with_context(|| format!("opening {}", db_path.display()))?;
    schema::run_migrations(&conn)?;

    if let Some(seed) = seed {
        let sql = std::fs::read_to_string(seed)
            .with_context(|| format!("reading seed {}", seed.display()))?;
        apply_seed(&conn, &sql).with_context(|| format!("applying seed {}", seed.display()))?;
    }
    Ok(())
}

pub fn apply_seed(conn: &Connection, sql: &str) -> anyhow::Result<()> {
    let fiscal_events: i64 =
        conn.query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))?;
    ensure!(
        fiscal_events == 0,
        "refusing to seed: the database already holds {fiscal_events} fiscal event(s)"
    );
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(sql)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_supported_commands() {
        assert_eq!(parse(&[]).unwrap(), Command::RunUi);
        assert_eq!(parse(&args(&["--version"])).unwrap(), Command::Version);
        assert_eq!(
            parse(&args(&["--init-db"])).unwrap(),
            Command::InitDb { seed: None }
        );
        assert_eq!(
            parse(&args(&["--init-db", "--seed", "seed.sql"])).unwrap(),
            Command::InitDb {
                seed: Some(PathBuf::from("seed.sql"))
            }
        );
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(parse(&args(&["--seed", "seed.sql"])).is_err());
        assert!(parse(&args(&["--init-db", "seed.sql"])).is_err());
    }

    fn migrated() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        conn
    }

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn spf_test_seed_loads_the_manual_product_lines() {
        let conn = migrated();
        apply_seed(&conn, include_str!("../seed_spf_test.sql")).unwrap();
        // Manual §III lists 25 predefined products; 9 of them need
        // features declared unsupported in v1.0.0 and stay inactive.
        assert_eq!(count(&conn, "menu_items"), 25);
        let active: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM menu_items WHERE active = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active, 16);
        let price: i64 = conn
            .query_row(
                "SELECT price_cents FROM menu_items WHERE id = 10006",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(price, 1200, "Dry Martini is EUR 12");
    }

    #[test]
    fn restaurant_seed_applies_on_a_fresh_schema() {
        let conn = migrated();
        apply_seed(&conn, include_str!("../seed.sql")).unwrap();
        assert!(count(&conn, "menu_items") > 0);
        assert!(count(&conn, "users") > 0);
    }

    #[test]
    fn seeding_is_refused_once_fiscal_events_exist() {
        let conn = migrated();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        conn.execute(
            "INSERT INTO fiscal_events
                 (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('N', 1, 'CFOD0061234567', 'POS1', '00000000029', '{}')",
            [],
        )
        .unwrap();
        let err = apply_seed(&conn, include_str!("../seed_spf_test.sql")).unwrap_err();
        assert!(err.to_string().contains("refusing to seed"));
    }

    #[test]
    fn init_db_creates_the_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nested").join("pos.db");
        init_db(&db, None).unwrap();
        assert!(db.exists());
    }
}
