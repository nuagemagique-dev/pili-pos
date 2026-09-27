//! SPF Finances Basic Set v1.8, replayed from the official fixtures.
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
//! 4. compares the request pili-pos built with the fixture, field by
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

use pili_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pili_pos::commands::copy::do_copy_ticket;
use pili_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use pili_pos::commands::order::{AddOrderLineInput, do_add_order_line};
use pili_pos::commands::refund::{RefundItem, do_refund_checkout};
use pili_pos::commands::social::{do_work_in, do_work_out};
use pili_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;

// ── Setup ──────────────────────────────────────────────────────────

/// A TestApp with the manual's catalogue and the fixture's operator.
async fn spf_app(operator_insz: &str) -> TestApp {
    let app = TestApp::new().await;
    {
        let conn = app.state.db.lock().unwrap();
        pili_pos::cli::apply_seed(&conn, include_str!("../seed_spf_test.sql")).expect("seed");
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

/// The archived requests built by pili-pos, oldest first. Also checks
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

/// An intended difference between pili-pos and the fixture.
struct Deviation {
    path: &'static str,
    reason: &'static str,
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
            json!({ "path": path, "piliPos": ours, "fixture": fixture, "reason": reason })
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

/// pili-pos sells dine-in orders through a table (Tables → order →
/// checkout), so a settled dine-in sale carries its table; the
/// fixture's "direct sale" has no cost center.
const TABLE_SALE: Deviation = Deviation {
    path: "costCenter.type",
    reason: "dine-in sales are settled from a table and carry costCenter TABLE",
};

#[tokio::test]
// covers: FAQ_Q505
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
// covers: FAQ_Q301
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
            reason: "pili-pos references the refunded ticket on every refund \
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
// covers: FAQ_Q401
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

/// M133: the cash counted in the drawer, declared as it is.
#[tokio::test]
async fn m133_drawer_declaration() {
    let vars = variables("M133");
    let app = spf_app(&operator(&vars)).await;
    mock_uc(&app, "M133", None).await;

    let amount = vars["financials"][0]["amount"].as_f64().unwrap();
    do_money_in_out(
        &app.state,
        operator(&vars),
        (amount * 100.0).round() as i64,
        MoneyDirection::Declaration,
        "Declaration tiroir".to_string(),
    )
    .await
    .unwrap();

    assert_matches_fixture("M133", &requests(&app)[0], &vars, &[]);
    app.dump_evidence("M133");
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

/// M171B is M171 with the query style of the changelog 1.7: the
/// fields that are always null in training are `@skip`ped, so the
/// FDM's answer has no `shortSignature`, `verificationUrl` or
/// `vatCalc` at all. The sale is signed and archived all the same.
#[tokio::test]
async fn m171b_sale_in_training_with_skipped_fields() {
    let vars = variables("M171B_signSale_Training");
    let app = spf_app(&operator(&vars)).await;
    set_training_mode(&app, true);
    let order_id = order_from(&app, &vars, OrderType::DineIn, Some(1));
    let envelope = read_response_envelope(&load_uc(&basic_set_root(), "M171B_signSale_Training"));
    let result_obj = envelope["data"]["signSale"].as_object().unwrap();
    for skipped in ["shortSignature", "verificationUrl", "vatCalc"] {
        assert!(
            !result_obj.contains_key(skipped),
            "M171B answers without {skipped}"
        );
    }
    mock_response(&app, envelope).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let event = &app.fiscal_events()[0];
    assert_eq!(event.event_type, "T");
    assert!(event.short_signature.is_none());

    assert_matches_fixture("M171B", &requests(&app)[0], &vars, &[TABLE_SALE]);
    app.dump_evidence("M171B");
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

// ── Advanced Set cases pili-pos supports ──────────────────────────

/// UC230: 3 Dry Martini ordered, corrected at once to 2, paid by debit
/// card. The operator lowers the quantity on the order screen; the
/// journal keeps the 3 and the -1, and the sale carries both, the -1
/// with negQuantityReason CORRECTION.
#[tokio::test]
// covers: FAQ_Q301
async fn uc230_a_line_corrected_before_payment() {
    let fixture = load_uc(&advanced_set_root(), "UC230");
    let vars = read_variables(&fixture)["data"].clone();
    let app = spf_app(&operator(&vars)).await;
    let order_id = order_from(&app, &vars, OrderType::DineIn, Some(1));
    let line = &pili_pos::commands::order::do_list_order_lines(&app.state, order_id).unwrap()[0];
    assert_eq!(line.quantity, 3);
    pili_pos::commands::order::do_update_order_line_quantity(&app.state, line.id, 2).unwrap();
    mock_from_fixture(&app, &fixture).await;

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let ours = &requests(&app)[0];
    assert_eq!(
        ours["transaction"]["transactionLines"][1]["mainProduct"]["negQuantityReason"],
        "CORRECTION"
    );
    assert_matches_fixture("UC230", ours, &vars, &[TABLE_SALE]);
    app.dump_evidence("UC230");
}

/// UC4119: table T1 ordered by John, then paid by Elisa a third each
/// in cash, by card and in meal vouchers, on one VAT ticket. Only the
/// cash third is rounded (+0,01).
#[tokio::test]
async fn uc4119_split_payment_on_one_ticket() {
    use pili_pos::commands::checkout::{PaymentPart, do_checkout_split};
    let order_uc = load_uc(&advanced_set_root(), "UC4119_SplitPayment/UC4119_1");
    let sale_uc = load_uc(&advanced_set_root(), "UC4119_SplitPayment/UC4119_2");
    let order_vars = read_variables(&order_uc)["data"].clone();
    let sale_vars = read_variables(&sale_uc)["data"].clone();
    let app = spf_app(&operator(&order_vars)).await;
    let order_id = order_from(&app, &order_vars, OrderType::DineIn, Some(1));
    mock_from_fixture(&app, &order_uc).await;
    do_sign_table_order(&app.state, order_id).await.unwrap();

    app.login_as(&operator(&sale_vars), pos_core::user::UserRole::Admin);
    mock_from_fixture(&app, &sale_uc).await;
    let result = do_checkout_split(
        &app.state,
        order_id,
        &[
            PaymentPart::cash(),
            PaymentPart::of(CheckoutPaymentMethod::Card, 3104),
            PaymentPart::of(CheckoutPaymentMethod::MealVoucher, 3104),
        ],
        None,
    )
    .await
    .unwrap();
    assert!(result.success, "{:?}", result.error);

    let r = requests(&app);
    assert_matches_fixture("UC4119_1", &r[0], &order_vars, &[]);
    assert_matches_fixture(
        "UC4119_2",
        &r[1],
        &sale_vars,
        &[Deviation {
            path: "financials[0].type",
            reason: "the operator does not tell debit from credit cards: CARD_DEBIT",
        }],
    );
    app.dump_evidence("UC4119");
}

/// UC4109: a business menu all-in ordered on table 1, then the
/// customer leaves without paying. A manager closes the table unpaid:
/// the sale is signed with no payment line.
// covers: UC4109
#[tokio::test]
async fn uc4109_unpaid_table() {
    use pili_pos::commands::auth::do_set_pin;
    use pili_pos::commands::unpaid::do_close_unpaid;
    let order_uc = load_uc(&advanced_set_root(), "UC4109_NoPayment/UC4109_1");
    let sale_uc = load_uc(&advanced_set_root(), "UC4109_NoPayment/UC4109_2");
    let order_vars = read_variables(&order_uc)["data"].clone();
    let sale_vars = read_variables(&sale_uc)["data"].clone();
    let app = spf_app(&operator(&order_vars)).await;
    // The menu is sold here as one product (the seed keeps it off the
    // menu: composite products are not offered).
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "UPDATE menu_items SET active = 1 WHERE id = 30010;
             UPDATE categories SET active = 1 WHERE id = 30;",
        )
        .unwrap();
    let order_id = order_from(&app, &order_vars, OrderType::DineIn, Some(1));
    mock_from_fixture(&app, &order_uc).await;
    do_sign_table_order(&app.state, order_id).await.unwrap();

    let manager = operator(&sale_vars);
    app.login_as(&manager, pos_core::user::UserRole::Admin);
    do_set_pin(&app.state, manager.clone(), "4321".into()).unwrap();
    mock_from_fixture(&app, &sale_uc).await;
    let result = do_close_unpaid(&app.state, order_id, &manager, "4321")
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    // The menu is one product here, not a composite of its dishes:
    // its VAT is on the line itself. `financials: []` matches exactly.
    let menu = [
        Deviation {
            path: "transaction.transactionLines[0].lineType",
            reason: "composite products are not supported: the menu is a single product",
        },
        Deviation {
            path: "transaction.transactionLines[0].vats",
            reason: "a single product carries its own VAT line (B 58,00)",
        },
    ];
    let r = requests(&app);
    assert_matches_fixture("UC4109_1", &r[0], &order_vars, &menu);
    assert_matches_fixture("UC4109_2", &r[1], &sale_vars, &menu);
    app.dump_evidence("UC4109");
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
