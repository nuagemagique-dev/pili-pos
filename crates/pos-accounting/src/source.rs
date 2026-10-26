//! The source: a copy of `pos.db`, opened read-only.
//!
//! The file is opened with `mode=ro&immutable=1`: SQLite neither writes to it nor
//! creates a journal or a WAL beside it, and ignores a WAL that is there. The tool is
//! meant for a copy (the SPF export folder, a decrypted backup), never the live file.
//!
//! Only the signed archive is read: `fiscal_events` rows that have an FDM answer
//! (labels N, I, F, R), `z_reports` and `booking_periods`. Never `orders`,
//! `order_lines`, `payments` or any other unsigned state.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{AccountingError, Result};
use crate::parse::RawEvent;

/// An opened source.
#[derive(Debug)]
pub struct Source {
    conn: Connection,
    /// The `pos.db` file that was read.
    pub db_path: PathBuf,
    /// SHA-256 of that file, lowercase hex.
    pub sha256: String,
    /// True when `--db` was an export folder whose `MANIFEST.sha256` was checked.
    pub manifest_checked: bool,
}

/// A booking period closed by a real Z.
#[derive(Debug, Clone)]
pub struct ClosedPeriod {
    pub period_id: String,
    pub booking_date: String,
    pub z_report_no: i64,
    pub z_event: RawEvent,
}

fn io_err(path: &Path, e: &std::io::Error) -> AccountingError {
    AccountingError::DbUnreadable(format!("{}: {e}", path.display()))
}

/// SHA-256 of a file as lowercase hex, read as a stream.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|e| io_err(path, &e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| io_err(path, &e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Split a `sha256sum` line into hash and name: text mode is `hash  name`, binary
/// mode (`--binary`) is `hash *name`.
fn split_manifest_line(line: &str) -> Option<(&str, &str)> {
    if let Some((hash, name)) = line.split_once("  ") {
        return Some((hash, name));
    }
    line.split_once(" *")
}

/// Check every file listed in `MANIFEST.sha256` (`sha256sum` format) of `dir`. The
/// manifest must cover `pos.db`: a manifest that lists nothing, or other files only,
/// proves nothing about the database that is read.
fn verify_manifest(dir: &Path, manifest: &Path) -> Result<()> {
    let text = std::fs::read_to_string(manifest).map_err(|e| io_err(manifest, &e))?;
    let mut covers_db = false;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Some((hash, name)) = split_manifest_line(line) else {
            return Err(AccountingError::ManifestMismatch(format!(
                "unreadable line: {line}"
            )));
        };
        // `sha256sum ./pos.db` prints the name with its `./`.
        let name = name.trim_start_matches('*');
        let name = name.strip_prefix("./").unwrap_or(name);
        if name == "pos.db" {
            covers_db = true;
        }
        let relative = Path::new(name);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(AccountingError::ManifestMismatch(format!(
                "path outside the folder: {name}"
            )));
        }
        let path = dir.join(relative);
        if !path.is_file() {
            return Err(AccountingError::ManifestMismatch(format!(
                "{name} is missing"
            )));
        }
        if sha256_file(&path)? != hash.to_ascii_lowercase() {
            return Err(AccountingError::ManifestMismatch(format!(
                "{name} has changed"
            )));
        }
    }
    if !covers_db {
        return Err(AccountingError::ManifestMismatch(
            "the manifest does not list pos.db".to_string(),
        ));
    }
    Ok(())
}

/// Percent-encode a path for a SQLite `file:` URI.
fn uri(path: &Path) -> String {
    let mut out = String::from("file:");
    for b in path.to_string_lossy().bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(char::from(b));
            }
            _ => {
                use std::fmt::Write as _;
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out.push_str("?mode=ro&immutable=1");
    out
}

fn sql<T>(r: rusqlite::Result<T>, what: &str) -> Result<T> {
    r.map_err(|e| AccountingError::DbUnreadable(format!("{what}: {e}")))
}

impl Source {
    /// Open `--db`: a `pos.db` file, or an SPF export folder holding one.
    pub fn open(db: &Path) -> Result<Self> {
        let (db_path, manifest_checked) = if db.is_dir() {
            let manifest = db.join("MANIFEST.sha256");
            let checked = manifest.is_file();
            if checked {
                verify_manifest(db, &manifest)?;
            }
            (db.join("pos.db"), checked)
        } else {
            (db.to_path_buf(), false)
        };
        if !db_path.is_file() {
            return Err(AccountingError::DbUnreadable(format!(
                "{} is not a file",
                db_path.display()
            )));
        }
        // A WAL file with content beside the file means a copy of a live database that
        // misses what is still in the WAL: immutable mode would silently ignore it.
        let mut wal = db_path.as_os_str().to_os_string();
        wal.push("-wal");
        if std::fs::metadata(&wal).is_ok_and(|m| m.len() > 0) {
            return Err(AccountingError::DbUnreadable(format!(
                "{} has a non-empty WAL file: this is a copy of a live database. \
                 Use the export folder of the till or a backup, not the live files",
                db_path.display()
            )));
        }
        let sha256 = sha256_file(&db_path)?;
        let conn = Connection::open_with_flags(
            uri(&db_path),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_URI
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| AccountingError::DbUnreadable(format!("{}: {e}", db_path.display())))?;
        let source = Self {
            conn,
            db_path,
            sha256,
            manifest_checked,
        };
        source.check_archive()?;
        Ok(source)
    }

    /// The file must be a POS archive that has migration 4 (append-only archive).
    fn check_archive(&self) -> Result<()> {
        let not_pili = |what: &str| AccountingError::NotAPiliDb(what.to_string());
        let unreadable = |e: rusqlite::Error| {
            not_pili(&format!(
                "the file cannot be read as a SQLite database: {e}"
            ))
        };
        let tables: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table'")
                .map_err(unreadable)?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(unreadable)?;
            rows.collect::<std::result::Result<_, _>>()
                .map_err(unreadable)?
        };
        for needed in [
            "_migrations_applied",
            "fiscal_events",
            "z_reports",
            "booking_periods",
        ] {
            if !tables.iter().any(|t| t == needed) {
                return Err(not_pili(&format!("table {needed} is missing")));
            }
        }
        let four: Option<i64> = sql(
            self.conn
                .query_row(
                    "SELECT version FROM _migrations_applied WHERE version = 4",
                    [],
                    |r| r.get(0),
                )
                .optional(),
            "read the migrations",
        )?;
        if four.is_none() {
            return Err(not_pili(
                "migration 4 (append-only archive) is not applied: the database is too old",
            ));
        }
        Ok(())
    }

    /// The booking periods closed by a real Z whose booking date is in `from..=to`
    /// (`YYYY-MM-DD`), by date then Z number. A training Z has no `z_reports` row.
    pub fn closed_periods(&self, from: &str, to: &str) -> Result<Vec<ClosedPeriod>> {
        let mut stmt = sql(
            self.conn.prepare(
                "SELECT p.id, p.booking_date, z.sequence_number, e.id, e.event_data_json,
                        e.fdm_response_json, e.fdm_datetime
                 FROM z_reports z
                 JOIN fiscal_events e ON e.id = z.fiscal_event_id AND e.event_type = 'R'
                 JOIN booking_periods p ON p.closing_event_id = e.id
                 WHERE p.closed_at IS NOT NULL AND e.fdm_response_json IS NOT NULL
                   AND p.booking_date BETWEEN ?1 AND ?2
                 ORDER BY p.booking_date, z.sequence_number",
            ),
            "prepare the periods",
        )?;
        let rows = sql(
            stmt.query_map([from, to], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            }),
            "read the periods",
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (period_id, booking_date, z_report_no, event_id, request, response, fdm) =
                sql(row, "read a period")?;
            out.push(ClosedPeriod {
                period_id,
                booking_date,
                z_report_no,
                z_event: raw_event(event_id, "R", &request, Some(&response), fdm)?,
            });
        }
        Ok(out)
    }

    /// Booking dates of the periods in the range that no Z closed yet.
    pub fn open_periods(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let mut stmt = sql(
            self.conn.prepare(
                "SELECT booking_date FROM booking_periods
                 WHERE closed_at IS NULL AND booking_date BETWEEN ?1 AND ?2
                 ORDER BY booking_date",
            ),
            "prepare the open periods",
        )?;
        let rows = sql(
            stmt.query_map([from, to], |r| r.get::<_, String>(0)),
            "read the open periods",
        )?;
        rows.map(|r| sql(r, "read an open period")).collect()
    }

    /// The signed N, I and F events of a booking period that have an FDM answer, in
    /// archive order. Training (T), copies (C), pro forma (P), social (S) and the reports
    /// are never listed.
    pub fn events(&self, period_id: &str) -> Result<Vec<RawEvent>> {
        let mut stmt = sql(
            self.conn.prepare(
                "SELECT id, event_type, event_data_json, fdm_response_json, fdm_datetime, event_label
                 FROM fiscal_events
                 WHERE json_extract(event_data_json, '$.bookingPeriodId') = ?1
                   AND event_type IN ('N', 'I', 'F')
                   AND fdm_response_json IS NOT NULL
                 ORDER BY id",
            ),
            "prepare the events",
        )?;
        let rows = sql(
            stmt.query_map([period_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                ))
            }),
            "read the events",
        )?;
        let mut out = Vec::new();
        for row in rows {
            let (id, label, request, response, fdm, signed_label) = sql(row, "read an event")?;
            // An event the FDM signed under another label (training) is not a sale.
            if signed_label.as_deref().is_some_and(|l| l != label) {
                continue;
            }
            out.push(raw_event(id, &label, &request, Some(&response), fdm)?);
        }
        Ok(out)
    }

    /// The archived signed event an `fdmRef` names (a refund's ticket, an invoice line).
    pub fn find_by_fdm_ref(&self, fdm_ref: &Value) -> Result<Option<RawEvent>> {
        let found = sql(
            self.conn
                .query_row(
                    "SELECT id, event_type, event_data_json, fdm_response_json, fdm_datetime
                     FROM fiscal_events
                     WHERE fdm_id = ?1 AND fdm_datetime = ?2 AND event_label = ?3
                       AND json_extract(fdm_response_json, '$.fdmRef.eventCounter') = ?4
                       AND json_extract(fdm_response_json, '$.fdmRef.totalCounter') = ?5
                     LIMIT 1",
                    rusqlite::params![
                        fdm_ref["fdmId"].as_str(),
                        fdm_ref["fdmDateTime"].as_str(),
                        fdm_ref["eventLabel"].as_str(),
                        fdm_ref["eventCounter"].as_i64(),
                        fdm_ref["totalCounter"].as_i64(),
                    ],
                    |r| {
                        Ok((
                            r.get::<_, i64>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, Option<String>>(3)?,
                            r.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .optional(),
            "read a referenced ticket",
        )?;
        found
            .map(|(id, label, request, response, fdm)| {
                raw_event(id, &label, &request, response.as_deref(), fdm)
            })
            .transpose()
    }
}

fn raw_event(
    id: i64,
    label: &str,
    request: &str,
    response: Option<&str>,
    fdm_datetime: Option<String>,
) -> Result<RawEvent> {
    let bad = |detail: String| AccountingError::EventUnreadable {
        event_id: id,
        detail,
    };
    let request: Value =
        serde_json::from_str(request).map_err(|e| bad(format!("request JSON: {e}")))?;
    let response: Value = match response {
        Some(r) => serde_json::from_str(r).map_err(|e| bad(format!("response JSON: {e}")))?,
        None => Value::Null,
    };
    Ok(RawEvent {
        id,
        label: label.to_string(),
        request,
        response,
        fdm_datetime,
    })
}
