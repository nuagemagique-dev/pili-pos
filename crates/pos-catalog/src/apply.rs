//! Write a validated catalog into the POS menu, in one transaction.
//!
//! Rows are matched on `external_id` (the source's key), so a rename,
//! a price change, or an item removed then added back keeps its
//! `menu_items.id`: the `productId` the FDM has already seen. Nothing
//! is ever deleted, since fiscal archives and order lines point at
//! these rows: what the catalog no longer has is deactivated.

use std::collections::HashSet;

use pos_core::allergen::AllergenSet;
use pos_db::queries::allergens::{self, Entity};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use crate::hash::hash;
use crate::model::{Catalog, Choice, Group, Item, ItemKind, Names, fiscal_name};
use crate::validate::{Issue, validate};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ApplyOutcome {
    /// The menu now mirrors the catalog.
    Applied {
        hash: String,
        active_items: usize,
        /// Items on sale before and no longer.
        deactivated_items: usize,
    },
    /// Same catalog as the last applied one: nothing written.
    Unchanged,
    /// First sync over a menu entered by hand: the catalog is kept
    /// aside until a manager picks the products to adopt
    /// ([`adopt_and_apply`]); the menu is untouched.
    AwaitingAdoption,
    /// The catalog has issues: the menu is untouched, the issues are
    /// stored for Settings. A struct variant, so the internally tagged
    /// form serializes: `{"outcome": "rejected", "issues": [...]}` (a
    /// tagged newtype holding a sequence has no serde form).
    Rejected { issues: Vec<Issue> },
}

/// Validate `catalog` and, if it is sound, make the menu mirror it.
/// `source` names where it came from (`"hubrise"`); that source then
/// manages the menu (see [`managed_by`]).
pub fn apply(conn: &mut Connection, source: &str, catalog: &Catalog) -> Result<ApplyOutcome> {
    apply_checked(conn, source, catalog, Vec::new())
}

/// [`apply`], with the issues the source found while mapping its own
/// format into `catalog`: any of them refuses the catalog too.
pub fn apply_checked(
    conn: &mut Connection,
    source: &str,
    catalog: &Catalog,
    mut issues: Vec<Issue>,
) -> Result<ApplyOutcome> {
    issues.extend(validate(catalog));
    if !issues.is_empty() {
        record_rejection(conn, source, catalog, &issues)?;
        return Ok(ApplyOutcome::Rejected { issues });
    }
    if has_unkeyed_items(conn)? {
        hold_for_adoption(conn, source, catalog)?;
        return Ok(ApplyOutcome::AwaitingAdoption);
    }
    write_menu(conn, source, catalog)
}

/// Products on sale that no catalog has keyed yet: a menu entered by
/// hand (seed) before the first sync.
fn has_unkeyed_items(conn: &Connection) -> Result<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM menu_items WHERE external_id IS NULL AND active = 1)",
        [],
        |r| r.get(0),
    )?)
}

fn hold_for_adoption(conn: &Connection, source: &str, catalog: &Catalog) -> Result<()> {
    conn.execute(
        "INSERT INTO catalog_sync_state (source, rejected_at, issues_json, catalog_json)
         VALUES (?1, NULL, NULL, ?2)
         ON CONFLICT(source) DO UPDATE SET
            rejected_at = NULL, issues_json = NULL, catalog_json = excluded.catalog_json",
        params![source, serde_json::to_string(catalog)?],
    )?;
    Ok(())
}

/// The catalog kept aside by [`ApplyOutcome::AwaitingAdoption`].
pub fn pending_catalog(conn: &Connection, source: &str) -> Result<Option<Catalog>> {
    let json: Option<Option<String>> = conn
        .query_row(
            "SELECT catalog_json FROM catalog_sync_state
             WHERE source = ?1 AND manages_menu = 0",
            params![source],
            |r| r.get(0),
        )
        .optional()?;
    match json.flatten() {
        Some(json) => Ok(Some(serde_json::from_str(&json)?)),
        None => Ok(None),
    }
}

/// The manager's answer to [`ApplyOutcome::AwaitingAdoption`]: the
/// chosen local products take their catalog key (and keep their id),
/// every other hand-entered product is deactivated, and the menu
/// mirrors the pending catalog. `None` when nothing is pending.
pub fn adopt_and_apply(
    conn: &mut Connection,
    source: &str,
    adoptions: &[Adoption],
) -> Result<Option<ApplyOutcome>> {
    let Some(catalog) = pending_catalog(conn, source)? else {
        return Ok(None);
    };
    let issues = validate(&catalog);
    if !issues.is_empty() {
        record_rejection(conn, source, &catalog, &issues)?;
        return Ok(Some(ApplyOutcome::Rejected { issues }));
    }
    adopt(conn, adoptions)?;
    write_menu(conn, source, &catalog).map(Some)
}

fn write_menu(conn: &mut Connection, source: &str, catalog: &Catalog) -> Result<ApplyOutcome> {
    let new_hash = hash(catalog);
    let tx = conn.transaction()?;
    let last: Option<Option<String>> = tx
        .query_row(
            "SELECT hash FROM catalog_sync_state WHERE source = ?1 AND manages_menu = 1",
            params![source],
            |r| r.get(0),
        )
        .optional()?;
    if last.flatten().as_deref() == Some(new_hash.as_str()) {
        tx.execute(
            "UPDATE catalog_sync_state SET rejected_at = NULL, issues_json = NULL
             WHERE source = ?1",
            params![source],
        )?;
        tx.commit()?;
        return Ok(ApplyOutcome::Unchanged);
    }

    let before = active_item_ids(&tx)?;
    tx.execute_batch(
        "UPDATE categories SET active = 0;
         UPDATE menu_items SET active = 0;
         UPDATE modifier_groups SET active = 0;
         UPDATE modifiers SET active = 0;",
    )?;

    for (sort, cat) in catalog.categories.iter().enumerate() {
        let id: i64 = tx.query_row(
            "INSERT INTO categories (external_id, sort_order, active, hidden)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(external_id) DO UPDATE SET
                sort_order = excluded.sort_order, active = 1, hidden = excluded.hidden
             RETURNING id",
            params![cat.key, sort_key(sort), !cat.visible],
            |r| r.get(0),
        )?;
        write_names(&tx, "category", id, &cat.names)?;
    }

    for (sort, item) in catalog.items.iter().enumerate() {
        let id = upsert_item(&tx, item, sort)?;
        write_names(&tx, "menu_item", id, &item.names)?;
        write_allergens(&tx, Entity::MenuItem, id, item.allergens.as_ref())?;
        for (group_sort, group) in item.groups.iter().enumerate() {
            let group_id = upsert_group(&tx, id, group, group_sort)?;
            write_names(&tx, "modifier_group", group_id, &group.names)?;
            for (option_sort, choice) in group.options.iter().enumerate() {
                let option_id = upsert_choice(&tx, group_id, choice, option_sort)?;
                write_names(&tx, "modifier", option_id, &choice.names)?;
                write_allergens(&tx, Entity::Modifier, option_id, choice.allergens.as_ref())?;
            }
        }
    }

    let after = active_item_ids(&tx)?;
    tx.execute(
        "INSERT INTO catalog_sync_state
            (source, manages_menu, hash, applied_at, rejected_at, issues_json, catalog_json)
         VALUES (?1, 1, ?2, datetime('now'), NULL, NULL, ?3)
         ON CONFLICT(source) DO UPDATE SET
            manages_menu = 1, hash = excluded.hash, applied_at = excluded.applied_at,
            rejected_at = NULL, issues_json = NULL, catalog_json = excluded.catalog_json",
        params![source, new_hash, serde_json::to_string(catalog)?],
    )?;
    tx.commit()?;

    Ok(ApplyOutcome::Applied {
        hash: new_hash,
        active_items: after.len(),
        deactivated_items: before.difference(&after).count(),
    })
}

fn active_item_ids(tx: &Transaction<'_>) -> Result<HashSet<i64>> {
    let mut stmt = tx.prepare("SELECT id FROM menu_items WHERE active = 1")?;
    let ids = stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<HashSet<i64>>>()?;
    Ok(ids)
}

/// `vat_category` is the legacy column the old `HubRise` push reads;
/// derived from the eat-in label.
fn vat_category(item: &Item) -> &'static str {
    use pos_core::vat::VatCode;
    match item.vat_eat_in {
        VatCode::A => "beverage",
        VatCode::D => "zero_rated",
        VatCode::X => "out_of_scope",
        VatCode::B | VatCode::C => "food",
    }
}

/// A list position as the `sort_order` column value.
fn sort_key(sort: usize) -> i64 {
    i64::try_from(sort).unwrap_or(i64::MAX)
}

fn upsert_item(tx: &Transaction<'_>, item: &Item, sort: usize) -> Result<i64> {
    let category_id: i64 = tx.query_row(
        "SELECT id FROM categories WHERE external_id = ?1",
        params![item.category_key],
        |r| r.get(0),
    )?;
    let kind = match item.kind {
        ItemKind::Product => "product",
        ItemKind::Charge => "charge",
    };
    let id = tx.query_row(
        "INSERT INTO menu_items
            (external_id, category_id, price_cents, vat_code, vat_code_takeaway, vat_category,
             kitchen_printer, sort_order, active, open_price, kind, on_pos, code)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(external_id) DO UPDATE SET
            category_id = excluded.category_id, price_cents = excluded.price_cents,
            vat_code = excluded.vat_code, vat_code_takeaway = excluded.vat_code_takeaway,
            vat_category = excluded.vat_category, kitchen_printer = excluded.kitchen_printer,
            sort_order = excluded.sort_order, active = excluded.active,
            open_price = excluded.open_price, kind = excluded.kind, on_pos = excluded.on_pos,
            code = excluded.code
         RETURNING id",
        params![
            item.key,
            category_id,
            item.price_cents,
            item.vat_eat_in.as_str(),
            item.vat_takeaway.as_str(),
            vat_category(item),
            item.kitchen,
            sort_key(sort),
            item.available,
            item.open_price,
            kind,
            item.on_pos,
            item.code,
        ],
        |r| r.get(0),
    )?;
    Ok(id)
}

fn upsert_group(tx: &Transaction<'_>, item_id: i64, group: &Group, sort: usize) -> Result<i64> {
    // -1: no maximum.
    let max = group.max.map_or(-1, i64::from);
    let id = tx.query_row(
        "INSERT INTO modifier_groups
            (external_id, min_select, max_select, menu_item_id, active, sort_order)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)
         ON CONFLICT(external_id) DO UPDATE SET
            min_select = excluded.min_select, max_select = excluded.max_select,
            menu_item_id = excluded.menu_item_id, active = 1, sort_order = excluded.sort_order
         RETURNING id",
        params![group.key, group.min, max, item_id, sort_key(sort)],
        |r| r.get(0),
    )?;
    Ok(id)
}

fn upsert_choice(tx: &Transaction<'_>, group_id: i64, choice: &Choice, sort: usize) -> Result<i64> {
    let id = tx.query_row(
        "INSERT INTO modifiers (external_id, group_id, price_cents, sort_order, active)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(external_id) DO UPDATE SET
            group_id = excluded.group_id, price_cents = excluded.price_cents,
            sort_order = excluded.sort_order, active = excluded.active
         RETURNING id",
        params![
            choice.key,
            group_id,
            choice.price_cents,
            sort_key(sort),
            choice.available
        ],
        |r| r.get(0),
    )?;
    Ok(id)
}

/// The catalog's allergen statement about one row: a log row only when
/// the list really changes (V12-D18).
fn write_allergens(
    tx: &Transaction<'_>,
    entity: Entity,
    id: i64,
    set: Option<&AllergenSet>,
) -> Result<()> {
    allergens::write_from_catalog(tx, entity, id, set)?;
    Ok(())
}

/// Replace the names of one row. Translations are display data, not
/// fiscal archive: the name sent to the FDM is copied onto the order
/// line when it is added.
fn write_names(tx: &Transaction<'_>, entity: &str, id: i64, names: &Names) -> Result<()> {
    tx.execute(
        "DELETE FROM translations WHERE entity_type = ?1 AND entity_id = ?2 AND field = 'name'",
        params![entity, id],
    )?;
    for (lang, value) in names {
        if value.trim().is_empty() {
            continue;
        }
        tx.execute(
            "INSERT INTO translations (entity_type, entity_id, field, lang, value)
             VALUES (?1, ?2, 'name', ?3, ?4)",
            params![entity, id, lang, value],
        )?;
    }
    Ok(())
}

/// Keep the issues of a refused catalog; the applied menu and its
/// hash stay as they were.
pub fn record_rejection(
    conn: &Connection,
    source: &str,
    catalog: &Catalog,
    issues: &[Issue],
) -> Result<()> {
    conn.execute(
        "INSERT INTO catalog_sync_state (source, rejected_at, issues_json, catalog_json)
         VALUES (?1, datetime('now'), ?2, ?3)
         ON CONFLICT(source) DO UPDATE SET
            rejected_at = excluded.rejected_at, issues_json = excluded.issues_json,
            catalog_json = excluded.catalog_json",
        params![
            source,
            serde_json::to_string(issues)?,
            serde_json::to_string(catalog)?
        ],
    )?;
    Ok(())
}

/// The source that manages the menu, if any: local menu editing is
/// then refused.
pub fn managed_by(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT source FROM catalog_sync_state WHERE manages_menu = 1
             ORDER BY applied_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?)
}

/// The source stops managing the menu (disconnected). The menu stays
/// as last applied; editing is allowed again.
pub fn release(conn: &Connection, source: &str) -> Result<()> {
    conn.execute(
        "UPDATE catalog_sync_state SET manages_menu = 0, hash = NULL WHERE source = ?1",
        params![source],
    )?;
    Ok(())
}

/// What Settings shows about a source's catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncState {
    pub source: String,
    pub manages_menu: bool,
    pub applied_at: Option<String>,
    pub rejected_at: Option<String>,
    pub issues: Vec<Issue>,
}

pub fn state(conn: &Connection, source: &str) -> Result<Option<SyncState>> {
    let row = conn
        .query_row(
            "SELECT manages_menu, applied_at, rejected_at, issues_json
             FROM catalog_sync_state WHERE source = ?1",
            params![source],
            |r| {
                Ok((
                    r.get::<_, bool>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((manages_menu, applied_at, rejected_at, issues_json)) = row else {
        return Ok(None);
    };
    let issues = match issues_json {
        Some(json) => serde_json::from_str(&json)?,
        None => Vec::new(),
    };
    Ok(Some(SyncState {
        source: source.to_owned(),
        manages_menu,
        applied_at,
        rejected_at,
        issues,
    }))
}

/// A local product that matches a catalog item (same fiscal name and
/// price) and can take its key, keeping its id, before the first
/// apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Adoption {
    pub menu_item_id: i64,
    pub key: String,
    pub name: String,
    pub price_cents: i64,
}

/// Local products without a key that match exactly one catalog product
/// by French name and price, and that no other local product matches.
pub fn adoption_preview(conn: &Connection, catalog: &Catalog) -> Result<Vec<Adoption>> {
    let taken: HashSet<String> = {
        let mut stmt =
            conn.prepare("SELECT external_id FROM menu_items WHERE external_id IS NOT NULL")?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let locals: Vec<(i64, String, i64)> = {
        let mut stmt = conn.prepare(
            "SELECT m.id, t.value, m.price_cents FROM menu_items m
             JOIN translations t ON t.entity_type = 'menu_item' AND t.entity_id = m.id
                AND t.field = 'name' AND t.lang = 'fr'
             WHERE m.external_id IS NULL AND m.active = 1
             ORDER BY m.id",
        )?;
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?
    };

    let matches = |name: &str, price: i64| -> Vec<&Item> {
        catalog
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Product && !taken.contains(&i.key))
            .filter(|i| i.price_cents == price && fiscal_name(&i.names) == Some(name))
            .collect()
    };

    let mut out = Vec::new();
    for (id, name, price) in &locals {
        let found = matches(name, *price);
        let [only] = found.as_slice() else { continue };
        let rivals = locals
            .iter()
            .filter(|(_, n, p)| n == name && p == price)
            .count();
        if rivals == 1 {
            out.push(Adoption {
                menu_item_id: *id,
                key: only.key.clone(),
                name: name.clone(),
                price_cents: *price,
            });
        }
    }
    Ok(out)
}

/// Give the chosen local products their catalog key; the next apply
/// then updates them in place instead of creating new rows.
pub fn adopt(conn: &mut Connection, adoptions: &[Adoption]) -> Result<usize> {
    let tx = conn.transaction()?;
    let mut done = 0;
    for a in adoptions {
        done += tx.execute(
            "UPDATE menu_items SET external_id = ?1 WHERE id = ?2 AND external_id IS NULL",
            params![a.key, a.menu_item_id],
        )?;
    }
    tx.commit()?;
    Ok(done)
}
