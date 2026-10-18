use chrono::{DateTime, Utc};
use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use pos_core::vat::VatCode;
use rusqlite::{Connection, OptionalExtension, params};

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
            // The line's discounts are corrected with it.
            #[cfg(feature = "hubrise")]
            super::price_changes::negate_on_correction(conn, line_id)?;
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
        // A discount was spread over the whole line: it is removed
        // whole or not at all.
        #[cfg(feature = "hubrise")]
        if quantity != previous && super::price_changes::line_has_changes(conn, line_id)? {
            return Err(rusqlite::Error::InvalidParameterName(
                "line_has_price_changes".into(),
            ));
        }
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

/// The entries fiscal event `event_id` transmitted.
pub fn get_entries_sent_in(
    conn: &Connection,
    event_id: i64,
) -> Result<Vec<OrderLine>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name,
                NULL, created_at, department_id, department_name
         FROM order_entries WHERE sent_event_id = ?1 ORDER BY id",
    )?;
    stmt.query_map(params![event_id], map_order_line)?.collect()
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
    insert_entry_op(conn, line_id, quantity, kind, None).map(|_| ())
}

/// [`insert_entry`] returning the entry's id, with the `till_op` mark of
/// a PRICE_CHANGE pair (`Some("price_change")`) when it is one.
fn insert_entry_op(
    conn: &Connection,
    line_id: i64,
    quantity: i32,
    kind: &str,
    till_op: Option<&str>,
) -> Result<i64, rusqlite::Error> {
    conn.execute(
        "INSERT INTO order_entries (order_id, line_id, menu_item_id, quantity, unit_price_cents,
                                    vat_code, name, department_id, department_name, kind, created_at,
                                    till_op)
         SELECT order_id, id, menu_item_id, ?2, unit_price_cents, vat_code, name,
                COALESCE(department_id, ''), COALESCE(department_name, ''), ?3, ?4, ?5
         FROM order_lines WHERE id = ?1",
        params![
            line_id,
            quantity,
            kind,
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            till_op
        ],
    )?;
    let entry_id = conn.last_insert_rowid();
    // A provisional bill no longer matches the order once it changes.
    conn.execute(
        "UPDATE orders SET pre_bill_at = NULL
         WHERE id = (SELECT order_id FROM order_lines WHERE id = ?1)",
        params![line_id],
    )?;
    Ok(entry_id)
}

/// The two journal entries of a PRICE_CHANGE pair on a line: a
/// correction of its whole quantity, then the same quantity again, both
/// marked `till_op` = `price_change`. The caller records the changes on
/// them (the first carries the line's existing changes negated, the
/// second the new ones). Returns `(correction_entry_id, reentry_entry_id)`.
#[cfg(feature = "hubrise")]
pub fn add_price_change_pair(
    conn: &Connection,
    line_id: i64,
) -> Result<(i64, i64), rusqlite::Error> {
    let (order_id, quantity): (i64, i32) = conn.query_row(
        "SELECT order_id, quantity FROM order_lines WHERE id = ?1",
        params![line_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure_open(conn, order_id)?;
    let correction = insert_entry_op(conn, line_id, -quantity, "correction", Some("price_change"))?;
    let reentry = insert_entry_op(conn, line_id, quantity, "entry", Some("price_change"))?;
    Ok((correction, reentry))
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
    /// The product's menu code ("A2"), when the catalog gives one.
    pub code: Option<String>,
    pub unsent_quantity: i64,
    /// Σ of the line's price changes in cents (negative for a discount).
    pub discount_cents: i64,
    /// The line's consolidated price changes, as the cart shows them.
    pub price_changes: Vec<CartPriceChange>,
    /// The note for the kitchen ("sans wasabi"), local only: never part
    /// of a fiscal record (migration 31).
    pub kitchen_note: Option<String>,
    /// The line belongs to an online order (HubRise): the till gives no
    /// discount on it (`discount_online_order`), so the screens hide the
    /// action.
    pub online: bool,
    /// The line's course (kitchen only): 0 "Direct", 1 to 3. Never
    /// part of the signed order line.
    pub course: i32,
    /// `unsent`, `held` (a course above the released one waits for
    /// "Envoyer la suite") or `printed` (a kitchen slip carried the line).
    pub kitchen_state: String,
}

/// A price change on a cart line (a till discount, an online order's).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CartPriceChange {
    pub id: String,
    pub name: String,
    /// `LINE` or `EVENT`.
    pub scope: String,
    pub amount_cents: i64,
}

/// The menu code of a line's product, a column of the mirrored catalog.
#[cfg(feature = "catalog-sync")]
const LINE_CODE: &str = "(SELECT code FROM menu_items WHERE id = ol.menu_item_id)";
#[cfg(not(feature = "catalog-sync"))]
const LINE_CODE: &str = "NULL";

pub fn get_cart_lines(conn: &Connection, order_id: i64) -> Result<Vec<CartLine>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT ol.id, ol.order_id, ol.menu_item_id, ol.quantity, ol.unit_price_cents, ol.vat_code,
                ol.name, ol.modifiers_json, ol.created_at,
                COALESCE(ol.department_id, ''), COALESCE(ol.department_name, ''),
                (SELECT value FROM translations WHERE entity_type = 'menu_item'
                    AND entity_id = ol.menu_item_id AND field = 'name' AND lang = 'fr'),
                (SELECT value FROM translations WHERE entity_type = 'menu_item'
                    AND entity_id = ol.menu_item_id AND field = 'name' AND lang = 'zh'),
                COALESCE((SELECT SUM(e.quantity) FROM order_entries e
                          WHERE e.line_id = ol.id AND e.sent_event_id IS NULL), 0),
                {LINE_CODE}, ol.kitchen_note,
                EXISTS (SELECT 1 FROM orders o WHERE o.id = ol.order_id AND o.hubrise_order_id IS NOT NULL),
                COALESCE((SELECT course FROM order_line_courses WHERE line_id = ol.id), 0),
                EXISTS (SELECT 1 FROM kitchen_slip_entries k
                        JOIN order_entries e ON e.id = k.entry_id WHERE e.line_id = ol.id)
         FROM order_lines ol WHERE ol.order_id = ?1 ORDER BY ol.id",
    ))?;
    let released = super::courses::released_course(conn, order_id)?;
    let rows = stmt.query_map(params![order_id], |row| {
        let course: i32 = row.get(17)?;
        let printed: bool = row.get(18)?;
        let kitchen_state = if printed {
            "printed"
        } else if course > released {
            "held"
        } else {
            "unsent"
        };
        Ok(CartLine {
            line: map_order_line(row)?,
            name_fr: row.get(11)?,
            name_zh: row.get(12)?,
            unsent_quantity: row.get(13)?,
            code: row.get(14)?,
            discount_cents: 0,
            price_changes: Vec::new(),
            kitchen_note: row.get(15)?,
            online: row.get(16)?,
            course,
            kitchen_state: kitchen_state.to_string(),
        })
    })?;
    #[allow(unused_mut)]
    let mut lines: Vec<CartLine> = rows.collect::<Result<_, _>>()?;
    #[cfg(feature = "hubrise")]
    {
        let changes = super::price_changes::by_line(conn, order_id)?;
        for line in &mut lines {
            for c in changes.get(&line.line.id).into_iter().flatten() {
                line.discount_cents += c.amount_cents;
                line.price_changes.push(CartPriceChange {
                    id: c.change_id.clone(),
                    name: c.name.clone(),
                    scope: c.scope.clone(),
                    amount_cents: c.amount_cents,
                });
            }
        }
    }
    Ok(lines)
}

/// What the kitchen note commands need to know about a line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTarget {
    pub order_id: i64,
    pub menu_item_id: i64,
    pub name: String,
    /// The table of the order; `None` for a takeaway.
    pub table_number: Option<i32>,
    /// An online order (HubRise): notes are not taken at the till.
    pub online: bool,
    pub note: Option<String>,
    /// The note last printed on a bar / kitchen slip.
    pub printed: Option<String>,
    /// Quantity of the line already transmitted to the FDM (so already
    /// sent to the kitchen).
    pub sent_quantity: i64,
}

/// The state of line `line_id` for a note change.
pub fn note_target(conn: &Connection, line_id: i64) -> Result<NoteTarget, rusqlite::Error> {
    conn.query_row(
        "SELECT ol.order_id, ol.menu_item_id, ol.name, o.table_number,
                o.hubrise_order_id IS NOT NULL, ol.kitchen_note, ol.kitchen_note_printed,
                COALESCE((SELECT SUM(e.quantity) FROM order_entries e
                          WHERE e.line_id = ol.id AND e.sent_event_id IS NOT NULL), 0)
         FROM order_lines ol JOIN orders o ON o.id = ol.order_id
         WHERE ol.id = ?1",
        params![line_id],
        |row| {
            Ok(NoteTarget {
                order_id: row.get(0)?,
                menu_item_id: row.get(1)?,
                name: row.get(2)?,
                table_number: row.get(3)?,
                online: row.get(4)?,
                note: row.get(5)?,
                printed: row.get(6)?,
                sent_quantity: row.get(7)?,
            })
        },
    )
}

/// Store the kitchen note of a line of an open order (`None` clears
/// it). The note is local data: nothing is journalled.
pub fn set_kitchen_note(
    conn: &Connection,
    line_id: i64,
    note: Option<&str>,
) -> Result<(), rusqlite::Error> {
    let order_id: i64 = conn.query_row(
        "SELECT order_id FROM order_lines WHERE id = ?1",
        params![line_id],
        |row| row.get(0),
    )?;
    ensure_open(conn, order_id)?;
    conn.execute(
        "UPDATE order_lines SET kitchen_note = ?1 WHERE id = ?2",
        params![note, line_id],
    )?;
    Ok(())
}

/// Record that the line's current note reached a bar / kitchen slip.
pub fn mark_kitchen_note_printed(conn: &Connection, line_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE order_lines SET kitchen_note_printed = kitchen_note WHERE id = ?1",
        params![line_id],
    )?;
    Ok(())
}

/// The line of a journal entry and that line's note, for the bar slip.
/// An entry whose line is gone (removed) maps to no note.
pub fn entry_notes(
    conn: &Connection,
    entry_ids: &[i64],
) -> Result<std::collections::BTreeMap<i64, (i64, Option<String>)>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT e.line_id, ol.kitchen_note
         FROM order_entries e LEFT JOIN order_lines ol ON ol.id = e.line_id
         WHERE e.id = ?1",
    )?;
    let mut out = std::collections::BTreeMap::new();
    for id in entry_ids {
        let found: Option<(i64, Option<String>)> = stmt
            .query_row(params![id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;
        if let Some(found) = found {
            out.insert(*id, found);
        }
    }
    Ok(out)
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

/// Void an open POS order that never held anything: no line, no signed
/// event, no payment (cash, card or terminal), not an online order.
/// Returns whether it was voided. Such an order is what opening a table
/// or the takeaway tab and leaving again leaves behind; it must not
/// block the SPF export, an update or a menu refresh. An order with any
/// fiscal event or terminal payment is never touched.
pub fn void_if_empty(conn: &Connection, order_id: i64) -> Result<bool, rusqlite::Error> {
    Ok(void_empty(conn, Some(order_id))? > 0)
}

/// [`void_if_empty`] for every open order, at start-up: orders left
/// empty by a session that ended without leaving them (a crash, a kill).
/// Returns how many were voided.
pub fn void_all_empty(conn: &Connection) -> Result<usize, rusqlite::Error> {
    void_empty(conn, None)
}

/// The condition of an order that never held anything (the orders row
/// is `orders`): what [`void_if_empty`] voids, and what never makes a
/// table taken ([`table_taken`]).
const NEVER_HELD_ANYTHING: &str =
    "NOT EXISTS (SELECT 1 FROM order_lines WHERE order_id = orders.id)
           AND NOT EXISTS (SELECT 1 FROM fiscal_events WHERE order_id = orders.id)
           AND NOT EXISTS (SELECT 1 FROM payments WHERE order_id = orders.id)
           AND NOT EXISTS (SELECT 1 FROM eft_payments WHERE order_id = orders.id)";

fn void_empty(conn: &Connection, order_id: Option<i64>) -> Result<usize, rusqlite::Error> {
    conn.execute(
        &format!(
            "UPDATE orders SET status = 'voided', closed_at = datetime('now')
             WHERE (?1 IS NULL OR id = ?1) AND status = 'open' AND hubrise_order_id IS NULL
               AND {NEVER_HELD_ANYTHING}"
        ),
        params![order_id],
    )
}

/// Whether table `table` holds an open dine-in till order other than
/// `except` once its empty orders are left aside: any line (whatever
/// its quantity), any signed event or any payment makes the table
/// taken. An order that [`void_if_empty`] would void (a table opened and
/// left) never does. The table transfer reads this both before it signs
/// and in the archive transaction of the move, so the two never differ.
pub fn table_taken(conn: &Connection, table: i32, except: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        &format!(
            "SELECT EXISTS (
                 SELECT 1 FROM orders
                 WHERE status = 'open' AND order_type = 'dine_in' AND table_number = ?1
                   AND id <> ?2 AND hubrise_order_id IS NULL
                   AND NOT ({NEVER_HELD_ANYTHING}))"
        ),
        params![table, except],
        |r| r.get(0),
    )
}

/// Void the empty open dine-in till orders of table `table` (other than
/// `except`), as [`void_if_empty`] would. Returns how many were voided.
pub fn void_empty_on_table(
    conn: &Connection,
    table: i32,
    except: i64,
) -> Result<usize, rusqlite::Error> {
    conn.execute(
        &format!(
            "UPDATE orders SET status = 'voided', closed_at = datetime('now')
             WHERE status = 'open' AND order_type = 'dine_in' AND table_number = ?1
               AND id <> ?2 AND hubrise_order_id IS NULL
               AND {NEVER_HELD_ANYTHING}"
        ),
        params![table, except],
    )
}

/// Open POS orders. Online orders (`hubrise_order_id`) have their own
/// board and never show among tables and takeaways.
pub fn list_open_orders(conn: &Connection) -> Result<Vec<Order>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, table_number, order_type, status, user_insz, created_at, closed_at, subtotal_cents, total_cents
         FROM orders WHERE status = 'open' AND hubrise_order_id IS NULL ORDER BY created_at DESC",
    )?;

    let rows = stmt.query_map([], map_order)?;
    rows.collect()
}

/// Bring the order's totals up to date (after a price change).
#[cfg(feature = "hubrise")]
pub(crate) fn recalculate_totals(conn: &Connection, order_id: i64) -> Result<(), rusqlite::Error> {
    recalculate_order_totals(conn, order_id)
}

fn recalculate_order_totals(conn: &Connection, order_id: i64) -> Result<(), rusqlite::Error> {
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(unit_price_cents * quantity), 0) FROM order_lines WHERE order_id = ?1",
        params![order_id],
        |row| row.get(0),
    )?;
    // The total is after the order's discounts.
    #[cfg(feature = "hubrise")]
    let total = total + super::price_changes::order_total_cents(conn, order_id)?;

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
    fn a_table_is_taken_by_any_order_that_void_if_empty_keeps() {
        let conn = setup();
        let moving = create_order(&conn, OrderType::DineIn, Some(4), "00000000029").unwrap();
        // A ghost: opened and left.
        let ghost = create_order(&conn, OrderType::DineIn, Some(8), "00000000029").unwrap();
        assert!(!table_taken(&conn, 8, moving.id).unwrap());
        // The order itself never takes its own table.
        assert!(!table_taken(&conn, 4, moving.id).unwrap());
        // A payment and no line: kept by void_if_empty, so taken.
        let paid = create_order(&conn, OrderType::DineIn, Some(9), "00000000029").unwrap();
        conn.execute(
            "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, 'cash', 500)",
            [paid.id],
        )
        .unwrap();
        assert!(table_taken(&conn, 9, moving.id).unwrap());
        // A takeaway or an online order never takes a table.
        let online = create_order(&conn, OrderType::DineIn, Some(10), "00000000029").unwrap();
        conn.execute(
            "UPDATE orders SET hubrise_order_id = 'H-1' WHERE id = ?1",
            [online.id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, 'card', 500)",
            [online.id],
        )
        .unwrap();
        assert!(!table_taken(&conn, 10, moving.id).unwrap());

        // Voiding the ghosts leaves exactly the orders that take a table.
        assert_eq!(void_empty_on_table(&conn, 9, moving.id).unwrap(), 0);
        assert_eq!(void_empty_on_table(&conn, 8, moving.id).unwrap(), 1);
        assert_eq!(
            get_order(&conn, ghost.id).unwrap().status,
            OrderStatus::Voided
        );
        assert_eq!(get_order(&conn, paid.id).unwrap().status, OrderStatus::Open);
        assert_eq!(
            get_order(&conn, moving.id).unwrap().status,
            OrderStatus::Open
        );
    }

    #[test]
    fn cart_lines_say_whether_the_order_is_online() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1000, 'B')",
            [],
        )
        .unwrap();
        let add = |order_id: i64| {
            add_order_line(
                &conn,
                NewOrderLine {
                    order_id,
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
        };
        let till = create_order(&conn, OrderType::DineIn, Some(1), "00000000029").unwrap();
        add(till.id);
        let online = create_order(&conn, OrderType::Takeaway, None, "00000000029").unwrap();
        add(online.id);
        conn.execute(
            "UPDATE orders SET hubrise_order_id = 'HR1' WHERE id = ?1",
            [online.id],
        )
        .unwrap();

        assert!(!get_cart_lines(&conn, till.id).unwrap()[0].online);
        assert!(get_cart_lines(&conn, online.id).unwrap()[0].online);
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
