//! The on-screen menu editor's storage (migration 42, V11-D17): reads of the whole menu with its
//! names per language, writes of one entity with its translations, and the append-only
//! `menu_change_log`. Nothing is ever deleted: an `active` flag hides an entry.
//!
//! This module trusts its arguments (the engine validates); it only keeps rows, translations
//! and the log row consistent. The caller wraps a save in one transaction.

use std::collections::{BTreeMap, HashMap};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// Names per language code (`fr`, `nl`, `en`, `zh`, `de`).
pub type Names = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminCategory {
    pub id: i64,
    pub names: Names,
    pub sort_order: i32,
    pub color: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminProduct {
    pub id: i64,
    pub category_id: i64,
    pub names: Names,
    pub price_cents: i64,
    pub vat_code: String,
    pub vat_code_takeaway: Option<String>,
    pub kitchen_printer: bool,
    pub code: Option<String>,
    pub open_price: bool,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminGroup {
    pub id: i64,
    pub menu_item_id: i64,
    pub names: Names,
    pub min_select: i32,
    pub max_select: i32,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminOption {
    pub id: i64,
    pub group_id: i64,
    pub names: Names,
    pub price_cents: i64,
    pub sort_order: i32,
    pub active: bool,
}

/// One line of `menu_change_log`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeRow {
    pub id: i64,
    /// `category`, `product`, `modifier_group` or `modifier`.
    pub entity_type: String,
    pub entity_id: i64,
    /// `create` or `update`.
    pub action: String,
    pub before_json: Option<String>,
    pub after_json: String,
    pub changed_by: String,
    pub changed_at: String,
}

pub const CATEGORY: &str = "category";
pub const PRODUCT: &str = "product";
pub const GROUP: &str = "modifier_group";
pub const OPTION: &str = "modifier";

/// The `translations.entity_type` of an entity type of the log.
fn translation_type(entity_type: &str) -> &'static str {
    match entity_type {
        CATEGORY => "category",
        PRODUCT => "menu_item",
        GROUP => "modifier_group",
        _ => "modifier",
    }
}

fn all_names(conn: &Connection, entity_type: &str) -> rusqlite::Result<HashMap<i64, Names>> {
    let mut stmt = conn.prepare(
        "SELECT entity_id, lang, value FROM translations
         WHERE entity_type = ?1 AND field = 'name'",
    )?;
    let mut map: HashMap<i64, Names> = HashMap::new();
    for row in stmt.query_map(params![translation_type(entity_type)], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })? {
        let (id, lang, value) = row?;
        map.entry(id).or_default().insert(lang, value);
    }
    Ok(map)
}

fn names_of(conn: &Connection, entity_type: &str, id: i64) -> rusqlite::Result<Names> {
    let mut stmt = conn.prepare(
        "SELECT lang, value FROM translations
         WHERE entity_type = ?1 AND entity_id = ?2 AND field = 'name'",
    )?;
    stmt.query_map(params![translation_type(entity_type), id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?
    .collect()
}

pub fn list_categories(conn: &Connection) -> rusqlite::Result<Vec<AdminCategory>> {
    let mut names = all_names(conn, CATEGORY)?;
    let mut stmt = conn.prepare(
        "SELECT id, sort_order, color, active FROM categories WHERE hidden = 0
         ORDER BY sort_order, id",
    )?;
    stmt.query_map([], |r| {
        Ok(AdminCategory {
            id: r.get(0)?,
            names: Names::new(),
            sort_order: r.get(1)?,
            color: r.get(2)?,
            active: r.get::<_, i32>(3)? != 0,
        })
    })?
    .map(|c| {
        c.map(|mut c| {
            c.names = names.remove(&c.id).unwrap_or_default();
            c
        })
    })
    .collect()
}

const PRODUCT_COLUMNS: &str = "m.id, m.category_id, m.price_cents, m.vat_code, m.vat_code_takeaway,
    m.kitchen_printer, m.code, m.open_price, m.sort_order, m.active";

fn map_product(r: &rusqlite::Row<'_>) -> rusqlite::Result<AdminProduct> {
    Ok(AdminProduct {
        id: r.get(0)?,
        category_id: r.get(1)?,
        names: Names::new(),
        price_cents: r.get(2)?,
        vat_code: r.get(3)?,
        vat_code_takeaway: r.get(4)?,
        kitchen_printer: r.get::<_, i32>(5)? != 0,
        code: r.get(6)?,
        open_price: r.get::<_, i32>(7)? != 0,
        sort_order: r.get(8)?,
        active: r.get::<_, i32>(9)? != 0,
    })
}

pub fn list_products(conn: &Connection) -> rusqlite::Result<Vec<AdminProduct>> {
    let mut names = all_names(conn, PRODUCT)?;
    let mut stmt = conn.prepare(&format!(
        "SELECT {PRODUCT_COLUMNS} FROM menu_items m
         WHERE m.kind = 'product' AND m.on_pos = 1
         ORDER BY m.category_id, m.sort_order, m.id"
    ))?;
    stmt.query_map([], map_product)?
        .map(|p| {
            p.map(|mut p| {
                p.names = names.remove(&p.id).unwrap_or_default();
                p
            })
        })
        .collect()
}

fn map_group(r: &rusqlite::Row<'_>) -> rusqlite::Result<AdminGroup> {
    Ok(AdminGroup {
        id: r.get(0)?,
        menu_item_id: r.get(1)?,
        names: Names::new(),
        min_select: r.get(2)?,
        max_select: r.get(3)?,
        sort_order: r.get(4)?,
        active: r.get::<_, i32>(5)? != 0,
    })
}

pub fn list_groups(conn: &Connection) -> rusqlite::Result<Vec<AdminGroup>> {
    let mut names = all_names(conn, GROUP)?;
    let mut stmt = conn.prepare(
        "SELECT id, menu_item_id, min_select, max_select, sort_order, active
         FROM modifier_groups ORDER BY menu_item_id, sort_order, id",
    )?;
    stmt.query_map([], map_group)?
        .map(|g| {
            g.map(|mut g| {
                g.names = names.remove(&g.id).unwrap_or_default();
                g
            })
        })
        .collect()
}

fn map_option(r: &rusqlite::Row<'_>) -> rusqlite::Result<AdminOption> {
    Ok(AdminOption {
        id: r.get(0)?,
        group_id: r.get(1)?,
        names: Names::new(),
        price_cents: r.get(2)?,
        sort_order: r.get(3)?,
        active: r.get::<_, i32>(4)? != 0,
    })
}

pub fn list_options(conn: &Connection) -> rusqlite::Result<Vec<AdminOption>> {
    let mut names = all_names(conn, OPTION)?;
    let mut stmt = conn.prepare(
        "SELECT id, group_id, price_cents, sort_order, active
         FROM modifiers ORDER BY group_id, sort_order, id",
    )?;
    stmt.query_map([], map_option)?
        .map(|o| {
            o.map(|mut o| {
                o.names = names.remove(&o.id).unwrap_or_default();
                o
            })
        })
        .collect()
}

pub fn get_category(conn: &Connection, id: i64) -> rusqlite::Result<Option<AdminCategory>> {
    let row = conn
        .query_row(
            "SELECT id, sort_order, color, active FROM categories WHERE id = ?1 AND hidden = 0",
            params![id],
            |r| {
                Ok(AdminCategory {
                    id: r.get(0)?,
                    names: Names::new(),
                    sort_order: r.get(1)?,
                    color: r.get(2)?,
                    active: r.get::<_, i32>(3)? != 0,
                })
            },
        )
        .optional()?;
    row.map(|mut c| {
        c.names = names_of(conn, CATEGORY, id)?;
        Ok(c)
    })
    .transpose()
}

pub fn get_product(conn: &Connection, id: i64) -> rusqlite::Result<Option<AdminProduct>> {
    let row = conn
        .query_row(
            &format!(
                "SELECT {PRODUCT_COLUMNS} FROM menu_items m
                 WHERE m.id = ?1 AND m.kind = 'product' AND m.on_pos = 1"
            ),
            params![id],
            map_product,
        )
        .optional()?;
    row.map(|mut p| {
        p.names = names_of(conn, PRODUCT, id)?;
        Ok(p)
    })
    .transpose()
}

pub fn get_group(conn: &Connection, id: i64) -> rusqlite::Result<Option<AdminGroup>> {
    let row = conn
        .query_row(
            "SELECT id, menu_item_id, min_select, max_select, sort_order, active
             FROM modifier_groups WHERE id = ?1",
            params![id],
            map_group,
        )
        .optional()?;
    row.map(|mut g| {
        g.names = names_of(conn, GROUP, id)?;
        Ok(g)
    })
    .transpose()
}

pub fn get_option(conn: &Connection, id: i64) -> rusqlite::Result<Option<AdminOption>> {
    let row = conn
        .query_row(
            "SELECT id, group_id, price_cents, sort_order, active FROM modifiers WHERE id = ?1",
            params![id],
            map_option,
        )
        .optional()?;
    row.map(|mut o| {
        o.names = names_of(conn, OPTION, id)?;
        Ok(o)
    })
    .transpose()
}

/// Write the names of an entity: a non-empty value is kept (replacing the old one), a missing
/// or empty one removes that language's translation.
fn set_names(conn: &Connection, entity_type: &str, id: i64, names: &Names) -> rusqlite::Result<()> {
    let kind = translation_type(entity_type);
    conn.execute(
        "DELETE FROM translations WHERE entity_type = ?1 AND entity_id = ?2 AND field = 'name'",
        params![kind, id],
    )?;
    for (lang, value) in names {
        if value.is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO translations (entity_type, entity_id, field, lang, value)
             VALUES (?1, ?2, 'name', ?3, ?4)",
            params![kind, id, lang, value],
        )?;
    }
    Ok(())
}

/// Insert (`id` 0) or update a category and its names; returns its id.
pub fn write_category(conn: &Connection, c: &AdminCategory) -> rusqlite::Result<i64> {
    let id = if c.id == 0 {
        conn.execute(
            "INSERT INTO categories (sort_order, color, active) VALUES (?1, ?2, ?3)",
            params![c.sort_order, c.color, c.active],
        )?;
        conn.last_insert_rowid()
    } else {
        conn.execute(
            "UPDATE categories SET sort_order = ?2, color = ?3, active = ?4 WHERE id = ?1",
            params![c.id, c.sort_order, c.color, c.active],
        )?;
        c.id
    };
    set_names(conn, CATEGORY, id, &c.names)?;
    Ok(id)
}

/// Insert (`id` 0) or update a product and its names; returns its id.
pub fn write_product(conn: &Connection, p: &AdminProduct) -> rusqlite::Result<i64> {
    let id = if p.id == 0 {
        conn.execute(
            "INSERT INTO menu_items (category_id, price_cents, vat_code, vat_code_takeaway,
                                     kitchen_printer, code, open_price, sort_order, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                p.category_id,
                p.price_cents,
                p.vat_code,
                p.vat_code_takeaway,
                p.kitchen_printer,
                p.code,
                p.open_price,
                p.sort_order,
                p.active
            ],
        )?;
        conn.last_insert_rowid()
    } else {
        conn.execute(
            "UPDATE menu_items SET category_id = ?2, price_cents = ?3, vat_code = ?4,
                    vat_code_takeaway = ?5, kitchen_printer = ?6, code = ?7, open_price = ?8,
                    sort_order = ?9, active = ?10
             WHERE id = ?1",
            params![
                p.id,
                p.category_id,
                p.price_cents,
                p.vat_code,
                p.vat_code_takeaway,
                p.kitchen_printer,
                p.code,
                p.open_price,
                p.sort_order,
                p.active
            ],
        )?;
        p.id
    };
    set_names(conn, PRODUCT, id, &p.names)?;
    Ok(id)
}

/// Insert (`id` 0) or update an option group and its names; returns its id.
pub fn write_group(conn: &Connection, g: &AdminGroup) -> rusqlite::Result<i64> {
    let id =
        if g.id == 0 {
            conn.execute(
            "INSERT INTO modifier_groups (menu_item_id, min_select, max_select, sort_order, active)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![g.menu_item_id, g.min_select, g.max_select, g.sort_order, g.active],
        )?;
            conn.last_insert_rowid()
        } else {
            conn.execute(
                "UPDATE modifier_groups SET menu_item_id = ?2, min_select = ?3, max_select = ?4,
                    sort_order = ?5, active = ?6
             WHERE id = ?1",
                params![
                    g.id,
                    g.menu_item_id,
                    g.min_select,
                    g.max_select,
                    g.sort_order,
                    g.active
                ],
            )?;
            g.id
        };
    set_names(conn, GROUP, id, &g.names)?;
    Ok(id)
}

/// Insert (`id` 0) or update an option and its names; returns its id.
pub fn write_option(conn: &Connection, o: &AdminOption) -> rusqlite::Result<i64> {
    let id = if o.id == 0 {
        conn.execute(
            "INSERT INTO modifiers (group_id, price_cents, sort_order, active)
             VALUES (?1, ?2, ?3, ?4)",
            params![o.group_id, o.price_cents, o.sort_order, o.active],
        )?;
        conn.last_insert_rowid()
    } else {
        conn.execute(
            "UPDATE modifiers SET group_id = ?2, price_cents = ?3, sort_order = ?4, active = ?5
             WHERE id = ?1",
            params![o.id, o.group_id, o.price_cents, o.sort_order, o.active],
        )?;
        o.id
    };
    set_names(conn, OPTION, id, &o.names)?;
    Ok(id)
}

/// Append one line to the log.
pub fn log_change(
    conn: &Connection,
    entity_type: &str,
    entity_id: i64,
    before_json: Option<&str>,
    after_json: &str,
    changed_by: &str,
) -> rusqlite::Result<()> {
    let action = if before_json.is_some() {
        "update"
    } else {
        "create"
    };
    conn.execute(
        "INSERT INTO menu_change_log
             (entity_type, entity_id, action, before_json, after_json, changed_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entity_type,
            entity_id,
            action,
            before_json,
            after_json,
            changed_by
        ],
    )?;
    Ok(())
}

/// The log, newest first.
pub fn list_changes(conn: &Connection, limit: i64) -> rusqlite::Result<Vec<ChangeRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, entity_type, entity_id, action, before_json, after_json, changed_by, changed_at
         FROM menu_change_log ORDER BY id DESC LIMIT ?1",
    )?;
    stmt.query_map(params![limit], |r| {
        Ok(ChangeRow {
            id: r.get(0)?,
            entity_type: r.get(1)?,
            entity_id: r.get(2)?,
            action: r.get(3)?,
            before_json: r.get(4)?,
            after_json: r.get(5)?,
            changed_by: r.get(6)?,
            changed_at: r.get(7)?,
        })
    })?
    .collect()
}
