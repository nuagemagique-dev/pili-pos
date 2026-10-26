//! B2B customers, invoices and the Peppol delivery queue (migration 40, V11).
//!
//! The invoice rows (`invoices`, `invoice_sales`) are append-only and are written in the same
//! transaction as the I event's archive (the engine's signing commit); the queue row
//! (`peppol_deliveries`) moves from `queued` to `accepted` or `failed`, and every attempt
//! is appended to `peppol_delivery_log`.

use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

// ── Customers ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub id: i64,
    pub vat_number: String,
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country_code: String,
    /// `unknown`, `registered` or `not_registered`.
    pub peppol_status: String,
    pub peppol_checked_at: Option<String>,
}

/// What a customer form holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCustomer {
    pub vat_number: String,
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country_code: String,
}

const CUSTOMER_COLS: &str = "id, vat_number, name, street, postal_code, city, country_code, \
                             peppol_status, peppol_checked_at";

fn customer_of(row: &Row<'_>) -> rusqlite::Result<Customer> {
    Ok(Customer {
        id: row.get(0)?,
        vat_number: row.get(1)?,
        name: row.get(2)?,
        street: row.get(3)?,
        postal_code: row.get(4)?,
        city: row.get(5)?,
        country_code: row.get(6)?,
        peppol_status: row.get(7)?,
        peppol_checked_at: row.get(8)?,
    })
}

pub fn get_customer(conn: &Connection, id: i64) -> rusqlite::Result<Option<Customer>> {
    conn.query_row(
        &format!("SELECT {CUSTOMER_COLS} FROM b2b_customers WHERE id = ?1"),
        params![id],
        customer_of,
    )
    .optional()
}

pub fn find_customer_by_vat(conn: &Connection, vat: &str) -> rusqlite::Result<Option<Customer>> {
    conn.query_row(
        &format!("SELECT {CUSTOMER_COLS} FROM b2b_customers WHERE vat_number = ?1"),
        params![vat],
        customer_of,
    )
    .optional()
}

/// Customers whose VAT number or name starts with `query` (case-insensitive), at most `limit`,
/// by name.
pub fn search_customers(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> rusqlite::Result<Vec<Customer>> {
    let q: String = query
        .trim()
        .chars()
        .filter(|c| !matches!(c, '%' | '_' | '\\'))
        .collect();
    let like = format!("{}%", q.to_lowercase());
    let mut stmt = conn.prepare(&format!(
        "SELECT {CUSTOMER_COLS} FROM b2b_customers
         WHERE lower(vat_number) LIKE ?1 OR lower(name) LIKE ?1
         ORDER BY lower(name), id LIMIT ?2"
    ))?;
    let rows = stmt.query_map(params![like, limit], customer_of)?;
    rows.collect()
}

pub fn insert_customer(conn: &Connection, c: &NewCustomer, by: &str) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO b2b_customers (vat_number, name, street, postal_code, city, country_code, updated_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![c.vat_number, c.name, c.street, c.postal_code, c.city, c.country_code, by],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Update a customer. A changed VAT number resets the cached Peppol status.
pub fn update_customer(
    conn: &Connection,
    id: i64,
    c: &NewCustomer,
    by: &str,
) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "UPDATE b2b_customers
         SET peppol_status = CASE WHEN vat_number <> ?2 THEN 'unknown' ELSE peppol_status END,
             peppol_checked_at = CASE WHEN vat_number <> ?2 THEN NULL ELSE peppol_checked_at END,
             vat_number = ?2, name = ?3, street = ?4, postal_code = ?5, city = ?6,
             country_code = ?7, updated_at = datetime('now'), updated_by = ?8
         WHERE id = ?1",
        params![
            id,
            c.vat_number,
            c.name,
            c.street,
            c.postal_code,
            c.city,
            c.country_code,
            by
        ],
    )?;
    Ok(n == 1)
}

/// Cache the result of a Peppol lookup on the customer (`checked_at` is `now`).
pub fn set_customer_peppol_status(
    conn: &Connection,
    id: i64,
    status: &str,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE b2b_customers SET peppol_status = ?2, peppol_checked_at = ?3 WHERE id = ?1",
        params![id, status, now],
    )?;
    Ok(())
}

// ── Invoices ───────────────────────────────────────────────────────

/// Ids of the most recent signed sales (event N, never training, never closed unpaid), newest
/// first. `query` keeps those whose ticket number or `posDateTime` starts with it.
pub fn recent_sales(conn: &Connection, limit: i64, query: &str) -> rusqlite::Result<Vec<i64>> {
    let q: String = query
        .trim()
        .chars()
        .filter(|c| !matches!(c, '%' | '_' | '\\'))
        .collect();
    let like = format!("{q}%");
    let mut stmt = conn.prepare(
        "SELECT fe.id FROM fiscal_events fe
         WHERE fe.event_type = 'N' AND fe.fdm_id IS NOT NULL
           AND NOT (json_array_length(COALESCE(json_extract(fe.event_data_json, '$.financials'), '[]')) = 0
                    AND COALESCE(json_extract(fe.event_data_json, '$.transaction.transactionTotal'), 0) <> 0)
           AND (?1 = '%'
                OR CAST(json_extract(fe.event_data_json, '$.posFiscalTicketNo') AS TEXT) LIKE ?1
                OR json_extract(fe.event_data_json, '$.posDateTime') LIKE ?1)
         ORDER BY fe.id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![like, limit], |r| r.get(0))?;
    rows.collect()
}

/// The next invoice sequence number: one continuous counter over all invoices, never reset
/// (AM art. 32).
pub fn next_invoice_seq(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(invoice_seq), 0) + 1 FROM invoices",
        [],
        |r| r.get(0),
    )
}

/// The number of the invoice a sale is already on.
pub fn invoice_of_sale(conn: &Connection, sale_event_id: i64) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT i.invoice_no FROM invoice_sales s JOIN invoices i ON i.id = s.invoice_id
         WHERE s.sale_fiscal_event_id = ?1",
        params![sale_event_id],
        |r| r.get(0),
    )
    .optional()
}

/// An invoice to record with the I event's archive.
#[derive(Debug, Clone)]
pub struct NewInvoice<'a> {
    pub invoice_seq: i64,
    pub invoice_no: &'a str,
    pub fiscal_event_id: i64,
    pub customer_id: Option<i64>,
    pub customer_json: &'a str,
    pub issue_date: &'a str,
    pub total_cents: i64,
    pub vat_json: &'a str,
    pub created_by: &'a str,
    pub sale_event_ids: &'a [i64],
}

/// Insert the invoice and its sales, all or nothing (a savepoint, so it also works inside the
/// signing transaction).
pub fn insert_invoice(conn: &Connection, inv: &NewInvoice<'_>) -> rusqlite::Result<i64> {
    super::with_savepoint(conn, |conn| insert_invoice_rows(conn, inv))
}

fn insert_invoice_rows(conn: &Connection, inv: &NewInvoice<'_>) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO invoices (invoice_seq, invoice_no, fiscal_event_id, customer_id, customer_json,
                               issue_date, total_cents, vat_json, created_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            inv.invoice_seq,
            inv.invoice_no,
            inv.fiscal_event_id,
            inv.customer_id,
            inv.customer_json,
            inv.issue_date,
            inv.total_cents,
            inv.vat_json,
            inv.created_by
        ],
    )?;
    let id = conn.last_insert_rowid();
    for (position, sale) in (1_i64..).zip(inv.sale_event_ids.iter()) {
        conn.execute(
            "INSERT INTO invoice_sales (invoice_id, sale_fiscal_event_id, position) VALUES (?1, ?2, ?3)",
            params![id, sale, position],
        )?;
    }
    Ok(id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceRow {
    pub id: i64,
    pub invoice_no: String,
    pub fiscal_event_id: i64,
    pub issue_date: String,
    pub customer_json: String,
    pub total_cents: i64,
    pub vat_json: String,
    pub created_at: String,
    /// Status of the latest Peppol delivery row, if there is one.
    pub delivery_status: Option<String>,
    pub delivery_id: Option<i64>,
    pub sale_event_ids: Vec<i64>,
}

fn invoice_row_of(conn: &Connection, row: &Row<'_>) -> rusqlite::Result<InvoiceRow> {
    let id: i64 = row.get(0)?;
    let mut stmt = conn.prepare(
        "SELECT sale_fiscal_event_id FROM invoice_sales WHERE invoice_id = ?1 ORDER BY position",
    )?;
    let sale_event_ids = stmt
        .query_map(params![id], |r| r.get(0))?
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(InvoiceRow {
        id,
        invoice_no: row.get(1)?,
        fiscal_event_id: row.get(2)?,
        issue_date: row.get(3)?,
        customer_json: row.get(4)?,
        total_cents: row.get(5)?,
        vat_json: row.get(6)?,
        created_at: row.get(7)?,
        delivery_status: row.get(8)?,
        delivery_id: row.get(9)?,
        sale_event_ids,
    })
}

const INVOICE_SELECT: &str =
    "SELECT i.id, i.invoice_no, i.fiscal_event_id, i.issue_date, i.customer_json,
        i.total_cents, i.vat_json, i.created_at,
        (SELECT p.status FROM peppol_deliveries p WHERE p.fiscal_event_id = i.fiscal_event_id
          ORDER BY p.id DESC LIMIT 1),
        (SELECT p.id FROM peppol_deliveries p WHERE p.fiscal_event_id = i.fiscal_event_id
          ORDER BY p.id DESC LIMIT 1)
 FROM invoices i";

/// The most recent invoices, newest first.
pub fn list_invoices(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<InvoiceRow>> {
    let mut stmt = conn.prepare(&format!(
        "{INVOICE_SELECT} ORDER BY i.invoice_seq DESC LIMIT ?1"
    ))?;
    let rows = stmt.query_map(params![limit], |r| invoice_row_of(conn, r))?;
    rows.collect()
}

/// The invoice whose I event is `fiscal_event_id`.
pub fn invoice_of_event(
    conn: &Connection,
    fiscal_event_id: i64,
) -> rusqlite::Result<Option<InvoiceRow>> {
    conn.query_row(
        &format!("{INVOICE_SELECT} WHERE i.fiscal_event_id = ?1"),
        params![fiscal_event_id],
        |r| invoice_row_of(conn, r),
    )
    .optional()
}

// ── Peppol deliveries ──────────────────────────────────────────────

/// A delivery to queue, in the invoice's archive transaction.
#[derive(Debug, Clone)]
pub struct NewDelivery<'a> {
    pub fiscal_event_id: i64,
    pub invoice_no: &'a str,
    pub customer_endpoint_id: &'a str,
    pub ap_vendor: &'a str,
    pub ubl_xml: &'a str,
    pub lookup_status: &'a str,
    /// RFC 3339: the first attempt is due now.
    pub now: &'a str,
}

pub fn insert_delivery(conn: &Connection, d: &NewDelivery<'_>) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO peppol_deliveries
            (fiscal_event_id, invoice_no, customer_endpoint_id, ap_vendor, ubl_xml, status,
             lookup_status, next_attempt_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'queued', ?6, ?7)",
        params![
            d.fiscal_event_id,
            d.invoice_no,
            d.customer_endpoint_id,
            d.ap_vendor,
            d.ubl_xml,
            d.lookup_status,
            d.now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub id: i64,
    pub fiscal_event_id: Option<i64>,
    pub invoice_no: String,
    pub customer_endpoint_id: String,
    pub ap_vendor: String,
    pub ap_message_id: Option<String>,
    /// `queued`, `accepted` or `failed` (parked).
    pub status: String,
    pub error_msg: Option<String>,
    pub attempts: i64,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub parked_at: Option<String>,
    pub accepted_at: Option<String>,
    pub created_at: String,
    pub lookup_status: String,
}

const DELIVERY_COLS: &str = "id, fiscal_event_id, invoice_no, customer_endpoint_id, ap_vendor,
        ap_message_id, status, error_msg, attempts, next_attempt_at, last_attempt_at, parked_at,
        accepted_at, created_at, lookup_status";

fn delivery_of(row: &Row<'_>) -> rusqlite::Result<Delivery> {
    Ok(Delivery {
        id: row.get(0)?,
        fiscal_event_id: row.get(1)?,
        invoice_no: row.get(2)?,
        customer_endpoint_id: row.get(3)?,
        ap_vendor: row.get(4)?,
        ap_message_id: row.get(5)?,
        status: row.get(6)?,
        error_msg: row.get(7)?,
        attempts: row.get(8)?,
        next_attempt_at: row.get(9)?,
        last_attempt_at: row.get(10)?,
        parked_at: row.get(11)?,
        accepted_at: row.get(12)?,
        created_at: row.get(13)?,
        lookup_status: row.get(14)?,
    })
}

pub fn get_delivery(conn: &Connection, id: i64) -> rusqlite::Result<Option<Delivery>> {
    conn.query_row(
        &format!("SELECT {DELIVERY_COLS} FROM peppol_deliveries WHERE id = ?1"),
        params![id],
        delivery_of,
    )
    .optional()
}

/// The UBL document of a delivery.
pub fn delivery_ubl(conn: &Connection, id: i64) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT ubl_xml FROM peppol_deliveries WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )
    .optional()
}

/// Newest first.
pub fn list_deliveries(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<Delivery>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {DELIVERY_COLS} FROM peppol_deliveries ORDER BY id DESC LIMIT ?1"
    ))?;
    let rows = stmt.query_map(params![limit], delivery_of)?;
    rows.collect()
}

/// Queued deliveries whose next attempt is due at `now` (RFC 3339), oldest first.
pub fn due_deliveries(conn: &Connection, now: &str) -> rusqlite::Result<Vec<Delivery>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {DELIVERY_COLS} FROM peppol_deliveries
         WHERE status = 'queued' AND (next_attempt_at IS NULL OR next_attempt_at <= ?1)
         ORDER BY id"
    ))?;
    let rows = stmt.query_map(params![now], delivery_of)?;
    rows.collect()
}

/// `(queued, accepted, failed)` counts.
pub fn queue_counts(conn: &Connection) -> rusqlite::Result<(i64, i64, i64)> {
    let count = |status: &str| -> rusqlite::Result<i64> {
        conn.query_row(
            "SELECT COUNT(*) FROM peppol_deliveries WHERE status = ?1",
            params![status],
            |r| r.get(0),
        )
    };
    Ok((count("queued")?, count("accepted")?, count("failed")?))
}

/// Append one attempt to the log.
pub fn log_attempt(
    conn: &Connection,
    delivery_id: i64,
    attempt: i64,
    outcome: &str,
    http_status: Option<u16>,
    detail: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO peppol_delivery_log (delivery_id, attempt, outcome, http_status, detail)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            delivery_id,
            attempt,
            outcome,
            http_status.map(i64::from),
            detail
        ],
    )?;
    Ok(())
}

/// One attempt of the log: `(attempt, outcome, http_status, detail)`.
pub type LogRow = (i64, String, Option<i64>, Option<String>);

/// The log of one delivery, oldest first.
pub fn delivery_log(conn: &Connection, delivery_id: i64) -> rusqlite::Result<Vec<LogRow>> {
    let mut stmt = conn.prepare(
        "SELECT attempt, outcome, http_status, detail FROM peppol_delivery_log
         WHERE delivery_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![delivery_id], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    })?;
    rows.collect()
}

pub fn mark_accepted(
    conn: &Connection,
    id: i64,
    attempts: i64,
    message_id: &str,
    response: &str,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE peppol_deliveries
         SET status = 'accepted', attempts = ?2, ap_message_id = ?3, ap_response_json = ?4,
             accepted_at = ?5, last_attempt_at = ?5, next_attempt_at = NULL, error_msg = NULL
         WHERE id = ?1",
        params![id, attempts, message_id, response, now],
    )?;
    Ok(())
}

/// A transient failure: stays queued, retried at `next_attempt_at`.
pub fn mark_retry(
    conn: &Connection,
    id: i64,
    attempts: i64,
    next_attempt_at: &str,
    error: &str,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE peppol_deliveries
         SET attempts = ?2, next_attempt_at = ?3, error_msg = ?4, last_attempt_at = ?5
         WHERE id = ?1",
        params![id, attempts, next_attempt_at, error, now],
    )?;
    Ok(())
}

/// Park a delivery: `failed`, with its reason, no further attempt until an admin re-queues it.
pub fn park(
    conn: &Connection,
    id: i64,
    attempts: i64,
    reason: &str,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE peppol_deliveries
         SET status = 'failed', attempts = ?2, error_msg = ?3, parked_at = ?4,
             last_attempt_at = ?4, next_attempt_at = NULL
         WHERE id = ?1",
        params![id, attempts, reason, now],
    )?;
    Ok(())
}

/// Re-queue a parked delivery. `false` when it is not parked.
pub fn requeue(conn: &Connection, id: i64, now: &str) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "UPDATE peppol_deliveries
         SET status = 'queued', parked_at = NULL, next_attempt_at = ?2, error_msg = NULL
         WHERE id = ?1 AND status = 'failed'",
        params![id, now],
    )?;
    Ok(n == 1)
}

pub fn set_delivery_lookup(conn: &Connection, id: i64, status: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE peppol_deliveries SET lookup_status = ?2 WHERE id = ?1",
        params![id, status],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::run_migrations;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn customer(vat: &str, name: &str) -> NewCustomer {
        NewCustomer {
            vat_number: vat.into(),
            name: name.into(),
            street: "Rue 1".into(),
            postal_code: "4000".into(),
            city: "Liege".into(),
            country_code: "BE".into(),
        }
    }

    fn event(conn: &Connection, seq: i64, kind: &str) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json, fdm_id, event_label)
             VALUES (?1, ?2, 'P', 'T', '00000000029', '{}', 'F1', ?1)",
            params![kind, seq],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn customers_are_found_by_vat_and_by_name_prefix() {
        let conn = setup();
        let a = insert_customer(&conn, &customer("BE0308357159", "Acme SA"), "u").unwrap();
        insert_customer(&conn, &customer("BE0499999960", "Alpha SRL"), "u").unwrap();
        insert_customer(&conn, &customer("FR12345678901", "Zeta SAS"), "u").unwrap();
        assert!(insert_customer(&conn, &customer("BE0308357159", "Dup"), "u").is_err());
        assert_eq!(
            find_customer_by_vat(&conn, "BE0308357159")
                .unwrap()
                .unwrap()
                .id,
            a
        );
        assert!(
            find_customer_by_vat(&conn, "BE0000000000")
                .unwrap()
                .is_none()
        );
        let by_name: Vec<String> = search_customers(&conn, "al", 20)
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(by_name, vec!["Alpha SRL"]);
        assert_eq!(search_customers(&conn, "be0", 20).unwrap().len(), 2);
        assert_eq!(search_customers(&conn, "", 20).unwrap().len(), 3);
        assert_eq!(search_customers(&conn, "", 2).unwrap().len(), 2);
        // LIKE wildcards in the query are not wildcards.
        assert_eq!(search_customers(&conn, "%", 20).unwrap().len(), 3);
        assert_eq!(search_customers(&conn, "a_me", 20).unwrap().len(), 0);
    }

    #[test]
    fn changing_the_vat_number_resets_the_cached_peppol_status() {
        let conn = setup();
        let id = insert_customer(&conn, &customer("BE0308357159", "Acme"), "u").unwrap();
        set_customer_peppol_status(&conn, id, "registered", "2026-10-17T10:00:00Z").unwrap();
        let c = get_customer(&conn, id).unwrap().unwrap();
        assert_eq!(c.peppol_status, "registered");
        assert_eq!(c.peppol_checked_at.as_deref(), Some("2026-10-17T10:00:00Z"));
        // Same VAT: the status stays.
        assert!(update_customer(&conn, id, &customer("BE0308357159", "Acme NV"), "u").unwrap());
        assert_eq!(
            get_customer(&conn, id).unwrap().unwrap().peppol_status,
            "registered"
        );
        // New VAT: back to unknown.
        assert!(update_customer(&conn, id, &customer("BE0499999960", "Acme NV"), "u").unwrap());
        let c = get_customer(&conn, id).unwrap().unwrap();
        assert_eq!(
            (c.peppol_status.as_str(), c.peppol_checked_at),
            ("unknown", None)
        );
        assert!(!update_customer(&conn, 999, &customer("BE1", "x"), "u").unwrap());
    }

    #[test]
    fn invoices_number_continuously_and_a_sale_is_on_one_invoice() {
        let conn = setup();
        assert_eq!(next_invoice_seq(&conn).unwrap(), 1);
        let (s1, s2, s3) = (
            event(&conn, 1, "N"),
            event(&conn, 2, "N"),
            event(&conn, 3, "N"),
        );
        let (i1, i2) = (event(&conn, 4, "I"), event(&conn, 5, "I"));
        let new = |seq, no, ev, sales: &'static [i64]| NewInvoice {
            invoice_seq: seq,
            invoice_no: no,
            fiscal_event_id: ev,
            customer_id: None,
            customer_json: "{}",
            issue_date: "2026-10-17",
            total_cents: 100,
            vat_json: "{}",
            created_by: "u",
            sale_event_ids: sales,
        };
        let sales_a: &'static [i64] = Box::leak(vec![s1, s2].into_boxed_slice());
        let sales_b: &'static [i64] = Box::leak(vec![s3, s1].into_boxed_slice());
        let sales_c: &'static [i64] = Box::leak(vec![s3].into_boxed_slice());
        insert_invoice(&conn, &new(1, "20260001", i1, sales_a)).unwrap();
        assert_eq!(next_invoice_seq(&conn).unwrap(), 2);
        assert_eq!(
            invoice_of_sale(&conn, s1).unwrap().as_deref(),
            Some("20260001")
        );
        assert_eq!(invoice_of_sale(&conn, s3).unwrap(), None);
        // s1 cannot be on a second invoice; the whole insert fails.
        assert!(insert_invoice(&conn, &new(2, "20260002", i2, sales_b)).is_err());
        insert_invoice(&conn, &new(2, "20260002", i2, sales_c)).unwrap();
        let listed = list_invoices(&conn, 10).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|i| i.invoice_no.as_str())
                .collect::<Vec<_>>(),
            ["20260002", "20260001"]
        );
        assert_eq!(listed[1].sale_event_ids, vec![s1, s2]);
        assert_eq!(
            invoice_of_event(&conn, i1).unwrap().unwrap().invoice_no,
            "20260001"
        );
        assert!(invoice_of_event(&conn, s1).unwrap().is_none());
    }

    #[test]
    fn the_queue_runs_queued_to_accepted_or_parked_and_logs_every_attempt() {
        let conn = setup();
        let i = event(&conn, 1, "I");
        let d = insert_delivery(
            &conn,
            &NewDelivery {
                fiscal_event_id: i,
                invoice_no: "20260001",
                customer_endpoint_id: "0208:0308357159",
                ap_vendor: "storecove",
                ubl_xml: "<x/>",
                lookup_status: "unknown",
                now: "2026-10-17T10:00:00Z",
            },
        )
        .unwrap();
        assert_eq!(delivery_ubl(&conn, d).unwrap().as_deref(), Some("<x/>"));
        // Due at once, not before.
        assert_eq!(
            due_deliveries(&conn, "2026-10-17T10:00:00Z").unwrap().len(),
            1
        );
        assert_eq!(
            due_deliveries(&conn, "2026-10-17T09:59:59Z").unwrap().len(),
            0
        );

        mark_retry(
            &conn,
            d,
            1,
            "2026-10-17T10:00:30Z",
            "503",
            "2026-10-17T10:00:00Z",
        )
        .unwrap();
        log_attempt(&conn, d, 1, "retry", Some(503), Some("503")).unwrap();
        assert_eq!(
            due_deliveries(&conn, "2026-10-17T10:00:29Z").unwrap().len(),
            0
        );
        assert_eq!(
            due_deliveries(&conn, "2026-10-17T10:00:30Z").unwrap()[0].attempts,
            1
        );
        assert_eq!(queue_counts(&conn).unwrap(), (1, 0, 0));

        park(&conn, d, 2, "422 refused", "2026-10-17T10:01:00Z").unwrap();
        log_attempt(&conn, d, 2, "parked", Some(422), None).unwrap();
        let row = get_delivery(&conn, d).unwrap().unwrap();
        assert_eq!(
            (row.status.as_str(), row.error_msg.as_deref()),
            ("failed", Some("422 refused"))
        );
        assert!(row.parked_at.is_some() && row.next_attempt_at.is_none());
        assert_eq!(
            due_deliveries(&conn, "2030-01-01T00:00:00Z").unwrap().len(),
            0
        );
        assert_eq!(queue_counts(&conn).unwrap(), (0, 0, 1));

        assert!(requeue(&conn, d, "2026-10-17T11:00:00Z").unwrap());
        let row = get_delivery(&conn, d).unwrap().unwrap();
        assert_eq!(
            (row.status.as_str(), row.attempts, row.parked_at),
            ("queued", 2, None)
        );
        // Only a parked row can be re-queued.
        assert!(!requeue(&conn, d, "2026-10-17T11:00:00Z").unwrap());

        mark_accepted(&conn, d, 3, "guid-1", "{}", "2026-10-17T11:00:05Z").unwrap();
        log_attempt(&conn, d, 3, "accepted", Some(200), None).unwrap();
        let row = get_delivery(&conn, d).unwrap().unwrap();
        assert_eq!(
            (row.status.as_str(), row.ap_message_id.as_deref()),
            ("accepted", Some("guid-1"))
        );
        assert_eq!(queue_counts(&conn).unwrap(), (0, 1, 0));
        let log = delivery_log(&conn, d).unwrap();
        assert_eq!(
            log.iter().map(|l| l.1.as_str()).collect::<Vec<_>>(),
            ["retry", "parked", "accepted"]
        );
        assert_eq!(log[0].2, Some(503));
        assert_eq!(list_deliveries(&conn, 10).unwrap().len(), 1);
        set_delivery_lookup(&conn, d, "registered").unwrap();
        assert_eq!(
            get_delivery(&conn, d).unwrap().unwrap().lookup_status,
            "registered"
        );
    }

    #[test]
    fn an_invoice_row_shows_the_status_of_its_delivery() {
        let conn = setup();
        let i = event(&conn, 1, "I");
        insert_invoice(
            &conn,
            &NewInvoice {
                invoice_seq: 1,
                invoice_no: "20260001",
                fiscal_event_id: i,
                customer_id: None,
                customer_json: "{}",
                issue_date: "2026-10-17",
                total_cents: 5,
                vat_json: "{}",
                created_by: "u",
                sale_event_ids: &[],
            },
        )
        .unwrap();
        assert_eq!(list_invoices(&conn, 5).unwrap()[0].delivery_status, None);
        let d = insert_delivery(
            &conn,
            &NewDelivery {
                fiscal_event_id: i,
                invoice_no: "20260001",
                customer_endpoint_id: "0208:1",
                ap_vendor: "logging",
                ubl_xml: "<x/>",
                lookup_status: "unknown",
                now: "2026-10-17T10:00:00Z",
            },
        )
        .unwrap();
        let row = list_invoices(&conn, 5).unwrap().remove(0);
        assert_eq!(
            (row.delivery_status.as_deref(), row.delivery_id),
            (Some("queued"), Some(d))
        );
    }
}
