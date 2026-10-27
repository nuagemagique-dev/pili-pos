//! The upload passes of the outbox against the mock receiver (card X1-T5,
//! decision X1D8): order, hashes, backoff, unauthorized, retry now, retention
//! of sent and unsent exports, and the token kept out of files and logs.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

mod common;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use common::{ExportKit, Fixture, add_empty_z, export_kit, fixture_db};
use pos_export::manifest::{Manifest, sha256_file};
use pos_export::outbox::{self, ExportEnv, PassReport};
use pos_export::status::ExportStatus;
use pos_export::{Uploader, naming};
use pos_export_mock::Receiver;
use serde_json::Value;

const ID_41: &str = "20261003-z000041";
const ID_42: &str = "20261004-z000042";

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn t0() -> DateTime<Utc> {
    t("2026-10-05T01:00:00Z")
}

fn at(base: DateTime<Utc>, seconds: i64) -> DateTime<Utc> {
    base + Duration::seconds(seconds)
}

struct Rig {
    _dir: tempfile::TempDir,
    fx: Fixture,
    kit: ExportKit,
    exports: PathBuf,
    receiver: Receiver,
    uploader: Uploader,
}

impl Rig {
    fn new(keep_days: i64) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let fx = fixture_db(&dir.path().join("pos.db"));
        let mut kit = export_kit(dir.path(), "http://127.0.0.1:1", keep_days);
        let receiver = Receiver::start(&kit.token);
        kit.settings.url = receiver.url();
        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(20))
            .build()
            .unwrap();
        let uploader = Uploader::new(client, &kit.settings.url);
        let exports = dir.path().join("exports");
        Rig {
            _dir: dir,
            fx,
            kit,
            exports,
            receiver,
            uploader,
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

    fn prepare(&self, now: DateTime<Utc>) -> Vec<String> {
        outbox::prepare_due(&self.env(), now).unwrap()
    }

    async fn upload(&self, now: DateTime<Utc>) -> PassReport {
        outbox::upload_due(&self.env(), &self.uploader, now).await
    }

    fn set_cursor(&self, no: i64) {
        std::fs::create_dir_all(&self.exports).unwrap();
        std::fs::write(
            self.exports.join("cursor.json"),
            format!("{{\"lastZReportNo\": {no}}}"),
        )
        .unwrap();
    }

    fn status(&self) -> ExportStatus {
        outbox::status(&self.exports, Some(&self.kit.settings), None)
    }

    fn state(&self, id: &str) -> Value {
        json(&self.exports.join(id).join("state.json"))
    }

    fn path(&self, id: &str, name: String) -> PathBuf {
        self.exports.join(id).join(name)
    }

    /// The token appears in no file under the exports directory.
    fn assert_token_is_nowhere(&self) {
        let mut files = Vec::new();
        all_files(&self.exports, &mut files);
        assert_ne!(files.len(), 0);
        for path in files {
            let bytes = std::fs::read(&path).unwrap();
            let hay = String::from_utf8_lossy(&bytes);
            assert!(
                !hay.contains(&self.kit.token),
                "token in {}",
                path.display()
            );
        }
    }
}

fn json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
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

fn paths(receiver: &Receiver) -> Vec<String> {
    receiver.received().into_iter().map(|r| r.path).collect()
}

/// Collects the text of every tracing event, to prove no log holds the token.
#[derive(Clone, Default)]
struct LogCapture(Arc<Mutex<Vec<String>>>);

struct Fields<'a>(&'a mut String);

impl tracing::field::Visit for Fields<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let _ = write!(self.0, "{}={value:?} ", field.name());
    }
}

impl tracing::Subscriber for LogCapture {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut line = String::new();
        event.record(&mut Fields(&mut line));
        self.0.lock().unwrap().push(line);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

// ── Upload ─────────────────────────────────────────────────────────

#[tokio::test]
async fn a_complete_export_arrives_in_order_with_matching_hashes() {
    let rig = Rig::new(30);
    assert_eq!(rig.prepare(t0()), vec![ID_42.to_string()]);

    let report = rig.upload(t0()).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 1,
            failed: 0
        }
    );

    assert_eq!(
        paths(&rig.receiver),
        vec![
            format!("{ID_42}/{}", naming::backup_name(ID_42)),
            format!("{ID_42}/{}", naming::stats_name(ID_42)),
            format!("{ID_42}/{}", naming::manifest_name(ID_42)),
        ]
    );
    let manifest = Manifest::read(&rig.path(ID_42, naming::manifest_name(ID_42))).unwrap();
    for got in rig.receiver.received() {
        let name = got.path.rsplit('/').next().unwrap().to_string();
        let local = rig.path(ID_42, name.clone());
        assert_eq!(got.bytes, std::fs::read(&local).unwrap(), "{name}");
        assert_eq!(got.sha256, sha256_file(&local).unwrap(), "{name}");
        if let Some(entry) = manifest.files.iter().find(|f| f.name == name) {
            assert_eq!(got.sha256, entry.sha256, "{name}");
            assert_eq!(
                i64::try_from(got.bytes.len()).unwrap(),
                entry.bytes,
                "{name}"
            );
        } else {
            assert_eq!(name, naming::manifest_name(ID_42));
            assert_eq!(got.content_type, "application/json");
        }
    }
    assert_eq!(
        rig.receiver.received()[0].content_type,
        "application/octet-stream"
    );

    let status = rig.status();
    assert_eq!(status.pending, 0);
    assert_eq!(status.kept, 1);
    assert_eq!(status.last_uploaded_id.as_deref(), Some(ID_42));
    assert_eq!(
        status.last_uploaded_at.as_deref(),
        Some("2026-10-05T01:00:00Z")
    );
    assert!(status.last_error.is_none() && status.last_error_at.is_none());
    assert!(status.next_attempt_at.is_none());

    let state = rig.state(ID_42);
    assert_eq!(state["uploaded"].as_array().unwrap().len(), 3);
    assert_eq!(state["attempts"], 0);

    // A complete export is not sent again.
    let report = rig.upload(at(t0(), 3600)).await;
    assert_eq!(report, PassReport::default());
    assert_eq!(rig.receiver.put_count(), 3);
    rig.assert_token_is_nowhere();
}

#[tokio::test]
async fn exports_go_out_in_id_order() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    assert_eq!(
        rig.prepare(t0()),
        vec![ID_41.to_string(), ID_42.to_string()]
    );
    let report = rig.upload(t0()).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 2,
            failed: 0
        }
    );
    let got = paths(&rig.receiver);
    assert_eq!(got.len(), 6);
    assert!(got[..3].iter().all(|p| p.starts_with(ID_41)));
    assert!(got[3..].iter().all(|p| p.starts_with(ID_42)));
    assert_eq!(rig.status().last_uploaded_id.as_deref(), Some(ID_42));
}

#[tokio::test]
async fn failures_back_off_one_then_two_minutes_and_a_success_clears_the_error() {
    let logs = LogCapture::default();
    let _guard = tracing::subscriber::set_default(logs.clone());
    let rig = Rig::new(30);
    rig.prepare(t0());
    rig.receiver.fail_next(2, 503);

    // First failure: attempts 1, next attempt one minute later.
    let report = rig.upload(t0()).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 0,
            failed: 1
        }
    );
    let state = rig.state(ID_42);
    assert_eq!(state["attempts"], 1);
    assert_eq!(state["nextAttemptAt"], "2026-10-05T01:01:00Z");
    assert_eq!(state["lastError"], "HTTP 503");
    assert_eq!(state["uploaded"].as_array().unwrap().len(), 0);
    let status = rig.status();
    assert_eq!(status.pending, 1);
    assert_eq!(
        status.next_attempt_at.as_deref(),
        Some("2026-10-05T01:01:00Z")
    );
    assert_eq!(status.last_error.as_deref(), Some("HTTP 503"));
    assert_eq!(
        status.last_error_at.as_deref(),
        Some("2026-10-05T01:00:00Z")
    );

    // Not due yet: nothing is sent.
    assert_eq!(rig.upload(at(t0(), 30)).await, PassReport::default());
    assert_eq!(rig.receiver.put_count(), 1);

    // Due: second failure, attempts 2, two minutes later.
    let now = at(t0(), 60);
    assert_eq!(
        rig.upload(now).await,
        PassReport {
            uploaded: 0,
            failed: 1
        }
    );
    let state = rig.state(ID_42);
    assert_eq!(state["attempts"], 2);
    assert_eq!(state["nextAttemptAt"], "2026-10-05T01:03:00Z");
    assert_eq!(
        rig.status().last_error_at.as_deref(),
        Some("2026-10-05T01:01:00Z")
    );
    assert_eq!(rig.upload(at(t0(), 120)).await, PassReport::default());
    assert_eq!(rig.receiver.put_count(), 2);

    // Due again: the receiver accepts, the error is cleared.
    let now = at(t0(), 180);
    assert_eq!(
        rig.upload(now).await,
        PassReport {
            uploaded: 1,
            failed: 0
        }
    );
    let status = rig.status();
    assert_eq!(status.pending, 0);
    assert!(status.last_error.is_none() && status.last_error_at.is_none());
    assert!(status.next_attempt_at.is_none());
    assert_eq!(
        status.last_uploaded_at.as_deref(),
        Some("2026-10-05T01:03:00Z")
    );
    let state = rig.state(ID_42);
    assert!(state["nextAttemptAt"].is_null());
    assert!(state["lastError"].is_null());
    assert_eq!(paths(&rig.receiver).len(), 3);

    rig.assert_token_is_nowhere();
    let lines = logs.0.lock().unwrap().join("\n");
    assert!(lines.contains("upload failed"), "{lines}");
    assert!(!lines.contains(&rig.kit.token));
}

#[tokio::test]
async fn the_backoff_keeps_doubling_up_to_an_hour() {
    let rig = Rig::new(30);
    rig.prepare(t0());
    rig.receiver.fail_next(100, 500);
    let mut now = t0();
    let mut waits = Vec::new();
    for _ in 0..9 {
        rig.upload(now).await;
        let next = t(rig.state(ID_42)["nextAttemptAt"].as_str().unwrap());
        waits.push((next - now).num_minutes());
        now = next;
    }
    assert_eq!(waits, vec![1, 2, 4, 8, 16, 32, 60, 60, 60]);
    assert_eq!(rig.state(ID_42)["attempts"], 9);
}

#[tokio::test]
async fn a_refused_token_shows_as_unauthorized() {
    let logs = LogCapture::default();
    let _guard = tracing::subscriber::set_default(logs.clone());
    let rig = Rig::new(30);
    rig.prepare(t0());
    rig.receiver.reject_all();

    let report = rig.upload(t0()).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 0,
            failed: 1
        }
    );
    let status = rig.status();
    assert_eq!(
        status.last_error.as_deref(),
        Some("unauthorized (HTTP 401)")
    );
    assert_eq!(rig.state(ID_42)["lastError"], "unauthorized (HTTP 401)");
    assert_eq!(rig.state(ID_42)["attempts"], 1);
    assert_eq!(status.pending, 1);

    // Fixed on the receiver: the next due pass sends everything.
    rig.receiver.accept_tokens();
    let report = rig.upload(at(t0(), 60)).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 1,
            failed: 0
        }
    );
    assert!(rig.status().last_error.is_none());
    rig.assert_token_is_nowhere();
    assert!(!logs.0.lock().unwrap().join("\n").contains(&rig.kit.token));
}

#[tokio::test]
async fn an_interrupted_export_resumes_with_the_missing_files_only() {
    let rig = Rig::new(30);
    rig.prepare(t0());
    // The statistics file disappears: the backup goes out, the pass stops there.
    let stats_path = rig.path(ID_42, naming::stats_name(ID_42));
    let bytes = std::fs::read(&stats_path).unwrap();
    std::fs::remove_file(&stats_path).unwrap();

    let report = rig.upload(t0()).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 0,
            failed: 1
        }
    );
    assert_eq!(
        paths(&rig.receiver),
        vec![format!("{ID_42}/{}", naming::backup_name(ID_42))]
    );
    let state = rig.state(ID_42);
    assert_eq!(
        state["uploaded"],
        serde_json::json!([naming::backup_name(ID_42)])
    );
    assert!(
        state["lastError"]
            .as_str()
            .unwrap()
            .starts_with("file unreadable")
    );
    assert_eq!(state["attempts"], 1);

    std::fs::write(&stats_path, bytes).unwrap();
    let report = rig.upload(at(t0(), 60)).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 1,
            failed: 0
        }
    );
    assert_eq!(
        paths(&rig.receiver),
        vec![
            format!("{ID_42}/{}", naming::backup_name(ID_42)),
            format!("{ID_42}/{}", naming::stats_name(ID_42)),
            format!("{ID_42}/{}", naming::manifest_name(ID_42)),
        ]
    );
}

#[tokio::test]
async fn a_failing_export_does_not_hold_back_the_next_one() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    rig.prepare(t0());
    // Z 41 cannot be read: it fails, Z 42 goes through.
    std::fs::remove_file(rig.path(ID_41, naming::backup_name(ID_41))).unwrap();
    let report = rig.upload(t0()).await;
    assert_eq!(
        report,
        PassReport {
            uploaded: 1,
            failed: 1
        }
    );
    assert_eq!(rig.state(ID_41)["attempts"], 1);
    assert_eq!(rig.status().pending, 1);
    assert_eq!(rig.status().last_uploaded_id.as_deref(), Some(ID_42));
    // The later success does not hide the failure of the earlier export.
    assert!(rig.status().last_error.is_some());
}

#[tokio::test]
async fn retry_now_makes_a_backed_off_export_due() {
    let rig = Rig::new(30);
    rig.prepare(t0());
    rig.receiver.fail_next(1, 503);
    rig.upload(t0()).await;
    assert_eq!(rig.state(ID_42)["nextAttemptAt"], "2026-10-05T01:01:00Z");

    // Ten seconds later it is still waiting.
    let now = at(t0(), 10);
    assert_eq!(rig.upload(now).await, PassReport::default());
    assert_eq!(outbox::retry_now(&rig.env(), now).unwrap(), 1);
    assert_eq!(rig.state(ID_42)["nextAttemptAt"], "2026-10-05T01:00:10Z");
    assert_eq!(
        rig.status().next_attempt_at.as_deref(),
        Some("2026-10-05T01:00:10Z")
    );
    assert_eq!(
        rig.upload(now).await,
        PassReport {
            uploaded: 1,
            failed: 0
        }
    );

    // Nothing incomplete is left to retry.
    assert_eq!(outbox::retry_now(&rig.env(), now).unwrap(), 0);
}

#[tokio::test]
async fn an_unreadable_token_ends_the_pass_with_an_error_and_sends_nothing() {
    let rig = Rig::new(30);
    rig.prepare(t0());
    std::fs::remove_file(&rig.kit.settings.token_file).unwrap();
    let report = rig.upload(t0()).await;
    assert_eq!(report, PassReport::default());
    assert_eq!(rig.receiver.put_count(), 0);
    let status = rig.status();
    assert!(status.last_error.unwrap().starts_with("export token file "));
    assert_eq!(status.pending, 1);
    // The export itself was not charged an attempt.
    assert_eq!(rig.state(ID_42)["attempts"], 0);

    std::fs::write(&rig.kit.settings.token_file, &rig.kit.token).unwrap();
    assert_eq!(
        rig.upload(t0()).await,
        PassReport {
            uploaded: 1,
            failed: 0
        }
    );
    assert!(rig.status().last_error.is_none());
}

// ── Retention of sent and unsent exports ───────────────────────────

#[tokio::test]
async fn retention_counts_only_the_unsent_exports_in_dropped() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    rig.prepare(t0());
    assert_eq!(rig.upload(t0()).await.uploaded, 2);
    // A third export, created a day later, is never sent.
    let id_43 = add_empty_z(&rig.fx, 43, "2026-10-05");
    let day_later = t0() + Duration::days(1);
    assert_eq!(rig.prepare(day_later), vec![id_43]);
    assert_eq!(rig.status().kept, 3);
    assert_eq!(rig.status().pending, 1);

    // The two sent ones are old, the unsent one is not.
    let first = t0() + Duration::days(31);
    assert_eq!(outbox::retention(&rig.env(), first).unwrap(), 2);
    let status = rig.status();
    assert_eq!((status.kept, status.pending, status.dropped), (1, 1, 0));
    assert_eq!(
        status.last_uploaded_id.as_deref(),
        Some(ID_42),
        "history of the status stays"
    );

    // Later the unsent one is dropped, and counted.
    let second = day_later + Duration::days(31);
    assert_eq!(outbox::retention(&rig.env(), second).unwrap(), 1);
    let status = rig.status();
    assert_eq!((status.kept, status.pending, status.dropped), (0, 0, 1));
    assert!(rig.exports.join("cursor.json").exists());
    assert!(rig.exports.join("pseudonym.key").exists());
    assert!(rig.exports.join("status.json").exists());
    rig.assert_token_is_nowhere();
}

#[tokio::test]
async fn status_names_the_newest_export_even_when_its_state_is_unreadable() {
    let rig = Rig::new(30);
    rig.set_cursor(40);
    assert_eq!(rig.prepare(t0()).len(), 2);
    std::fs::write(rig.exports.join(ID_42).join("state.json"), b"not json").unwrap();
    let status = rig.status();
    assert_eq!(status.kept, 2);
    assert_eq!(status.last_export_id.as_deref(), Some(ID_42));
    assert_eq!(status.last_prepared_at, None);
    assert_eq!(status.pending, 1);
}
