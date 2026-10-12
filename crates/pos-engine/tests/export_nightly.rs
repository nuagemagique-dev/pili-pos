//! The nightly export in the engine (phase X1, card X1-T6): after a real Z
//! the engine builds the three files and uploads them to the endpoint of the
//! `export` section, in the background, without ever slowing a sale.
//!
//! Real engines are opened with `Engine::open` on a temporary data directory
//! (a real `pos.db` file: the export snapshots the file), with the SPF test
//! identity, an in-process FDM (`pos-fdm-mock`) and the export receiver
//! (`pos-export-mock`) on a local port. Users are seeded and logged in
//! directly, as the other engine suites do.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pos_core::order::OrderType;
use pos_core::user::{User, UserRole};
use pos_engine::commands::access::ERR_FORBIDDEN;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::export::{ExportStatus, do_export_run_now, do_export_status};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::report::{BACKUPS_KEPT, close_overdue_day, do_generate_z_report};
use pos_engine::{
    ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers, HttpTls, NullSink,
    SoftwareIdentity,
};
use pos_export::manifest::Manifest;
use pos_export::naming;
use pos_export_mock::Receiver;
use rusqlite::{Connection, OpenFlags};

const OPERATOR: &str = "12345678958";
const OPERATOR_NAME: &str = "Alice Dupont";
const CASHIER: &str = "84022899837";
const CASHIER_NAME: &str = "Bob Martin";
const TOKEN: &str = "tok-export-nightly-test";

// ── The rig ────────────────────────────────────────────────────────

/// What the `export` section of `pos.json` looks like.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    /// No `export` section at all.
    Absent,
    /// A usable section, the token file holding the right token.
    Valid,
    /// A usable section whose token file holds a token the receiver refuses.
    WrongToken,
    /// An owner recipient that has the shape of an age key but not its
    /// checksum.
    BadRecipient,
}

struct Rig {
    _dir: tempfile::TempDir,
    data_dir: PathBuf,
    config: PathBuf,
    receiver: Receiver,
    owner_secret: String,
    owner_public: String,
    producer_secret: String,
    producer_public: String,
    export_outbox: Duration,
    engine: Option<Engine>,
}

fn timers(export_outbox: Duration) -> EngineTimers {
    EngineTimers {
        // The automatic Z is driven by hand in these tests.
        auto_z: Duration::from_secs(3600),
        export_outbox,
        ..EngineTimers::default()
    }
}

impl Rig {
    async fn start(section: Section) -> Rig {
        Rig::start_with(section, Duration::from_millis(100)).await
    }

    /// `export_outbox` is the interval of the export loop.
    async fn start_with(section: Section, export_outbox: Duration) -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let data_dir = root.join("data");

        let fdm = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
        tokio::spawn(async move {
            axum::serve(fdm, pos_fdm_mock::app()).await.unwrap();
        });

        let (owner_secret, owner_public) = pos_export::crypt::generate_identity();
        let (producer_secret, producer_public) = pos_export::crypt::generate_identity();
        let receiver = Receiver::start(TOKEN);

        let token_file = root.join("export.token");
        let token = if section == Section::WrongToken {
            "not-the-right-token"
        } else {
            TOKEN
        };
        std::fs::write(&token_file, token).unwrap();

        let owner = if section == Section::BadRecipient {
            // Same shape, wrong checksum.
            let last = if owner_public.ends_with('q') {
                "p"
            } else {
                "q"
            };
            format!("{}{last}", &owner_public[..owner_public.len() - 1])
        } else {
            owner_public.clone()
        };
        let export = if section == Section::Absent {
            String::new()
        } else {
            format!(
                r#", "export": {{
                    "url": "{}", "token_file": "{}",
                    "owner_recipient": "{owner}", "producer_recipient": "{producer_public}",
                    "keep_days": 14 }}"#,
                receiver.url(),
                token_file.display()
            )
        };
        // The SPF test identity (UC_TestSetup_20240806), as the harness has it.
        let config = root.join("pos.json");
        std::fs::write(
            &config,
            format!(
                r#"{{
                "pos_id": "CPOS0031234567", "terminal_id": "POS-1-BAR",
                "device_id": "F0:1D:3D:CC:13:C8",
                "restaurant_name": "The FPS Finances Experience",
                "restaurant_address": "Boulevard Albert II 33, 1030 Brussels",
                "restaurant_vat": "BE0000000097", "restaurant_est_no": "2000000042",
                "fdm_url": "{fdm_url}", "table_count": 25, "day_end_hour": 5
                {export} }}"#
            ),
        )
        .unwrap();

        let mut rig = Rig {
            _dir: dir,
            data_dir,
            config,
            receiver,
            owner_secret,
            owner_public,
            producer_secret,
            producer_public,
            export_outbox,
            engine: None,
        };
        rig.open_engine();
        rig.seed();
        rig.login(OPERATOR, OPERATOR_NAME, UserRole::Admin);
        rig.engine()
            .start_background(&tokio::runtime::Handle::current());
        rig
    }

    fn open_engine(&mut self) {
        let engine = Engine::open(EngineConfig {
            paths: EnginePaths::under(self.data_dir.clone()),
            config: ConfigSource::File(self.config.clone()),
            events: Arc::new(NullSink),
            timers: timers(self.export_outbox),
            identity: SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
            http_tls: HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        })
        .expect("open the engine");
        self.engine = Some(engine);
    }

    fn engine(&self) -> &Engine {
        self.engine.as_ref().expect("an open engine")
    }

    /// A menu with one item. Idempotent.
    fn seed(&self) {
        self.engine()
            .db
            .lock()
            .unwrap()
            .execute_batch(
                "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1);
                 INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
                    VALUES (1, 1, 1200, 'A', 0, 1);",
            )
            .unwrap();
    }

    /// Register `insz` and make them the logged-in operator.
    fn login(&self, insz: &str, name: &str, role: UserRole) {
        self.engine()
            .db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO users (insz, name, role, active) VALUES (?1, ?2, ?3, 1)
                 ON CONFLICT(insz) DO UPDATE SET name = excluded.name, role = excluded.role, active = 1",
                rusqlite::params![insz, name, role.as_str()],
            )
            .unwrap();
        *self.engine().session.lock().unwrap() = Some(User {
            insz: insz.to_string(),
            name: name.to_string(),
            role,
            active: true,
            has_pin: false,
        });
    }

    fn logout(&self) {
        *self.engine().session.lock().unwrap() = None;
    }

    /// Stop the engine and release its data directory.
    async fn close_engine(&mut self) {
        let engine = self.engine.take().expect("an open engine");
        engine.shutdown();
        let finished = tokio::time::timeout(Duration::from_secs(10), async {
            for task in engine.take_background_tasks() {
                task.await.expect("a loop panicked");
            }
        })
        .await;
        assert!(finished.is_ok(), "a background task outlived shutdown");
        drop(engine);
    }

    /// One sale of 12.00 by the logged-in operator.
    async fn sell(&self) -> Result<bool, String> {
        let state: &pos_engine::state::AppState = self.engine();
        let order = do_create_order(
            state,
            CreateOrderInput {
                order_type: OrderType::Takeaway,
                table_number: None,
                user_insz: OPERATOR.into(),
            },
        )?
        .id;
        do_add_order_line(
            state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: 1,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )?;
        do_checkout(state, order, CheckoutPaymentMethod::Card)
            .await
            .map(|r| r.success)
    }

    /// The Z of the open day, by the operator.
    async fn z(&self) -> pos_engine::commands::report::ReportResult {
        do_generate_z_report(self.engine(), OPERATOR.into())
            .await
            .expect("the Z command")
    }

    /// Move the open booking period `days` business days back, as if the POS
    /// had been left on since then (as `spf_e2e_auto_z.rs` does); return its
    /// date.
    fn age_open_period(&self, days: i64) -> String {
        let day_end = i64::from(self.engine().config.lock().unwrap().day_end_hour);
        let date = (pos_fiscal::event::now_in_belgium()
            - chrono::Duration::hours(day_end)
            - chrono::Duration::days(days))
        .date_naive()
        .to_string();
        self.engine()
            .db
            .lock()
            .unwrap()
            .execute(
                "UPDATE booking_periods SET booking_date = ?1 WHERE closed_at IS NULL",
                rusqlite::params![date],
            )
            .unwrap();
        date
    }

    fn live_event_count(&self) -> i64 {
        self.engine()
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
            .unwrap()
    }

    async fn status(&self) -> ExportStatus {
        do_export_status(self.engine())
            .await
            .expect("export status")
    }

    /// Poll the status until `done` holds (10 s at most).
    async fn wait_status(&self, what: &str, done: impl Fn(&ExportStatus) -> bool) -> ExportStatus {
        let started = Instant::now();
        loop {
            let status = self.status().await;
            if done(&status) {
                return status;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "timed out waiting for {what}; last status: {status:?}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Wait until the receiver holds the three files of `id`.
    async fn wait_files(&self, id: &str) -> Files {
        let started = Instant::now();
        loop {
            let backup = self
                .receiver
                .file(&format!("{id}/{}", naming::backup_name(id)));
            let stats = self
                .receiver
                .file(&format!("{id}/{}", naming::stats_name(id)));
            let manifest = self
                .receiver
                .file(&format!("{id}/{}", naming::manifest_name(id)));
            if let (Some(backup), Some(stats), Some(manifest)) = (backup, stats, manifest) {
                return Files {
                    backup,
                    stats,
                    manifest,
                };
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "timed out waiting for the files of {id}; received: {:?}",
                self.receiver
                    .received()
                    .iter()
                    .map(|r| r.path.clone())
                    .collect::<Vec<_>>()
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// A sale, then the Z by hand; return the export id once the three files
    /// reached the receiver.
    async fn export_a_day(&self) -> (String, Files) {
        assert!(self.sell().await.unwrap());
        let z = self.z().await;
        assert!(z.success, "Z: {:?}", z.error);
        let status = self
            .wait_status("the export to be uploaded", |s| {
                s.pending == 0 && s.last_uploaded_id.is_some()
            })
            .await;
        let id = status.last_uploaded_id.unwrap();
        let files = self.wait_files(&id).await;
        (id, files)
    }

    fn exports_dir(&self) -> PathBuf {
        self.data_dir.join("exports")
    }
}

struct Files {
    backup: Vec<u8>,
    stats: Vec<u8>,
    manifest: Vec<u8>,
}

fn ro(path: &Path) -> Connection {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only")
}

fn count_events(path: &Path) -> i64 {
    ro(path)
        .query_row("SELECT COUNT(*) FROM fiscal_events", [], |r| r.get(0))
        .unwrap()
}

// ── (a) the whole way ──────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_sale_and_a_z_reach_the_receiver_intact() {
    let mut rig = Rig::start(Section::Valid).await;
    let started = Instant::now();
    let (id, files) = rig.export_a_day().await;
    eprintln!("(a) Z to the three files received: {:?}", started.elapsed());
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(id.ends_with("-z000001"), "the first Z: {id}");

    // Sizes feed decision X1D14.
    eprintln!(
        "(a) sizes of {id}: pos-{id}.db.age = {} bytes, stats-{id}.sqlite = {} bytes, manifest-{id}.json = {} bytes",
        files.backup.len(),
        files.stats.len(),
        files.manifest.len()
    );

    // The manifest hashes match the bodies.
    let work = tempfile::tempdir().unwrap();
    let backup_path = work.path().join(naming::backup_name(&id));
    let stats_path = work.path().join(naming::stats_name(&id));
    let manifest_path = work.path().join(naming::manifest_name(&id));
    std::fs::write(&backup_path, &files.backup).unwrap();
    std::fs::write(&stats_path, &files.stats).unwrap();
    std::fs::write(&manifest_path, &files.manifest).unwrap();
    let manifest = Manifest::read(&manifest_path).unwrap();
    manifest
        .verify(work.path())
        .expect("the manifest matches the bodies");
    assert_eq!(manifest.export_id, id);
    assert_eq!(manifest.z_report_no, 1);
    assert_eq!(manifest.pos_id, "CPOS0031234567");
    assert_eq!(manifest.pos_sw_version, "pili-pos/1.0.0");
    assert_eq!(
        manifest.recipients,
        vec![rig.owner_public.clone(), rig.producer_public.clone()]
    );
    let names: Vec<&str> = manifest.files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [naming::backup_name(&id), naming::stats_name(&id)],
        "the manifest lists the two data files"
    );
    for f in &manifest.files {
        let body = if f.name == naming::backup_name(&id) {
            &files.backup
        } else {
            &files.stats
        };
        assert_eq!(f.bytes as usize, body.len(), "{}", f.name);
    }

    // The backup decrypts with the owner's and with the producer's key to a
    // database with the live fiscal events.
    let live = rig.live_event_count();
    assert!(live >= 3, "a sale and the two R events: {live}");
    for (who, secret) in [
        ("owner", &rig.owner_secret),
        ("producer", &rig.producer_secret),
    ] {
        let plain = work.path().join(format!("{who}.db"));
        pos_export::crypt::decrypt_file(secret, &backup_path, &plain)
            .unwrap_or_else(|e| panic!("{who} cannot decrypt: {e}"));
        assert_eq!(count_events(&plain), live, "{who}'s copy");
    }

    // The statistics file: schema 1 and the sale.
    let stats = ro(&stats_path);
    let schema: String = stats
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(schema, "1");
    let (tickets, total): (i64, i64) = stats
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(total_cents), 0) FROM tickets",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((tickets, total), (1, 1200), "the sale of the day");

    // The status says it is done.
    let status = rig.status().await;
    assert!(status.enabled && status.problem.is_none(), "{status:?}");
    assert_eq!(status.pending, 0);
    assert_eq!(status.last_uploaded_id.as_deref(), Some(id.as_str()));
    assert_eq!(status.last_export_id.as_deref(), Some(id.as_str()));
    assert_eq!(status.last_error, None);
    assert_eq!(
        status.endpoint.as_deref(),
        Some(rig.receiver.url().as_str())
    );
    assert_eq!(status.keep_days, 14);
    rig.close_engine().await;
}

// ── The wake after a real Z ────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_real_z_starts_the_export_without_waiting_for_the_tick() {
    // The loop would not tick for an hour: only the wake after the Z can
    // explain an upload within seconds.
    let mut rig = Rig::start_with(Section::Valid, Duration::from_secs(3600)).await;
    let (id, _) = rig.export_a_day().await;
    assert!(id.ends_with("-z000001"), "{id}");
    rig.close_engine().await;
}

// ── Backup retention after each Z ──────────────────────────────────

/// The engine backups (`pos-<timestamp>.db`) of the backup directory.
fn engine_backups(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("pos-") && n.ends_with(".db"))
        .collect();
    names.sort();
    names
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn nine_z_in_a_row_leave_the_seven_newest_backups() {
    let mut rig = Rig::start_with(Section::Valid, Duration::from_secs(3600)).await;
    let backups = rig.data_dir.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    // A copy made by hand (REMOTE-OPS.md) is not the engine's to delete.
    std::fs::write(backups.join("manual-20250101-000000.db"), "by hand").unwrap();

    let mut made = Vec::new();
    for day in 1..=9 {
        let before = engine_backups(&backups);
        assert!(rig.sell().await.unwrap());
        let z = rig.z().await;
        assert!(z.success, "Z {day}: {:?}", z.error);
        let after = engine_backups(&backups);
        let new: Vec<_> = after.iter().filter(|n| !before.contains(n)).collect();
        assert_eq!(new.len(), 1, "one backup per Z: {after:?}");
        made.push(new[0].clone());
        assert_eq!(after.len(), day.min(BACKUPS_KEPT), "{after:?}");
    }

    assert_eq!(engine_backups(&backups), made[made.len() - BACKUPS_KEPT..]);
    assert!(backups.join("manual-20250101-000000.db").is_file());
    assert!(rig.data_dir.join("pos.db").is_file(), "pos.db untouched");
    // The nightly export of the ninth day goes out as usual.
    rig.wait_status("the ninth export to be uploaded", |s| {
        s.pending == 0
            && s.last_uploaded_id
                .as_deref()
                .is_some_and(|i| i.ends_with("-z000009"))
    })
    .await;
    assert!(rig.exports_dir().is_dir());
    // The newest backup holds the ninth Z.
    let newest = backups.join(made.last().unwrap());
    let z_events: i64 = ro(&newest)
        .query_row(
            "SELECT COUNT(*) FROM fiscal_events WHERE event_type = 'R'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(z_events >= 9, "{z_events} R events");
    rig.close_engine().await;
}

// ── (b) the export never blocks ────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_receiver_that_takes_30_seconds_blocks_neither_the_z_nor_the_next_sale() {
    let mut rig = Rig::start(Section::Valid).await;
    rig.receiver.set_delay(Duration::from_secs(30));
    assert!(rig.sell().await.unwrap());

    let started = Instant::now();
    let z = rig.z().await;
    let z_took = started.elapsed();
    assert!(z.success, "Z: {:?}", z.error);
    assert!(z_took < Duration::from_secs(3), "the Z took {z_took:?}");

    // The export is prepared and waiting on the receiver while the next
    // sale goes through.
    let started = Instant::now();
    assert!(rig.sell().await.unwrap());
    let sale_took = started.elapsed();
    assert!(
        sale_took < Duration::from_secs(3),
        "the sale after the Z took {sale_took:?}"
    );
    eprintln!("(b) Z took {z_took:?}, the next sale {sale_took:?}, receiver delay 30 s");

    let status = rig
        .wait_status("the export to be prepared and in flight", |s| {
            s.last_export_id.is_some() && rig.receiver.put_count() >= 1
        })
        .await;
    assert_eq!(status.pending, 1, "{status:?}");
    assert_eq!(status.last_uploaded_id, None);

    // Shutting down does not wait for the slow answer.
    let started = Instant::now();
    rig.close_engine().await;
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "shutdown waited for the receiver"
    );
}

// ── (c) the automatic Z is exported too ────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_automatic_z_is_exported() {
    let mut rig = Rig::start(Section::Valid).await;
    assert!(rig.sell().await.unwrap());
    let day = rig.age_open_period(2);

    assert!(
        close_overdue_day(rig.engine()).await.unwrap(),
        "the automatic Z"
    );

    let status = rig
        .wait_status("the automatic Z to be uploaded", |s| {
            s.pending == 0 && s.last_uploaded_id.is_some()
        })
        .await;
    let id = status.last_uploaded_id.unwrap();
    assert_eq!(id, naming::export_id(&day, 1).unwrap());
    rig.wait_files(&id).await;
    rig.close_engine().await;
}

// ── (d) a training Z exports nothing ───────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_training_z_exports_nothing() {
    let mut rig = Rig::start(Section::Valid).await;
    rig.engine().config.lock().unwrap().training_mode = true;
    assert!(rig.sell().await.unwrap());
    let z = rig.z().await;
    assert!(z.success, "training Z: {:?}", z.error);
    assert_eq!(z.report_no, None, "a training Z takes no number");

    // Let the loop pass a few times over the training day.
    let passes = rig.engine().export_passes();
    let started = Instant::now();
    while rig.engine().export_passes() < passes + 3 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the loop stopped"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(rig.receiver.put_count(), 0, "nothing was uploaded");
    let status = rig.status().await;
    assert_eq!(status.last_export_id, None, "{status:?}");
    assert_eq!(status.kept, 0);
    assert_eq!(status.last_error, None);

    // The same engine exports the real day that follows.
    rig.engine().config.lock().unwrap().training_mode = false;
    let (id, _) = rig.export_a_day().await;
    assert!(id.ends_with("-z000001"), "{id}");
    rig.close_engine().await;
}

// ── (e) retries ────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn failed_uploads_are_retried_and_run_now_makes_them_due_at_once() {
    let mut rig = Rig::start(Section::Valid).await;
    rig.receiver.fail_next(2, 503);
    assert!(rig.sell().await.unwrap());
    let z = rig.z().await;
    assert!(z.success, "Z: {:?}", z.error);

    // First attempt refused: pending, with the error and a retry time.
    let status = rig
        .wait_status("the first refusal", |s| s.last_error.is_some())
        .await;
    assert_eq!(status.pending, 1, "{status:?}");
    assert_eq!(status.last_error.as_deref(), Some("HTTP 503"));
    assert!(status.next_attempt_at.is_some(), "{status:?}");
    assert_eq!(status.last_uploaded_id, None);

    // The backoff (1 minute) keeps the loop from trying again.
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(rig.receiver.put_count(), 1, "the backoff holds");

    // Run now: due at once, refused a second time.
    let status = do_export_run_now(rig.engine()).await.unwrap();
    assert_eq!(status.pending, 1);
    let started = Instant::now();
    while rig.receiver.put_count() < 2 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "no second attempt"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    // Let the pass that made the second attempt finish.
    let passes = rig.engine().export_passes();
    while rig.engine().export_passes() < passes + 2 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the loop stopped"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let status = rig.status().await;
    assert_eq!(status.pending, 1, "{status:?}");
    assert_eq!(status.last_error.as_deref(), Some("HTTP 503"));
    assert_eq!(
        rig.receiver.put_count(),
        2,
        "no third attempt inside the backoff"
    );

    // Run now again: accepted this time.
    do_export_run_now(rig.engine()).await.unwrap();
    let status = rig
        .wait_status("the export to be uploaded", |s| {
            s.pending == 0 && s.last_uploaded_id.is_some()
        })
        .await;
    assert_eq!(status.last_error, None, "{status:?}");
    assert_eq!(status.next_attempt_at, None);
    let id = status.last_uploaded_id.expect("uploaded");
    assert_eq!(status.last_export_id.as_deref(), Some(id.as_str()));
    rig.wait_files(&id).await;
    rig.close_engine().await;
}

// ── (f) a wrong token ──────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_wrong_token_is_reported_and_sales_go_on() {
    let mut rig = Rig::start(Section::WrongToken).await;
    assert!(rig.sell().await.unwrap());
    let z = rig.z().await;
    assert!(z.success, "Z: {:?}", z.error);

    let status = rig
        .wait_status("the refusal", |s| s.last_error.is_some())
        .await;
    assert_eq!(
        status.last_error.as_deref(),
        Some("unauthorized (HTTP 401)"),
        "{status:?}"
    );
    assert_eq!(status.pending, 1);
    assert!(
        !format!("{status:?}").contains("not-the-right-token"),
        "the token stays out of the status"
    );

    // Sales and a second Z still work.
    assert!(rig.sell().await.unwrap());
    let started = Instant::now();
    assert!(rig.sell().await.unwrap());
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(rig.receiver.received().is_empty());
    rig.close_engine().await;
}

// ── (g) no section, or an unusable one ─────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn without_an_export_section_nothing_exists() {
    let mut rig = Rig::start(Section::Absent).await;
    assert!(rig.sell().await.unwrap());
    let z = rig.z().await;
    assert!(z.success, "Z: {:?}", z.error);

    let status = rig.status().await;
    assert!(!status.enabled);
    assert_eq!(status.problem, None);
    assert_eq!(status.endpoint, None);
    assert_eq!((status.pending, status.kept), (0, 0));

    // Run now is only a status, and still creates nothing.
    let status = do_export_run_now(rig.engine()).await.unwrap();
    assert!(!status.enabled);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!rig.exports_dir().exists(), "no exports directory");
    assert_eq!(rig.receiver.put_count(), 0);
    assert_eq!(rig.engine().export_passes(), 0, "no export loop runs");
    rig.close_engine().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn an_unusable_section_turns_the_export_off_and_nothing_else() {
    let mut rig = Rig::start(Section::BadRecipient).await;
    assert!(rig.sell().await.unwrap());
    let z = rig.z().await;
    assert!(z.success, "Z: {:?}", z.error);

    let status = rig.status().await;
    assert!(!status.enabled, "{status:?}");
    let problem = status.problem.expect("the problem is shown");
    assert!(problem.contains("owner_recipient"), "{problem}");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!rig.exports_dir().exists(), "no exports directory");
    assert_eq!(rig.receiver.put_count(), 0);
    assert!(rig.sell().await.unwrap(), "sales work");
    rig.close_engine().await;
}

// ── (h) catch-up after a restart ───────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_z_made_while_the_receiver_is_down_is_uploaded_after_a_restart() {
    let mut rig = Rig::start(Section::Valid).await;
    let addr = rig.receiver.addr();
    rig.receiver.stop();

    assert!(rig.sell().await.unwrap());
    let z = rig.z().await;
    assert!(z.success, "Z: {:?}", z.error);
    let status = rig
        .wait_status("the failed upload", |s| s.last_error.is_some())
        .await;
    assert_eq!(status.pending, 1, "{status:?}");
    assert!(
        status
            .last_error
            .as_deref()
            .unwrap()
            .starts_with("connection failed"),
        "{status:?}"
    );
    let id = status.last_export_id.clone().expect("prepared");

    // The engine goes away, the receiver comes back, a new engine opens the
    // same data directory.
    rig.close_engine().await;
    rig.receiver = Receiver::start_on(addr, TOKEN, None).expect("the receiver restarts");
    rig.open_engine();
    rig.login(OPERATOR, OPERATOR_NAME, UserRole::Admin);
    rig.engine()
        .start_background(&tokio::runtime::Handle::current());

    let status = rig
        .wait_status("the catch-up upload", |s| {
            s.pending == 0 && s.last_uploaded_id.is_some()
        })
        .await;
    assert_eq!(status.last_uploaded_id.as_deref(), Some(id.as_str()));
    assert_eq!(status.last_error, None, "{status:?}");
    rig.wait_files(&id).await;
    // Nothing was prepared twice.
    assert_eq!(status.kept, 1);
    rig.close_engine().await;
}

// ── (i) administrators only ────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn both_commands_refuse_a_session_that_is_not_an_admin() {
    let mut rig = Rig::start(Section::Valid).await;

    rig.login(CASHIER, CASHIER_NAME, UserRole::Cashier);
    assert_eq!(
        do_export_status(rig.engine()).await.unwrap_err(),
        ERR_FORBIDDEN
    );
    assert_eq!(
        do_export_run_now(rig.engine()).await.unwrap_err(),
        ERR_FORBIDDEN
    );

    rig.login("91051234567", "Carla Peeters", UserRole::Waiter);
    assert_eq!(
        do_export_status(rig.engine()).await.unwrap_err(),
        errors::FORBIDDEN
    );
    assert_eq!(
        do_export_run_now(rig.engine()).await.unwrap_err(),
        errors::FORBIDDEN
    );

    rig.logout();
    assert_eq!(
        do_export_status(rig.engine()).await.unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_export_run_now(rig.engine()).await.unwrap_err(),
        errors::NOT_LOGGED_IN
    );

    rig.login(OPERATOR, OPERATOR_NAME, UserRole::Admin);
    assert!(rig.status().await.enabled);
    rig.close_engine().await;
}

// ── (j) no personal data end to end ────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_statistics_file_holds_no_personal_data() {
    let mut rig = Rig::start(Section::Valid).await;
    // A second user who signs a sale too.
    assert!(rig.sell().await.unwrap());
    rig.login(CASHIER, CASHIER_NAME, UserRole::Cashier);
    let state: &pos_engine::state::AppState = rig.engine();
    let order = do_create_order(
        state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: CASHIER.into(),
        },
    )
    .unwrap()
    .id;
    do_add_order_line(
        state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: 1,
            quantity: 2,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    assert!(
        do_checkout(state, order, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    rig.login(OPERATOR, OPERATOR_NAME, UserRole::Admin);
    assert!(rig.z().await.success);
    let status = rig
        .wait_status("the export to be uploaded", |s| {
            s.pending == 0 && s.last_uploaded_id.is_some()
        })
        .await;
    let id = status.last_uploaded_id.unwrap();
    let files = rig.wait_files(&id).await;

    let work = tempfile::tempdir().unwrap();
    let stats_path = work.path().join(naming::stats_name(&id));
    std::fs::write(&stats_path, &files.stats).unwrap();
    let tickets: i64 = ro(&stats_path)
        .query_row("SELECT COUNT(*) FROM tickets", [], |r| r.get(0))
        .unwrap();
    assert_eq!(tickets, 2);

    let mut secrets: Vec<String> = [OPERATOR, CASHIER, OPERATOR_NAME, CASHIER_NAME]
        .iter()
        .map(|s| s.to_string())
        .collect();
    secrets.push(TOKEN.to_string());
    assert_no_personal_data(&stats_path, &secrets);
    // The manifest carries the installation, never a person or the token.
    let manifest = String::from_utf8(files.manifest.clone()).unwrap();
    for secret in &secrets {
        assert!(
            !manifest.contains(secret.as_str()),
            "manifest holds {secret:?}"
        );
    }
    rig.close_engine().await;
}

// ── The privacy proof of crates/pos-export/tests/common/mod.rs ─────

/// Column-name parts that mark personal or secret data.
const FORBIDDEN_COLUMN_PARTS: [&str; 13] = [
    "insz",
    "niss",
    "pin",
    "hash",
    "token",
    "secret",
    "password",
    "mail",
    "phone",
    "address",
    "customer",
    "signature",
    "qr",
];

/// Names of the tables of an SQLite file (without internal tables), sorted.
fn table_names(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Column names of a table, in order.
fn column_names(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// True when `s` is a user pseudonym of the statistics file: `u-` and 16
/// lowercase hex characters (`pos_export::pseudonym`).
fn is_pseudonym(s: &str) -> bool {
    s.strip_prefix("u-").is_some_and(|h| {
        h.len() == 16
            && h.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    })
}

/// True when `s` holds a run of at least `n` ASCII digits.
fn has_digit_run(s: &str, n: usize) -> bool {
    let mut run = 0;
    for c in s.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= n {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// Fail unless the SQLite file at `path` holds no personal data.
///
/// 1. No column name contains one of [`FORBIDDEN_COLUMN_PARTS`] (any case),
///    and no column is called `name` or `user_name` (`product_name` is fine).
/// 2. No TEXT value of any table contains a run of 11 digits (an INSZ), an
///    `@`, a bcrypt prefix (`$2a$`, `$2b$`, `$2y$`) or any of `secrets`.
/// 3. The raw bytes of the file contain none of the `secrets` either, so
///    nothing survives in free pages or in the schema text.
fn assert_no_personal_data(path: &Path, secrets: &[String]) {
    let conn = ro(path);
    for table in table_names(&conn) {
        for column in column_names(&conn, &table) {
            let lower = column.to_lowercase();
            for part in FORBIDDEN_COLUMN_PARTS {
                assert!(
                    !lower.contains(part),
                    "column {table}.{column} matches {part:?}"
                );
            }
            assert!(
                lower != "name" && lower != "user_name",
                "column {table}.{column} is a personal name column"
            );
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT \"{column}\" FROM \"{table}\" WHERE typeof(\"{column}\") = 'text'"
                ))
                .unwrap();
            let values: Vec<String> = stmt
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            for value in values {
                // A pseudonym is 16 random hex characters: about one in 30
                // holds 11 digits in a row, which is not an INSZ.
                assert!(
                    is_pseudonym(&value) || !has_digit_run(&value, 11),
                    "{table}.{column} holds 11 digits: {value:?}"
                );
                assert!(
                    !value.contains('@'),
                    "{table}.{column} holds an @: {value:?}"
                );
                for prefix in ["$2a$", "$2b$", "$2y$"] {
                    assert!(
                        !value.contains(prefix),
                        "{table}.{column} holds a bcrypt hash: {value:?}"
                    );
                }
                for secret in secrets {
                    assert!(
                        !value.contains(secret.as_str()),
                        "{table}.{column} holds a secret: {secret:?}"
                    );
                }
            }
        }
    }
    let bytes = std::fs::read(path).expect("read the file");
    for secret in secrets {
        let needle = secret.as_bytes();
        assert!(
            needle.is_empty() || !bytes.windows(needle.len()).any(|w| w == needle),
            "the file bytes contain a secret: {secret:?}"
        );
    }
}
