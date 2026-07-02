use pos_core::menu::{Category, MenuItem};
use pos_core::vat::VatCode;
use rusqlite::{Connection, params};

pub fn get_categories(conn: &Connection, lang: &str) -> Result<Vec<Category>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.sort_order, c.color, c.active,
                COALESCE(t.value, 'unnamed') as name
         FROM categories c
         LEFT JOIN translations t
           ON t.entity_type = 'category' AND t.entity_id = c.id
              AND t.field = 'name' AND t.lang = ?1
         WHERE c.active = 1
         ORDER BY c.sort_order",
    )?;

    let rows = stmt.query_map(params![lang], |row| {
        Ok(Category {
            id: row.get(0)?,
            sort_order: row.get(1)?,
            color: row.get(2)?,
            active: row.get::<_, i32>(3)? != 0,
            name: row.get(4)?,
        })
    })?;

    rows.collect()
}

pub fn get_menu_items(
    conn: &Connection,
    category_id: Option<i64>,
    lang: &str,
) -> Result<Vec<MenuItem>, rusqlite::Error> {
    let sql = if category_id.is_some() {
        "SELECT m.id, m.category_id, m.price_cents, m.vat_code,
                m.kitchen_printer, m.sort_order, m.active,
                COALESCE(t.value, 'unnamed') as name
         FROM menu_items m
         LEFT JOIN translations t
           ON t.entity_type = 'menu_item' AND t.entity_id = m.id
              AND t.field = 'name' AND t.lang = ?1
         WHERE m.active = 1 AND m.category_id = ?2
         ORDER BY m.sort_order"
    } else {
        "SELECT m.id, m.category_id, m.price_cents, m.vat_code,
                m.kitchen_printer, m.sort_order, m.active,
                COALESCE(t.value, 'unnamed') as name
         FROM menu_items m
         LEFT JOIN translations t
           ON t.entity_type = 'menu_item' AND t.entity_id = m.id
              AND t.field = 'name' AND t.lang = ?1
         WHERE m.active = 1
         ORDER BY m.sort_order"
    };

    let mut stmt = conn.prepare(sql)?;

    let rows = if let Some(cat_id) = category_id {
        stmt.query_map(params![lang, cat_id], map_menu_item)?
    } else {
        stmt.query_map(params![lang], map_menu_item)?
    };

    rows.collect()
}

fn map_menu_item(row: &rusqlite::Row<'_>) -> Result<MenuItem, rusqlite::Error> {
    let vat_str: String = row.get(3)?;
    let vat_code: VatCode = vat_str.parse().map_err(|e: String| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::from(e))
    })?;

    Ok(MenuItem {
        id: row.get(0)?,
        category_id: row.get(1)?,
        price_cents: row.get(2)?,
        vat_code,
        kitchen_printer: row.get::<_, i32>(4)? != 0,
        sort_order: row.get(5)?,
        active: row.get::<_, i32>(6)? != 0,
        name: row.get(7)?,
    })
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
    fn empty_categories() {
        let conn = setup();
        let cats = get_categories(&conn, "fr").unwrap();
        assert!(cats.is_empty());
    }

    #[test]
    fn insert_and_query_category() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO translations (entity_type, entity_id, field, lang, value)
             VALUES ('category', 1, 'name', 'fr', 'Sushi')",
            [],
        )
        .unwrap();

        let cats = get_categories(&conn, "fr").unwrap();
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].name, "Sushi");
    }

    #[test]
    fn insert_and_query_menu_item() {
        let conn = setup();
        conn.execute("INSERT INTO categories (id, sort_order) VALUES (1, 0)", [])
            .unwrap();
        conn.execute(
            "INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order)
             VALUES (1, 1, 1250, 'B', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO translations (entity_type, entity_id, field, lang, value)
             VALUES ('menu_item', 1, 'name', 'fr', 'Saumon roll x8')",
            [],
        )
        .unwrap();

        let items = get_menu_items(&conn, Some(1), "fr").unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Saumon roll x8");
        assert_eq!(items[0].price_cents, 1250);
        assert_eq!(items[0].vat_code, VatCode::B);
    }
}
