//! The 14 EU allergens (V12-D17 to D21): the catalog import with
//! its change log, the local edit and its lock while a source manages
//! the menu, the sheet shown and printed, and the proof that no
//! allergen data reaches a signed request.
//!
//! Events are signed by the admission image's mock FDM.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::allergen::AllergenSet;
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_db::queries::allergens::{self as store, Entity};
use pos_engine::commands::allergens::{
    SetAllergensInput, do_get_allergen_sheet, do_list_allergens, do_print_allergen_sheet,
    do_set_allergens,
};
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::tables::do_sign_table_order;

const ADMIN: &str = "12345678958";

fn set(contains: &[&str], may: &[&str]) -> AllergenSet {
    AllergenSet {
        contains: contains.iter().map(|s| (*s).to_owned()).collect(),
        may_contain: may.iter().map(|s| (*s).to_owned()).collect(),
    }
}

/// Catalogue: category 1 (grid) with 1 = Maki (kitchen) and 2 = Sake;
/// category 2 is hidden and holds 3 = a charge; 4 is off sale.
async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(ADMIN, UserRole::Admin);
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 1200, 'B', 1, 0, 1),
                (2, 1, 650, 'A', 1, 1, 1),
                (4, 1, 500, 'A', 1, 2, 0);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Maki saumon'),
                ('menu_item', 2, 'name', 'fr', 'Sake'),
                ('menu_item', 4, 'name', 'fr', 'Hors vente'),
                ('category', 1, 'name', 'fr', 'Carte');",
        )
        .unwrap();
    }
    app
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "test helper: the flag names what the call switches"
)]
fn edit(
    entity: &str,
    id: i64,
    declared: bool,
    contains: &[&str],
    may: &[&str],
) -> SetAllergensInput {
    SetAllergensInput {
        entity_type: entity.into(),
        entity_id: id,
        declared,
        contains: contains.iter().map(|s| (*s).to_owned()).collect(),
        may_contain: may.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn order(app: &TestApp, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: ADMIN.into(),
        },
    )
    .unwrap()
    .id
}

fn add(app: &TestApp, order: i64, item: i64) {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: item,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
}

#[tokio::test]
async fn the_legend_has_the_14_in_annex_order() {
    let app = app().await;
    let legend = do_list_allergens(&app.state).unwrap();
    assert_eq!(legend.len(), 14);
    let numbers: Vec<i32> = legend.iter().map(|a| a.number).collect();
    assert_eq!(numbers, (1..=14).collect::<Vec<_>>());
    assert_eq!(legend[0].names.fr, "Céréales contenant du gluten");
    assert_eq!(legend[6].code, "milk");
    assert_eq!(legend[6].names.zh, "奶");
}

#[tokio::test]
async fn a_product_without_a_declaration_has_no_information() {
    let app = app().await;
    let sheet = do_get_allergen_sheet(&app.state, Some(1), None).unwrap();
    let p = &sheet.products[0];
    assert!(!p.declared && p.contains.is_empty() && p.traces.is_empty());
    let all = do_get_allergen_sheet(&app.state, None, Some("zh".into())).unwrap();
    assert_eq!(all.lang, "zh");
    assert!(all.products.iter().all(|p| !p.declared));
    // The menu records say it too.
    let items =
        pos_db::queries::menu::get_menu_items(&app.state.db.lock().unwrap(), None, "fr").unwrap();
    assert!(
        items
            .iter()
            .all(|i| !i.allergens_declared && i.allergens.is_empty())
    );
}

#[tokio::test]
async fn a_local_edit_shows_on_the_menu_and_the_sheet_and_is_logged() {
    let app = app().await;
    assert!(
        do_set_allergens(
            &app.state,
            edit("menu_item", 1, true, &["fish", "sesame"], &["molluscs"])
        )
        .unwrap()
    );
    assert!(
        !do_set_allergens(
            &app.state,
            edit("menu_item", 1, true, &["sesame", "fish"], &["molluscs"])
        )
        .unwrap()
    );

    let sheet = do_get_allergen_sheet(&app.state, None, Some("fr".into())).unwrap();
    // Whole menu: the active products only (the off-sale one is left out).
    let ids: Vec<i64> = sheet.products.iter().map(|p| p.id).collect();
    assert_eq!(ids, vec![1, 2]);
    let maki = &sheet.products[0];
    assert!(maki.declared);
    let names: Vec<&str> = maki.contains.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, vec!["Poissons", "Sésame"], "Annex order, in French");
    assert_eq!(maki.contains[0].number, 4);
    assert_eq!(maki.traces[0].name, "Mollusques");
    assert!(!sheet.products[1].declared, "Sake: still no information");

    let items = pos_db::queries::menu::get_menu_items(&app.state.db.lock().unwrap(), Some(1), "fr")
        .unwrap();
    let maki = items.iter().find(|i| i.id == 1).unwrap();
    assert!(maki.allergens_declared);
    assert_eq!(maki.allergens, vec!["fish", "sesame"]);
    assert_eq!(maki.traces, vec!["molluscs"]);

    let log = store::history(&app.state.db.lock().unwrap(), Entity::MenuItem, 1).unwrap();
    assert_eq!(log.len(), 1, "an identical second edit logs nothing");
    assert_eq!(log[0].source, "local");
    assert_eq!(log[0].changed_by.as_deref(), Some(ADMIN));
    assert!(log[0].before_json.contains("\"declared\":false"));
    assert!(log[0].after_json.contains("sesame"));

    // "No allergen" is a statement; clearing goes back to no information.
    assert!(do_set_allergens(&app.state, edit("menu_item", 1, true, &[], &[])).unwrap());
    assert!(
        do_get_allergen_sheet(&app.state, Some(1), None)
            .unwrap()
            .products[0]
            .declared
    );
    assert!(do_set_allergens(&app.state, edit("menu_item", 1, false, &[], &[])).unwrap());
    assert!(
        !do_get_allergen_sheet(&app.state, Some(1), None)
            .unwrap()
            .products[0]
            .declared
    );
    assert_eq!(
        store::history(&app.state.db.lock().unwrap(), Entity::MenuItem, 1)
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn bad_edits_are_refused_with_their_codes() {
    let app = app().await;
    let err =
        do_set_allergens(&app.state, edit("menu_item", 1, true, &["glutten"], &[])).unwrap_err();
    assert_eq!(err, "allergen_unknown|code=glutten");
    let err = do_set_allergens(&app.state, edit("menu_item", 1, true, &[], &["Milk"])).unwrap_err();
    assert_eq!(err, "allergen_unknown|code=Milk");
    let err = do_set_allergens(&app.state, edit("menu_item", 999, true, &[], &[])).unwrap_err();
    assert_eq!(err, "allergen_target_not_found|id=999");
    let err = do_set_allergens(&app.state, edit("modifier", 999, true, &[], &[])).unwrap_err();
    assert_eq!(err, "allergen_target_not_found|id=999");
    let err = do_set_allergens(&app.state, edit("category", 1, true, &[], &[])).unwrap_err();
    assert!(err.starts_with(errors::ALLERGEN_TARGET_NOT_FOUND), "{err}");
    assert!(do_get_allergen_sheet(&app.state, Some(999), None).is_err());
    let err = do_get_allergen_sheet(&app.state, None, Some("xx".into())).unwrap_err();
    assert!(err.starts_with(errors::ALLERGEN_UNKNOWN), "{err}");
    // Nothing was written.
    assert_eq!(
        store::history(&app.state.db.lock().unwrap(), Entity::MenuItem, 1)
            .unwrap()
            .len(),
        0
    );
}

#[tokio::test]
async fn an_option_has_its_own_list() {
    let app = app().await;
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO modifier_groups (id, menu_item_id, min_select, max_select) VALUES (1, 1, 0, 1);
             INSERT INTO modifiers (id, group_id, price_cents, sort_order) VALUES (1, 1, 0, 0), (2, 1, 50, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('modifier_group', 1, 'name', 'fr', 'Sauce'),
                ('modifier', 1, 'name', 'fr', 'Soja'),
                ('modifier', 2, 'name', 'fr', 'Nature');",
        )
        .unwrap();
    }
    // The option tables need the catalog columns (`active`): present in
    // every build that mirrors a catalog.
    #[cfg(feature = "hubrise")]
    {
        do_set_allergens(
            &app.state,
            edit("modifier", 1, true, &["soybeans", "gluten"], &[]),
        )
        .unwrap();
        do_set_allergens(&app.state, edit("modifier", 2, true, &[], &[])).unwrap();
        let sheet = do_get_allergen_sheet(&app.state, Some(1), Some("en".into())).unwrap();
        let options = &sheet.products[0].options;
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].group, "Sauce");
        assert_eq!(options[0].name, "Soja");
        assert!(options[0].declared);
        let names: Vec<&str> = options[0]
            .contains
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(names, vec!["Cereals containing gluten", "Soybeans"]);
        assert!(options[1].declared && options[1].contains.is_empty());
    }
}

#[tokio::test]
async fn roles_staff_reads_and_a_manager_edits() {
    let app = app().await;
    app.login_as("75061189702", UserRole::Waiter);
    assert!(do_list_allergens(&app.state).is_ok());
    assert!(do_get_allergen_sheet(&app.state, None, None).is_ok());
    assert!(
        do_print_allergen_sheet(&app.state, None, None)
            .await
            .is_ok()
    );
    assert_eq!(
        do_set_allergens(&app.state, edit("menu_item", 1, true, &[], &[])).unwrap_err(),
        errors::FORBIDDEN
    );
    app.logout();
    assert_eq!(
        do_list_allergens(&app.state).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_get_allergen_sheet(&app.state, None, None).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_print_allergen_sheet(&app.state, None, None)
            .await
            .unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_set_allergens(&app.state, edit("menu_item", 1, true, &[], &[])).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
}

#[tokio::test]
async fn the_sheet_prints_without_a_price_and_with_both_footers() {
    let app = app().await;
    do_set_allergens(&app.state, edit("menu_item", 1, true, &["fish"], &["nuts"])).unwrap();
    do_set_allergens(&app.state, edit("menu_item", 2, true, &[], &[])).unwrap();

    // No printer configured.
    assert_eq!(
        do_print_allergen_sheet(&app.state, None, None)
            .await
            .unwrap(),
        SlipOutcome::NoPrinter
    );

    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    assert_eq!(
        do_print_allergen_sheet(&app.state, None, Some("fr".into()))
            .await
            .unwrap(),
        SlipOutcome::Printed
    );
    let mut text = String::new();
    for _ in 0..40 {
        text = printer.rendered().join("\n");
        if text.contains("ALLERG") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let one = pos_print::render::one_line(&text);
    for expected in [
        "ALLERGÈNES",
        "Maki saumon",
        "Contient: Poissons",
        "Traces possibles: Fruits à coque",
        "Sake",
        "Aucun allergène",
        "Selon le règlement (UE) n° 1169/2011, annexe II.",
        "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
    ] {
        assert!(one.contains(expected), "{expected:?} missing from:\n{text}");
    }
    assert!(
        !one.contains("Hors vente"),
        "off-sale items stay off the sheet"
    );
    assert!(
        !text.contains("12,00") && !text.contains('€'),
        "no price: {text}"
    );

    // A product that does not exist prints nothing.
    assert!(
        do_print_allergen_sheet(&app.state, Some(999), None)
            .await
            .is_err()
    );
    // A language the printer cannot print.
    assert!(
        do_print_allergen_sheet(&app.state, None, Some("zh".into()))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn no_allergen_data_reaches_a_signed_request() {
    let app = app().await;
    do_set_allergens(
        &app.state,
        edit("menu_item", 1, true, &["fish", "gluten"], &["molluscs"]),
    )
    .unwrap();
    do_set_allergens(&app.state, edit("menu_item", 2, true, &["sulphites"], &[])).unwrap();

    let table = order(&app, Some(5));
    add(&app, table, 1);
    add(&app, table, 2);
    do_sign_table_order(&app.state, table).await.unwrap();
    let direct = order(&app, None);
    add(&app, direct, 1);
    let r = do_checkout(&app.state, direct, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    let events = app.fiscal_events();
    assert!(events.len() >= 2, "a P and an N were signed");
    for event in &events {
        let archived = format!(
            "{}{}",
            event.event_data_json.as_deref().unwrap_or_default(),
            event.fdm_response_json.as_deref().unwrap_or_default()
        )
        .to_lowercase();
        for word in [
            "allerg", "gluten", "fish", "poisson", "sulphite", "molluscs", "traces", "annex",
        ] {
            assert!(!archived.contains(word), "{word} in {}", event.event_type);
        }
    }
}

#[cfg(feature = "hubrise")]
mod catalog {
    use super::*;
    use pos_catalog::testing::catalog;
    use pos_catalog::{ApplyOutcome, Catalog, IssueCode};

    fn with_allergens(contains: &[&str], may: &[&str]) -> Catalog {
        let mut c = catalog();
        c.items[0].allergens = Some(set(contains, may));
        c.items[0].groups[0].options[0].allergens = Some(set(&["soybeans"], &[]));
        c.items[0].groups[0].options[1].allergens = Some(AllergenSet::default());
        c
    }

    fn item_id(app: &TestApp, key: &str) -> i64 {
        app.state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM menu_items WHERE external_id = ?1",
                [key],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn apply(app: &TestApp, c: &Catalog) -> ApplyOutcome {
        let mut conn = app.state.db.lock().unwrap();
        pos_catalog::apply(&mut conn, "hubrise", c).unwrap()
    }

    fn logs(app: &TestApp) -> i64 {
        app.state
            .db
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM allergen_change_log", [], |r| r.get(0))
            .unwrap()
    }

    async fn empty_app() -> TestApp {
        let app = TestApp::new().await;
        app.login_as(ADMIN, UserRole::Admin);
        app
    }

    #[tokio::test]
    async fn the_import_writes_rows_declarations_and_log_rows() {
        let app = empty_app().await;
        assert!(matches!(
            apply(&app, &with_allergens(&["fish"], &["molluscs"])),
            ApplyOutcome::Applied { .. }
        ));
        let maki = item_id(&app, "MAKI-6");
        let coca = item_id(&app, "COCA");
        let conn = app.state.db.lock().unwrap();
        let (declared, source) = store::get_declared(&conn, Entity::MenuItem, maki)
            .unwrap()
            .unwrap();
        assert_eq!(declared.contains, vec!["fish"]);
        assert_eq!(declared.may_contain, vec!["molluscs"]);
        assert_eq!(source, "catalog");
        assert!(
            store::get(&conn, Entity::MenuItem, coca).unwrap().is_none(),
            "no tag, no information"
        );
        let options = store::load_all(&conn, Entity::Modifier).unwrap();
        assert_eq!(options.len(), 2);
        assert!(options.values().any(|s| s.contains == vec!["soybeans"]));
        assert!(
            options
                .values()
                .any(|s| s.contains.is_empty() && s.may_contain.is_empty())
        );
        drop(conn);
        assert_eq!(logs(&app), 3, "the product and its two options");
    }

    #[tokio::test]
    async fn the_same_catalog_and_an_unrelated_change_write_no_log_row() {
        let app = empty_app().await;
        let c = with_allergens(&["fish"], &[]);
        apply(&app, &c);
        assert_eq!(logs(&app), 3);
        assert_eq!(apply(&app, &c), ApplyOutcome::Unchanged);
        assert_eq!(logs(&app), 3);
        // A price change re-applies the menu but not the allergens.
        let mut priced = c;
        priced.items[0].price_cents += 10;
        assert!(matches!(apply(&app, &priced), ApplyOutcome::Applied { .. }));
        assert_eq!(logs(&app), 3, "same allergens: no log row");
    }

    #[tokio::test]
    async fn a_changed_tag_logs_before_and_after() {
        let app = empty_app().await;
        apply(&app, &with_allergens(&["fish"], &[]));
        apply(&app, &with_allergens(&["fish", "eggs"], &[]));
        let maki = item_id(&app, "MAKI-6");
        let log = store::history(&app.state.db.lock().unwrap(), Entity::MenuItem, maki).unwrap();
        assert_eq!(log.len(), 2);
        assert!(log[1].before_json.contains("fish") && !log[1].before_json.contains("eggs"));
        assert!(log[1].after_json.contains("eggs"));
        assert_eq!(log[1].source, "catalog");
        assert_eq!(log[1].changed_by, None);
        // The tag disappears: back to no information, logged.
        let mut gone = with_allergens(&["fish", "eggs"], &[]);
        gone.items[0].allergens = None;
        apply(&app, &gone);
        let log = store::history(&app.state.db.lock().unwrap(), Entity::MenuItem, maki).unwrap();
        assert_eq!(log.len(), 3);
        assert!(log[2].after_json.contains("\"declared\":false"));
    }

    #[tokio::test]
    async fn an_unknown_code_refuses_the_whole_catalog() {
        let app = empty_app().await;
        apply(&app, &with_allergens(&["fish"], &[]));
        let outcome = apply(&app, &with_allergens(&["fish"], &["glutten"]));
        let ApplyOutcome::Rejected { issues } = outcome else {
            panic!("refused: {outcome:?}");
        };
        assert!(
            issues
                .iter()
                .any(|i| i.code == IssueCode::UnknownAllergen && i.key == "MAKI-6")
        );
        // The menu is as it was.
        let maki = item_id(&app, "MAKI-6");
        let conn = app.state.db.lock().unwrap();
        assert_eq!(
            store::get(&conn, Entity::MenuItem, maki)
                .unwrap()
                .unwrap()
                .contains,
            vec!["fish"]
        );
    }

    #[tokio::test]
    async fn a_local_edit_is_refused_while_a_source_manages_the_menu() {
        let app = empty_app().await;
        apply(&app, &with_allergens(&["fish"], &[]));
        let maki = item_id(&app, "MAKI-6");
        let err = do_set_allergens(&app.state, edit("menu_item", maki, true, &["milk"], &[]))
            .unwrap_err();
        assert_eq!(err, "hubrise_menu_managed");
        // The source lets go: editing is allowed again.
        pos_catalog::release(&app.state.db.lock().unwrap(), "hubrise").unwrap();
        assert!(
            do_set_allergens(&app.state, edit("menu_item", maki, true, &["milk"], &[])).unwrap()
        );
        let log = store::history(&app.state.db.lock().unwrap(), Entity::MenuItem, maki).unwrap();
        assert_eq!(log.last().unwrap().source, "local");
    }

    #[tokio::test]
    async fn a_catalog_without_information_keeps_what_a_manager_entered() {
        let app = empty_app().await;
        let mut c = catalog();
        apply(&app, &c);
        let coca = item_id(&app, "COCA");
        pos_catalog::release(&app.state.db.lock().unwrap(), "hubrise").unwrap();
        do_set_allergens(
            &app.state,
            edit("menu_item", coca, true, &["sulphites"], &[]),
        )
        .unwrap();
        // The source takes the menu back, still silent about COCA.
        c.items[0].price_cents += 1;
        apply(&app, &c);
        let conn = app.state.db.lock().unwrap();
        let (kept, source) = store::get_declared(&conn, Entity::MenuItem, coca)
            .unwrap()
            .unwrap();
        assert_eq!(kept.contains, vec!["sulphites"]);
        assert_eq!(source, "local");
    }

    #[tokio::test]
    async fn the_whole_menu_sheet_leaves_out_hidden_and_system_items() {
        let app = empty_app().await;
        apply(&app, &with_allergens(&["fish"], &[]));
        let sheet = do_get_allergen_sheet(&app.state, None, Some("fr".into())).unwrap();
        let names: Vec<&str> = sheet.products.iter().map(|p| p.name_fr.as_str()).collect();
        assert_eq!(
            names,
            vec!["MAKI-6", "COCA"],
            "the charge is not on the sheet: {names:?}"
        );
        let maki = &sheet.products[0];
        assert_eq!(maki.options.len(), 2);
        assert!(
            maki.options
                .iter()
                .any(|o| o.declared && o.contains[0].code == "soybeans")
        );
    }
}
