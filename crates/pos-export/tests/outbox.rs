//! The outbox: preparation, retention, status and start-up cleaning (card
//! X1-T5, decisions X1D6, X1D7, X1D8 and X1D10). The upload passes are in
//! `outbox_upload.rs`.

mod common;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use common::{ExportKit, Fixture, add_empty_z, export_kit, fixture_db};
use pos_export::manifest::Manifest;
use pos_export::outbox::{self, ExportEnv};
use pos_export::status::ExportStatus;
use pos_export::{crypt, naming, stats};
use rusqlite::Connection;
use serde_json::Value;

const ID_41: &str = "20261003-z000041";
const ID_42: &str = "20261004-z000042";

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// The morning after the day of Z 42.
fn now() -> DateTime<Utc> {
    t("2026-10-05T01:00:00Z")
}

struct Rig {
    _dir: tempfile::TempDir,
    fx: Fixture,
    kit: ExportKit,
    exports: PathBuf,
}

impl Rig {
    fn new(keep_days: i64) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let fx = fixture_db(&dir.path().join("pos.db"));
        let kit = export_kit(dir.path(), "http://127.0.0.1:1", keep_days);
        let exports = dir.path().join("exports");
        Rig {
            _dir: dir,
            fx,
            kit,
            exports,
        }
    }

    fn env(&self) -> ExportEnv<'_> {
        ExportEnv {
            db_path: &self.fx.db_path,
            exports_dir: &self.exports,
            settings: &self.kit.settings,
            pos_id: common::POS_ID,
            terminal_id: common::TERMINAL_ID,
            pos_sw_version: common::SW_VERSION,
        }
    }

    fn prepare(&self, at: DateTime<Utc>) -> Result<Vec<String>, String> {
        outbox::prepare_due(&self.env(), at)
    }

    fn set_cursor(&self, no: i64) {
        std::fs::create_dir_all(&self.exports).unwrap();
        std::fs::write(
            self.exports.join("cursor.json"),
            format!("{{\"lastZReportNo\": {no}}}"),
        )
        .unwrap();
    }

    fn cursor(&self) -> i64 {
        json(&self.exports.join("cursor.json"))["lastZReportNo"]
            .as_i64()
            .unwrap()
    }

    fn status(&self) -> ExportStatus {
        outbox::status(&self.exports, Some(&self.kit.settings), None)
    }
}

fn json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// Names of the export directories, sorted.
fn export_dirs(exports: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(exports)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_dir())
        .map(|e| e.file_name().into_string().unwrap())
        .filter(|n| n.contains("-z"))
        .collect();
    names.sort();
    names
}

fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

fn all_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            all_files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn state(rig: &Rig, id: &str) -> Value {
    json(&rig.exports.join(id).join("state.json"))
}

// ── Preparation ────────────────────────────────────────────────────

#[test]
fn a_first_enable_exports_only_the_latest_z() {
    let rig = Rig::new(30);
    let ids = rig.prepare(now()).unwrap();
    assert_eq!(ids, vec![ID_42.to_string()]);
    assert_eq!(export_dirs(&rig.exports), vec![ID_42.to_string()]);
    assert_eq!(rig.cursor(), 42);
    assert_eq!(
        file_names(&rig.exports.join(ID_42)),
        vec![
            naming::manifest_name(ID_42),
            naming::backup_name(ID_42),
            "state.json".to_string(),
            naming::stats_name(ID_42),
        ]
    );
    let s = state(&rig, ID_42);
    assert_eq!(s["exportId"], ID_42);
    assert_eq!(s["createdAt"], "2026-10-05T01:00:00Z");
    assert_eq!(s["zReportNo"], 42);
    assert_eq!(s["bookingDate"], "2026-10-04");
    assert_eq!(s["uploaded"], serde_json::json!([]));
    assert_eq!(s["attempts"], 0);
    assert!(s["nextAttemptAt"].is_null());
    assert!(s["lastError"].is_null());
}

#[test]
fn a_database_without_a_z_starts_the_cursor_at_zero() {
    let rig = Rig::new(30);
    let empty = rig._dir.path().join("empty.db");
    {
        let conn = Connection::open(&empty).unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
    }
    let env = ExportEnv {
        db_path: &empty,
        ..rig.env()
    };
    assert!(outbox::prepare_due(&env, now()).unwrap().is_empty());
    assert_eq!(rig.cursor(), 0);
    assert!(export_dirs(&rig.exports).is_empty());
}

#[test]
fn a_second_prepare_does_nothing() {
    let rig = Rig::new(30);
    rig.prepare(now()).unwrap();
    let before = std::fs::read(rig.exports.join(ID_42).join(naming::manifest_name(ID_42))).unwrap();
    assert!(rig.prepare(now() + Duration::hours(1)).unwrap().is_empty());
    assert_eq!(export_dirs(&rig.exports), vec![ID_42.to_string()]);
    assert_eq!(rig.cursor(), 42);
    let after = std::fs::read(rig.exports.join(ID_42).join(naming::manifest_name(ID_42))).unwrap();
    assert_eq!(before, after);
}

#[test]
fn a_new_z_is_prepared() {
    let rig = Rig::new(30);
    rig.prepare(now()).unwrap();
    let id = add_empty_z(&rig.fx, 43, "2026-10-05");
    let later = now() + Duration::days(1);
    assert_eq!(rig.prepare(later).unwrap(), vec![id.clone()]);
    assert_eq!(
        export_dirs(&rig.exports),
        vec![ID_42.to_string(), id.clone()]
    );
    assert_eq!(rig.cursor(), 43);
    let m = Manifest::read(&rig.exports.join(&id).join(naming::manifest_name(&id))).unwrap();
    assert_eq!(m.z_report_no, 43);
    assert_eq!(m.booking_date, "2026-10-05");
    assert_eq!(m.created_at, "2026-10-06T01:00:00Z");
}

#[test]
fn two_missed_zs_are_both_prepared_in_order() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    let ids = rig.prepare(now()).unwrap();
    assert_eq!(ids, vec![ID_41.to_string(), ID_42.to_string()]);
    assert_eq!(rig.cursor(), 42);

    let id_43 = add_empty_z(&rig.fx, 43, "2026-10-05");
    let id_44 = add_empty_z(&rig.fx, 44, "2026-10-06");
    let ids = rig.prepare(now() + Duration::days(2)).unwrap();
    assert_eq!(ids, vec![id_43, id_44]);
    assert_eq!(rig.cursor(), 44);
    assert_eq!(rig.status().kept, 4);
}

#[test]
fn a_corrupt_cursor_resumes_after_the_newest_export_and_is_reported() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    rig.prepare(now()).unwrap();
    assert_eq!(rig.cursor(), 42);

    let id_43 = add_empty_z(&rig.fx, 43, "2026-10-05");
    let id_44 = add_empty_z(&rig.fx, 44, "2026-10-06");
    std::fs::write(rig.exports.join("cursor.json"), "{ not json").unwrap();

    // The Zs in between are not skipped, and the operator sees the problem.
    let ids = rig.prepare(now() + Duration::days(2)).unwrap();
    assert_eq!(ids, vec![id_43, id_44]);
    assert_eq!(rig.cursor(), 44);
    let status = rig.status();
    assert!(
        status
            .last_error
            .as_deref()
            .is_some_and(|e| e.contains("cursor.json")),
        "{:?}",
        status.last_error
    );
}

#[test]
fn a_z_older_than_keep_days_is_skipped_and_the_cursor_moves_past_it() {
    // One day kept: at 2026-10-05 01:00 UTC the oldest booking date is 2026-10-04.
    let rig = Rig::new(1);
    rig.set_cursor(40);
    let ids = rig.prepare(now()).unwrap();
    assert_eq!(ids, vec![ID_42.to_string()]);
    assert_eq!(export_dirs(&rig.exports), vec![ID_42.to_string()]);
    assert_eq!(rig.cursor(), 42);

    // Everything is old: nothing is built, the cursor still moves.
    let rig = Rig::new(1);
    rig.set_cursor(40);
    assert!(rig.prepare(now() + Duration::days(10)).unwrap().is_empty());
    assert!(export_dirs(&rig.exports).is_empty());
    assert_eq!(rig.cursor(), 42);
}

#[test]
fn no_plaintext_database_is_left_except_the_stats_files() {
    let rig = Rig::new(30);
    // A crashed earlier run left a plaintext snapshot in .tmp.
    let stale = rig.exports.join(".tmp").join("20260101-z000001");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("pos.db"), b"SQLite format 3\0stale").unwrap();
    rig.set_cursor(40);
    rig.prepare(now()).unwrap();

    let mut files = Vec::new();
    all_files(&rig.exports, &mut files);
    let mut stats_files = 0;
    for path in files {
        let head = std::fs::read(&path).unwrap();
        let is_sqlite = head.starts_with(b"SQLite format 3\0");
        let name = path.file_name().unwrap().to_str().unwrap();
        if name.starts_with("stats-") && name.ends_with(".sqlite") {
            assert!(is_sqlite, "{name} is a statistics database");
            stats_files += 1;
        } else {
            assert!(
                !is_sqlite,
                "{} holds a plaintext SQLite database",
                path.display()
            );
            assert!(
                !name.ends_with("-wal") && !name.ends_with("-shm") && !name.ends_with("-journal")
            );
        }
    }
    assert_eq!(stats_files, 2);
    assert!(file_names(&rig.exports.join(".tmp")).is_empty());
}

#[test]
fn a_failure_leaves_no_partial_export_and_no_plaintext() {
    let rig = Rig::new(30);

    // 1. The database cannot be opened: nothing is created, the cursor is not set.
    let missing = rig._dir.path().join("absent.db");
    let env = ExportEnv {
        db_path: &missing,
        ..rig.env()
    };
    let err = outbox::prepare_due(&env, now()).unwrap_err();
    assert!(err.starts_with("prepare failed: "), "{err}");
    assert!(export_dirs(&rig.exports).is_empty());
    assert!(file_names(&rig.exports.join(".tmp")).is_empty());
    assert!(!rig.exports.join("cursor.json").exists());
    let status = json(&rig.exports.join("status.json"));
    assert_eq!(status["lastError"], err.as_str());
    assert_eq!(status["lastErrorAt"], "2026-10-05T01:00:00Z");

    // 2. The statistics cannot be built after the snapshot exists: the key
    // file is a directory. The work directory, with its plaintext snapshot,
    // is gone; the cursor stays where it was.
    std::fs::create_dir_all(rig.exports.join("pseudonym.key")).unwrap();
    let err = rig.prepare(now() + Duration::minutes(5)).unwrap_err();
    assert!(err.starts_with("prepare failed: "), "{err}");
    assert!(export_dirs(&rig.exports).is_empty());
    assert!(file_names(&rig.exports.join(".tmp")).is_empty());
    assert_eq!(rig.cursor(), 41, "the cursor stays before Z 42");
    let mut files = Vec::new();
    all_files(&rig.exports, &mut files);
    for path in &files {
        assert!(
            !std::fs::read(path)
                .unwrap_or_default()
                .starts_with(b"SQLite format 3\0"),
            "{} is plaintext",
            path.display()
        );
    }
    let status = rig.status();
    assert_eq!(status.last_error.as_deref(), Some(err.as_str()));
    assert_eq!(
        status.last_error_at.as_deref(),
        Some("2026-10-05T01:05:00Z")
    );
    assert_eq!(status.kept, 0);

    // 3. The next pass retries and succeeds once the cause is gone.
    std::fs::remove_dir(rig.exports.join("pseudonym.key")).unwrap();
    assert_eq!(
        rig.prepare(now() + Duration::minutes(10)).unwrap(),
        vec![ID_42.to_string()]
    );
    assert_eq!(rig.cursor(), 42);
}

#[test]
fn a_stats_build_error_leaves_nothing_either() {
    // A Z whose period holds an unreadable event: the statistics refuse it
    // after the snapshot was taken.
    let rig = Rig::new(30);
    {
        let conn = Connection::open(&rig.fx.db_path).unwrap();
        // The test database is a throwaway: lift the guard that checks the
        // JSON of an inserted event, to plant a corrupt one.
        conn.execute_batch("DROP TRIGGER fiscal_events_period_open;")
            .unwrap();
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('R', 900, 'P', 'T', '00000000029', 'not json')",
            [],
        )
        .unwrap();
        let bad_event = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO booking_periods (id, booking_date, opened_at, closed_at, closing_event_id)
             VALUES ('bad-period', '2026-10-06', '2026-10-06T07:00:00.000Z', '2026-10-06T22:00:00.000Z', ?1)",
            [bad_event],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO z_reports (report_date, sequence_number, data_json, fiscal_event_id)
             VALUES ('2026-10-06', 43, '{}', ?1)",
            [bad_event],
        )
        .unwrap();
    }
    rig.set_cursor(42);
    let err = rig.prepare(now() + Duration::days(1)).unwrap_err();
    assert!(err.starts_with("prepare failed: stats: "), "{err}");
    assert!(export_dirs(&rig.exports).is_empty());
    assert!(file_names(&rig.exports.join(".tmp")).is_empty());
    assert_eq!(rig.cursor(), 42);
    let mut files = Vec::new();
    all_files(&rig.exports, &mut files);
    for path in files {
        assert!(
            !std::fs::read(&path)
                .unwrap()
                .starts_with(b"SQLite format 3\0")
        );
    }
}

#[test]
fn the_backup_decrypts_to_the_database_and_the_manifest_verifies() {
    let rig = Rig::new(30);
    rig.prepare(now()).unwrap();
    let dir = rig.exports.join(ID_42);

    let m = Manifest::read(&dir.join(naming::manifest_name(ID_42))).unwrap();
    m.verify(&dir).unwrap();
    assert_eq!(m.export_id, ID_42);
    assert_eq!(m.pos_id, common::POS_ID);
    assert_eq!(m.terminal_id, common::TERMINAL_ID);
    assert_eq!(m.pos_sw_version, common::SW_VERSION);
    assert_eq!(m.z_report_no, 42);
    assert_eq!(m.booking_date, "2026-10-04");
    assert_eq!(m.stats_schema_version, stats::STATS_SCHEMA_VERSION);
    assert_eq!(m.created_at, "2026-10-05T01:00:00Z");
    assert_eq!(m.recipients, rig.kit.settings.recipient_strings());
    let names: Vec<&str> = m.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        vec![naming::backup_name(ID_42), naming::stats_name(ID_42)]
    );

    let plain = rig._dir.path().join("restored.db");
    crypt::decrypt_file(
        &rig.kit.owner_secret,
        &dir.join(naming::backup_name(ID_42)),
        &plain,
    )
    .unwrap();
    let count = |path: &Path| -> i64 {
        common::open_ro(path)
            .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
            .unwrap()
    };
    assert!(count(&rig.fx.db_path) > 10);
    assert_eq!(count(&plain), count(&rig.fx.db_path));
    let verdict: String = common::open_ro(&plain)
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(verdict, "ok");

    // The statistics file of the export is clean and describes Z 42.
    let stats_file = dir.join(naming::stats_name(ID_42));
    common::assert_no_personal_data(&stats_file, &rig.fx.secrets);
    let conn = common::open_ro(&stats_file);
    let tickets: i64 = conn
        .query_row("SELECT COUNT(*) FROM tickets", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tickets, rig.fx.tickets.len() as i64);
    let generated: String = conn
        .query_row("SELECT value FROM meta WHERE key = 'export_id'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(generated, ID_42);
    assert!(rig.exports.join("pseudonym.key").exists());
}

#[test]
fn the_sizes_of_the_export_of_the_test_day() {
    let rig = Rig::new(30);
    rig.prepare(now()).unwrap();
    let dir = rig.exports.join(ID_42);
    let size = |name: String| std::fs::metadata(dir.join(name)).unwrap().len();
    let backup = size(naming::backup_name(ID_42));
    let stats_bytes = size(naming::stats_name(ID_42));
    let manifest = size(naming::manifest_name(ID_42));
    println!(
        "export of the fixture day: backup {backup} bytes, stats {stats_bytes} bytes, manifest {manifest} bytes, total {} bytes, source {} bytes",
        backup + stats_bytes + manifest,
        std::fs::metadata(&rig.fx.db_path).unwrap().len()
    );
    assert!(backup > 0 && stats_bytes > 0 && manifest > 0);
}

#[test]
fn the_source_database_is_only_read() {
    let rig = Rig::new(30);
    let events = |p: &Path| -> i64 {
        common::open_ro(p)
            .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
            .unwrap()
    };
    let before = events(&rig.fx.db_path);
    rig.prepare(now()).unwrap();
    assert_eq!(events(&rig.fx.db_path), before);
}

// ── Cleaning, retention, status ────────────────────────────────────

#[test]
fn clean_tmp_empties_the_work_area_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let exports = dir.path().join("exports");
    // No directory yet: it is created.
    outbox::clean_tmp(&exports).unwrap();
    assert!(exports.join(".tmp").is_dir());

    std::fs::create_dir_all(exports.join(".tmp/20261004-z000042")).unwrap();
    std::fs::write(
        exports.join(".tmp/20261004-z000042/pos.db"),
        b"SQLite format 3\0",
    )
    .unwrap();
    std::fs::write(exports.join(".tmp/stray"), b"x").unwrap();
    std::fs::write(exports.join("cursor.json"), b"{\"lastZReportNo\":1}").unwrap();
    std::fs::write(exports.join("pseudonym.key"), [7u8; 32]).unwrap();
    std::fs::create_dir_all(exports.join(ID_42)).unwrap();
    outbox::clean_tmp(&exports).unwrap();
    assert!(exports.join(".tmp").is_dir());
    assert!(file_names(&exports.join(".tmp")).is_empty());
    assert!(exports.join("cursor.json").exists());
    assert!(exports.join("pseudonym.key").exists());
    assert!(exports.join(ID_42).is_dir());
}

#[test]
fn retention_deletes_old_exports_and_counts_the_unsent_ones() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    rig.prepare(now()).unwrap();
    // Things the retention must never touch.
    std::fs::write(rig.exports.join("notes.txt"), b"keep").unwrap();
    std::fs::create_dir_all(rig.exports.join("other")).unwrap();
    let key = std::fs::read(rig.exports.join("pseudonym.key")).unwrap();
    let outside = rig._dir.path().join("outside.txt");
    std::fs::write(&outside, b"keep").unwrap();

    // Not yet old.
    assert_eq!(
        outbox::retention(&rig.env(), now() + Duration::days(29)).unwrap(),
        0
    );
    assert_eq!(rig.status().kept, 2);

    // Both are older than 30 days and never sent.
    let at = now() + Duration::days(31);
    assert_eq!(outbox::retention(&rig.env(), at).unwrap(), 2);
    let status = rig.status();
    assert_eq!((status.kept, status.pending, status.dropped), (0, 0, 2));
    assert!(rig.exports.join("cursor.json").exists());
    assert!(rig.exports.join("status.json").exists());
    assert_eq!(
        std::fs::read(rig.exports.join("pseudonym.key")).unwrap(),
        key
    );
    assert!(rig.exports.join("notes.txt").exists());
    assert!(rig.exports.join("other").is_dir());
    assert!(outside.exists());
    assert_eq!(rig.cursor(), 42);

    // Nothing left: nothing more is counted.
    assert_eq!(outbox::retention(&rig.env(), at).unwrap(), 0);
    assert_eq!(rig.status().dropped, 2);
}

#[test]
fn status_without_any_file_is_empty_never_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let nothing = dir.path().join("exports");
    assert_eq!(
        outbox::status(&nothing, None, None),
        ExportStatus::disabled(None)
    );

    let kit = export_kit(dir.path(), "https://export.example.com:8443/v1/inst-42", 14);
    let s = outbox::status(&nothing, Some(&kit.settings), None);
    assert!(s.enabled);
    assert_eq!(
        s.endpoint.as_deref(),
        Some("https://export.example.com:8443")
    );
    assert_eq!(s.keep_days, 14);
    assert_eq!((s.pending, s.kept, s.dropped), (0, 0, 0));
    assert!(s.last_export_id.is_none() && s.last_error.is_none() && s.next_attempt_at.is_none());

    // A configuration problem switches it off but keeps what is known.
    let s = outbox::status(
        &nothing,
        Some(&kit.settings),
        Some("export.url: bad".into()),
    );
    assert!(!s.enabled);
    assert_eq!(s.problem.as_deref(), Some("export.url: bad"));
    let s = outbox::status(&nothing, None, Some("export.url: bad".into()));
    assert_eq!(s, ExportStatus::disabled(Some("export.url: bad".into())));

    // Unreadable files are as good as missing ones.
    std::fs::create_dir_all(&nothing).unwrap();
    std::fs::write(nothing.join("status.json"), b"{not json").unwrap();
    std::fs::write(nothing.join("cursor.json"), b"").unwrap();
    let s = outbox::status(&nothing, Some(&kit.settings), None);
    assert!(s.enabled && s.last_error.is_none() && s.dropped == 0);
}

#[test]
fn status_after_a_preparation_shows_one_pending_export() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    rig.prepare(now()).unwrap();
    let s = rig.status();
    assert!(s.enabled && s.problem.is_none());
    assert_eq!(s.endpoint.as_deref(), Some("http://127.0.0.1:1"));
    assert_eq!(s.keep_days, 30);
    assert_eq!(s.kept, 2);
    assert_eq!(s.pending, 2);
    assert_eq!(s.last_export_id.as_deref(), Some(ID_42));
    assert_eq!(s.last_prepared_at.as_deref(), Some("2026-10-05T01:00:00Z"));
    assert!(s.next_attempt_at.is_none(), "never tried: due at once");
    assert!(s.last_uploaded_id.is_none() && s.last_uploaded_at.is_none());
    assert!(s.last_error.is_none() && s.last_error_at.is_none());
    assert_eq!(s.dropped, 0);
    // It serialises with the contract's names.
    let value = serde_json::to_value(&s).unwrap();
    assert_eq!(value["lastExportId"], ID_42);
    assert_eq!(value["keepDays"], 30);
}

#[test]
fn the_status_files_hold_no_secret() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    rig.prepare(now()).unwrap();
    let missing = rig._dir.path().join("absent.db");
    let env = ExportEnv {
        db_path: &missing,
        ..rig.env()
    };
    let _ = outbox::prepare_due(&env, now());
    let mut files = Vec::new();
    all_files(&rig.exports, &mut files);
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let hay = String::from_utf8_lossy(&bytes);
        assert!(!hay.contains(&rig.kit.token), "{}", path.display());
        assert!(!hay.contains(&rig.kit.owner_secret), "{}", path.display());
    }
}
