//! The on-screen menu editor (V11-D17, V11-D18; feature `hubrise`, which carries the schema of
//! option groups and of the product code): a manager creates and edits categories, products,
//! option groups and options at the till, on installations whose menu no external catalog
//! manages.
//!
//! - Reads (`menu_admin_get`, `list_menu_changes`) work whoever manages the menu, so the screen
//!   can say who does; every save is refused with `hubrise_menu_managed` while a source
//!   manages the menu.
//! - Nothing is deleted: an entry is hidden with `active = false`.
//! - A save writes the row, its translations and one `menu_change_log` row (append-only: the
//!   row before and after as JSON, who, when) in one transaction. A save that changes nothing
//!   writes nothing.
//! - Order lines keep a snapshot of name, price, VAT label and department when they are added,
//!   so an edit never alters an open order or a signed event (AM art. 44). Tests prove it.

use std::collections::BTreeSet;

use pos_db::queries::menu_admin::{
    self as q, AdminCategory, AdminGroup, AdminOption, AdminProduct, ChangeRow, Names,
};
use serde::{Deserialize, Serialize};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::state::AppState;

/// Languages a name can be given in. German only when the establishment's language is German.
const LANGUAGES: [&str; 4] = ["fr", "nl", "en", "zh"];
/// Longest name, in characters.
const NAME_MAX: usize = 60;
/// Longest product code, in characters.
const CODE_MAX: usize = 12;
/// Highest price: 9 999,99 EUR.
const PRICE_MAX: i64 = 999_999;
const SORT_MAX: i32 = 99_999;

/// The whole menu, hidden entries included, for the editor.
#[derive(Debug, Clone, Serialize)]
pub struct MenuAdmin {
    /// The source that manages the menu (`hubrise`), if any: the editor is then read-only.
    pub managed_by: Option<String>,
    /// The language codes a name can be given in; the first is the establishment's language,
    /// whose name is required.
    pub languages: Vec<String>,
    pub categories: Vec<AdminCategory>,
    pub products: Vec<AdminProduct>,
    pub groups: Vec<AdminGroup>,
    pub options: Vec<AdminOption>,
}

/// The answer of a save: the id of the entity written and the menu as it is now.
#[derive(Debug, Clone, Serialize)]
pub struct MenuSaved {
    pub id: i64,
    pub menu: MenuAdmin,
}

/// One line of the change log, ready to show.
#[derive(Debug, Clone, Serialize)]
pub struct MenuChange {
    pub id: i64,
    /// `category`, `product`, `modifier_group` or `modifier`.
    pub entity_type: String,
    pub entity_id: i64,
    /// `create` or `update`.
    pub action: String,
    /// The name of the entry after the change (fiscal language, then French, then any).
    pub label: String,
    /// The fields that changed (all of them on a creation).
    pub changed_fields: Vec<String>,
    pub before_json: Option<String>,
    pub after_json: String,
    pub changed_by: String,
    pub changed_by_name: String,
    pub changed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveCategoryInput {
    /// `None` creates.
    pub id: Option<i64>,
    pub names: Names,
    pub sort_order: i32,
    /// `#RRGGBB` or none.
    pub color: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProductInput {
    pub id: Option<i64>,
    pub category_id: i64,
    pub names: Names,
    pub price_cents: i64,
    /// `A`, `B`, `C`, `D` or `X`.
    pub vat_code: String,
    /// The label for takeaway sales; none: the dine-in label applies.
    pub vat_code_takeaway: Option<String>,
    pub kitchen_printer: bool,
    pub code: Option<String>,
    pub open_price: bool,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveModifierGroupInput {
    pub id: Option<i64>,
    pub menu_item_id: i64,
    pub names: Names,
    pub min_select: i32,
    pub max_select: i32,
    pub sort_order: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveModifierInput {
    pub id: Option<i64>,
    pub group_id: i64,
    pub names: Names,
    pub price_cents: i64,
    pub sort_order: i32,
    pub active: bool,
}

fn invalid(field: &str) -> String {
    errors::coded(errors::MENU_INVALID, &[("field", &field)])
}

fn not_found(entity: &str, id: i64) -> String {
    errors::coded(errors::MENU_NOT_FOUND, &[("entity", &entity), ("id", &id)])
}

/// The establishment's language code (`fr`, `nl`, `de`, `en` or `zh`).
fn fiscal_language(state: &AppState) -> Result<String, String> {
    Ok(state
        .config
        .lock()
        .map_err(errors::internal)?
        .language
        .clone())
}

/// The language codes a name can be given in: the establishment's first. German is offered only
/// by a German till (it is then the first).
fn languages_of(fiscal: &str) -> Vec<String> {
    let mut out = vec![fiscal.to_string()];
    for l in LANGUAGES {
        if l != fiscal {
            out.push(l.to_string());
        }
    }
    out
}

/// Refuse a save while a source manages the menu.
fn refuse_if_managed(db: &rusqlite::Connection) -> Result<(), String> {
    match pos_catalog::apply::managed_by(db).map_err(errors::internal)? {
        Some(_) => Err(errors::hubrise::MENU_MANAGED.to_string()),
        None => Ok(()),
    }
}

/// Names trimmed, empty ones dropped, only known languages, no control characters, bounded.
/// The establishment's language is required.
fn clean_names(input: &Names, fiscal: &str) -> Result<Names, String> {
    let allowed: BTreeSet<String> = languages_of(fiscal).into_iter().collect();
    let mut out = Names::new();
    for (lang, value) in input {
        if !allowed.contains(lang) {
            return Err(invalid("names"));
        }
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > NAME_MAX || value.chars().any(char::is_control) {
            return Err(invalid("name"));
        }
        out.insert(lang.clone(), value.to_string());
    }
    if !out.contains_key(fiscal) {
        return Err(invalid("name"));
    }
    Ok(out)
}

fn clean_sort(sort_order: i32) -> Result<i32, String> {
    if (0..=SORT_MAX).contains(&sort_order) {
        Ok(sort_order)
    } else {
        Err(invalid("sort_order"))
    }
}

fn clean_price(field: &str, cents: i64) -> Result<i64, String> {
    if (0..=PRICE_MAX).contains(&cents) {
        Ok(cents)
    } else {
        Err(invalid(field))
    }
}

fn clean_vat(field: &str, code: &str) -> Result<String, String> {
    match code {
        "A" | "B" | "C" | "D" | "X" => Ok(code.to_string()),
        _ => Err(invalid(field)),
    }
}

fn clean_color(color: Option<&str>) -> Result<Option<String>, String> {
    let Some(color) = color.map(str::trim).filter(|c| !c.is_empty()) else {
        return Ok(None);
    };
    let ok = color.len() == 7
        && color.starts_with('#')
        && color[1..].chars().all(|c| c.is_ascii_hexdigit());
    if ok {
        Ok(Some(color.to_ascii_uppercase()))
    } else {
        Err(invalid("color"))
    }
}

fn clean_code(code: Option<&str>) -> Result<Option<String>, String> {
    let Some(code) = code.map(str::trim).filter(|c| !c.is_empty()) else {
        return Ok(None);
    };
    if code.chars().count() > CODE_MAX || code.chars().any(char::is_control) {
        return Err(invalid("code"));
    }
    Ok(Some(code.to_string()))
}

fn read_menu(db: &rusqlite::Connection, fiscal: &str) -> Result<MenuAdmin, String> {
    Ok(MenuAdmin {
        managed_by: pos_catalog::apply::managed_by(db).map_err(errors::internal)?,
        languages: languages_of(fiscal),
        categories: q::list_categories(db).map_err(errors::internal)?,
        products: q::list_products(db).map_err(errors::internal)?,
        groups: q::list_groups(db).map_err(errors::internal)?,
        options: q::list_options(db).map_err(errors::internal)?,
    })
}

/// The whole menu with its names per language, hidden entries included. Manager only.
pub fn do_menu_admin_get(state: &AppState) -> Result<MenuAdmin, String> {
    access::operator(state, Access::Admin)?;
    let fiscal = fiscal_language(state)?;
    let db = state.db.lock().map_err(errors::internal)?;
    read_menu(&db, &fiscal)
}

/// Write one entity and its log row in one transaction; a save that changes nothing writes
/// nothing. Returns the entity's id.
fn commit<T: Serialize + PartialEq>(
    db: &rusqlite::Connection,
    entity_type: &str,
    before: Option<&T>,
    after: &T,
    by: &str,
    write: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<i64>,
    entity_id_if_unchanged: i64,
) -> Result<i64, String> {
    if before == Some(after) {
        return Ok(entity_id_if_unchanged);
    }
    let tx = db.unchecked_transaction().map_err(errors::internal)?;
    let id = write(&tx).map_err(errors::internal)?;
    // The "after" snapshot carries the id the row got.
    let mut after_value = serde_json::to_value(after).map_err(errors::internal)?;
    after_value["id"] = id.into();
    let before_json = before
        .map(serde_json::to_string)
        .transpose()
        .map_err(errors::internal)?;
    let after_json = serde_json::to_string(&after_value).map_err(errors::internal)?;
    q::log_change(
        &tx,
        entity_type,
        id,
        before_json.as_deref(),
        &after_json,
        by,
    )
    .map_err(errors::internal)?;
    tx.commit().map_err(errors::internal)?;
    tracing::info!(entity_type, id, "menu entry saved");
    Ok(id)
}

fn saved(state: &AppState, id: i64) -> Result<MenuSaved, String> {
    let fiscal = fiscal_language(state)?;
    let db = state.db.lock().map_err(errors::internal)?;
    Ok(MenuSaved {
        id,
        menu: read_menu(&db, &fiscal)?,
    })
}

/// Create (no `id`) or update a category. Manager only; refused while a source manages the menu.
pub fn do_menu_save_category(
    state: &AppState,
    input: SaveCategoryInput,
) -> Result<MenuSaved, String> {
    let user = access::operator(state, Access::Admin)?;
    let fiscal = fiscal_language(state)?;
    let names = clean_names(&input.names, &fiscal)?;
    let after = AdminCategory {
        id: input.id.unwrap_or(0),
        names,
        sort_order: clean_sort(input.sort_order)?,
        color: clean_color(input.color.as_deref())?,
        active: input.active,
    };
    let id = {
        let db = state.db.lock().map_err(errors::internal)?;
        refuse_if_managed(&db)?;
        let before = match input.id {
            Some(id) => Some(
                q::get_category(&db, id)
                    .map_err(errors::internal)?
                    .ok_or_else(|| not_found("category", id))?,
            ),
            None => None,
        };
        commit(
            &db,
            q::CATEGORY,
            before.as_ref(),
            &after,
            &user.insz,
            |tx| q::write_category(tx, &after),
            input.id.unwrap_or(0),
        )?
    };
    saved(state, id)
}

/// Create (no `id`) or update a product. Manager only; refused while a source manages the menu.
pub fn do_menu_save_product(
    state: &AppState,
    input: SaveProductInput,
) -> Result<MenuSaved, String> {
    let user = access::operator(state, Access::Admin)?;
    let fiscal = fiscal_language(state)?;
    let names = clean_names(&input.names, &fiscal)?;
    let after = AdminProduct {
        id: input.id.unwrap_or(0),
        category_id: input.category_id,
        names,
        price_cents: clean_price("price_cents", input.price_cents)?,
        vat_code: clean_vat("vat_code", &input.vat_code)?,
        vat_code_takeaway: input
            .vat_code_takeaway
            .as_deref()
            .map(|c| clean_vat("vat_code_takeaway", c))
            .transpose()?,
        kitchen_printer: input.kitchen_printer,
        code: clean_code(input.code.as_deref())?,
        open_price: input.open_price,
        sort_order: clean_sort(input.sort_order)?,
        active: input.active,
    };
    let id = {
        let db = state.db.lock().map_err(errors::internal)?;
        refuse_if_managed(&db)?;
        if q::get_category(&db, input.category_id)
            .map_err(errors::internal)?
            .is_none()
        {
            return Err(not_found("category", input.category_id));
        }
        let before = match input.id {
            Some(id) => Some(
                q::get_product(&db, id)
                    .map_err(errors::internal)?
                    .ok_or_else(|| not_found("product", id))?,
            ),
            None => None,
        };
        commit(
            &db,
            q::PRODUCT,
            before.as_ref(),
            &after,
            &user.insz,
            |tx| q::write_product(tx, &after),
            input.id.unwrap_or(0),
        )?
    };
    saved(state, id)
}

/// Create (no `id`) or update an option group of a product. Manager only; refused while a
/// source manages the menu.
pub fn do_menu_save_modifier_group(
    state: &AppState,
    input: SaveModifierGroupInput,
) -> Result<MenuSaved, String> {
    let user = access::operator(state, Access::Admin)?;
    let fiscal = fiscal_language(state)?;
    let names = clean_names(&input.names, &fiscal)?;
    if input.min_select < 0 {
        return Err(invalid("min_select"));
    }
    if input.max_select < 1 {
        return Err(invalid("max_select"));
    }
    if input.min_select > input.max_select {
        return Err(invalid("min_select"));
    }
    let after = AdminGroup {
        id: input.id.unwrap_or(0),
        menu_item_id: input.menu_item_id,
        names,
        min_select: input.min_select,
        max_select: input.max_select,
        sort_order: clean_sort(input.sort_order)?,
        active: input.active,
    };
    let id = {
        let db = state.db.lock().map_err(errors::internal)?;
        refuse_if_managed(&db)?;
        if q::get_product(&db, input.menu_item_id)
            .map_err(errors::internal)?
            .is_none()
        {
            return Err(not_found("product", input.menu_item_id));
        }
        let before = match input.id {
            Some(id) => Some(
                q::get_group(&db, id)
                    .map_err(errors::internal)?
                    .ok_or_else(|| not_found("modifier_group", id))?,
            ),
            None => None,
        };
        commit(
            &db,
            q::GROUP,
            before.as_ref(),
            &after,
            &user.insz,
            |tx| q::write_group(tx, &after),
            input.id.unwrap_or(0),
        )?
    };
    saved(state, id)
}

/// Create (no `id`) or update an option of a group. Manager only; refused while a source
/// manages the menu.
pub fn do_menu_save_modifier(
    state: &AppState,
    input: SaveModifierInput,
) -> Result<MenuSaved, String> {
    let user = access::operator(state, Access::Admin)?;
    let fiscal = fiscal_language(state)?;
    let names = clean_names(&input.names, &fiscal)?;
    let after = AdminOption {
        id: input.id.unwrap_or(0),
        group_id: input.group_id,
        names,
        price_cents: clean_price("price_cents", input.price_cents)?,
        sort_order: clean_sort(input.sort_order)?,
        active: input.active,
    };
    let id = {
        let db = state.db.lock().map_err(errors::internal)?;
        refuse_if_managed(&db)?;
        if q::get_group(&db, input.group_id)
            .map_err(errors::internal)?
            .is_none()
        {
            return Err(not_found("modifier_group", input.group_id));
        }
        let before = match input.id {
            Some(id) => Some(
                q::get_option(&db, id)
                    .map_err(errors::internal)?
                    .ok_or_else(|| not_found("modifier", id))?,
            ),
            None => None,
        };
        commit(
            &db,
            q::OPTION,
            before.as_ref(),
            &after,
            &user.insz,
            |tx| q::write_option(tx, &after),
            input.id.unwrap_or(0),
        )?
    };
    saved(state, id)
}

/// The fields whose value differs between two snapshots (every field of a creation).
fn changed_fields(before: Option<&str>, after: &str) -> Vec<String> {
    let after: serde_json::Value = serde_json::from_str(after).unwrap_or_default();
    let before: Option<serde_json::Value> = before.and_then(|b| serde_json::from_str(b).ok());
    let Some(fields) = after.as_object() else {
        return Vec::new();
    };
    fields
        .iter()
        .filter(|(k, _)| k.as_str() != "id")
        .filter(|(k, v)| {
            before
                .as_ref()
                .is_none_or(|b| b.get(k.as_str()) != Some(*v))
        })
        .map(|(k, _)| k.clone())
        .collect()
}

/// The name an entry had after a change.
fn label_of(after: &str, fiscal: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(after).unwrap_or_default();
    let names = value.get("names").and_then(|n| n.as_object());
    names
        .and_then(|n| {
            n.get(fiscal)
                .or_else(|| n.get("fr"))
                .or_else(|| n.values().next())
        })
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn present(db: &rusqlite::Connection, row: ChangeRow, fiscal: &str) -> MenuChange {
    let changed_by_name = pos_db::queries::users::get_user(db, &row.changed_by)
        .ok()
        .flatten()
        .map_or_else(|| row.changed_by.clone(), |u| u.name);
    MenuChange {
        label: label_of(&row.after_json, fiscal),
        changed_fields: changed_fields(row.before_json.as_deref(), &row.after_json),
        id: row.id,
        entity_type: row.entity_type,
        entity_id: row.entity_id,
        action: row.action,
        before_json: row.before_json,
        after_json: row.after_json,
        changed_by: row.changed_by,
        changed_by_name,
        changed_at: row.changed_at,
    }
}

/// The change log, newest first (at most `limit`, 1 to 500; 100 when none). Manager only.
pub fn do_list_menu_changes(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<MenuChange>, String> {
    access::operator(state, Access::Admin)?;
    let fiscal = fiscal_language(state)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let rows =
        q::list_changes(&db, limit.unwrap_or(100).clamp(1, 500)).map_err(errors::internal)?;
    Ok(rows.into_iter().map(|r| present(&db, r, &fiscal)).collect())
}
