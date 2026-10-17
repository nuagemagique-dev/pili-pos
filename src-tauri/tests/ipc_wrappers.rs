//! The IPC wrappers of the table transfer, the payment correction, the
//! till discounts and the kitchen note, called as the webview calls
//! them: the command name and the JSON arguments `ui/src/api/ipc.ts`
//! builds (camelCase keys, the discount input and the payment method in
//! their serde form), through Tauri's mock runtime, on a real engine.
//!
//! Each case asserts an engine answer that depends on the arguments
//! (a refusal code, a stored note): it proves the arguments reached the
//! engine under the names the UI uses, and that the wrapper made the
//! single engine call.
//!
//! Needs the `hubrise` feature (the discount wrappers), like every
//! shipped build.

#![cfg(feature = "hubrise")]

use std::path::PathBuf;
use std::sync::Arc;

use pili_pos::events::TauriEventSink;
use pili_pos::ipc;
use pos_core::order::OrderType;
use pos_core::user::{User, UserRole};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::engine::{ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers};
use pos_engine::{HttpTls, SoftwareIdentity};
use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::MockRuntime;
use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};

const MANAGER: &str = "12345678958";
const PIN: &str = "4321";

/// A data directory removed when the test ends.
struct DataDir(PathBuf);

impl Drop for DataDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Fixture {
    window: WebviewWindow<MockRuntime>,
    // Keeps the app (and the engine it manages) alive.
    app: tauri::App<MockRuntime>,
    _dir: DataDir,
}

impl Fixture {
    fn engine(&self) -> tauri::State<'_, Engine> {
        use tauri::Manager;
        self.app.state::<Engine>()
    }

    /// Call `cmd` with the JSON `args` the UI sends.
    fn call(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        let response = get_ipc_response(
            &self.window,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                // The app's own origin (a remote URL is refused by the ACL).
                url: "tauri://localhost".parse().unwrap(),
                body: InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )?;
        Ok(response.deserialize::<Value>().expect("a JSON answer"))
    }
}

/// A real engine on an empty database, the manager logged in with the PIN
/// 4321, a Sake at 6,50 on the menu, and the six wrappers registered.
fn fixture(name: &str) -> Fixture {
    let dir = std::env::temp_dir().join(format!("pili-ipc-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let guard = DataDir(dir.clone());

    let engine = Engine::open(EngineConfig {
        paths: EnginePaths::under(dir),
        config: ConfigSource::DevelopmentDefaults,
        events: Arc::new(TauriEventSink::new()),
        timers: EngineTimers::default(),
        identity: SoftwareIdentity::new("pili-pos/ipc-test").unwrap(),
        http_tls: HttpTls::Platform,
        device_id_override: None,
        support_program: None,
    })
    .expect("open the engine");
    {
        let db = engine.db.lock().unwrap();
        db.execute_batch(
            "INSERT INTO users (insz, name, role, active) VALUES ('12345678958', 'Gerant', 'admin', 1);
             INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active)
                VALUES (1, 1, 650, 'A', 0, 0, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value)
                VALUES ('menu_item', 1, 'name', 'fr', 'Sake');",
        )
        .unwrap();
    }
    *engine.session.lock().unwrap() = Some(User {
        insz: MANAGER.into(),
        name: "Gerant".into(),
        role: UserRole::Admin,
        active: true,
        has_pin: false,
        presence_only: false,
    });
    pos_engine::commands::auth::do_set_pin(&engine, MANAGER.into(), PIN.into(), None).unwrap();

    let app = mock_builder()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![
            ipc::tables::transfer_table,
            ipc::financial::correct_payment,
            ipc::financial::list_sale_payments,
            ipc::discount::apply_discount,
            ipc::discount::remove_discount,
            ipc::order::set_line_note,
        ])
        .build(mock_context(noop_assets()))
        .expect("the mock app");
    let window = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("the mock window");
    Fixture {
        window,
        app,
        _dir: guard,
    }
}

/// A dine-in order on `table` with one Sake: `(order id, line id)`.
fn order_with_a_line(f: &Fixture, table: i32) -> (i64, i64) {
    let engine = f.engine();
    let order = do_create_order(
        &engine,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(table),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap();
    let line = do_add_order_line(
        &engine,
        AddOrderLineInput {
            order_id: order.id,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    (order.id, line.id)
}

fn refusal(result: Result<Value, Value>) -> String {
    match result {
        Err(Value::String(code)) => code,
        other => panic!("expected a refusal code, got {other:?}"),
    }
}

#[test]
fn transfer_table_takes_orderid_and_totable() {
    let f = fixture("transfer");
    let (order, _) = order_with_a_line(&f, 4);

    // Both arguments reached the engine: the order is found, and its
    // table is compared with `toTable`.
    assert_eq!(
        refusal(f.call("transfer_table", json!({ "orderId": order, "toTable": 4 }))),
        "table_transfer_same_table"
    );
    assert_eq!(
        refusal(f.call(
            "transfer_table",
            json!({ "orderId": order, "toTable": 9999 })
        )),
        "table_transfer_target_invalid"
    );
    assert_eq!(
        refusal(f.call("transfer_table", json!({ "orderId": 424242, "toTable": 5 }))),
        "order_not_open"
    );
    // The UI's names are the contract: snake_case keys are not accepted.
    let wrong = f
        .call(
            "transfer_table",
            json!({ "order_id": order, "to_table": 5 }),
        )
        .unwrap_err();
    assert!(wrong.to_string().contains("orderId"), "{wrong}");
}

#[test]
fn apply_discount_takes_the_input_the_ui_builds() {
    let f = fixture("apply");
    let (order, line) = order_with_a_line(&f, 4);
    let free_line = json!({
        "target": { "type": "line", "line_id": line + 1000 },
        "kind": "free",
        "reason_code": "staff_meal",
    });

    // The PIN is checked first, with the INSZ and PIN the UI sends.
    assert_eq!(
        refusal(f.call(
            "apply_discount",
            json!({ "orderId": order, "input": free_line, "insz": MANAGER, "pin": "0000" })
        )),
        "pin_wrong"
    );
    // Then the nested input: a line that is not on the order.
    assert_eq!(
        refusal(f.call(
            "apply_discount",
            json!({ "orderId": order, "input": free_line, "insz": MANAGER, "pin": PIN })
        )),
        "line_not_found"
    );
    // A percent out of range is refused by the engine, so `kind` and
    // `value` were read.
    let bad_percent = json!({
        "target": { "type": "order" },
        "kind": "percent",
        "value": 100,
        "reason_code": "commercial_gesture",
        "reason_text": null,
    });
    assert_eq!(
        refusal(f.call(
            "apply_discount",
            json!({ "orderId": order, "input": bad_percent, "insz": MANAGER, "pin": PIN })
        )),
        "discount_invalid"
    );
    // A real discount comes back as a DiscountResult in camelCase.
    let ten = json!({
        "target": { "type": "order" },
        "kind": "percent",
        "value": 10,
        "reason_code": "commercial_gesture",
    });
    let done = f
        .call(
            "apply_discount",
            json!({ "orderId": order, "input": ten, "insz": MANAGER, "pin": PIN }),
        )
        .unwrap();
    assert_eq!(done["order"]["total_cents"], 585);
    assert_eq!(done["kitchenSlip"], "nothing_to_print");
    assert_eq!(done["registrationError"], Value::Null);
}

#[test]
fn remove_discount_takes_orderid_lineid_insz_and_pin() {
    let f = fixture("remove");
    let (order, line) = order_with_a_line(&f, 4);

    assert_eq!(
        refusal(f.call(
            "remove_discount",
            json!({ "orderId": order, "lineId": null, "insz": MANAGER, "pin": "0000" })
        )),
        "pin_wrong"
    );
    // Nothing to remove, for the order and for a line.
    assert_eq!(
        refusal(f.call(
            "remove_discount",
            json!({ "orderId": order, "lineId": null, "insz": MANAGER, "pin": PIN })
        )),
        "discount_none"
    );
    assert_eq!(
        refusal(f.call(
            "remove_discount",
            json!({ "orderId": order, "lineId": line, "insz": MANAGER, "pin": PIN })
        )),
        "discount_none"
    );

    // Give the discount the UI way, then remove it the UI way.
    let ten = json!({
        "target": { "type": "order" },
        "kind": "percent",
        "value": 10,
        "reason_code": "commercial_gesture",
    });
    f.call(
        "apply_discount",
        json!({ "orderId": order, "input": ten, "insz": MANAGER, "pin": PIN }),
    )
    .unwrap();
    let removed = f
        .call(
            "remove_discount",
            json!({ "orderId": order, "lineId": null, "insz": MANAGER, "pin": PIN }),
        )
        .unwrap();
    assert_eq!(removed["order"]["total_cents"], 650);
}

#[test]
fn set_line_note_takes_lineid_and_note() {
    let f = fixture("note");
    let (_, line) = order_with_a_line(&f, 4);

    // The note argument is validated by the engine.
    assert_eq!(
        refusal(f.call(
            "set_line_note",
            json!({ "lineId": line, "note": "x".repeat(61) })
        )),
        "note_too_long"
    );
    // A valid note is stored; `null` clears it.
    let outcome = f
        .call(
            "set_line_note",
            json!({ "lineId": line, "note": "sans wasabi" }),
        )
        .unwrap();
    assert_eq!(outcome, json!("nothing_to_print"));
    let stored = |f: &Fixture| -> Option<String> {
        f.engine()
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT kitchen_note FROM order_lines WHERE id = ?1",
                [line],
                |r| r.get(0),
            )
            .unwrap()
    };
    assert_eq!(stored(&f).as_deref(), Some("sans wasabi"));
    f.call("set_line_note", json!({ "lineId": line, "note": null }))
        .unwrap();
    assert_eq!(stored(&f), None);
}

#[test]
fn correct_payment_takes_the_method_in_its_serde_form() {
    let f = fixture("correct");

    // A wrong PIN comes back as `success: false` with the code, so the
    // method deserialised and the arguments reached the engine.
    for method in ["cash", "card", "meal_voucher"] {
        let answer = f
            .call(
                "correct_payment",
                json!({
                    "fiscalEventId": 1,
                    "lineIndex": 0,
                    "method": method,
                    "insz": MANAGER,
                    "pin": "0000",
                }),
            )
            .unwrap();
        assert_eq!(answer["success"], false, "{method}");
        assert_eq!(answer["error"], "pin_wrong", "{method}");
    }
    // With the PIN right, the sale is looked up: there is none.
    let answer = f
        .call(
            "correct_payment",
            json!({
                "fiscalEventId": 424242,
                "lineIndex": 0,
                "method": "cash",
                "insz": MANAGER,
                "pin": PIN,
            }),
        )
        .unwrap();
    assert_eq!(answer["success"], false);
    assert_ne!(answer["error"], "pin_wrong");
    assert!(answer["error"].is_string(), "{answer}");
    // A method the UI never sends is refused at the boundary.
    assert!(
        f.call(
            "correct_payment",
            json!({
                "fiscalEventId": 1, "lineIndex": 0, "method": "bitcoin",
                "insz": MANAGER, "pin": PIN,
            })
        )
        .is_err()
    );
}

#[test]
fn list_sale_payments_takes_fiscaleventid() {
    let f = fixture("list");
    // The id reached the engine: no such sale.
    let code = refusal(f.call("list_sale_payments", json!({ "fiscalEventId": 424242 })));
    assert!(!code.is_empty());
    let wrong = f
        .call("list_sale_payments", json!({ "fiscal_event_id": 1 }))
        .unwrap_err();
    assert!(wrong.to_string().contains("fiscalEventId"), "{wrong}");
}
