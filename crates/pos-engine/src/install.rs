//! Install-time entry points of the engine, for a shell that provisions a
//! terminal before (or without) opening an [`crate::Engine`]. They touch
//! no path of their own: the caller names every file.
//!
//! - [`init_db`]: create or migrate the database file, optionally loading
//!   a catalogue seed.
//! - [`apply_seed`]: load a seed into an open database (refused once the
//!   terminal holds fiscal events or a catalogue manages the menu).
//! - [`check_config`]: what the FDM would refuse in the `pos.json` the
//!   Linux kiosk would load (`PILI_POS_CONFIG`, else the installed file).
//! - [`check_config_source`]: the same check for any [`ConfigSource`],
//!   the entry of the Android provisioning check.
//!
//! The `pili-pos` binary exposes the first and third as
//! `--init-db [--seed <file.sql>]` and `--check-config` (see
//! `src-tauri/src/cli.rs`); they run without a display and are used by
//! the image build and `deploy/install.sh`.

use std::path::Path;

use anyhow::{Context, ensure};
use pos_db::schema;
use rusqlite::Connection;

use crate::engine::{ConfigSource, config_source_from};

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
    check_config_source(&config_source_from(env_path, production_path))
}

/// What the FDM would refuse in the configuration `source` gives, or
/// why it cannot be loaded; empty when it is usable. The files the
/// configuration names (token, CA, client certificate) must exist, a
/// relative path of a [`ConfigSource::Bundle`] being resolved against the
/// bundle first. [`ConfigSource::DevelopmentDefaults`] is itself a
/// problem: the development identity is never acceptable on an installed
/// terminal.
pub fn check_config_source(source: &ConfigSource) -> Vec<String> {
    if matches!(source, ConfigSource::DevelopmentDefaults) {
        return vec![
            "no configuration: the development identity (TEST) is not acceptable on an \
             installed terminal"
                .to_string(),
        ];
    }
    match source.load() {
        Ok(config) => {
            let mut problems = pos_fiscal::fdm_client::FdmClient::config_problems(&config);
            problems.extend(export_problems(&config, &problems));
            problems
        }
        Err(e) => vec![format!("{e:#}")],
    }
}

/// What keeps the nightly export from working beyond the syntax
/// `PosConfig::problems` (already in `reported`) checks: a recipient that is
/// not a valid age key, and a token file that cannot be read. Empty without
/// an `export` section.
fn export_problems(config: &pos_core::config::PosConfig, reported: &[String]) -> Vec<String> {
    let Some(section) = config.export.as_ref() else {
        return Vec::new();
    };
    if reported.iter().any(|p| p.starts_with("export.")) {
        return Vec::new();
    }
    match pos_export::settings::ExportSettings::from_config(section) {
        Err(e) => vec![format!("export: {e}")],
        Ok(settings) => settings
            .read_token()
            .err()
            .map(|e| format!("export: {e}"))
            .into_iter()
            .collect(),
    }
}

/// Load `sql` into an open database as a seed.
///
/// Refused (an error naming the reason) when the database already holds
/// fiscal events, or, with the HubRise feature, when a catalogue manages
/// the menu. The seed runs in one transaction: all of it or nothing.
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

    const GOOD: &str = r#"{
        "pos_id": "CFOD0061234567", "terminal_id": "POS1", "device_id": "aa:bb:cc:dd:ee:ff",
        "restaurant_name": "R", "restaurant_address": "A", "restaurant_vat": "BE0499999960",
        "restaurant_est_no": "2123456791", "fdm_url": "http://192.168.1.20/graphql"
    }"#;

    #[test]
    fn check_config_source_reports_a_missing_broken_or_wrong_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pos.json");

        let missing = check_config_source(&ConfigSource::File(file.clone()));
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(
            missing[0].contains("cannot read config file"),
            "{missing:?}"
        );

        std::fs::write(&file, "{ not json").unwrap();
        assert_eq!(
            check_config_source(&ConfigSource::File(file.clone())).len(),
            1
        );
        assert_eq!(
            check_config_source(&ConfigSource::Bundle(file.clone())).len(),
            1
        );

        std::fs::write(&file, GOOD).unwrap();
        for source in [
            ConfigSource::File(file.clone()),
            ConfigSource::Bundle(file.clone()),
        ] {
            assert_eq!(check_config_source(&source), Vec::<String>::new());
        }

        std::fs::write(&file, GOOD.replace("BE0499999960", "BE0499999961")).unwrap();
        let typo = check_config_source(&ConfigSource::File(file));
        assert_eq!(typo.len(), 1);
        assert!(typo[0].starts_with("restaurant_vat"), "{typo:?}");
    }

    #[test]
    fn the_development_identity_is_a_problem() {
        let problems = check_config_source(&ConfigSource::DevelopmentDefaults);
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("development identity"), "{problems:?}");
    }

    #[test]
    fn a_bundle_is_checked_with_its_relative_files_resolved() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pos.json");
        let json = GOOD.replace(
            "\"fdm_url\": \"http://192.168.1.20/graphql\"",
            "\"fdm_url\": \"http://192.168.1.20/graphql\", \"fdm_token_file\": \"secrets/fdm.token\"",
        );
        std::fs::write(&file, &json).unwrap();

        // The token is not in the bundle yet: reported, by its resolved path.
        let missing = check_config_source(&ConfigSource::Bundle(file.clone()));
        assert!(!missing.is_empty(), "{missing:?}");
        assert!(missing[0].starts_with("fdm_token_file"), "{missing:?}");
        assert!(
            missing[0].contains(dir.path().join("secrets").to_str().unwrap()),
            "{missing:?}"
        );
        // As a plain file the same relative path is read from the working
        // directory, where it is not either.
        assert!(!check_config_source(&ConfigSource::File(file.clone())).is_empty());

        std::fs::create_dir_all(dir.path().join("secrets")).unwrap();
        std::fs::write(dir.path().join("secrets/fdm.token"), "token\n").unwrap();
        assert_eq!(
            check_config_source(&ConfigSource::Bundle(file)),
            Vec::<String>::new()
        );
    }

    /// `GOOD` with an `export` section; `token_file` is relative, so the
    /// check resolves it against a bundle.
    fn with_export(owner: &str, producer: Option<&str>, keep_days: u32) -> String {
        let producer = producer
            .map(|p| format!(", \"producer_recipient\": \"{p}\""))
            .unwrap_or_default();
        GOOD.replace(
            "\"fdm_url\": \"http://192.168.1.20/graphql\"",
            &format!(
                "\"fdm_url\": \"http://192.168.1.20/graphql\", \"export\": {{ \
                 \"url\": \"https://export.example.com/v1/inst-42\", \
                 \"token_file\": \"secrets/export.token\", \
                 \"owner_recipient\": \"{owner}\"{producer}, \"keep_days\": {keep_days} }}"
            ),
        )
    }

    #[test]
    fn the_export_section_is_checked_with_its_token_file() {
        let (_, owner) = pos_export::crypt::generate_identity();
        let (_, producer) = pos_export::crypt::generate_identity();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pos.json");
        let bundle = ConfigSource::Bundle(file.clone());

        // No section: nothing to report about the export.
        std::fs::write(&file, GOOD).unwrap();
        assert_eq!(check_config_source(&bundle), Vec::<String>::new());

        // A section whose token is not in the bundle yet.
        std::fs::write(&file, with_export(&owner, Some(&producer), 14)).unwrap();
        let missing = check_config_source(&bundle);
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].starts_with("export: "), "{missing:?}");
        assert!(missing[0].contains("unreadable"), "{missing:?}");
        assert!(
            missing[0].contains(dir.path().join("secrets").to_str().unwrap()),
            "the relative token path is resolved: {missing:?}"
        );

        // An empty token file.
        std::fs::create_dir_all(dir.path().join("secrets")).unwrap();
        std::fs::write(dir.path().join("secrets/export.token"), "\n").unwrap();
        let empty = check_config_source(&bundle);
        assert_eq!(empty.len(), 1, "{empty:?}");
        assert!(
            empty[0].starts_with("export: ") && empty[0].contains("empty"),
            "{empty:?}"
        );

        // A usable section.
        std::fs::write(dir.path().join("secrets/export.token"), "tok\n").unwrap();
        assert_eq!(check_config_source(&bundle), Vec::<String>::new());
    }

    #[test]
    fn a_bad_export_section_is_reported_once() {
        let (_, owner) = pos_export::crypt::generate_identity();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("pos.json");
        std::fs::create_dir_all(dir.path().join("secrets")).unwrap();
        std::fs::write(dir.path().join("secrets/export.token"), "tok").unwrap();

        // Syntax problem: reported by PosConfig::problems only, not twice.
        std::fs::write(&file, with_export(&owner, None, 0)).unwrap();
        let syntax = check_config_source(&ConfigSource::Bundle(file.clone()));
        assert_eq!(syntax.len(), 1, "{syntax:?}");
        assert!(syntax[0].starts_with("export.keep_days: "), "{syntax:?}");

        // Well-formed but not a valid age key (bad checksum).
        let mut wrong = owner.clone();
        wrong.replace_range(
            wrong.len() - 1..,
            if owner.ends_with('q') { "p" } else { "q" },
        );
        std::fs::write(&file, with_export(&wrong, None, 14)).unwrap();
        let key = check_config_source(&ConfigSource::Bundle(file));
        assert_eq!(key.len(), 1, "{key:?}");
        assert!(
            key[0].starts_with("export: export.owner_recipient: "),
            "{key:?}"
        );
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
