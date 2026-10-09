//! Local outbox: preparation, upload, retry and purge of the exports (card
//! X1-T5, decisions X1D6 to X1D8 and X1D10).
//!
//! # Layout under `exports_dir` (the engine passes `<data_dir>/exports`)
//!
//! ```text
//! cursor.json        {lastZReportNo}: the last Z report handled
//! status.json        {lastError, lastErrorAt, lastUploadedId, lastUploadedAt, dropped}
//! pseudonym.key      per-installation key of the statistics (never exported)
//! .tmp/              work area; emptied at start-up and before each preparation
//! <export_id>/       pos-<id>.db.age, stats-<id>.sqlite, manifest-<id>.json, state.json
//! ```
//!
//! `state.json` holds `{exportId, createdAt, zReportNo, bookingDate, uploaded,
//! attempts, nextAttemptAt, lastError}`. Every JSON file is written atomically
//! (temporary file in the same directory, fsync, rename). Times are RFC 3339
//! UTC strings (`2026-10-08T22:15:03Z`). Every function that needs the time
//! takes `now`, so tests inject it.
//!
//! # Flow
//!
//! - [`prepare_due`] (blocking): for every real Z newer than the cursor, build
//!   the export in `.tmp/<id>` and rename it into place. The plaintext
//!   snapshot exists only in `.tmp` and is deleted after encryption; a failure
//!   at any step deletes `.tmp/<id>` and keeps the cursor, so the next pass
//!   retries.
//! - [`upload_due`] (async): PUT the files of every incomplete export that is
//!   due, in the order backup, stats, manifest. The first failure of an
//!   export sets its next attempt (1, 2, 4 ... minutes, capped at 60).
//! - [`retry_now`], [`retention`], [`status`].
//!
//! The token is read from its file at each pass and is never written to a
//! file or a log.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Duration, NaiveDate, SecondsFormat, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

use crate::manifest::{self, Manifest, ManifestFile};
use crate::settings::ExportSettings;
use crate::status::ExportStatus;
use crate::upload::Uploader;
use crate::zref::ZRef;
use crate::{crypt, naming, pseudonym, snapshot, stats, zref};

const CURSOR_FILE: &str = "cursor.json";
const STATUS_FILE: &str = "status.json";
const STATE_FILE: &str = "state.json";
const TMP_DIR: &str = ".tmp";
/// Name of the plaintext snapshot inside `.tmp/<id>`.
const WORK_DB: &str = "pos.db";
/// Longest wait between two attempts of one export.
const MAX_BACKOFF_MINUTES: i64 = 60;

const OCTET_STREAM: &str = "application/octet-stream";
const JSON: &str = "application/json";

/// Serialises the read-modify-write of `status.json` inside the process.
static STATUS_LOCK: Mutex<()> = Mutex::new(());

/// What the outbox needs to work. Cheap to copy; borrows everything.
#[derive(Debug, Clone, Copy)]
pub struct ExportEnv<'a> {
    /// The live database (opened read-only).
    pub db_path: &'a Path,
    /// `<data_dir>/exports`.
    pub exports_dir: &'a Path,
    pub settings: &'a ExportSettings,
    pub pos_id: &'a str,
    pub terminal_id: &'a str,
    pub pos_sw_version: &'a str,
}

/// Result of one upload pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PassReport {
    /// Exports that became complete during the pass.
    pub uploaded: i64,
    /// Exports whose upload failed during the pass.
    pub failed: i64,
}

// ── Files ──────────────────────────────────────────────────────────

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CursorFile {
    last_z_report_no: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StatusFile {
    last_error: Option<String>,
    last_error_at: Option<String>,
    last_uploaded_id: Option<String>,
    last_uploaded_at: Option<String>,
    dropped: i64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct ExportState {
    export_id: String,
    created_at: String,
    z_report_no: i64,
    booking_date: String,
    uploaded: Vec<String>,
    attempts: i64,
    next_attempt_at: Option<String>,
    last_error: Option<String>,
}

impl ExportState {
    /// Complete when the three files are uploaded (the manifest last).
    fn is_complete(&self, id: &str) -> bool {
        [
            naming::backup_name(id),
            naming::stats_name(id),
            naming::manifest_name(id),
        ]
        .iter()
        .all(|name| self.uploaded.contains(name))
    }
}

fn stamp(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// Write `value` as JSON, atomically: a temporary file next to `path`, synced,
/// then renamed.
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let mut json = serde_json::to_string_pretty(value)
        .map_err(|e| format!("cannot encode {}: {e}", path.display()))?;
    json.push('\n');
    let mut tmp_name = path.as_os_str().to_os_string();
    tmp_name.push(".tmp");
    let tmp = PathBuf::from(tmp_name);
    let result = (|| {
        let mut f =
            File::create(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
        f.write_all(json.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        f.sync_all()
            .map_err(|e| format!("cannot sync {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("cannot rename {}: {e}", tmp.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Make a rename durable (best effort: not every platform can sync a directory).
fn sync_dir(dir: &Path) {
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let raw = std::fs::read(path).ok()?;
    serde_json::from_slice(&raw).ok()
}

fn read_cursor(exports_dir: &Path) -> Option<i64> {
    let path = exports_dir.join(CURSOR_FILE);
    if !path.exists() {
        return None;
    }
    match read_json::<CursorFile>(&path) {
        Some(c) => Some(c.last_z_report_no),
        None => {
            tracing::warn!("export: {CURSOR_FILE} is unreadable, starting from the latest Z");
            None
        }
    }
}

fn write_cursor(exports_dir: &Path, last_z_report_no: i64) -> Result<(), String> {
    write_json(
        &exports_dir.join(CURSOR_FILE),
        &CursorFile { last_z_report_no },
    )
}

fn read_status_file(exports_dir: &Path) -> StatusFile {
    read_json(&exports_dir.join(STATUS_FILE)).unwrap_or_default()
}

/// Read-modify-write of `status.json`.
fn update_status(exports_dir: &Path, change: impl FnOnce(&mut StatusFile)) -> Result<(), String> {
    let _guard = STATUS_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut file = read_status_file(exports_dir);
    change(&mut file);
    std::fs::create_dir_all(exports_dir)
        .map_err(|e| format!("cannot create {}: {e}", exports_dir.display()))?;
    write_json(&exports_dir.join(STATUS_FILE), &file)
}

/// Keep an error in `status.json`; a failure to do so is only logged.
fn record_error(exports_dir: &Path, message: &str, now: DateTime<Utc>) {
    let message = message.to_string();
    if let Err(e) = update_status(exports_dir, |s| {
        s.last_error = Some(message);
        s.last_error_at = Some(stamp(now));
    }) {
        tracing::warn!("export: cannot record the error: {e}");
    }
}

/// `YYYYMMDD-z000000`.
fn is_export_id(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 16
        && b[..8].iter().all(u8::is_ascii_digit)
        && b[8] == b'-'
        && b[9] == b'z'
        && b[10..].iter().all(u8::is_ascii_digit)
}

/// The export directories, by id (that is by booking date, then Z number).
fn list_exports(exports_dir: &Path) -> Result<Vec<String>, String> {
    let entries = match std::fs::read_dir(exports_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot read {}: {e}", exports_dir.display())),
    };
    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", exports_dir.display()))?;
        // `file_type` does not follow symbolic links.
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if let Some(name) = entry.file_name().to_str()
            && is_dir
            && is_export_id(name)
        {
            ids.push(name.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

fn load_state(exports_dir: &Path, id: &str) -> Option<ExportState> {
    read_json(&exports_dir.join(id).join(STATE_FILE))
}

fn save_state(exports_dir: &Path, id: &str, state: &ExportState) -> Result<(), String> {
    write_json(&exports_dir.join(id).join(STATE_FILE), state)
}

/// Minutes to wait after `attempts` failures: 1, 2, 4 ... capped at 60.
fn backoff_minutes(attempts: i64) -> i64 {
    let exp = (attempts - 1).clamp(0, 10);
    (1i64 << exp).min(MAX_BACKOFF_MINUTES)
}

// ── Start-up ───────────────────────────────────────────────────────

/// Empty `.tmp` (created when absent). The engine calls it at start-up: a
/// crash during a preparation leaves a plaintext snapshot there.
pub fn clean_tmp(exports_dir: &Path) -> Result<(), String> {
    let tmp = exports_dir.join(TMP_DIR);
    std::fs::create_dir_all(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    let entries =
        std::fs::read_dir(&tmp).map_err(|e| format!("cannot read {}: {e}", tmp.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read {}: {e}", tmp.display()))?;
        let path = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let removed = if is_dir {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        removed.map_err(|e| format!("cannot remove {}: {e}", path.display()))?;
    }
    Ok(())
}

// ── Preparation ────────────────────────────────────────────────────

/// Prepare an export for every real Z newer than the cursor; return their ids.
///
/// Blocking: the engine runs it in `spawn_blocking`. When the cursor file is
/// absent (the export was just enabled) the cursor starts just before the
/// latest Z, so only that Z is exported, no history (X1D6). A Z whose booking
/// date is older than `now - keep_days` is skipped and the cursor moves past
/// it. A failure leaves nothing partial, records `prepare failed: <reason>` in
/// `status.json`, keeps the cursor and returns the error.
pub fn prepare_due(env: &ExportEnv, now: DateTime<Utc>) -> Result<Vec<String>, String> {
    match prepare_all(env, now) {
        Ok(ids) => Ok(ids),
        Err(e) => {
            let message = format!("prepare failed: {e}");
            tracing::warn!("export: {message}");
            record_error(env.exports_dir, &message, now);
            Err(message)
        }
    }
}

fn prepare_all(env: &ExportEnv, now: DateTime<Utc>) -> Result<Vec<String>, String> {
    std::fs::create_dir_all(env.exports_dir)
        .map_err(|e| format!("cannot create {}: {e}", env.exports_dir.display()))?;
    clean_tmp(env.exports_dir)?;
    let conn = Connection::open_with_flags(
        env.db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| format!("cannot open {}: {e}", env.db_path.display()))?;

    let cursor = match read_cursor(env.exports_dir) {
        Some(cursor) => cursor,
        None => {
            let first = zref::latest_real_z(&conn)?
                .map(|z| (z.z_report_no - 1).max(0))
                .unwrap_or(0);
            write_cursor(env.exports_dir, first)?;
            first
        }
    };
    let oldest_kept = (now - Duration::days(env.settings.keep_days)).date_naive();
    let mut prepared = Vec::new();
    for z in zref::real_zs_after(&conn, Some(cursor))? {
        let id = naming::export_id(&z.booking_date, z.z_report_no)?;
        if env.exports_dir.join(&id).exists() {
            // Prepared before a crash that kept the cursor back.
            write_cursor(env.exports_dir, z.z_report_no)?;
            continue;
        }
        let booking_date = NaiveDate::parse_from_str(&z.booking_date, "%Y-%m-%d")
            .map_err(|e| format!("booking date {:?}: {e}", z.booking_date))?;
        if booking_date < oldest_kept {
            tracing::info!(
                "export: Z {} ({}) is older than {} days, not exported",
                z.z_report_no,
                z.booking_date,
                env.settings.keep_days
            );
            write_cursor(env.exports_dir, z.z_report_no)?;
            continue;
        }
        build_export(env, &z, &id, now)?;
        write_cursor(env.exports_dir, z.z_report_no)?;
        tracing::info!("export: {id} prepared");
        prepared.push(id);
    }
    Ok(prepared)
}

/// Build `.tmp/<id>` and move it to `<id>`; remove `.tmp/<id>` on any failure.
fn build_export(env: &ExportEnv, z: &ZRef, id: &str, now: DateTime<Utc>) -> Result<(), String> {
    let work = env.exports_dir.join(TMP_DIR).join(id);
    let result = build_in(env, z, id, now, &work).and_then(|()| {
        let dest = env.exports_dir.join(id);
        std::fs::rename(&work, &dest)
            .map_err(|e| format!("cannot move {} into place: {e}", work.display()))?;
        sync_dir(env.exports_dir);
        Ok(())
    });
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&work);
    }
    result
}

fn build_in(
    env: &ExportEnv,
    z: &ZRef,
    id: &str,
    now: DateTime<Utc>,
    work: &Path,
) -> Result<(), String> {
    if work.exists() {
        std::fs::remove_dir_all(work)
            .map_err(|e| format!("cannot clear {}: {e}", work.display()))?;
    }
    std::fs::create_dir_all(work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;

    let plain = work.join(WORK_DB);
    snapshot::snapshot(env.db_path, &plain)?;

    let key = pseudonym::load_or_create_key(env.exports_dir)?;
    let stats_path = work.join(naming::stats_name(id));
    {
        let copy = Connection::open_with_flags(
            &plain,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("cannot open the snapshot: {e}"))?;
        let meta = stats::StatsMeta {
            export_id: id.to_string(),
            pos_id: env.pos_id.to_string(),
            terminal_id: env.terminal_id.to_string(),
            pos_sw_version: env.pos_sw_version.to_string(),
            generated_at: stamp(now),
        };
        stats::build_stats(&copy, z, &meta, &key, &stats_path)?;
    }

    let backup_path = work.join(naming::backup_name(id));
    crypt::encrypt_file(&plain, &backup_path, &env.settings.recipients)?;
    remove_plaintext(&plain)?;

    let mut files = Vec::new();
    for path in [&backup_path, &stats_path] {
        files.push(manifest_entry(path)?);
    }
    let manifest = Manifest {
        export_id: id.to_string(),
        created_at: stamp(now),
        pos_id: env.pos_id.to_string(),
        terminal_id: env.terminal_id.to_string(),
        pos_sw_version: env.pos_sw_version.to_string(),
        z_report_no: z.z_report_no,
        booking_date: z.booking_date.clone(),
        stats_schema_version: stats::STATS_SCHEMA_VERSION,
        recipients: env.settings.recipient_strings(),
        files,
        ..Manifest::default()
    };
    manifest.write(&work.join(naming::manifest_name(id)))?;

    let state = ExportState {
        export_id: id.to_string(),
        created_at: stamp(now),
        z_report_no: z.z_report_no,
        booking_date: z.booking_date.clone(),
        ..ExportState::default()
    };
    write_json(&work.join(STATE_FILE), &state)?;
    sync_dir(work);
    Ok(())
}

fn manifest_entry(path: &Path) -> Result<ManifestFile, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("{} has no file name", path.display()))?
        .to_string();
    let bytes = std::fs::metadata(path)
        .map_err(|e| format!("cannot stat {}: {e}", path.display()))?
        .len();
    Ok(ManifestFile {
        name,
        bytes: i64::try_from(bytes).map_err(|_| "file size out of range".to_string())?,
        sha256: manifest::sha256_file(path)?,
    })
}

/// Delete the plaintext snapshot and the files SQLite may keep beside it.
fn remove_plaintext(plain: &Path) -> Result<(), String> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = plain.as_os_str().to_os_string();
        name.push(suffix);
        match std::fs::remove_file(&name) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot remove {name:?}: {e}")),
        }
    }
    Ok(())
}

// ── Upload ─────────────────────────────────────────────────────────

/// Run a blocking closure without stalling the executor.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| format!("background task failed: {e}"))
}

/// One upload pass: PUT the missing files of every incomplete export that is
/// due, in id order, each export as backup, stats, manifest.
///
/// The first failure of an export counts an attempt, schedules the next one
/// ([`backoff_minutes`]) and moves on to the next export. An unreadable token
/// is the error of the whole pass. Nothing here panics or blocks a sale.
pub async fn upload_due(
    env: &ExportEnv<'_>,
    uploader: &Uploader,
    now: DateTime<Utc>,
) -> PassReport {
    let mut report = PassReport::default();
    let exports_dir = env.exports_dir.to_path_buf();
    let token = match env.settings.read_token() {
        Ok(token) => token,
        Err(e) => {
            tracing::warn!("export: {e}");
            record_error(&exports_dir, &e, now);
            return report;
        }
    };
    let ids = {
        let dir = exports_dir.clone();
        match blocking(move || list_exports(&dir)).await {
            Ok(Ok(ids)) => ids,
            Ok(Err(e)) | Err(e) => {
                record_error(&exports_dir, &e, now);
                return report;
            }
        }
    };
    for id in ids {
        let dir = exports_dir.clone();
        let wanted = id.clone();
        let loaded = blocking(move || load_state(&dir, &wanted)).await;
        let Ok(Some(mut state)) = loaded else {
            tracing::warn!("export: {id}: state.json is unreadable, skipped");
            continue;
        };
        if state.is_complete(&id) {
            continue;
        }
        if let Some(at) = state.next_attempt_at.as_deref().and_then(parse_time)
            && at > now
        {
            continue;
        }
        match upload_export(&exports_dir, &id, &mut state, uploader, &token).await {
            Ok(()) => {
                state.next_attempt_at = None;
                state.last_error = None;
                if let Err(e) = save_state(&exports_dir, &id, &state) {
                    tracing::warn!("export: {id}: {e}");
                }
                report.uploaded += 1;
                tracing::info!("export: {id} uploaded");
                let clear_error = report.failed == 0;
                let done = id.clone();
                if let Err(e) = update_status(&exports_dir, |s| {
                    s.last_uploaded_id = Some(done);
                    s.last_uploaded_at = Some(stamp(now));
                    if clear_error {
                        s.last_error = None;
                        s.last_error_at = None;
                    }
                }) {
                    tracing::warn!("export: {id}: {e}");
                }
            }
            Err(error) => {
                let error = error.replace(&token, "***");
                state.attempts += 1;
                state.next_attempt_at = Some(stamp(
                    now + Duration::minutes(backoff_minutes(state.attempts)),
                ));
                state.last_error = Some(error.clone());
                if let Err(e) = save_state(&exports_dir, &id, &state) {
                    tracing::warn!("export: {id}: {e}");
                }
                tracing::warn!(
                    "export: {id}: upload failed (attempt {}): {error}",
                    state.attempts
                );
                record_error(&exports_dir, &error, now);
                report.failed += 1;
            }
        }
    }
    report
}

/// Upload the files of one export not yet in `state.uploaded`, recording each
/// success in `state.json`. The error is the text for the operator.
async fn upload_export(
    exports_dir: &Path,
    id: &str,
    state: &mut ExportState,
    uploader: &Uploader,
    token: &str,
) -> Result<(), String> {
    let dir = exports_dir.join(id);
    let manifest_name = naming::manifest_name(id);
    let manifest_path = dir.join(&manifest_name);
    let manifest = {
        let path = manifest_path.clone();
        blocking(move || Manifest::read(&path)).await??
    };
    let plan = [
        (naming::backup_name(id), OCTET_STREAM),
        (naming::stats_name(id), OCTET_STREAM),
        (manifest_name.clone(), JSON),
    ];
    for (name, content_type) in plan {
        if state.uploaded.contains(&name) {
            continue;
        }
        let path = dir.join(&name);
        let sha256 = if name == manifest_name {
            let path = path.clone();
            blocking(move || manifest::sha256_file(&path)).await??
        } else {
            manifest
                .files
                .iter()
                .find(|f| f.name == name)
                .map(|f| f.sha256.clone())
                .ok_or_else(|| format!("{name} is not listed in the manifest"))?
        };
        uploader
            .put_file(id, &name, &path, &sha256, token, content_type)
            .await
            .map_err(|e| e.to_string())?;
        state.uploaded.push(name);
        if let Err(e) = save_state(exports_dir, id, state) {
            // The next pass uploads the file again: a PUT is idempotent.
            tracing::warn!("export: {id}: {e}");
        }
    }
    Ok(())
}

// ── Retry, retention, status ───────────────────────────────────────

/// Make every incomplete export due now; return their number.
pub fn retry_now(env: &ExportEnv, now: DateTime<Utc>) -> Result<i64, String> {
    let mut count = 0;
    for id in list_exports(env.exports_dir)? {
        let Some(mut state) = load_state(env.exports_dir, &id) else {
            continue;
        };
        if state.is_complete(&id) {
            continue;
        }
        state.next_attempt_at = Some(stamp(now));
        save_state(env.exports_dir, &id, &state)?;
        count += 1;
    }
    Ok(count)
}

/// Delete every export created before `now - keep_days`, sent or not; return
/// the number deleted. Each unsent one is counted in `dropped` and logged.
/// Only export directories of `exports_dir` are touched.
pub fn retention(env: &ExportEnv, now: DateTime<Utc>) -> Result<i64, String> {
    let limit = now - Duration::days(env.settings.keep_days);
    let mut deleted = 0;
    let mut dropped = 0;
    for id in list_exports(env.exports_dir)? {
        let dir = env.exports_dir.join(&id);
        let state = load_state(env.exports_dir, &id);
        let created = state
            .as_ref()
            .and_then(|s| parse_time(&s.created_at))
            .or_else(|| {
                // No readable state: the age of the directory decides.
                std::fs::metadata(&dir)
                    .and_then(|m| m.modified())
                    .ok()
                    .map(DateTime::<Utc>::from)
            });
        let Some(created) = created else { continue };
        if created >= limit {
            continue;
        }
        let incomplete = state.as_ref().is_none_or(|s| !s.is_complete(&id));
        std::fs::remove_dir_all(&dir)
            .map_err(|e| format!("cannot delete {}: {e}", dir.display()))?;
        deleted += 1;
        if incomplete {
            dropped += 1;
            tracing::warn!(
                "export: {id} was never completely uploaded and is deleted after {} days",
                env.settings.keep_days
            );
        }
    }
    if dropped > 0 {
        update_status(env.exports_dir, |s| {
            s.dropped = s.dropped.saturating_add(dropped);
        })?;
    }
    Ok(deleted)
}

/// The status shown to the operator. Missing files give empty values, never
/// an error. `problem` is the configuration error, if any.
pub fn status(
    exports_dir: &Path,
    settings: Option<&ExportSettings>,
    problem: Option<String>,
) -> ExportStatus {
    let file = read_status_file(exports_dir);
    let ids = list_exports(exports_dir).unwrap_or_default();
    let mut pending = 0;
    let mut next: Option<(DateTime<Utc>, String)> = None;
    let mut last_prepared: Option<(String, String)> = None;
    for id in &ids {
        let Some(state) = load_state(exports_dir, id) else {
            continue;
        };
        last_prepared = Some((id.clone(), state.created_at.clone()));
        if state.is_complete(id) {
            continue;
        }
        pending += 1;
        if let Some(raw) = &state.next_attempt_at
            && let Some(at) = parse_time(raw)
            && next.as_ref().is_none_or(|(best, _)| at < *best)
        {
            next = Some((at, raw.clone()));
        }
    }
    let newest = ids.last().cloned();
    let (last_export_id, last_prepared_at) = match last_prepared {
        Some((id, at)) => (Some(id), Some(at).filter(|a| !a.is_empty())),
        None => (newest, None),
    };
    ExportStatus {
        enabled: settings.is_some() && problem.is_none(),
        problem,
        endpoint: settings.map(ExportSettings::endpoint),
        keep_days: settings.map_or(0, |s| s.keep_days),
        last_export_id,
        last_prepared_at,
        last_uploaded_id: file.last_uploaded_id,
        last_uploaded_at: file.last_uploaded_at,
        pending,
        next_attempt_at: next.map(|(_, raw)| raw),
        last_error: file.last_error,
        last_error_at: file.last_error_at,
        kept: i64::try_from(ids.len()).unwrap_or(i64::MAX),
        dropped: file.dropped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backoff_doubles_and_stops_at_an_hour() {
        let minutes: Vec<i64> = (1..=9).map(backoff_minutes).collect();
        assert_eq!(minutes, vec![1, 2, 4, 8, 16, 32, 60, 60, 60]);
        assert_eq!(backoff_minutes(0), 1);
        assert_eq!(backoff_minutes(i64::MAX), 60);
    }

    #[test]
    fn export_ids_are_recognised() {
        assert!(is_export_id("20261008-z000042"));
        for bad in [
            ".tmp",
            "20261008-z42",
            "2026100-z0000042",
            "pseudonym.key",
            "",
        ] {
            assert!(!is_export_id(bad), "{bad}");
        }
    }

    #[test]
    fn times_are_whole_seconds_in_utc() {
        let t = DateTime::parse_from_rfc3339("2026-10-08T22:15:03.789+02:00")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(stamp(t), "2026-10-08T20:15:03Z");
        assert_eq!(
            parse_time("2026-10-08T20:15:03Z"),
            Some(t.with_nanosecond_zero())
        );
    }

    trait Zero {
        fn with_nanosecond_zero(self) -> Self;
    }
    impl Zero for DateTime<Utc> {
        fn with_nanosecond_zero(self) -> Self {
            use chrono::Timelike;
            self.with_nanosecond(0).unwrap()
        }
    }
}
