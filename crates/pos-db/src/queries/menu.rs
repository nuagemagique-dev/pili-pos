use pos_core::menu::{Category, MenuItem};
use pos_core::vat::VatCode;
use rusqlite::{Connection, params};

use super::allergens;

/// With a mirrored catalog, the grid leaves out hidden categories
/// (charges) and items not sold on the POS; they still resolve on an
/// order line.
#[cfg(feature = "catalog-sync")]
const GRID_CATEGORY_FILTER: &str = " AND c.hidden = 0";
#[cfg(not(feature = "catalog-sync"))]
const GRID_CATEGORY_FILTER: &str = "";
#[cfg(feature = "catalog-sync")]
const GRID_ITEM_FILTER: &str = " AND m.on_pos = 1 AND m.kind = 'product'";
#[cfg(not(feature = "catalog-sync"))]
const GRID_ITEM_FILTER: &str = "";
/// The product's menu code, a column of the mirrored catalog.
#[cfg(feature = "catalog-sync")]
const ITEM_CODE: &str = "m.code";
#[cfg(not(feature = "catalog-sync"))]
const ITEM_CODE: &str = "NULL";

pub fn get_categories(conn: &Connection, lang: &str) -> Result<Vec<Category>, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT c.id, c.sort_order, c.color, c.active,
                COALESCE(t.value, 'unnamed') as name,
                (SELECT value FROM translations WHERE entity_type = 'category' AND entity_id = c.id AND field = 'name' AND lang = 'fr'),
                (SELECT value FROM translations WHERE entity_type = 'category' AND entity_id = c.id AND field = 'name' AND lang = 'zh')
         FROM categories c
         LEFT JOIN translations t
           ON t.entity_type = 'category' AND t.entity_id = c.id
              AND t.field = 'name' AND t.lang = ?1
         WHERE c.active = 1{GRID_CATEGORY_FILTER}
         ORDER BY c.sort_order"
    ))?;

    let rows = stmt.query_map(params![lang], |row| {
        Ok(Category {
            id: row.get(0)?,
            sort_order: row.get(1)?,
            color: row.get(2)?,
            active: row.get::<_, i32>(3)? != 0,
            name: row.get(4)?,
            name_fr: row.get(5)?,
            name_zh: row.get(6)?,
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
        format!("SELECT m.id, m.category_id, m.price_cents, m.vat_code,
                m.kitchen_printer, m.sort_order, m.active,
                COALESCE(t.value, 'unnamed') as name, m.vat_code_takeaway, m.open_price,
                (SELECT value FROM translations WHERE entity_type = 'menu_item' AND entity_id = m.id AND field = 'name' AND lang = 'fr'),
                (SELECT value FROM translations WHERE entity_type = 'menu_item' AND entity_id = m.id AND field = 'name' AND lang = 'zh'),
                {ITEM_CODE}
         FROM menu_items m
         LEFT JOIN translations t
           ON t.entity_type = 'menu_item' AND t.entity_id = m.id
              AND t.field = 'name' AND t.lang = ?1
         WHERE m.active = 1 AND m.category_id = ?2{GRID_ITEM_FILTER}
         ORDER BY m.sort_order")
    } else {
        format!("SELECT m.id, m.category_id, m.price_cents, m.vat_code,
                m.kitchen_printer, m.sort_order, m.active,
                COALESCE(t.value, 'unnamed') as name, m.vat_code_takeaway, m.open_price,
                (SELECT value FROM translations WHERE entity_type = 'menu_item' AND entity_id = m.id AND field = 'name' AND lang = 'fr'),
                (SELECT value FROM translations WHERE entity_type = 'menu_item' AND entity_id = m.id AND field = 'name' AND lang = 'zh'),
                {ITEM_CODE}
         FROM menu_items m
         LEFT JOIN translations t
           ON t.entity_type = 'menu_item' AND t.entity_id = m.id
              AND t.field = 'name' AND t.lang = ?1
         WHERE m.active = 1{GRID_ITEM_FILTER}
         ORDER BY m.sort_order")
    };

    let mut stmt = conn.prepare(&sql)?;

    let rows = if let Some(cat_id) = category_id {
        stmt.query_map(params![lang, cat_id], map_menu_item)?
    } else {
        stmt.query_map(params![lang], map_menu_item)?
    };

    let mut items = rows.collect::<Result<Vec<MenuItem>, _>>()?;
    let declared = allergens::load_all(conn, allergens::Entity::MenuItem)?;
    for item in &mut items {
        if let Some(set) = declared.get(&item.id) {
            item.allergens = set.contains.clone();
            item.traces = set.may_contain.clone();
            item.allergens_declared = true;
        }
    }
    Ok(items)
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
        vat_code_takeaway: parse_optional_vat(row, 8)?,
        open_price: row.get::<_, i32>(9)? != 0,
        name_fr: row.get(10)?,
        name_zh: row.get(11)?,
        code: row.get(12)?,
        allergens: Vec::new(),
        traces: Vec::new(),
        allergens_declared: false,
    })
}

fn parse_optional_vat(
    row: &rusqlite::Row<'_>,
    idx: usize,
) -> Result<Option<VatCode>, rusqlite::Error> {
    let raw: Option<String> = row.get(idx)?;
    raw.map(|v| {
        v.parse().map_err(|e: String| {
            rusqlite::Error::FromSqlConversionFailure(
                idx,
                rusqlite::types::Type::Text,
                Box::from(e),
            )
        })
    })
    .transpose()
}

/// Fiscal data of a product as it must appear on an order line: price
/// and VAT label from the catalogue (never from the UI), the product
/// and department names in the fiscal language of the establishment.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogueLine {
    /// Catalogue price; meaningless when `open_price` is set.
    pub price_cents: i64,
    pub open_price: bool,
    pub vat_code: VatCode,
    pub name: String,
    pub department_id: String,
    pub department_name: String,
}

/// Resolve an active product for an order line. `takeaway` selects the
/// "À emporter" VAT label when the product has one. Names fall back to
/// French, then English, when the fiscal language has no translation.
///
/// Returns `QueryReturnedNoRows` if the product or its category is
/// inactive or unknown: such products cannot be sold.
pub fn resolve_catalogue_line(
    conn: &Connection,
    menu_item_id: i64,
    takeaway: bool,
    lang: &str,
) -> Result<CatalogueLine, rusqlite::Error> {
    conn.query_row(
        "SELECT m.price_cents, m.vat_code, m.vat_code_takeaway, m.category_id, m.open_price,
                COALESCE(
                    (SELECT value FROM translations WHERE entity_type = 'menu_item'
                        AND entity_id = m.id AND field = 'name' AND lang = ?2),
                    (SELECT value FROM translations WHERE entity_type = 'menu_item'
                        AND entity_id = m.id AND field = 'name' AND lang = 'fr'),
                    (SELECT value FROM translations WHERE entity_type = 'menu_item'
                        AND entity_id = m.id AND field = 'name' AND lang = 'en'),
                    CAST(m.id AS TEXT)),
                COALESCE(
                    (SELECT value FROM translations WHERE entity_type = 'category'
                        AND entity_id = c.id AND field = 'name' AND lang = ?2),
                    (SELECT value FROM translations WHERE entity_type = 'category'
                        AND entity_id = c.id AND field = 'name' AND lang = 'fr'),
                    (SELECT value FROM translations WHERE entity_type = 'category'
                        AND entity_id = c.id AND field = 'name' AND lang = 'en'),
                    CAST(c.id AS TEXT))
         FROM menu_items m
         JOIN categories c ON c.id = m.category_id
         WHERE m.id = ?1 AND m.active = 1 AND c.active = 1",
        params![menu_item_id, lang],
        |row| {
            let dine_in: String = row.get(1)?;
            let vat_str = match (takeaway, row.get::<_, Option<String>>(2)?) {
                (true, Some(takeaway_code)) => takeaway_code,
                _ => dine_in,
            };
            let vat_code = vat_str.parse().map_err(|e: String| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::from(e),
                )
            })?;
            Ok(CatalogueLine {
                price_cents: row.get(0)?,
                open_price: row.get::<_, i32>(4)? != 0,
                vat_code,
                department_id: row.get::<_, i64>(3)?.to_string(),
                name: row.get(5)?,
                department_name: row.get(6)?,
            })
        },
    )
}

/// Whether an ordered product goes on the bar / kitchen slip
/// (`menu_items.kitchen_printer`). An unknown product does not.
pub fn sends_to_kitchen(conn: &Connection, menu_item_id: i64) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT kitchen_printer FROM menu_items WHERE id = ?1",
        params![menu_item_id],
        |row| row.get::<_, i32>(0),
    )
    .map(|flag| flag != 0)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(false),
        e => Err(e),
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

    fn seed_burger(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (22, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, vat_code_takeaway, sort_order)
                 VALUES (22001, 22, 2800, 'B', 'C', 0);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                 ('menu_item', 22001, 'name', 'fr', 'Burger du chef'),
                 ('menu_item', 22001, 'name', 'en', 'Burger of the Chef'),
                 ('category', 22, 'name', 'fr', 'Plats'),
                 ('category', 22, 'name', 'en', 'Main Dishes');",
        )
        .unwrap();
    }

    #[test]
    fn catalogue_line_uses_the_dine_in_or_takeaway_vat_label() {
        let conn = setup();
        seed_burger(&conn);
        let dine_in = resolve_catalogue_line(&conn, 22001, false, "fr").unwrap();
        assert_eq!(
            dine_in,
            CatalogueLine {
                price_cents: 2800,
                open_price: false,
                vat_code: VatCode::B,
                name: "Burger du chef".into(),
                department_id: "22".into(),
                department_name: "Plats".into(),
            }
        );
        let takeaway = resolve_catalogue_line(&conn, 22001, true, "fr").unwrap();
        assert_eq!(takeaway.vat_code, VatCode::C);
    }

    #[test]
    fn catalogue_line_falls_back_to_french_names() {
        let conn = setup();
        seed_burger(&conn);
        let nl = resolve_catalogue_line(&conn, 22001, false, "nl").unwrap();
        assert_eq!(nl.name, "Burger du chef");
        assert_eq!(nl.department_name, "Plats");
    }

    #[test]
    fn takeaway_without_a_takeaway_label_keeps_the_dine_in_one() {
        let conn = setup();
        seed_burger(&conn);
        conn.execute("UPDATE menu_items SET vat_code_takeaway = NULL", [])
            .unwrap();
        let takeaway = resolve_catalogue_line(&conn, 22001, true, "fr").unwrap();
        assert_eq!(takeaway.vat_code, VatCode::B);
    }

    #[test]
    fn inactive_products_cannot_be_resolved() {
        let conn = setup();
        seed_burger(&conn);
        conn.execute("UPDATE categories SET active = 0 WHERE id = 22", [])
            .unwrap();
        assert!(matches!(
            resolve_catalogue_line(&conn, 22001, false, "fr"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
    }

    #[test]
    fn inactive_item_itself_cannot_be_resolved() {
        let conn = setup();
        seed_burger(&conn);
        conn.execute("UPDATE menu_items SET active = 0 WHERE id = 22001", [])
            .unwrap();
        assert!(matches!(
            resolve_catalogue_line(&conn, 22001, false, "fr"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
    }

    #[test]
    fn unknown_product_id_cannot_be_resolved() {
        let conn = setup();
        seed_burger(&conn);
        assert!(matches!(
            resolve_catalogue_line(&conn, 999_999, false, "fr"),
            Err(rusqlite::Error::QueryReturnedNoRows)
        ));
    }

    #[test]
    fn catalogue_line_falls_back_to_id_when_no_translation_exists_at_all() {
        let conn = setup();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (30, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order)
                 VALUES (30001, 30, 500, 'B', 0);",
        )
        .unwrap();
        // No translations at all (fr/en/requested) - falls back to the
        // numeric id as text, never panics or errors.
        let line = resolve_catalogue_line(&conn, 30001, false, "zh").unwrap();
        assert_eq!(line.name, "30001");
        assert_eq!(line.department_name, "30");
    }

    #[test]
    fn open_price_flag_is_carried_through() {
        let conn = setup();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (40, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, open_price, sort_order)
                 VALUES (40001, 40, 0, 'C', 1, 0);",
        )
        .unwrap();
        let line = resolve_catalogue_line(&conn, 40001, true, "fr").unwrap();
        assert!(line.open_price);
        assert_eq!(line.price_cents, 0);

        let items = get_menu_items(&conn, Some(40), "fr").unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].open_price);
    }

    #[test]
    fn get_menu_items_without_category_filter_returns_everything_active() {
        let conn = setup();
        seed_burger(&conn);
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (23, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order)
                 VALUES (23001, 23, 400, 'A', 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
                 VALUES (23002, 23, 400, 'A', 1, 0);",
        )
        .unwrap();

        let all = get_menu_items(&conn, None, "fr").unwrap();
        // Burger (22001) + active item 23001, but NOT the inactive 23002.
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|i| i.active));
    }
}

/// An option group of a product, as the order screen offers it.
#[cfg(feature = "catalog-sync")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ModifierGroup {
    pub id: i64,
    pub name: String,
    pub min: i64,
    /// `None`: no maximum.
    pub max: Option<i64>,
    pub options: Vec<Modifier>,
}

#[cfg(feature = "catalog-sync")]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Modifier {
    pub id: i64,
    pub name: String,
    pub price_cents: i64,
    /// Allergen codes the option contains / may contain as traces.
    pub allergens: Vec<String>,
    pub traces: Vec<String>,
    /// The option's allergen list is declared (possibly empty).
    pub allergens_declared: bool,
}

/// The active option groups of `menu_item_id`, names in `lang` (then
/// French, then English), each with its active options.
#[cfg(feature = "catalog-sync")]
pub fn get_modifier_groups(
    conn: &Connection,
    menu_item_id: i64,
    lang: &str,
) -> Result<Vec<ModifierGroup>, rusqlite::Error> {
    let name = |entity: &str, alias: &str| {
        format!(
            "COALESCE(
                (SELECT value FROM translations WHERE entity_type = '{entity}'
                    AND entity_id = {alias}.id AND field = 'name' AND lang = ?2),
                (SELECT value FROM translations WHERE entity_type = '{entity}'
                    AND entity_id = {alias}.id AND field = 'name' AND lang = 'fr'),
                (SELECT value FROM translations WHERE entity_type = '{entity}'
                    AND entity_id = {alias}.id AND field = 'name' AND lang = 'en'),
                CAST({alias}.id AS TEXT))"
        )
    };
    let mut groups: Vec<ModifierGroup> = {
        let mut stmt = conn.prepare(&format!(
            "SELECT g.id, {}, g.min_select, g.max_select FROM modifier_groups g
             WHERE g.menu_item_id = ?1 AND g.active = 1
             ORDER BY g.sort_order, g.id",
            name("modifier_group", "g")
        ))?;
        stmt.query_map(params![menu_item_id, lang], |r| {
            let max: i64 = r.get(3)?;
            Ok(ModifierGroup {
                id: r.get(0)?,
                name: r.get(1)?,
                min: r.get(2)?,
                max: (max >= 0).then_some(max),
                options: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT m.id, {}, m.price_cents FROM modifiers m
         WHERE m.group_id = ?1 AND m.active = 1
         ORDER BY m.sort_order, m.id",
        name("modifier", "m")
    ))?;
    for group in &mut groups {
        group.options = stmt
            .query_map(params![group.id, lang], |r| {
                Ok(Modifier {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    price_cents: r.get(2)?,
                    allergens: Vec::new(),
                    traces: Vec::new(),
                    allergens_declared: false,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
    }
    let declared = allergens::load_all(conn, allergens::Entity::Modifier)?;
    for option in groups.iter_mut().flat_map(|g| g.options.iter_mut()) {
        if let Some(set) = declared.get(&option.id) {
            option.allergens = set.contains.clone();
            option.traces = set.may_contain.clone();
            option.allergens_declared = true;
        }
    }
    // A group with nothing to pick is not offered.
    groups.retain(|g| !g.options.is_empty());
    Ok(groups)
}
