//! The card payments taken on the linked terminal (migration 27):
//! stored before the terminal is called, resolved by its answer, used
//! once by the sale that settles them. See `schema_eft`.

use rusqlite::{Connection, OptionalExtension, Row, params};

/// Where a payment stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Asked of the terminal, no answer stored yet.
    Sent,
    Approved,
    Declined,
    Cancelled,
    /// No final answer: the card may or may not be charged.
    Unknown,
    /// The request never left (terminal busy, unreachable, refused).
    NotSent,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::Approved => "approved",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
            Self::NotSent => "not_sent",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "sent" => Self::Sent,
            "approved" => Self::Approved,
            "declined" => Self::Declined,
            "cancelled" => Self::Cancelled,
            "unknown" => Self::Unknown,
            "not_sent" => Self::NotSent,
            _ => return None,
        })
    }

    /// Whether the payment still waits for a final answer.
    pub fn is_open(self) -> bool {
        matches!(self, Self::Sent | Self::Unknown)
    }
}

/// Debit or credit, as the terminal says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    Debit,
    Credit,
    Unknown,
}

impl CardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debit => "debit",
            Self::Credit => "credit",
            Self::Unknown => "unknown",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "debit" => Self::Debit,
            "credit" => Self::Credit,
            "unknown" => Self::Unknown,
            _ => return None,
        })
    }
}

/// A payment to store before asking the terminal.
#[derive(Debug)]
pub struct NewPayment<'a> {
    pub reference: &'a str,
    pub order_id: i64,
    pub terminal_kind: &'a str,
    pub amount_cents: i64,
    pub user_insz: &'a str,
}

/// What the terminal said (or what stopped the request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Approved {
        card_type: CardKind,
        brand: Option<String>,
        terminal_reference: Option<String>,
        authorisation_code: Option<String>,
        masked_pan: Option<String>,
        customer_receipt: Option<String>,
    },
    Declined {
        reason: String,
    },
    Cancelled,
    Unknown {
        detail: String,
    },
    NotSent {
        detail: String,
    },
}

impl Resolution {
    fn status(&self) -> Status {
        match self {
            Self::Approved { .. } => Status::Approved,
            Self::Declined { .. } => Status::Declined,
            Self::Cancelled => Status::Cancelled,
            Self::Unknown { .. } => Status::Unknown,
            Self::NotSent { .. } => Status::NotSent,
        }
    }
}

/// One stored payment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EftPaymentRow {
    pub id: i64,
    pub reference: String,
    pub order_id: i64,
    pub terminal_kind: String,
    pub amount_cents: i64,
    pub status: Status,
    pub card_type: Option<CardKind>,
    pub brand: Option<String>,
    pub terminal_reference: Option<String>,
    pub authorisation_code: Option<String>,
    pub masked_pan: Option<String>,
    pub customer_receipt: Option<String>,
    pub detail: Option<String>,
    pub user_insz: String,
    pub created_at: String,
    pub resolved_at: Option<String>,
    pub used_event_id: Option<i64>,
}

impl EftPaymentRow {
    /// An approved payment no sale has settled yet.
    pub fn is_usable(&self) -> bool {
        self.status == Status::Approved && self.used_event_id.is_none()
    }
}

const COLUMNS: &str = "id, reference, order_id, terminal_kind, amount_cents, status, card_type, \
    brand, terminal_reference, authorisation_code, masked_pan, customer_receipt, detail, \
    user_insz, created_at, resolved_at, used_event_id";

fn corrupt(what: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("eft_payments.{what}: unknown value {value:?}").into(),
    )
}

fn read(r: &Row<'_>) -> rusqlite::Result<EftPaymentRow> {
    let status: String = r.get(5)?;
    let card_type: Option<String> = r.get(6)?;
    Ok(EftPaymentRow {
        id: r.get(0)?,
        reference: r.get(1)?,
        order_id: r.get(2)?,
        terminal_kind: r.get(3)?,
        amount_cents: r.get(4)?,
        status: Status::parse(&status).ok_or_else(|| corrupt("status", &status))?,
        card_type: card_type
            .map(|t| CardKind::parse(&t).ok_or_else(|| corrupt("card_type", &t)))
            .transpose()?,
        brand: r.get(7)?,
        terminal_reference: r.get(8)?,
        authorisation_code: r.get(9)?,
        masked_pan: r.get(10)?,
        customer_receipt: r.get(11)?,
        detail: r.get(12)?,
        user_insz: r.get(13)?,
        created_at: r.get(14)?,
        resolved_at: r.get(15)?,
        used_event_id: r.get(16)?,
    })
}

/// Store a payment as `sent`, before the terminal is called.
pub fn insert_sent(conn: &Connection, new: &NewPayment<'_>) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO eft_payments (reference, order_id, terminal_kind, amount_cents, user_insz)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new.reference,
            new.order_id,
            new.terminal_kind,
            new.amount_cents,
            new.user_insz
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<EftPaymentRow>> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM eft_payments WHERE id = ?1"),
        params![id],
        read,
    )
    .optional()
}

fn many(
    conn: &Connection,
    clause: &str,
    args: &[&dyn rusqlite::ToSql],
) -> rusqlite::Result<Vec<EftPaymentRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM eft_payments {clause} ORDER BY id"
    ))?;
    stmt.query_map(args, read)?.collect()
}

/// Store the terminal's answer for a payment that still waits for one
/// (`sent` or `unknown`). `Ok(false)` when it does not: nothing is
/// changed.
pub fn resolve(conn: &Connection, id: i64, resolution: &Resolution) -> rusqlite::Result<bool> {
    let status = resolution.status().as_str();
    let n = match resolution {
        Resolution::Approved {
            card_type,
            brand,
            terminal_reference,
            authorisation_code,
            masked_pan,
            customer_receipt,
        } => conn.execute(
            "UPDATE eft_payments SET status = ?2, card_type = ?3, brand = ?4,
                terminal_reference = ?5, authorisation_code = ?6, masked_pan = ?7,
                customer_receipt = ?8, detail = NULL, resolved_at = datetime('now')
             WHERE id = ?1 AND status IN ('sent','unknown')",
            params![
                id,
                status,
                card_type.as_str(),
                brand,
                terminal_reference,
                authorisation_code,
                masked_pan,
                customer_receipt
            ],
        )?,
        Resolution::Declined { reason: detail }
        | Resolution::NotSent { detail }
        | Resolution::Unknown { detail } => conn.execute(
            // An 'unknown' answer keeps the payment open: no resolved_at.
            "UPDATE eft_payments SET status = ?2, detail = ?3,
                resolved_at = CASE WHEN ?2 = 'unknown' THEN NULL ELSE datetime('now') END
             WHERE id = ?1 AND status IN ('sent','unknown')",
            params![id, status, detail],
        )?,
        Resolution::Cancelled => conn.execute(
            "UPDATE eft_payments SET status = ?2, resolved_at = datetime('now')
             WHERE id = ?1 AND status IN ('sent','unknown')",
            params![id, status],
        )?,
    };
    Ok(n == 1)
}

/// Every payment of the till still waiting for a final answer, oldest
/// first. The terminal takes one payment at a time: while there is one,
/// no other is asked and no sale of its order is signed.
pub fn open(conn: &Connection) -> rusqlite::Result<Vec<EftPaymentRow>> {
    many(conn, "WHERE status IN ('sent','unknown')", &[])
}

/// The payment in `sent`, if any: the one an abort is about.
pub fn in_flight(conn: &Connection) -> rusqlite::Result<Option<EftPaymentRow>> {
    Ok(many(conn, "WHERE status = 'sent'", &[])?.into_iter().next())
}

/// An order's payments waiting for a final answer.
pub fn open_for_order(conn: &Connection, order_id: i64) -> rusqlite::Result<Vec<EftPaymentRow>> {
    many(
        conn,
        "WHERE order_id = ?1 AND status IN ('sent','unknown')",
        &[&order_id],
    )
}

/// An order's approved payments no sale has settled.
pub fn usable_for_order(conn: &Connection, order_id: i64) -> rusqlite::Result<Vec<EftPaymentRow>> {
    many(
        conn,
        "WHERE order_id = ?1 AND status = 'approved' AND used_event_id IS NULL",
        &[&order_id],
    )
}

/// Every approved payment of the till no sale has settled, oldest
/// first: money taken that no ticket records yet.
pub fn unsettled(conn: &Connection) -> rusqlite::Result<Vec<EftPaymentRow>> {
    many(
        conn,
        "WHERE status = 'approved' AND used_event_id IS NULL",
        &[],
    )
}

/// What the checkout screen shows of an order: its approved payments
/// not yet settled and those still waiting for an answer, oldest first.
pub fn pending_for_order(conn: &Connection, order_id: i64) -> rusqlite::Result<Vec<EftPaymentRow>> {
    many(
        conn,
        "WHERE order_id = ?1
           AND ((status = 'approved' AND used_event_id IS NULL) OR status IN ('sent','unknown'))",
        &[&order_id],
    )
}

/// Settle an approved payment in the sale `event_id`. `Ok(false)` when
/// it is not approved or was already settled: nothing is changed.
pub fn mark_used(conn: &Connection, id: i64, event_id: i64) -> rusqlite::Result<bool> {
    let n = conn.execute(
        "UPDATE eft_payments SET used_event_id = ?2
         WHERE id = ?1 AND status = 'approved' AND used_event_id IS NULL",
        params![id, event_id],
    )?;
    Ok(n == 1)
}

/// At start-up: a payment still `sent` lost its answer with the crash.
/// It becomes `unknown`, to be asked again. Returns how many.
pub fn sent_to_unknown(conn: &Connection) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE eft_payments
         SET status = 'unknown', detail = 'the till stopped before the terminal answered'
         WHERE status = 'sent'",
        [],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::run_migrations(&conn).unwrap();
        conn
    }

    fn order(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO orders (order_type, status, user_insz) VALUES ('takeaway', 'open', '00000000029')",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn event(conn: &Connection, seq: i64) -> i64 {
        conn.execute(
            "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
             VALUES ('N', ?1, 'P', 'T1', '00000000029', '{}')",
            [seq],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn sent(conn: &Connection, order_id: i64, reference: &str, cents: i64) -> i64 {
        insert_sent(
            conn,
            &NewPayment {
                reference,
                order_id,
                terminal_kind: "ccv_opi",
                amount_cents: cents,
                user_insz: "00000000029",
            },
        )
        .unwrap()
    }

    fn approval() -> Resolution {
        Resolution::Approved {
            card_type: CardKind::Debit,
            brand: Some("Bancontact".into()),
            terminal_reference: Some("T-1".into()),
            authorisation_code: Some("A1".into()),
            masked_pan: Some("XXXX 1234".into()),
            customer_receipt: None,
        }
    }

    fn fails(conn: &Connection, sql: &str) -> String {
        conn.execute(sql, []).unwrap_err().to_string()
    }

    #[test]
    fn a_payment_starts_sent_and_is_read_back() {
        let conn = db();
        let o = order(&conn);
        let id = sent(&conn, o, "ref-1", 1250);
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, Status::Sent);
        assert_eq!(row.amount_cents, 1250);
        assert_eq!(row.terminal_kind, "ccv_opi");
        assert_eq!(row.used_event_id, None);
        assert!(!row.is_usable());
        assert_eq!(get(&conn, id + 1).unwrap(), None);
        assert_eq!(in_flight(&conn).unwrap().unwrap().id, id);
    }

    #[test]
    fn the_amount_and_the_reference_are_checked() {
        let conn = db();
        let o = order(&conn);
        sent(&conn, o, "ref-1", 100);
        assert!(
            insert_sent(
                &conn,
                &NewPayment {
                    reference: "ref-1",
                    order_id: o,
                    terminal_kind: "ccv_opi",
                    amount_cents: 100,
                    user_insz: "00000000029",
                }
            )
            .is_err(),
            "reference is unique"
        );
        for cents in [0, -5] {
            assert!(
                insert_sent(
                    &conn,
                    &NewPayment {
                        reference: "ref-x",
                        order_id: o,
                        terminal_kind: "ccv_opi",
                        amount_cents: cents,
                        user_insz: "00000000029",
                    }
                )
                .is_err(),
                "{cents} is not an amount"
            );
        }
        assert!(
            insert_sent(
                &conn,
                &NewPayment {
                    reference: "ref-y",
                    order_id: 9999,
                    terminal_kind: "ccv_opi",
                    amount_cents: 100,
                    user_insz: "00000000029",
                }
            )
            .is_err(),
            "the order must exist"
        );
    }

    #[test]
    fn a_row_cannot_start_in_another_status() {
        let conn = db();
        let o = order(&conn);
        let e = fails(
            &conn,
            &format!(
                "INSERT INTO eft_payments (reference, order_id, terminal_kind, amount_cents, status, card_type, user_insz)
                 VALUES ('r', {o}, 'ccv_opi', 100, 'approved', 'debit', '00000000029')"
            ),
        );
        assert!(e.contains("start as sent"), "{e}");
    }

    #[test]
    fn nothing_is_ever_deleted() {
        let conn = db();
        let o = order(&conn);
        let id = sent(&conn, o, "ref-1", 100);
        let e = fails(&conn, &format!("DELETE FROM eft_payments WHERE id = {id}"));
        assert!(e.contains("never deleted"), "{e}");
        resolve(&conn, id, &approval()).unwrap();
        assert!(conn.execute("DELETE FROM eft_payments", []).is_err());
    }

    #[test]
    fn an_approval_stores_the_card_data_and_is_usable() {
        let conn = db();
        let o = order(&conn);
        let id = sent(&conn, o, "ref-1", 1250);
        assert!(resolve(&conn, id, &approval()).unwrap());
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, Status::Approved);
        assert_eq!(row.card_type, Some(CardKind::Debit));
        assert_eq!(row.brand.as_deref(), Some("Bancontact"));
        assert_eq!(row.terminal_reference.as_deref(), Some("T-1"));
        assert!(row.resolved_at.is_some());
        assert!(row.is_usable());
        assert_eq!(usable_for_order(&conn, o).unwrap().len(), 1);
        assert_eq!(pending_for_order(&conn, o).unwrap().len(), 1);
        assert_eq!(open(&conn).unwrap().len(), 0);
    }

    #[test]
    fn an_approved_payment_always_has_a_card_type() {
        let conn = db();
        let o = order(&conn);
        let id = sent(&conn, o, "ref-1", 100);
        let e = fails(
            &conn,
            &format!("UPDATE eft_payments SET status = 'approved' WHERE id = {id}"),
        );
        assert!(e.contains("CHECK"), "{e}");
    }

    #[test]
    fn declined_cancelled_and_not_sent_are_final() {
        let conn = db();
        let o = order(&conn);
        for (i, resolution) in [
            Resolution::Declined {
                reason: "insufficient funds".into(),
            },
            Resolution::Cancelled,
            Resolution::NotSent {
                detail: "terminal busy".into(),
            },
        ]
        .into_iter()
        .enumerate()
        {
            let id = sent(&conn, o, &format!("ref-{i}"), 100);
            assert!(resolve(&conn, id, &resolution).unwrap());
            let status = get(&conn, id).unwrap().unwrap().status;
            // Asked again: nothing changes, nothing fails.
            assert!(!resolve(&conn, id, &approval()).unwrap());
            assert_eq!(get(&conn, id).unwrap().unwrap().status, status);
            // And the database itself refuses it.
            for to in ["sent", "approved", "unknown", "declined", "cancelled"] {
                if to == status.as_str() {
                    continue;
                }
                let extra = if to == "approved" {
                    ", card_type = 'debit'"
                } else {
                    ""
                };
                assert!(
                    conn.execute(
                        &format!("UPDATE eft_payments SET status = '{to}'{extra} WHERE id = {id}"),
                        []
                    )
                    .is_err(),
                    "{status:?} -> {to}"
                );
            }
            assert!(
                conn.execute(
                    &format!("UPDATE eft_payments SET detail = 'edited' WHERE id = {id}"),
                    []
                )
                .is_err(),
                "the detail of a final answer is kept"
            );
            assert!(
                conn.execute(
                    &format!("UPDATE eft_payments SET used_event_id = 1 WHERE id = {id}"),
                    []
                )
                .is_err(),
                "only an approved payment is used"
            );
        }
        assert_eq!(
            get(&conn, 1).unwrap().unwrap().detail.as_deref(),
            Some("insufficient funds")
        );
    }

    #[test]
    fn an_unknown_payment_moves_on_but_never_back_to_sent() {
        let conn = db();
        let o = order(&conn);
        let id = sent(&conn, o, "ref-1", 100);
        assert!(
            resolve(
                &conn,
                id,
                &Resolution::Unknown {
                    detail: "timeout".into()
                }
            )
            .unwrap()
        );
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.status, Status::Unknown);
        assert_eq!(row.resolved_at, None, "still open");
        assert_eq!(open(&conn).unwrap().len(), 1);
        assert_eq!(open_for_order(&conn, o).unwrap().len(), 1);
        assert!(in_flight(&conn).unwrap().is_none());
        let e = fails(
            &conn,
            &format!("UPDATE eft_payments SET status = 'sent' WHERE id = {id}"),
        );
        assert!(e.contains("not allowed"), "{e}");
        // Asked again: still unknown, the detail follows.
        assert!(
            resolve(
                &conn,
                id,
                &Resolution::Unknown {
                    detail: "still pending".into()
                }
            )
            .unwrap()
        );
        assert_eq!(
            get(&conn, id).unwrap().unwrap().detail.as_deref(),
            Some("still pending")
        );
        // Then the terminal answers.
        assert!(resolve(&conn, id, &approval()).unwrap());
        assert_eq!(get(&conn, id).unwrap().unwrap().status, Status::Approved);
        assert_eq!(get(&conn, id).unwrap().unwrap().detail, None);
    }

    #[test]
    fn an_approved_payment_never_changes_but_is_used_once() {
        let conn = db();
        let o = order(&conn);
        let other = order(&conn);
        let id = sent(&conn, o, "ref-1", 1250);
        resolve(&conn, id, &approval()).unwrap();
        for set in [
            "amount_cents = 1",
            "brand = 'Visa'",
            "card_type = 'credit'",
            "terminal_reference = 'T-2'",
            "authorisation_code = 'A2'",
            "masked_pan = 'XXXX 9999'",
            "customer_receipt = 'x'",
            "detail = 'x'",
            "status = 'declined'",
            "status = 'unknown'",
            "status = 'cancelled'",
            &format!("order_id = {other}"),
            "reference = 'other'",
            "user_insz = '1'",
            "resolved_at = '2000-01-01'",
        ] {
            assert!(
                conn.execute(
                    &format!("UPDATE eft_payments SET {set} WHERE id = {id}"),
                    []
                )
                .is_err(),
                "{set}"
            );
        }
        let e1 = event(&conn, 1);
        let e2 = event(&conn, 2);
        assert!(mark_used(&conn, id, e1).unwrap());
        let row = get(&conn, id).unwrap().unwrap();
        assert_eq!(row.used_event_id, Some(e1));
        assert!(!row.is_usable());
        assert_eq!(usable_for_order(&conn, o).unwrap().len(), 0);
        assert_eq!(pending_for_order(&conn, o).unwrap().len(), 0);
        // Once only: through the query nothing happens, by hand it is refused.
        assert!(!mark_used(&conn, id, e2).unwrap());
        for set in ["used_event_id = NULL", &format!("used_event_id = {e2}")] {
            let e = fails(
                &conn,
                &format!("UPDATE eft_payments SET {set} WHERE id = {id}"),
            );
            assert!(e.contains("used"), "{set}: {e}");
        }
        assert_eq!(get(&conn, id).unwrap().unwrap().used_event_id, Some(e1));
    }

    #[test]
    fn only_an_approved_payment_can_be_used() {
        let conn = db();
        let o = order(&conn);
        let e1 = event(&conn, 1);
        let id = sent(&conn, o, "ref-1", 100);
        assert!(!mark_used(&conn, id, e1).unwrap(), "sent");
        let e = fails(
            &conn,
            &format!("UPDATE eft_payments SET used_event_id = {e1} WHERE id = {id}"),
        );
        assert!(e.contains("only an approved payment"), "{e}");
        // Not even in the same statement as the approval.
        assert!(
            conn.execute(
                &format!(
                    "UPDATE eft_payments SET status = 'approved', card_type = 'debit',
                        used_event_id = {e1} WHERE id = {id}"
                ),
                []
            )
            .is_err()
        );
        assert_eq!(get(&conn, id).unwrap().unwrap().status, Status::Sent);
    }

    #[test]
    fn a_used_payment_must_name_an_existing_event() {
        let conn = db();
        let o = order(&conn);
        let id = sent(&conn, o, "ref-1", 100);
        resolve(&conn, id, &approval()).unwrap();
        assert!(mark_used(&conn, id, 424_242).is_err());
    }

    #[test]
    fn start_up_turns_sent_into_unknown_and_nothing_else() {
        let conn = db();
        let o = order(&conn);
        let a = sent(&conn, o, "ref-a", 100);
        let b = sent(&conn, o, "ref-b", 200);
        let c = sent(&conn, o, "ref-c", 300);
        resolve(&conn, a, &approval()).unwrap();
        resolve(
            &conn,
            b,
            &Resolution::Declined {
                reason: "no".into(),
            },
        )
        .unwrap();
        assert_eq!(sent_to_unknown(&conn).unwrap(), 1);
        assert_eq!(get(&conn, a).unwrap().unwrap().status, Status::Approved);
        assert_eq!(get(&conn, b).unwrap().unwrap().status, Status::Declined);
        let row = get(&conn, c).unwrap().unwrap();
        assert_eq!(row.status, Status::Unknown);
        assert!(row.detail.unwrap().contains("stopped"));
        assert_eq!(sent_to_unknown(&conn).unwrap(), 0);
    }

    #[test]
    fn the_checkout_list_is_the_unsettled_and_unresolved_oldest_first() {
        let conn = db();
        let o = order(&conn);
        let other = order(&conn);
        let first = sent(&conn, o, "ref-a", 100);
        let cancelled = sent(&conn, o, "ref-b", 200);
        let used = sent(&conn, o, "ref-c", 300);
        let pending = sent(&conn, o, "ref-d", 400);
        sent(&conn, other, "ref-e", 500);
        resolve(&conn, first, &approval()).unwrap();
        resolve(&conn, cancelled, &Resolution::Cancelled).unwrap();
        resolve(&conn, used, &approval()).unwrap();
        let event_id = event(&conn, 1);
        mark_used(&conn, used, event_id).unwrap();
        let ids: Vec<i64> = pending_for_order(&conn, o)
            .unwrap()
            .iter()
            .map(|r| r.id)
            .collect();
        assert_eq!(ids, vec![first, pending]);
        assert_eq!(
            open(&conn).unwrap().len(),
            2,
            "the pending one and the other order's"
        );
    }
}
