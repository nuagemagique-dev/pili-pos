//! Delivery outbox: turns queued `ticket_deliveries` rows into TEM
//! send attempts, with exponential backoff, a bounded inline attempt
//! for the checkout path, and the GDPR recipient sweep.
//!
//! The SQLite connection is always taken as `&Mutex<Connection>` (the
//! same shape as `src-tauri`'s `AppState.db`/`poller_db`) and the lock
//! is only ever held across a synchronous block - never across an
//! `.await` - so a slow/hanging TEM request can't stall every other
//! DB user in the app.
//!
//! There is deliberately no background scheduling loop in this crate:
//! `run_once` is a single pass over due work, and it's the
//! integrator's call how it gets driven (a `tokio::time::interval`
//! tied into the app's async runtime, matching however the rest of
//! `src-tauri` schedules background work).

use std::sync::Mutex;
use std::time::Duration;

use rusqlite::Connection;

use pos_db::queries::deliveries::{self, TicketDelivery};

use crate::errors::MailError;
use crate::render;
use crate::tem::TemClient;

/// Retry/retention policy constants. The owner's "the business can't
/// have any bug" requirement means bounded, predictable behaviour
/// matters more than squeezing out delivery odds: a handful of
/// retries over about ninety minutes, then give up and let the
/// integrator fall back to paper - never retry forever, and never
/// silently drop a ticket without a `failed` row to show for it.
#[derive(Debug)]
pub struct DeliveryPolicy;

impl DeliveryPolicy {
    /// Delay before retrying after the 1st, 2nd, 3rd and 4th failed
    /// attempts, in order: 1 min, 5 min, 15 min, 60 min.
    pub const BACKOFF: [Duration; 4] = [
        Duration::from_secs(60),
        Duration::from_mins(5),
        Duration::from_mins(15),
        Duration::from_hours(1),
    ];
    /// Total attempts allowed (the initial one plus up to 4 retries)
    /// before a delivery is given up as `failed`.
    pub const MAX_ATTEMPTS: i64 = 5;
    /// Rows processed per `run_once` tick.
    pub const BATCH_SIZE: i64 = 20;
    /// A row stuck in `sending` longer than this is assumed orphaned
    /// (e.g. the app crashed mid-send) and is recovered to `pending`.
    /// Comfortably above both the inline attempt budget (~8 s) and any
    /// realistic background HTTP timeout.
    pub const STALE_SENDING_TIMEOUT: Duration = Duration::from_secs(90);
    /// Maximum time a plaintext recipient address is retained (GDPR).
    pub const GDPR_RETENTION: Duration = Duration::from_hours(72);

    /// Delay before retry number `attempt_number` (1-based: the count
    /// *after* the failure just recorded), or `None` once the attempt
    /// budget ([`Self::MAX_ATTEMPTS`]) is exhausted - the caller
    /// should give the delivery up as `failed` at that point.
    pub fn backoff_after(attempt_number: i64) -> Option<Duration> {
        if attempt_number >= Self::MAX_ATTEMPTS {
            return None;
        }
        let last = Self::BACKOFF.len() - 1;
        let idx = usize::try_from(attempt_number - 1).unwrap_or(0).min(last);
        Some(Self::BACKOFF[idx])
    }
}

/// Outcome of a single `run_once` pass.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RunReport {
    pub sent: usize,
    pub retried: usize,
    pub given_up: usize,
    pub recovered_stale: usize,
    pub purged_recipients: usize,
}

/// Outcome of [`send_now`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendNowOutcome {
    Sent {
        message_id: String,
    },
    /// Not sent within the inline budget: either the attempt failed,
    /// timed out, or the row had already been claimed elsewhere. In
    /// every case the row is left for the background worker (via
    /// `run_once`) to pick up later - check
    /// `pos_db::queries::deliveries::get` for the exact status/error
    /// if the caller needs to explain it to the operator.
    Queued,
}

/// The inline, bounded ("try now, fall back to paper later") send
/// attempt made right after checkout signs the ticket. Claims the row,
/// tries to send within `timeout`, and on any failure - including
/// the timeout itself - leaves the delivery queued for the background
/// worker rather than blocking checkout any longer.
pub async fn send_now(
    db: &Mutex<Connection>,
    client: &TemClient,
    restaurant_name: &str,
    delivery_id: i64,
    timeout: Duration,
) -> SendNowOutcome {
    let claimed = {
        let conn = lock_db(db);
        deliveries::claim(&conn, delivery_id)
    };
    match claimed {
        Ok(true) => {}
        Ok(false) => return SendNowOutcome::Queued,
        Err(e) => {
            tracing::warn!(delivery_id, error = %e, "pos-mail: send_now claim failed");
            return SendNowOutcome::Queued;
        }
    }

    let row = {
        let conn = lock_db(db);
        deliveries::get(&conn, delivery_id)
    };
    let row = match row {
        Ok(Some(row)) => row,
        Ok(None) => return SendNowOutcome::Queued,
        Err(e) => {
            tracing::warn!(delivery_id, error = %e, "pos-mail: send_now fetch failed");
            return SendNowOutcome::Queued;
        }
    };

    match tokio::time::timeout(timeout, attempt_send(db, client, restaurant_name, &row)).await {
        Ok(AttemptOutcome::Sent { message_id }) => SendNowOutcome::Sent { message_id },
        Ok(AttemptOutcome::Retrying | AttemptOutcome::GivenUp) => SendNowOutcome::Queued,
        Err(_elapsed) => {
            // The inline budget ran out. `attempt_send`'s future was
            // dropped mid-`.await` inside `client.send`, so the row
            // is still `sending` - no lock was held across that await,
            // so this is safe to do right away rather than waiting for
            // `run_once`'s stale-`sending` recovery.
            finalize_failure(
                db,
                delivery_id,
                "inline send timed out",
                Some(&iso_after(INLINE_TIMEOUT_RETRY_DELAY)),
            );
            SendNowOutcome::Queued
        }
    }
}

/// One pass over due work: recover orphaned `sending` rows, purge
/// GDPR-expired recipients, then attempt every currently-due delivery
/// (claiming each atomically first, so a concurrent `run_once` or
/// `send_now` can never double-send the same row).
pub async fn run_once(
    db: &Mutex<Connection>,
    client: &TemClient,
    restaurant_name: &str,
) -> Result<RunReport, MailError> {
    let mut report = RunReport::default();
    let now = now_iso();

    let stale_cutoff = iso_before(DeliveryPolicy::STALE_SENDING_TIMEOUT);
    {
        let conn = lock_db(db);
        report.recovered_stale = deliveries::recover_stale_sending(&conn, &stale_cutoff)?;
    }

    let purge_cutoff = iso_before(DeliveryPolicy::GDPR_RETENTION);
    {
        let conn = lock_db(db);
        report.purged_recipients = deliveries::purge_recipients_older_than(&conn, &purge_cutoff)?;
    }

    let due_rows = {
        let conn = lock_db(db);
        deliveries::due(&conn, &now, DeliveryPolicy::BATCH_SIZE)?
    };

    for row in due_rows {
        let claimed = {
            let conn = lock_db(db);
            deliveries::claim(&conn, row.id)
        };
        match claimed {
            Ok(true) => {}
            Ok(false) => continue, // another worker got there first
            Err(e) => {
                tracing::warn!(delivery_id = row.id, error = %e, "pos-mail: run_once claim failed");
                continue;
            }
        }

        match attempt_send(db, client, restaurant_name, &row).await {
            AttemptOutcome::Sent { .. } => report.sent += 1,
            AttemptOutcome::Retrying => report.retried += 1,
            AttemptOutcome::GivenUp => report.given_up += 1,
        }
    }

    Ok(report)
}

enum AttemptOutcome {
    Sent { message_id: String },
    Retrying,
    GivenUp,
}

/// Send a single already-claimed row and record the outcome. Shared by
/// `send_now` and `run_once`.
async fn attempt_send(
    db: &Mutex<Connection>,
    client: &TemClient,
    restaurant_name: &str,
    row: &TicketDelivery,
) -> AttemptOutcome {
    let Some(recipient) = row.recipient.clone() else {
        // A claimed (`sending`) row always came from `due()`, which
        // only returns `pending` rows - and a `pending` row should
        // always still have its recipient (the GDPR sweep only nulls
        // it once retries are exhausted, well past when `due()` would
        // stop returning it). Treat it defensively as unsendable
        // rather than ever calling TEM with an empty recipient.
        finalize_failure(db, row.id, "recipient missing at send time", None);
        return AttemptOutcome::GivenUp;
    };

    let email = match render::build_email(
        restaurant_name,
        &recipient,
        &row.ticket_text,
        row.verification_url.as_deref(),
    ) {
        Ok(email) => email,
        Err(e) => {
            finalize_failure(db, row.id, &format!("render error: {e}"), None);
            return AttemptOutcome::GivenUp;
        }
    };

    match client.send(&email).await {
        Ok(message_id) => {
            let conn = lock_db(db);
            if let Err(e) = deliveries::mark_sent(&conn, row.id, &message_id) {
                tracing::warn!(delivery_id = row.id, error = %e, "pos-mail: mark_sent failed");
            }
            AttemptOutcome::Sent { message_id }
        }
        Err(tem_err) => {
            let attempt_number = row.attempts + 1;
            // Never retries a permanent (non-retryable) error, even
            // if attempts remain - no point burning ~90 minutes of
            // backoff on a request that will never succeed.
            let next_attempt_at = if tem_err.is_retryable() {
                DeliveryPolicy::backoff_after(attempt_number).map(iso_after)
            } else {
                None
            };
            let gave_up = next_attempt_at.is_none();
            finalize_failure(db, row.id, &tem_err.to_string(), next_attempt_at.as_deref());
            if gave_up {
                AttemptOutcome::GivenUp
            } else {
                AttemptOutcome::Retrying
            }
        }
    }
}

fn finalize_failure(db: &Mutex<Connection>, id: i64, error: &str, next_attempt_at: Option<&str>) {
    let conn = lock_db(db);
    if let Err(e) = deliveries::mark_failed(&conn, id, error, next_attempt_at) {
        tracing::warn!(delivery_id = id, error = %e, "pos-mail: mark_failed failed");
    }
}

/// Lock the database, recovering from a poisoned mutex: a panic elsewhere rolled its open
/// transaction back, so the connection stays usable and the background worker must not die.
fn lock_db(db: &Mutex<Connection>) -> std::sync::MutexGuard<'_, Connection> {
    db.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How long a row waits after an inline send timed out. The mail may have been accepted by
/// TEM without the answer coming back; retrying at once would risk a duplicate.
const INLINE_TIMEOUT_RETRY_DELAY: Duration = Duration::from_secs(60);

const ISO_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Current time formatted to match the `strftime('%Y-%m-%dT%H:%M:%fZ',
/// 'now')` timestamps SQLite writes - both sort lexicographically, so
/// they compare correctly as plain TEXT.
fn now_iso() -> String {
    chrono::Utc::now().format(ISO_FORMAT).to_string()
}

fn iso_after(d: Duration) -> String {
    (chrono::Utc::now() + chrono::Duration::from_std(d).unwrap_or_default())
        .format(ISO_FORMAT)
        .to_string()
}

fn iso_before(d: Duration) -> String {
    (chrono::Utc::now() - chrono::Duration::from_std(d).unwrap_or_default())
        .format(ISO_FORMAT)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DeliveryPolicy::backoff_after - pure, no I/O ───────────

    #[test]
    fn backoff_follows_the_documented_schedule() {
        assert_eq!(
            DeliveryPolicy::backoff_after(1),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            DeliveryPolicy::backoff_after(2),
            Some(Duration::from_mins(5))
        );
        assert_eq!(
            DeliveryPolicy::backoff_after(3),
            Some(Duration::from_mins(15))
        );
        assert_eq!(
            DeliveryPolicy::backoff_after(4),
            Some(Duration::from_hours(1))
        );
    }

    #[test]
    fn backoff_gives_up_at_max_attempts() {
        assert_eq!(DeliveryPolicy::backoff_after(5), None);
        assert_eq!(DeliveryPolicy::backoff_after(6), None);
    }

    #[test]
    fn max_attempts_matches_the_backoff_table_plus_one() {
        assert_eq!(
            DeliveryPolicy::MAX_ATTEMPTS,
            i64::try_from(DeliveryPolicy::BACKOFF.len()).unwrap() + 1
        );
    }

    // ── attempt_send defensive guard: missing recipient ────────
    // No network involved - the guard fires before any TEM call, so a
    // client pointed at an address that would refuse the connection
    // proves the call was never made.

    fn unreachable_client() -> TemClient {
        TemClient::new(
            reqwest::Client::new(),
            "http://127.0.0.1:1", // port 0/1 refuses immediately
            "fr-par",
            "project",
            "secret",
            "from@example.com",
            "From",
            Duration::from_millis(200),
        )
    }

    fn test_db() -> Mutex<Connection> {
        let conn = Connection::open_in_memory().unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        Mutex::new(conn)
    }

    fn fiscal_event(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('N', 1, 'P', 'T', '00000000029', '{}')",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[tokio::test]
    async fn attempt_send_with_no_recipient_gives_up_without_a_network_call() {
        let db = test_db();
        let id = {
            let conn = db.lock().unwrap();
            let fe = fiscal_event(&conn);
            let id = deliveries::enqueue(
                &conn,
                deliveries::NewTicketDelivery {
                    fiscal_event_id: fe,
                    medium: "DIGITAL",
                    recipient: "customer@example.be",
                    recipient_hint: "c***@e***.be",
                    ticket_text: "ticket",
                    verification_url: Some("https://www.gs2.be/abc"),
                },
            )
            .unwrap();
            // Simulate the row having already lost its recipient
            // (defensive edge case).
            conn.execute(
                "UPDATE ticket_deliveries SET recipient = NULL WHERE id = ?1",
                [id],
            )
            .unwrap();
            id
        };

        let row = {
            let conn = db.lock().unwrap();
            deliveries::get(&conn, id).unwrap().unwrap()
        };
        let outcome = attempt_send(&db, &unreachable_client(), "Restaurant Exemple", &row).await;
        assert!(matches!(outcome, AttemptOutcome::GivenUp));

        let conn = db.lock().unwrap();
        let after = deliveries::get(&conn, id).unwrap().unwrap();
        assert_eq!(after.status, "failed");
        assert_eq!(
            after.last_error.as_deref(),
            Some("recipient missing at send time")
        );
    }

    #[test]
    fn a_poisoned_database_lock_is_recovered() {
        let db = std::sync::Arc::new(test_db());
        let other = std::sync::Arc::clone(&db);
        let _ = std::thread::spawn(move || {
            let _guard = other.lock().unwrap();
            panic!("poison the mutex");
        })
        .join();
        assert!(db.is_poisoned());
        finalize_failure(&db, 1, "x", None);
        assert!(lock_db(&db).execute_batch("SELECT 1").is_ok());
    }

    #[tokio::test]
    async fn an_inline_timeout_delays_the_retry() {
        // A listener that never answers: the connection opens, the request hangs.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = TemClient::new(
            reqwest::Client::new(),
            format!("http://{}", listener.local_addr().unwrap()),
            "fr-par",
            "project",
            "secret",
            "from@example.com",
            "From",
            Duration::from_secs(30),
        );
        let db = test_db();
        let id = {
            let conn = db.lock().unwrap();
            let fe = fiscal_event(&conn);
            deliveries::enqueue(
                &conn,
                deliveries::NewTicketDelivery {
                    fiscal_event_id: fe,
                    medium: "DIGITAL",
                    recipient: "customer@example.be",
                    recipient_hint: "c***@e***.be",
                    ticket_text: "ticket",
                    verification_url: Some("https://www.gs2.be/abc"),
                },
            )
            .unwrap()
        };
        let outcome = send_now(&db, &client, "R", id, Duration::from_millis(100)).await;
        assert!(matches!(outcome, SendNowOutcome::Queued));
        let after = deliveries::get(&lock_db(&db), id).unwrap().unwrap();
        let due = after.next_attempt_at.expect("a retry time");
        assert!(due > iso_after(Duration::from_secs(30)), "due {due}");
    }
}
