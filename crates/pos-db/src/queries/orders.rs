use chrono::{DateTime, Utc};
use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use pos_core::vat::VatCode;
use rusqlite::{Connection, params};

pub struct NewOrderLine<'a> {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub name: &'a str,
    pub modifiers_json: Option<&'a str>,
    pub department_id: &'a str,
    pub department_name: &'a str,
}

pub fn create_order(
    conn: &Connection,
    order_type: OrderType,
    table_number: Option<i32>,
    user_insz: &str,
) -> Result<Order, rusqlite::Error> {
    // Reuse an existing empty open order for the same dine-in table
    // instead of spawning a new row. Prevents zombie rows piling up
    // when the operator taps a table, walks away without adding any
    // item, and taps the same table again later. Takeaway orders
    // always spawn a fresh row - they have no picker to resume from.
    if let Some(t) = table_number {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT o.id
                 FROM orders o
                 LEFT JOIN order_lines ol ON ol.order_id = o.id
                 WHERE o.table_number = ?1
                   AND o.status = 'open'
                   AND o.order_type = 'dine_in'
                 GROUP BY o.id
                 HAVING COUNT(ol.id) = 0
                 LIMIT 1",
                params![t],
                |row| row.get(0),
            )
            .ok();
        if let Some(existing_id) = existing {
            return get_order(conn, existing_id);
        }
    }

    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    conn.execute(
        "INSERT INTO orders (table_number, order_type, status, user_insz, created_at, subtotal_cents, total_cents)
         VALUES (?1, ?2, 'open', ?3, ?4, 0, 0)",
        params![table_number, order_type.as_str(), user_insz, now_str],
    )?;

    let id = conn.last_insert_rowid();

    Ok(Order {
        id,
        table_number,
        order_type,
        status: OrderStatus::Open,
        user_insz: user_insz.to_string(),
        created_at: now,
        closed_at: None,
        subtotal_cents: 0,
        total_cents: 0,
    })
}

/// Add a line to an open order and journal it (`order_entries`).
pub fn add_order_line(
    conn: &Connection,
    input: NewOrderLine<'_>,
) -> Result<OrderLine, rusqlite::Error> {
    with_savepoint(conn, || {
        ensure_open(conn, input.order_id)?;
        add_order_line_inner(conn, input)
    })
}

fn add_order_line_inner(
    conn: &Connection,
    input: NewOrderLine<'_>,
) -> Result<OrderLine, rusqlite::Error> {
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    conn.execute(
        "INSERT INTO order_lines (order_id, menu_item_id, quantity, unit_price_cents, vat_code, name, modifiers_json, created_at, department_id, department_name)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            input.order_id,
            input.menu_item_id,
            input.quantity,
            input.unit_price_cents,
            input.vat_code.as_str(),
            input.name,
            input.modifiers_json,
            now_str,
            input.department_id,
            input.department_name,
        ],
    )?;

    let id = conn.last_insert_rowid();

    // Refund orders are the only ones built from negative lines.
    let kind = if input.quantity < 0 {
        "refund"
    } else {
        "entry"
    };
    insert_entry(conn, id, input.quantity, kind)?;
    recalculate_order_totals(conn, input.order_id)?;

    Ok(OrderLine {
        id,
        order_id: input.order_id,
        menu_item_id: input.menu_item_id,
        quantity: input.quantity,
        unit_price_cents: input.unit_price_cents,
        vat_code: input.vat_code,
        name: input.name.to_string(),
        modifiers_json: input.modifiers_json.map(|s| s.to_string()),
        created_at: now,
        department_id: input.department_id.to_string(),
        department_name: input.department_name.to_string(),
    })
}

/// Remove a line from an open order. The removal is journalled as a
/// correction (negative entry) so the FDM message keeps the history.
pub fn remove_order_line(conn: &Connection, line_id: i64) -> Result<(), rusqlite::Error> {
    with_savepoint(conn, || {
        let (order_id, quantity): (i64, i32) = conn.query_row(
            "SELECT order_id, quantity FROM order_lines WHERE id = ?1",
            params![line_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure_open(conn, order_id)?;
        if quantity != 0 {
            insert_entry(conn, line_id, -quantity, "correction")?;
        }
        conn.execute("DELETE FROM order_lines WHERE id = ?1", params![line_id])?;
        recalculate_order_totals(conn, order_id)
    })
}

/// Change a line's quantity on an open order, journalling the
/// difference (an addition, or a correction when it decreases).
pub fn update_order_line_quantity(
    conn: &Connection,
    line_id: i64,
    quantity: i32,
) -> Result<(), rusqlite::Error> {
    with_savepoint(conn, || {
        let (order_id, previous): (i64, i32) = conn.query_row(
            "SELECT order_id, quantity FROM order_lines WHERE id = ?1",
            params![line_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure_open(conn, order_id)?;
        let delta = quantity - previous;
        if delta != 0 {
            let kind = if delta > 0 { "entry" } else { "correction" };
            insert_entry(conn, line_id, delta, kind)?;
        }
        conn.execute(
            "UPDATE order_lines SET quantity = ?1 WHERE id = ?2",
            params![quantity, line_id],
        )?;
        recalculate_order_totals(conn, order_id)
    })
}

/// The order's journal, oldest first, as lines ready for an FDM
/// message: additions are positive, corrections and refunds negative.
/// `unsent_only` keeps the entries not yet transmitted in an event.
/// Orders recorded before the journal existed fall back to their lines.
pub fn get_order_entries(
    conn: &Connection,
    order_id: i64,
    unsent_only: bool,
) -> Result<Vec<OrderLine>, rusqlite::Error> {
    let sql = if unsent_only {
        "SELECT id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name,
                NULL, created_at, department_id, department_name
         FROM order_entries WHERE order_id = ?1 AND sent_event_id IS NULL ORDER BY id"
    } else {
        "SELECT id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name,
                NULL, created_at, department_id, department_name
         FROM order_entries WHERE order_id = ?1 ORDER BY id"
    };
    let mut stmt = conn.prepare(sql)?;
    let entries: Vec<OrderLine> = stmt
        .query_map(params![order_id], map_order_line)?
        .collect::<Result<_, _>>()?;
    if entries.is_empty() && !unsent_only {
        let journalled: i64 = conn.query_row(
            "SELECT COUNT(*) FROM order_entries WHERE order_id = ?1",
            params![order_id],
            |row| row.get(0),
        )?;
        if journalled == 0 {
            return get_order_lines(conn, order_id);
        }
    }
    Ok(entries)
}

/// Record that the order's entries up to `last_entry_id` were
/// transmitted in fiscal event `event_id`.
pub fn mark_order_entries_sent(
    conn: &Connection,
    order_id: i64,
    last_entry_id: i64,
    event_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE order_entries SET sent_event_id = ?1
         WHERE order_id = ?2 AND id <= ?3 AND sent_event_id IS NULL",
        params![event_id, order_id, last_entry_id],
    )?;
    Ok(())
}

fn insert_entry(
    conn: &Connection,
    line_id: i64,
    quantity: i32,
    kind: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO order_entries (order_id, line_id, menu_item_id, quantity, unit_price_cents,
                                    vat_code, name, department_id, department_name, kind, created_at)
         SELECT order_id, id, menu_item_id, ?2, unit_price_cents, vat_code, name,
                COALESCE(department_id, ''), COALESCE(department_name, ''), ?3, ?4
         FROM order_lines WHERE id = ?1",
        params![
            line_id,
            quantity,
            kind,
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
        ],
    )?;
    // A provisional bill no longer matches the order once it changes.
    conn.execute(
        "UPDATE orders SET pre_bill_at = NULL
         WHERE id = (SELECT order_id FROM order_lines WHERE id = ?1)",
        params![line_id],
    )?;
    Ok(())
}

/// Record that a provisional bill of the order was printed.
pub fn mark_pre_bill_printed(conn: &Connection, order_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE orders SET pre_bill_at = ?1 WHERE id = ?2",
        params![
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            order_id
        ],
    )?;
    Ok(())
}

/// A cart line as the order screen shows it: the consolidated line,
/// its product name in French and Chinese, and the quantity not yet
/// sent to the bar / kitchen (entries not yet in a signed P or N).
#[derive(Debug, Clone, serde::Serialize)]
pub struct CartLine {
    #[serde(flatten)]
    pub line: OrderLine,
    pub name_fr: Option<String>,
    pub name_zh: Option<String>,
    pub unsent_quantity: i64,
}

pub fn get_cart_lines(conn: &Connection, order_id: i64) -> Result<Vec<CartLine>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT ol.id, ol.order_id, ol.menu_item_id, ol.quantity, ol.unit_price_cents, ol.vat_code,
                ol.name, ol.modifiers_json, ol.created_at,
                COALESCE(ol.department_id, ''), COALESCE(ol.department_name, ''),
                (SELECT value FROM translations WHERE entity_type = 'menu_item'
                    AND entity_id = ol.menu_item_id AND field = 'name' AND lang = 'fr'),
                (SELECT value FROM translations WHERE entity_type = 'menu_item'
                    AND entity_id = ol.menu_item_id AND field = 'name' AND lang = 'zh'),
                COALESCE((SELECT SUM(e.quantity) FROM order_entries e
                          WHERE e.line_id = ol.id AND e.sent_event_id IS NULL), 0)
         FROM order_lines ol WHERE ol.order_id = ?1 ORDER BY ol.id",
    )?;
    let rows = stmt.query_map(params![order_id], |row| {
        Ok(CartLine {
            line: map_order_line(row)?,
            name_fr: row.get(11)?,
            name_zh: row.get(12)?,
            unsent_quantity: row.get(13)?,
        })
    })?;
    rows.collect()
}

/// Lines of closed or voided orders are part of a signed record and
/// can no longer change.
fn ensure_open(conn: &Connection, order_id: i64) -> Result<(), rusqlite::Error> {
    let status: String = conn.query_row(
        "SELECT status FROM orders WHERE id = ?1",
        params![order_id],
        |row| row.get(0),
    )?;
    if status != "open" {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
            Some(format!("order {order_id} is {status}")),
        ));
    }
    Ok(())
}

/// Run `f` inside a SAVEPOINT so a line change and its journal entry
/// are written together, whether or not the caller holds a transaction.
fn with_savepoint<T>(
    conn: &Connection,
    f: impl FnOnce() -> Result<T, rusqlite::Error>,
) -> Result<T, rusqlite::Error> {
    conn.execute_batch("SAVEPOINT order_change")?;
    match f() {
        Ok(v) => {
            conn.execute_batch("RELEASE order_change")?;
            Ok(v)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO order_change; RELEASE order_change");
            Err(e)
        }
    }
}

pub fn get_order(conn: &Connection, order_id: i64) -> Result<Order, rusqlite::Error> {
    conn.query_row(
        "SELECT id, table_number, order_type, status, user_insz, created_at, closed_at, subtotal_cents, total_cents
         FROM orders WHERE id = ?1",
        params![order_id],
        map_order,
    )
}

pub fn get_order_lines(
    conn: &Connection,
    order_id: i64,
) -> Result<Vec<OrderLine>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name, modifiers_json, created_at,
                COALESCE(department_id, ''), COALESCE(department_name, '')
         FROM order_lines WHERE order_id = ?1 ORDER BY id",
    )?;

    let rows = stmt.query_map(params![order_id], map_order_line)?;
    rows.collect()
}

pub fn list_open_orders(conn: &Connection) -> Result<Vec<Order>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, table_number, order_type, status, user_insz, created_at, closed_at, subtotal_cents, total_cents
         FROM orders WHERE status = 'open' ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([], map_order)?;
    rows.collect()
}

fn recalculate_order_totals(conn: &Connection, order_id: i64) -> Result<(), rusqlite::Error> {
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(unit_price_cents * quantity), 0) FROM order_lines WHERE order_id = ?1",
        params![order_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE orders SET subtotal_cents = ?1, total_cents = ?1 WHERE id = ?2",
        params![total, order_id],
    )?;

    Ok(())
}

fn map_order(row: &rusqlite::Row<'_>) -> Result<Order, rusqlite::Error> {
    let order_type_str: String = row.get(2)?;
    let status_str: String = row.get(3)?;
    let created_str: String = row.get(5)?;
    let closed_str: Option<String> = row.get(6)?;

    Ok(Order {
        id: row.get(0)?,
        table_number: row.get(1)?,
        order_type: order_type_str.parse().map_err(|e: String| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::from(e))
        })?,
        status: status_str.parse().map_err(|e: String| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::from(e))
        })?,
        user_insz: row.get(4)?,
        created_at: parse_datetime(&created_str),
        closed_at: closed_str.as_deref().map(parse_datetime),
        subtotal_cents: row.get(7)?,
        total_cents: row.get(8)?,
    })
}

fn map_order_line(row: &rusqlite::Row<'_>) -> Result<OrderLine, rusqlite::Error> {
    let vat_str: String = row.get(5)?;
    let created_str: String = row.get(8)?;

    Ok(OrderLine {
        id: row.get(0)?,
        order_id: row.get(1)?,
        menu_item_id: row.get(2)?,
        quantity: row.get(3)?,
        unit_price_cents: row.get(4)?,
        vat_code: vat_str.parse().map_err(|e: String| {
            rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::from(e))
        })?,
        name: row.get(6)?,
        modifiers_json: row.get(7)?,
        created_at: parse_datetime(&created_str),
        department_id: row.get(9)?,
        department_name: row.get(10)?,
    })
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%SZ")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map(|dt| dt.and_utc())
        .unwrap_or_default()
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

    #[test]
    fn create_and_get_order() {
        let conn = setup();
        let order = create_order(&conn, OrderType::DineIn, Some(5), "00000000029").unwrap();
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.table_number, Some(5));

        let fetched = get_order(&conn, order.id).unwrap();
        assert_eq!(fetched.id, order.id);
    }

    #[test]
    fn takeaway_order_has_no_table() {
        let conn = setup();
        let order = create_order(&conn, OrderType::Takeaway, None, "00000000029").unwrap();
        assert!(order.table_number.is_none());
        assert_eq!(order.order_type, OrderType::Takeaway);
    }

    #[test]
    fn reuse_empty_dine_in_order() {
        // Tapping the same table twice without adding items should
        // reuse the empty order instead of creating a second one.
        let conn = setup();
        let order1 = create_order(&conn, OrderType::DineIn, Some(3), "00000000029").unwrap();
        let order2 = create_order(&conn, OrderType::DineIn, Some(3), "00000000029").unwrap();
        assert_eq!(order1.id, order2.id, "should reuse the empty order");
    }

    #[test]
    fn no_reuse_when_order_has_lines() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1000, 'B')",
            [],
        ).unwrap();

        let order1 = create_order(&conn, OrderType::DineIn, Some(3), "00000000029").unwrap();
        add_order_line(
            &conn,
            NewOrderLine {
                order_id: order1.id,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: 1000,
                vat_code: VatCode::B,
                name: "Item",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();

        // Second call should create a NEW order since the first one has lines
        let order2 = create_order(&conn, OrderType::DineIn, Some(3), "00000000029").unwrap();
        assert_ne!(
            order1.id, order2.id,
            "should create new order when existing has lines"
        );
    }

    #[test]
    fn takeaway_never_reuses() {
        let conn = setup();
        let order1 = create_order(&conn, OrderType::Takeaway, None, "00000000029").unwrap();
        let order2 = create_order(&conn, OrderType::Takeaway, None, "00000000029").unwrap();
        assert_ne!(order1.id, order2.id, "takeaway always creates fresh orders");
    }

    #[test]
    fn update_quantity_recalculates_total() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1000, 'B')",
            [],
        ).unwrap();

        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        let line = add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: 1000,
                vat_code: VatCode::B,
                name: "Gyoza",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();

        assert_eq!(get_order(&conn, order.id).unwrap().total_cents, 1000);

        update_order_line_quantity(&conn, line.id, 3).unwrap();
        assert_eq!(get_order(&conn, order.id).unwrap().total_cents, 3000);
    }

    #[test]
    fn multiple_lines_sum_correctly() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1250, 'B')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (2, 1, 450, 'A')",
            [],
        ).unwrap();

        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 1,
                quantity: 2,
                unit_price_cents: 1250,
                vat_code: VatCode::B,
                name: "Saumon roll x8",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();
        add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 2,
                quantity: 1,
                unit_price_cents: 450,
                vat_code: VatCode::A,
                name: "Biere Asahi",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();

        let total = get_order(&conn, order.id).unwrap().total_cents;
        assert_eq!(total, 2 * 1250 + 450); // 2950
    }

    #[test]
    fn get_order_lines_returns_correct_items() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1000, 'B')",
            [],
        ).unwrap();

        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 1,
                quantity: 3,
                unit_price_cents: 1000,
                vat_code: VatCode::B,
                name: "Edamame",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();

        let lines = get_order_lines(&conn, order.id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].quantity, 3);
        assert_eq!(lines[0].name, "Edamame");
        assert_eq!(lines[0].vat_code, VatCode::B);
    }

    #[test]
    fn list_open_orders_excludes_closed() {
        let conn = setup();
        let o1 = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        let _o2 = create_order(&conn, OrderType::DineIn, Some(2), "00000000029").unwrap();

        // Close order 1
        conn.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            params![o1.id],
        )
        .unwrap();

        let open = list_open_orders(&conn).unwrap();
        assert_eq!(open.len(), 1);
        assert_ne!(open[0].id, o1.id);
    }

    #[test]
    fn closed_order_not_reused_for_same_table() {
        // After checkout, the order is closed. Tapping the same table
        // should create a fresh order, not reuse the closed one.
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1000, 'B')",
            [],
        ).unwrap();

        let order1 = create_order(&conn, OrderType::DineIn, Some(5), "00000000029").unwrap();
        add_order_line(
            &conn,
            NewOrderLine {
                order_id: order1.id,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: 1000,
                vat_code: VatCode::B,
                name: "Item",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();

        // Simulate checkout: close the order
        conn.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            params![order1.id],
        )
        .unwrap();

        // Tapping table 5 again should get a fresh order
        let order2 = create_order(&conn, OrderType::DineIn, Some(5), "00000000029").unwrap();
        assert_ne!(order1.id, order2.id, "closed order should not be reused");
        assert_eq!(order2.status, OrderStatus::Open);
        assert_eq!(order2.total_cents, 0);
    }

    #[test]
    fn cannot_add_line_to_closed_order() {
        // `add_order_line` calls `ensure_open` under the same savepoint
        // as the insert, so - unlike the stale comment this test used
        // to carry - the DB layer itself rejects the write; it is not
        // solely a Tauri-command-layer guard.
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1000, 'B')",
            [],
        ).unwrap();

        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        conn.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now') WHERE id = ?1",
            params![order.id],
        )
        .unwrap();

        let closed = get_order(&conn, order.id).unwrap();
        assert_eq!(closed.status, OrderStatus::Closed);

        let result = add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: 1000,
                vat_code: VatCode::B,
                name: "Item",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        );
        assert!(result.is_err(), "adding a line to a closed order must fail");
        // No half-written line or journal entry from the rejected attempt.
        assert!(get_order_lines(&conn, order.id).unwrap().is_empty());
        assert!(
            get_order_entries(&conn, order.id, false)
                .unwrap()
                .is_empty()
        );
        assert_eq!(get_order(&conn, order.id).unwrap().total_cents, 0);
    }

    #[test]
    fn add_and_remove_lines() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1250, 'B')",
            [],
        ).unwrap();

        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        let line = add_order_line(
            &conn,
            NewOrderLine {
                order_id: order.id,
                menu_item_id: 1,
                quantity: 2,
                unit_price_cents: 1250,
                vat_code: VatCode::B,
                name: "Saumon roll",
                modifiers_json: None,
                department_id: "1",
                department_name: "Test",
            },
        )
        .unwrap();

        let updated = get_order(&conn, order.id).unwrap();
        assert_eq!(updated.total_cents, 2500);

        remove_order_line(&conn, line.id).unwrap();
        let updated = get_order(&conn, order.id).unwrap();
        assert_eq!(updated.total_cents, 0);
    }

    fn cola(conn: &Connection, order_id: i64, quantity: i32) -> OrderLine {
        conn.execute_batch(
            "INSERT OR IGNORE INTO categories (id, sort_order) VALUES (12, 0);
             INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order)
                 VALUES (12009, 12, 304, 'A', 0);",
        )
        .unwrap();
        add_order_line(
            conn,
            NewOrderLine {
                order_id,
                menu_item_id: 12009,
                quantity,
                unit_price_cents: 304,
                vat_code: VatCode::A,
                name: "Coca Cola",
                modifiers_json: None,
                department_id: "12",
                department_name: "Soft Drinks",
            },
        )
        .unwrap()
    }

    fn quantities(entries: &[OrderLine]) -> Vec<i32> {
        entries.iter().map(|e| e.quantity).collect()
    }

    #[test]
    fn every_change_is_journalled_in_order() {
        let conn = setup();
        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        let line = cola(&conn, order.id, 1);
        update_order_line_quantity(&conn, line.id, 3).unwrap();
        update_order_line_quantity(&conn, line.id, 2).unwrap();
        remove_order_line(&conn, line.id).unwrap();

        let entries = get_order_entries(&conn, order.id, false).unwrap();
        assert_eq!(quantities(&entries), [1, 2, -1, -2]);
        assert!(entries.iter().all(|e| e.department_id == "12"));
        assert!(get_order_lines(&conn, order.id).unwrap().is_empty());
        assert_eq!(get_order(&conn, order.id).unwrap().total_cents, 0);
    }

    #[test]
    fn sent_entries_are_not_sent_again() {
        let conn = setup();
        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        let line = cola(&conn, order.id, 2);
        let first = get_order_entries(&conn, order.id, true).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = OFF;").unwrap();
        mark_order_entries_sent(&conn, order.id, first.last().unwrap().id, 1).unwrap();
        update_order_line_quantity(&conn, line.id, 3).unwrap();

        assert_eq!(
            quantities(&get_order_entries(&conn, order.id, true).unwrap()),
            [1]
        );
        assert_eq!(
            quantities(&get_order_entries(&conn, order.id, false).unwrap()),
            [2, 1]
        );
    }

    #[test]
    fn closed_orders_cannot_change() {
        let conn = setup();
        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        let line = cola(&conn, order.id, 1);
        conn.execute(
            "UPDATE orders SET status = 'closed' WHERE id = ?1",
            params![order.id],
        )
        .unwrap();
        assert!(update_order_line_quantity(&conn, line.id, 2).is_err());
        assert!(remove_order_line(&conn, line.id).is_err());
        assert_eq!(get_order_entries(&conn, order.id, false).unwrap().len(), 1);
    }

    #[test]
    fn journal_entries_are_append_only() {
        let conn = setup();
        let order = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        cola(&conn, order.id, 1);
        assert!(conn.execute("DELETE FROM order_entries", []).is_err());
        assert!(
            conn.execute("UPDATE order_entries SET quantity = 5", [])
                .is_err()
        );
    }
}
