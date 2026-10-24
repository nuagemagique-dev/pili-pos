//! SPF Finances Advanced Set v1.6, multi-purpose vouchers (UC260 sale,
//! UC261 and UC262 use), replayed from the official fixtures.
//!
//! Same method as `spf_e2e_fixtures.rs`: the order is built through the
//! commands the UI uses, the real `do_*` command runs against a mock FDM
//! that replays the fixture's response, and the request pili-pos built is
//! compared with the fixture's, field by field. The payment lines
//! (`VOUCHER_STORE`, `MANUAL`, the amounts) must match exactly; every
//! other difference is listed with its reason, and an unlisted one fails
//! the test, as does a listed one that no longer occurs.
//!
//! Deferred (V12-D24): UC270 and UC271 (single-purpose vouchers) and
//! UC4105 (voucher to cash); `scripts/uc_scope.toml` says why.

use pos_engine_testkit as common;

use common::*;
use serde_json::{Value, json};

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, PaymentPart, do_checkout_split};
use pos_engine::commands::gift_vouchers::{
    GiftVoucherSettings, do_add_gift_voucher_line, do_set_gift_voucher_settings,
};
use pos_engine::commands::order::{AddOrderLineInput, do_add_order_line};
use pos_engine::commands::tables::do_sign_table_order;

// ── Setup ──────────────────────────────────────────────────────────

/// A TestApp with the manual's catalogue, the fixture's operator, and
/// gift vouchers switched on.
async fn spf_app(operator_insz: &str) -> TestApp {
    let app = TestApp::new().await;
    {
        let conn = app.state.db.lock().unwrap();
        pos_engine::install::apply_seed(
            &conn,
            include_str!("../../../src-tauri/seed_spf_test.sql"),
        )
        .expect("seed");
        // The menu item is sold as one product (the seed keeps it off the
        // menu: composite products are not offered).
        conn.execute_batch(
            "UPDATE menu_items SET active = 1 WHERE id = 30010;
             UPDATE categories SET active = 1 WHERE id = 30;",
        )
        .unwrap();
    }
    app.login_as(operator_insz, pos_core::user::UserRole::Admin);
    do_set_gift_voucher_settings(
        &app.state,
        GiftVoucherSettings {
            enabled: true,
            ..GiftVoucherSettings::default()
        },
    )
    .unwrap();
    app
}

fn variables_of(fixture: &UcFixture) -> Value {
    read_variables(fixture)["data"].clone()
}

fn operator(vars: &Value) -> String {
    vars["employeeId"].as_str().expect("employeeId").to_string()
}

/// An order of the fixture's products, through the UI's command.
fn order_from(app: &TestApp, vars: &Value, order_type: OrderType, table: Option<i32>) -> i64 {
    let order = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::create_order(&conn, order_type, table, &operator(vars))
            .expect("create order")
    };
    for line in vars["transaction"]["transactionLines"].as_array().unwrap() {
        let product = &line["mainProduct"];
        let menu_item_id: i64 = product["productId"].as_str().unwrap().parse().unwrap();
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order.id,
                menu_item_id,
                quantity: product["quantity"].as_f64().unwrap() as i32,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .expect("add order line");
    }
    order.id
}

/// The archived requests, oldest first.
fn requests(app: &TestApp) -> Vec<Value> {
    app.fiscal_events()
        .iter()
        .filter_map(|e| e.event_data_json.as_deref())
        .map(|j| serde_json::from_str(j).unwrap())
        .collect()
}

struct Deviation {
    path: &'static str,
    reason: &'static str,
}

/// Compare the business fields of `ours` with the fixture's.
fn assert_matches_fixture(uc: &str, ours: &Value, fixture: &Value, allowed: &[Deviation]) {
    let mut found = Vec::new();
    diff(
        "",
        &business_view(ours),
        &business_view(fixture),
        &mut found,
    );
    let unexpected: Vec<_> = found
        .iter()
        .filter(|(p, _, _)| !allowed.iter().any(|d| d.path == p))
        .collect();
    let stale: Vec<_> = allowed
        .iter()
        .filter(|d| !found.iter().any(|(p, _, _)| p == d.path))
        .map(|d| (d.path, d.reason))
        .collect();
    if let Ok(dir) = std::env::var("UC_EVIDENCE_DIR") {
        let deviations: Vec<Value> = found
            .iter()
            .map(|(path, o, f)| {
                let reason = allowed.iter().find(|d| d.path == path).map(|d| d.reason);
                json!({ "path": path, "piliPos": o, "fixture": f, "reason": reason })
            })
            .collect();
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join(format!("{uc}_check.json")),
            serde_json::to_string_pretty(&json!({ "uc": uc, "deviations": deviations })).unwrap(),
        )
        .unwrap();
    }
    assert!(
        unexpected.is_empty(),
        "{uc}: request differs from the fixture: {unexpected:#?}"
    );
    assert!(
        stale.is_empty(),
        "{uc}: listed deviations no longer occur: {stale:?}"
    );
}

/// The payment lines of a request as `(type, inputMethod, amountType, amount)`.
fn payment_lines(request: &Value) -> Vec<(String, String, String, f64)> {
    request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["type"].as_str().unwrap().to_string(),
                f["inputMethod"].as_str().unwrap().to_string(),
                f["amountType"].as_str().unwrap().to_string(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect()
}

fn pay_line(t: &str, input: &str, a: f64) -> (String, String, String, f64) {
    (t.into(), input.into(), "PAYMENT".into(), a)
}

/// The voucher line is the gift voucher product, not a catalogue product.
const VOUCHER_PRODUCT: Deviation = Deviation {
    path: "transaction.transactionLines[0].productId",
    reason: "the voucher is the system product 'Bon cadeau' (990001), not the test setup's 40001",
};
const VOUCHER_DEPARTMENT: Deviation = Deviation {
    path: "transaction.transactionLines[0].departmentId",
    reason: "the voucher line is in department GV (Bons cadeaux), not the test setup's 400",
};
/// Sell a voucher of 50 EUR (the UC260 sale, paid by card) and return the
/// voucher's id.
async fn sell_fifty(app: &TestApp, uc260: &UcFixture) -> i64 {
    let order = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::create_order(
            &conn,
            OrderType::Takeaway,
            None,
            &operator(&variables_of(uc260)),
        )
        .unwrap()
        .id
    };
    do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    mock_from_fixture(app, uc260).await;
    let r = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::of(CheckoutPaymentMethod::Card, 5000)],
        None,
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::gift_vouchers::list(&conn, 10).unwrap()[0].id
}

// ── UC260: sale of a multi-purpose voucher ─────────────────────────

/// UC260: a 50,00 EUR voucher is sold for the card: one line at VAT X,
/// one payment line. The voucher exists once the sale is signed.
#[tokio::test]
// covers: UC260
async fn uc260_mpv_sale() {
    let fixture = load_uc(&advanced_set_root(), "UC260");
    let vars = variables_of(&fixture);
    let app = spf_app(&operator(&vars)).await;
    let id = sell_fifty(&app, &fixture).await;

    let ours = &requests(&app)[0];
    assert_matches_fixture("UC260", ours, &vars, &[VOUCHER_PRODUCT, VOUCHER_DEPARTMENT]);
    // VAT X, the whole 50,00 EUR, quantity 1.
    let line = &ours["transaction"]["transactionLines"][0]["mainProduct"];
    assert_eq!(line["vats"], json!([{ "label": "X", "price": 50.0 }]));
    assert_eq!(line["quantity"], 1.0);
    // A card typed by hand is MANUAL (the fixture's EFT card is
    // AUTOMATIC, a terminal-linked payment tested in eft_signing.rs).
    assert_eq!(
        payment_lines(ours),
        [pay_line("CARD_DEBIT", "MANUAL", 50.0)]
    );

    let conn = app.state.db.lock().unwrap();
    let v = pos_db::queries::gift_vouchers::get(&conn, id)
        .unwrap()
        .unwrap();
    assert_eq!((v.face_value_cents, v.balance_cents), (5000, 5000));
    drop(conn);
    app.dump_evidence("UC260");
}

// ── UC261: a business menu paid by voucher and cash ────────────────

/// UC261: two business menus (116,00 EUR) ordered on table 1, then paid
/// with a multi-purpose voucher (50,00 EUR) and 66,00 EUR in cash. The
/// voucher payment is `VOUCHER_STORE` / `MANUAL`, exact.
#[tokio::test]
// covers: UC261
async fn uc261_mpv_use() {
    let sale_voucher = load_uc(&advanced_set_root(), "UC260");
    let order_uc = load_uc(&advanced_set_root(), "UC261_MPV_Use/UC261_1");
    let sale_uc = load_uc(&advanced_set_root(), "UC261_MPV_Use/UC261_2");
    let order_vars = variables_of(&order_uc);
    let sale_vars = variables_of(&sale_uc);
    let app = spf_app(&operator(&order_vars)).await;
    let voucher_id = sell_fifty(&app, &sale_voucher).await;

    let order_id = order_from(&app, &order_vars, OrderType::DineIn, Some(1));
    mock_from_fixture(&app, &order_uc).await;
    do_sign_table_order(&app.state, order_id).await.unwrap();

    app.login_as(&operator(&sale_vars), pos_core::user::UserRole::Admin);
    mock_from_fixture(&app, &sale_uc).await;
    let result = do_checkout_split(
        &app.state,
        order_id,
        &[
            PaymentPart::gift_voucher(5000, voucher_id),
            PaymentPart::cash(),
        ],
        None,
    )
    .await
    .unwrap();
    assert!(result.success, "{:?}", result.error);

    // The menu is one product here, not a composite of its dishes: its
    // VAT is on the line itself (as in UC4109).
    let menu = [
        Deviation {
            path: "transaction.transactionLines[0].lineType",
            reason: "composite products are not supported: the menu is a single product",
        },
        Deviation {
            path: "transaction.transactionLines[0].vats",
            reason: "a single product carries its own VAT line (B 116,00)",
        },
    ];
    let r = requests(&app);
    let order = &r[1];
    let sale = &r[2];
    assert_matches_fixture("UC261_1", order, &order_vars, &menu);
    assert_matches_fixture("UC261_2", sale, &sale_vars, &menu);
    assert_eq!(
        payment_lines(sale),
        [
            pay_line("VOUCHER_STORE", "MANUAL", 50.0),
            pay_line("CASH", "MANUAL", 66.0),
        ]
    );
    assert_eq!(sale["transaction"]["transactionTotal"], 116.0);

    // The voucher is spent: 50,00 EUR redeemed, one `redeem` movement.
    let conn = app.state.db.lock().unwrap();
    let v = pos_db::queries::gift_vouchers::get(&conn, voucher_id)
        .unwrap()
        .unwrap();
    assert_eq!(v.balance_cents, 0);
    let moves = pos_db::queries::gift_vouchers::movements(&conn, voucher_id).unwrap();
    assert_eq!(
        moves
            .iter()
            .map(|m| (m.kind.as_str(), m.amount_cents))
            .collect::<Vec<_>>(),
        [("issue", 5000), ("redeem", -5000)]
    );
    drop(conn);
    app.dump_evidence("UC261");
}

// ── UC262: a sale paid entirely by voucher ─────────────────────────

/// UC262: a Dry Martini (12,00), a Burger of the chef (28,00) and a
/// Perrier (2,50) paid with one multi-purpose voucher, 42,50 EUR in one
/// payment line, no cash, no rounding.
#[tokio::test]
// covers: UC262
async fn uc262_mpv_use() {
    let sale_voucher = load_uc(&advanced_set_root(), "UC260");
    let sale_uc = load_uc(&advanced_set_root(), "UC262");
    let vars = variables_of(&sale_uc);
    let app = spf_app(&operator(&vars)).await;
    let voucher_id = sell_fifty(&app, &sale_voucher).await;

    let order_id = order_from(&app, &vars, OrderType::Takeaway, None);
    mock_from_fixture(&app, &sale_uc).await;
    let result = do_checkout_split(
        &app.state,
        order_id,
        &[PaymentPart::gift_voucher(4250, voucher_id)],
        None,
    )
    .await
    .unwrap();
    assert!(result.success, "{:?}", result.error);

    let sale = &requests(&app)[1];
    assert_matches_fixture(
        "UC262",
        sale,
        &vars,
        &[Deviation {
            path: "transaction.transactionLines[1].vats[0].label",
            reason: "an order is dine-in or takeaway as a whole: sold as a takeaway, the burger takes the takeaway label C (the fixture mixes a room burger B and a takeaway Perrier C on one ticket)",
        }],
    );
    assert_eq!(
        payment_lines(sale),
        [pay_line("VOUCHER_STORE", "MANUAL", 42.5)]
    );
    // 50,00 - 42,50 left on the voucher.
    let conn = app.state.db.lock().unwrap();
    let v = pos_db::queries::gift_vouchers::get(&conn, voucher_id)
        .unwrap()
        .unwrap();
    assert_eq!(v.balance_cents, 750);
    drop(conn);
    app.dump_evidence("UC262");
}
