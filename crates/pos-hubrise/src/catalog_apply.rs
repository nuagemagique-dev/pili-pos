//! Apply a HubRise catalog snapshot to the local SQLite DB in a
//! single transaction. Uses soft delete (`active = 0`) to preserve
//! FK references from fiscal archives.

use pos_hubrise_types::Catalog;
use rusqlite::{Connection, Transaction, params};

use crate::catalog_mapper::{decode_category_translations, decode_sku_translations};
use crate::errors::Result;

/// Derive the default SCE 2.0 vat_code to persist on menu_items
/// given the HubRise vat_category. Because menu_items doesn't know
/// about service type, we default to "takeaway" semantics:
///
///   food → C (6%),  beverage → A (21%),  zero_rated → D,  out_of_scope → X
fn default_vat_code(category: &str) -> &'static str {
    match category {
        "food" => "C",
        "beverage" => "A",
        "zero_rated" => "D",
        "out_of_scope" => "X",
        _ => "C",
    }
}

/// Apply the given catalog snapshot atomically. On success,
/// `last_catalog_sync_at` is bumped.
pub fn apply(conn: &mut Connection, catalog: &Catalog) -> Result<()> {
    let tx = conn.transaction()?;

    // 1. Soft-delete everything - rows are resurrected by the UPSERT
    //    loop below for items still present in the snapshot.
    tx.execute("UPDATE categories SET active = 0", [])?;
    tx.execute("UPDATE menu_items SET active = 0", [])?;

    let Some(data) = catalog.data.as_ref() else {
        // Catalog without data - treat as a no-op apart from the
        // soft-delete sweep and cursor bump below.
        tx.execute(
            "UPDATE hubrise_connection SET last_catalog_sync_at = datetime('now') WHERE id = 1",
            [],
        )?;
        tx.commit()?;
        return Ok(());
    };

    // 2. Categories.
    for cat in &data.categories {
        upsert_category(&tx, cat)?;
    }

    // 3. Products → menu_items (1 product = 1 SKU convention).
    for product in &data.products {
        if product.skus.is_empty() {
            continue;
        }
        upsert_product(&tx, product)?;
    }

    // 4. Option lists + options → modifier_groups + modifiers.
    for list in &data.option_lists {
        upsert_option_list(&tx, list)?;
    }

    // 5. Bump sync timestamp.
    tx.execute(
        "UPDATE hubrise_connection SET last_catalog_sync_at = datetime('now') WHERE id = 1",
        [],
    )?;

    tx.commit()?;
    Ok(())
}

fn upsert_category(tx: &Transaction<'_>, cat: &pos_hubrise_types::Category) -> Result<()> {
    let decoded = decode_category_translations(&cat.description);
    let external_id = &cat.ref_field;

    // Upsert the category row (keyed by external_id).
    tx.execute(
        "INSERT INTO categories (external_id, sort_order, color, active)
         VALUES (?1, 0, NULL, 1)
         ON CONFLICT(external_id) DO UPDATE SET active = 1",
        params![external_id],
    )?;

    // Fetch the local integer id.
    let local_id: i64 = tx.query_row(
        "SELECT id FROM categories WHERE external_id = ?1",
        params![external_id],
        |r| r.get(0),
    )?;

    // Refresh translation rows.
    tx.execute(
        "DELETE FROM translations WHERE entity_type = 'category' AND entity_id = ?1",
        params![local_id],
    )?;
    insert_translation(tx, "category", local_id, "fr", &decoded.fr)?;
    insert_translation(tx, "category", local_id, "en", &decoded.en)?;
    insert_translation(tx, "category", local_id, "nl", &decoded.nl)?;
    insert_translation(tx, "category", local_id, "zh", &decoded.zh)?;
    Ok(())
}

fn upsert_product(tx: &Transaction<'_>, product: &pos_hubrise_types::Product) -> Result<()> {
    let sku = &product.skus[0];
    let decoded = decode_sku_translations(&sku.custom_fields);
    let vat_code = default_vat_code(&decoded.vat_category);

    // Parse price from "12.50 EUR" → 1250 cents.
    let price_cents = parse_price_cents(&sku.price);

    let category_local_id: i64 = tx.query_row(
        "SELECT id FROM categories WHERE external_id = ?1",
        params![product.category_ref],
        |r| r.get(0),
    )?;

    let external_id = sku
        .ref_field
        .clone()
        .or_else(|| product.ref_field.clone())
        .unwrap_or_else(|| format!("sku_{}", uuid_like(product)));

    tx.execute(
        "INSERT INTO menu_items
            (external_id, category_id, price_cents, vat_code, vat_category, kitchen_printer, sort_order, active)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, 0, 1)
         ON CONFLICT(external_id) DO UPDATE SET
            category_id = excluded.category_id,
            price_cents = excluded.price_cents,
            vat_code = excluded.vat_code,
            vat_category = excluded.vat_category,
            active = 1",
        params![external_id, category_local_id, price_cents, vat_code, decoded.vat_category],
    )?;

    let local_id: i64 = tx.query_row(
        "SELECT id FROM menu_items WHERE external_id = ?1",
        params![external_id],
        |r| r.get(0),
    )?;

    tx.execute(
        "DELETE FROM translations WHERE entity_type = 'menu_item' AND entity_id = ?1",
        params![local_id],
    )?;
    insert_translation(
        tx,
        "menu_item",
        local_id,
        "fr",
        &decoded.translations.fr.name,
    )?;
    insert_translation(
        tx,
        "menu_item",
        local_id,
        "en",
        &decoded.translations.en.name,
    )?;
    insert_translation(
        tx,
        "menu_item",
        local_id,
        "nl",
        &decoded.translations.nl.name,
    )?;
    insert_translation(
        tx,
        "menu_item",
        local_id,
        "zh",
        &decoded.translations.zh.name,
    )?;

    Ok(())
}

fn upsert_option_list(_tx: &Transaction<'_>, _list: &pos_hubrise_types::OptionList) -> Result<()> {
    // Minimal skeleton - option list sync will be fleshed out when the
    // POS UI grows a modifier selection screen. For now we accept
    // catalogs with option lists but don't persist them locally,
    // preventing accidental constraint violations on parent menu_items.
    Ok(())
}

fn insert_translation(
    tx: &Transaction<'_>,
    entity_type: &str,
    entity_id: i64,
    lang: &str,
    value: &str,
) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    tx.execute(
        "INSERT INTO translations (entity_type, entity_id, field, lang, value)
         VALUES (?1, ?2, 'name', ?3, ?4)",
        params![entity_type, entity_id, lang, value],
    )?;
    Ok(())
}

/// Parse a HubRise price string like "12.50 EUR" → 1250 cents.
fn parse_price_cents(s: &str) -> i64 {
    let number = s.split_whitespace().next().unwrap_or("0");
    match number.parse::<f64>() {
        Ok(f) => (f * 100.0).round() as i64,
        Err(_) => 0,
    }
}

fn uuid_like(p: &pos_hubrise_types::Product) -> String {
    p.ref_field.clone().unwrap_or_else(|| p.name.clone())
}
