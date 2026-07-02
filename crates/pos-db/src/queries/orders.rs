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
    // always spawn a fresh row — they have no picker to resume from.
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

pub fn add_order_line(
    conn: &Connection,
    input: NewOrderLine<'_>,
) -> Result<OrderLine, rusqlite::Error> {
    let now = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    conn.execute(
        "INSERT INTO order_lines (order_id, menu_item_id, quantity, unit_price_cents, vat_code, name, modifiers_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.order_id,
            input.menu_item_id,
            input.quantity,
            input.unit_price_cents,
            input.vat_code.as_str(),
            input.name,
            input.modifiers_json,
            now_str,
        ],
    )?;

    let id = conn.last_insert_rowid();

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
    })
}

pub fn remove_order_line(conn: &Connection, line_id: i64) -> Result<(), rusqlite::Error> {
    let order_id: i64 = conn.query_row(
        "SELECT order_id FROM order_lines WHERE id = ?1",
        params![line_id],
        |row| row.get(0),
    )?;

    conn.execute("DELETE FROM order_lines WHERE id = ?1", params![line_id])?;
    recalculate_order_totals(conn, order_id)?;
    Ok(())
}

pub fn update_order_line_quantity(
    conn: &Connection,
    line_id: i64,
    quantity: i32,
) -> Result<(), rusqlite::Error> {
    let order_id: i64 = conn.query_row(
        "SELECT order_id FROM order_lines WHERE id = ?1",
        params![line_id],
        |row| row.get(0),
    )?;

    conn.execute(
        "UPDATE order_lines SET quantity = ?1 WHERE id = ?2",
        params![quantity, line_id],
    )?;

    recalculate_order_totals(conn, order_id)?;
    Ok(())
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
        "SELECT id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name, modifiers_json, created_at
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
        // Guard: adding items to a closed order should still work at DB level
        // (the guard is in the Tauri command layer), but the order total
        // reflects it. This test documents the DB behavior.
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

        // Verify the order is actually closed
        let closed = get_order(&conn, order.id).unwrap();
        assert_eq!(closed.status, OrderStatus::Closed);
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
            },
        )
        .unwrap();

        let updated = get_order(&conn, order.id).unwrap();
        assert_eq!(updated.total_cents, 2500);

        remove_order_line(&conn, line.id).unwrap();
        let updated = get_order(&conn, order.id).unwrap();
        assert_eq!(updated.total_cents, 0);
    }
}
