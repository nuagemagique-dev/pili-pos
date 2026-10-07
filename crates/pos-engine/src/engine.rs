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
}

impl Default for EngineTimers {
    fn default() -> Self {
        Self {
            auto_z: Duration::from_secs(60),
            mail_outbox: crate::commands::delivery::OUTBOX_INTERVAL,
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
}

impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("paths", &self.paths)
            .field("config", &self.config)
            .field("timers", &self.timers)
            .finish_non_exhaustive()
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
}

/// Passes the background loops have completed since the engine started.
#[derive(Debug, Default)]
pub struct BackgroundTicks {
    pub auto_z: AtomicU64,
    pub mail_outbox: AtomicU64,
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
        } = cfg;
        let db_path = paths.db.clone();
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

        let config = config.load()?;
        for problem in pos_fiscal::fdm_client::FdmClient::config_problems(&config) {
            tracing::error!(%problem, "pos.json: the FDM will refuse this setting");
        }
        let mail = crate::commands::delivery::MailState::from_config(&config);
        #[cfg(feature = "hubrise")]
        let hubrise_on =
            pos_db::queries::integrations::hubrise_at_start(&conn, config.hubrise_enabled)?;
        #[cfg(feature = "hubrise")]
        tracing::info!(enabled = hubrise_on, "hubrise integration switch");

        #[cfg(feature = "hubrise")]
        let shared_http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        Ok(Engine {
            state: Arc::new(AppState {
                paths,
                db: Mutex::new(conn),
                #[cfg(feature = "hubrise")]
                poller_db: Arc::new(Mutex::new(poller_conn)),
                config: Mutex::new(config),
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
                support: crate::commands::support::SupportState::installed(),
                pin_throttle: Default::default(),
                signing: Default::default(),
            }),
            tasks: Mutex::new(Vec::new()),
            ticks: Arc::new(BackgroundTicks::default()),
        })
    }

    /// The shared state, for the background tasks that outlive a call.
    pub fn state(&self) -> &Arc<AppState> {
        &self.state
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
    fn the_state_carries_the_paths_and_derefs() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(test_config(dir.path())).unwrap();
        assert_eq!(engine.paths, EnginePaths::under(dir.path().to_path_buf()));
        assert_eq!(engine.state().paths, engine.paths);
        assert_eq!(engine.config.lock().unwrap().pos_id, "CFOD0061234567");
    }

    #[test]
    fn open_with_a_broken_config_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = test_config(dir.path());
        cfg.config = ConfigSource::File(write(dir.path(), "pos.json", "not json"));
        assert!(Engine::open(cfg).is_err());
    }

    #[test]
    fn default_timers_are_the_historical_intervals() {
        let t = EngineTimers::default();
        assert_eq!(t.auto_z, Duration::from_secs(60));
        assert_eq!(t.mail_outbox, crate::commands::delivery::OUTBOX_INTERVAL);
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
