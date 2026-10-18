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
    MIGRATION_010_PENDING_SIGNS, MIGRATION_011_TABLE_PLAN, MIGRATION_028_PRESENCE_REGISTRATION,
    SCHEMA_SQL,
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
    // Versions 20 to 27 are taken (HUBRISE_MIGRATIONS, CATALOG_MIGRATIONS,
    // EFT_MIGRATIONS below); 28 is presence registration; 30 and 31 are
    // TILL_MIGRATIONS below; 70 is staff badges (after the other lanes'
    // ranges, see below).
    Migration {
        version: 28,
        name: "presence-registration",
        sql: MIGRATION_028_PRESENCE_REGISTRATION,
    },
    // 30, 31, 40 to 4x and 60 to 6x are taken by other lanes of the v1.0
    // work; 70 is staff badges.
    Migration {
        version: 70,
        name: "staff-badges",
        sql: crate::schema_badges::MIGRATION_070_STAFF_BADGES,
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

/// Migrations of the till additions (discounts, kitchen notes), in the
/// certified binary on every installation (28 is presence registration
/// in `MIGRATIONS`, 29 is left free).
const TILL_MIGRATIONS: &[Migration] = &[
    Migration {
        version: 30,
        name: "till-discounts",
        sql: crate::schema_till::MIGRATION_030_TILL_DISCOUNTS,
    },
    Migration {
        version: 31,
        name: "kitchen-notes",
        sql: crate::schema_till::MIGRATION_031_KITCHEN_NOTES,
    },
];

/// Migrations of the invoicing, sold-out and menu editor group, in every build (V11-D2): 40 and
/// up, clear of the 12 to 19 hotfix range and of the lanes that claim 20 to 39. Gaps are allowed
/// by the runner.
const INVOICING_MIGRATIONS: &[Migration] = &[
    Migration {
        version: 40,
        name: "invoicing",
        sql: crate::schema_invoicing::MIGRATION_040_INVOICING,
    },
    Migration {
        version: 41,
        name: "sold-out",
        sql: crate::schema_invoicing::MIGRATION_041_SOLD_OUT,
    },
    Migration {
        version: 42,
        name: "menu-editor",
        sql: crate::schema_invoicing::MIGRATION_042_MENU_EDITOR,
    },
];

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
    all.extend_from_slice(TILL_MIGRATIONS);
    all.extend_from_slice(INVOICING_MIGRATIONS);
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

    /// Migration 11 (the table plan) follows them, still below HubRise's 20;
    /// then 28 (presence registration), then 70 (staff badges), in every
    /// build.
    #[test]
    fn the_base_migrations_are_the_certified_ones_then_presence_then_badges() {
        let base: Vec<(i64, &str)> = MIGRATIONS.iter().map(|m| (m.version, m.name)).collect();
        assert_eq!(base.len(), 13);
        assert_eq!(base[10], (11, "table-plan"));
        assert_eq!(base[11], (28, "presence-registration"));
        assert_eq!(base[12], (70, "staff-badges"));
    }

    #[test]
    fn presence_registration_adds_its_columns_with_safe_defaults() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO users (insz, name, role) VALUES ('11111111111', 'A', 'waiter')",
            [],
        )
        .unwrap();
        let flag: i64 = conn
            .query_row("SELECT presence_only FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(flag, 0);
        // The CHECK refuses anything but 0 or 1.
        assert!(
            conn.execute("UPDATE users SET presence_only = 2", [])
                .is_err()
        );
        conn.execute(
            "INSERT INTO staff_shifts (user_insz, started_at) VALUES ('11111111111', 'x')",
            [],
        )
        .unwrap();
        let (a, b): (Option<String>, Option<String>) = conn
            .query_row("SELECT started_by, ended_by FROM staff_shifts", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((a, b), (None, None));
    }

    #[test]
    fn an_existing_database_keeps_its_users_unflagged_and_does_not_rerun_28() {
        // A database at version 27 (before this migration).
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO users (insz, name, role) VALUES ('11111111111', 'A', 'waiter');
             UPDATE users SET presence_only = 1;
             ALTER TABLE users DROP COLUMN presence_only;
             ALTER TABLE staff_shifts DROP COLUMN started_by;
             ALTER TABLE staff_shifts DROP COLUMN ended_by;
             DELETE FROM _migrations_applied WHERE version = 28;",
        )
        .unwrap();
        run_migrations(&conn).unwrap();
        let flag: i64 = conn
            .query_row("SELECT presence_only FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(flag, 0, "existing users keep 0");
        // A second run does not add the columns again (it would fail).
        run_migrations(&conn).unwrap();
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations_applied WHERE version = 28",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);
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

    /// The till discounts are migration 30 and the kitchen notes 31, in
    /// every build, after 28 (presence registration) and before the
    /// invoicing block (40 to 42) and 70 (staff badges); 29 stays free.
    #[test]
    fn the_till_migrations_are_30_and_31_in_every_build() {
        let till: Vec<(i64, &str)> = TILL_MIGRATIONS
            .iter()
            .map(|m| (m.version, m.name))
            .collect();
        assert_eq!(till, vec![(30, "till-discounts"), (31, "kitchen-notes")]);
        let versions = all_versions();
        assert!(!versions.contains(&29));
        let from_28: Vec<i64> = versions.into_iter().filter(|v| *v >= 28).collect();
        assert_eq!(from_28, vec![28, 30, 31, 40, 41, 42, 70]);
    }

    #[test]
    fn v30_marks_price_change_entries_and_keeps_the_audit_append_only() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Applied once; a second run is a no-op.
        run_migrations(&conn).unwrap();
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations_applied WHERE version = 30",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);

        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (1, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 650, 'A');
             INSERT INTO orders (id, table_number, order_type, status, user_insz, created_at)
                VALUES (1, 1, 'dine_in', 'open', '12345678958', '2026-10-17T10:00:00Z');
             INSERT INTO order_entries (id, order_id, line_id, menu_item_id, quantity, unit_price_cents,
                                        vat_code, name, department_id, department_name, kind, created_at)
                VALUES (1, 1, 1, 1, 2, 650, 'A', 'Sake', '1', 'D', 'entry', '2026-10-17T10:00:00Z');",
        )
        .unwrap();
        // The mark is NULL or `price_change`, nothing else.
        let entry = |till_op: &str| {
            conn.execute(
                &format!(
                    "INSERT INTO order_entries (order_id, line_id, menu_item_id, quantity,
                        unit_price_cents, vat_code, name, department_id, department_name, kind,
                        created_at, till_op)
                     VALUES (1, 1, 1, -2, 650, 'A', 'Sake', '1', 'D', 'correction',
                        '2026-10-17T10:00:00Z', {till_op})"
                ),
                [],
            )
        };
        assert!(entry("'other'").is_err());
        assert!(entry("'price_change'").is_ok());
        // Once set, never rewritten (the older trigger does not list the column).
        assert!(
            conn.execute(
                "UPDATE order_entries SET till_op = NULL WHERE till_op IS NOT NULL",
                []
            )
            .is_err()
        );
        assert!(
            conn.execute(
                "UPDATE order_entries SET till_op = 'price_change' WHERE id = 1",
                []
            )
            .is_err()
        );

        let insert = |action: &str, kind: &str| {
            conn.execute(
                &format!(
                    "INSERT INTO till_discounts (order_id, action, kind, amount_cents, change_id,
                        grouping_id, reason_code, manager_insz)
                     VALUES (1, {action}, {kind}, -100, 'TILL-DISCOUNT', 1, 'other', '12345678958')"
                ),
                [],
            )
        };
        assert!(insert("'apply'", "'percent'").is_ok());
        assert!(insert("'apply'", "'bogus'").is_err());
        assert!(insert("'bogus'", "'percent'").is_err());
        // A removal carries no kind.
        assert!(insert("'remove'", "'percent'").is_err());
        assert!(insert("'remove'", "NULL").is_ok());
        assert!(conn.execute("DELETE FROM till_discounts", []).is_err());
        assert!(
            conn.execute("UPDATE till_discounts SET amount_cents = 0", [])
                .is_err()
        );
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM till_discounts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rows, 2);
    }

    #[test]
    fn v31_adds_the_kitchen_note_columns_to_the_order_lines() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        // Applied once; a second run is a no-op.
        run_migrations(&conn).unwrap();
        let applied: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM _migrations_applied WHERE version = 31",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(applied, 1);

        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (1, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 650, 'A');
             INSERT INTO orders (id, table_number, order_type, status, user_insz, created_at)
                VALUES (1, 1, 'dine_in', 'open', '12345678958', '2026-10-17T10:00:00Z');
             INSERT INTO order_lines (id, order_id, menu_item_id, quantity, unit_price_cents,
                                      vat_code, name, created_at)
                VALUES (1, 1, 1, 2, 650, 'A', 'Sake', '2026-10-17T10:00:00Z');",
        )
        .unwrap();
        // Both columns start empty and are plain editable text.
        let read = |conn: &Connection| -> (Option<String>, Option<String>) {
            conn.query_row(
                "SELECT kitchen_note, kitchen_note_printed FROM order_lines WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap()
        };
        assert_eq!(read(&conn), (None, None));
        conn.execute(
            "UPDATE order_lines SET kitchen_note = 'sans wasabi' WHERE id = 1",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE order_lines SET kitchen_note_printed = kitchen_note WHERE id = 1",
            [],
        )
        .unwrap();
        assert_eq!(
            read(&conn),
            (Some("sans wasabi".into()), Some("sans wasabi".into()))
        );
        // The journal never carries a note.
        let journal_columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('order_entries')")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(!journal_columns.iter().any(|c| c.contains("note")));
    }

    /// The invoicing block starts at 40 and is in every build (V11-D2).
    #[test]
    fn the_invoicing_migrations_are_40_to_42_and_in_every_build() {
        let block: Vec<(i64, &str)> = INVOICING_MIGRATIONS
            .iter()
            .map(|m| (m.version, m.name))
            .collect();
        assert_eq!(
            block,
            vec![(40, "invoicing"), (41, "sold-out"), (42, "menu-editor")]
        );
        assert!(all_versions().contains(&40));
        assert!(INVOICING_MIGRATIONS.iter().all(|m| m.version >= 40));
    }

    fn columns(conn: &Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    #[test]
    fn v42_menu_change_log_is_append_only_and_checks_its_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let cols = columns(&conn, "menu_change_log");
        for c in [
            "entity_type",
            "entity_id",
            "action",
            "before_json",
            "after_json",
            "changed_by",
            "changed_at",
        ] {
            assert!(cols.contains(&c.to_string()), "menu_change_log lacks {c}");
        }
        conn.execute(
            "INSERT INTO menu_change_log (entity_type, entity_id, action, after_json, changed_by)
             VALUES ('product', 1, 'create', '{}', '00000000029')",
            [],
        )
        .unwrap();
        for bad in [
            "INSERT INTO menu_change_log (entity_type, entity_id, action, after_json, changed_by)
             VALUES ('bogus', 1, 'create', '{}', 'x')",
            "INSERT INTO menu_change_log (entity_type, entity_id, action, after_json, changed_by)
             VALUES ('product', 1, 'delete', '{}', 'x')",
        ] {
            assert!(conn.execute(bad, []).is_err(), "{bad}");
        }
        for sql in [
            "UPDATE menu_change_log SET action = 'update'",
            "DELETE FROM menu_change_log",
        ] {
            let err = conn.execute(sql, []).unwrap_err().to_string();
            assert!(err.contains("append-only"), "{sql}: {err}");
        }
    }

    #[test]
    fn v41_creates_the_sold_out_tables() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        for (table, expected) in [
            (
                "menu_item_sold_out",
                vec!["menu_item_id", "until", "set_by", "set_at"],
            ),
            (
                "sold_out_log",
                vec!["menu_item_id", "action", "until", "by", "at"],
            ),
            (
                "hubrise_inventory_sync",
                vec![
                    "menu_item_id",
                    "sku_ref",
                    "dirty",
                    "version",
                    "attempts",
                    "next_attempt_at",
                    "last_attempt_at",
                    "parked_at",
                    "error_msg",
                    "synced_at",
                ],
            ),
        ] {
            let cols = columns(&conn, table);
            for c in expected {
                assert!(cols.contains(&c.to_string()), "{table} lacks {c}");
            }
        }
    }

    #[test]
    fn v41_sold_out_log_is_append_only_and_checks_its_action() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 100, 'B')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sold_out_log (menu_item_id, action, by) VALUES (1, 'set', '00000000029')",
            [],
        )
        .unwrap();
        assert!(
            conn.execute(
                "INSERT INTO sold_out_log (menu_item_id, action) VALUES (1, 'bogus')",
                [],
            )
            .is_err()
        );
        for sql in [
            "UPDATE sold_out_log SET action = 'clear'",
            "DELETE FROM sold_out_log",
        ] {
            let err = conn.execute(sql, []).unwrap_err().to_string();
            assert!(err.contains("append-only"), "{sql}: {err}");
        }
    }

    #[test]
    fn v40_creates_the_invoicing_tables_and_queue_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        for (table, expected) in [
            (
                "b2b_customers",
                vec!["vat_number", "peppol_status", "peppol_checked_at"],
            ),
            (
                "invoices",
                vec![
                    "invoice_seq",
                    "invoice_no",
                    "fiscal_event_id",
                    "customer_json",
                    "vat_json",
                ],
            ),
            (
                "invoice_sales",
                vec!["invoice_id", "sale_fiscal_event_id", "position"],
            ),
            (
                "peppol_deliveries",
                vec![
                    "ubl_xml",
                    "customer_endpoint_id",
                    "status",
                    "attempts",
                    "next_attempt_at",
                    "last_attempt_at",
                    "parked_at",
                    "lookup_status",
                ],
            ),
            (
                "peppol_delivery_log",
                vec![
                    "delivery_id",
                    "attempt",
                    "outcome",
                    "http_status",
                    "detail",
                    "at",
                ],
            ),
        ] {
            let cols = columns(&conn, table);
            for c in expected {
                assert!(cols.contains(&c.to_string()), "{table} lacks {c}");
            }
        }
        let view: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='view' AND name='v_invoice_journal'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(view, 1);
    }

    fn seed_event(conn: &Connection, seq: i64, event_type: &str) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json, fdm_id, event_label, fdm_datetime)
             VALUES (?, ?, 'P', 'T1', '00000000029', '{}', 'FDM1', ?, '2026-10-17T10:00:00')",
            rusqlite::params![event_type, seq, event_type],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn v40_invoice_tables_are_append_only_and_a_sale_is_invoiced_once() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let sale1 = seed_event(&conn, 1, "N");
        let sale2 = seed_event(&conn, 2, "N");
        let inv1 = seed_event(&conn, 3, "I");
        let inv2 = seed_event(&conn, 4, "I");
        let insert_invoice = |seq: i64, no: &str, event: i64| {
            conn.execute(
                "INSERT INTO invoices (invoice_seq, invoice_no, fiscal_event_id, customer_json, issue_date, total_cents, vat_json, created_by)
                 VALUES (?, ?, ?, '{\"vat_number\":\"BE0308357159\",\"name\":\"ACME\"}', '2026-10-17', 1210, '{\"B\":{\"base\":1000,\"vat\":210,\"total\":1210}}', '00000000029')",
                rusqlite::params![seq, no, event],
            )
        };
        insert_invoice(1, "20260001", inv1).unwrap();
        // Invoice number and sequence are unique.
        assert!(insert_invoice(1, "20260002", inv2).is_err());
        assert!(insert_invoice(2, "20260001", inv2).is_err());
        insert_invoice(2, "20260002", inv2).unwrap();

        conn.execute("INSERT INTO invoice_sales VALUES (1, ?, 1)", [sale1])
            .unwrap();
        // The same sale cannot be on a second invoice.
        assert!(
            conn.execute("INSERT INTO invoice_sales VALUES (2, ?, 1)", [sale1])
                .is_err()
        );
        conn.execute("INSERT INTO invoice_sales VALUES (2, ?, 1)", [sale2])
            .unwrap();

        for sql in [
            "UPDATE invoices SET total_cents = 1",
            "DELETE FROM invoices",
            "UPDATE invoice_sales SET position = 9",
            "DELETE FROM invoice_sales",
        ] {
            let err = conn.execute(sql, []).unwrap_err().to_string();
            assert!(err.contains("append-only"), "{sql}: {err}");
        }

        // The journal reads the invoice, the base per rate and the I event.
        let (no, vat, base_b, fdm): (String, String, i64, String) = conn
            .query_row(
                "SELECT invoice_no, customer_vat_number, base_b_cents, fdm_id FROM v_invoice_journal ORDER BY invoice_no LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            (no.as_str(), vat.as_str(), base_b, fdm.as_str()),
            ("20260001", "BE0308357159", 1000, "FDM1")
        );
    }

    #[test]
    fn v40_delivery_log_is_append_only() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO peppol_deliveries (invoice_no, customer_endpoint_id, ap_vendor, ubl_xml)
             VALUES ('20260001', '0208:0308357159', 'logging', '<x/>')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO peppol_delivery_log (delivery_id, attempt, outcome) VALUES (1, 1, 'accepted')",
            [],
        )
        .unwrap();
        // The queue row moves on; the log does not.
        conn.execute(
            "UPDATE peppol_deliveries SET status='accepted', attempts=1",
            [],
        )
        .unwrap();
        assert!(
            conn.execute("UPDATE peppol_delivery_log SET outcome='x'", [])
                .is_err()
        );
        assert!(conn.execute("DELETE FROM peppol_delivery_log", []).is_err());
        // Lookup status is bounded.
        assert!(
            conn.execute("UPDATE peppol_deliveries SET lookup_status='maybe'", [])
                .is_err()
        );
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
