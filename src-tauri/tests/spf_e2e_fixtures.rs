//! SPF Finances Basic Set v1.7, replayed from the official fixtures.
//!
//! Each test:
//!
//! 1. seeds the manual's predefined products (`seed_spf_test.sql`);
//! 2. builds the order from the fixture's `GQL-Variables.json`
//!    (products and quantities only) through the same command the UI
//!    uses (`do_add_order_line`), so prices, VAT labels and
//!    departments come from the catalogue;
//! 3. runs the real `do_*` command against a mock FDM that replays the
//!    fixture's response;
//! 4. compares the request moyun-pos built with the fixture, field by
//!    field (the "Champs à vérifier" of manual §IV).
//!
//! Business fields (products, quantities, prices, VAT, totals,
//! payments, cost-center type, referenced tickets, ticket medium) must
//! be equal. Identification fields that belong to the installation
//! (posId, deviceId, ticket number, dates, …) are checked for format
//! only. Names are free text and checked for presence.
//!
//! A difference that is intended is listed in the test with its reason
//! (`Deviation`); an unlisted difference fails the test, and so does a
//! listed one that no longer occurs.
//!
//! With `UC_EVIDENCE_DIR` set, each test writes the archived request and
//! response plus a `<UC>_check.json` comparison report for the dossier
//! (`scripts/capture_uc_evidence.sh`).

mod common;

use common::*;
use serde_json::{Value, json};

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::copy::do_copy_ticket;
use moyun_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use moyun_pos::commands::order::{AddOrderLineInput, do_add_order_line};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::social::{do_work_in, do_work_out};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;

// ── Setup ──────────────────────────────────────────────────────────

/// A TestApp with the manual's catalogue and the fixture's operator.
async fn spf_app(operator_insz: &str) -> TestApp {
    let app = TestApp::new().await;
    {
        let conn = app.state.db.lock().unwrap();
        moyun_pos::cli::apply_seed(&conn, include_str!("../seed_spf_test.sql")).expect("seed");
    }
    // The fixture's operator is logged in, as a manager: the UCs cover
    // manager-only events (refund, copy, cash, reports).
    app.login_as(operator_insz, pos_core::user::UserRole::Admin);
    app
}

fn set_training_mode(app: &TestApp, on: bool) {
    app.state.config.lock().unwrap().training_mode = on;
}

/// The `data` object of a fixture's Variables file.
fn variables(uc: &str) -> Value {
    read_variables(&load_uc(&basic_set_root(), uc))["data"].clone()
}

fn operator(vars: &Value) -> String {
    vars["employeeId"].as_str().expect("employeeId").to_string()
}

/// Create an order and add the fixture's products through the UI's
/// command. Only positive quantities are ordered (refund lines are
/// produced by the refund command).
fn order_from(app: &TestApp, vars: &Value, order_type: OrderType, table: Option<i32>) -> i64 {
    let order = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::create_order(&conn, order_type, table, &operator(vars))
            .expect("create order")
    };
    for line in vars["transaction"]["transactionLines"].as_array().unwrap() {
        let product = &line["mainProduct"];
        let quantity = product["quantity"].as_f64().unwrap() as i32;
        if quantity <= 0 {
            continue;
        }
        let menu_item_id: i64 = product["productId"].as_str().unwrap().parse().unwrap();
        // Free-price products take the price the operator types in.
        let open_price: bool = app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT open_price FROM menu_items WHERE id = ?1",
                [menu_item_id],
                |r| r.get(0),
            )
            .unwrap();
        let unit_price_cents =
            open_price.then(|| (product["unitPrice"].as_f64().unwrap() * 100.0).round() as i64);
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order.id,
                menu_item_id,
                quantity,
                modifiers_json: None,
                unit_price_cents,
            },
        )
        .expect("add order line");
    }
    order.id
}

/// Mount a fixture response, optionally overriding its `fdmRef` so a
/// later event of the same UC must reference exactly this ticket.
async fn mock_uc(app: &TestApp, uc: &str, fdm_ref: Option<&Value>) {
    let mut envelope = read_response_envelope(&load_uc(&basic_set_root(), uc));
    if let Some(fdm_ref) = fdm_ref {
        let data = envelope["data"].as_object_mut().unwrap();
        let (_, result) = data.iter_mut().next().expect("mutation result");
        result["fdmRef"] = fdm_ref.clone();
    }
    mock_response(app, envelope).await;
}

/// The archived requests built by moyun-pos, oldest first. Also checks
/// that each event is archived under the label the FDM signed it with.
fn requests(app: &TestApp) -> Vec<Value> {
    for e in app.fiscal_events() {
        assert_eq!(
            Some(e.event_type.as_str()),
            e.event_label.as_deref(),
            "event {} archived as {} but signed by the FDM as {:?}",
            e.id,
            e.event_type,
            e.event_label
        );
    }
    app.fiscal_events()
        .iter()
        .filter_map(|e| e.event_data_json.as_deref())
        .map(|j| serde_json::from_str(j).unwrap())
        .collect()
}

// ── Comparison ─────────────────────────────────────────────────────

/// An intended difference between moyun-pos and the fixture.
struct Deviation {
    path: &'static str,
    reason: &'static str,
}

/// The business content of a request, independent of the installation.
fn business_view(d: &Value) -> Value {
    let mut view = serde_json::Map::new();
    view.insert("ticketMedium".into(), d["ticketMedium"].clone());
    if let Some(t) = d.get("transaction") {
        let lines: Vec<Value> = t["transactionLines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                let p = &l["mainProduct"];
                json!({
                    "lineType": l["lineType"],
                    "productId": p["productId"],
                    "departmentId": p["departmentId"],
                    "quantity": p["quantity"],
                    "quantityType": p["quantityType"],
                    "negQuantityReason": p.get("negQuantityReason").cloned().unwrap_or(Value::Null),
                    "unitPrice": p["unitPrice"],
                    "vats": p["vats"].as_array().unwrap().iter()
                        .map(|v| json!({ "label": v["label"], "price": v["price"] }))
                        .collect::<Vec<_>>(),
                    "lineTotal": l["lineTotal"],
                })
            })
            .collect();
        view.insert(
            "transaction".into(),
            json!({ "transactionLines": lines, "transactionTotal": t["transactionTotal"] }),
        );
    }
    if let Some(f) = d.get("financials").and_then(Value::as_array) {
        let lines: Vec<Value> = f
            .iter()
            .map(|p| json!({ "type": p["type"], "amount": p["amount"], "amountType": p["amountType"] }))
            .collect();
        view.insert("financials".into(), Value::Array(lines));
    }
    if let Some(cc) = d.get("costCenter") {
        view.insert("costCenter.type".into(), cc["type"].clone());
    }
    if let Some(r) = d.get("fdmRef") {
        view.insert("fdmRef".into(), r.clone());
    }
    if d.get("drawer").is_some() {
        view.insert("drawer".into(), json!(true));
    }
    Value::Object(view)
}

fn diff(path: &str, ours: &Value, fixture: &Value, out: &mut Vec<(String, Value, Value)>) {
    match (ours, fixture) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                diff(
                    &child,
                    a.get(k).unwrap_or(&Value::Null),
                    b.get(k).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                diff(&format!("{path}[{i}]"), x, y, out);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            if (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() > 0.005 {
                out.push((path.to_string(), ours.clone(), fixture.clone()));
            }
        }
        _ if ours != fixture => out.push((path.to_string(), ours.clone(), fixture.clone())),
        _ => {}
    }
}

/// Format of the fields that identify the installation, not the sale.
fn check_identification(uc: &str, d: &Value) {
    let s = |k: &str| d[k].as_str().unwrap_or_else(|| panic!("{uc}: {k} missing"));
    let digits = |v: &str, n: usize| v.len() == n && v.bytes().all(|b| b.is_ascii_digit());
    assert!(
        ["FR", "NL", "DE", "EN"].contains(&s("language")),
        "{uc}: language"
    );
    let vat = s("vatNo");
    assert!(
        vat.starts_with("BE") && digits(&vat[2..], 10),
        "{uc}: vatNo {vat}"
    );
    assert!(digits(s("estNo"), 10), "{uc}: estNo");
    let pos_id = s("posId");
    assert!(
        pos_id.len() == 14 && pos_id.is_ascii(),
        "{uc}: posId {pos_id}"
    );
    assert!(
        d["posFiscalTicketNo"].as_i64().unwrap() >= 1,
        "{uc}: posFiscalTicketNo"
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(s("posDateTime")).is_ok(),
        "{uc}: posDateTime"
    );
    assert!(!s("posSwVersion").is_empty(), "{uc}: posSwVersion");
    let device = s("deviceId");
    let mac = device.split(':').collect::<Vec<_>>();
    assert!(
        mac.len() == 6
            && mac
                .iter()
                .all(|b| b.len() == 2 && b.chars().all(|c| c.is_ascii_hexdigit())),
        "{uc}: deviceId is the terminal's MAC address, got {device:?}"
    );
    assert!(!s("terminalId").is_empty(), "{uc}: terminalId");
    assert!(
        uuid::Uuid::parse_str(s("bookingPeriodId")).is_ok(),
        "{uc}: bookingPeriodId"
    );
    assert!(
        chrono::NaiveDate::parse_from_str(s("bookingDate"), "%Y-%m-%d").is_ok(),
        "{uc}: bookingDate"
    );
    assert!(digits(s("employeeId"), 11), "{uc}: employeeId");
    if let Some(lines) = d["transaction"]["transactionLines"].as_array() {
        for l in lines {
            assert!(
                !l["mainProduct"]["productName"]
                    .as_str()
                    .unwrap_or("")
                    .is_empty(),
                "{uc}: productName"
            );
            assert!(
                !l["mainProduct"]["departmentName"]
                    .as_str()
                    .unwrap_or("")
                    .is_empty(),
                "{uc}: departmentName"
            );
        }
    }
}

/// Compare one request with its fixture and record the evidence.
fn assert_matches_fixture(uc: &str, ours: &Value, fixture: &Value, allowed: &[Deviation]) {
    check_identification(uc, ours);
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
        .map(|d| d.path)
        .collect();

    write_check_report(uc, &found, allowed);
    assert!(
        unexpected.is_empty(),
        "{uc}: request differs from the fixture: {unexpected:#?}"
    );
    assert!(
        stale.is_empty(),
        "{uc}: listed deviations no longer occur: {stale:?}"
    );
}

fn write_check_report(uc: &str, found: &[(String, Value, Value)], allowed: &[Deviation]) {
    let Ok(dir) = std::env::var("UC_EVIDENCE_DIR") else {
        return;
    };
    let deviations: Vec<Value> = found
        .iter()
        .map(|(path, ours, fixture)| {
            let reason = allowed.iter().find(|d| d.path == path).map(|d| d.reason);
            json!({ "path": path, "moyunPos": ours, "fixture": fixture, "reason": reason })
        })
        .collect();
    let report = json!({
        "uc": uc,
        "checked": "business fields equal; identification fields by format; names present",
        "deviations": deviations,
    });
    let dir = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(format!("{uc}_check.json")),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
}

// ── Sales (N) ──────────────────────────────────────────────────────

/// moyun-pos sells dine-in orders through a table (Tables → order →
/// checkout), so a settled dine-in sale carries its table; the
/// fixture's "direct sale" has no cost center.
const TABLE_SALE: Deviation = Deviation {
    path: "costCenter.type",
    reason: "dine-in sales are settled from a table and carry costCenter TABLE",
};

#[tokio::test]
async fn m110_sale() {
    let vars = variables("M110");
    let app = spf_app(&operator(&vars)).await;
    let order_id = order_from(&app, &vars, OrderType::DineIn, Some(1));
    mock_uc(&app, "M110", None).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    assert_matches_fixture("M110", &requests(&app)[0], &vars, &[TABLE_SALE]);
    app.dump_evidence("M110");
}

#[tokio::test]
async fn m111_full_refund() {
    let sale = variables("M110");
    let refund = variables("M111");
    let app = spf_app(&operator(&refund)).await;
    let order_id = order_from(&app, &sale, OrderType::DineIn, Some(1));
    // The sale's FDM response carries the reference the refund must cite.
    mock_uc(&app, "M110", Some(&refund["fdmRef"])).await;
    mock_uc(&app, "M111", None).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original = app.fiscal_events()[0].id;
    let items = vec![
        RefundItem {
            menu_item_id: 10006,
            quantity: 2,
        },
        RefundItem {
            menu_item_id: 22001,
            quantity: 1,
        },
    ];
    let result = do_refund_checkout(&app.state, original, items, operator(&refund))
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    assert_matches_fixture("M111", &requests(&app)[1], &refund, &[]);
    app.dump_evidence("M111");
}

#[tokio::test]
async fn m112b_partial_refund_as_a_separate_ticket() {
    let sale = variables("M112B_signSale_RefundPartial/M112B_1");
    let refund = variables("M112B_signSale_RefundPartial/M112B_2");
    let app = spf_app(&operator(&refund)).await;
    let order_id = order_from(&app, &sale, OrderType::DineIn, Some(1));
    mock_uc(&app, "M112B_signSale_RefundPartial/M112B_1", None).await;
    mock_uc(&app, "M112B_signSale_RefundPartial/M112B_2", None).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert_matches_fixture("M112B_1", &requests(&app)[0], &sale, &[TABLE_SALE]);

    let original = app.fiscal_events()[0].id;
    let result = do_refund_checkout(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 10006,
            quantity: 1,
        }],
        operator(&refund),
    )
    .await
    .unwrap();
    assert!(result.success, "{:?}", result.error);

    assert_matches_fixture(
        "M112B_2",
        &requests(&app)[1],
        &refund,
        &[Deviation {
            path: "fdmRef",
            reason: "moyun-pos references the refunded ticket on every refund \
                     (FLUX_REMBOURSEMENT_FR.md); the fixture's partial refund \
                     carries no reference",
        }],
    );
    app.dump_evidence("M112B");
}

// ── Orders and pre-bills (P) ───────────────────────────────────────

#[tokio::test]
async fn m121_table_order() {
    let vars = variables("M121");
    let app = spf_app(&operator(&vars)).await;
    let order_id = order_from(&app, &vars, OrderType::DineIn, Some(1));
    mock_uc(&app, "M121", None).await;

    do_sign_table_order(&app.state, order_id).await.unwrap();

    assert_matches_fixture(
        "M121",
        &requests(&app)[0],
        &vars,
        &[Deviation {
            path: "ticketMedium",
            reason: "orders are not printed in v1.0.0 (no kitchen ticket), so the medium is NONE",
        }],
    );
    app.dump_evidence("M121");
}

#[tokio::test]
async fn m123_pre_bill() {
    let vars = variables("M123");
    let app = spf_app(&operator(&vars)).await;
    let order_id = order_from(&app, &vars, OrderType::DineIn, Some(1));
    // Unregistered lines are sent as an order before the pre-bill.
    mock_uc(&app, "M121", None).await;
    mock_uc(&app, "M123", None).await;

    do_sign_pre_bill(&app.state, order_id).await.unwrap();

    let requests = requests(&app);
    assert_eq!(requests.len(), 2, "order, then pre-bill");
    assert_matches_fixture("M123", &requests[1], &vars, &[]);
    app.dump_evidence("M123");
}

// ── Financial (F) ──────────────────────────────────────────────────

#[tokio::test]
async fn m130_money_in() {
    let vars = variables("M130");
    let app = spf_app(&operator(&vars)).await;
    mock_uc(&app, "M130", None).await;

    let amount = vars["financials"][0]["amount"].as_f64().unwrap();
    do_money_in_out(
        &app.state,
        operator(&vars),
        (amount * 100.0).round() as i64,
        MoneyDirection::In,
        "Fond de caisse".to_string(),
    )
    .await
    .unwrap();

    assert_matches_fixture("M130", &requests(&app)[0], &vars, &[]);
    app.dump_evidence("M130");
}

#[tokio::test]
async fn m131_drawer_open() {
    let vars = variables("M131");
    let app = spf_app(&operator(&vars)).await;
    mock_uc(&app, "M131", None).await;

    do_open_drawer_fiscal(&app.state, operator(&vars))
        .await
        .unwrap();

    assert_matches_fixture("M131", &requests(&app)[0], &vars, &[]);
    app.dump_evidence("M131");
}

// ── Social (S) ─────────────────────────────────────────────────────

#[tokio::test]
async fn m140_work_in() {
    let vars = variables("M140");
    let app = spf_app(&operator(&vars)).await;
    mock_uc(&app, "M140", None).await;

    do_work_in(&app.state, operator(&vars)).await.unwrap();

    assert_matches_fixture("M140", &requests(&app)[0], &vars, &[]);
    app.dump_evidence("M140");
}

#[tokio::test]
async fn m141_work_out() {
    let vars = variables("M141");
    let app = spf_app(&operator(&vars)).await;
    mock_uc(&app, "M141", None).await;

    do_work_out(&app.state, operator(&vars)).await.unwrap();

    assert_matches_fixture("M141", &requests(&app)[0], &vars, &[]);
    app.dump_evidence("M141");
}

// ── Copies (C) ─────────────────────────────────────────────────────

#[tokio::test]
async fn m160_copy_of_a_sale() {
    let sale = variables("M110");
    let copy = variables("M160");
    let app = spf_app(&operator(&copy)).await;
    let order_id = order_from(&app, &sale, OrderType::DineIn, Some(1));
    mock_uc(&app, "M110", Some(&copy["fdmRef"])).await;
    mock_uc(&app, "M160", None).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    let original = app.fiscal_events()[0].id;
    let result = do_copy_ticket(&app.state, original, operator(&copy))
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    assert_matches_fixture("M160", &requests(&app)[1], &copy, &[]);
    app.dump_evidence("M160");
}

#[tokio::test]
async fn m161_copy_of_an_order() {
    let order = variables("M121");
    let copy = variables("M161");
    let app = spf_app(&operator(&copy)).await;
    let order_id = order_from(&app, &order, OrderType::DineIn, Some(1));
    mock_uc(&app, "M121", Some(&copy["fdmRef"])).await;
    mock_uc(&app, "M161", None).await;

    do_sign_table_order(&app.state, order_id).await.unwrap();
    let original = app.fiscal_events()[0].id;
    let result = do_copy_ticket(&app.state, original, operator(&copy))
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    assert_matches_fixture("M161", &requests(&app)[1], &copy, &[]);
    app.dump_evidence("M161");
}

// ── Training (T) ───────────────────────────────────────────────────

#[tokio::test]
async fn m170_work_in_in_training() {
    let vars = variables("M170");
    let app = spf_app(&operator(&vars)).await;
    set_training_mode(&app, true);
    mock_uc(&app, "M170", None).await;

    do_work_in(&app.state, operator(&vars)).await.unwrap();

    assert_matches_fixture("M170", &requests(&app)[0], &vars, &[]);
    app.dump_evidence("M170");
}

#[tokio::test]
async fn m171_sale_in_training() {
    let vars = variables("M171_signSale_Training");
    let app = spf_app(&operator(&vars)).await;
    set_training_mode(&app, true);
    let order_id = order_from(&app, &vars, OrderType::DineIn, Some(1));
    mock_uc(&app, "M171_signSale_Training", None).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    assert_matches_fixture("M171", &requests(&app)[0], &vars, &[TABLE_SALE]);
    app.dump_evidence("M171");
}

/// M172 copies a normal ticket (N 35/73) while the POS is in training
/// mode: the sale is made first, then training mode is switched on.
#[tokio::test]
async fn m172_copy_in_training() {
    let sale = variables("M110");
    let copy = variables("M172");
    let app = spf_app(&operator(&copy)).await;
    let order_id = order_from(&app, &sale, OrderType::DineIn, Some(1));
    mock_uc(&app, "M110", Some(&copy["fdmRef"])).await;
    mock_uc(&app, "M172", None).await;

    do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    set_training_mode(&app, true);
    let original = app.fiscal_events()[0].id;
    let result = do_copy_ticket(&app.state, original, operator(&copy))
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(app.fiscal_events()[1].event_type, "T");

    assert_matches_fixture(
        "M172",
        &requests(&app)[1],
        &copy,
        &[Deviation {
            path: "ticketMedium",
            reason: "copies are printed in training mode too, so the medium is PAPER",
        }],
    );
    app.dump_evidence("M172");
}

// ── Catalogue guards ───────────────────────────────────────────────

fn add(app: &TestApp, order_id: i64, menu_item_id: i64, price: Option<i64>) -> Result<(), String> {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: price,
        },
    )
    .map(|_| ())
}

#[tokio::test]
async fn catalogue_prices_cannot_be_overridden() {
    let app = spf_app("84022899837").await;
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::create_order(&conn, OrderType::DineIn, Some(1), "84022899837")
            .unwrap()
            .id
    };
    // Dry Martini has a fixed price; the Dish of the day is free-price.
    assert!(add(&app, order_id, 10006, Some(100)).is_err());
    assert!(add(&app, order_id, 22010, None).is_err());
    assert!(add(&app, order_id, 22010, Some(0)).is_err());
    assert!(add(&app, order_id, 22010, Some(2000)).is_ok());
    // Products of unsupported features are inactive and cannot be sold.
    assert!(add(&app, order_id, 22005, None).is_err());
}

#[tokio::test]
async fn takeaway_orders_use_the_takeaway_vat_label() {
    let app = spf_app("84022899837").await;
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::create_order(&conn, OrderType::Takeaway, None, "84022899837")
            .unwrap()
            .id
    };
    add(&app, order_id, 22001, None).unwrap(); // Burger: B on site, C takeaway
    add(&app, order_id, 10006, None).unwrap(); // Dry Martini: A both
    let lines = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&conn, order_id).unwrap()
    };
    let vat: Vec<_> = lines.iter().map(|l| l.vat_code.as_str()).collect();
    assert_eq!(vat, ["C", "A"]);
    assert_eq!(lines[0].department_id, "22");
}
