//! Applying a catalog to a real POS database.

use pos_catalog::testing::{catalog, category, choice, group, item, names};
use pos_catalog::{
    ApplyOutcome, IssueCode, ItemKind, adopt_and_apply, adoption_preview, apply, managed_by,
};
use pos_core::vat::VatCode;
use pos_db::queries::menu::{get_categories, get_menu_items, resolve_catalogue_line};
use rusqlite::{Connection, params};

const SOURCE: &str = "hubrise";

fn db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    pos_db::migrate::run_migrations(&conn).unwrap();
    conn
}

fn id_of(conn: &Connection, key: &str) -> i64 {
    conn.query_row(
        "SELECT id FROM menu_items WHERE external_id = ?1",
        params![key],
        |r| r.get(0),
    )
    .unwrap()
}

fn active(conn: &Connection, key: &str) -> bool {
    conn.query_row(
        "SELECT active FROM menu_items WHERE external_id = ?1",
        params![key],
        |r| r.get(0),
    )
    .unwrap()
}

fn applied(outcome: ApplyOutcome) -> (usize, usize) {
    match outcome {
        ApplyOutcome::Applied {
            active_items,
            deactivated_items,
            ..
        } => (active_items, deactivated_items),
        other => panic!("not applied: {other:?}"),
    }
}

#[test]
fn a_catalog_becomes_the_menu() {
    let mut conn = db();
    assert_eq!(
        applied(apply(&mut conn, SOURCE, &catalog()).unwrap()),
        (3, 0)
    );
    assert_eq!(managed_by(&conn).unwrap().as_deref(), Some(SOURCE));

    let maki = id_of(&conn, "MAKI-6");
    let eat_in = resolve_catalogue_line(&conn, maki, false, "fr").unwrap();
    let takeaway = resolve_catalogue_line(&conn, maki, true, "fr").unwrap();
    assert_eq!(
        (eat_in.vat_code, takeaway.vat_code),
        (VatCode::B, VatCode::C)
    );
    assert_eq!(eat_in.price_cents, 650);
    assert_eq!(eat_in.name, "MAKI-6");

    let coca = id_of(&conn, "COCA");
    assert_eq!(
        resolve_catalogue_line(&conn, coca, true, "fr")
            .unwrap()
            .vat_code,
        VatCode::A
    );
}

#[test]
fn charges_stay_off_the_grid_but_resolve_on_an_order_line() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let grid: Vec<String> = get_menu_items(&conn, None, "fr")
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert_eq!(grid, vec!["MAKI-6", "COCA"]);
    let tabs: Vec<String> = get_categories(&conn, "fr")
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(tabs, vec!["SUSHI", "DRINKS"]);

    let fee = id_of(&conn, "charge:DELIVERY");
    assert_eq!(
        resolve_catalogue_line(&conn, fee, true, "fr")
            .unwrap()
            .price_cents,
        250
    );
}

#[test]
fn an_item_sold_online_only_stays_off_the_grid() {
    let mut conn = db();
    let mut c = catalog();
    c.items[1].on_pos = false;
    apply(&mut conn, SOURCE, &c).unwrap();
    let grid = get_menu_items(&conn, None, "fr").unwrap();
    assert_eq!(grid.len(), 1);
    assert!(resolve_catalogue_line(&conn, id_of(&conn, "COCA"), false, "fr").is_ok());
}

#[test]
fn applying_twice_keeps_the_ids_and_writes_nothing() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let before = id_of(&conn, "MAKI-6");
    assert_eq!(
        apply(&mut conn, SOURCE, &catalog()).unwrap(),
        ApplyOutcome::Unchanged
    );
    assert_eq!(id_of(&conn, "MAKI-6"), before);
}

#[test]
fn a_rename_or_price_change_keeps_the_id() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let before = id_of(&conn, "MAKI-6");
    let mut c = catalog();
    c.items[0].names = names("Maki saumon");
    c.items[0].price_cents = 700;
    applied(apply(&mut conn, SOURCE, &c).unwrap());
    assert_eq!(id_of(&conn, "MAKI-6"), before);
    let line = resolve_catalogue_line(&conn, before, false, "fr").unwrap();
    assert_eq!((line.name.as_str(), line.price_cents), ("Maki saumon", 700));
}

#[test]
fn a_removed_item_is_deactivated_and_comes_back_with_its_id() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let coca = id_of(&conn, "COCA");

    let mut without = catalog();
    without.items.retain(|i| i.key != "COCA");
    assert_eq!(applied(apply(&mut conn, SOURCE, &without).unwrap()), (2, 1));
    assert!(!active(&conn, "COCA"));
    assert!(resolve_catalogue_line(&conn, coca, false, "fr").is_err());

    applied(apply(&mut conn, SOURCE, &catalog()).unwrap());
    assert_eq!(id_of(&conn, "COCA"), coca);
    assert!(active(&conn, "COCA"));
}

#[test]
fn an_unavailable_item_cannot_be_sold() {
    let mut conn = db();
    let mut c = catalog();
    c.items[1].available = false;
    apply(&mut conn, SOURCE, &c).unwrap();
    assert!(!active(&conn, "COCA"));
}

#[test]
fn option_groups_are_stored_per_item_with_their_limits() {
    let mut conn = db();
    let mut c = catalog();
    // The same list attached to a second item: its own group.
    let mut big = item("MAKI-12", "SUSHI", ItemKind::Product, 1200);
    big.groups = vec![group(
        "MAKI-12|SAUCE",
        1,
        None,
        vec![choice("MAKI-12|SAUCE|SOJA", 0)],
    )];
    c.items.push(big);
    apply(&mut conn, SOURCE, &c).unwrap();

    let groups: Vec<(String, i64, i64, i64)> = conn
        .prepare(
            "SELECT g.external_id, g.min_select, g.max_select, g.menu_item_id
             FROM modifier_groups g WHERE g.active = 1 ORDER BY g.external_id",
        )
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        groups,
        vec![
            ("MAKI-12|SAUCE".into(), 1, -1, id_of(&conn, "MAKI-12")),
            ("MAKI-6|SAUCE".into(), 0, 1, id_of(&conn, "MAKI-6")),
        ]
    );
    let spicy: i64 = conn
        .query_row(
            "SELECT price_cents FROM modifiers WHERE external_id = 'MAKI-6|SAUCE|SPICY'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(spicy, 50);
}

#[test]
fn a_removed_option_is_deactivated() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let mut c = catalog();
    c.items[0].groups[0].options.pop();
    apply(&mut conn, SOURCE, &c).unwrap();
    let spicy: bool = conn
        .query_row(
            "SELECT active FROM modifiers WHERE external_id = 'MAKI-6|SAUCE|SPICY'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!spicy);
}

#[test]
fn an_invalid_catalog_leaves_the_menu_untouched() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let mut bad = catalog();
    bad.items[0].price_cents = 1;
    bad.items[1].category_key = "nowhere".into();

    let ApplyOutcome::Rejected(issues) = apply(&mut conn, SOURCE, &bad).unwrap() else {
        panic!("applied an invalid catalog");
    };
    assert_eq!(issues[0].code, IssueCode::UnknownCategory);
    let maki = id_of(&conn, "MAKI-6");
    assert_eq!(
        resolve_catalogue_line(&conn, maki, false, "fr")
            .unwrap()
            .price_cents,
        650
    );
    let state = pos_catalog::state(&conn, SOURCE).unwrap().unwrap();
    assert!(state.rejected_at.is_some());
    assert_eq!(state.issues, issues);

    // The good catalog again clears the rejection without rewriting.
    assert_eq!(
        apply(&mut conn, SOURCE, &catalog()).unwrap(),
        ApplyOutcome::Unchanged
    );
    let state = pos_catalog::state(&conn, SOURCE).unwrap().unwrap();
    assert!(state.rejected_at.is_none() && state.issues.is_empty());
}

#[test]
fn a_released_source_no_longer_manages_the_menu() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    pos_catalog::release(&conn, SOURCE).unwrap();
    assert_eq!(managed_by(&conn).unwrap(), None);
    // Connecting again rewrites the menu (no stale hash).
    applied(apply(&mut conn, SOURCE, &catalog()).unwrap());
}

/// A menu entered by hand before the first connect.
fn local_item(conn: &Connection, name: &str, price: i64) -> i64 {
    conn.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order) VALUES (900, 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO menu_items (category_id, price_cents, vat_code) VALUES (900, ?1, 'B')",
        params![price],
    )
    .unwrap();
    let id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO translations (entity_type, entity_id, field, lang, value)
         VALUES ('menu_item', ?1, 'name', 'fr', ?2)",
        params![id, name],
    )
    .unwrap();
    id
}

#[test]
fn a_menu_entered_by_hand_waits_for_the_manager() {
    let mut conn = db();
    let maki = local_item(&conn, "MAKI-6", 650);
    let other = local_item(&conn, "Gyoza", 550);
    let wrong_price = local_item(&conn, "COCA", 280);

    assert_eq!(
        apply(&mut conn, SOURCE, &catalog()).unwrap(),
        ApplyOutcome::AwaitingAdoption
    );
    assert_eq!(managed_by(&conn).unwrap(), None, "not locked yet");
    let grid = get_menu_items(&conn, None, "fr").unwrap();
    assert_eq!(grid.len(), 3, "the hand-entered menu is untouched");

    let pending = pos_catalog::pending_catalog(&conn, SOURCE)
        .unwrap()
        .unwrap();
    let preview = adoption_preview(&conn, &pending).unwrap();
    assert_eq!(preview.len(), 1);
    assert_eq!(
        (preview[0].menu_item_id, preview[0].key.as_str()),
        (maki, "MAKI-6")
    );

    let outcome = adopt_and_apply(&mut conn, SOURCE, &preview)
        .unwrap()
        .unwrap();
    assert_eq!(applied(outcome), (3, 2));
    assert_eq!(
        id_of(&conn, "MAKI-6"),
        maki,
        "the adopted product keeps its id"
    );
    assert_eq!(managed_by(&conn).unwrap().as_deref(), Some(SOURCE));
    for id in [other, wrong_price] {
        let on_sale: bool = conn
            .query_row(
                "SELECT active FROM menu_items WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!on_sale, "the rest is deactivated");
    }
    assert!(
        pos_catalog::pending_catalog(&conn, SOURCE)
            .unwrap()
            .is_none()
    );
}

#[test]
fn adopting_nothing_still_applies() {
    let mut conn = db();
    local_item(&conn, "Gyoza", 550);
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    applied(adopt_and_apply(&mut conn, SOURCE, &[]).unwrap().unwrap());
}

#[test]
fn nothing_to_adopt_without_a_pending_catalog() {
    let mut conn = db();
    assert_eq!(adopt_and_apply(&mut conn, SOURCE, &[]).unwrap(), None);
}

#[test]
fn an_ambiguous_match_is_not_offered() {
    let conn = db();
    local_item(&conn, "MAKI-6", 650);
    local_item(&conn, "MAKI-6", 650);
    assert!(adoption_preview(&conn, &catalog()).unwrap().is_empty());
}

#[test]
fn categories_keep_their_ids_too() {
    let mut conn = db();
    apply(&mut conn, SOURCE, &catalog()).unwrap();
    let id = |conn: &Connection| -> i64 {
        conn.query_row(
            "SELECT id FROM categories WHERE external_id = 'DRINKS'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    let before = id(&conn);
    let mut c = catalog();
    c.categories.insert(0, category("NEW"));
    c.items[0].category_key = "NEW".into();
    applied(apply(&mut conn, SOURCE, &c).unwrap());
    assert_eq!(id(&conn), before);
}
