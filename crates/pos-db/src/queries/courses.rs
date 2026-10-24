//! Courses (migration 62): kitchen data only, never fiscal.
//!
//! The course of a line, the courses released for an order and the
//! journal entries a kitchen slip printed. This module only moves
//! data; the rules (what is held, what a release prints) live in
//! `pos_engine::commands::courses`.

use rusqlite::{Connection, params};

/// The highest course: 0 Direct, 1 Entrees, 2 Plats, 3 Desserts.
pub const MAX_COURSE: i32 = 3;

/// The course of a line: 0 ("Direct") when none was ever set.
pub fn line_course(conn: &Connection, line_id: i64) -> Result<i32, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE((SELECT course FROM order_line_courses WHERE line_id = ?1), 0)",
        params![line_id],
        |r| r.get(0),
    )
}

/// Set a line's course (`set_by`: INSZ of the operator, `None` when the
/// product's category gave it).
pub fn set_line_course(
    conn: &Connection,
    line_id: i64,
    order_id: i64,
    course: i32,
    set_by: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO order_line_courses (line_id, order_id, course, set_by, set_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(line_id) DO UPDATE SET
             order_id = excluded.order_id, course = excluded.course,
             set_by = excluded.set_by, set_at = excluded.set_at",
        params![line_id, order_id, course, set_by],
    )?;
    Ok(())
}

/// Forget a line's course row (a new line may reuse the id of a removed
/// one).
pub fn clear_line_course(conn: &Connection, line_id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM order_line_courses WHERE line_id = ?1",
        params![line_id],
    )?;
    Ok(())
}

/// The course released for the order: the highest released one, 1 when
/// none (courses 0 and 1 go out with the first slip).
pub fn released_course(conn: &Connection, order_id: i64) -> Result<i32, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(MAX(course), 1) FROM order_course_releases WHERE order_id = ?1",
        params![order_id],
        |r| r.get(0),
    )
}

/// Release `course` for the order. Returns false when it already was.
pub fn release(
    conn: &Connection,
    order_id: i64,
    course: i32,
    released_by: Option<&str>,
) -> Result<bool, rusqlite::Error> {
    let n = conn.execute(
        "INSERT OR IGNORE INTO order_course_releases (order_id, course, released_by, released_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        params![order_id, course, released_by],
    )?;
    Ok(n > 0)
}

/// Whether a release was recorded for exactly this course.
pub fn was_released(
    conn: &Connection,
    order_id: i64,
    course: i32,
) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM order_course_releases WHERE order_id = ?1 AND course = ?2)",
        params![order_id, course],
        |r| r.get(0),
    )
}

/// Whether any journal entry of the line was printed on a kitchen slip.
pub fn line_was_printed(conn: &Connection, line_id: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM kitchen_slip_entries k
                        JOIN order_entries e ON e.id = k.entry_id
                        WHERE e.line_id = ?1)",
        params![line_id],
        |r| r.get(0),
    )
}

/// A course with lines still waiting for its release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldCourse {
    pub course: i32,
    pub lines: i64,
    pub quantity: i64,
}

/// The courses above the released one that hold lines for the kitchen
/// (products flagged `kitchen_printer`, quantity above 0), lowest
/// first.
pub fn held_courses(conn: &Connection, order_id: i64) -> Result<Vec<HeldCourse>, rusqlite::Error> {
    let released = released_course(conn, order_id)?;
    let mut stmt = conn.prepare(
        "SELECT c.course, COUNT(*), COALESCE(SUM(ol.quantity), 0)
         FROM order_lines ol
         JOIN order_line_courses c ON c.line_id = ol.id
         JOIN menu_items m ON m.id = ol.menu_item_id
         WHERE ol.order_id = ?1 AND c.course > ?2 AND ol.quantity > 0 AND m.kitchen_printer <> 0
         GROUP BY c.course ORDER BY c.course",
    )?;
    let rows = stmt.query_map(params![order_id, released], |r| {
        Ok(HeldCourse {
            course: r.get(0)?,
            lines: r.get(1)?,
            quantity: r.get(2)?,
        })
    })?;
    rows.collect()
}

/// A journal entry no kitchen slip printed yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KitchenEntry {
    pub entry_id: i64,
    pub line_id: i64,
    pub quantity: i32,
    pub name: String,
    pub course: i32,
    /// Registered by a signed event (a kitchen slip only follows a
    /// signed P or N).
    pub signed: bool,
    /// One half of a PRICE_CHANGE pair (a till discount, migration 30):
    /// it orders nothing, so it is never held and never printed.
    pub price_change: bool,
    /// The kitchen note of the entry's line (migration 31): the current
    /// note for an entry still to print, the printed copy for a reprint.
    pub note: Option<String>,
}

/// The order's journal entries for products that go to the kitchen and
/// that no slip printed, oldest first, with the course of their line.
pub fn unprinted_entries(
    conn: &Connection,
    order_id: i64,
) -> Result<Vec<KitchenEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.line_id, e.quantity, e.name,
                COALESCE(c.course, 0), e.sent_event_id IS NOT NULL,
                COALESCE(e.till_op = 'price_change', 0), ol.kitchen_note
         FROM order_entries e
         LEFT JOIN order_line_courses c ON c.line_id = e.line_id
         LEFT JOIN order_lines ol ON ol.id = e.line_id
         JOIN menu_items m ON m.id = e.menu_item_id
         WHERE e.order_id = ?1 AND m.kitchen_printer <> 0 AND e.kind <> 'refund'
           AND NOT EXISTS (SELECT 1 FROM kitchen_slip_entries k WHERE k.entry_id = e.id)
         ORDER BY e.id",
    )?;
    let rows = stmt.query_map(params![order_id], |r| {
        Ok(KitchenEntry {
            entry_id: r.get(0)?,
            line_id: r.get(1)?,
            quantity: r.get(2)?,
            name: r.get(3)?,
            course: r.get(4)?,
            signed: r.get(5)?,
            price_change: r.get(6)?,
            note: r.get(7)?,
        })
    })?;
    rows.collect()
}

/// The entries a slip of this order printed for `course`, oldest first.
pub fn printed_entries(
    conn: &Connection,
    order_id: i64,
    course: i32,
) -> Result<Vec<KitchenEntry>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.line_id, e.quantity, e.name, k.course, 1,
                COALESCE(e.till_op = 'price_change', 0), ol.kitchen_note_printed
         FROM kitchen_slip_entries k
         JOIN order_entries e ON e.id = k.entry_id
         LEFT JOIN order_lines ol ON ol.id = e.line_id
         WHERE e.order_id = ?1 AND k.course = ?2
         ORDER BY e.id",
    )?;
    let rows = stmt.query_map(params![order_id, course], |r| {
        Ok(KitchenEntry {
            entry_id: r.get(0)?,
            line_id: r.get(1)?,
            quantity: r.get(2)?,
            name: r.get(3)?,
            course: r.get(4)?,
            signed: r.get(5)?,
            price_change: r.get(6)?,
            note: r.get(7)?,
        })
    })?;
    rows.collect()
}

/// Record that a slip printed these journal entries (`slip`:
/// `immediate`, `release` or `sale`). The course stored is the line's
/// at this moment. Entries already recorded are left alone.
pub fn record_printed(
    conn: &Connection,
    entry_ids: &[i64],
    slip: &str,
) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(
        "INSERT OR IGNORE INTO kitchen_slip_entries (entry_id, course, printed_at, slip)
         SELECT e.id, COALESCE((SELECT course FROM order_line_courses WHERE line_id = e.line_id), 0),
                datetime('now'), ?2
         FROM order_entries e WHERE e.id = ?1",
    )?;
    for id in entry_ids {
        stmt.execute(params![id, slip])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO orders (id, table_number, order_type, status, user_insz, created_at)
                 VALUES (1, 4, 'dine_in', 'open', '00000000029', '2026-10-17T10:00:00Z');
             INSERT INTO categories (id, sort_order) VALUES (1, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer)
                 VALUES (1, 1, 100, 'B', 1), (2, 1, 100, 'B', 0);
             INSERT INTO order_lines (id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name)
                 VALUES (1, 1, 1, 2, 100, 'B', 'a'), (2, 1, 1, 1, 100, 'B', 'b'), (3, 1, 2, 1, 100, 'B', 'c');
             INSERT INTO order_entries (id, order_id, line_id, menu_item_id, quantity, unit_price_cents,
                                        vat_code, name, department_id, department_name, kind, created_at)
                 VALUES (1, 1, 1, 1, 2, 100, 'B', 'a', 'd', 'd', 'entry', '2026-10-17T10:00:00Z'),
                        (2, 1, 2, 1, 1, 100, 'B', 'b', 'd', 'd', 'entry', '2026-10-17T10:00:00Z'),
                        (3, 1, 3, 2, 1, 100, 'B', 'c', 'd', 'd', 'entry', '2026-10-17T10:00:00Z');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_line_without_a_row_is_course_zero() {
        let conn = db();
        assert_eq!(line_course(&conn, 1).unwrap(), 0);
        set_line_course(&conn, 1, 1, 2, Some("00000000029")).unwrap();
        assert_eq!(line_course(&conn, 1).unwrap(), 2);
        set_line_course(&conn, 1, 1, 3, None).unwrap();
        assert_eq!(line_course(&conn, 1).unwrap(), 3);
        clear_line_course(&conn, 1).unwrap();
        assert_eq!(line_course(&conn, 1).unwrap(), 0);
    }

    #[test]
    fn the_released_course_starts_at_one_and_only_grows() {
        let conn = db();
        assert_eq!(released_course(&conn, 1).unwrap(), 1);
        assert!(release(&conn, 1, 2, None).unwrap());
        assert!(!release(&conn, 1, 2, None).unwrap());
        assert_eq!(released_course(&conn, 1).unwrap(), 2);
        assert!(was_released(&conn, 1, 2).unwrap());
        assert!(!was_released(&conn, 1, 3).unwrap());
    }

    #[test]
    fn held_courses_count_the_kitchen_lines_above_the_released_course() {
        let conn = db();
        set_line_course(&conn, 1, 1, 2, None).unwrap();
        set_line_course(&conn, 2, 1, 3, None).unwrap();
        set_line_course(&conn, 3, 1, 3, None).unwrap();
        assert_eq!(
            held_courses(&conn, 1).unwrap(),
            vec![
                HeldCourse {
                    course: 2,
                    lines: 1,
                    quantity: 2
                },
                HeldCourse {
                    course: 3,
                    lines: 1,
                    quantity: 1
                }
            ],
            "line 3 is not a kitchen product"
        );
        release(&conn, 1, 2, None).unwrap();
        assert_eq!(held_courses(&conn, 1).unwrap().len(), 1);
    }

    #[test]
    fn unprinted_entries_skip_non_kitchen_products_and_printed_ones() {
        let conn = db();
        set_line_course(&conn, 2, 1, 2, None).unwrap();
        let all = unprinted_entries(&conn, 1).unwrap();
        assert_eq!(all.iter().map(|e| e.entry_id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(all[1].course, 2);
        assert!(!all[0].signed);
        record_printed(&conn, &[1], "immediate").unwrap();
        assert_eq!(unprinted_entries(&conn, 1).unwrap().len(), 1);
        assert!(line_was_printed(&conn, 1).unwrap());
        assert!(!line_was_printed(&conn, 2).unwrap());
        assert_eq!(printed_entries(&conn, 1, 0).unwrap().len(), 1);
        assert!(printed_entries(&conn, 1, 2).unwrap().is_empty());
    }
}
