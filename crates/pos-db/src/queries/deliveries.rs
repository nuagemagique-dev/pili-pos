//! Digital ticket delivery outbox (see migration v5,
//! `ticket_deliveries`, in `crate::schema`).
//!
//! This module is pure SQLite: no network, no `pos-mail` types. The
//! retry policy, email rendering and Scaleway TEM HTTP calls live in
//! the `pos-mail` crate's `outbox` module, which calls the functions
//! here to read/claim/update rows.

use rusqlite::{Connection, OptionalExtension, params};

/// A row of `ticket_deliveries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketDelivery {
    pub id: i64,
    pub fiscal_event_id: i64,
    pub medium: String,
    pub recipient: Option<String>,
    pub recipient_hint: String,
    pub ticket_text: String,
    pub verification_url: Option<String>,
    pub status: String,
    pub attempts: i64,
    pub next_attempt_at: Option<String>,
    pub provider_message_id: Option<String>,
    pub last_error: Option<String>,
    pub fallback_printed: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Fields needed to enqueue a new delivery. Built by the caller
/// (`pos-mail::render`) right after a fiscal ticket is signed.
pub struct NewTicketDelivery<'a> {
    pub fiscal_event_id: i64,
    /// `DIGITAL` or `PAPER_DIGITAL`.
    pub medium: &'a str,
    /// Plaintext recipient email.
    pub recipient: &'a str,
    /// Masked recipient (e.g. `j***@e***.be`) for permanent display.
    pub recipient_hint: &'a str,
    /// The rendered plain-text ticket (identical to what prints).
    pub ticket_text: &'a str,
    pub verification_url: Option<&'a str>,
}

const SELECT_COLUMNS: &str = "id, fiscal_event_id, medium, recipient, recipient_hint, \
     ticket_text, verification_url, status, attempts, next_attempt_at, \
     provider_message_id, last_error, fallback_printed, created_at, updated_at";

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<TicketDelivery> {
    Ok(TicketDelivery {
        id: row.get(0)?,
        fiscal_event_id: row.get(1)?,
        medium: row.get(2)?,
        recipient: row.get(3)?,
        recipient_hint: row.get(4)?,
        ticket_text: row.get(5)?,
        verification_url: row.get(6)?,
        status: row.get(7)?,
        attempts: row.get(8)?,
        next_attempt_at: row.get(9)?,
        provider_message_id: row.get(10)?,
        last_error: row.get(11)?,
        fallback_printed: row.get::<_, i64>(12)? != 0,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

/// Enqueue a new delivery (status `pending`). Call inside the same
/// transaction as the fiscal event / order write it belongs to.
pub fn enqueue(conn: &Connection, new: NewTicketDelivery<'_>) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO ticket_deliveries
            (fiscal_event_id, medium, recipient, recipient_hint, ticket_text, verification_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new.fiscal_event_id,
            new.medium,
            new.recipient,
            new.recipient_hint,
            new.ticket_text,
            new.verification_url,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Fetch a single delivery by id.
pub fn get(conn: &Connection, id: i64) -> Result<Option<TicketDelivery>, rusqlite::Error> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM ticket_deliveries WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()
}

/// Most recent deliveries (any status), newest first - for the
/// settings/support screen.
pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<TicketDelivery>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM ticket_deliveries ORDER BY id DESC LIMIT ?1"
    ))?;
    stmt.query_map(params![limit], map_row)?.collect()
}

/// Pending deliveries ready to (re)try: `next_attempt_at` is unset or
/// has passed `now`. `now` is caller-supplied (rather than
/// `datetime('now')`) so tests can control exactly what's due.
pub fn due(
    conn: &Connection,
    now: &str,
    limit: i64,
) -> Result<Vec<TicketDelivery>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM ticket_deliveries
         WHERE status = 'pending'
           AND (next_attempt_at IS NULL OR next_attempt_at <= ?1)
         ORDER BY created_at ASC
         LIMIT ?2"
    ))?;
    stmt.query_map(params![now, limit], map_row)?.collect()
}

/// Atomically claim a pending row for sending: `pending` → `sending`.
/// Returns `false` (no error) if the row wasn't `pending` anymore -
/// e.g. a concurrent `run_once`/`send_now` already claimed it. This
/// is what makes concurrent workers claim-safe instead of
/// double-sending: only one `UPDATE ... WHERE status = 'pending'` can
/// ever affect the row.
pub fn claim(conn: &Connection, id: i64) -> Result<bool, rusqlite::Error> {
    let changed = conn.execute(
        "UPDATE ticket_deliveries
         SET status = 'sending', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1 AND status = 'pending'",
        params![id],
    )?;
    Ok(changed == 1)
}

/// Reclaim rows stuck in `sending` for longer than `older_than`
/// (e.g. the app crashed mid-send). Resets them to `pending` so the
/// next `run_once` retries them; does not touch `attempts`.
pub fn recover_stale_sending(
    conn: &Connection,
    older_than: &str,
) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "UPDATE ticket_deliveries
         SET status = 'pending', updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE status = 'sending' AND updated_at < ?1",
        params![older_than],
    )
}

/// Record a successful send: `sent`, and the recipient is nulled
/// immediately - the email has done its job, GDPR retention starts at
/// zero for a successful delivery.
pub fn mark_sent(conn: &Connection, id: i64, message_id: &str) -> Result<(), rusqlite::Error> {
    let changed = conn.execute(
        "UPDATE ticket_deliveries
         SET status = 'sent',
             provider_message_id = ?2,
             recipient = NULL,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1",
        params![id, message_id],
    )?;
    if changed == 1 {
        Ok(())
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

/// Record a failed attempt. `next_attempt_at` being `Some` means
/// "retry later" (status goes back to `pending`); `None` means the
/// caller's retry policy has given up (status becomes the terminal
/// `failed`). The attempt counter is always incremented.
pub fn mark_failed(
    conn: &Connection,
    id: i64,
    error: &str,
    next_attempt_at: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let changed = conn.execute(
        "UPDATE ticket_deliveries
         SET attempts = attempts + 1,
             last_error = ?2,
             next_attempt_at = ?3,
             status = CASE WHEN ?3 IS NULL THEN 'failed' ELSE 'pending' END,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1",
        params![id, error, next_attempt_at],
    )?;
    if changed == 1 {
        Ok(())
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

/// Record that the paper fallback was printed for this delivery.
/// Can only ever happen once per row - a second call errors instead
/// of silently no-op-ing, so a caller bug (double-printing a fallback
/// ticket) is never masked. The email is no longer retried, so the
/// recipient is cleared at once (GDPR).
pub fn mark_fallback_printed(conn: &Connection, id: i64) -> Result<(), rusqlite::Error> {
    let changed = conn.execute(
        "UPDATE ticket_deliveries
         SET fallback_printed = 1,
             status = 'fallback_printed',
             recipient = NULL,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE id = ?1 AND fallback_printed = 0",
        params![id],
    )?;
    if changed == 1 {
        Ok(())
    } else {
        Err(rusqlite::Error::QueryReturnedNoRows)
    }
}

/// GDPR sweep: null the plaintext recipient on every row older than
/// `cutoff` that still has one (72 h max retention, POS owner
/// requirement - in practice the retry policy always reaches a
/// terminal state within ~1.5 h, so this mostly targets `sent`/
/// `failed`/`fallback_printed` rows well past their deadline). A
/// `failed` row that gets purged this way also flips to `purged`,
/// since it can never be retried again without a recipient.
/// Returns the number of rows purged.
pub fn purge_recipients_older_than(
    conn: &Connection,
    cutoff: &str,
) -> Result<usize, rusqlite::Error> {
    conn.execute(
        "UPDATE ticket_deliveries
         SET recipient = NULL,
             status = CASE WHEN status = 'failed' THEN 'purged' ELSE status END,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
         WHERE recipient IS NOT NULL AND created_at < ?1",
        params![cutoff],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::run_migrations;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    /// A fiscal event row to satisfy the FK.
    fn fiscal_event(conn: &Connection, seq: i64) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('N', ?1, 'P', 'T', '00000000029', '{}')",
            [seq],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn new_delivery(fiscal_event_id: i64) -> NewTicketDelivery<'static> {
        NewTicketDelivery {
            fiscal_event_id,
            medium: "DIGITAL",
            recipient: "customer@example.be",
            recipient_hint: "c***@e***.be",
            ticket_text: "TICKET DE CAISSE TVA\n...",
            verification_url: Some("https://www.gs2.be/abc123"),
        }
    }

    // ── enqueue / get / list_recent ────────────────────────────

    #[test]
    fn enqueue_then_get_round_trips() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.fiscal_event_id, fe);
        assert_eq!(row.medium, "DIGITAL");
        assert_eq!(row.recipient.as_deref(), Some("customer@example.be"));
        assert_eq!(row.recipient_hint, "c***@e***.be");
        assert_eq!(row.status, "pending");
        assert_eq!(row.attempts, 0);
        assert!(!row.fallback_printed);
        assert!(row.next_attempt_at.is_none());
    }

    #[test]
    fn get_missing_row_is_none() {
        let conn = db();
        assert_eq!(get(&conn, 999).unwrap(), None);
    }

    #[test]
    fn list_recent_orders_newest_first() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let a = enqueue(&conn, new_delivery(fe)).unwrap();
        let b = enqueue(&conn, new_delivery(fe)).unwrap();
        let rows = list_recent(&conn, 10).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, b);
        assert_eq!(rows[1].id, a);
    }

    #[test]
    fn list_recent_respects_limit() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        for _ in 0..5 {
            enqueue(&conn, new_delivery(fe)).unwrap();
        }
        assert_eq!(list_recent(&conn, 2).unwrap().len(), 2);
    }

    // ── due() ───────────────────────────────────────────────────

    #[test]
    fn due_returns_freshly_enqueued_rows() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        enqueue(&conn, new_delivery(fe)).unwrap();
        let rows = due(&conn, "2026-09-25T12:00:00.000Z", 10).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn due_excludes_rows_not_yet_scheduled() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        // Fail it with a future retry time.
        mark_failed(&conn, id, "boom", Some("2026-09-25T13:00:00.000Z")).unwrap();

        let too_early = due(&conn, "2026-09-25T12:30:00.000Z", 10).unwrap();
        assert!(too_early.is_empty());

        let after_backoff = due(&conn, "2026-09-25T13:00:00.000Z", 10).unwrap();
        assert_eq!(after_backoff.len(), 1);
    }

    #[test]
    fn due_excludes_non_pending_statuses() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        assert!(claim(&conn, id).unwrap());
        assert!(
            due(&conn, "2099-01-01T00:00:00.000Z", 10)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn due_respects_limit() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        for _ in 0..5 {
            enqueue(&conn, new_delivery(fe)).unwrap();
        }
        assert_eq!(due(&conn, "2099-01-01T00:00:00.000Z", 3).unwrap().len(), 3);
    }

    // ── claim() atomicity ──────────────────────────────────────

    #[test]
    fn claim_transitions_pending_to_sending() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        assert!(claim(&conn, id).unwrap());
        assert_eq!(get(&conn, id).unwrap().unwrap().status, "sending");
    }

    #[test]
    fn a_second_claim_on_the_same_row_fails() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        assert!(claim(&conn, id).unwrap());
        // Simulates a second, concurrent `run_once`/`send_now` racing
        // for the same row: only the first claim wins.
        assert!(!claim(&conn, id).unwrap());
        assert_eq!(get(&conn, id).unwrap().unwrap().status, "sending");
    }

    #[test]
    fn claim_on_unknown_row_returns_false_not_error() {
        let conn = db();
        assert!(!claim(&conn, 999).unwrap());
    }

    // ── recover_stale_sending ──────────────────────────────────

    #[test]
    fn stale_sending_rows_are_recovered_to_pending() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        assert!(claim(&conn, id).unwrap());
        // Force the row's updated_at far in the past, as if the app
        // crashed mid-send.
        conn.execute(
            "UPDATE ticket_deliveries SET updated_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        let recovered = recover_stale_sending(&conn, "2020-01-01T00:00:00.000Z").unwrap();
        assert_eq!(recovered, 1);
        assert_eq!(get(&conn, id).unwrap().unwrap().status, "pending");
    }

    #[test]
    fn fresh_sending_rows_are_not_recovered() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        assert!(claim(&conn, id).unwrap());
        let recovered = recover_stale_sending(&conn, "2000-01-01T00:00:00.000Z").unwrap();
        assert_eq!(recovered, 0);
        assert_eq!(get(&conn, id).unwrap().unwrap().status, "sending");
    }

    // ── mark_sent ───────────────────────────────────────────────

    #[test]
    fn mark_sent_nulls_recipient_and_sets_status() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        claim(&conn, id).unwrap();
        mark_sent(&conn, id, "msg-123").unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, "sent");
        assert_eq!(row.provider_message_id.as_deref(), Some("msg-123"));
        assert_eq!(row.recipient, None);
        // The masked hint survives for support/dossier readability.
        assert_eq!(row.recipient_hint, "c***@e***.be");
    }

    #[test]
    fn mark_sent_on_unknown_row_errors() {
        let conn = db();
        assert!(mark_sent(&conn, 999, "msg").is_err());
    }

    // ── mark_failed ─────────────────────────────────────────────

    #[test]
    fn mark_failed_with_retry_time_stays_pending_and_bumps_attempts() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        claim(&conn, id).unwrap();
        mark_failed(&conn, id, "tem 503", Some("2026-09-25T13:00:00.000Z")).unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, "pending");
        assert_eq!(row.attempts, 1);
        assert_eq!(row.last_error.as_deref(), Some("tem 503"));
        assert_eq!(
            row.next_attempt_at.as_deref(),
            Some("2026-09-25T13:00:00.000Z")
        );
    }

    #[test]
    fn mark_failed_without_retry_time_gives_up() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        claim(&conn, id).unwrap();
        mark_failed(&conn, id, "tem 400", None).unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, "failed");
        assert_eq!(row.attempts, 1);
        assert!(row.next_attempt_at.is_none());
    }

    #[test]
    fn mark_failed_accumulates_attempts_across_retries() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        for _ in 0..3 {
            mark_failed(&conn, id, "err", Some("2099-01-01T00:00:00.000Z")).unwrap();
        }
        assert_eq!(get(&conn, id).unwrap().unwrap().attempts, 3);
    }

    // ── mark_fallback_printed ───────────────────────────────────

    #[test]
    fn mark_fallback_printed_sets_flag_and_status() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        mark_fallback_printed(&conn, id).unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert!(row.fallback_printed);
        assert_eq!(row.status, "fallback_printed");
    }

    #[test]
    fn mark_fallback_printed_a_second_time_errors() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        mark_fallback_printed(&conn, id).unwrap();
        assert!(mark_fallback_printed(&conn, id).is_err());
        // Still just printed once.
        assert!(get(&conn, id).unwrap().unwrap().fallback_printed);
    }

    #[test]
    fn mark_fallback_printed_clears_the_recipient_and_stops_retries() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        mark_fallback_printed(&conn, id).unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert!(row.recipient.is_none());
        assert!(!row.recipient_hint.is_empty(), "the masked hint is kept");
        assert!(
            due(&conn, "9999-12-31T00:00:00.000Z", 10)
                .unwrap()
                .is_empty()
        );
    }

    // ── purge_recipients_older_than (GDPR) ─────────────────────

    #[test]
    fn purge_nulls_old_recipients() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        conn.execute(
            "UPDATE ticket_deliveries SET created_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        let purged = purge_recipients_older_than(&conn, "2020-01-01T00:00:00.000Z").unwrap();
        assert_eq!(purged, 1);
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.recipient, None);
        // Hint survives - only the plaintext address is GDPR-sensitive.
        assert_eq!(row.recipient_hint, "c***@e***.be");
    }

    #[test]
    fn purge_leaves_recent_recipients_alone() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        let purged = purge_recipients_older_than(&conn, "2000-01-01T00:00:00.000Z").unwrap();
        assert_eq!(purged, 0);
        assert!(get(&conn, id).unwrap().unwrap().recipient.is_some());
    }

    #[test]
    fn purge_flips_a_failed_row_to_purged() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        mark_failed(&conn, id, "gave up", None).unwrap();
        conn.execute(
            "UPDATE ticket_deliveries SET created_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        purge_recipients_older_than(&conn, "2020-01-01T00:00:00.000Z").unwrap();
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, "purged");
        assert_eq!(row.recipient, None);
    }

    #[test]
    fn purge_does_not_touch_recipient_already_nulled_by_mark_sent() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        claim(&conn, id).unwrap();
        mark_sent(&conn, id, "msg-1").unwrap();
        conn.execute(
            "UPDATE ticket_deliveries SET created_at = '2000-01-01T00:00:00.000Z' WHERE id = ?1",
            params![id],
        )
        .unwrap();

        // Nothing to purge (recipient already NULL); status stays `sent`.
        let purged = purge_recipients_older_than(&conn, "2020-01-01T00:00:00.000Z").unwrap();
        assert_eq!(purged, 0);
        assert_eq!(get(&conn, id).unwrap().unwrap().status, "sent");
    }

    // ── triggers: append-only ───────────────────────────────────

    #[test]
    fn delete_is_forbidden() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        let err = conn.execute("DELETE FROM ticket_deliveries WHERE id = ?1", params![id]);
        assert!(err.is_err());
    }

    #[test]
    fn changing_fiscal_event_id_is_forbidden() {
        let conn = db();
        let fe1 = fiscal_event(&conn, 1);
        let fe2 = fiscal_event(&conn, 2);
        let id = enqueue(&conn, new_delivery(fe1)).unwrap();
        let err = conn.execute(
            "UPDATE ticket_deliveries SET fiscal_event_id = ?1 WHERE id = ?2",
            params![fe2, id],
        );
        assert!(err.is_err());
    }

    #[test]
    fn changing_ticket_text_is_forbidden() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        let err = conn.execute(
            "UPDATE ticket_deliveries SET ticket_text = 'tampered' WHERE id = ?1",
            params![id],
        );
        assert!(err.is_err());
    }

    #[test]
    fn changing_medium_is_forbidden() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        let err = conn.execute(
            "UPDATE ticket_deliveries SET medium = 'PAPER_DIGITAL' WHERE id = ?1",
            params![id],
        );
        assert!(err.is_err());
    }

    #[test]
    fn ordinary_status_updates_are_still_allowed() {
        // Sanity check that the immutability trigger only guards the
        // three named columns, not every UPDATE.
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        assert!(claim(&conn, id).unwrap());
    }

    #[test]
    fn medium_check_constraint_rejects_unknown_values() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let err = conn.execute(
            "INSERT INTO ticket_deliveries
                (fiscal_event_id, medium, recipient_hint, ticket_text)
             VALUES (?1, 'CARRIER_PIGEON', 'x***@y***.be', 'txt')",
            params![fe],
        );
        assert!(err.is_err());
    }

    #[test]
    fn status_check_constraint_rejects_unknown_values() {
        let conn = db();
        let fe = fiscal_event(&conn, 1);
        let id = enqueue(&conn, new_delivery(fe)).unwrap();
        let err = conn.execute(
            "UPDATE ticket_deliveries SET status = 'teleported' WHERE id = ?1",
            params![id],
        );
        assert!(err.is_err());
    }
}
