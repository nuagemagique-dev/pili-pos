//! Tests of the orders facade (`api::orders`, `records::orders`): the contract of every record,
//! one wiring test per method through `PosEngine`, and `vat_split`.
//!
//! - Contract, output types: a real engine value (a command run on a `TestApp`) or a hand-built
//!   one, with `Option` fields set and empty, goes through `convert::out`; the two serde forms must
//!   be equal. Input types: an FFI value with every field set goes through `convert::inp` and the
//!   engine fields are checked.
//! - Wiring: an engine on the seeded database of `support`, extended with a few orders. No FFI login
//!   exists before the auth card, so the methods that need an operator answer `not_logged_in`
//!   here; the logged-in dine-in flow (create, add, cart, leave, lock) runs in the unit tests of
//!   `src/api/orders.rs`, which can open the session through the engine's own state.

use std::fmt::Debug;

use pos_core::order::{OrderStatus as EngineOrderStatus, OrderType as EngineOrderType};
use pos_core::vat::VatCode as EngineVatCode;
use pos_engine::commands::order::{
    AddOrderLineInput as EngineAddOrderLineInput, CreateOrderInput as EngineCreateOrderInput,
    do_add_order_line, do_create_order, do_get_order, do_list_cart_lines, do_list_open_orders,
    do_list_order_lines,
};
use pos_engine::commands::session::{
    LeaveReason as EngineLeaveReason, LockOutcome as EngineLockOutcome, do_lock_session,
};
use pos_engine::commands::tables::{
    TableInfo as EngineTableInfo, TakeawayInfo as EngineTakeawayInfo, do_list_open_takeaways,
    do_list_tables,
};
use pos_engine::commands::{menu, modifiers};
use pos_engine_testkit::TestApp;
use pos_ffi::PosError;
use pos_ffi::api::orders::vat_split;
use pos_ffi::records::common::{VatCode, contract_inp, contract_out};
use pos_ffi::records::orders as ffi;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, to_value};

use crate::support::{MANAGER_INSZ, TestEngine, options_for};

// ───────────────────────────── helpers ─────────────────────────────

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

/// Engine value to FFI record through `convert::out`; the two serde forms must be equal.
fn out_equal<E: Serialize, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = contract_out(engine).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        json_of(engine),
        json_of(&record),
        "engine vs FFI serde form"
    );
    record
}

/// `out_equal` for an enum, plus: the variants stay distinct, and the way back gives the same
/// engine value.
fn enum_both<
    E: Serialize + DeserializeOwned + PartialEq + Debug,
    F: Serialize + DeserializeOwned + PartialEq + Debug,
>(
    variants: &[E],
) {
    for (i, v) in variants.iter().enumerate() {
        let record: F = out_equal(v);
        let back: E = contract_inp(&record).unwrap_or_else(|e| panic!("convert::inp: {e}"));
        assert_eq!(&back, v);
        for (j, w) in variants.iter().enumerate() {
            let other: F = out_equal(w);
            assert_eq!(i == j, record == other, "{record:?} vs {other:?}");
        }
    }
}

fn code_of(err: PosError) -> String {
    match err {
        PosError::Coded { code, .. } => code,
        other => panic!("expected a coded error, got {other:?}"),
    }
}

// ───────────────────────────── real engine values ─────────────────────────────

/// A mod-97-valid INSZ.
const INSZ: &str = "12345678958";

/// A `TestApp` with a small menu: two categories (one with a colour), three items (a menu code, a
/// takeaway label, a free price), Chinese names for some, and option groups on item 2.
async fn app() -> TestApp {
    let app = TestApp::new().await;
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order, color, active) VALUES (1, 0, '#E74C3C', 1), (2, 1, NULL, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, vat_code_takeaway, sort_order, active, open_price, code) VALUES
                (1, 1, 1200, 'A', NULL, 0, 1, 0, 'A1'),
                (2, 2, 2800, 'B', 'C', 1, 1, 0, NULL),
                (3, 2, 0,    'B', NULL, 2, 1, 1, NULL);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('category', 1, 'name', 'fr', 'Boissons'),
                ('category', 1, 'name', 'zh', '饮料'),
                ('category', 2, 'name', 'fr', 'Cuisine'),
                ('menu_item', 1, 'name', 'fr', 'Eau pétillante'),
                ('menu_item', 1, 'name', 'zh', '气泡水'),
                ('menu_item', 2, 'name', 'fr', 'Bœuf bourguignon'),
                ('menu_item', 3, 'name', 'fr', 'Plat du jour');
             INSERT INTO modifier_groups (id, min_select, max_select, menu_item_id) VALUES (1, 1, 1, 2), (2, 0, -1, 2);
             INSERT INTO modifiers (id, group_id, price_cents, sort_order) VALUES (1, 1, 0, 0), (2, 1, 150, 1), (3, 2, 100, 0);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('modifier_group', 1, 'name', 'fr', 'Sauce'),
                ('modifier_group', 2, 'name', 'fr', 'Extras'),
                ('modifier', 1, 'name', 'fr', 'Soja'),
                ('modifier', 2, 'name', 'fr', 'Sucrée'),
                ('modifier', 3, 'name', 'fr', 'Gingembre');",
        )
        .unwrap();
    }
    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    app
}

fn create(app: &TestApp, table: Option<i32>) -> pos_core::order::Order {
    do_create_order(
        &app.state,
        EngineCreateOrderInput {
            order_type: if table.is_some() {
                EngineOrderType::DineIn
            } else {
                EngineOrderType::Takeaway
            },
            table_number: table,
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
}

fn add(
    app: &TestApp,
    order_id: i64,
    menu_item_id: i64,
    modifiers_json: Option<&str>,
    unit_price_cents: Option<i64>,
) -> pos_core::order::OrderLine {
    do_add_order_line(
        &app.state,
        EngineAddOrderLineInput {
            order_id,
            menu_item_id,
            quantity: 2,
            modifiers_json: modifiers_json.map(str::to_string),
            unit_price_cents,
        },
    )
    .unwrap()
}

#[tokio::test]
async fn orders_of_every_kind_and_status_convert() {
    let app = app().await;
    let dine_in = create(&app, Some(4));
    let takeaway = create(&app, None);
    let order: ffi::Order = out_equal(&dine_in);
    assert_eq!(order.table_number, Some(4));
    assert_eq!(order.order_type, ffi::OrderType::DineIn);
    assert_eq!(order.status, ffi::OrderStatus::Open);
    assert_eq!(order.closed_at, None);
    let order: ffi::Order = out_equal(&takeaway);
    assert_eq!(order.table_number, None);
    assert_eq!(order.order_type, ffi::OrderType::Takeaway);

    // closed_at set, and the three statuses, read back by the engine.
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute(
            "UPDATE orders SET status = 'closed', closed_at = datetime('now'), total_cents = 777 WHERE id = ?1",
            [takeaway.id],
        )
        .unwrap();
        conn.execute(
            "UPDATE orders SET status = 'voided' WHERE id = ?1",
            [dine_in.id],
        )
        .unwrap();
    }
    let closed = do_get_order(&app.state, takeaway.id).unwrap();
    assert_eq!(closed.status, EngineOrderStatus::Closed);
    let record: ffi::Order = out_equal(&closed);
    assert_eq!(record.status, ffi::OrderStatus::Closed);
    assert!(record.closed_at.is_some());
    assert_eq!(record.total_cents, 777);
    let voided: ffi::Order = out_equal(&do_get_order(&app.state, dine_in.id).unwrap());
    assert_eq!(voided.status, ffi::OrderStatus::Voided);

    // The list commands convert as a whole.
    let third = create(&app, Some(6));
    let open = do_list_open_orders(&app.state).unwrap();
    assert_eq!(open.len(), 1);
    let records: Vec<ffi::Order> = out_equal(&open);
    assert_eq!(records[0].id, third.id);
}

#[tokio::test]
async fn lines_and_cart_lines_convert_with_and_without_options() {
    let app = app().await;
    let order = create(&app, Some(2));
    let plain = add(&app, order.id, 1, None, None);
    let with_json = add(
        &app,
        order.id,
        2,
        Some(r#"[{"id":1,"name":"Soja","price_cents":0}]"#),
        None,
    );
    let free = add(&app, order.id, 3, None, Some(1550));

    let record: ffi::OrderLine = out_equal(&plain);
    assert_eq!(record.modifiers_json, None);
    assert_eq!(record.vat_code, VatCode::A);
    assert!(!record.department_id.is_empty());
    let record: ffi::OrderLine = out_equal(&with_json);
    assert!(record.modifiers_json.is_some());
    let record: ffi::OrderLine = out_equal(&free);
    assert_eq!(record.unit_price_cents, 1550);

    let lines = do_list_order_lines(&app.state, order.id).unwrap();
    assert_eq!(lines.len(), 3);
    let records: Vec<ffi::OrderLine> = out_equal(&lines);
    assert_eq!(records.len(), 3);

    // Cart lines: menu code and Chinese name present for item 1, absent for the others.
    let cart = do_list_cart_lines(&app.state, order.id).unwrap();
    assert_eq!(cart.len(), 3);
    let records: Vec<ffi::CartLine> = out_equal(&cart);
    let first = &records[0];
    assert_eq!(first.menu_item_id, 1);
    assert_eq!(first.code.as_deref(), Some("A1"));
    assert_eq!(first.name_zh.as_deref(), Some("气泡水"));
    assert_eq!(first.name_fr.as_deref(), Some("Eau pétillante"));
    assert_eq!(first.unsent_quantity, 2);
    let second = &records[1];
    assert_eq!(second.code, None);
    assert_eq!(second.name_zh, None);
    // The flattened OrderLine fields are the line's own.
    assert_eq!(first.id, plain.id);
    assert_eq!(first.quantity, 2);
    assert_eq!(first.vat_code, VatCode::A);
}

#[tokio::test]
async fn categories_and_menu_items_convert_with_options_set_and_empty() {
    let app = app().await;
    let categories = menu::do_get_categories(&app.state, None).unwrap();
    assert_eq!(categories.len(), 2);
    let records: Vec<ffi::Category> = out_equal(&categories);
    assert_eq!(records[0].color.as_deref(), Some("#E74C3C"));
    assert_eq!(records[0].name_zh.as_deref(), Some("饮料"));
    assert_eq!(records[1].color, None);
    assert_eq!(records[1].name_zh, None);
    // Another language of the UI.
    let zh = menu::do_get_categories(&app.state, Some("zh".into())).unwrap();
    let _: Vec<ffi::Category> = out_equal(&zh);

    let items = menu::do_get_menu_items(&app.state, None, None).unwrap();
    assert_eq!(items.len(), 3);
    let records: Vec<ffi::MenuItem> = out_equal(&items);
    assert_eq!(records[0].code.as_deref(), Some("A1"));
    assert_eq!(records[0].vat_code_takeaway, None);
    assert!(!records[0].open_price);
    assert_eq!(records[1].vat_code_takeaway, Some(VatCode::C));
    assert_eq!(records[1].code, None);
    assert!(records[2].open_price);
    let of_category = menu::do_get_menu_items(&app.state, Some(2), Some("fr".into())).unwrap();
    let records: Vec<ffi::MenuItem> = out_equal(&of_category);
    assert_eq!(records.len(), 2);
}

#[tokio::test]
async fn modifier_groups_convert_with_and_without_a_maximum() {
    let app = app().await;
    let groups = modifiers::do_get_item_modifiers_localized(&app.state, 2, Some("fr")).unwrap();
    assert_eq!(groups.len(), 2);
    let records: Vec<ffi::ModifierGroup> = out_equal(&groups);
    assert_eq!(records[0].name, "Sauce");
    assert_eq!((records[0].min, records[0].max), (1, Some(1)));
    assert_eq!(records[0].options.len(), 2);
    assert_eq!(records[0].options[1].price_cents, 150);
    assert_eq!(records[1].max, None, "max_select -1 is no maximum");
    assert_eq!(records[1].options[0].name, "Gingembre");
    // No group at all.
    let none = modifiers::do_get_item_modifiers_localized(&app.state, 1, None).unwrap();
    let records: Vec<ffi::ModifierGroup> = out_equal(&none);
    assert!(records.is_empty());
}

#[tokio::test]
async fn tables_and_takeaways_convert() {
    let app = app().await;
    let dine_in = create(&app, Some(3));
    add(&app, dine_in.id, 1, None, None);
    let takeaway = create(&app, None);
    add(&app, takeaway.id, 2, None, None);

    let tables = do_list_tables(&app.state, 5).unwrap();
    assert_eq!(tables.len(), 5);
    let records: Vec<ffi::TableInfo> = out_equal(&tables);
    let busy = &records[2];
    assert_eq!(busy.table_number, 3);
    assert_eq!(busy.order_id, Some(dine_in.id));
    assert_eq!(busy.status, "occupied");
    assert!(busy.opened_at.is_some());
    assert_eq!(busy.line_count, 1);
    assert_eq!(records[0].order_id, None);
    assert_eq!(records[0].opened_at, None);
    assert_eq!(records[0].status, "free");

    // A table whose provisional bill was printed.
    let printed = EngineTableInfo {
        table_number: 9,
        order_id: Some(77),
        total_cents: 4200,
        line_count: 3,
        status: "occupied".into(),
        opened_at: Some("2026-10-05T18:30:00+00:00".into()),
        pre_bill_printed: true,
    };
    let record: ffi::TableInfo = out_equal(&printed);
    assert!(record.pre_bill_printed);
    assert_eq!(
        record.opened_at.as_deref(),
        Some("2026-10-05T18:30:00+00:00")
    );

    let takeaways = do_list_open_takeaways(&app.state).unwrap();
    assert_eq!(takeaways.len(), 1);
    let records: Vec<ffi::TakeawayInfo> = out_equal(&takeaways);
    assert_eq!(records[0].order_id, takeaway.id);
    assert_eq!(records[0].line_count, 1);
    let hand_built = EngineTakeawayInfo {
        order_id: 5,
        total_cents: 1,
        line_count: 2,
        created_at: "2026-10-05 18:30:00".into(),
    };
    let record: ffi::TakeawayInfo = out_equal(&hand_built);
    assert_eq!(record.created_at, "2026-10-05 18:30:00");
}

#[tokio::test]
async fn session_answers_convert() {
    let app = app().await;
    // A real answer: no session rules to apply, so no order error.
    let outcome = do_lock_session(&app.state).await.unwrap();
    let record: ffi::LockOutcome = out_equal(&outcome);
    assert_eq!(record.order_error, None);
    // An order error carried by the answer.
    let failed = EngineLockOutcome {
        order_error: Some("fdm_unreachable".into()),
    };
    let record: ffi::LockOutcome = out_equal(&failed);
    assert_eq!(record.order_error.as_deref(), Some("fdm_unreachable"));
}

#[test]
fn the_enums_keep_their_serde_names_and_variants() {
    enum_both::<EngineOrderType, ffi::OrderType>(&[
        EngineOrderType::DineIn,
        EngineOrderType::Takeaway,
    ]);
    enum_both::<EngineOrderStatus, ffi::OrderStatus>(&[
        EngineOrderStatus::Open,
        EngineOrderStatus::Closed,
        EngineOrderStatus::Voided,
    ]);
    enum_both::<EngineLeaveReason, ffi::LeaveReason>(&[
        EngineLeaveReason::Navigate,
        EngineLeaveReason::SwitchOrder,
        EngineLeaveReason::SendToKitchen,
        EngineLeaveReason::Lock,
        EngineLeaveReason::Idle,
    ]);
    enum_both::<EngineVatCode, VatCode>(&[
        EngineVatCode::A,
        EngineVatCode::B,
        EngineVatCode::C,
        EngineVatCode::D,
        EngineVatCode::X,
    ]);
}

#[test]
fn a_record_refuses_a_field_the_engine_did_not_have() {
    let mut value = json_of(&ffi::LockOutcome { order_error: None });
    value["extra"] = Value::Bool(true);
    assert!(serde_json::from_value::<ffi::LockOutcome>(value).is_err());
    let mut value = json_of(&ffi::TakeawayInfo {
        order_id: 1,
        total_cents: 2,
        line_count: 3,
        created_at: "x".into(),
    });
    value["extra"] = Value::Bool(true);
    assert!(serde_json::from_value::<ffi::TakeawayInfo>(value).is_err());
}

// ───────────────────────────── input records ─────────────────────────────

#[test]
fn a_create_order_input_with_every_field_set_reaches_the_engine() {
    let input = ffi::CreateOrderInput {
        order_type: ffi::OrderType::DineIn,
        table_number: Some(12),
        user_insz: INSZ.to_string(),
    };
    let engine: EngineCreateOrderInput = contract_inp(&input).unwrap();
    assert_eq!(engine.order_type, EngineOrderType::DineIn);
    assert_eq!(engine.table_number, Some(12));
    assert_eq!(engine.user_insz, INSZ);

    let takeaway = ffi::CreateOrderInput {
        order_type: ffi::OrderType::Takeaway,
        table_number: None,
        user_insz: INSZ.to_string(),
    };
    let engine: EngineCreateOrderInput = contract_inp(&takeaway).unwrap();
    assert_eq!(engine.order_type, EngineOrderType::Takeaway);
    assert_eq!(engine.table_number, None);
}

#[test]
fn an_add_order_line_input_with_every_field_set_reaches_the_engine() {
    let input = ffi::AddOrderLineInput {
        order_id: 41,
        menu_item_id: 22010,
        quantity: 3,
        modifiers_json: Some(r#"[{"id":1}]"#.into()),
        unit_price_cents: Some(1550),
    };
    let engine: EngineAddOrderLineInput = contract_inp(&input).unwrap();
    assert_eq!(engine.order_id, 41);
    assert_eq!(engine.menu_item_id, 22010);
    assert_eq!(engine.quantity, 3);
    assert_eq!(engine.modifiers_json.as_deref(), Some(r#"[{"id":1}]"#));
    assert_eq!(engine.unit_price_cents, Some(1550));

    let bare = ffi::AddOrderLineInput {
        order_id: 1,
        menu_item_id: 2,
        quantity: 1,
        modifiers_json: None,
        unit_price_cents: None,
    };
    let engine: EngineAddOrderLineInput = contract_inp(&bare).unwrap();
    assert_eq!(engine.modifiers_json, None);
    assert_eq!(engine.unit_price_cents, None);
}

#[test]
fn an_add_with_modifiers_input_with_every_field_set_reaches_the_engine() {
    let input = ffi::AddWithModifiersInput {
        order_id: 8,
        menu_item_id: 9,
        quantity: 4,
        modifier_ids: vec![3, 1, 2],
    };
    let engine: modifiers::AddWithModifiersInput = contract_inp(&input).unwrap();
    assert_eq!(engine.order_id, 8);
    assert_eq!(engine.menu_item_id, 9);
    assert_eq!(engine.quantity, 4);
    assert_eq!(engine.modifier_ids, vec![3, 1, 2]);
}

#[test]
fn a_leave_reason_reaches_the_engine() {
    let engine: EngineLeaveReason = contract_inp(&ffi::LeaveReason::SendToKitchen).unwrap();
    assert_eq!(engine, EngineLeaveReason::SendToKitchen);
}

#[test]
fn the_helper_records_serialize_with_their_field_names() {
    let line = ffi::VatLine {
        vat_code: VatCode::B,
        unit_price_cents: 1250,
        quantity: 2,
    };
    assert_eq!(
        json_of(&line),
        serde_json::json!({"vat_code": "B", "unit_price_cents": 1250, "quantity": 2})
    );
    let row = ffi::VatSplitRow {
        vat_code: VatCode::A,
        rate_percent: 21,
        total_incl_cents: 1210,
        vat_cents: 210,
    };
    assert_eq!(
        json_of(&row),
        serde_json::json!({"vat_code": "A", "rate_percent": 21, "total_incl_cents": 1210, "vat_cents": 210})
    );
}

// ───────────────────────────── wiring through PosEngine ─────────────────────────────

/// The data the wiring tests read, added to the seed of the SPF test machine: a dine-in order at
/// table 3 with a line, an open takeaway with a line, and two option groups on item 22001.
const EXTRA_SQL: &str = "
INSERT INTO orders (id, table_number, order_type, status, user_insz, subtotal_cents, total_cents) VALUES
    (1, 3, 'dine_in', 'open', '94060242576', 2400, 2400),
    (2, NULL, 'takeaway', 'open', '94060242576', 800, 800);
INSERT INTO order_lines (id, order_id, menu_item_id, quantity, unit_price_cents, vat_code, name) VALUES
    (1, 1, 10006, 2, 1200, 'A', 'Dry Martini'),
    (2, 2, 15003, 1, 800, 'A', 'Wine (glass)');
INSERT INTO modifier_groups (id, min_select, max_select, menu_item_id) VALUES (1, 1, 1, 22001), (2, 0, -1, 22001);
INSERT INTO modifiers (id, group_id, price_cents, sort_order) VALUES (1, 1, 0, 0), (2, 1, 150, 1), (3, 2, 100, 0);
INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
    ('modifier_group', 1, 'name', 'fr', 'Cuisson'),
    ('modifier_group', 2, 'name', 'fr', 'Extras'),
    ('modifier', 1, 'name', 'fr', 'Saignant'),
    ('modifier', 2, 'name', 'fr', 'A point'),
    ('modifier', 3, 'name', 'fr', 'Frites');
";

/// Same as `support::open_seeded`, with [`EXTRA_SQL`] appended to the seed.
async fn open_with_orders() -> TestEngine {
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(fdm, pos_fdm_mock::app()).await.unwrap();
    });
    let printer_mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
    let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
        .await
        .unwrap()
        .to_string();

    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("pos.json"),
        format!(
            r#"{{
            "pos_id": "CPOS0031234567", "terminal_id": "POS-2-ANDROID", "device_id": "bundle-device",
            "restaurant_name": "Restaurant Test", "restaurant_address": "Rue de l'Exemple 1, 4000 Liege",
            "restaurant_vat": "BE0499999960", "restaurant_est_no": "2123456791",
            "fdm_url": "{fdm_url}", "printer_address": "{printer}", "table_count": 25
        }}"#
        ),
    )
    .unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    let extended = dir.path().join("seed_with_orders.sql");
    std::fs::write(
        &extended,
        format!("{}\n{EXTRA_SQL}", std::fs::read_to_string(seed).unwrap()),
    )
    .unwrap();
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&extended))
        .expect("seed the database");
    let engine =
        pos_ffi::PosEngine::open(options_for(dir.path())).unwrap_or_else(|e| panic!("open: {e}"));
    TestEngine {
        engine,
        dir,
        fdm_url,
        printer,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_order_reads_reach_the_engine() {
    let t = open_with_orders().await;
    let e = &t.engine;

    let order = e.get_order(1).await.unwrap();
    assert_eq!(order.id, 1);
    assert_eq!(order.table_number, Some(3));
    assert_eq!(order.order_type, ffi::OrderType::DineIn);
    assert_eq!(order.status, ffi::OrderStatus::Open);
    assert_eq!(order.user_insz, MANAGER_INSZ);
    assert_eq!(order.total_cents, 2400);

    let lines = e.list_order_lines(1).await.unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].menu_item_id, 10006);
    assert_eq!(lines[0].quantity, 2);
    assert_eq!(lines[0].unit_price_cents, 1200);
    assert_eq!(lines[0].vat_code, VatCode::A);

    let cart = e.list_cart_lines(1).await.unwrap();
    assert_eq!(cart.len(), 1);
    assert_eq!(cart[0].id, lines[0].id);
    assert_eq!(cart[0].name_fr.as_deref(), Some("Dry Martini"));
    assert_eq!(
        cart[0].unsent_quantity, 0,
        "no journal entry for a seeded line"
    );

    let open = e.list_open_orders().await.unwrap();
    assert_eq!(open.iter().map(|o| o.id).collect::<Vec<_>>(), vec![1, 2]);

    // An unknown order is not a coded error: the engine's text is kept as Internal.
    assert!(matches!(
        e.get_order(999).await,
        Err(PosError::Internal { .. })
    ));
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_menu_reads_reach_the_engine() {
    let t = open_with_orders().await;
    let e = &t.engine;

    let categories = e.get_categories(None).await.unwrap();
    let aperitifs = categories.iter().find(|c| c.id == 10).expect("category 10");
    assert_eq!(aperitifs.name, "Aperitifs");
    assert_eq!(aperitifs.color.as_deref(), Some("#E74C3C"));
    assert!(categories.iter().all(|c| c.id != 400), "hidden vouchers");
    // The language reaches the engine: an unknown one falls back to no translated name.
    let english = e.get_categories(Some("en".into())).await.unwrap();
    assert_eq!(english.len(), categories.len());

    let soft = e.get_menu_items(Some(12), Some("fr".into())).await.unwrap();
    let ids: Vec<i64> = soft.iter().map(|i| i.id).collect();
    assert_eq!(ids, vec![12007, 12009]);
    assert!(soft.iter().all(|i| i.category_id == 12));
    assert_eq!(soft[0].name, "Perrier");
    assert_eq!(soft[0].price_cents, 250);
    assert_eq!(soft[0].vat_code, VatCode::A);
    assert_eq!(soft[0].vat_code_takeaway, Some(VatCode::C));
    let all = e.get_menu_items(None, None).await.unwrap();
    assert!(all.len() > soft.len());
    let free = all.iter().find(|i| i.id == 22010).expect("free price item");
    assert!(free.open_price);

    let groups = e
        .get_item_modifiers(22001, Some("fr".into()))
        .await
        .unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "Cuisson");
    assert_eq!((groups[0].min, groups[0].max), (1, Some(1)));
    assert_eq!(
        groups[0]
            .options
            .iter()
            .map(|o| o.price_cents)
            .collect::<Vec<_>>(),
        vec![0, 150]
    );
    assert_eq!(groups[1].max, None);
    assert!(e.get_item_modifiers(10006, None).await.unwrap().is_empty());
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_table_reads_reach_the_engine() {
    let t = open_with_orders().await;
    let e = &t.engine;

    let tables = e.list_tables().await.unwrap();
    assert_eq!(tables.len(), 25, "table_count of pos.json");
    let busy = &tables[2];
    assert_eq!(busy.table_number, 3);
    assert_eq!(busy.order_id, Some(1));
    assert_eq!(busy.status, "occupied");
    assert_eq!(busy.total_cents, 2400);
    assert_eq!(busy.line_count, 1);
    assert!(busy.opened_at.is_some());
    assert!(!busy.pre_bill_printed);
    assert_eq!(tables[0].status, "free");
    assert_eq!(tables[0].order_id, None);

    let takeaways = e.list_open_takeaways().await.unwrap();
    assert_eq!(takeaways.len(), 1);
    assert_eq!(takeaways[0].order_id, 2);
    assert_eq!(takeaways[0].total_cents, 800);
    assert_eq!(takeaways[0].line_count, 1);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_session_methods_that_need_no_operator_reach_the_engine() {
    let t = open_with_orders().await;
    let e = &t.engine;
    e.user_activity().await.unwrap();
    // No session is open: nothing to register, nothing to close.
    let outcome = e.lock_session().await.unwrap();
    assert_eq!(outcome.order_error, None);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_commands_that_need_an_operator_answer_not_logged_in_and_change_nothing() {
    let t = open_with_orders().await;
    let e = &t.engine;

    // The operator's INSZ is checked before the session: the arguments reach the engine.
    let bad_insz = ffi::CreateOrderInput {
        order_type: ffi::OrderType::DineIn,
        table_number: Some(5),
        user_insz: "123".into(),
    };
    assert_eq!(
        code_of(e.create_order(bad_insz).await.unwrap_err()),
        "operator_invalid"
    );
    let good_insz = ffi::CreateOrderInput {
        order_type: ffi::OrderType::DineIn,
        table_number: Some(5),
        user_insz: MANAGER_INSZ.into(),
    };
    assert_eq!(
        code_of(e.create_order(good_insz).await.unwrap_err()),
        "not_logged_in"
    );

    let add = ffi::AddOrderLineInput {
        order_id: 1,
        menu_item_id: 10006,
        quantity: 1,
        modifiers_json: None,
        unit_price_cents: None,
    };
    assert_eq!(
        code_of(e.add_order_line(add).await.unwrap_err()),
        "not_logged_in"
    );
    let with_modifiers = ffi::AddWithModifiersInput {
        order_id: 1,
        menu_item_id: 22001,
        quantity: 1,
        modifier_ids: vec![1],
    };
    assert_eq!(
        code_of(
            e.add_order_line_with_modifiers(with_modifiers)
                .await
                .unwrap_err()
        ),
        "not_logged_in"
    );
    assert_eq!(
        code_of(e.remove_order_line(1).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(
        code_of(e.update_order_line_quantity(1, 4).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(
        code_of(e.set_active_order(Some(1)).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(
        code_of(
            e.leave_order(1, ffi::LeaveReason::Navigate)
                .await
                .unwrap_err()
        ),
        "not_logged_in"
    );
    assert_eq!(
        code_of(e.sign_table_order(1).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(
        code_of(e.sign_pre_bill(1).await.unwrap_err()),
        "not_logged_in"
    );
    assert_eq!(
        code_of(e.transfer_table(1, 4).await.unwrap_err()),
        "not_logged_in"
    );

    // Nothing was written.
    let lines = e.list_order_lines(1).await.unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].quantity, 2);
    assert_eq!(e.list_open_orders().await.unwrap().len(), 2);
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn every_method_answers_engine_closed_after_shutdown() {
    let t = open_with_orders().await;
    let e = &t.engine;
    e.shutdown().await.unwrap();
    assert!(matches!(e.get_order(1).await, Err(PosError::EngineClosed)));
    assert!(matches!(e.list_tables().await, Err(PosError::EngineClosed)));
    assert!(matches!(
        e.leave_order(1, ffi::LeaveReason::Idle).await,
        Err(PosError::EngineClosed)
    ));
    assert!(matches!(
        e.lock_session().await,
        Err(PosError::EngineClosed)
    ));
    assert!(matches!(
        e.user_activity().await,
        Err(PosError::EngineClosed)
    ));
}

// ───────────────────────────── vat_split ─────────────────────────────

fn line(vat_code: VatCode, unit_price_cents: i64, quantity: i64) -> ffi::VatLine {
    ffi::VatLine {
        vat_code,
        unit_price_cents,
        quantity,
    }
}

fn row(
    vat_code: VatCode,
    rate_percent: i64,
    total_incl_cents: i64,
    vat_cents: i64,
) -> ffi::VatSplitRow {
    ffi::VatSplitRow {
        vat_code,
        rate_percent,
        total_incl_cents,
        vat_cents,
    }
}

// The expected rows below come from running the formula of ui/src/components/VatSummary.vue in
// node (`Math.round`, doubles), not from this crate.

#[tokio::test]
async fn vat_split_reproduces_vat_summary_vue_on_a_mixed_order() {
    // Rows in the order A B C D, X left out, totals summed per code across lines.
    let lines = vec![
        line(VatCode::B, 1250, 2),
        line(VatCode::A, 304, 3),
        line(VatCode::C, 900, 1),
        line(VatCode::D, 500, 1),
        line(VatCode::X, 777, 1),
        line(VatCode::A, 250, 1),
    ];
    assert_eq!(
        vat_split(lines).await,
        vec![
            row(VatCode::A, 21, 1162, 202),
            row(VatCode::B, 12, 2500, 268),
            row(VatCode::C, 6, 900, 51),
            row(VatCode::D, 0, 500, 0),
        ]
    );
}

#[tokio::test]
async fn vat_split_rounds_a_half_cent_up() {
    // 14 at 12%: 14 * 10000 / 11200 = 12.5 exactly; Math.round gives 13, so the VAT is 1.
    assert_eq!(
        vat_split(vec![line(VatCode::B, 14, 1)]).await,
        vec![row(VatCode::B, 12, 14, 1)]
    );
    // The same tie on a negative total: Math.round(-12.5) is -12, so the VAT is -2.
    assert_eq!(
        vat_split(vec![line(VatCode::B, -14, 1)]).await,
        vec![row(VatCode::B, 12, -14, -2)]
    );
}

#[tokio::test]
async fn vat_split_handles_negative_totals() {
    // -100 at 21%: -1000000 / 12100 = -82.64, Math.round gives -83, the VAT is -17.
    assert_eq!(
        vat_split(vec![line(VatCode::A, -100, 1)]).await,
        vec![row(VatCode::A, 21, -100, -17)]
    );
    // A negative quantity (a correction) is the same sum; -1210 at 21% is exact.
    assert_eq!(
        vat_split(vec![line(VatCode::A, 1210, -1)]).await,
        vec![row(VatCode::A, 21, -1210, -210)]
    );
    // Lines of one code cancel into a smaller total.
    assert_eq!(
        vat_split(vec![line(VatCode::C, 1060, 2), line(VatCode::C, -1060, 1)]).await,
        vec![row(VatCode::C, 6, 1060, 60)]
    );
}

#[tokio::test]
async fn vat_split_can_differ_from_vat_from_inclusive_by_a_cent() {
    // The facade rounds like the screen; `vat_from_inclusive` truncates. Display only (PD62).
    let cases = [
        // (code, total, split VAT from node, vat_from_inclusive)
        (VatCode::A, EngineVatCode::A, 100, 17, 18),
        (VatCode::B, EngineVatCode::B, 14, 1, 2),
        (VatCode::A, EngineVatCode::A, -100, -17, -18),
    ];
    for (code, engine_code, total, vat, truncated) in cases {
        let rows = vat_split(vec![line(code, total, 1)]).await;
        assert_eq!(rows[0].vat_cents, vat, "split of {total}");
        assert_eq!(
            engine_code.vat_from_inclusive(total),
            truncated,
            "vat_from_inclusive of {total}"
        );
        assert_ne!(rows[0].vat_cents, truncated);
    }
    // Where both agree (exact amounts) there is no difference.
    let rows = vat_split(vec![line(VatCode::A, 1210, 1)]).await;
    assert_eq!(rows[0].vat_cents, EngineVatCode::A.vat_from_inclusive(1210));
}

#[tokio::test]
async fn vat_split_shows_only_the_codes_present() {
    assert!(vat_split(vec![]).await.is_empty());
    assert!(vat_split(vec![line(VatCode::X, 100, 1)]).await.is_empty());
    // A code with a zero total is still present, as in the screen.
    assert_eq!(
        vat_split(vec![line(VatCode::B, 100, 0)]).await,
        vec![row(VatCode::B, 12, 0, 0)]
    );
    // D has a rate of 0: no VAT.
    assert_eq!(
        vat_split(vec![line(VatCode::D, 840, 2)]).await,
        vec![row(VatCode::D, 0, 1680, 0)]
    );
}
