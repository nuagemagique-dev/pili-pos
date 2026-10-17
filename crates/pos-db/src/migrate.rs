//! Versioned migration runner.
//!
//! The MVP shipped with a single one-shot `schema::run_migrations`
//! that ran the entire `SCHEMA_SQL` blob via `CREATE TABLE IF NOT
//! EXISTS`. That was fine while we owned every database, but once a
//! kiosk is in production we can no longer assume the DB is empty -
//! we need to know **which** schema migrations have already been
//! applied so we can append new ones safely.
//!
//! Design:
//!
//! - `_migrations_applied` table tracks `(version, name, applied_at)`.
//! - The `MIGRATIONS` array lists every migration in ascending
//!   `version` order. Order is the source of truth - versions must be
//!   strictly increasing.
//! - Each migration runs in its own `IMMEDIATE` transaction; if the
//!   SQL fails the transaction rolls back and the runner returns the
//!   error.
//! - Migration `001` is the original `SCHEMA_SQL` blob. For backwards
//!   compatibility we detect databases that were created by the old
//!   one-shot runner (have `fiscal_events` but no `_migrations_applied`)
//!   and stamp them as version 1 without re-running the SQL.
//!
//! Adding a new migration:
//! 1. Append a `Migration { version: N, name: …, sql: … }` to
//!    `MIGRATIONS` below.
//! 2. Versions must be strictly monotonic - no gaps required, no
//!    re-ordering ever.
//! 3. Tests live in this file; add coverage when introducing a
//!    schema-changing migration.

use rusqlite::Connection;

use crate::schema::{
    MIGRATION_002_CATALOGUE_FISCAL_FIELDS, MIGRATION_003_BOOKING_PERIODS,
    MIGRATION_004_FISCAL_ARCHIVE_APPEND_ONLY, MIGRATION_005_TICKET_DELIVERIES,
    MIGRATION_006_ORIGINAL_PRINTS, MIGRATION_007_GERMAN_LANGUAGE,
    MIGRATION_008_MEAL_VOUCHER_PAYMENTS, MIGRATION_009_SHIFTS_AND_PRE_BILL,
    MIGRATION_010_PENDING_SIGNS, MIGRATION_011_TABLE_PLAN, SCHEMA_SQL,
};

#[derive(Debug, Clone, Copy)]
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// The migration registry. Strictly ordered by `version`.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial-schema",
        sql: SCHEMA_SQL,
    },
    Migration {
        version: 2,
        name: "catalogue-fiscal-fields",
        sql: MIGRATION_002_CATALOGUE_FISCAL_FIELDS,
    },
    Migration {
        version: 3,
        name: "booking-periods",
        sql: MIGRATION_003_BOOKING_PERIODS,
    },
    Migration {
        version: 4,
        name: "fiscal-archive-append-only",
        sql: MIGRATION_004_FISCAL_ARCHIVE_APPEND_ONLY,
    },
    Migration {
        version: 5,
        name: "ticket-deliveries",
        sql: MIGRATION_005_TICKET_DELIVERIES,
    },
    Migration {
        version: 6,
        name: "original-prints",
        sql: MIGRATION_006_ORIGINAL_PRINTS,
    },
    Migration {
        version: 7,
        name: "german-language",
        sql: MIGRATION_007_GERMAN_LANGUAGE,
    },
    Migration {
        version: 8,
        name: "meal-voucher-payments",
        sql: MIGRATION_008_MEAL_VOUCHER_PAYMENTS,
    },
    Migration {
        version: 9,
        name: "shifts-and-pre-bill",
        sql: MIGRATION_009_SHIFTS_AND_PRE_BILL,
    },
    Migration {
        version: 10,
        name: "pending-signs",
        sql: MIGRATION_010_PENDING_SIGNS,
    },
    Migration {
        version: 11,
        name: "table-plan",
        sql: MIGRATION_011_TABLE_PLAN,
    },
    // Future schema diffs go here, as a new entry rather than an edit
    // of an existing one. Versions 12 to 19 are kept for the certified
    // build; the HubRise integration starts at 20 (`HUBRISE_MIGRATIONS`).
];

/// Migrations of the linked card payment terminal (EFT), in the
/// certified binary on every installation: the table exists everywhere
/// and is written only when `pos.json` has a `payment_terminal`.
const EFT_MIGRATIONS: &[Migration] = &[Migration {
    version: 27,
    name: "eft-payments",
    sql: crate::schema_eft::MIGRATION_027_EFT_PAYMENTS,
}];

/// Migrations of the v1.2 owner features (60 to 63, `schema_v12.rs`),
/// in every build: 60 owner settings and alerts. The later tasks of the
/// lane append 61 allergens, 62 courses and 63 gift vouchers. Renumbering
/// at merge touches only `schema_v12.rs`, this slice and its test.
const V12_MIGRATIONS: &[Migration] = &[Migration {
    version: 60,
    name: "owner-settings-alerts",
    sql: crate::schema_v12::MIGRATION_060_OWNER_SETTINGS_ALERTS,
}];

/// Migrations of the HubRise integration (feature `hubrise`), from
/// version 20 (in the certified binary too).
#[cfg(feature = "hubrise")]
const HUBRISE_MIGRATIONS: &[Migration] = &[
    Migration {
        version: 20,
        name: "hubrise-transport",
        sql: crate::schema_hubrise::MIGRATION_020_HUBRISE_TRANSPORT,
    },
    Migration {
        version: 22,
        name: "webshop-orders",
        sql: crate::schema_hubrise::MIGRATION_022_WEBSHOP_ORDERS,
    },
    Migration {
        version: 23,
        name: "price-changes",
        sql: crate::schema_hubrise::MIGRATION_023_PRICE_CHANGES,
    },
    Migration {
        version: 24,
        name: "platform-charges",
        sql: crate::schema_hubrise::MIGRATION_024_PLATFORM_CHARGES,
    },
    Migration {
        version: 25,
        name: "integration-switch",
        sql: crate::schema_hubrise::MIGRATION_025_INTEGRATION_SWITCH,
    },
];

/// Migrations of the menu mirrored from an external catalog (feature
/// `catalog-sync`, which `hubrise` turns on).
#[cfg(feature = "catalog-sync")]
const CATALOG_MIGRATIONS: &[Migration] = &[
    Migration {
        version: 21,
        name: "catalog-sync",
        sql: crate::schema_catalog::MIGRATION_021_CATALOG_SYNC,
    },
    Migration {
        version: 26,
        name: "product-codes",
        sql: crate::schema_catalog::MIGRATION_026_PRODUCT_CODES,
    },
];

/// Every migration this build knows, in ascending version order.
fn registry() -> Vec<Migration> {
    #[allow(unused_mut)]
    let mut all = MIGRATIONS.to_vec();
    all.extend_from_slice(EFT_MIGRATIONS);
    all.extend_from_slice(V12_MIGRATIONS);
    #[cfg(feature = "hubrise")]
    all.extend_from_slice(HUBRISE_MIGRATIONS);
    #[cfg(feature = "catalog-sync")]
    all.extend_from_slice(CATALOG_MIGRATIONS);
    all.sort_by_key(|m| m.version);
    all
}

/// Run all pending migrations. Idempotent - a second call is a no-op.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // PRAGMAs must run outside of transactions and must run before the
    // migration tracker query (WAL changes journal mode).
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    // Every commit reaches the disk before it returns, in WAL mode too:
    // a sign request is stored before it is sent.
    conn.execute_batch("PRAGMA synchronous=FULL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations_applied (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    backfill_legacy_db(conn)?;
    check_applied_names(conn)?;

    let applied = applied_versions(conn)?;
    for m in &registry() {
        if applied.contains(&m.version) {
            continue;
        }
        // IMMEDIATE forces SQLite to acquire a write lock up-front so
        // we don't discover mid-migration that another connection is
        // holding the DB.
        conn.execute_batch("BEGIN IMMEDIATE;")?;
        let result = conn.execute_batch(m.sql).and_then(|_| {
            conn.execute(
                "INSERT INTO _migrations_applied (version, name) VALUES (?, ?)",
                rusqlite::params![m.version, m.name],
            )
            .map(|_| ())
        });
        match result {
            Ok(()) => conn.execute_batch("COMMIT;")?,
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK;");
                return Err(e);
            }
        }
    }
    Ok(())
}

/// For databases created by the pre-versioning runner (one-shot
/// SCHEMA_SQL): if `fiscal_events` exists and `_migrations_applied` is
/// empty, mark v1 as already-applied so we don't re-run it.
fn backfill_legacy_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    let already_tracked: i64 =
        conn.query_row("SELECT COUNT(*) FROM _migrations_applied", [], |row| {
            row.get(0)
        })?;
    if already_tracked > 0 {
        return Ok(());
    }
    let has_fiscal_events: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type='table' AND name='fiscal_events'",
        [],
        |row| row.get(0),
    )?;
    if has_fiscal_events > 0 {
        // Old-style DB: schema is already there, just record it.
        conn.execute(
            "INSERT INTO _migrations_applied (version, name) VALUES (1, 'initial-schema (backfilled)')",
            [],
        )?;
    }
    Ok(())
}

/// Refuse a database where a version this build knows was applied
/// under another name: the same number reused for two different
/// schema changes (a certified hotfix and a HubRise table, say) would
/// otherwise be skipped as "already applied" and the schema would
/// silently lack it. Versions this build does not know are left alone.
fn check_applied_names(conn: &Connection) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT version, name FROM _migrations_applied")?;
    let applied = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (version, name) in applied {
        let Some(known) = registry().into_iter().find(|m| m.version == version) else {
            continue;
        };
        let backfilled = version == 1 && name == format!("{} (backfilled)", known.name);
        if name != known.name && !backfilled {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISMATCH),
                Some(format!(
                    "migration {version} was applied as {name:?}, this build knows it as {:?}",
                    known.name
                )),
            ));
        }
    }
    Ok(())
}

fn applied_versions(conn: &Connection) -> Result<Vec<i64>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT version FROM _migrations_applied")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The certified versions, then HubRise's when the feature is on.
    fn all_versions() -> Vec<i64> {
        registry().iter().map(|m| m.version).collect()
    }

    #[test]
    fn the_certified_migrations_are_1_to_10() {
        let certified: Vec<i64> = MIGRATIONS.iter().map(|m| m.version).take(10).collect();
        assert_eq!(certified, (1..=10).collect::<Vec<_>>());
    }

    /// Migration 11 (the table plan) follows them, still below HubRise's 20.
    #[test]
    fn the_certified_migrations_end_with_the_table_plan() {
        let certified: Vec<(i64, &str)> = MIGRATIONS.iter().map(|m| (m.version, m.name)).collect();
        assert_eq!(certified.len(), 11);
        assert_eq!(certified[10], (11, "table-plan"));
    }

    /// The payment terminal's table is migration 27, in every build.
    #[test]
    fn the_eft_migration_is_27_and_in_every_build() {
        let eft: Vec<(i64, &str)> = EFT_MIGRATIONS.iter().map(|m| (m.version, m.name)).collect();
        assert_eq!(eft, vec![(27, "eft-payments")]);
        assert!(all_versions().contains(&27));
        // It keeps clear of the reserved 12 to 19.
        assert!(EFT_MIGRATIONS.iter().all(|m| m.version >= 20));
    }

    /// The v1.2 owner features are migrations 60 to 63, in every build
    /// (60 owner settings and alerts for now; 61 to 63 follow).
    #[test]
    fn the_v12_migrations_are_60_to_63_and_in_every_build() {
        let v12: Vec<(i64, &str)> = V12_MIGRATIONS.iter().map(|m| (m.version, m.name)).collect();
        assert_eq!(v12, vec![(60, "owner-settings-alerts")]);
        for (version, _) in &v12 {
            assert!(all_versions().contains(version), "{version}");
            assert!((60..=63).contains(version), "{version}");
        }
    }

    #[test]
    fn migration_60_makes_the_settings_and_the_alert_log_append_only() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO owner_settings_log (key, value_json) VALUES ('alerts', '{}')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO owner_alert_log (kind, transition, outcome) VALUES ('fdm', 'raised', 'sent')",
            [],
        )
        .unwrap();
        for sql in [
            "UPDATE owner_settings_log SET key = 'x'",
            "DELETE FROM owner_settings_log",
            "UPDATE owner_alert_log SET detail = 'x'",
            "DELETE FROM owner_alert_log",
        ] {
            assert!(conn.execute(sql, []).is_err(), "{sql}");
        }
    }

    #[test]
    fn migration_60_checks_the_alert_log_values() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let insert = |kind: &str, transition: &str, outcome: &str| {
            conn.execute(
                "INSERT INTO owner_alert_log (kind, transition, outcome) VALUES (?1, ?2, ?3)",
                rusqlite::params![kind, transition, outcome],
            )
        };
        assert!(insert("fdm", "raised", "sent").is_ok());
        assert!(insert("test", "test", "failed").is_ok());
        assert!(insert("online_waiting", "reminder", "suppressed").is_ok());
        assert!(insert("nope", "raised", "sent").is_err());
        assert!(insert("fdm", "nope", "sent").is_err());
        assert!(insert("fdm", "raised", "nope").is_err());
    }

    #[test]
    fn commits_are_synchronous_full() {
        let path = std::env::temp_dir().join(format!("pos-db-sync-{}.db", std::process::id()));
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();
        let mode: i64 = conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap();
        drop(conn);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
        assert_eq!(mode, 2, "FULL");
    }

    #[cfg(feature = "hubrise")]
    #[test]
    fn hubrise_migrations_start_at_20() {
        assert!(HUBRISE_MIGRATIONS.iter().all(|m| m.version >= 20));
    }

    #[cfg(feature = "catalog-sync")]
    #[test]
    fn the_catalog_migrations_are_21_and_26() {
        let versions: Vec<i64> = CATALOG_MIGRATIONS.iter().map(|m| m.version).collect();
        assert_eq!(versions, vec![21, 26]);
        assert!(all_versions().contains(&21));
        assert!(all_versions().contains(&26));
    }

    #[test]
    fn a_version_applied_under_another_name_is_refused() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "UPDATE _migrations_applied SET name = 'something-else' WHERE version = 9",
            [],
        )
        .unwrap();
        let err = run_migrations(&conn).unwrap_err().to_string();
        assert!(err.contains("migration 9"), "{err}");
    }

    #[test]
    fn a_version_this_build_does_not_know_is_left_alone() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO _migrations_applied (version, name) VALUES (999, 'from-a-later-build')",
            [],
        )
        .unwrap();
        run_migrations(&conn).unwrap();
    }

    #[test]
    fn fresh_db_runs_all_migrations() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        let applied = applied_versions(&conn).unwrap();
        assert_eq!(applied, all_versions());

        // Running again is a no-op.
        run_migrations(&conn).unwrap();
        let applied_again = applied_versions(&conn).unwrap();
        assert_eq!(applied_again, all_versions());
    }

    #[test]
    fn legacy_db_backfilled_to_v1() {
        let conn = Connection::open_in_memory().unwrap();
        // Simulate a database created by the pre-versioning runner:
        // raw schema applied, no _migrations_applied table.
        conn.execute_batch(SCHEMA_SQL).unwrap();
        run_migrations(&conn).unwrap();

        // v1 is stamped, later migrations run on top of it.
        let applied = applied_versions(&conn).unwrap();
        assert_eq!(applied, all_versions());

        let stamp: String = conn
            .query_row(
                "SELECT name FROM _migrations_applied WHERE version=1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(stamp.contains("backfilled"));
    }

    #[test]
    fn v2_adds_the_catalogue_fiscal_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let columns = |table: &str| -> Vec<String> {
            let mut stmt = conn
                .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let items = columns("menu_items");
        assert!(items.contains(&"vat_code_takeaway".to_string()));
        assert!(items.contains(&"open_price".to_string()));
        let lines = columns("order_lines");
        assert!(lines.contains(&"department_id".to_string()));
        assert!(lines.contains(&"department_name".to_string()));
    }

    #[test]
    fn migrating_an_already_v2_db_adds_the_later_migrations() {
        // A DB that already has v1+v2 applied (e.g. upgraded before v3
        // shipped) must pick up v3 onwards on the next run without re-running
        // v1/v2 - and without needing a full fresh DB.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn.execute_batch(MIGRATION_002_CATALOGUE_FISCAL_FIELDS)
            .unwrap();
        conn.execute(
            "CREATE TABLE _migrations_applied (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
             )",
            [],
        )
        .unwrap();
        conn.execute_batch(
            "INSERT INTO _migrations_applied (version, name) VALUES
                (1, 'initial-schema'), (2, 'catalogue-fiscal-fields');",
        )
        .unwrap();

        run_migrations(&conn).unwrap();
        assert_eq!(applied_versions(&conn).unwrap(), all_versions());

        // The v4 triggers are actually present after this incremental path.
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('N', 1, 'P', 'T1', '00000000029', '{}')",
            [],
        )
        .unwrap();
        assert!(conn.execute("DELETE FROM fiscal_events", []).is_err());
    }

    #[test]
    fn v5_creates_ticket_deliveries_with_correct_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM pragma_table_info('ticket_deliveries')")
            .unwrap();
        let columns: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for expected in [
            "id",
            "fiscal_event_id",
            "medium",
            "recipient",
            "recipient_hint",
            "ticket_text",
            "verification_url",
            "status",
            "attempts",
            "next_attempt_at",
            "provider_message_id",
            "last_error",
            "fallback_printed",
            "created_at",
            "updated_at",
        ] {
            assert!(
                columns.contains(&expected.to_string()),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn migration_versions_are_strictly_monotonic() {
        let mut prev = 0;
        for m in &registry() {
            assert!(
                m.version > prev,
                "migration versions must be strictly increasing; \
                 got {prev} then {}",
                m.version
            );
            prev = m.version;
        }
    }
}
