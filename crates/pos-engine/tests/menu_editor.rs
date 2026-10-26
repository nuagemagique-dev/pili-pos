//! The on-screen menu editor (V11-D17, V11-D18): create and update of the four entities, the
//! validation codes, the append-only log, the lock while a source manages the menu, the manager
//! role, and AM art. 44 (an edit never changes an open order or a signed event).

#![cfg(feature = "hubrise")]
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::menu::{do_get_categories, do_get_menu_items};
use pos_engine::commands::menu_admin::{
    SaveCategoryInput, SaveModifierGroupInput, SaveModifierInput, SaveProductInput,
    do_list_menu_changes, do_menu_admin_get, do_menu_save_category, do_menu_save_modifier,
    do_menu_save_modifier_group, do_menu_save_product,
};
use pos_engine::commands::modifiers::{AddWithModifiersInput, do_add_order_line_with_modifiers};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};

const MANAGER: &str = "12345678958";
const CASHIER: &str = "75061189702";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, UserRole::Admin);
    app
}

fn names(fr: &str) -> std::collections::BTreeMap<String, String> {
    [("fr".to_string(), fr.to_string())].into()
}

fn category(id: Option<i64>, fr: &str) -> SaveCategoryInput {
    SaveCategoryInput {
        id,
        names: names(fr),
        sort_order: 1,
        color: Some("#c0392b".into()),
        active: true,
    }
}

fn product(id: Option<i64>, category_id: i64, fr: &str, price: i64) -> SaveProductInput {
    SaveProductInput {
        id,
        category_id,
        names: names(fr),
        price_cents: price,
        vat_code: "B".into(),
        vat_code_takeaway: Some("C".into()),
        kitchen_printer: true,
        code: Some("A1".into()),
        open_price: false,
        sort_order: 1,
        active: true,
    }
}

fn group(id: Option<i64>, product: i64, fr: &str) -> SaveModifierGroupInput {
    SaveModifierGroupInput {
        id,
        menu_item_id: product,
        names: names(fr),
        min_select: 0,
        max_select: 2,
        sort_order: 1,
        active: true,
    }
}

fn option(id: Option<i64>, group: i64, fr: &str, price: i64) -> SaveModifierInput {
    SaveModifierInput {
        id,
        group_id: group,
        names: names(fr),
        price_cents: price,
        sort_order: 1,
        active: true,
    }
}

fn invalid(field: &str) -> String {
    errors::coded(errors::MENU_INVALID, &[("field", &field)])
}

fn log_count(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM menu_change_log", [], |r| r.get(0))
        .unwrap()
}

#[tokio::test]
async fn the_four_entities_are_created_and_updated() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi")).unwrap();
    assert!(cat.id > 0);
    assert_eq!(cat.menu.categories.len(), 1);
    assert_eq!(cat.menu.categories[0].names["fr"], "Sushi");
    assert_eq!(cat.menu.categories[0].color.as_deref(), Some("#C0392B"));

    let prod = do_menu_save_product(&app.state, product(None, cat.id, "Maki saumon", 650)).unwrap();
    let p = &prod.menu.products[0];
    assert_eq!(
        (p.id, p.price_cents, p.vat_code.as_str()),
        (prod.id, 650, "B")
    );
    assert_eq!(p.vat_code_takeaway.as_deref(), Some("C"));
    assert_eq!(p.code.as_deref(), Some("A1"));

    let grp = do_menu_save_modifier_group(&app.state, group(None, prod.id, "Sauce")).unwrap();
    assert_eq!(grp.menu.groups[0].max_select, 2);
    let opt = do_menu_save_modifier(&app.state, option(None, grp.id, "Soja", 50)).unwrap();
    assert_eq!(opt.menu.options[0].price_cents, 50);

    // Updates keep the id and replace the values.
    let mut upd = product(Some(prod.id), cat.id, "Maki saumon x8", 720);
    upd.vat_code = "A".into();
    upd.names.insert("zh".into(), "三文鱼卷".into());
    let after = do_menu_save_product(&app.state, upd).unwrap();
    assert_eq!(after.menu.products.len(), 1);
    let p = &after.menu.products[0];
    assert_eq!((p.price_cents, p.vat_code.as_str()), (720, "A"));
    assert_eq!(p.names["fr"], "Maki saumon x8");
    assert_eq!(p.names["zh"], "三文鱼卷");
    // A name left out is removed.
    let again =
        do_menu_save_product(&app.state, product(Some(prod.id), cat.id, "Maki", 720)).unwrap();
    assert!(!again.menu.products[0].names.contains_key("zh"));

    let mut c = category(Some(cat.id), "Sushis");
    c.active = false;
    let c = do_menu_save_category(&app.state, c).unwrap();
    assert!(!c.menu.categories[0].active);
    assert_eq!(c.menu.categories[0].names["fr"], "Sushis");
    do_menu_save_modifier_group(&app.state, group(Some(grp.id), prod.id, "Sauces")).unwrap();
    let o =
        do_menu_save_modifier(&app.state, option(Some(opt.id), grp.id, "Soja sucrée", 60)).unwrap();
    assert_eq!(o.menu.options[0].names["fr"], "Soja sucrée");
    assert_eq!(o.menu.groups[0].names["fr"], "Sauces");
    // Nothing was deleted along the way.
    assert_eq!(o.menu.options.len(), 1);
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "one scenario read top to bottom, clearer unsplit"
)]
async fn validation_codes_name_the_field() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;

    // A name in the establishment's language is required (fr by default).
    let mut c = category(None, "  ");
    assert_eq!(
        do_menu_save_category(&app.state, c.clone()).unwrap_err(),
        invalid("name")
    );
    c.names = [("en".to_string(), "Sushi".to_string())].into();
    assert_eq!(
        do_menu_save_category(&app.state, c.clone()).unwrap_err(),
        invalid("name")
    );
    c.names = [
        ("fr".to_string(), "x".to_string()),
        ("xx".to_string(), "y".to_string()),
    ]
    .into();
    assert_eq!(
        do_menu_save_category(&app.state, c).unwrap_err(),
        invalid("names")
    );
    assert_eq!(
        do_menu_save_category(&app.state, category(None, &"n".repeat(61))).unwrap_err(),
        invalid("name")
    );
    assert_eq!(
        do_menu_save_category(&app.state, category(None, "a\nb")).unwrap_err(),
        invalid("name")
    );
    let mut c = category(None, "Boissons");
    c.color = Some("red".into());
    assert_eq!(
        do_menu_save_category(&app.state, c).unwrap_err(),
        invalid("color")
    );
    let mut c = category(None, "Boissons");
    c.sort_order = -1;
    assert_eq!(
        do_menu_save_category(&app.state, c).unwrap_err(),
        invalid("sort_order")
    );

    for (price, ok) in [(-1, false), (0, true), (999_999, true), (1_000_000, false)] {
        let r = do_menu_save_product(&app.state, product(None, cat, "Thé", price));
        assert_eq!(r.is_ok(), ok, "price {price}");
        if !ok {
            assert_eq!(r.unwrap_err(), invalid("price_cents"));
        }
    }
    for code in ["A", "B", "C", "D", "X"] {
        let mut p = product(None, cat, "Thé", 100);
        p.vat_code = code.into();
        assert!(do_menu_save_product(&app.state, p).is_ok(), "{code}");
    }
    for bad in ["E", "", "a", "AB"] {
        let mut p = product(None, cat, "Thé", 100);
        p.vat_code = bad.into();
        assert_eq!(
            do_menu_save_product(&app.state, p).unwrap_err(),
            invalid("vat_code")
        );
    }
    let mut p = product(None, cat, "Thé", 100);
    p.vat_code_takeaway = Some("Z".into());
    assert_eq!(
        do_menu_save_product(&app.state, p).unwrap_err(),
        invalid("vat_code_takeaway")
    );
    let mut p = product(None, cat, "Thé", 100);
    p.code = Some("x".repeat(13));
    assert_eq!(
        do_menu_save_product(&app.state, p).unwrap_err(),
        invalid("code")
    );
    // An empty code or takeaway label is stored as none.
    let mut p = product(None, cat, "Thé vert", 100);
    p.code = Some("  ".into());
    p.vat_code_takeaway = None;
    let saved = do_menu_save_product(&app.state, p).unwrap();
    let p = saved
        .menu
        .products
        .iter()
        .find(|p| p.id == saved.id)
        .unwrap();
    assert_eq!((p.code.clone(), p.vat_code_takeaway.clone()), (None, None));

    let prod = saved.id;
    for (min, max, field) in [
        (0, 0, "max_select"),
        (-1, 2, "min_select"),
        (3, 2, "min_select"),
    ] {
        let mut g = group(None, prod, "Sauce");
        g.min_select = min;
        g.max_select = max;
        assert_eq!(
            do_menu_save_modifier_group(&app.state, g).unwrap_err(),
            invalid(field)
        );
    }
    let mut g = group(None, prod, "Sauce");
    g.min_select = 2;
    g.max_select = 2;
    let g = do_menu_save_modifier_group(&app.state, g).unwrap().id;
    assert_eq!(
        do_menu_save_modifier(&app.state, option(None, g, "Soja", -5)).unwrap_err(),
        invalid("price_cents")
    );
    assert!(do_menu_save_modifier(&app.state, option(None, g, "Soja", 0)).is_ok());
}

#[tokio::test]
async fn a_german_till_requires_german_and_offers_it() {
    let app = app().await;
    app.state.config.lock().unwrap().language = "de".into();
    let menu = do_menu_admin_get(&app.state).unwrap();
    assert_eq!(menu.languages[0], "de");
    assert!(menu.languages.contains(&"fr".to_string()));
    assert_eq!(
        do_menu_save_category(&app.state, category(None, "Sushi")).unwrap_err(),
        invalid("name")
    );
    let mut c = category(None, "Sushi");
    c.names.insert("de".into(), "Sushi".into());
    let saved = do_menu_save_category(&app.state, c).unwrap();
    assert_eq!(saved.menu.categories[0].names["de"], "Sushi");
    // Another till does not offer German.
    app.state.config.lock().unwrap().language = "fr".into();
    assert!(
        !do_menu_admin_get(&app.state)
            .unwrap()
            .languages
            .contains(&"de".to_string())
    );
}

#[tokio::test]
async fn a_name_kept_in_a_language_the_till_does_not_offer_survives_an_edit() {
    // Found on the emulator: the seed menu holds German names, a French till does not offer
    // German, and the editor sends every stored name back, so each save was refused as `names`.
    let app = app().await;
    app.state.config.lock().unwrap().language = "de".into();
    let mut c = category(None, "Sushi");
    c.names.insert("de".into(), "Sushi".into());
    let saved = do_menu_save_category(&app.state, c).unwrap();
    let id = saved.id;
    app.state.config.lock().unwrap().language = "fr".into();
    let menu = do_menu_admin_get(&app.state).unwrap();
    let stored = menu.categories.iter().find(|c| c.id == id).unwrap();
    assert_eq!(stored.names["de"], "Sushi");
    // The editor sends the stored names back with the edited French one.
    let mut edit = category(Some(id), "Sushi bar");
    edit.names = stored.names.clone();
    edit.names.insert("fr".into(), "Sushi bar".into());
    let saved = do_menu_save_category(&app.state, edit).unwrap();
    let after = saved.menu.categories.iter().find(|c| c.id == id).unwrap();
    assert_eq!(after.names["fr"], "Sushi bar");
    assert_eq!(after.names["de"], "Sushi");
    // A code that is no language of the table is still refused.
    let mut junk = category(Some(id), "x");
    junk.names.insert("xx".into(), "y".into());
    assert_eq!(
        do_menu_save_category(&app.state, junk).unwrap_err(),
        invalid("names")
    );
}

#[tokio::test]
async fn unknown_parents_and_entries_are_not_found() {
    let app = app().await;
    let nf = |entity: &str, id: i64| {
        errors::coded(errors::MENU_NOT_FOUND, &[("entity", &entity), ("id", &id)])
    };
    assert_eq!(
        do_menu_save_category(&app.state, category(Some(77), "x")).unwrap_err(),
        nf("category", 77)
    );
    assert_eq!(
        do_menu_save_product(&app.state, product(None, 77, "x", 1)).unwrap_err(),
        nf("category", 77)
    );
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    assert_eq!(
        do_menu_save_product(&app.state, product(Some(99), cat, "x", 1)).unwrap_err(),
        nf("product", 99)
    );
    assert_eq!(
        do_menu_save_modifier_group(&app.state, group(None, 5, "x")).unwrap_err(),
        nf("product", 5)
    );
    assert_eq!(
        do_menu_save_modifier(&app.state, option(None, 5, "x", 1)).unwrap_err(),
        nf("modifier_group", 5)
    );
    let prod = do_menu_save_product(&app.state, product(None, cat, "x", 1))
        .unwrap()
        .id;
    let grp = do_menu_save_modifier_group(&app.state, group(None, prod, "g"))
        .unwrap()
        .id;
    assert_eq!(
        do_menu_save_modifier(&app.state, option(Some(9), grp, "x", 1)).unwrap_err(),
        nf("modifier", 9)
    );
    assert_eq!(
        do_menu_save_modifier_group(&app.state, group(Some(9), prod, "x")).unwrap_err(),
        nf("modifier_group", 9)
    );
    // A refused save writes nothing.
    assert_eq!(log_count(&app), 3);
}

#[tokio::test]
async fn every_save_is_logged_with_before_and_after_and_the_log_is_immutable() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    let prod = do_menu_save_product(&app.state, product(None, cat, "Maki", 650))
        .unwrap()
        .id;
    let mut p = product(Some(prod), cat, "Maki", 700);
    p.vat_code = "A".into();
    do_menu_save_product(&app.state, p.clone()).unwrap();
    // The same values again change nothing and log nothing.
    do_menu_save_product(&app.state, p).unwrap();
    assert_eq!(log_count(&app), 3);

    let log = do_list_menu_changes(&app.state, None).unwrap();
    assert_eq!(log.len(), 3);
    let last = &log[0];
    assert_eq!(
        (last.entity_type.as_str(), last.action.as_str()),
        ("product", "update")
    );
    assert_eq!(last.entity_id, prod);
    assert_eq!(last.label, "Maki");
    let mut fields = last.changed_fields.clone();
    fields.sort();
    assert_eq!(fields, vec!["price_cents", "vat_code"]);
    let before: serde_json::Value =
        serde_json::from_str(last.before_json.as_ref().unwrap()).unwrap();
    let after: serde_json::Value = serde_json::from_str(&last.after_json).unwrap();
    assert_eq!(
        (
            before["price_cents"].as_i64(),
            after["price_cents"].as_i64()
        ),
        (Some(650), Some(700))
    );
    assert_eq!(
        (before["vat_code"].as_str(), after["vat_code"].as_str()),
        (Some("B"), Some("A"))
    );
    assert_eq!(last.changed_by, MANAGER);
    assert_ne!(last.changed_by_name.len(), 0);
    assert_ne!(last.changed_at.len(), 0);
    let first = &log[2];
    assert_eq!(
        (first.entity_type.as_str(), first.action.as_str()),
        ("category", "create")
    );
    assert!(first.before_json.is_none());
    assert!(first.changed_fields.contains(&"names".to_string()));
    // The limit caps the list.
    assert_eq!(do_list_menu_changes(&app.state, Some(1)).unwrap().len(), 1);

    let db = app.state.db.lock().unwrap();
    for sql in [
        "UPDATE menu_change_log SET changed_by = 'x'",
        "DELETE FROM menu_change_log",
    ] {
        assert!(
            db.execute(sql, [])
                .unwrap_err()
                .to_string()
                .contains("append-only"),
            "{sql}"
        );
    }
}

#[tokio::test]
async fn saves_are_refused_while_a_source_manages_the_menu_but_reads_work() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO catalog_sync_state (source, manages_menu, applied_at)
             VALUES ('hubrise', 1, datetime('now'))",
            [],
        )
        .unwrap();
    let locked = errors::hubrise::MENU_MANAGED.to_string();
    assert_eq!(
        do_menu_save_category(&app.state, category(None, "X")).unwrap_err(),
        locked
    );
    assert_eq!(
        do_menu_save_category(&app.state, category(Some(cat), "Y")).unwrap_err(),
        locked
    );
    assert_eq!(
        do_menu_save_product(&app.state, product(None, cat, "X", 1)).unwrap_err(),
        locked
    );
    assert_eq!(
        do_menu_save_modifier_group(&app.state, group(None, 1, "X")).unwrap_err(),
        locked
    );
    assert_eq!(
        do_menu_save_modifier(&app.state, option(None, 1, "X", 1)).unwrap_err(),
        locked
    );
    let menu = do_menu_admin_get(&app.state).unwrap();
    assert_eq!(menu.managed_by.as_deref(), Some("hubrise"));
    assert_eq!(menu.categories.len(), 1);
    assert_eq!(do_list_menu_changes(&app.state, None).unwrap().len(), 1);
    // Released: editing is allowed again.
    pos_catalog::apply::release(&app.state.db.lock().unwrap(), "hubrise").unwrap();
    assert!(do_menu_save_category(&app.state, category(Some(cat), "Z")).is_ok());
    assert_eq!(do_menu_admin_get(&app.state).unwrap().managed_by, None);
}

#[tokio::test]
async fn the_menu_shows_edits_and_has_modifiers_follows_active_groups_and_options() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    let prod = do_menu_save_product(&app.state, product(None, cat, "Maki", 650))
        .unwrap()
        .id;
    let flag = |app: &TestApp| {
        do_get_menu_items(&app.state, Some(cat), None)
            .unwrap()
            .into_iter()
            .find(|m| m.id == prod)
            .map(|m| m.has_modifiers)
    };
    assert_eq!(flag(&app), Some(false));
    let grp = do_menu_save_modifier_group(&app.state, group(None, prod, "Sauce"))
        .unwrap()
        .id;
    assert_eq!(flag(&app), Some(false), "a group without option");
    let opt = do_menu_save_modifier(&app.state, option(None, grp, "Soja", 50))
        .unwrap()
        .id;
    assert_eq!(flag(&app), Some(true));
    let mut o = option(Some(opt), grp, "Soja", 50);
    o.active = false;
    do_menu_save_modifier(&app.state, o).unwrap();
    assert_eq!(flag(&app), Some(false), "no active option");
    do_menu_save_modifier(&app.state, option(Some(opt), grp, "Soja", 50)).unwrap();
    let mut g = group(Some(grp), prod, "Sauce");
    g.active = false;
    do_menu_save_modifier_group(&app.state, g).unwrap();
    assert_eq!(flag(&app), Some(false), "no active group");

    // An inactive product or category leaves the grid.
    let mut p = product(Some(prod), cat, "Maki", 650);
    p.active = false;
    do_menu_save_product(&app.state, p).unwrap();
    assert_eq!(flag(&app), None);
    let mut c = category(Some(cat), "Sushi");
    c.active = false;
    do_menu_save_category(&app.state, c).unwrap();
    assert!(do_get_categories(&app.state, None).unwrap().is_empty());
    // The editor still lists them.
    let menu = do_menu_admin_get(&app.state).unwrap();
    assert_eq!((menu.categories.len(), menu.products.len()), (1, 1));
}

#[tokio::test]
async fn the_options_of_an_edited_product_are_sold_with_the_new_prices() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    let prod = do_menu_save_product(&app.state, product(None, cat, "Maki", 650))
        .unwrap()
        .id;
    let grp = do_menu_save_modifier_group(&app.state, group(None, prod, "Sauce"))
        .unwrap()
        .id;
    let opt = do_menu_save_modifier(&app.state, option(None, grp, "Soja", 50))
        .unwrap()
        .id;
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(2),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    let line = do_add_order_line_with_modifiers(
        &app.state,
        &AddWithModifiersInput {
            order_id: order,
            menu_item_id: prod,
            quantity: 1,
            modifier_ids: vec![opt],
        },
    )
    .unwrap();
    assert_eq!(line.unit_price_cents, 700);
    assert_eq!(line.name, "Maki (Soja)");
}

#[tokio::test]
async fn only_a_manager_may_use_the_editor() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    app.logout();
    assert_eq!(
        do_menu_admin_get(&app.state).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as(CASHIER, role);
        assert_eq!(
            do_menu_admin_get(&app.state).unwrap_err(),
            errors::FORBIDDEN
        );
        assert_eq!(
            do_list_menu_changes(&app.state, None).unwrap_err(),
            errors::FORBIDDEN
        );
        assert_eq!(
            do_menu_save_category(&app.state, category(None, "X")).unwrap_err(),
            errors::FORBIDDEN
        );
        assert_eq!(
            do_menu_save_product(&app.state, product(None, cat, "X", 1)).unwrap_err(),
            errors::FORBIDDEN
        );
        assert_eq!(
            do_menu_save_modifier_group(&app.state, group(None, 1, "X")).unwrap_err(),
            errors::FORBIDDEN
        );
        assert_eq!(
            do_menu_save_modifier(&app.state, option(None, 1, "X", 1)).unwrap_err(),
            errors::FORBIDDEN
        );
    }
    assert_eq!(log_count(&app), 1);
}

/// AM art. 44: a price, name and VAT change while an order is open leaves its lines and the
/// signed P and N events unchanged; the next line takes the new values.
#[tokio::test]
async fn art_44_an_edit_never_changes_an_open_order_or_a_signed_event() {
    let app = app().await;
    let cat = do_menu_save_category(&app.state, category(None, "Sushi"))
        .unwrap()
        .id;
    let prod = do_menu_save_product(&app.state, product(None, cat, "Maki saumon", 650))
        .unwrap()
        .id;

    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(4),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    let add = |quantity| {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: prod,
                quantity,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap()
    };
    let before_line = add(2);
    assert_eq!(
        (before_line.unit_price_cents, before_line.name.as_str()),
        (650, "Maki saumon")
    );
    do_sign_table_order(&app.state, order).await.unwrap();
    let signed_p = app.fiscal_events();
    assert_eq!(signed_p.len(), 1);

    // The manager changes the name, the price and the VAT label while the order is open.
    let mut p = product(Some(prod), cat, "Maki saumon x8", 990);
    p.vat_code = "A".into();
    p.vat_code_takeaway = None;
    do_menu_save_product(&app.state, p).unwrap();
    // And renames the category (the department of the next line).
    do_menu_save_category(&app.state, category(Some(cat), "Sushis et makis")).unwrap();

    // The lines already on the order are as they were.
    let lines = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&db, order).unwrap()
    };
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].unit_price_cents, 650);
    assert_eq!(lines[0].vat_code, before_line.vat_code);
    assert_eq!(lines[0].name, "Maki saumon");
    assert_eq!(lines[0].department_name, before_line.department_name);
    // The signed P event is byte for byte what the FDM signed.
    let after_p = app.fiscal_events();
    assert_eq!(after_p.len(), 1);
    assert_eq!(after_p[0].event_data_json, signed_p[0].event_data_json);
    assert_eq!(after_p[0].fdm_response_json, signed_p[0].fdm_response_json);

    // The pre-bill and the sale are signed with the snapshot, not the new menu.
    do_sign_pre_bill(&app.state, order).await.unwrap();
    let result = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let events = app.fiscal_events();
    let sale = events.iter().find(|e| e.event_type == "N").unwrap();
    let sent: serde_json::Value =
        serde_json::from_str(sale.event_data_json.as_deref().unwrap()).unwrap();
    let main = &sent["transaction"]["transactionLines"][0]["mainProduct"];
    assert_eq!(main["productName"], "Maki saumon");
    assert_eq!(main["departmentName"], "Sushi");
    assert_eq!(main["unitPrice"].as_f64(), Some(6.5));
    assert_eq!(main["vats"][0]["label"], "B");
    assert_eq!(sent["transaction"]["transactionTotal"].as_f64(), Some(13.0));
    let p_event = events.iter().find(|e| e.event_type == "P").unwrap();
    assert_eq!(p_event.event_data_json, signed_p[0].event_data_json);

    // A new line on a new order takes the new values.
    let order2 = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(5),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    let fresh = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order2,
            menu_item_id: prod,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    assert_eq!(
        (fresh.unit_price_cents, fresh.name.as_str()),
        (990, "Maki saumon x8")
    );
    assert_eq!(fresh.vat_code.to_string(), "A");
    assert_eq!(fresh.department_name, "Sushis et makis");
}
