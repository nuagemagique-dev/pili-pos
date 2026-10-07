//! Install-time entry points of the engine: create or migrate the
//! database, load a catalogue seed, and check a `pos.json`.
//!
//! The `pili-pos` binary exposes them as `--init-db [--seed <file.sql>]`
//! and `--check-config` (see `src-tauri/src/cli.rs`); they run without a
//! display and are used by the image build and `deploy/install.sh`.

use std::path::Path;

use anyhow::{Context, ensure};
use pos_db::schema;
use rusqlite::Connection;

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

/// What is wrong with the configuration the kiosk would load: the file
/// named by `env_path` (`PILI_POS_CONFIG`), else `production_path`.
/// Unlike the kiosk, a missing file is a problem here: the development
/// identity is never acceptable on an installed terminal.
pub fn check_config(env_path: Option<&str>, production_path: &Path) -> Vec<String> {
    if env_path.is_none() && !production_path.exists() {
        return vec![format!(
            "no configuration: {} is missing and PILI_POS_CONFIG is not set",
            production_path.display()
        )];
    }
    match crate::state::load_config_from(env_path, production_path) {
        Ok(config) => pos_fiscal::fdm_client::FdmClient::config_problems(&config),
        Err(e) => vec![format!("{e:#}")],
    }
}

pub fn apply_seed(conn: &Connection, sql: &str) -> anyhow::Result<()> {
    let fiscal_events: i64 =
        conn.query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))?;
    ensure!(
        fiscal_events == 0,
        "refusing to seed: the database already holds {fiscal_events} fiscal event(s)"
    );
    #[cfg(feature = "hubrise")]
    if let Some(source) = pos_catalog::managed_by(conn)? {
        anyhow::bail!("refusing to seed: the menu is managed by {source}");
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(sql)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_config_reports_a_missing_broken_or_wrong_file() {
        let dir = tempfile::tempdir().unwrap();
        let prod = dir.path().join("pos.json");

        let missing = check_config(None, &prod);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].contains("no configuration"), "{missing:?}");

        std::fs::write(&prod, "{ not json").unwrap();
        assert_eq!(check_config(None, &prod).len(), 1);

        let good = r#"{
            "pos_id": "CFOD0061234567", "terminal_id": "POS1", "device_id": "aa:bb:cc:dd:ee:ff",
            "restaurant_name": "R", "restaurant_address": "A", "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "2123456791", "fdm_url": "http://192.168.1.20/graphql"
        }"#;
        std::fs::write(&prod, good).unwrap();
        assert_eq!(check_config(None, &prod), Vec::<String>::new());

        // A typo in the VAT number: one problem, naming the key.
        std::fs::write(&prod, good.replace("BE0499999960", "BE0499999961")).unwrap();
        let typo = check_config(None, &prod);
        assert_eq!(typo.len(), 1);
        assert!(typo[0].starts_with("restaurant_vat"), "{typo:?}");

        // PILI_POS_CONFIG wins, and a missing file there is reported.
        let env = dir.path().join("elsewhere.json");
        assert_eq!(check_config(Some(env.to_str().unwrap()), &prod).len(), 1);
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
        apply_seed(&conn, include_str!("../../../src-tauri/seed_spf_test.sql")).unwrap();
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
        apply_seed(&conn, include_str!("../../../src-tauri/seed.sql")).unwrap();
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
        let err =
            apply_seed(&conn, include_str!("../../../src-tauri/seed_spf_test.sql")).unwrap_err();
        assert!(err.to_string().contains("refusing to seed"));
    }

    #[cfg(feature = "hubrise")]
    #[test]
    fn seeding_is_refused_while_a_catalog_manages_the_menu() {
        let mut conn = migrated();
        pos_catalog::apply(&mut conn, "hubrise", &pos_catalog::testing::catalog()).unwrap();
        let err = apply_seed(&conn, include_str!("../../../src-tauri/seed.sql")).unwrap_err();
        assert!(err.to_string().contains("managed by hubrise"), "{err}");
    }

    #[test]
    fn init_db_creates_the_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("nested").join("pos.db");
        init_db(&db, None).unwrap();
        assert!(db.exists());
    }
}
