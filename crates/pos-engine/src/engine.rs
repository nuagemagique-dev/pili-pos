//! The engine: the fiscal core and its state, opened from an
//! [`EngineConfig`] that says where the data lives and where the
//! configuration comes from. The shell (the Tauri binary today) builds
//! that value from its environment and hands it to [`Engine::open`]; the
//! engine itself reads neither the process environment nor fixed paths.

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_core::config::PosConfig;
use pos_db::schema;
use rusqlite::Connection;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use pos_fiscal::http_tls::HttpTls;
use pos_fiscal::identity::SoftwareIdentity;

use crate::events::EventSink;
use crate::state::{AppState, recover_orphaned_checkouts};

/// Where the engine keeps its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnginePaths {
    /// Directory holding the database and the backups.
    pub data_dir: PathBuf,
    /// The SQLite database file.
    pub db: PathBuf,
    /// Default directory of the database backups (`backup_dir` of the
    /// configuration overrides it).
    pub backups: PathBuf,
}

impl EnginePaths {
    /// The layout under one data directory: `pos.db` and `backups/`.
    pub fn under(data_dir: PathBuf) -> Self {
        Self {
            db: data_dir.join("pos.db"),
            backups: data_dir.join("backups"),
            data_dir,
        }
    }
}

/// Where the configuration comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    /// A configuration file; any error loading it is fatal.
    File(PathBuf),
    /// A configuration bundle: the file is loaded like [`ConfigSource::File`],
    /// then every relative file or directory path in it is resolved
    /// against the directory holding the file (see
    /// [`PosConfig::resolve_relative_to`]). The Android provisioning
    /// bundle uses it; the Linux shell never does.
    Bundle(PathBuf),
    /// The hardcoded TEST identity of a development machine.
    DevelopmentDefaults,
}

impl ConfigSource {
    /// Read the configuration. A file that is missing, unreadable or
    /// wrong is an error: a fiscal terminal must never boot with a
    /// partial or wrong identity.
    pub fn load(&self) -> anyhow::Result<PosConfig> {
        match self {
            ConfigSource::File(path) => Ok(PosConfig::load_from_file(path)?),
            ConfigSource::Bundle(path) => {
                let mut config = PosConfig::load_from_file(path)?;
                config.resolve_relative_to(path.parent().unwrap_or(Path::new("")));
                Ok(config)
            }
            ConfigSource::DevelopmentDefaults => Ok(development_defaults()),
        }
    }
}

/// The development identity: a TEST one, never a production setup.
fn development_defaults() -> PosConfig {
    PosConfig {
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "00:1b:44:11:3a:b7".to_string(),
        restaurant_name: "Restaurant Exemple".to_string(),
        restaurant_address: "Rue de l'Exemple 1, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456791".to_string(),
        fdm_url: "http://localhost:8081/graphql".to_string(),
        ..Default::default()
    }
}

/// Choose the configuration source, in priority order:
///
/// 1. `env_path` (`PILI_POS_CONFIG`) - explicit path; any load error
///    is fatal.
/// 2. `production_path` (`/etc/pili-pos/pos.json`, the path of the
///    installation manual) when the file exists. A parse error is
///    fatal: a fiscal terminal must never boot with a partial or wrong
///    identity.
/// 3. Development defaults - a hardcoded TEST identity, warned loudly
///    so it can't be mistaken for a production setup.
pub fn config_source_from(env_path: Option<&str>, production_path: &Path) -> ConfigSource {
    if let Some(path) = env_path {
        tracing::info!(%path, "config loaded from PILI_POS_CONFIG");
        return ConfigSource::File(PathBuf::from(path));
    }

    if production_path.exists() {
        tracing::info!(path = %production_path.display(), "config loaded");
        return ConfigSource::File(production_path.to_path_buf());
    }

    tracing::warn!(
        "no config file found (PILI_POS_CONFIG unset, /etc/pili-pos/pos.json absent) - \
         using development defaults with a TEST identity"
    );
    ConfigSource::DevelopmentDefaults
}

/// The intervals of the background loops (`crate::background`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineTimers {
    /// Automatic Z of a business day left open: checked at start-up,
    /// then at this interval.
    pub auto_z: Duration,
    /// Digital ticket outbox: queued emails retried at this interval.
    pub mail_outbox: Duration,
    /// How long to wait for each DLE EOT answer of the ticket printer
    /// after a print (PD37). Linux 500 ms (the default); Android about
    /// 2 s, where the answer often arrives later. A late answer leaves
    /// `PrinterStatus::status_known` false.
    pub printer_status_read: Duration,
    /// How long a session may sit without user activity before the
    /// engine locks it. An unattended terminal must not be used under
    /// the previous operator's INSZ (AM: every fiscal event carries the
    /// INSZ of the person who made it). Restarted by `user_activity` and
    /// by every login (planner decision PD35).
    pub idle_lock: Duration,
    /// How often the FDM monitor checks the FDM status while a session
    /// is open and the FDM answers (planner decision PD36). While it does
    /// not answer the wait doubles with each failed check up to 10 times
    /// this value (30 s, 60, 120, 240, then 300 s). A session that opens
    /// is checked at once, and no check is made without a session.
    pub fdm_poll: Duration,
    /// Nightly export: an export pass (prepare the exports of the new Z
    /// reports, apply the retention, upload what is due) runs at this
    /// interval, and at once after a real Z. Nothing runs without an
    /// `export` section in the configuration.
    pub export_outbox: Duration,
    /// Peppol delivery queue: due deliveries are sent at this interval, and at
    /// once after an invoice is signed or a parked delivery re-queued. Nothing
    /// runs without a `peppol` section in the configuration.
    pub peppol_outbox: Duration,
}

impl Default for EngineTimers {
    fn default() -> Self {
        Self {
            auto_z: Duration::from_secs(60),
            mail_outbox: crate::commands::delivery::OUTBOX_INTERVAL,
            printer_status_read: pos_print::transport::DEFAULT_STATUS_READ_TIMEOUT,
            idle_lock: Duration::from_secs(300),
            fdm_poll: Duration::from_secs(30),
            export_outbox: Duration::from_secs(60),
            peppol_outbox: Duration::from_secs(60),
        }
    }
}

/// Everything [`Engine::open`] needs.
#[derive(Clone)]
pub struct EngineConfig {
    pub paths: EnginePaths,
    pub config: ConfigSource,
    /// Where the engine sends its events (the shell forwards them).
    pub events: Arc<dyn EventSink>,
    pub timers: EngineTimers,
    /// The `posSwVersion` of every event and ticket (for example
    /// `pili-pos/1.0.0`), built by the shell that knows its product and
    /// version. Required: there is no default.
    pub identity: SoftwareIdentity,
    /// Whom the engine's HTTPS clients trust. The shell chooses; the
    /// engine never infers it from the operating system. An empty
    /// `RootsOnly` is refused by [`Engine::open`].
    pub http_tls: HttpTls,
    /// Replaces the `device_id` of the configuration after it is loaded
    /// (the POS spec `deviceId`, 1 to 600 characters once trimmed; an
    /// empty or longer value is an [`Engine::open`] error). Android
    /// passes the identifier of the handset; Linux passes `None` and
    /// keeps the value of `pos.json`.
    pub device_id_override: Option<String>,
    /// The remote support program (RustDesk) when this terminal has one;
    /// the shell decides, the engine never looks for it. `None`: the
    /// support commands answer as on a terminal without it (not
    /// available, and a start is refused).
    pub support_program: Option<PathBuf>,
}

impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("paths", &self.paths)
            .field("config", &self.config)
            .field("timers", &self.timers)
            .field("identity", &self.identity)
            .field("http_tls", &HttpTlsKind(&self.http_tls))
            .field("device_id_override", &self.device_id_override)
            .field("support_program", &self.support_program)
            .finish_non_exhaustive()
    }
}

/// How an [`HttpTls`] shows in `Debug` output: the mode and the number
/// of roots, never the certificates.
struct HttpTlsKind<'a>(&'a HttpTls);

impl std::fmt::Debug for HttpTlsKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            HttpTls::Platform => f.write_str("Platform"),
            HttpTls::RootsOnly(ders) => write!(f, "RootsOnly({} roots)", ders.len()),
        }
    }
}

/// The opened engine: the shared [`AppState`] behind a cheap handle.
/// It derefs to the state, so command bodies take `&AppState` as before.
pub struct Engine {
    state: Arc<AppState>,
    /// The background tasks [`Engine::start_background`] spawned.
    pub(crate) tasks: Mutex<Vec<JoinHandle<()>>>,
    /// How often each background pass has run (for tests).
    pub(crate) ticks: Arc<BackgroundTicks>,
    /// The exclusive lock on `engine.lock` of the data directory, held
    /// for the engine's lifetime: closing the file releases it.
    _lock: std::fs::File,
}

/// Passes the background loops have completed since the engine started.
#[derive(Debug, Default)]
pub struct BackgroundTicks {
    pub auto_z: AtomicU64,
    pub mail_outbox: AtomicU64,
    pub export_outbox: AtomicU64,
    pub peppol_outbox: AtomicU64,
}

impl Engine {
    /// Open the database, migrate it, repair what a crash left behind
    /// and load the configuration.
    pub fn open(cfg: EngineConfig) -> anyhow::Result<Engine> {
        let EngineConfig {
            paths,
            config,
            events,
            timers,
            identity,
            http_tls,
            device_id_override,
            support_program,
        } = cfg;
        // Before anything is created or opened: a trust setting that
        // cannot make a client is a refusal, not a later panic.
        http_tls
            .check()
            .map_err(|e| anyhow::anyhow!("HTTPS trust (http_tls): {e}"))?;
        let device_id_override = device_id_override
            .map(|id| validated_device_id(&id))
            .transpose()?;
        let db_path = paths.db.clone();
        let lock = lock_data_dir(&paths.data_dir)?;
        tracing::info!(path = %db_path.display(), "opening database");

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        schema::run_migrations(&conn)?;

        let recovered = recover_orphaned_checkouts(&conn).unwrap_or(0);
        if recovered > 0 {
            tracing::warn!(recovered, "boot: recovered orphaned checkout(s)");
        }
        // Orders a session left empty (a crash on a fresh table): not to
        // block the SPF export, an update or a menu refresh.
        match pos_db::queries::orders::void_all_empty(&conn) {
            Ok(0) => {}
            Ok(n) => tracing::info!(voided = n, "boot: voided empty open order(s)"),
            Err(e) => tracing::warn!(%e, "boot: empty open orders not voided"),
        }
        // Card payments whose answer was lost with a crash.
        match crate::commands::eft::recover_after_crash(&conn) {
            Ok(0) => {}
            Ok(n) => tracing::warn!(
                n,
                "boot: card payment(s) left without an answer, to be checked"
            ),
            Err(e) => tracing::error!(%e, "boot: card payment recovery failed"),
        }
        // Online orders a crash left half-signed.
        #[cfg(feature = "hubrise")]
        match crate::commands::webshop::recover_webshop_orders(&conn) {
            Ok(0) => {}
            Ok(n) => tracing::warn!(n, "boot: recovered online order(s) left signing"),
            Err(e) => tracing::error!(%e, "boot: online order recovery failed"),
        }

        // Dedicated background-poller connection to the same WAL-mode
        // SQLite file (pos-db enables WAL in run_migrations). HubRise only.
        #[cfg(feature = "hubrise")]
        let poller_conn = Connection::open(&db_path)?;

        let mut config = config.load()?;
        if let Some(device_id) = device_id_override {
            config.device_id = device_id;
        }
        for problem in pos_fiscal::fdm_client::FdmClient::config_problems(&config) {
            tracing::error!(%problem, "pos.json: the FDM will refuse this setting");
        }
        let mail = crate::commands::delivery::MailState::from_config(&config, &http_tls);
        // A bad `export` section turns the export off and shows in its
        // status; it never stops the engine from opening.
        let export = crate::commands::export::ExportState::from_config(&config, &http_tls);
        if export.is_enabled() {
            let exports_dir = paths.data_dir.join("exports");
            if let Err(e) = pos_export::outbox::clean_tmp(&exports_dir) {
                tracing::warn!(%e, "export: cannot empty the work area");
            }
        }
        let table_plan = pos_db::queries::table_plan::at_start(&conn, &config.table_plan())?;
        tracing::info!(%table_plan, "table plan");
        #[cfg(feature = "hubrise")]
        let hubrise_on =
            pos_db::queries::integrations::hubrise_at_start(&conn, config.hubrise_enabled)?;
        #[cfg(feature = "hubrise")]
        tracing::info!(enabled = hubrise_on, "hubrise integration switch");

        #[cfg(feature = "hubrise")]
        let shared_http = http_tls
            .client_builder()?
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        Ok(Engine {
            state: Arc::new(AppState {
                paths,
                db: Mutex::new(conn),
                #[cfg(feature = "hubrise")]
                poller_db: Arc::new(Mutex::new(poller_conn)),
                config: Mutex::new(config),
                identity,
                http_tls,
                #[cfg(feature = "hubrise")]
                shared_http,
                #[cfg(feature = "hubrise")]
                hubrise_poll_now: Arc::new(tokio::sync::Notify::new()),
                #[cfg(feature = "hubrise")]
                hubrise_outbox_wake: Arc::new(tokio::sync::Notify::new()),
                #[cfg(feature = "hubrise")]
                hubrise_switch: pos_hubrise::switch::Switch::new(hubrise_on),
                shutdown_token: CancellationToken::new(),
                events,
                timers,
                mail,
                session: Mutex::new(None),
                support: crate::commands::support::SupportState::with_program(support_program),
                pin_throttle: Default::default(),
                signing: Default::default(),
                fdm_status_cache: Mutex::new(None),
                session_rules: Default::default(),
                fdm_monitor: Default::default(),
                export,
                peppol_wake: tokio::sync::Notify::new(),
                eft_in_flight: Mutex::new(None),
                eft_line: tokio::sync::Mutex::new(()),
                #[cfg(feature = "eft-scripted")]
                eft_terminal_override: None,
            }),
            tasks: Mutex::new(Vec::new()),
            ticks: Arc::new(BackgroundTicks::default()),
            _lock: lock,
        })
    }

    /// The shared state, for the background tasks that outlive a call.
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
    }
}

/// The longest `deviceId` the POS spec accepts.
const DEVICE_ID_MAX_CHARS: usize = 600;

/// An overriding `deviceId`, trimmed: 1 to 600 characters.
fn validated_device_id(raw: &str) -> anyhow::Result<String> {
    let id = raw.trim();
    anyhow::ensure!(!id.is_empty(), "device_id_override: must not be empty");
    anyhow::ensure!(
        id.chars().count() <= DEVICE_ID_MAX_CHARS,
        "device_id_override: at most {DEVICE_ID_MAX_CHARS} characters expected"
    );
    Ok(id.to_string())
}

/// Name of the lock file in the data directory.
pub const LOCK_FILE: &str = "engine.lock";

/// Create the data directory if needed and take an exclusive lock on its
/// `engine.lock`, before anything touches the database: two engines on
/// one data directory would sign events with the same counters. The file
/// stays empty and holds no data; the lock lives as long as the returned
/// file stays open.
fn lock_data_dir(data_dir: &Path) -> anyhow::Result<std::fs::File> {
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join(LOCK_FILE);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(std::fs::TryLockError::WouldBlock) => {
            anyhow::bail!("engine already open: {}", path.display())
        }
        Err(std::fs::TryLockError::Error(e)) => {
            Err(anyhow::Error::new(e).context(format!("locking {}", path.display())))
        }
    }
}

impl Deref for Engine {
    type Target = AppState;

    fn deref(&self) -> &AppState {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one place a test builds an [`EngineConfig`]: the development
    /// identity on a data directory of the caller's.
    fn test_config(data_dir: &Path) -> EngineConfig {
        EngineConfig {
            paths: EnginePaths::under(data_dir.to_path_buf()),
            config: ConfigSource::DevelopmentDefaults,
            events: Arc::new(crate::events::NullSink),
            timers: EngineTimers::default(),
            identity: SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
            http_tls: HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        }
    }

    fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn open_creates_the_database_and_runs_the_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("fresh");
        let engine = Engine::open(test_config(&data)).unwrap();
        assert!(data.join("pos.db").is_file());
        let db = engine.db.lock().unwrap();
        let tables: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('orders', 'fiscal_events', 'users')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 3);
    }

    #[test]
    fn open_closes_an_orphaned_checkout() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_config(dir.path());
        {
            let conn = Connection::open(&cfg.paths.db).unwrap();
            schema::run_migrations(&conn).unwrap();
            conn.execute(
                "INSERT INTO orders (order_type, status, user_insz) VALUES ('dine_in', 'open', '00000000029')",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO fiscal_events (event_type, sequence_number, order_id, pos_id, terminal_id, user_insz, event_data_json)
                 VALUES ('N', 1, 1, 'P', 'T1', '00000000029', '{}')",
                [],
            )
            .unwrap();
        }
        let engine = Engine::open(cfg).unwrap();
        let status: String = engine
            .db
            .lock()
            .unwrap()
            .query_row("SELECT status FROM orders WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "closed");
    }

    #[test]
    fn a_second_open_on_one_data_dir_fails() {
        let dir = tempfile::tempdir().unwrap();
        let _first = Engine::open(test_config(dir.path())).unwrap();
        let err = Engine::open(test_config(dir.path()))
            .err()
            .expect("the data directory is locked");
        let msg = err.to_string();
        assert!(msg.contains("engine already open"), "{msg}");
        assert!(msg.contains("engine.lock"), "{msg}");
    }

    #[test]
    fn open_after_drop_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        drop(Engine::open(test_config(dir.path())).unwrap());
        Engine::open(test_config(dir.path())).unwrap();
    }

    #[test]
    fn two_data_dirs_open_together() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let _first = Engine::open(test_config(a.path())).unwrap();
        let _second = Engine::open(test_config(b.path())).unwrap();
    }

    #[test]
    fn the_lock_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let _engine = Engine::open(test_config(dir.path())).unwrap();
        let lock = std::fs::metadata(dir.path().join(LOCK_FILE)).unwrap();
        assert_eq!(lock.len(), 0);
    }

    #[test]
    fn the_state_carries_the_paths_and_derefs() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(test_config(dir.path())).unwrap();
        assert_eq!(engine.paths, EnginePaths::under(dir.path().to_path_buf()));
        assert_eq!(engine.state().paths, engine.paths);
        assert_eq!(engine.config.lock().unwrap().pos_id, "CFOD0061234567");
    }

    const GOOD_POS_JSON: &str = r#"{
        "pos_id": "CFOD0061234567", "terminal_id": "POS1", "device_id": "aa:bb:cc:dd:ee:ff",
        "restaurant_name": "R", "restaurant_address": "A", "restaurant_vat": "BE0499999960",
        "restaurant_est_no": "2123456791", "fdm_url": "http://192.168.1.20/graphql"
    }"#;

    /// The `deviceId` the engine puts in the common fields of an event.
    fn signed_device_id(engine: &Engine) -> String {
        let config = engine.config.lock().unwrap().clone();
        let booking = pos_fiscal::event::Booking {
            period_id: "bp".into(),
            date: "2026-01-01".into(),
        };
        pos_fiscal::event::build_common_fields(
            &config,
            &engine.identity,
            1,
            "00000000029",
            &booking,
            pos_fiscal::types::TicketMedium::None,
        )
        .device_id
    }

    #[test]
    fn the_common_fields_carry_the_pos_json_device_id_without_an_override() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.config = ConfigSource::File(write(dir.path(), "pos.json", GOOD_POS_JSON));
        let engine = Engine::open(cfg).unwrap();
        assert_eq!(signed_device_id(&engine), "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn the_common_fields_carry_the_device_id_override() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.config = ConfigSource::File(write(dir.path(), "pos.json", GOOD_POS_JSON));
        cfg.device_id_override = Some("  handset-0042 \n".into());
        let engine = Engine::open(cfg).unwrap();
        assert_eq!(signed_device_id(&engine), "handset-0042");
        assert_eq!(engine.config.lock().unwrap().device_id, "handset-0042");
    }

    #[test]
    fn the_override_replaces_the_development_identity_device_id_too() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.device_id_override = Some("x".repeat(600));
        let engine = Engine::open(cfg).unwrap();
        assert_eq!(signed_device_id(&engine), "x".repeat(600));
    }

    #[test]
    fn open_refuses_an_empty_or_too_long_device_id_override() {
        for bad in ["", "   ", "\t\n"] {
            let dir = tempfile::tempdir().unwrap();
            let mut cfg = test_config(dir.path());
            cfg.device_id_override = Some(bad.into());
            let err = Engine::open(cfg).err().expect("empty override");
            assert!(err.to_string().contains("device_id_override"), "{err}");
        }
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("fresh");
        let mut cfg = test_config(&data);
        cfg.device_id_override = Some("x".repeat(601));
        let err = Engine::open(cfg).err().expect("601 characters");
        assert!(err.to_string().contains("600"), "{err}");
        assert!(!data.exists(), "nothing was created");
    }

    #[test]
    fn the_support_program_reaches_the_state() {
        let dir = tempfile::tempdir().unwrap();
        let none = Engine::open(test_config(dir.path())).unwrap();
        assert_eq!(none.support.program(), None);
        let status = crate::commands::support::do_remote_support_running(&none).unwrap();
        assert!(!status.running);
        drop(none);

        let program = dir.path().join("AppRun");
        let mut cfg = test_config(dir.path());
        cfg.support_program = Some(program.clone());
        let engine = Engine::open(cfg).unwrap();
        assert_eq!(engine.support.program(), Some(program));
    }

    #[test]
    fn a_bundle_resolves_its_relative_paths_against_its_directory() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = dir.path().join("bundle");
        std::fs::create_dir_all(&bundle).unwrap();
        let json = GOOD_POS_JSON.replace(
            "\"fdm_url\": \"http://192.168.1.20/graphql\"",
            "\"fdm_url\": \"https://192.168.1.20/graphql\", \"fdm_token_file\": \"fdm.token\", \
             \"fdm_ca_file\": \"tls/ca.pem\", \"backup_dir\": \"/abs/backups\"",
        );
        let file = write(&bundle, "pos.json", &json);

        let config = ConfigSource::Bundle(file.clone()).load().unwrap();
        assert_eq!(
            config.fdm_token_file.as_deref(),
            Some(bundle.join("fdm.token").to_str().unwrap())
        );
        assert_eq!(
            config.fdm_ca_file.as_deref(),
            Some(bundle.join("tls/ca.pem").to_str().unwrap())
        );
        assert_eq!(config.backup_dir.as_deref(), Some("/abs/backups"));
        assert_eq!(config.fdm_url, "https://192.168.1.20/graphql");

        // A plain file keeps the values as written.
        let plain = ConfigSource::File(file.clone()).load().unwrap();
        assert_eq!(plain.fdm_token_file.as_deref(), Some("fdm.token"));

        // The engine opens on a bundle, with the resolved paths.
        let mut cfg = test_config(&dir.path().join("data"));
        cfg.config = ConfigSource::Bundle(file);
        let engine = Engine::open(cfg).unwrap();
        assert_eq!(
            engine.config.lock().unwrap().fdm_token_file.as_deref(),
            Some(bundle.join("fdm.token").to_str().unwrap())
        );
    }

    #[test]
    fn a_missing_bundle_is_an_open_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.config = ConfigSource::Bundle(dir.path().join("absent").join("pos.json"));
        assert!(Engine::open(cfg).is_err());
    }

    #[test]
    fn open_with_a_broken_config_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.config = ConfigSource::File(write(dir.path(), "pos.json", "not json"));
        assert!(Engine::open(cfg).is_err());
    }

    #[test]
    fn open_refuses_empty_roots_before_touching_the_data_directory() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("fresh");
        let mut cfg = test_config(&data);
        cfg.http_tls = HttpTls::RootsOnly(vec![]);
        let err = Engine::open(cfg).err().expect("no roots, no engine");
        let msg = err.to_string();
        assert!(msg.contains("no trusted root certificate"), "{msg}");
        assert!(!data.exists(), "nothing was created");
    }

    #[test]
    fn open_refuses_an_invalid_root() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.http_tls = HttpTls::RootsOnly(vec![b"not a certificate".to_vec()]);
        let err = Engine::open(cfg).err().expect("invalid root");
        assert!(
            err.to_string().contains("invalid root certificate"),
            "{err}"
        );
        assert!(!dir.path().join("pos.db").exists());
    }

    #[test]
    fn the_state_carries_the_identity_and_the_trust_it_was_given() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(test_config(dir.path())).unwrap();
        assert_eq!(engine.identity.as_str(), "pili-pos/1.0.0");
        assert_eq!(engine.http_tls, HttpTls::Platform);
    }

    #[test]
    fn default_timers_are_the_historical_intervals() {
        let t = EngineTimers::default();
        assert_eq!(t.auto_z, Duration::from_secs(60));
        assert_eq!(t.mail_outbox, crate::commands::delivery::OUTBOX_INTERVAL);
        assert_eq!(t.idle_lock, Duration::from_secs(300));
        assert_eq!(t.fdm_poll, Duration::from_secs(30));
        assert_eq!(t.export_outbox, Duration::from_secs(60));
        assert_eq!(t.peppol_outbox, Duration::from_secs(60));
    }

    #[test]
    fn engine_paths_layout_under_a_data_dir() {
        let paths = EnginePaths::under(PathBuf::from("/data/pili"));
        assert_eq!(paths.data_dir, PathBuf::from("/data/pili"));
        assert_eq!(paths.db, PathBuf::from("/data/pili/pos.db"));
        assert_eq!(paths.backups, PathBuf::from("/data/pili/backups"));
    }

    #[test]
    fn development_defaults_give_the_test_identity() {
        let c = ConfigSource::DevelopmentDefaults.load().unwrap();
        assert_eq!(c.pos_id, "CFOD0061234567");
        assert_eq!(c.terminal_id, "POS1");
        assert_eq!(c.restaurant_vat, "BE0499999960");
        assert_eq!(c.fdm_url, "http://localhost:8081/graphql");
    }

    #[test]
    fn a_file_source_with_a_broken_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let broken = write(dir.path(), "pos.json", r#"{"pos_id": "CFOD0061234567"}"#);
        assert!(ConfigSource::File(broken).load().is_err());
        assert!(
            ConfigSource::File(dir.path().join("missing.json"))
                .load()
                .is_err()
        );
    }

    #[test]
    fn config_source_from_picks_env_then_production_then_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let env = dir.path().join("env.json");
        let prod = write(dir.path(), "pos.json", "{}");
        assert_eq!(
            config_source_from(Some(env.to_str().unwrap()), &prod),
            ConfigSource::File(env)
        );
        assert_eq!(
            config_source_from(None, &prod),
            ConfigSource::File(prod.clone())
        );
        assert_eq!(
            config_source_from(None, &dir.path().join("absent.json")),
            ConfigSource::DevelopmentDefaults
        );
    }
}
