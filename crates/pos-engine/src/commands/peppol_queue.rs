//! The Peppol delivery queue (V11-D12): sends the UBL documents of signed invoices to the
//! access point, with retries.
//!
//! A pass takes the queued deliveries whose time has come, oldest first, one at a time:
//!
//! 1. when the recipient's Peppol status is `unknown`, ask the network (3 s); a recipient who
//!    is not registered parks the delivery at once, a lookup that fails is ignored;
//! 2. submit the document (30 s);
//! 3. record the outcome: `accepted`; a transient failure stays queued and is retried after
//!    a backoff (30 s, doubling, 30 min at most); a refusal parks the delivery (`failed`);
//!    a rejected API key parks it and stops the pass (every later call would fail alike).
//!
//! Every attempt is appended to `peppol_delivery_log`, whatever its outcome. A parked
//! delivery waits for an admin (`retry_peppol_delivery`). The queue never touches the
//! invoice or its fiscal event.

use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use pos_core::config::PeppolConfig;
use pos_db::queries::invoicing as q;
use pos_peppol::{
    AccessPoint, LOOKUP_TIMEOUT, LoggingAccessPoint, LookupResult, PeppolError, SUBMIT_TIMEOUT,
    StorecoveAccessPoint,
};
use tokio_util::sync::CancellationToken;

use crate::commands::errors;
use crate::state::AppState;

/// Delay before the first retry.
const BACKOFF_FIRST: Duration = Duration::from_secs(30);
/// Longest delay between two attempts.
const BACKOFF_MAX: Duration = Duration::from_secs(30 * 60);

/// Delay before the next attempt, after `attempts` failed attempts (1 = first failure).
pub fn backoff(attempts: i64) -> Duration {
    let doublings = u32::try_from(attempts.saturating_sub(1).clamp(0, 16)).unwrap_or(16);
    BACKOFF_FIRST
        .saturating_mul(1u32 << doublings)
        .min(BACKOFF_MAX)
}

/// The access point the configuration names.
pub(crate) fn access_point(
    state: &AppState,
    peppol: &PeppolConfig,
) -> Result<Box<dyn AccessPoint>, String> {
    match peppol.provider.as_str() {
        "storecove" => {
            let base = peppol
                .base_url
                .as_deref()
                .ok_or_else(|| errors::CONFIG_PEPPOL.to_string())?;
            let key = peppol
                .api_key()
                .map_err(errors::internal)?
                .ok_or_else(|| errors::CONFIG_PEPPOL.to_string())?;
            let entity = peppol
                .legal_entity_id
                .ok_or_else(|| errors::CONFIG_PEPPOL.to_string())?;
            let client = state
                .http_tls
                .client_builder()
                .map_err(errors::internal)?
                .build()
                .map_err(errors::internal)?;
            Ok(Box::new(StorecoveAccessPoint::new(
                client, base, &key, entity,
            )))
        }
        // Nothing is sent by the logging provider: debug builds only.
        _ if cfg!(debug_assertions) => Ok(Box::new(LoggingAccessPoint)),
        _ => Err(errors::CONFIG_PEPPOL.to_string()),
    }
}

fn stamp(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// Log a failed queue write with the delivery it was for. A write that
/// fails after the access point accepted a document must never vanish:
/// the delivery would be submitted again at the next pass.
fn written<T>(delivery_id: i64, what: &str, result: rusqlite::Result<T>) {
    if let Err(e) = result {
        tracing::error!(%e, delivery_id, what, "peppol queue: could not record the outcome");
    }
}

/// Park a delivery for good (log the attempt, then park), recording both.
fn park_with_log(
    db: &rusqlite::Connection,
    delivery_id: i64,
    attempt: i64,
    outcome: &str,
    http_status: Option<u16>,
    reason: &str,
    stamp_now: &str,
) {
    written(
        delivery_id,
        "log attempt",
        q::log_attempt(db, delivery_id, attempt, outcome, http_status, Some(reason)),
    );
    written(
        delivery_id,
        "park",
        q::park(db, delivery_id, attempt, reason, stamp_now),
    );
}

/// One pass at the current time. Returns how many deliveries it handled.
pub async fn run_pass(state: &AppState) -> usize {
    run_pass_at(state, Utc::now()).await
}

/// One pass as if it were `now`.
pub async fn run_pass_at(state: &AppState, now: DateTime<Utc>) -> usize {
    run_pass_until(state, now, &CancellationToken::new()).await
}

/// One pass that stops between two deliveries once `stop` is cancelled.
/// A delivery in flight is always finished (its submit is bounded by
/// `SUBMIT_TIMEOUT`): dropping it after the access point accepted the
/// document, before `mark_accepted`, would send the invoice twice at the
/// next start.
pub async fn run_pass_until(
    state: &AppState,
    now: DateTime<Utc>,
    stop: &CancellationToken,
) -> usize {
    match try_pass(state, now, stop).await {
        Ok(handled) => handled,
        Err(e) => {
            tracing::error!(%e, "peppol queue: pass failed");
            0
        }
    }
}

/// One pass, with a database failure reported as such.
///
/// # Errors
/// The due deliveries could not be read.
pub async fn try_pass(
    state: &AppState,
    now: DateTime<Utc>,
    stop: &CancellationToken,
) -> Result<usize, String> {
    let Some(peppol) = state.config.lock().ok().and_then(|c| c.peppol.clone()) else {
        return Ok(0);
    };
    let due = {
        let db = state.db.lock().map_err(errors::internal)?;
        q::due_deliveries(&db, &stamp(now)).map_err(errors::internal)?
    };
    if due.is_empty() {
        // No HTTP client is built for an empty queue.
        return Ok(0);
    }
    let ap = match access_point(state, &peppol) {
        Ok(ap) => ap,
        Err(e) => {
            tracing::warn!(%e, "peppol queue: access point not usable");
            return Ok(0);
        }
    };
    let mut handled = 0;
    for delivery in due {
        if stop.is_cancelled() {
            break;
        }
        handled += 1;
        let attempt = delivery.attempts + 1;
        let stamp_now = stamp(now);

        // 1. Is the recipient on the network?
        // A customer cached as `not_registered` is asked again (they may have
        // joined since); unless the network now says `registered`, the
        // delivery is parked, never sent.
        let cached_not_registered = delivery.lookup_status == "not_registered";
        if delivery.lookup_status == "unknown" || cached_not_registered {
            let answer =
                tokio::time::timeout(LOOKUP_TIMEOUT, ap.lookup(&delivery.customer_endpoint_id))
                    .await;
            match answer {
                Ok(Ok(LookupResult::NotRegistered)) => {
                    if let Ok(db) = state.db.lock() {
                        written(
                            delivery.id,
                            "lookup status",
                            q::set_delivery_lookup(&db, delivery.id, "not_registered"),
                        );
                        park_with_log(
                            &db,
                            delivery.id,
                            attempt,
                            "not_registered",
                            None,
                            "recipient is not registered on Peppol",
                            &stamp_now,
                        );
                    }
                    continue;
                }
                Ok(Ok(LookupResult::Registered)) => {
                    if let Ok(db) = state.db.lock() {
                        written(
                            delivery.id,
                            "lookup status",
                            q::set_delivery_lookup(&db, delivery.id, "registered"),
                        );
                    }
                }
                // A lookup that fails or is slow never blocks the delivery,
                // except for a recipient already known as not registered.
                _ if cached_not_registered => {
                    if let Ok(db) = state.db.lock() {
                        park_with_log(
                            &db,
                            delivery.id,
                            attempt,
                            "not_registered",
                            None,
                            "recipient is not registered on Peppol",
                            &stamp_now,
                        );
                    }
                    continue;
                }
                _ => {}
            }
        }

        // 2. Submit.
        let ubl = match state.db.lock().map(|db| q::delivery_ubl(&db, delivery.id)) {
            Ok(Ok(Some(ubl))) => ubl,
            _ => {
                if let Ok(db) = state.db.lock() {
                    park_with_log(
                        &db,
                        delivery.id,
                        attempt,
                        "rejected",
                        None,
                        "invoice UBL could not be read",
                        &stamp_now,
                    );
                }
                continue;
            }
        };
        let sent = tokio::time::timeout(
            SUBMIT_TIMEOUT,
            ap.submit_to(
                &delivery.customer_endpoint_id,
                &ubl,
                &format!("pili-pos-peppol-delivery-{}", delivery.id),
            ),
        )
        .await;
        let outcome = sent.unwrap_or_else(|_| {
            Err(PeppolError::Transient {
                status: None,
                detail: "submit timed out".to_string(),
            })
        });

        // 3. Record.
        let Ok(db) = state.db.lock() else {
            tracing::error!(
                delivery_id = delivery.id,
                "peppol queue: database lock poisoned, outcome not recorded"
            );
            continue;
        };
        match outcome {
            Ok(receipt) => {
                written(
                    delivery.id,
                    "log attempt",
                    q::log_attempt(
                        &db,
                        delivery.id,
                        attempt,
                        "accepted",
                        Some(200),
                        Some(&receipt.message_id),
                    ),
                );
                written(
                    delivery.id,
                    "mark accepted",
                    q::mark_accepted(
                        &db,
                        delivery.id,
                        attempt,
                        &receipt.message_id,
                        &receipt.raw_response,
                        &receipt.accepted_at,
                    ),
                );
            }
            Err(e @ (PeppolError::Transient { .. } | PeppolError::Http(_))) => {
                let next =
                    stamp(now + chrono::Duration::from_std(backoff(attempt)).unwrap_or_default());
                written(
                    delivery.id,
                    "log attempt",
                    q::log_attempt(
                        &db,
                        delivery.id,
                        attempt,
                        "transient",
                        e.http_status(),
                        Some(&e.to_string()),
                    ),
                );
                written(
                    delivery.id,
                    "mark retry",
                    q::mark_retry(&db, delivery.id, attempt, &next, &e.to_string(), &stamp_now),
                );
            }
            Err(e @ PeppolError::Unauthorized { .. }) => {
                park_with_log(
                    &db,
                    delivery.id,
                    attempt,
                    "unauthorized",
                    e.http_status(),
                    &e.to_string(),
                    &stamp_now,
                );
                // Every later call would fail the same way.
                break;
            }
            Err(e) => {
                park_with_log(
                    &db,
                    delivery.id,
                    attempt,
                    "rejected",
                    e.http_status(),
                    &e.to_string(),
                    &stamp_now,
                );
            }
        }
    }
    Ok(handled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backoff_doubles_from_30_seconds_up_to_30_minutes() {
        assert_eq!(backoff(1), Duration::from_secs(30));
        assert_eq!(backoff(2), Duration::from_secs(60));
        assert_eq!(backoff(3), Duration::from_secs(120));
        assert_eq!(backoff(7), Duration::from_secs(1800));
        assert_eq!(backoff(50), Duration::from_secs(1800));
        assert_eq!(backoff(0), Duration::from_secs(30));
    }
}
