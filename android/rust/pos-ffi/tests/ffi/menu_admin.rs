//! Tests of the menu editor facade (`api::menu_admin`, `records::menu_admin`, v1.1).
//!
//! (a) Contract: every FFI record is the engine's type field by field, with names in several
//! languages, and the inputs convert back to the engine's inputs.
//!
//! (b) Wiring: a `PosEngine` on the seeded database; a manager creates the four entities, reads
//! the menu and the log; a cashier is refused; a catalog that manages the menu locks the saves.

use pos_core::user::UserRole;
use pos_engine::commands::menu_admin as eng;
use pos_engine_testkit::TestApp;
use pos_ffi::PosError;
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::records::menu_admin as rec;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, to_value};

use crate::support::{MANAGER_INSZ, MANAGER_PIN, open_seeded};

const MANAGER: &str = "94060242576";
const CASHIER_INSZ: &str = "75061189702";
const CASHIER_PIN: &str = "4444";

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

fn out_equal<E: Serialize, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = contract_out(engine).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        json_of(engine),
        json_of(&record),
        "engine vs FFI serde form"
    );
    record
}

fn in_equal<E: Serialize + DeserializeOwned, F: Serialize + DeserializeOwned>(record: &F) -> E {
    let engine: E = contract_inp(record).unwrap_or_else(|e| panic!("convert::inp: {e}"));
    assert_eq!(
        json_of(record),
        json_of(&engine),
        "FFI vs engine serde form"
    );
    engine
}

fn names(fr: &str, zh: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut n = std::collections::HashMap::from([("fr".to_string(), fr.to_string())]);
    if let Some(zh) = zh {
        n.insert("zh".into(), zh.into());
    }
    n
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[tokio::test]
async fn the_menu_records_are_the_engines_types_field_by_field() {
    let app = TestApp::new().await;
    app.login_as(MANAGER, UserRole::Admin);
    let btree = |fr: &str, zh: &str| {
        std::collections::BTreeMap::from([
            ("fr".to_string(), fr.to_string()),
            ("zh".to_string(), zh.to_string()),
        ])
    };
    let cat = eng::do_menu_save_category(
        &app.state,
        eng::SaveCategoryInput {
            id: None,
            names: btree("Sushi", "寿司"),
            sort_order: 1,
            color: Some("#E94560".into()),
            active: true,
        },
    )
    .unwrap();
    let prod = eng::do_menu_save_product(
        &app.state,
        eng::SaveProductInput {
            id: None,
            category_id: cat.id,
            names: btree("Maki", "卷"),
            price_cents: 650,
            vat_code: "B".into(),
            vat_code_takeaway: Some("C".into()),
            kitchen_printer: true,
            code: Some("A1".into()),
            open_price: false,
            sort_order: 1,
            active: true,
        },
    )
    .unwrap();
    let grp = eng::do_menu_save_modifier_group(
        &app.state,
        eng::SaveModifierGroupInput {
            id: None,
            menu_item_id: prod.id,
            names: btree("Sauce", "酱"),
            min_select: 0,
            max_select: 2,
            sort_order: 1,
            active: true,
        },
    )
    .unwrap();
    let opt = eng::do_menu_save_modifier(
        &app.state,
        eng::SaveModifierInput {
            id: None,
            group_id: grp.id,
            names: btree("Soja", "酱油"),
            price_cents: 50,
            sort_order: 1,
            active: false,
        },
    )
    .unwrap();

    // Every option set: names in two languages, colour, takeaway label, code.
    let saved: rec::MenuSaved = out_equal(&opt);
    assert_eq!(saved.menu.products[0].names["zh"], "卷");
    assert_eq!(saved.menu.options.len(), 1);
    let menu: rec::MenuAdmin = out_equal(&eng::do_menu_admin_get(&app.state).unwrap());
    assert_eq!(menu.languages[0], "fr");
    assert_eq!(menu.managed_by, None);
    let changes: Vec<rec::MenuChange> =
        out_equal(&eng::do_list_menu_changes(&app.state, None).unwrap());
    assert_eq!(changes.len(), 4);
    assert!(changes[3].before_json.is_none());

    // Every option empty.
    let eng_sparse = eng::do_menu_save_product(
        &app.state,
        eng::SaveProductInput {
            id: None,
            category_id: cat.id,
            names: btree("Thé", "茶"),
            price_cents: 0,
            vat_code: "A".into(),
            vat_code_takeaway: None,
            kitchen_printer: false,
            code: None,
            open_price: true,
            sort_order: 2,
            active: true,
        },
    )
    .unwrap();
    let sparse: rec::MenuSaved = out_equal(&eng_sparse);
    let product = sparse
        .menu
        .products
        .iter()
        .find(|p| p.id == sparse.id)
        .unwrap();
    assert!(product.vat_code_takeaway.is_none() && product.code.is_none());
    let mut no_color = eng::do_menu_admin_get(&app.state).unwrap();
    no_color.categories[0].color = None;
    no_color.managed_by = Some("hubrise".into());
    let _: rec::MenuAdmin = out_equal(&no_color);

    // Inputs: an FFI value with every field set converts to the engine's.
    let cat_in = rec::SaveCategoryInput {
        id: Some(3),
        names: names("Sushi", Some("寿司")),
        sort_order: 4,
        color: Some("#00D4FF".into()),
        active: false,
    };
    let e: eng::SaveCategoryInput = in_equal(&cat_in);
    assert_eq!((e.id, e.sort_order, e.active), (Some(3), 4, false));
    assert_eq!(e.names["zh"], "寿司");
    let prod_in = rec::SaveProductInput {
        id: Some(5),
        category_id: 2,
        names: names("Maki", None),
        price_cents: 1250,
        vat_code: "C".into(),
        vat_code_takeaway: Some("D".into()),
        kitchen_printer: true,
        code: Some("B14".into()),
        open_price: true,
        sort_order: 9,
        active: true,
    };
    let e: eng::SaveProductInput = in_equal(&prod_in);
    assert_eq!(
        (e.price_cents, e.vat_code_takeaway.as_deref()),
        (1250, Some("D"))
    );
    let grp_in = rec::SaveModifierGroupInput {
        id: None,
        menu_item_id: 7,
        names: names("Sauce", None),
        min_select: 1,
        max_select: 3,
        sort_order: 2,
        active: true,
    };
    let e: eng::SaveModifierGroupInput = in_equal(&grp_in);
    assert_eq!((e.min_select, e.max_select), (1, 3));
    let opt_in = rec::SaveModifierInput {
        id: Some(8),
        group_id: 7,
        names: names("Soja", Some("酱油")),
        price_cents: 75,
        sort_order: 3,
        active: true,
    };
    let e: eng::SaveModifierInput = in_equal(&opt_in);
    assert_eq!(e.price_cents, 75);

    // The records refuse a field the engine does not have.
    let mut value = json_of(&cat_in);
    value["extra"] = Value::Bool(true);
    assert!(serde_json::from_value::<rec::SaveCategoryInput>(value).is_err());
    let mut value = json_of(&menu);
    value["extra"] = Value::Bool(true);
    assert!(serde_json::from_value::<rec::MenuAdmin>(value).is_err());
}

// ───────────────────────────── (b) wiring ─────────────────────────────

fn code_of(err: PosError) -> String {
    match err {
        PosError::Coded { code, .. } => code,
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn category(fr: &str) -> rec::SaveCategoryInput {
    rec::SaveCategoryInput {
        id: None,
        names: names(fr, None),
        sort_order: 1,
        color: None,
        active: true,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_manager_edits_the_menu_through_the_facade() {
    let rig = open_seeded().await;
    let e = &rig.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .expect("login");

    let before = e.menu_admin_get().await.expect("menu_admin_get");
    assert_eq!(before.managed_by, None);
    assert_eq!(before.languages[0], "fr");

    let cat = e
        .menu_save_category(category("Desserts"))
        .await
        .expect("category");
    assert!(cat.menu.categories.iter().any(|c| c.id == cat.id));
    let prod = e
        .menu_save_product(rec::SaveProductInput {
            id: None,
            category_id: cat.id,
            names: names("Mochi", Some("麻糬")),
            price_cents: 450,
            vat_code: "B".into(),
            vat_code_takeaway: None,
            kitchen_printer: false,
            code: None,
            open_price: false,
            sort_order: 1,
            active: true,
        })
        .await
        .expect("product");
    let grp = e
        .menu_save_modifier_group(rec::SaveModifierGroupInput {
            id: None,
            menu_item_id: prod.id,
            names: names("Parfum", None),
            min_select: 1,
            max_select: 1,
            sort_order: 1,
            active: true,
        })
        .await
        .expect("group");
    e.menu_save_modifier(rec::SaveModifierInput {
        id: None,
        group_id: grp.id,
        names: names("Matcha", None),
        price_cents: 0,
        sort_order: 1,
        active: true,
    })
    .await
    .expect("option");

    // The till sees the product with its options flag.
    let items = e.get_menu_items(Some(cat.id), None).await.expect("items");
    assert_eq!(items.len(), 1);
    assert!(items[0].has_modifiers);

    let log = e.list_menu_changes(Some(10)).await.expect("log");
    assert_eq!(log.len(), 4);
    assert_eq!(log[0].entity_type, "modifier");
    assert_eq!(log[3].entity_type, "category");

    // A refused value comes back coded with its field.
    let err = e
        .menu_save_product(rec::SaveProductInput {
            id: None,
            category_id: cat.id,
            names: names("X", None),
            price_cents: -1,
            vat_code: "B".into(),
            vat_code_takeaway: None,
            kitchen_printer: false,
            code: None,
            open_price: false,
            sort_order: 1,
            active: true,
        })
        .await
        .unwrap_err();
    assert_eq!(code_of(err), "menu_invalid");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_cashier_is_refused_and_every_method_is_closed_after_shutdown() {
    let rig = open_seeded().await;
    let e = &rig.engine;
    e.login(CASHIER_INSZ.into(), Some(CASHIER_PIN.into()))
        .await
        .expect("login");
    assert_eq!(code_of(e.menu_admin_get().await.unwrap_err()), "forbidden");
    assert_eq!(
        code_of(e.menu_save_category(category("X")).await.unwrap_err()),
        "forbidden"
    );
    assert_eq!(
        code_of(e.list_menu_changes(None).await.unwrap_err()),
        "forbidden"
    );
    e.shutdown().await.unwrap();
    assert!(matches!(
        e.menu_admin_get().await,
        Err(PosError::EngineClosed)
    ));
}
