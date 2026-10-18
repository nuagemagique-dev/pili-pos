//! Multi-purpose gift vouchers (migration 63).
//!
//! A voucher and its movements are append-only rows: the balance, the
//! cancellation and the expiry are computed. This module only moves
//! data; the rules (what a sale may redeem, the code, the settings)
//! live in `pos_engine::commands::gift_vouchers`.

use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

/// `menu_items.external_id` of the system product that sells a voucher.
pub const SYSTEM_ITEM_EXTERNAL_ID: &str = "pili:gift-voucher";

/// The department id of a voucher line (`departmentId`).
pub const DEPARTMENT_ID: &str = "GV";

/// The id of the system product (inactive, never on the grid).
pub fn system_item_id(conn: &Connection) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT id FROM menu_items WHERE external_id = ?1",
        params![SYSTEM_ITEM_EXTERNAL_ID],
        |r| r.get(0),
    )
}

/// The system product's id, creating its rows again if a reseed or a
/// manual clean-up removed them (the statements of migration 63 are
/// idempotent).
pub fn ensure_system_item(conn: &Connection) -> Result<i64, rusqlite::Error> {
    match system_item_id(conn) {
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            conn.execute_batch(crate::schema_owner::MIGRATION_063_GIFT_VOUCHERS)?;
            system_item_id(conn)
        }
        other => other,
    }
}

/// Whether a line sells a voucher: it is the system product's line.
pub fn is_voucher_line(conn: &Connection, line_id: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM order_lines ol
            JOIN menu_items m ON m.id = ol.menu_item_id
            WHERE ol.id = ?1 AND m.external_id = ?2)",
        params![line_id, SYSTEM_ITEM_EXTERNAL_ID],
        |r| r.get(0),
    )
}

/// The voucher lines of an order: `(line_id, unit_price_cents)`, in line order.
pub fn voucher_lines(conn: &Connection, order_id: i64) -> Result<Vec<(i64, i64)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT ol.id, ol.unit_price_cents FROM order_lines ol
         JOIN menu_items m ON m.id = ol.menu_item_id
         WHERE ol.order_id = ?1 AND m.external_id = ?2 ORDER BY ol.id",
    )?;
    let rows = stmt.query_map(params![order_id, SYSTEM_ITEM_EXTERNAL_ID], |r| {
        Ok((r.get(0)?, r.get(1)?))
    })?;
    rows.collect()
}

/// A voucher to create.
#[derive(Debug, Clone)]
pub struct NewVoucher<'a> {
    pub code: &'a str,
    pub face_value_cents: i64,
    pub issued_on: &'a str,
    pub expires_on: &'a str,
    pub training: bool,
    pub issue_event_id: i64,
    pub issue_order_id: i64,
    pub issue_line_id: i64,
}

/// Create a voucher and its `issue` movement (+ face value).
pub fn insert_voucher(conn: &Connection, v: &NewVoucher<'_>) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO gift_vouchers
            (code, face_value_cents, issued_on, expires_on, training,
             issue_event_id, issue_order_id, issue_line_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            v.code,
            v.face_value_cents,
            v.issued_on,
            v.expires_on,
            v.training,
            v.issue_event_id,
            v.issue_order_id,
            v.issue_line_id
        ],
    )?;
    let id = conn.last_insert_rowid();
    insert_movement(
        conn,
        id,
        "issue",
        v.face_value_cents,
        v.issue_event_id,
        Some(v.issue_order_id),
    )?;
    Ok(id)
}

/// Append a movement: `issue` (+face), `redeem` (-x) or `cancel` (-balance).
pub fn insert_movement(
    conn: &Connection,
    voucher_id: i64,
    kind: &str,
    amount_cents: i64,
    fiscal_event_id: i64,
    order_id: Option<i64>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO gift_voucher_movements
            (voucher_id, kind, amount_cents, fiscal_event_id, order_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![voucher_id, kind, amount_cents, fiscal_event_id, order_id],
    )?;
    Ok(())
}

/// A voucher with what is computed from its movements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VoucherRow {
    pub id: i64,
    pub code: String,
    pub face_value_cents: i64,
    pub issued_on: String,
    pub expires_on: String,
    pub training: bool,
    pub issue_event_id: i64,
    pub issue_order_id: i64,
    pub issue_line_id: i64,
    /// Face value minus what was redeemed (0 once cancelled).
    pub balance_cents: i64,
    pub cancelled: bool,
}

const SELECT_VOUCHER: &str = "SELECT g.id, g.code, g.face_value_cents, g.issued_on, g.expires_on,
        g.training, g.issue_event_id, g.issue_order_id, g.issue_line_id,
        COALESCE((SELECT SUM(m.amount_cents) FROM gift_voucher_movements m
                  WHERE m.voucher_id = g.id), 0),
        EXISTS (SELECT 1 FROM gift_voucher_movements m
                WHERE m.voucher_id = g.id AND m.kind = 'cancel')
     FROM gift_vouchers g";

fn map_voucher(r: &rusqlite::Row<'_>) -> rusqlite::Result<VoucherRow> {
    Ok(VoucherRow {
        id: r.get(0)?,
        code: r.get(1)?,
        face_value_cents: r.get(2)?,
        issued_on: r.get(3)?,
        expires_on: r.get(4)?,
        training: r.get(5)?,
        issue_event_id: r.get(6)?,
        issue_order_id: r.get(7)?,
        issue_line_id: r.get(8)?,
        balance_cents: r.get(9)?,
        cancelled: r.get(10)?,
    })
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<VoucherRow>, rusqlite::Error> {
    conn.query_row(
        &format!("{SELECT_VOUCHER} WHERE g.id = ?1"),
        params![id],
        map_voucher,
    )
    .optional()
}

pub fn find_by_code(conn: &Connection, code: &str) -> Result<Option<VoucherRow>, rusqlite::Error> {
    conn.query_row(
        &format!("{SELECT_VOUCHER} WHERE g.code = ?1"),
        params![code],
        map_voucher,
    )
    .optional()
}

/// How many real (non-training) vouchers can still be redeemed: not
/// cancelled, balance above zero, not expired on `today` (`YYYY-MM-DD`;
/// a voucher is usable through its expiry date).
pub fn count_redeemable(conn: &Connection, today: &str) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM gift_vouchers g
         WHERE g.training = 0 AND g.expires_on >= ?1
           AND NOT EXISTS (SELECT 1 FROM gift_voucher_movements m
                           WHERE m.voucher_id = g.id AND m.kind = 'cancel')
           AND COALESCE((SELECT SUM(m.amount_cents) FROM gift_voucher_movements m
                         WHERE m.voucher_id = g.id), 0) > 0",
        params![today],
        |r| r.get(0),
    )
}

/// The voucher a sale line created, if the sale was signed.
pub fn find_by_line(
    conn: &Connection,
    line_id: i64,
) -> Result<Option<VoucherRow>, rusqlite::Error> {
    conn.query_row(
        &format!("{SELECT_VOUCHER} WHERE g.issue_line_id = ?1"),
        params![line_id],
        map_voucher,
    )
    .optional()
}

/// Every voucher, newest first (the caller filters by status).
pub fn list(conn: &Connection, limit: i64) -> Result<Vec<VoucherRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("{SELECT_VOUCHER} ORDER BY g.id DESC LIMIT ?1"))?;
    let rows = stmt.query_map(params![limit], map_voucher)?;
    rows.collect()
}

/// One movement of a voucher, with the ticket that made it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MovementRow {
    pub kind: String,
    pub amount_cents: i64,
    /// The local date of the movement (`YYYY-MM-DD`, UTC clock of the row).
    pub date: String,
    /// The ticket: label and number of the fiscal event (`N 123`).
    pub ticket: String,
}

pub fn movements(conn: &Connection, voucher_id: i64) -> Result<Vec<MovementRow>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT m.kind, m.amount_cents, substr(m.created_at, 1, 10),
                e.event_type || ' ' || e.sequence_number
         FROM gift_voucher_movements m
         JOIN fiscal_events e ON e.id = m.fiscal_event_id
         WHERE m.voucher_id = ?1 ORDER BY m.id",
    )?;
    let rows = stmt.query_map(params![voucher_id], |r| {
        Ok(MovementRow {
            kind: r.get(0)?,
            amount_cents: r.get(1)?,
            date: r.get(2)?,
            ticket: r.get(3)?,
        })
    })?;
    rows.collect()
}

/// The ticket of a fiscal event as the movements show it (`N 1234`).
pub fn event_ticket(conn: &Connection, event_id: i64) -> Result<String, rusqlite::Error> {
    conn.query_row(
        "SELECT event_type || ' ' || sequence_number FROM fiscal_events WHERE id = ?1",
        params![event_id],
        |r| r.get(0),
    )
}

/// Log a print of a voucher slip (`original`, `duplicate` or `balance`).
pub fn record_print(
    conn: &Connection,
    voucher_id: i64,
    kind: &str,
    printed_by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO gift_voucher_prints (voucher_id, kind, printed_by) VALUES (?1, ?2, ?3)",
        params![voucher_id, kind, printed_by],
    )?;
    Ok(())
}

/// How many prints of `kind` the voucher has.
pub fn count_prints(
    conn: &Connection,
    voucher_id: i64,
    kind: &str,
) -> Result<i64, rusqlite::Error> {
    conn.query_row(
        "SELECT COUNT(*) FROM gift_voucher_prints WHERE voucher_id = ?1 AND kind = ?2",
        params![voucher_id, kind],
        |r| r.get(0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO orders (id, order_type, status, user_insz, created_at)
                 VALUES (1, 'takeaway', 'open', '00000000029', '2026-10-17T10:00:00Z');
             INSERT INTO fiscal_events (id, event_type, sequence_number, order_id, pos_id,
                                        terminal_id, user_insz, event_data_json)
                 VALUES (1, 'N', 1, 1, 'pos', 'term', '00000000029', '{}'),
                        (2, 'N', 2, 1, 'pos', 'term', '00000000029', '{}');",
        )
        .unwrap();
        conn
    }

    fn voucher(conn: &Connection, code: &str, line: i64) -> i64 {
        insert_voucher(
            conn,
            &NewVoucher {
                code,
                face_value_cents: 5000,
                issued_on: "2026-10-17",
                expires_on: "2027-10-17",
                training: false,
                issue_event_id: 1,
                issue_order_id: 1,
                issue_line_id: line,
            },
        )
        .unwrap()
    }

    #[test]
    fn the_system_item_exists_inactive_with_vat_x() {
        let conn = db();
        let id = system_item_id(&conn).unwrap();
        let (vat, takeaway, cat, kitchen, open, active): (String, String, String, i64, i64, i64) =
            conn.query_row(
                "SELECT vat_code, vat_code_takeaway, vat_category, kitchen_printer, open_price, active
                 FROM menu_items WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            (
                vat.as_str(),
                takeaway.as_str(),
                cat.as_str(),
                kitchen,
                open,
                active
            ),
            ("X", "X", "out_of_scope", 0, 1, 0)
        );
        let category_active: i64 = conn
            .query_row(
                "SELECT c.active FROM menu_items m JOIN categories c ON c.id = m.category_id WHERE m.id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(category_active, 0);
    }

    #[test]
    fn the_system_item_comes_back_after_a_reseed() {
        let conn = db();
        conn.execute_batch(
            "DELETE FROM translations WHERE entity_type IN ('category', 'menu_item');
             DELETE FROM menu_items; DELETE FROM categories;",
        )
        .unwrap();
        assert!(system_item_id(&conn).is_err());
        assert_eq!(ensure_system_item(&conn).unwrap(), 990_001);
        // Idempotent.
        assert_eq!(ensure_system_item(&conn).unwrap(), 990_001);
    }

    #[test]
    fn an_issued_voucher_has_its_face_value_as_balance() {
        let conn = db();
        let id = voucher(&conn, "123456789012", 7);
        let v = get(&conn, id).unwrap().unwrap();
        assert_eq!((v.balance_cents, v.cancelled), (5000, false));
        assert_eq!(find_by_code(&conn, "123456789012").unwrap().unwrap().id, id);
        assert_eq!(find_by_line(&conn, 7).unwrap().unwrap().id, id);
        assert!(find_by_code(&conn, "000000000000").unwrap().is_none());
    }

    #[test]
    fn redeeming_and_cancelling_move_the_balance() {
        let conn = db();
        let id = voucher(&conn, "123456789012", 7);
        insert_movement(&conn, id, "redeem", -1850, 2, Some(1)).unwrap();
        assert_eq!(get(&conn, id).unwrap().unwrap().balance_cents, 3150);
        insert_movement(&conn, id, "cancel", -3150, 2, None).unwrap();
        let v = get(&conn, id).unwrap().unwrap();
        assert_eq!((v.balance_cents, v.cancelled), (0, true));
        let kinds: Vec<_> = movements(&conn, id)
            .unwrap()
            .into_iter()
            .map(|m| (m.kind, m.amount_cents, m.ticket))
            .collect();
        assert_eq!(
            kinds,
            [
                ("issue".to_string(), 5000, "N 1".to_string()),
                ("redeem".to_string(), -1850, "N 2".to_string()),
                ("cancel".to_string(), -3150, "N 2".to_string()),
            ]
        );
    }

    #[test]
    fn the_rows_are_append_only_and_the_code_and_line_are_unique() {
        let conn = db();
        let id = voucher(&conn, "123456789012", 7);
        for sql in [
            "UPDATE gift_vouchers SET face_value_cents = 1",
            "DELETE FROM gift_vouchers",
            "UPDATE gift_voucher_movements SET amount_cents = 1",
            "DELETE FROM gift_voucher_movements",
        ] {
            assert!(conn.execute(sql, []).is_err(), "{sql}");
        }
        record_print(&conn, id, "original", Some("00000000029")).unwrap();
        for sql in [
            "UPDATE gift_voucher_prints SET kind = 'duplicate'",
            "DELETE FROM gift_voucher_prints",
        ] {
            assert!(conn.execute(sql, []).is_err(), "{sql}");
        }
        assert_eq!(count_prints(&conn, id, "original").unwrap(), 1);
        // Same code or same line twice: refused.
        let again = |code: &str, line: i64| {
            insert_voucher(
                &conn,
                &NewVoucher {
                    code,
                    face_value_cents: 100,
                    issued_on: "2026-10-17",
                    expires_on: "2027-10-17",
                    training: false,
                    issue_event_id: 1,
                    issue_order_id: 1,
                    issue_line_id: line,
                },
            )
        };
        assert!(again("123456789012", 8).is_err());
        assert!(again("999999999999", 7).is_err());
        // Face value must be positive; kinds are checked.
        assert!(
            conn.execute(
                "INSERT INTO gift_voucher_movements (voucher_id, kind, amount_cents, fiscal_event_id)
                 VALUES (?1, 'gift', 1, 1)",
                [id]
            )
            .is_err()
        );
    }

    #[test]
    fn a_movement_needs_a_real_event_and_voucher() {
        let conn = db();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let id = voucher(&conn, "123456789012", 7);
        assert!(insert_movement(&conn, id, "redeem", -1, 99, None).is_err());
        assert!(insert_movement(&conn, 99, "redeem", -1, 1, None).is_err());
    }
}
