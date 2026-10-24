//! The `pili-accounting` command line: exit codes and messages.

mod common;

use common::*;
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_pili-accounting"))
}

fn archive() -> Synth {
    let mut s = Synth::new();
    s.open_period("p1", "2026-10-01");
    s.sale(
        "p1",
        SaleSpec::new(&[L("10", "A", 12.10)], &[P("CASH", "PAYMENT", 12.10)]),
    );
    s.close("p1", 1);
    s
}

#[test]
fn a_good_run_exits_0_and_lists_the_days() {
    let s = archive();
    let out = tempfile::tempdir().unwrap();
    let r = bin()
        .args([
            "--db",
            s.path.to_str().unwrap(),
            "--month",
            "2026-10",
            "--out",
        ])
        .arg(out.path())
        .args(["--profile", "winbooks", "--decimal", "point"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&r.stdout);
    assert!(
        r.status.success(),
        "{stdout} {}",
        String::from_utf8_lossy(&r.stderr)
    );
    assert!(stdout.starts_with("OK "), "{stdout}");
    assert!(stdout.contains("2026-10-01  Z 1  reconciled"), "{stdout}");
    let folder = out.path().join("export-comptable-2026-10-01-2026-10-31");
    assert!(read(&folder, "journal.csv").starts_with("DBKCODE"));
    assert_eq!(rows(&folder, "jours.csv")[1][4], "12.10");
}

#[test]
fn a_reconciliation_failure_exits_2_and_writes_nothing() {
    let s = archive();
    let copy = s.dir.path().join("copy.db");
    snapshot(&rusqlite::Connection::open(&s.path).unwrap(), &copy);
    let conn = rusqlite::Connection::open(&copy).unwrap();
    conn.execute_batch("DROP TRIGGER fiscal_events_no_rewrite;")
        .unwrap();
    conn.execute(
        "UPDATE fiscal_events SET event_data_json = replace(event_data_json, '\"price\":12.1', '\"price\":12.2')
         WHERE event_type = 'N'",
        [],
    )
    .unwrap();
    drop(conn);
    let out = tempfile::tempdir().unwrap();
    let r = bin()
        .args([
            "--db",
            copy.to_str().unwrap(),
            "--from",
            "2026-10-01",
            "--to",
            "2026-10-31",
            "--out",
        ])
        .arg(out.path())
        .output()
        .unwrap();
    assert_eq!(r.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&r.stderr);
    assert!(stderr.starts_with("RECONCILIATION_FAILED "), "{stderr}");
    assert!(stderr.contains("2026-10-01"), "{stderr}");
    assert_eq!(std::fs::read_dir(out.path()).unwrap().count(), 0);
}

#[test]
fn other_errors_exit_1_with_their_code() {
    let s = archive();
    let out = tempfile::tempdir().unwrap();
    let ok = |args: &[&str]| {
        bin()
            .args(args)
            .arg("--out")
            .arg(out.path())
            .output()
            .unwrap()
    };
    let db = s.path.to_str().unwrap();

    let r = ok(&["--db", "/nonexistent/pos.db", "--month", "2026-10"]);
    assert_eq!(r.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&r.stderr).starts_with("ERROR db_unreadable:"));

    let r = ok(&["--db", db, "--month", "2026-11"]);
    assert!(String::from_utf8_lossy(&r.stderr).starts_with("ERROR empty_range:"));

    let r = ok(&["--db", db, "--month", "2026-10"]);
    assert!(r.status.success());
    let r = ok(&["--db", db, "--month", "2026-10"]);
    assert_eq!(r.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&r.stderr).starts_with("ERROR out_exists:"));

    let r = ok(&["--db", db]);
    assert_eq!(r.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&r.stderr).contains("--month"));
    let r = ok(&["--db", db, "--month", "2026-10", "--profile", "exact"]);
    assert_eq!(r.status.code(), Some(1));
    let r = ok(&["--db", db, "--month", "2026-10", "--lang", "zh"]);
    assert_eq!(r.status.code(), Some(1));
    let r = ok(&["--db", db, "--month", "2026-10", "--bogus"]);
    assert_eq!(r.status.code(), Some(1));
}

#[test]
fn help_exits_0() {
    let r = bin().arg("--help").output().unwrap();
    assert!(r.status.success());
    assert!(String::from_utf8_lossy(&r.stdout).contains("USAGE"));
}
