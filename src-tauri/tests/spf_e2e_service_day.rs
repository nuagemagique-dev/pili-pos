//! A long, realistic service day driven entirely through the Tauri
//! command layer: several tables, interleaved orders/corrections/
//! pre-bills/sales/refunds/copies/F/S events and a training-mode sale,
//! then an X and a Z report.
//!
//! Unlike the other e2e suites this one does not replay a single
//! official fixture — no one fixture covers "a whole day" — so the
//! mock FDM here is a small dynamic responder (`DynamicFdm`) that
//! actually reads each request and echoes back a coherent
//! `SignResult`: the ticket number the POS sent, the training-adjusted
//! label, and ever-increasing FDM counters. That lets this test make
//! the assertions the task calls for:
//!
//! - strictly sequential numbering per event label, no gaps;
//! - every archived `event_label` equals what the (fake) FDM signed;
//! - report totals (VAT, department, payment) equal the sum of the
//!   day's real sales minus refunds — training excluded.

mod common;

use std::sync::atomic::{AtomicI64, Ordering};

use common::*;
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, Request, Respond, ResponseTemplate};

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::copy::do_copy_ticket;
use moyun_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::report::{do_generate_x_report, do_generate_z_report};
use moyun_pos::commands::social::{do_work_in, do_work_out};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::{
    NewOrderLine, add_order_line, create_order, update_order_line_quantity,
};
use pos_fiscal::types::EventLabel;

const WAITER_1: &str = "11111111111";
const WAITER_2: &str = "22222222222";

// ── A dynamic fake FDM ──────────────────────────────────────────────

/// Signs whatever it's asked to sign: parses the request, figures out
/// which mutation was called (or the combined Z-report document) and
/// the ticket number(s) carried in `variables`, and returns a
/// `SignResult` (or pair, for the Z) with a matching `eventLabel` and
/// monotonically increasing FDM counters.
struct DynamicFdm {
    total_counter: AtomicI64,
}

impl DynamicFdm {
    fn new() -> Self {
        Self {
            total_counter: AtomicI64::new(0),
        }
    }

    fn sign_result(&self, ticket_number: i64, operation: &str, label: &str) -> Value {
        let total = self.total_counter.fetch_add(1, Ordering::SeqCst) + 1;
        json!({
            "posId": "CFOD0061234567",
            "posFiscalTicketNo": ticket_number,
            "posDateTime": "2026-07-03T12:00:00+02:00",
            "terminalId": "POS1",
            "deviceId": "DEV001",
            "eventOperation": operation,
            "fdmRef": {
                "fdmId": "CBX0SERVICEDAY1",
                "fdmDateTime": "2026-07-03T12:00:00+02:00",
                "eventLabel": label,
                "eventCounter": total,
                "totalCounter": total
            },
            "fdmSwVersion": "1.0.0-test",
            "digitalSignature": format!("SIGDAY{total:06}"),
            "shortSignature": format!("S{total:06}"),
            "verificationUrl": Value::Null,
            "vatCalc": Value::Null,
            "bufferCapacityUsed": 0.1,
            "warnings": Value::Null,
            "informations": Value::Null,
            "footer": []
        })
    }
}

/// (mutation name, eventOperation, base event label — overridden to
/// "T" whenever the request carries `isTraining: true`).
const MUTATIONS: &[(&str, &str, &str)] = &[
    ("signSale", "SALE", "N"),
    ("signOrder", "ORDER", "P"),
    ("signPreBill", "PRE_BILL", "P"),
    ("signMoneyInOut", "MONEY_IN_OUT", "F"),
    ("signDrawerOpen", "DRAWER_OPEN", "F"),
    ("signWorkIn", "WORK_IN", "S"),
    ("signWorkOut", "WORK_OUT", "S"),
    ("signCopy", "COPY", "C"),
    ("signReportTurnoverX", "REPORT_TURNOVER_X", "R"),
    ("signReportUserX", "REPORT_USER_X", "R"),
];

impl Respond for DynamicFdm {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = request.body_json().expect("valid GraphQL request JSON");
        let query = body["query"].as_str().unwrap_or_default();
        let vars = &body["variables"];
        let is_training = vars["isTraining"].as_bool().unwrap_or(false);

        // The M184-style combined Z-report carries two mutations in
        // one document — handle it before the generic single-mutation
        // lookup below.
        if query.contains("signReportTurnoverZ") && query.contains("signReportUserZ") {
            let label = if is_training { "T" } else { "R" };
            // `CommonFields` is `#[serde(flatten)]`ed into every input
            // struct, so `posFiscalTicketNo` sits at the top level of
            // each half of the payload, not nested under a `common` key.
            let t_ticket = vars["turnover"]["posFiscalTicketNo"]
                .as_i64()
                .expect("turnover ticket number");
            let u_ticket = vars["user"]["posFiscalTicketNo"]
                .as_i64()
                .expect("user ticket number");
            let data = json!({
                "signReportTurnoverZ": self.sign_result(t_ticket, "REPORT_TURNOVER_Z", label),
                "signReportUserZ": self.sign_result(u_ticket, "REPORT_USER_Z", label),
            });
            return ResponseTemplate::new(200).set_body_json(json!({ "data": data }));
        }

        let &(mutation, operation, base_label) = MUTATIONS
            .iter()
            .find(|(name, _, _)| query.contains(name))
            .unwrap_or_else(|| panic!("DynamicFdm: unrecognised mutation in query: {query}"));

        let ticket = vars["data"]["posFiscalTicketNo"]
            .as_i64()
            .unwrap_or_else(|| panic!("DynamicFdm: no posFiscalTicketNo in {vars}"));
        let label = if is_training { "T" } else { base_label };
        let result = self.sign_result(ticket, operation, label);
        ResponseTemplate::new(200).set_body_json(json!({ "data": { mutation: result } }))
    }
}

async fn dynamic_fdm(app: &TestApp) {
    Mock::given(method("POST"))
        .and(path("/graphql"))
        .respond_with(DynamicFdm::new())
        .mount(&app.fdm)
        .await;
}

// ── Catalogue ────────────────────────────────────────────────────────

/// Two departments, three products, two operators. Prices are chosen
/// so every total is a whole multiple of 5 cents — no cash-rounding
/// noise to account for in the report-total assertions.
fn seed_catalogue(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO users (insz, name, role, active) VALUES
            ('11111111111', 'Waiter One', 'waiter', 1),
            ('22222222222', 'Waiter Two', 'waiter', 1);
         INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
            (101, 1, 1200, 'B', 0, 1),
            (102, 2, 450, 'A', 0, 1),
            (103, 1, 800, 'B', 1, 1);",
    )
    .unwrap();
}

fn item(id: i64) -> (i64, VatCode, &'static str, &'static str) {
    match id {
        101 => (1200, VatCode::B, "1", "Food"),
        102 => (450, VatCode::A, "2", "Drinks"),
        103 => (800, VatCode::B, "1", "Food"),
        _ => panic!("unknown test item {id}"),
    }
}

/// `operator` logs in (as a manager: the scenario's waiters also run
/// refunds, copies, cash events and reports) and opens a table.
fn order_with_table(app: &TestApp, table: i32, operator: &str) -> i64 {
    app.login_as(operator, pos_core::user::UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    create_order(&conn, OrderType::DineIn, Some(table), operator)
        .unwrap()
        .id
}

fn add(app: &TestApp, order_id: i64, menu_item_id: i64, quantity: i32) -> i64 {
    let (price, vat, dept_id, dept_name) = item(menu_item_id);
    let conn = app.state.db.lock().unwrap();
    add_order_line(
        &conn,
        NewOrderLine {
            order_id,
            menu_item_id,
            quantity,
            unit_price_cents: price,
            vat_code: vat,
            name: "Test item",
            modifiers_json: None,
            department_id: dept_id,
            department_name: dept_name,
        },
    )
    .unwrap()
    .id
}

fn set_quantity(app: &TestApp, line_id: i64, quantity: i32) {
    let conn = app.state.db.lock().unwrap();
    update_order_line_quantity(&conn, line_id, quantity).unwrap();
}

fn set_training_mode(app: &TestApp, on: bool) {
    app.state.config.lock().unwrap().training_mode = on;
}

// ── The scenario ─────────────────────────────────────────────────────

#[tokio::test]
async fn a_full_service_day_reconciles_sequentially_and_excludes_training() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    dynamic_fdm(&app).await;

    // Table 1 (waiter 1): two rounds ordered, a pre-bill, then pays cash.
    // 101x2 (24.00) + 102x1 (4.50) = 28.50, flushed as an order (P).
    let table1 = order_with_table(&app, 1, WAITER_1);
    add(&app, table1, 101, 2);
    add(&app, table1, 102, 1);
    do_sign_table_order(&app.state, table1).await.unwrap();
    // A second round arrives: 103x1 (8.00) — unsent until the pre-bill,
    // which flushes it as its own order first (§1.4), then the bill.
    add(&app, table1, 103, 1);
    do_sign_pre_bill(&app.state, table1).await.unwrap();
    let table1_total_cents = 2 * 1200 + 450 + 800; // 3650
    let checkout1 = do_checkout(&app.state, table1, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(checkout1.success, "{:?}", checkout1.error);

    // Table 2 (waiter 2): straight to checkout, no P events, card.
    let table2 = order_with_table(&app, 2, WAITER_2);
    add(&app, table2, 101, 1);
    add(&app, table2, 102, 2);
    let table2_total_cents = 1200 + 2 * 450; // 2100
    let checkout2 = do_checkout(&app.state, table2, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(checkout2.success, "{:?}", checkout2.error);
    let table2_sale_event = app.fiscal_events().last().unwrap().id;

    // A customer at table 2 sends one drink back.
    let refund_cents = 450;
    let refund = do_refund_checkout(
        &app.state,
        table2_sale_event,
        vec![RefundItem {
            menu_item_id: 102,
            quantity: 1,
        }],
        WAITER_2.to_string(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);

    // Table 3 (waiter 1): order, then the kitchen ran out — the
    // operator corrects the quantity down before signing again.
    let table3 = order_with_table(&app, 3, WAITER_1);
    let table3_line = add(&app, table3, 101, 3);
    do_sign_table_order(&app.state, table3).await.unwrap(); // sends +3
    set_quantity(&app, table3_line, 2); // correction: -1, unsent
    do_sign_table_order(&app.state, table3).await.unwrap(); // sends -1
    let table3_total_cents = 2 * 1200; // 2400
    let checkout3 = do_checkout(&app.state, table3, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(checkout3.success, "{:?}", checkout3.error);

    // A copy of table 1's ticket (e.g. the customer asks for a reprint).
    let table1_sale_event = app
        .fiscal_events()
        .iter()
        .find(|e| e.order_id == Some(table1) && e.event_type == "N")
        .unwrap()
        .id;
    let copy = do_copy_ticket(&app.state, table1_sale_event, WAITER_1.to_string())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);

    // Cash float declared at the start of the shift, plus a manual
    // drawer open unrelated to any sale.
    do_money_in_out(
        &app.state,
        WAITER_1.to_string(),
        10000,
        MoneyDirection::In,
        "Fond de caisse".to_string(),
    )
    .await
    .unwrap();
    do_open_drawer_fiscal(&app.state, WAITER_1.to_string())
        .await
        .unwrap();

    // Staff presence.
    do_work_in(&app.state, WAITER_1.to_string()).await.unwrap();
    app.login_as(WAITER_2, pos_core::user::UserRole::Admin);
    do_work_in(&app.state, WAITER_2.to_string()).await.unwrap();
    do_work_out(&app.state, WAITER_2.to_string()).await.unwrap();

    // A training sale: must be labelled T and must not leak into the
    // real N counter or into any of the real report totals below.
    set_training_mode(&app, true);
    let training_order = order_with_table(&app, 9, WAITER_1);
    add(&app, training_order, 101, 5); // 60.00 — would visibly skew
    // VAT B / department "1" totals below if it weren't excluded.
    let training_checkout = do_checkout(&app.state, training_order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(training_checkout.success, "{:?}", training_checkout.error);
    set_training_mode(&app, false);

    // ── Sequential numbering, no gaps, per label ────────────────────
    let events = app.fiscal_events();
    for label in ["N", "P", "F", "S", "C", "T"] {
        let mut seqs: Vec<i64> = events
            .iter()
            .filter(|e| e.event_type == label)
            .map(|e| e.sequence_number)
            .collect();
        seqs.sort_unstable();
        let expected: Vec<i64> = (1..=seqs.len() as i64).collect();
        assert_eq!(
            seqs, expected,
            "label {label}: sequence numbers must be 1..N, no gaps"
        );
    }
    assert_eq!(events.iter().filter(|e| e.event_type == "N").count(), 4);
    assert_eq!(events.iter().filter(|e| e.event_type == "P").count(), 5);
    assert_eq!(events.iter().filter(|e| e.event_type == "F").count(), 2);
    assert_eq!(events.iter().filter(|e| e.event_type == "S").count(), 3);
    assert_eq!(events.iter().filter(|e| e.event_type == "C").count(), 1);
    assert_eq!(events.iter().filter(|e| e.event_type == "T").count(), 1);

    // ── Every archived event_label equals what the FDM signed ──────
    for e in &events {
        assert_eq!(
            Some(e.event_type.as_str()),
            e.event_label.as_deref(),
            "event {} archived as {} but the FDM signed it as {:?}",
            e.id,
            e.event_type,
            e.event_label
        );
    }

    // ── X report: mid-day, must not reset anything ──────────────────
    let x = do_generate_x_report(&app.state, WAITER_1.to_string())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    assert_eq!(
        app.counter_for(EventLabel::N),
        4,
        "X report must not touch N"
    );
    assert_eq!(
        app.counter_for(EventLabel::P),
        5,
        "X report must not touch P"
    );

    let real_total_cents = table1_total_cents as i64 + table2_total_cents as i64 - refund_cents
        + table3_total_cents as i64;
    assert_eq!(real_total_cents, 7700); // sanity on the arithmetic above

    let x_events = app.fiscal_events();
    let x_report = x_events.iter().find(|e| e.event_type == "R").unwrap();
    let x_request: Value =
        serde_json::from_str(x_report.event_data_json.as_ref().unwrap()).unwrap();
    assert_report_totals(&x_request, real_total_cents);

    // ── Z report: same totals, report_no = 1, both R events archived ─
    let z = do_generate_z_report(&app.state, WAITER_1.to_string())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);
    assert_eq!(z.report_no, Some(1));
    assert_eq!(
        app.counter_for(EventLabel::R),
        3,
        "X (1) + turnover Z + user Z"
    );

    let z_events = app.fiscal_events();
    let r_events: Vec<_> = z_events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(r_events.len(), 3);
    let turnover_z_request: Value =
        serde_json::from_str(r_events[1].event_data_json.as_ref().unwrap()).unwrap();
    assert_report_totals(&turnover_z_request, real_total_cents);

    // The training sale must be visible in the transactions (event
    // count) block for FDM reconciliation, but contribute nothing to
    // the real vats/departments/payments totals asserted above.
    let transactions = turnover_z_request["turnover"]["transactions"]
        .as_array()
        .unwrap();
    let t_row = transactions
        .iter()
        .find(|t| t["eventLabel"] == "T")
        .expect("training events must still appear in the transactions block");
    assert_eq!(t_row["ticketCount"], 1);
    assert_eq!(t_row["amount"], 60.0);

    let z_reports_count: i64 = {
        let conn = app.state.db.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM z_reports", [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!(z_reports_count, 1);
}

/// VAT B = 68.00 (all `101`/`103` lines, dine-in), VAT A = 9.00 (net
/// of the one refunded drink), department "1" = 68.00, department "2"
/// = 9.00, payments: cash 56.00 + card 21.00 = 77.00. All of it net of
/// the refund and excluding the training sale.
fn assert_report_totals(request: &Value, expected_total_cents: i64) {
    let vats = request["turnover"]["vats"].as_array().unwrap();
    let vat_b = vats.iter().find(|v| v["label"] == "B").unwrap();
    let vat_a = vats.iter().find(|v| v["label"] == "A").unwrap();
    assert_eq!(vat_b["totalAmount"], 68.0, "VAT B total");
    assert_eq!(vat_a["totalAmount"], 9.0, "VAT A total (net of the refund)");

    let departments = request["turnover"]["departments"].as_array().unwrap();
    let dept1 = departments
        .iter()
        .find(|d| d["departmentId"] == "1")
        .unwrap();
    let dept2 = departments
        .iter()
        .find(|d| d["departmentId"] == "2")
        .unwrap();
    assert_eq!(dept1["amount"], 68.0, "department 1 (Food) total");
    assert_eq!(
        dept2["amount"], 9.0,
        "department 2 (Drinks) total, net of the refund"
    );

    let payments = request["turnover"]["payments"].as_array().unwrap();
    let cash = payments.iter().find(|p| p["type"] == "CASH").unwrap();
    let card = payments.iter().find(|p| p["type"] == "CARD_DEBIT").unwrap();
    assert_eq!(
        cash["amount"][0]["totalAmount"], 56.0,
        "cash payments total"
    );
    assert_eq!(
        card["amount"][0]["totalAmount"], 21.0,
        "card payments total"
    );

    let cash_cents = (cash["amount"][0]["totalAmount"].as_f64().unwrap() * 100.0).round() as i64;
    let card_cents = (card["amount"][0]["totalAmount"].as_f64().unwrap() * 100.0).round() as i64;
    assert_eq!(
        cash_cents + card_cents,
        expected_total_cents,
        "cash + card payments must equal the day's net real sales"
    );

    let vat_total_cents =
        ((vat_b["totalAmount"].as_f64().unwrap() + vat_a["totalAmount"].as_f64().unwrap()) * 100.0)
            .round() as i64;
    assert_eq!(
        vat_total_cents, expected_total_cents,
        "VAT totals must reconcile with the day's net real sales"
    );
}
