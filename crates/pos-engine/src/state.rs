#[cfg(feature = "hubrise")]
use std::sync::Arc;
use std::sync::Mutex;

use pos_core::config::PosConfig;
use rusqlite::Connection;
use tokio_util::sync::CancellationToken;

/// Shared application state, owned by [`crate::engine::Engine`].
///
/// `db` is the connection used by UI command handlers; `poller_db`
/// is a separate connection dedicated to the background HubRise
/// poller so the two paths never contend on the same mutex.
///
/// `shared_http` is the single `reqwest::Client` used for all HubRise
/// traffic - cloning it is cheap (internal Arc) and enables HTTP
/// keep-alive across calls.
///
/// `shutdown_token` is cancelled by [`crate::engine::Engine::shutdown`]; the
/// poller task observes it via `tokio::select!` to exit gracefully.
///
/// `poller_db` and `shared_http` are gated behind the `hubrise` feature
/// because they only exist to serve the webshop integration. The
/// certification build includes the feature; each installation
/// switches the integration on or off at run time.
pub struct AppState {
    /// Where the engine keeps its data (database, backups).
    pub paths: crate::engine::EnginePaths,
    pub db: Mutex<Connection>,
    #[cfg(feature = "hubrise")]
    pub poller_db: Arc<Mutex<Connection>>,
    pub config: Mutex<PosConfig>,
    /// The `posSwVersion` of every event and ticket, given by the shell
    /// ([`crate::engine::EngineConfig::identity`]).
    pub identity: pos_fiscal::identity::SoftwareIdentity,
    /// Whom the engine's HTTPS clients trust
    /// ([`crate::engine::EngineConfig::http_tls`]); checked at open.
    pub http_tls: pos_fiscal::http_tls::HttpTls,
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
    /// Where the engine sends its events (`crate::events`).
    pub events: std::sync::Arc<dyn crate::events::EventSink>,
    /// The intervals of the background loops (`crate::background`).
    pub timers: crate::engine::EngineTimers,
    /// Digital ticket email client (disabled when not configured).
    pub mail: crate::commands::delivery::MailState,
    /// The logged-in operator (`commands::access`): set by `login`,
    /// cleared by `logout`, `None` at start-up.
    pub session: Mutex<Option<pos_core::user::User>>,
    /// Remote support program and its running session
    /// (`commands::support`).
    pub support: crate::commands::support::SupportState,
    /// Wrong PIN attempts (`commands::access::verify_pin`).
    pub pin_throttle: crate::commands::access::PinThrottle,
    /// Held while a fiscal event is signed (`commands::signing`).
    pub signing: crate::commands::signing::SigningLock,
    /// The last FDM status answer (`commands::checkout::do_check_fdm_status`).
    pub fdm_status_cache: Mutex<Option<FdmStatusAnswer>>,
}

/// One cached FDM status answer: the key of everything that reaches the
/// FDM (its URL, the token and TLS files, the message language), when it
/// was fetched, and the answer. A changed setting queries the FDM again
/// at once.
pub type FdmStatusAnswer = (String, std::time::Instant, serde_json::Value);

impl AppState {
    /// The client of the configured FDM. The warnings and informations
    /// of every event it signs reach the engine's sink as
    /// [`crate::events::EngineEvent::FdmSignMessages`] (POS description
    /// v2.2, Message), whichever command signed.
    pub fn fdm_client(&self, config: &PosConfig) -> pos_fiscal::fdm_client::FdmClient {
        let events = self.events.clone();
        pos_fiscal::fdm_client::FdmClient::from_config_with(config, &self.http_tls)
            .with_sign_messages(std::sync::Arc::new(move |messages| {
                events.emit(crate::events::EngineEvent::FdmSignMessages(
                    messages.clone(),
                ));
            }))
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

/// The configuration [`crate::engine::config_source_from`] selects, loaded:
/// `env_path` is `PILI_POS_CONFIG`, `production_path` the file of the
/// installation manual. Kept for the tests of the selection order.
pub fn load_config_from(
    env_path: Option<&str>,
    production_path: &std::path::Path,
) -> anyhow::Result<PosConfig> {
    crate::engine::config_source_from(env_path, production_path).load()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pos_db::schema;

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
