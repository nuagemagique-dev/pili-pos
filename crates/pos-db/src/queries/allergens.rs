//! Allergen data (migration 61, V12-D17): the list of a product or an
//! option, its declaration and the append-only change log.
//!
//! An entity is *declared* when `allergen_declarations` has a row for
//! it; its list is then complete, possibly empty ("no allergen"). An
//! entity without a declaration has no information. Non-fiscal data:
//! nothing here reaches the FDM or the signed payload.

use std::collections::HashMap;

use pos_core::allergen::AllergenSet;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

/// What an allergen list belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entity {
    MenuItem,
    Modifier,
}

impl Entity {
    pub fn as_str(self) -> &'static str {
        match self {
            Entity::MenuItem => "menu_item",
            Entity::Modifier => "modifier",
        }
    }

    fn table(self) -> &'static str {
        match self {
            Entity::MenuItem => "menu_item_allergens",
            Entity::Modifier => "modifier_allergens",
        }
    }

    fn id_column(self) -> &'static str {
        match self {
            Entity::MenuItem => "menu_item_id",
            Entity::Modifier => "modifier_id",
        }
    }
}

/// The declared list of `id` and where it came from (`catalog` or
/// `local`); `None` when nothing is declared.
pub fn get_declared(
    conn: &Connection,
    entity: Entity,
    id: i64,
) -> Result<Option<(AllergenSet, String)>, rusqlite::Error> {
    let source: Option<String> = conn
        .query_row(
            "SELECT source FROM allergen_declarations WHERE entity_type = ?1 AND entity_id = ?2",
            params![entity.as_str(), id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(source) = source else {
        return Ok(None);
    };
    Ok(Some((read_set(conn, entity, id)?, source)))
}

/// The declared list of `id`, `None` when it has no information.
pub fn get(
    conn: &Connection,
    entity: Entity,
    id: i64,
) -> Result<Option<AllergenSet>, rusqlite::Error> {
    Ok(get_declared(conn, entity, id)?.map(|(set, _)| set))
}

fn read_set(conn: &Connection, entity: Entity, id: i64) -> Result<AllergenSet, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!(
        "SELECT allergen, level FROM {} WHERE {} = ?1",
        entity.table(),
        entity.id_column()
    ))?;
    let rows: Vec<(String, String)> = stmt
        .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(set_of(rows))
}

fn set_of(rows: Vec<(String, String)>) -> AllergenSet {
    let mut set = AllergenSet::default();
    for (code, level) in rows {
        if level == "contains" {
            set.contains.push(code);
        } else {
            set.may_contain.push(code);
        }
    }
    set.normalized()
}

/// Every declared list of `entity`, by id (one query for the menu and
/// the option picker).
pub fn load_all(
    conn: &Connection,
    entity: Entity,
) -> Result<HashMap<i64, AllergenSet>, rusqlite::Error> {
    let mut rows: HashMap<i64, Vec<(String, String)>> = HashMap::new();
    {
        let mut stmt = conn.prepare(&format!(
            "SELECT {id}, allergen, level FROM {table}",
            id = entity.id_column(),
            table = entity.table()
        ))?;
        for row in stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })? {
            let (id, code, level) = row?;
            rows.entry(id).or_default().push((code, level));
        }
    }
    let mut out = HashMap::new();
    let mut stmt =
        conn.prepare("SELECT entity_id FROM allergen_declarations WHERE entity_type = ?1")?;
    for id in stmt.query_map(params![entity.as_str()], |r| r.get::<_, i64>(0))? {
        let id = id?;
        out.insert(id, set_of(rows.remove(&id).unwrap_or_default()));
    }
    Ok(out)
}

fn state_json(set: Option<&AllergenSet>) -> String {
    match set {
        Some(set) => serde_json::json!({
            "declared": true,
            "contains": set.contains,
            "may_contain": set.may_contain,
        })
        .to_string(),
        None => r#"{"declared":false}"#.to_owned(),
    }
}

/// Replace the declaration of `id` by `set` (`None`: back to "no
/// information"). Writes the rows, the declaration and one log row,
/// and only when the list really changes; it relies on the caller's
/// transaction for atomicity (it opens none). Returns whether anything changed. The codes must be
/// valid (callers check them with `AllergenSet::first_unknown`).
pub fn write(
    conn: &Connection,
    entity: Entity,
    id: i64,
    set: Option<&AllergenSet>,
    source: &str,
    changed_by: Option<&str>,
) -> Result<bool, rusqlite::Error> {
    let before = get_declared(conn, entity, id)?;
    let before_set = before.as_ref().map(|(s, _)| s);
    let after = set.map(AllergenSet::normalized);
    let before_json = state_json(before_set);
    let after_json = state_json(after.as_ref());
    if before_json == after_json {
        // Same list: only the provenance may move, without a log row.
        if let (Some((_, old)), Some(_)) = (&before, &after)
            && old != source
        {
            conn.execute(
                "UPDATE allergen_declarations SET source = ?3
                 WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity.as_str(), id, source],
            )?;
        }
        return Ok(false);
    }
    conn.execute(
        &format!(
            "DELETE FROM {} WHERE {} = ?1",
            entity.table(),
            entity.id_column()
        ),
        params![id],
    )?;
    match &after {
        Some(after) => {
            for (codes, level) in [
                (&after.contains, "contains"),
                (&after.may_contain, "may_contain"),
            ] {
                for code in codes {
                    conn.execute(
                        &format!(
                            "INSERT INTO {} ({}, allergen, level) VALUES (?1, ?2, ?3)",
                            entity.table(),
                            entity.id_column()
                        ),
                        params![id, code, level],
                    )?;
                }
            }
            conn.execute(
                "INSERT INTO allergen_declarations
                    (entity_type, entity_id, source, declared_at, declared_by)
                 VALUES (?1, ?2, ?3, datetime('now'), ?4)
                 ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                    source = excluded.source, declared_at = excluded.declared_at,
                    declared_by = excluded.declared_by",
                params![entity.as_str(), id, source, changed_by],
            )?;
        }
        None => {
            conn.execute(
                "DELETE FROM allergen_declarations WHERE entity_type = ?1 AND entity_id = ?2",
                params![entity.as_str(), id],
            )?;
        }
    }
    conn.execute(
        "INSERT INTO allergen_change_log
            (entity_type, entity_id, before_json, after_json, source, changed_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            entity.as_str(),
            id,
            before_json,
            after_json,
            source,
            changed_by
        ],
    )?;
    Ok(true)
}

/// A catalog sync's statement about `id`. `None` (the catalog says
/// nothing) clears what an earlier catalog declared but never
/// overwrites what a manager entered locally: no information from the
/// source is not a statement that the product is free of allergens.
pub fn write_from_catalog(
    conn: &Connection,
    entity: Entity,
    id: i64,
    set: Option<&AllergenSet>,
) -> Result<bool, rusqlite::Error> {
    if set.is_none()
        && let Some((_, source)) = get_declared(conn, entity, id)?
        && source == "local"
    {
        return Ok(false);
    }
    write(conn, entity, id, set, "catalog", None)
}

/// One row of the change log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Change {
    pub entity_type: String,
    pub entity_id: i64,
    pub before_json: String,
    pub after_json: String,
    pub source: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

/// The change log of one entity, oldest first.
pub fn history(conn: &Connection, entity: Entity, id: i64) -> Result<Vec<Change>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT entity_type, entity_id, before_json, after_json, source, changed_by, changed_at
         FROM allergen_change_log WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY id",
    )?;
    stmt.query_map(params![entity.as_str(), id], |r| {
        Ok(Change {
            entity_type: r.get(0)?,
            entity_id: r.get(1)?,
            before_json: r.get(2)?,
            after_json: r.get(3)?,
            source: r.get(4)?,
            changed_by: r.get(5)?,
            changed_at: r.get(6)?,
        })
    })?
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::migrate::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order) VALUES (1, 0);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code) VALUES (1, 1, 100, 'B');",
        )
        .unwrap();
        conn
    }

    fn set(contains: &[&str], may: &[&str]) -> AllergenSet {
        AllergenSet {
            contains: contains.iter().map(|s| (*s).to_owned()).collect(),
            may_contain: may.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn nothing_is_declared_until_written() {
        let conn = db();
        assert_eq!(get(&conn, Entity::MenuItem, 1).unwrap(), None);
        assert!(load_all(&conn, Entity::MenuItem).unwrap().is_empty());
    }

    #[test]
    fn an_empty_declaration_is_not_the_same_as_none() {
        let conn = db();
        assert!(
            write(
                &conn,
                Entity::MenuItem,
                1,
                Some(&set(&[], &[])),
                "local",
                Some("1")
            )
            .unwrap()
        );
        assert_eq!(
            get(&conn, Entity::MenuItem, 1).unwrap(),
            Some(AllergenSet::default())
        );
        let all = load_all(&conn, Entity::MenuItem).unwrap();
        assert_eq!(all.get(&1), Some(&AllergenSet::default()));
    }

    #[test]
    fn a_list_is_stored_sorted_and_logged_once() {
        let conn = db();
        let s = set(&["milk", "gluten"], &["sesame"]);
        assert!(write(&conn, Entity::MenuItem, 1, Some(&s), "catalog", None).unwrap());
        assert!(!write(&conn, Entity::MenuItem, 1, Some(&s), "catalog", None).unwrap());
        let got = get(&conn, Entity::MenuItem, 1).unwrap().unwrap();
        assert_eq!(got.contains, vec!["gluten", "milk"]);
        assert_eq!(got.may_contain, vec!["sesame"]);
        assert_eq!(history(&conn, Entity::MenuItem, 1).unwrap().len(), 1);
    }

    #[test]
    fn a_change_logs_before_and_after() {
        let conn = db();
        write(
            &conn,
            Entity::MenuItem,
            1,
            Some(&set(&["milk"], &[])),
            "catalog",
            None,
        )
        .unwrap();
        write(
            &conn,
            Entity::MenuItem,
            1,
            Some(&set(&["milk", "eggs"], &[])),
            "local",
            Some("42"),
        )
        .unwrap();
        let log = history(&conn, Entity::MenuItem, 1).unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[0].before_json.contains("\"declared\":false"));
        assert!(log[1].before_json.contains("\"milk\""));
        assert!(!log[1].before_json.contains("eggs"));
        assert!(log[1].after_json.contains("eggs"));
        assert_eq!(log[1].source, "local");
        assert_eq!(log[1].changed_by.as_deref(), Some("42"));
    }

    #[test]
    fn a_catalog_without_information_clears_a_catalog_declaration_only() {
        let conn = db();
        write_from_catalog(&conn, Entity::MenuItem, 1, Some(&set(&["fish"], &[]))).unwrap();
        assert!(write_from_catalog(&conn, Entity::MenuItem, 1, None).unwrap());
        assert_eq!(get(&conn, Entity::MenuItem, 1).unwrap(), None);
        write(
            &conn,
            Entity::MenuItem,
            1,
            Some(&set(&["fish"], &[])),
            "local",
            Some("1"),
        )
        .unwrap();
        assert!(!write_from_catalog(&conn, Entity::MenuItem, 1, None).unwrap());
        assert!(get(&conn, Entity::MenuItem, 1).unwrap().is_some());
    }
}
