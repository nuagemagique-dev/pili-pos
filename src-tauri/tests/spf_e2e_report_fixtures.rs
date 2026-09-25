//! SPF Reports v1.3, replayed end to end through the real commands.
//!
//! The scenario's service day (InputReport T1–T8) is played on the
//! manual's catalogue exactly as an operator would: a table order, its
//! sale and refund, a takeaway paid by card, a second table ordered in
//! two rounds (one with the free-price dish of the day), its pre-bill
//! and its sale. Each event is signed by a mock FDM replaying the
//! scenario's own response. The X, user X and Z reports moyun-pos then
//! builds from its archive are compared with the published M180, M182
//! and M184 reports (M181 and M183 carry the same totals).
//!
//! Compared exactly: event counts and amounts, departments (id and
//! amount), VAT (rate, taxable, VAT, total), payments (type and every
//! amount), negative quantities, drawer openings, per-user totals,
//! payments and presence. Intended differences:
//! - P tickets: 4 events, 250 € (T1, T5, T6, T7) where the published
//!   report counts 3 (198 €); the spec counts every event of the label
//!   (§2.2.2 `ticketCount`). See `pos-fiscal/tests/report_totals_fixtures.rs`.
//! - Payment id and name are moyun-pos's own; card payments are
//!   CARD_DEBIT (the operator does not tell debit from credit cards),
//!   CARD_CREDIT in the scenario.
//! - Department names are the catalogue's French names.
//! - posDevices / fdmDevices are this single terminal and the mock FDM.

mod common;

use common::*;
use serde_json::{Value, json};

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::order::{AddOrderLineInput, do_add_order_line};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::report::{
    do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;

const U1: &str = "75061189702";
const U2: &str = "84022899837";

fn read_json(dir: &str, suffix: &str) -> Value {
    let dir = reports_root().join(dir);
    let path = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with(suffix))
        .unwrap_or_else(|| panic!("no *{suffix} in {}", dir.display()));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Mount the scenario's response for input ticket `t` (T1–T8).
async fn fdm_signs(app: &TestApp, t: &str) {
    mock_response(
        app,
        read_json(&format!("InputReport/{t}"), "_Response.json"),
    )
    .await;
}

async fn fdm_signs_report(app: &TestApp, uc_dir: &str) {
    mock_response(app, read_json(uc_dir, "_Response.json")).await;
}

async fn scenario_app() -> TestApp {
    let app = TestApp::new().await;
    let conn = app.state.db.lock().unwrap();
    moyun_pos::cli::apply_seed(&conn, include_str!("../seed_spf_test.sql")).expect("seed");
    for (insz, name) in [(U1, "Operateur 1"), (U2, "Operateur 2")] {
        conn.execute(
            "INSERT OR IGNORE INTO users (insz, name, role) VALUES (?1, ?2, 'cashier')",
            rusqlite::params![insz, name],
        )
        .unwrap();
    }
    drop(conn);
    app
}

fn open_order(app: &TestApp, user: &str, order_type: OrderType, table: Option<i32>) -> i64 {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::orders::create_order(&conn, order_type, table, user)
        .unwrap()
        .id
}

fn add(app: &TestApp, order_id: i64, menu_item_id: i64, quantity: i32, price: Option<i64>) {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id,
            quantity,
            modifiers_json: None,
            unit_price_cents: price,
        },
    )
    .expect("add order line");
}

fn last_event_id(app: &TestApp) -> i64 {
    app.fiscal_events().last().unwrap().id
}

/// T1–T8, played through the commands the UI calls.
async fn play_service_day(app: &TestApp) {
    // T1 — table 1: 2 Dry Martini + 1 Burger, registered when the
    // waiter leaves the table (P).
    let table_1 = open_order(app, U1, OrderType::DineIn, Some(1));
    add(app, table_1, 10006, 2, None);
    add(app, table_1, 22001, 1, None);
    fdm_signs(app, "T1").await;
    do_sign_table_order(&app.state, table_1).await.unwrap();

    // T2 — table 1 pays cash (N).
    fdm_signs(app, "T2").await;
    let sale = do_checkout(&app.state, table_1, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "T2: {:?}", sale.error);
    let t2 = last_event_id(app);

    // T3 — the whole T2 sale is refunded (N, REFUND).
    fdm_signs(app, "T3").await;
    let refund = do_refund_checkout(
        &app.state,
        t2,
        vec![
            RefundItem {
                menu_item_id: 10006,
                quantity: 2,
            },
            RefundItem {
                menu_item_id: 22001,
                quantity: 1,
            },
        ],
        U1.to_string(),
    )
    .await
    .unwrap();
    assert!(refund.success, "T3: {:?}", refund.error);

    // T4 — a Coca Cola to take away, paid by card (N).
    let takeaway = open_order(app, U2, OrderType::Takeaway, None);
    add(app, takeaway, 12009, 1, None);
    fdm_signs(app, "T4").await;
    let sale = do_checkout(&app.state, takeaway, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(sale.success, "T4: {:?}", sale.error);

    // T5 — table 10, first round: 2 Dry Martini (P).
    let table_10 = open_order(app, U2, OrderType::DineIn, Some(10));
    add(app, table_10, 10006, 2, None);
    fdm_signs(app, "T5").await;
    do_sign_table_order(&app.state, table_10).await.unwrap();

    // T6 — second round: 2 dishes of the day at 20 € (free price) and a
    // Bourgogne (P). Only the new entries are sent.
    add(app, table_10, 22010, 2, Some(2000));
    add(app, table_10, 15001, 1, None);
    fdm_signs(app, "T6").await;
    do_sign_table_order(&app.state, table_10).await.unwrap();

    // T7 — the pre-bill (P): nothing left to register first.
    fdm_signs(app, "T7").await;
    do_sign_pre_bill(&app.state, table_10).await.unwrap();

    // T8 — table 10 pays cash (N).
    fdm_signs(app, "T8").await;
    let sale = do_checkout(&app.state, table_10, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "T8: {:?}", sale.error);

    let labels: Vec<_> = app
        .fiscal_events()
        .iter()
        .map(|e| e.event_type.clone())
        .collect();
    assert_eq!(labels, ["P", "N", "N", "N", "P", "P", "P", "N"]);
}

fn archived_request(app: &TestApp, index_from_end: usize) -> Value {
    let events = app.fiscal_events();
    let e = &events[events.len() - 1 - index_from_end];
    serde_json::from_str(e.event_data_json.as_deref().unwrap()).unwrap()
}

/// Amounts equal to the cent.
fn assert_amount(what: &str, ours: &Value, fixture: &Value) {
    let (a, b) = (ours.as_f64(), fixture.as_f64());
    assert!(
        matches!((a, b), (Some(a), Some(b)) if (a - b).abs() < 0.005),
        "{what}: moyun-pos {ours} vs fixture {fixture}"
    );
}

fn assert_payments(what: &str, ours: &Value, fixture: &Value) {
    let (ours, fixture) = (ours.as_array().unwrap(), fixture.as_array().unwrap());
    assert_eq!(ours.len(), fixture.len(), "{what}: payment methods");
    for (i, (o, f)) in ours.iter().zip(fixture).enumerate() {
        let expected_type = match f["type"].as_str().unwrap() {
            "CARD_CREDIT" => "CARD_DEBIT",
            other => other,
        };
        assert_eq!(o["type"], expected_type, "{what}[{i}].type");
        assert!(!o["id"].as_str().unwrap().is_empty(), "{what}[{i}].id");
        assert!(!o["name"].as_str().unwrap().is_empty(), "{what}[{i}].name");
        let (oa, fa) = (
            o["amount"].as_array().unwrap(),
            f["amount"].as_array().unwrap(),
        );
        assert_eq!(oa.len(), fa.len(), "{what}[{i}].amount");
        for (j, (x, y)) in oa.iter().zip(fa).enumerate() {
            assert_eq!(x["type"], y["type"], "{what}[{i}].amount[{j}].type");
            for k in [
                "normalAmount",
                "negativeCorrections",
                "positiveCorrections",
                "totalAmount",
            ] {
                assert_amount(&format!("{what}[{i}].amount[{j}].{k}"), &x[k], &y[k]);
            }
            assert_eq!(
                x["correctionsCount"], y["correctionsCount"],
                "{what}[{i}].amount[{j}].correctionsCount"
            );
        }
    }
}

fn assert_turnover(ours: &Value, fixture: &Value) {
    // Transactions, with the P count of the spec (see module comment).
    let expected_transactions = json!([
        fixture["transactions"][0],
        { "eventLabel": "P", "ticketCount": 4, "amount": 250 },
    ]);
    assert_eq!(fixture["transactions"][1]["ticketCount"], 3);
    let (o, f) = (
        ours["transactions"].as_array().unwrap(),
        expected_transactions.as_array().unwrap(),
    );
    assert_eq!(o.len(), f.len(), "transactions");
    for (x, y) in o.iter().zip(f) {
        assert_eq!(x["eventLabel"], y["eventLabel"]);
        assert_eq!(x["ticketCount"], y["ticketCount"], "{}", y["eventLabel"]);
        assert_amount(
            &format!("transactions.{}", y["eventLabel"]),
            &x["amount"],
            &y["amount"],
        );
    }

    let (o, f) = (
        ours["departments"].as_array().unwrap(),
        fixture["departments"].as_array().unwrap(),
    );
    assert_eq!(o.len(), f.len(), "departments");
    for (x, y) in o.iter().zip(f) {
        assert_eq!(x["departmentId"], y["departmentId"]);
        assert!(!x["departmentName"].as_str().unwrap().is_empty());
        assert_amount(
            &format!("departments.{}", y["departmentId"]),
            &x["amount"],
            &y["amount"],
        );
    }

    let (o, f) = (
        ours["vats"].as_array().unwrap(),
        fixture["vats"].as_array().unwrap(),
    );
    assert_eq!(o.len(), f.len(), "vats");
    for (x, y) in o.iter().zip(f) {
        assert_eq!(x["label"], y["label"]);
        for k in ["rate", "taxableAmount", "vatAmount", "totalAmount"] {
            assert_amount(&format!("vats.{}.{k}", y["label"]), &x[k], &y[k]);
        }
    }

    assert_payments("payments", &ours["payments"], &fixture["payments"]);
    assert_eq!(ours["drawersOpenCount"], fixture["drawersOpenCount"]);

    let (o, f) = (
        ours["negQuantities"].as_array().unwrap(),
        fixture["negQuantities"].as_array().unwrap(),
    );
    assert_eq!(o.len(), f.len(), "negQuantities");
    for (x, y) in o.iter().zip(f) {
        assert_eq!(x["negQuantityReason"], y["negQuantityReason"]);
        assert_eq!(x["negQuantityCount"], y["negQuantityCount"]);
        assert_eq!(x["ticketCount"], y["ticketCount"]);
        assert_amount("negQuantities.amount", &x["amount"], &y["amount"]);
    }
    assert_eq!(ours["priceChanges"], json!([]));
}

fn assert_users(ours: &Value, fixture: &Value) {
    let (o, f) = (ours.as_array().unwrap(), fixture.as_array().unwrap());
    assert_eq!(o.len(), f.len(), "users");
    for (x, y) in o.iter().zip(f) {
        let id = y["employeeId"].as_str().unwrap();
        assert_eq!(x["employeeId"], y["employeeId"]);
        assert_amount(
            &format!("users.{id}.totalAmount"),
            &x["totalAmount"],
            &y["totalAmount"],
        );
        for k in ["firstPosDateTime", "lastPosDateTime"] {
            assert!(
                chrono::DateTime::parse_from_rfc3339(x[k].as_str().unwrap()).is_ok(),
                "users.{id}.{k}"
            );
        }
        assert_eq!(
            x["socialEvents"], y["socialEvents"],
            "users.{id}.socialEvents"
        );
        assert_payments(
            &format!("users.{id}.payments"),
            &x["payments"],
            &y["payments"],
        );
    }
}

fn assert_report_identity(report: &Value, label: &str) {
    assert!(
        report["posDevices"]
            .as_array()
            .is_some_and(|d| d.len() == 1),
        "{label}: posDevices"
    );
    let fdm = &report["fdmDevices"][0];
    assert!(
        fdm["fdmId"].as_str().is_some_and(|s| !s.is_empty()),
        "{label}: fdmId"
    );
    assert!(
        fdm["firstTotalCounter"].as_i64() <= fdm["lastTotalCounter"].as_i64(),
        "{label}: fdm counters"
    );
    assert!(
        report["bookingPeriodId"].as_str().is_some(),
        "{label}: bookingPeriodId"
    );
}

#[tokio::test]
async fn m180_turnover_x_of_the_scenario_day() {
    let app = scenario_app().await;
    play_service_day(&app).await;

    fdm_signs_report(&app, "M180_signReportTurnoverX").await;
    let result = do_generate_x_report(&app.state, U1.to_string())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let ours = archived_request(&app, 0);
    let fixture = read_json("M180_signReportTurnoverX", "GQL-Variables.json");
    assert_turnover(&ours["turnover"], &fixture["data"]["turnover"]);
    assert_report_identity(&ours, "M180");
    // The FDM range covers T1 (first) to T8 (last) as the FDM signed them.
    assert_eq!(ours["fdmDevices"][0]["firstTotalCounter"], 49);
    assert_eq!(ours["fdmDevices"][0]["lastTotalCounter"], 56);
    app.dump_evidence("M180");
}

#[tokio::test]
async fn m182_user_x_of_the_scenario_day() {
    let app = scenario_app().await;
    play_service_day(&app).await;

    fdm_signs_report(&app, "M182_signReportUserX").await;
    let result = do_generate_user_x_report(&app.state, U1.to_string())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let ours = archived_request(&app, 0);
    let fixture = read_json("M182_signReportUserX", "GQL-Variables.json");
    assert_users(&ours["users"], &fixture["data"]["users"]);
    assert_report_identity(&ours, "M182");
    app.dump_evidence("M182");
}

#[tokio::test]
async fn m184_combined_z_of_the_scenario_day() {
    let app = scenario_app().await;
    play_service_day(&app).await;

    fdm_signs_report(&app, "M184_signReportTurnoverZUserZ").await;
    let result = do_generate_z_report(&app.state, U1.to_string())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.report_no, Some(1));

    let fixture = read_json("M184_signReportTurnoverZUserZ", "GQL-Variables.json");
    let turnover_z = archived_request(&app, 1);
    let user_z = archived_request(&app, 0);
    assert_turnover(&turnover_z["turnover"], &fixture["turnover"]["turnover"]);
    assert_users(&user_z["users"], &fixture["user"]["users"]);
    for (r, label) in [(&turnover_z, "turnover Z"), (&user_z, "user Z")] {
        assert_report_identity(r, label);
        assert_eq!(r["reportNo"], 1, "{label}");
        assert_eq!(r["reportBookingDate"], r["bookingDate"], "{label}");
    }
    // Both Z reports and every event of the day share the period the Z closes.
    let period = turnover_z["bookingPeriodId"].clone();
    assert!(
        app.fiscal_events().iter().all(|e| {
            serde_json::from_str::<Value>(e.event_data_json.as_deref().unwrap()).unwrap()
                ["bookingPeriodId"]
                == period
        }),
        "all events of the day are in the booking period"
    );
    assert_eq!(
        pos_db::queries::booking::open_period(&app.state.db.lock().unwrap()).unwrap(),
        None,
        "the Z closes the booking period"
    );
    app.dump_evidence("M184");
}

#[tokio::test]
async fn reports_after_a_z_only_cover_the_new_period() {
    let app = scenario_app().await;
    play_service_day(&app).await;
    fdm_signs_report(&app, "M184_signReportTurnoverZUserZ").await;
    do_generate_z_report(&app.state, U1.to_string())
        .await
        .unwrap();

    // Next day: one Coca Cola, then an X report.
    let takeaway = open_order(&app, U2, OrderType::Takeaway, None);
    add(&app, takeaway, 12009, 1, None);
    fdm_signs(&app, "T4").await;
    do_checkout(&app.state, takeaway, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    fdm_signs_report(&app, "M180_signReportTurnoverX").await;
    do_generate_x_report(&app.state, U1.to_string())
        .await
        .unwrap();

    let x = archived_request(&app, 0);
    assert_eq!(
        x["turnover"]["transactions"],
        json!([{ "eventLabel": "N", "ticketCount": 1, "amount": 3.04 }])
    );
    assert_eq!(x["turnover"]["negQuantities"], json!([]));
}

#[tokio::test]
async fn a_training_z_neither_takes_a_z_number_nor_closes_the_period() {
    let app = scenario_app().await;
    play_service_day(&app).await;
    app.state.config.lock().unwrap().training_mode = true;

    fdm_signs_report(&app, "M184_signReportTurnoverZUserZ").await;
    let result = do_generate_z_report(&app.state, U1.to_string())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let conn = app.state.db.lock().unwrap();
    let z_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM z_reports", [], |r| r.get(0))
        .unwrap();
    assert_eq!(z_count, 0, "a training Z takes no Z number");
    assert!(
        pos_db::queries::booking::open_period(&conn)
            .unwrap()
            .is_some(),
        "a training Z leaves the booking period open"
    );
}
