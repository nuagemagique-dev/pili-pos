use std::sync::Mutex;
#[cfg(feature = "hubrise")]
use std::sync::Arc;

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
    pub booking_period_id: Mutex<String>,
    #[cfg(feature = "hubrise")]
    pub shared_http: reqwest::Client,
    pub shutdown_token: CancellationToken,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let db_path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("moyun-pos")
            .join("pos.db");

        std::fs::create_dir_all(db_path.parent().unwrap())?;

        let conn = Connection::open(&db_path)?;
        schema::run_migrations(&conn)?;

        // Crash recovery: close orders that were fiscally signed
        // (N-event archived) but left `open` because a power cut
        // happened between FDM archive and the status UPDATE in
        // checkout.rs. Without this, orphaned orders would stay
        // "open" forever and pollute the tables view.
        let recovered = conn.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now')
             WHERE status = 'open'
               AND id IN (
                 SELECT DISTINCT order_id FROM fiscal_events
                 WHERE event_type = 'N' AND order_id IS NOT NULL
               )",
            [],
        ).unwrap_or(0);
        if recovered > 0 {
            tracing::warn!(recovered, "boot: recovered orphaned checkout(s)");
        }

        // Dedicated background-poller connection to the same WAL-mode
        // SQLite file (pos-db enables WAL in run_migrations). HubRise only.
        #[cfg(feature = "hubrise")]
        let poller_conn = Connection::open(&db_path)?;

        // Default config — in production, load from config file or DB
        let config = PosConfig {
            pos_id: "CFOD0061234567".to_string(),
            terminal_id: "POS1".to_string(),
            device_id: "DEV001".to_string(),
            restaurant_name: "Tokyo Sushi Bar".to_string(),
            restaurant_address: "Rue Test 1, 1000 Bruxelles".to_string(),
            restaurant_vat: "BE0499999960".to_string(),
            fdm_url: "http://localhost:8081/graphql".to_string(),
            ..Default::default()
        };

        let booking_period_id = uuid::Uuid::new_v4().to_string();

        #[cfg(feature = "hubrise")]
        let shared_http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;

        Ok(Self {
            db: Mutex::new(conn),
            #[cfg(feature = "hubrise")]
            poller_db: Arc::new(Mutex::new(poller_conn)),
            config: Mutex::new(config),
            booking_period_id: Mutex::new(booking_period_id),
            #[cfg(feature = "hubrise")]
            shared_http,
            shutdown_token: CancellationToken::new(),
        })
    }
}
