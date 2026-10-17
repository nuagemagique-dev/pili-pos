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
        _ => Ok(Box::new(LoggingAccessPoint)),
    }
}

fn stamp(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// One pass at the current time. Returns how many deliveries it handled.
pub async fn run_pass(state: &AppState) -> usize {
    run_pass_at(state, Utc::now()).await
}

/// One pass as if it were `now`.
pub async fn run_pass_at(state: &AppState, now: DateTime<Utc>) -> usize {
    let Some(peppol) = state.config.lock().ok().and_then(|c| c.peppol.clone()) else {
        return 0;
    };
    let ap = match access_point(state, &peppol) {
        Ok(ap) => ap,
        Err(e) => {
            tracing::warn!(%e, "peppol queue: access point not usable");
            return 0;
        }
    };
    let due = match state.db.lock() {
        Ok(db) => q::due_deliveries(&db, &stamp(now)).unwrap_or_default(),
        Err(_) => return 0,
    };
    let mut handled = 0;
    for delivery in due {
        handled += 1;
        let attempt = delivery.attempts + 1;
        let stamp_now = stamp(now);

        // 1. Is the recipient on the network?
        if delivery.lookup_status == "unknown" {
            let answer =
                tokio::time::timeout(LOOKUP_TIMEOUT, ap.lookup(&delivery.customer_endpoint_id))
                    .await;
            match answer {
                Ok(Ok(LookupResult::NotRegistered)) => {
                    if let Ok(db) = state.db.lock() {
                        let _ = q::set_delivery_lookup(&db, delivery.id, "not_registered");
                        let _ = q::log_attempt(
                            &db,
                            delivery.id,
                            attempt,
                            "not_registered",
                            None,
                            Some("recipient is not registered on Peppol"),
                        );
                        let _ = q::park(
                            &db,
                            delivery.id,
                            attempt,
                            "recipient is not registered on Peppol",
                            &stamp_now,
                        );
                    }
                    continue;
                }
                Ok(Ok(LookupResult::Registered)) => {
                    if let Ok(db) = state.db.lock() {
                        let _ = q::set_delivery_lookup(&db, delivery.id, "registered");
                    }
                }
                // A lookup that fails or is slow never blocks the delivery.
                _ => {}
            }
        }

        // 2. Submit.
        let ubl = match state.db.lock().map(|db| q::delivery_ubl(&db, delivery.id)) {
            Ok(Ok(Some(ubl))) => ubl,
            _ => continue,
        };
        let sent = tokio::time::timeout(
            SUBMIT_TIMEOUT,
            ap.submit_to(&delivery.customer_endpoint_id, &ubl),
        )
        .await;
        let outcome = sent.unwrap_or_else(|_| {
            Err(PeppolError::Transient {
                status: None,
                detail: "submit timed out".to_string(),
            })
        });

        // 3. Record.
        let Ok(db) = state.db.lock() else { continue };
        match outcome {
            Ok(receipt) => {
                let _ = q::log_attempt(
                    &db,
                    delivery.id,
                    attempt,
                    "accepted",
                    Some(200),
                    Some(&receipt.message_id),
                );
                let _ = q::mark_accepted(
                    &db,
                    delivery.id,
                    attempt,
                    &receipt.message_id,
                    &receipt.raw_response,
                    &receipt.accepted_at,
                );
            }
            Err(e @ PeppolError::Transient { .. }) | Err(e @ PeppolError::Http(_)) => {
                let next =
                    stamp(now + chrono::Duration::from_std(backoff(attempt)).unwrap_or_default());
                let _ = q::log_attempt(
                    &db,
                    delivery.id,
                    attempt,
                    "transient",
                    e.http_status(),
                    Some(&e.to_string()),
                );
                let _ = q::mark_retry(&db, delivery.id, attempt, &next, &e.to_string(), &stamp_now);
            }
            Err(e @ PeppolError::Unauthorized { .. }) => {
                let _ = q::log_attempt(
                    &db,
                    delivery.id,
                    attempt,
                    "unauthorized",
                    e.http_status(),
                    Some(&e.to_string()),
                );
                let _ = q::park(&db, delivery.id, attempt, &e.to_string(), &stamp_now);
                // Every later call would fail the same way.
                break;
            }
            Err(e) => {
                let _ = q::log_attempt(
                    &db,
                    delivery.id,
                    attempt,
                    "rejected",
                    e.http_status(),
                    Some(&e.to_string()),
                );
                let _ = q::park(&db, delivery.id, attempt, &e.to_string(), &stamp_now);
            }
        }
    }
    handled
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
