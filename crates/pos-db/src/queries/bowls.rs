//! The paid options of a weighed bowl (migration 81): each is a line of its
//! own, linked here to its bowl. Local data, never in an FDM message.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};

/// Link the option line `line_id` to the bowl line `bowl_line_id`.
pub fn link_option(
    conn: &Connection,
    line_id: i64,
    bowl_line_id: i64,
    order_id: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO bowl_option_lines (line_id, bowl_line_id, order_id) VALUES (?1, ?2, ?3)",
        params![line_id, bowl_line_id, order_id],
    )?;
    Ok(())
}

/// The option lines of the bowl `bowl_line_id`, oldest first.
pub fn options_of(conn: &Connection, bowl_line_id: i64) -> rusqlite::Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT line_id FROM bowl_option_lines WHERE bowl_line_id = ?1 ORDER BY line_id",
    )?;
    let rows = stmt.query_map(params![bowl_line_id], |row| row.get(0))?;
    rows.collect()
}

/// The bowl the line `line_id` is an option of, if it is one.
pub fn bowl_of(conn: &Connection, line_id: i64) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT bowl_line_id FROM bowl_option_lines WHERE line_id = ?1",
        params![line_id],
        |row| row.get(0),
    )
    .optional()
}

/// Every option line of `order_id` with its bowl: option line to bowl line.
pub fn links_of_order(conn: &Connection, order_id: i64) -> rusqlite::Result<BTreeMap<i64, i64>> {
    let mut stmt =
        conn.prepare("SELECT line_id, bowl_line_id FROM bowl_option_lines WHERE order_id = ?1")?;
    let rows = stmt.query_map(params![order_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    rows.collect()
}

/// The names of the options picked with the weighed line `line_id`, in the
/// menu's order (its `modifiers_json`); empty for any other line, whose
/// options are already in its name.
pub fn bowl_option_names(conn: &Connection, line_id: i64) -> rusqlite::Result<Vec<String>> {
    let json: Option<String> = conn
        .query_row(
            "SELECT modifiers_json FROM order_lines WHERE id = ?1 AND weight_grams IS NOT NULL",
            params![line_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    Ok(json.as_deref().map(option_names).unwrap_or_default())
}

/// The option names of a line's `modifiers_json`, in its order; empty when
/// it cannot be read.
pub fn option_names(json: &str) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Named {
        name: String,
    }
    serde_json::from_str::<Vec<Named>>(json)
        .map(|v| v.into_iter().map(|n| n.name).collect())
        .unwrap_or_default()
}

/// Forget the link of a line removed from its order (a line id can be
/// given again once its line is gone).
pub fn unlink(conn: &Connection, line_id: i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM bowl_option_lines WHERE line_id = ?1",
        params![line_id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queries::orders::{NewOrderLine, add_order_line, create_order};
    use pos_core::order::OrderType;
    use pos_core::vat::VatCode;

    fn line(conn: &Connection, order_id: i64, name: &str) -> i64 {
        add_order_line(
            conn,
            NewOrderLine {
                order_id,
                menu_item_id: 1,
                quantity: 1,
                unit_price_cents: 150,
                vat_code: VatCode::B,
                name,
                modifiers_json: None,
                department_id: "",
                department_name: "",
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn an_option_line_is_linked_to_its_bowl_until_it_is_unlinked() {
        let conn = Connection::open_in_memory().unwrap();
        crate::schema::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (1, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 1990, 'B');",
        )
        .unwrap();
        let order = create_order(&conn, OrderType::DineIn, Some(3), "00000000029").unwrap();
        let bowl = line(&conn, order.id, "Bol");
        let broth = line(&conn, order.id, "Bouillon tomate");
        let extra = line(&conn, order.id, "Nouilles");
        link_option(&conn, broth, bowl, order.id).unwrap();
        link_option(&conn, extra, bowl, order.id).unwrap();

        assert_eq!(options_of(&conn, bowl).unwrap(), vec![broth, extra]);
        assert_eq!(bowl_of(&conn, broth).unwrap(), Some(bowl));
        assert_eq!(bowl_of(&conn, bowl).unwrap(), None);
        assert_eq!(
            links_of_order(&conn, order.id).unwrap(),
            BTreeMap::from([(broth, bowl), (extra, bowl)])
        );

        unlink(&conn, broth).unwrap();
        assert_eq!(options_of(&conn, bowl).unwrap(), vec![extra]);
    }
}
