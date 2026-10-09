//! The privacy proof of the statistics file (card X1-T3): a database full of
//! personal data goes in, nothing of it comes out.

mod common;

use std::collections::BTreeSet;
use std::path::Path;

use common::*;
use pos_export::stats::STATS_TABLES;

/// The tables of the schema, as documented in `STATS_SCHEMA.md`.
const DOCUMENTED_TABLES: [&str; 10] = [
    "meta",
    "ticket_discounts",
    "ticket_lines",
    "ticket_payments",
    "ticket_vat",
    "tickets",
    "users",
    "z_payments",
    "z_report",
    "z_vat",
];

const SCHEMA_DOC: &str = include_str!("../STATS_SCHEMA.md");

fn built() -> (tempfile::TempDir, Fixture, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    let out = dir.path().join("stats.sqlite");
    build_fixture_stats(&fx, &out);
    (dir, fx, out)
}

#[test]
fn the_fixture_is_full_of_the_data_the_proof_looks_for() {
    let dir = tempfile::tempdir().unwrap();
    let fx = fixture_db(&dir.path().join("pos.db"));
    // Every secret really is in the database file the statistics are built from.
    let bytes = std::fs::read(&fx.db_path).unwrap();
    let missing: Vec<&String> = fx
        .secrets
        .iter()
        .filter(|s| !bytes.windows(s.len()).any(|w| w == s.as_bytes()))
        .collect();
    assert!(
        missing.is_empty(),
        "planted but not in the file: {missing:?}"
    );
    for insz in [ADMIN_INSZ, CASHIER_INSZ, WAITER_INSZ] {
        assert!(fx.secrets.iter().any(|s| s == insz));
    }
    assert!(fx.secrets.len() >= 25, "{} secrets", fx.secrets.len());

    let conn = open_ro(&fx.db_path);
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |r| r.get(0)).unwrap() };
    assert_eq!(
        count("SELECT COUNT(*) FROM users WHERE pin_hash LIKE '$2%'"),
        3
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM hubrise_connection WHERE access_token != ''"),
        1
    );
    assert_eq!(
        count("SELECT COUNT(*) FROM ticket_deliveries WHERE recipient LIKE '%@%'"),
        1
    );
    assert_eq!(
        count(
            "SELECT COUNT(*) FROM orders WHERE raw_hubrise_json LIKE '%Dupont%' AND raw_hubrise_json LIKE '%@%'"
        ),
        2
    );
    for label in ["S", "C", "T", "P", "F", "I"] {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM fiscal_events WHERE event_type = ?1",
                [label],
                |r| r.get(0),
            )
            .unwrap();
        assert!(n >= 1, "no {label} event in the fixture");
    }
}

#[test]
fn the_table_set_is_exactly_the_documented_one() {
    let (_dir, _fx, out) = built();
    let conn = open_ro(&out);
    assert_eq!(
        table_names(&conn),
        DOCUMENTED_TABLES.map(String::from).to_vec()
    );
    let mut declared: Vec<&str> = STATS_TABLES.to_vec();
    declared.sort_unstable();
    assert_eq!(declared, DOCUMENTED_TABLES.to_vec());
    // Nothing else lives in the file: no view, trigger or index of ours.
    let others: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('view', 'trigger')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(others, 0);
}

/// The rows of the column table under the heading of `table` in the document.
fn documented_columns(table: &str) -> BTreeSet<String> {
    let heading = format!("### `{table}`");
    let start = SCHEMA_DOC
        .find(&heading)
        .unwrap_or_else(|| panic!("STATS_SCHEMA.md has no section for {table}"));
    let section = &SCHEMA_DOC[start + heading.len()..];
    let end = section.find("\n##").unwrap_or(section.len());
    let section = &section[..end];
    // Only the first table of the section lists columns (meta has a second one for its keys).
    let mut columns = BTreeSet::new();
    let mut in_table = false;
    for line in section.lines() {
        if line.starts_with("| column |") {
            in_table = true;
        } else if in_table {
            match line.strip_prefix("| `") {
                Some(rest) => {
                    columns.insert(rest.split('`').next().unwrap().to_string());
                }
                None if line.starts_with("|--") => {}
                None => break,
            }
        }
    }
    columns
}

#[test]
fn the_document_lists_every_table_and_column_and_nothing_else() {
    let (_dir, _fx, out) = built();
    let conn = open_ro(&out);
    for table in table_names(&conn) {
        let actual: BTreeSet<String> = column_names(&conn, &table).into_iter().collect();
        assert_eq!(documented_columns(&table), actual, "columns of {table}");
    }
    // The document names the versioning rule, the channels and the pseudonym.
    for needle in [
        "## Versioning",
        "## Channels",
        "## Pseudonyms",
        "## Privacy guarantees",
        "pili-stats-user/1",
    ] {
        assert!(SCHEMA_DOC.contains(needle), "{needle}");
    }
    assert!(
        !SCHEMA_DOC.contains('\u{2014}'),
        "no em-dash in the document"
    );
}

#[test]
fn every_column_has_a_comment_in_its_declaration() {
    let (_dir, _fx, out) = built();
    let conn = open_ro(&out);
    for table in table_names(&conn) {
        let sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name = ?1",
                [&table],
                |r| r.get(0),
            )
            .unwrap();
        let columns = column_names(&conn, &table).len();
        let comments = sql.lines().filter(|l| l.contains("-- ")).count();
        assert!(
            comments >= columns,
            "{table}: {comments} comments for {columns} columns"
        );
    }
}

#[test]
fn the_built_file_holds_no_personal_data() {
    let (_dir, fx, out) = built();
    assert_no_personal_data(&out, &fx.secrets);
}

#[test]
fn the_check_constraints_close_the_value_sets() {
    let (_dir, _fx, out) = built();
    let conn = rusqlite::Connection::open(&out).unwrap();
    for bad in [
        "INSERT INTO users VALUES ('x', 'owner')",
        "INSERT INTO tickets VALUES (99, 'N', 1, 'gift', 'table', NULL, 'x', 'd', 0, 0)",
        "INSERT INTO tickets VALUES (99, 'N', 1, 'sale', 'drive', NULL, 'x', 'd', 0, 0)",
        "INSERT INTO tickets VALUES (99, 'N', 1, 'sale', 'table', NULL, 'x', 'd', 0, 2)",
        "INSERT INTO ticket_payments VALUES (1, 9, 'bitcoin', 'payment', 1)",
        "INSERT INTO ticket_payments VALUES (1, 9, 'cash', 'gift', 1)",
        "INSERT INTO ticket_discounts VALUES (1, NULL, 'secret', 1)",
        "INSERT INTO ticket_vat VALUES (1, 'Z', 0, 0, 0, 0)",
    ] {
        assert!(conn.execute(bad, []).is_err(), "{bad}");
    }
}

// ── The proof itself must catch leaks ──────────────────────────────

fn leaky(sql: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("leaky.sqlite");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch(sql).unwrap();
    drop(conn);
    (dir, path)
}

fn check(sql: &str, secrets: &[&str]) -> bool {
    let (_dir, path) = leaky(sql);
    let secrets: Vec<String> = secrets.iter().map(|s| s.to_string()).collect();
    std::panic::catch_unwind(|| assert_no_personal_data(Path::new(&path), &secrets)).is_ok()
}

#[test]
fn the_proof_accepts_a_clean_file() {
    assert!(check(
        "CREATE TABLE t (id INTEGER, product_name TEXT); INSERT INTO t VALUES (1, 'Nigiri 12');",
        &["Dupont"]
    ));
}

#[test]
fn the_proof_catches_a_forbidden_column_name() {
    for column in [
        "insz",
        "user_niss",
        "pin_hash",
        "token",
        "client_secret",
        "password",
        "email",
        "mobile_phone",
        "address_1",
        "customer_id",
        "short_signature",
        "qr_data",
        "name",
        "user_name",
        "Name",
    ] {
        let ok = check(&format!("CREATE TABLE t (\"{column}\" TEXT);"), &[]);
        assert!(!ok, "column {column} was accepted");
    }
}

#[test]
fn the_proof_catches_a_leaking_value() {
    for value in [
        "75061189702",
        "ref 12345678901 x",
        "jean@example.com",
        "$2b$12$abcdefghijklmnopqrstuv",
        "$2a$10$x",
        "$2y$10$x",
        "Mr Dupont",
    ] {
        let ok = check(
            &format!("CREATE TABLE t (note TEXT); INSERT INTO t VALUES ('{value}');"),
            &["Dupont"],
        );
        assert!(!ok, "value {value} was accepted");
    }
    // Ten digits and a number stored as an integer are not an INSZ.
    assert!(check(
        "CREATE TABLE t (note TEXT, n INTEGER); INSERT INTO t VALUES ('1234567890', 75061189702);",
        &[]
    ));
}

#[test]
fn the_proof_catches_a_secret_left_in_the_file_bytes() {
    // The value was deleted, but the page still holds it.
    let ok = check(
        "PRAGMA secure_delete = OFF; CREATE TABLE t (note TEXT); INSERT INTO t VALUES ('hr_at_secret_token_value'); DELETE FROM t;",
        &["hr_at_secret_token_value"],
    );
    assert!(!ok);
}
