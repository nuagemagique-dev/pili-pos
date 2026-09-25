#[cfg(feature = "hubrise")]
use std::sync::Arc;
use std::sync::Mutex;

use pos_core::config::PosConfig;
use pos_db::schema;
use rusqlite::Connection;
use tokio_util::sync::CancellationToken;

/// Shared application state managed by Tauri.
///
/// `db` is the connection used by UI command handlers; `poller_db`
/// is a separate connection dedicated to the background HubRise
/// poller so the two paths never contend on the same mutex.
///
/// `shared_http` is the single `reqwest::Client` used for all HubRise
/// traffic — cloning it is cheap (internal Arc) and enables HTTP
/// keep-alive across calls.
///
/// `shutdown_token` is cancelled when the Tauri window closes; the
/// poller task observes it via `tokio::select!` to exit gracefully.
///
/// `poller_db` and `shared_http` are gated behind the `hubrise` feature
/// because they only exist to serve the webshop integration; the
/// certification build (which excludes hubrise) doesn't need them.
pub struct AppState {
    pub db: Mutex<Connection>,
    #[cfg(feature = "hubrise")]
    pub poller_db: Arc<Mutex<Connection>>,
    pub config: Mutex<PosConfig>,
    /// Current booking period ID (UUID, set on first sale of the day / shift).
    #[cfg(feature = "hubrise")]
    pub shared_http: reqwest::Client,
    pub shutdown_token: CancellationToken,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let db_path = crate::paths::db_path();
        tracing::info!(path = %db_path.display(), "opening database");

        std::fs::create_dir_all(db_path.parent().unwrap())?;

        let conn = Connection::open(&db_path)?;
        schema::run_migrations(&conn)?;

        // Crash recovery: close orders that were fiscally signed
        // (N-event archived) but left `open` because a power cut
        // happened between FDM archive and the status UPDATE in
        // checkout.rs. Without this, orphaned orders would stay
        // "open" forever and pollute the tables view.
        let recovered = conn
            .execute(
                "UPDATE orders SET status = 'closed', closed_at = datetime('now')
             WHERE status = 'open'
               AND id IN (
                 SELECT DISTINCT order_id FROM fiscal_events
                 WHERE event_type = 'N' AND order_id IS NOT NULL
               )",
                [],
            )
            .unwrap_or(0);
        if recovered > 0 {
            tracing::warn!(recovered, "boot: recovered orphaned checkout(s)");
        }

        // Dedicated background-poller connection to the same WAL-mode
        // SQLite file (pos-db enables WAL in run_migrations). HubRise only.
        #[cfg(feature = "hubrise")]
        let poller_conn = Connection::open(&db_path)?;

        let config = Self::load_config()?;

        #[cfg(feature = "hubrise")]
        let shared_http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        Ok(Self {
            db: Mutex::new(conn),
            #[cfg(feature = "hubrise")]
            poller_db: Arc::new(Mutex::new(poller_conn)),
            config: Mutex::new(config),
            #[cfg(feature = "hubrise")]
            shared_http,
            shutdown_token: CancellationToken::new(),
        })
    }

    /// Resolve the POS configuration, in priority order:
    ///
    /// 1. `MOYUN_POS_CONFIG` env var — explicit path; any load error
    ///    is fatal.
    /// 2. `/etc/moyun-pos/pos.json` — the production path documented
    ///    in the installation manual. If the file exists, a parse
    ///    error is fatal: a fiscal terminal must never boot with a
    ///    partial or wrong identity.
    /// 3. Development fallback — hardcoded test identity, warned
    ///    loudly so it can't be mistaken for a production setup.
    fn load_config() -> anyhow::Result<PosConfig> {
        if let Ok(path) = std::env::var("MOYUN_POS_CONFIG") {
            let config = PosConfig::load_from_file(std::path::Path::new(&path))?;
            tracing::info!(%path, "config loaded from MOYUN_POS_CONFIG");
            return Ok(config);
        }

        let production_path = std::path::Path::new("/etc/moyun-pos/pos.json");
        if production_path.exists() {
            let config = PosConfig::load_from_file(production_path)?;
            tracing::info!("config loaded from /etc/moyun-pos/pos.json");
            return Ok(config);
        }

        tracing::warn!(
            "no config file found (MOYUN_POS_CONFIG unset, /etc/moyun-pos/pos.json absent) — \
             using development defaults with a TEST identity"
        );
        Ok(PosConfig {
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            device_id: "DEV001".to_string(),
            restaurant_name: "Restaurant Exemple".to_string(),
            restaurant_address: "Rue de l'Exemple 1, 4000 Liège".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            fdm_url: "http://localhost:8081/graphql".to_string(),
            ..Default::default()
        })
    }
}
