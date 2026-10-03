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
/// traffic - cloning it is cheap (internal Arc) and enables HTTP
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
    /// Wakes the HubRise poller for an early pass (HubRise only).
    #[cfg(feature = "hubrise")]
    pub hubrise_poll_now: Arc<tokio::sync::Notify>,
    /// Wakes the HubRise outbox: a change to send now (HubRise only).
    #[cfg(feature = "hubrise")]
    pub hubrise_outbox_wake: Arc<tokio::sync::Notify>,
    /// Whether the HubRise integration runs (`commands::integrations`).
    #[cfg(feature = "hubrise")]
    pub hubrise_switch: pos_hubrise::switch::Switch,
    pub shutdown_token: CancellationToken,
    /// Digital ticket email client (disabled when not configured).
    pub mail: crate::commands::delivery::MailState,
    /// The logged-in operator (`commands::access`): set by `login`,
    /// cleared by `logout`, `None` at start-up.
    pub session: Mutex<Option<pos_core::user::User>>,
    /// Remote support program and its running session
    /// (`commands::support`).
    pub support: crate::commands::support::SupportState,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        let db_path = crate::paths::db_path();
        tracing::info!(path = %db_path.display(), "opening database");

        std::fs::create_dir_all(db_path.parent().unwrap())?;

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

        let config = Self::load_config()?;
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

        Ok(Self {
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
            mail,
            session: Mutex::new(None),
            support: crate::commands::support::SupportState::installed(),
        })
    }

    /// Resolve the POS configuration, in priority order:
    ///
    /// 1. `PILI_POS_CONFIG` env var - explicit path; any load error
    ///    is fatal.
    /// 2. `/etc/pili-pos/pos.json` - the production path documented
    ///    in the installation manual. If the file exists, a parse
    ///    error is fatal: a fiscal terminal must never boot with a
    ///    partial or wrong identity.
    /// 3. Development fallback - hardcoded test identity, warned
    ///    loudly so it can't be mistaken for a production setup.
    fn load_config() -> anyhow::Result<PosConfig> {
        load_config_from(
            std::env::var("PILI_POS_CONFIG").ok().as_deref(),
            std::path::Path::new(crate::paths::PRODUCTION_CONFIG),
        )
    }
}

/// Crash recovery at boot: close orders whose sale (event N) is in the
/// archive but which are still `open`. Checkout archives the event and
/// closes the order in one transaction, so this only repairs databases
/// written by older versions. Orders with only P events (a table still
/// being served) are left open. Returns the number of orders closed.
pub fn recover_orphaned_checkouts(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE orders SET status = 'closed', closed_at = datetime('now')
         WHERE status = 'open'
           AND id IN (
             SELECT DISTINCT order_id FROM fiscal_events
             WHERE event_type = 'N' AND order_id IS NOT NULL
           )",
        [],
    )
}

/// [`AppState::load_config`] with its inputs made explicit, for tests:
/// `env_path` is `PILI_POS_CONFIG`, `production_path` the file of the
/// installation manual.
pub fn load_config_from(
    env_path: Option<&str>,
    production_path: &std::path::Path,
) -> anyhow::Result<PosConfig> {
    if let Some(path) = env_path {
        let config = PosConfig::load_from_file(std::path::Path::new(path))?;
        tracing::info!(%path, "config loaded from PILI_POS_CONFIG");
        return Ok(config);
    }

    if production_path.exists() {
        let config = PosConfig::load_from_file(production_path)?;
        tracing::info!(path = %production_path.display(), "config loaded");
        return Ok(config);
    }

    tracing::warn!(
        "no config file found (PILI_POS_CONFIG unset, /etc/pili-pos/pos.json absent) - \
         using development defaults with a TEST identity"
    );
    Ok(PosConfig {
        pos_id: "CFOD0061234567".to_string(),
        terminal_id: "POS1".to_string(),
        device_id: "00:1b:44:11:3a:b7".to_string(),
        restaurant_name: "Restaurant Exemple".to_string(),
        restaurant_address: "Rue de l'Exemple 1, 4000 Liège".to_string(),
        restaurant_vat: "BE0499999960".to_string(),
        restaurant_est_no: "2123456791".to_string(),
        fdm_url: "http://localhost:8081/graphql".to_string(),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        conn
    }

    fn order(conn: &Connection, status: &str) -> i64 {
        conn.execute(
            "INSERT INTO orders (order_type, status, user_insz) VALUES ('dine_in', ?1, '00000000029')",
            [status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn event(conn: &Connection, seq: i64, event_type: &str, order_id: Option<i64>) {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, order_id, pos_id, terminal_id, user_insz, event_data_json)
             VALUES (?1, ?2, ?3, 'P', 'T1', '00000000029', '{}')",
            rusqlite::params![event_type, seq, order_id],
        )
        .unwrap();
    }

    fn status(conn: &Connection, id: i64) -> String {
        conn.query_row("SELECT status FROM orders WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap()
    }

    #[test]
    fn an_open_order_with_a_sale_is_closed() {
        let conn = db();
        let id = order(&conn, "open");
        event(&conn, 1, "N", Some(id));
        assert_eq!(recover_orphaned_checkouts(&conn).unwrap(), 1);
        assert_eq!(status(&conn, id), "closed");
    }

    #[test]
    fn a_table_with_only_order_events_stays_open() {
        let conn = db();
        let id = order(&conn, "open");
        event(&conn, 1, "P", Some(id));
        assert_eq!(recover_orphaned_checkouts(&conn).unwrap(), 0);
        assert_eq!(status(&conn, id), "open");
    }

    #[test]
    fn an_order_without_events_stays_open() {
        let conn = db();
        let id = order(&conn, "open");
        assert_eq!(recover_orphaned_checkouts(&conn).unwrap(), 0);
        assert_eq!(status(&conn, id), "open");
    }

    #[test]
    fn closed_and_voided_orders_are_untouched() {
        let conn = db();
        let closed = order(&conn, "closed");
        let voided = order(&conn, "voided");
        event(&conn, 1, "N", Some(closed));
        event(&conn, 2, "N", Some(voided));
        assert_eq!(recover_orphaned_checkouts(&conn).unwrap(), 0);
        assert_eq!(status(&conn, voided), "voided");
    }

    #[test]
    fn recovery_is_idempotent() {
        let conn = db();
        let id = order(&conn, "open");
        event(&conn, 1, "N", Some(id));
        recover_orphaned_checkouts(&conn).unwrap();
        assert_eq!(recover_orphaned_checkouts(&conn).unwrap(), 0);
    }

    fn write(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, body).unwrap();
        p
    }

    const VALID: &str = r#"{
        "pos_id": "CFOD0061234567", "terminal_id": "POS1", "device_id": "aa:bb:cc:dd:ee:ff",
        "restaurant_name": "R", "restaurant_address": "A", "restaurant_vat": "BE0499999960",
        "restaurant_est_no": "2123456791", "fdm_url": "http://192.168.1.20/graphql"
    }"#;

    #[test]
    fn the_env_path_wins_over_the_production_file() {
        let dir = tempfile::tempdir().unwrap();
        let env = write(dir.path(), "env.json", VALID);
        let prod = write(dir.path(), "pos.json", "not json");
        let c = load_config_from(Some(env.to_str().unwrap()), &prod).unwrap();
        assert_eq!(c.device_id, "aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn a_bad_env_path_is_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        assert!(load_config_from(Some(missing.to_str().unwrap()), &missing).is_err());
    }

    #[test]
    fn the_production_file_is_used_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let prod = write(dir.path(), "pos.json", VALID);
        let c = load_config_from(None, &prod).unwrap();
        assert_eq!(c.restaurant_est_no, "2123456791");
    }

    #[test]
    fn a_broken_production_file_is_fatal_never_a_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let prod = write(dir.path(), "pos.json", r#"{"pos_id": "CFOD0061234567"}"#);
        assert!(load_config_from(None, &prod).is_err());
    }

    #[test]
    fn without_any_file_the_dev_identity_is_a_test_one() {
        let dir = tempfile::tempdir().unwrap();
        let c = load_config_from(None, &dir.path().join("absent.json")).unwrap();
        assert_eq!(c.pos_id, "CFOD0061234567");
        assert!(c.digital_ticket.is_none());
        assert!(c.printer_address.is_none());
        assert_eq!(
            c.problems(),
            Vec::<String>::new(),
            "the dev identity is one the FDM accepts"
        );
    }

    #[test]
    fn the_documented_example_is_a_usable_identity() {
        let dir = tempfile::tempdir().unwrap();
        let prod = write(dir.path(), "pos.json", VALID);
        let c = load_config_from(None, &prod).unwrap();
        assert_eq!(c.problems(), Vec::<String>::new());
    }
}
