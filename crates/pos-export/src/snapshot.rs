//! Consistent copy of the live database while the POS keeps selling.

use std::path::Path;
use std::time::Duration;

use rusqlite::backup::Backup;
use rusqlite::{Connection, OpenFlags};

/// Pages copied per step of the online backup.
const PAGES_PER_STEP: i32 = 256;
/// Pause between two steps, so the POS keeps its write access.
const PAUSE: Duration = Duration::from_millis(5);

/// Copy the database `db_path` to `dest` with the SQLite online backup API
/// and return the size of the copy in bytes.
///
/// The source is opened read-only and copied by steps of 256 pages with a
/// 5 ms pause, so a sale can be written meanwhile (WAL). SQLite restarts the
/// copy when another connection changes the source during it; the copy is
/// therefore always a consistent state. The copy is left in rollback-journal
/// mode (a single self-contained file) and must pass `PRAGMA
/// integrity_check`. An existing `dest` is replaced.
pub fn snapshot(db_path: &Path, dest: &Path) -> Result<i64, String> {
    let src = Connection::open_with_flags(
        db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("snapshot: cannot open {}: {e}", db_path.display()))?;
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = dest.as_os_str().to_os_string();
        name.push(suffix);
        match std::fs::remove_file(&name) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("snapshot: cannot clear {name:?}: {e}")),
        }
    }
    let result = copy_and_check(&src, dest);
    drop(src);
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result?;
    let len = std::fs::metadata(dest)
        .map_err(|e| format!("snapshot: cannot stat {}: {e}", dest.display()))?
        .len();
    i64::try_from(len).map_err(|_| "snapshot: size out of range".to_string())
}

fn copy_and_check(src: &Connection, dest: &Path) -> Result<(), String> {
    let mut dst = Connection::open(dest)
        .map_err(|e| format!("snapshot: cannot create {}: {e}", dest.display()))?;
    {
        let backup = Backup::new(src, &mut dst).map_err(|e| format!("snapshot: backup: {e}"))?;
        backup
            .run_to_completion(PAGES_PER_STEP, PAUSE, None)
            .map_err(|e| format!("snapshot: copy: {e}"))?;
    }
    // The copy carries the WAL flag of the source: make it a plain file.
    dst.query_row("PRAGMA journal_mode = DELETE", [], |r| {
        r.get::<_, String>(0)
    })
    .map_err(|e| format!("snapshot: journal mode: {e}"))?;
    let verdict: String = dst
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| format!("snapshot: integrity check: {e}"))?;
    if verdict != "ok" {
        return Err(format!("snapshot: integrity check failed: {verdict}"));
    }
    dst.close()
        .map_err(|(_, e)| format!("snapshot: close: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM probe", [], |r| r.get(0))
            .unwrap()
    }

    fn tables(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn a_wal_database_written_during_the_copy_gives_a_consistent_copy() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("pos.db");
        let conn = Connection::open(&db).unwrap();
        conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get::<_, String>(0))
            .unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        conn.execute_batch("CREATE TABLE probe (id INTEGER PRIMARY KEY, v BLOB NOT NULL);")
            .unwrap();
        let before = 3000;
        for i in 0..before {
            conn.execute("INSERT INTO probe (id, v) VALUES (?1, zeroblob(512))", [i])
                .unwrap();
        }
        let source_tables = tables(&conn);
        assert!(source_tables > 2, "migrations must have created tables");

        let more = 40;
        let barrier = Arc::new(Barrier::new(2));
        let writer = {
            let barrier = Arc::clone(&barrier);
            let db = db.clone();
            std::thread::spawn(move || {
                let w = Connection::open(&db).unwrap();
                w.busy_timeout(Duration::from_secs(5)).unwrap();
                barrier.wait();
                for i in 0..more {
                    w.execute(
                        "INSERT INTO probe (id, v) VALUES (?1, zeroblob(512))",
                        [100_000 + i],
                    )
                    .unwrap();
                    std::thread::sleep(Duration::from_millis(2));
                }
            })
        };

        let copy = dir.path().join("copy.db");
        barrier.wait();
        let size = snapshot(&db, &copy).unwrap();
        writer.join().unwrap();

        assert!(size > 0);
        assert_eq!(size, std::fs::metadata(&copy).unwrap().len() as i64);
        let c = Connection::open_with_flags(&copy, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let n = count(&c);
        assert!(
            (before..=before + more).contains(&n),
            "the copy holds {n} rows"
        );
        assert_eq!(tables(&c), source_tables);
        let journal: String = c
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal, "delete");
        assert_eq!(count(&conn), before + more);
    }

    #[test]
    fn a_missing_source_is_an_error_and_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let copy = dir.path().join("copy.db");
        let err = snapshot(&dir.path().join("absent.db"), &copy).unwrap_err();
        assert!(err.starts_with("snapshot: "), "{err}");
        assert!(!copy.exists());
        assert!(!dir.path().join("absent.db").exists());
    }

    #[test]
    fn a_corrupt_source_is_an_error_and_leaves_no_copy() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("pos.db");
        std::fs::write(&db, vec![0x55u8; 8192]).unwrap();
        let copy = dir.path().join("copy.db");
        assert!(snapshot(&db, &copy).is_err());
        assert!(!copy.exists());
    }

    #[test]
    fn an_existing_destination_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("pos.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE probe (id INTEGER PRIMARY KEY, v BLOB);")
            .unwrap();
        let copy = dir.path().join("copy.db");
        std::fs::write(&copy, b"stale").unwrap();
        snapshot(&db, &copy).unwrap();
        assert_eq!(count(&Connection::open(&copy).unwrap()), 0);
    }
}
