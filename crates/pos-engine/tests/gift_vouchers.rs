//! Multi-purpose gift vouchers (V12-D24 to V12-D31): sale as a
//! VAT X line, redemption as a `VOUCHER_STORE` payment, balance and
//! expiry, refunds, prints, lookup.
//!
//! Events are signed by the admission image's mock FDM; the fixture
//! replays of UC260 to UC262 are in `spf_e2e_gift_vouchers.rs`.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_db::queries::gift_vouchers as store;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{
    CheckoutPaymentMethod::{Card, MealVoucher},
    CheckoutResult, PaymentPart, do_checkout_split,
};
use pos_engine::commands::courses::{CourseSettings, do_set_course_settings, do_set_line_course};
use pos_engine::commands::errors;
use pos_engine::commands::gift_vouchers::{
    GiftVoucherSettings, do_add_gift_voucher_line, do_get_gift_voucher_settings,
    do_list_gift_vouchers, do_lookup_gift_voucher, do_print_gift_voucher,
    do_set_gift_voucher_settings, valid_code,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_remove_order_line,
    do_set_line_note, do_update_order_line_quantity,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::{do_generate_x_report, do_generate_z_report};
use pos_engine::commands::tables::do_sign_table_order;
use pos_fiscal::payment::PaymentMethod;
use pos_fiscal::types::EventLabel;

const MANAGER: &str = "12345678958";
const CASHIER: &str = "98765432109";
/// A 10,00 EUR dish (VAT B) and a 18,50 EUR one.
const DISH: i64 = 1;
const PRICEY: i64 = 2;
/// The system product's id (migration 63).
const VOUCHER_ITEM: i64 = 990_001;

fn on() -> GiftVoucherSettings {
    GiftVoucherSettings {
        enabled: true,
        ..GiftVoucherSettings::default()
    }
}

async fn app() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 1000, 'B', 0, 1),
                (2, 1, 1850, 'B', 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Plat'),
                ('menu_item', 2, 'name', 'fr', 'Grand plat');",
        )
        .unwrap();
    do_set_gift_voucher_settings(&app.state, on()).unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

fn new_order(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id
}

fn add(app: &TestApp, order: i64, item: i64) -> i64 {
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
    .unwrap()
    .id
}

async fn pay(app: &TestApp, order: i64, parts: &[PaymentPart]) -> CheckoutResult {
    do_checkout_split(&app.state, order, parts, None)
        .await
        .unwrap()
}

/// Sell a voucher of `cents` for cash; returns (sale event id, voucher id, order id).
async fn sell_voucher(app: &TestApp, cents: i64) -> (i64, i64, i64) {
    let order = new_order(app);
    do_add_gift_voucher_line(&app.state, order, cents).unwrap();
    let r = pay(app, order, &[PaymentPart::cash()]).await;
    assert!(r.success, "{:?}", r.error);
    let id = vouchers(app).last().unwrap().id;
    (r.fiscal_event_id.unwrap(), id, order)
}

fn vouchers(app: &TestApp) -> Vec<store::VoucherRow> {
    let conn = app.state.db.lock().unwrap();
    store::list(&conn, 1000)
        .unwrap()
        .into_iter()
        .rev()
        .collect()
}

fn voucher(app: &TestApp, id: i64) -> store::VoucherRow {
    let conn = app.state.db.lock().unwrap();
    store::get(&conn, id).unwrap().unwrap()
}

fn dish_order(app: &TestApp, item: i64) -> i64 {
    let order = new_order(app);
    add(app, order, item);
    order
}

fn request_of(app: &TestApp, event_id: i64) -> Value {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .unwrap();
    serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap()
}

fn payment_rows(app: &TestApp) -> Vec<(String, i64)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT method, amount_cents FROM payments ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// The signed report request holds exactly one VAT X turnover entry,
/// with no VAT, worth `expected` euros.
fn assert_vat_x_turnover(request: &Value, expected: f64) {
    let vats = request["turnover"]["vats"].as_array().unwrap();
    let x: Vec<&Value> = vats.iter().filter(|v| v["label"] == "X").collect();
    assert_eq!(x.len(), 1, "{vats:?}");
    assert_eq!(x[0]["totalAmount"].as_f64().unwrap(), expected, "{x:?}");
    assert_eq!(x[0]["vatAmount"].as_f64().unwrap(), 0.0, "{x:?}");
}

fn spaced(code: &str) -> String {
    format!("{} {} {}", &code[..4], &code[4..8], &code[8..])
}

// ── Settings ───────────────────────────────────────────────────────

#[tokio::test]
async fn the_setting_is_off_by_default_and_checked() {
    let app = TestApp::new().await;
    let s = do_get_gift_voucher_settings(&app.state).unwrap();
    assert_eq!(
        s,
        GiftVoucherSettings {
            enabled: false,
            validity_months: 12,
            max_amount_cents: 50_000
        }
    );
    for (months, max, field) in [
        (11, 50_000, "validity_months"),
        (37, 50_000, "validity_months"),
        (12, 99, "max_amount_cents"),
        (12, 1_000_001, "max_amount_cents"),
    ] {
        let err = do_set_gift_voucher_settings(
            &app.state,
            GiftVoucherSettings {
                enabled: true,
                validity_months: months,
                max_amount_cents: max,
            },
        )
        .unwrap_err();
        assert_eq!(err, format!("gift_voucher_settings_invalid|field={field}"));
    }
    for months in [12, 24, 36] {
        let stored = do_set_gift_voucher_settings(
            &app.state,
            GiftVoucherSettings {
                enabled: true,
                validity_months: months,
                max_amount_cents: 20_000,
            },
        )
        .unwrap();
        assert_eq!(do_get_gift_voucher_settings(&app.state).unwrap(), stored);
    }
}

#[tokio::test]
async fn only_a_manager_changes_the_setting_and_a_cashier_sells_and_looks_up() {
    let (app, _printer) = app().await;
    let order = new_order(&app);
    app.login_as(CASHIER, UserRole::Cashier);
    assert_eq!(
        do_set_gift_voucher_settings(&app.state, on()).unwrap_err(),
        errors::FORBIDDEN
    );
    assert!(do_get_gift_voucher_settings(&app.state).is_ok());
    assert!(do_add_gift_voucher_line(&app.state, order, 2500).is_ok());
    assert_eq!(
        do_list_gift_vouchers(&app.state, None).unwrap_err(),
        errors::FORBIDDEN
    );
    assert_eq!(
        do_print_gift_voucher(&app.state, 1).await.unwrap_err(),
        errors::FORBIDDEN
    );
    // A waiter cannot sell one.
    app.login_as("11111111111", UserRole::Waiter);
    assert_eq!(
        do_add_gift_voucher_line(&app.state, order, 2500).unwrap_err(),
        errors::FORBIDDEN
    );
}

// ── The voucher line ───────────────────────────────────────────────

#[tokio::test]
async fn a_voucher_line_is_vat_x_quantity_one_in_department_gv() {
    let (app, _printer) = app().await;
    let order = new_order(&app);
    let line = do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    assert_eq!(line.menu_item_id, VOUCHER_ITEM);
    assert_eq!(line.quantity, 1);
    assert_eq!(line.unit_price_cents, 5000);
    assert_eq!(line.vat_code.as_str(), "X");
    assert_eq!(line.name, "Bon cadeau");
    assert_eq!(line.department_id, "GV");
    assert_eq!(line.department_name, "Bons cadeaux");
    let o = pos_engine::commands::order::do_get_order(&app.state, order).unwrap();
    assert_eq!(o.total_cents, 5000);
}

#[tokio::test]
async fn the_line_names_follow_the_establishment_language() {
    let (app, _printer) = app().await;
    app.state.config.lock().unwrap().language = "nl".into();
    let order = new_order(&app);
    let line = do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    assert_eq!(
        (line.name.as_str(), line.department_name.as_str()),
        ("Cadeaubon", "Cadeaubonnen")
    );
}

#[tokio::test]
async fn amounts_are_checked_and_the_switch_is_honoured() {
    let (app, _printer) = app().await;
    let order = new_order(&app);
    for bad in [-100, 0, 99, 50_001] {
        let err = do_add_gift_voucher_line(&app.state, order, bad).unwrap_err();
        assert_eq!(err, "gift_voucher_amount_invalid|max=500,00", "{bad}");
    }
    assert!(do_add_gift_voucher_line(&app.state, order, 100).is_ok());
    assert!(do_add_gift_voucher_line(&app.state, order, 50_000).is_ok());
    do_set_gift_voucher_settings(&app.state, GiftVoucherSettings::default()).unwrap();
    assert_eq!(
        do_add_gift_voucher_line(&app.state, order, 2000).unwrap_err(),
        errors::GIFT_VOUCHERS_DISABLED
    );
    // The max setting moves the ceiling.
    do_set_gift_voucher_settings(
        &app.state,
        GiftVoucherSettings {
            enabled: true,
            validity_months: 12,
            max_amount_cents: 10_000,
        },
    )
    .unwrap();
    assert!(do_add_gift_voucher_line(&app.state, order, 10_000).is_ok());
    assert!(do_add_gift_voucher_line(&app.state, order, 10_001).is_err());
}

#[tokio::test]
async fn the_quantity_of_a_voucher_line_cannot_grow_but_the_line_can_go() {
    let (app, _printer) = app().await;
    let order = new_order(&app);
    let line = do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    for quantity in [1, 2, 5] {
        assert_eq!(
            do_update_order_line_quantity(&app.state, line.id, quantity).unwrap_err(),
            errors::GIFT_VOUCHER_LINE_FIXED
        );
    }
    do_remove_order_line(&app.state, line.id).unwrap();
    let o = pos_engine::commands::order::do_get_order(&app.state, order).unwrap();
    assert_eq!(o.total_cents, 0);
    // The system product is not on the menu: it cannot be added as a plain product.
    assert!(
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: VOUCHER_ITEM,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents: Some(5000),
            }
        )
        .is_err()
    );
    let menu = {
        let conn = app.state.db.lock().unwrap();
        pos_db::queries::menu::get_menu_items(&conn, None, "fr").unwrap()
    };
    assert!(menu.iter().all(|m| m.id != VOUCHER_ITEM));
}

// ── Selling: issue at the N commit ─────────────────────────────────

#[tokio::test]
async fn a_sale_issues_the_voucher_with_its_code_and_dates() {
    let (app, _printer) = app().await;
    let (event, id, _) = sell_voucher(&app, 5000).await;
    let v = voucher(&app, id);
    assert_eq!(v.face_value_cents, 5000);
    assert_eq!(v.balance_cents, 5000);
    assert!(!v.training && !v.cancelled);
    assert!(valid_code(&v.code), "{}", v.code);
    assert_eq!(v.issue_event_id, event);
    let today = pos_fiscal::event::now_in_belgium().date_naive();
    assert_eq!(v.issued_on, today.to_string());
    assert_eq!(
        v.expires_on,
        today
            .checked_add_months(chrono::Months::new(12))
            .unwrap()
            .to_string()
    );
    let conn = app.state.db.lock().unwrap();
    let moves = store::movements(&conn, id).unwrap();
    assert_eq!(moves.len(), 1);
    assert_eq!(
        (moves[0].kind.as_str(), moves[0].amount_cents),
        ("issue", 5000)
    );
}

#[tokio::test]
async fn the_validity_follows_the_setting() {
    let (app, _printer) = app().await;
    do_set_gift_voucher_settings(
        &app.state,
        GiftVoucherSettings {
            enabled: true,
            validity_months: 24,
            max_amount_cents: 50_000,
        },
    )
    .unwrap();
    let (_, id, _) = sell_voucher(&app, 3000).await;
    let today = pos_fiscal::event::now_in_belgium().date_naive();
    assert_eq!(
        voucher(&app, id).expires_on,
        today
            .checked_add_months(chrono::Months::new(24))
            .unwrap()
            .to_string()
    );
}

#[tokio::test]
async fn the_signed_sale_carries_a_vat_x_line_and_no_code() {
    let (app, _printer) = app().await;
    let (event, id, _) = sell_voucher(&app, 5000).await;
    let request = request_of(&app, event);
    let line = &request["transaction"]["transactionLines"][0]["mainProduct"];
    assert_eq!(line["vats"][0]["label"], "X");
    assert_eq!(line["unitPrice"], 50.0);
    assert_eq!(line["departmentId"], "GV");
    assert_eq!(line["productName"], "Bon cadeau");
    assert_eq!(request["transaction"]["transactionTotal"], 50.0);
    // The code is on the voucher only: not in any archived request, in
    // the stored sign request, nor in the FDM answer.
    let code = voucher(&app, id).code;
    for needle in [code.clone(), spaced(&code)] {
        for e in app.fiscal_events() {
            assert!(!e.event_data_json.unwrap_or_default().contains(&needle));
            assert!(!e.fdm_response_json.unwrap_or_default().contains(&needle));
        }
        let conn = app.state.db.lock().unwrap();
        let bodies: Vec<String> = conn
            .prepare("SELECT request_body FROM pending_signs")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert!(bodies.iter().all(|b| !b.contains(&needle)));
    }
}

#[tokio::test]
async fn a_voucher_bought_by_card_or_cash_with_a_dish_is_fine_and_each_line_is_a_voucher() {
    let (app, _printer) = app().await;
    let order = dish_order(&app, DISH);
    do_add_gift_voucher_line(&app.state, order, 2000).unwrap();
    do_add_gift_voucher_line(&app.state, order, 3000).unwrap();
    let r = pay(&app, order, &[PaymentPart::of(Card, 6000)]).await;
    assert!(r.success, "{:?}", r.error);
    let all = vouchers(&app);
    assert_eq!(
        all.iter().map(|v| v.face_value_cents).collect::<Vec<_>>(),
        [2000, 3000]
    );
    assert_ne!(all[0].code, all[1].code);
    // Two voucher slips came out.
    assert_eq!(
        r.gift_voucher_prints
            .iter()
            .filter(|p| p.kind == "voucher")
            .count(),
        2
    );
}

#[tokio::test]
async fn a_voucher_cannot_be_paid_with_meal_vouchers_or_a_gift_voucher_or_left_unpaid() {
    let (app, _printer) = app().await;
    let (_, paying, _) = sell_voucher(&app, 5000).await;
    let events = app.fiscal_events().len();
    let order = new_order(&app);
    do_add_gift_voucher_line(&app.state, order, 2000).unwrap();
    for parts in [
        vec![PaymentPart::of(MealVoucher, 2000)],
        vec![PaymentPart::gift_voucher(2000, paying)],
        vec![PaymentPart::of(MealVoucher, 500), PaymentPart::cash()],
    ] {
        let r = pay(&app, order, &parts).await;
        assert!(!r.success);
        assert_eq!(
            r.error.as_deref(),
            Some(errors::GIFT_VOUCHER_PAYMENT_INVALID)
        );
    }
    assert_eq!(app.fiscal_events().len(), events, "nothing signed");
    assert_eq!(app.order_status(order).as_deref(), Some("open"));
    // Cash and card are fine.
    assert!(
        pay(&app, order, &[PaymentPart::of(Card, 2000)])
            .await
            .success
    );
}

#[tokio::test]
async fn an_unpaid_close_cannot_issue_a_voucher() {
    use pos_engine::commands::auth::do_set_pin;
    use pos_engine::commands::unpaid::do_close_unpaid;
    let (app, _printer) = app().await;
    do_set_pin(&app.state, MANAGER.into(), "4321".into(), None).unwrap();
    let order = new_order(&app);
    do_add_gift_voucher_line(&app.state, order, 2000).unwrap();
    let r = do_close_unpaid(&app.state, order, MANAGER, "4321")
        .await
        .unwrap();
    assert!(!r.success);
    assert_eq!(
        r.error.as_deref(),
        Some(errors::GIFT_VOUCHER_PAYMENT_INVALID)
    );
    assert_eq!(vouchers(&app).len(), 0);
    assert!(app.fiscal_events().is_empty());
}

// ── FDM failure and recovery ───────────────────────────────────────

#[tokio::test]
async fn an_fdm_that_cannot_be_reached_issues_nothing() {
    let (app, printer) = app().await;
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();
    let order = new_order(&app);
    do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    let r = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(!r.success);
    assert_eq!(vouchers(&app).len(), 0);
    assert_eq!(r.gift_voucher_prints.len(), 0);
    assert_eq!(app.order_status(order).as_deref(), Some("open"));
    assert_eq!(printer.settle(0).await.len(), 0);
}

#[tokio::test]
async fn a_lost_answer_recovered_later_issues_the_same_code_once() {
    let (app, printer) = app().await;
    let fdm = LossyFdm::start().await;
    app.use_lossy_fdm(&fdm);
    fdm.lose_first("signSale");
    let order = new_order(&app);
    do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    let first = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(!first.success);
    // Nothing is issued while the answer is awaited, but the code is
    // already fixed in the stored request.
    assert_eq!(vouchers(&app).len(), 0);
    let pending = app.open_pending().expect("the request stays open");
    let stored: Value = serde_json::from_str(&pending.context_json).unwrap();
    let code = stored["gift_vouchers_issued"][0]["code"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(valid_code(&code));

    // The same sale, sent again: the FDM returns its first answer, the
    // sale is archived and the voucher created with that code, once.
    let second = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(second.success, "{:?}", second.error);
    let all = vouchers(&app);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].code, code);
    assert_eq!(fdm.sent_op("signSale").len(), 2);
    assert_eq!(app.fiscal_events().len(), 1);
    // The slip printed once, with the same code.
    for _ in 0..40 {
        if printer.rendered().iter().any(|r| r.contains("BON CADEAU")) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let slips: Vec<String> = printer
        .rendered()
        .into_iter()
        .filter(|r| r.contains("BON CADEAU"))
        .collect();
    assert_eq!(slips.len(), 1);
    assert!(slips[0].contains(&spaced(&code)));
}

// ── Redeeming ──────────────────────────────────────────────────────

#[tokio::test]
async fn a_partial_use_debits_the_balance_and_prints_a_balance_slip() {
    let (app, printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    printer.settle(1).await;
    let order = dish_order(&app, PRICEY);
    let r = pay(&app, order, &[PaymentPart::gift_voucher(1850, id)]).await;
    assert!(r.success, "{:?}", r.error);

    assert_eq!(voucher(&app, id).balance_cents, 3150);
    let request = request_of(&app, r.fiscal_event_id.unwrap());
    let f = &request["financials"][0];
    assert_eq!(f["type"], "VOUCHER_STORE");
    assert_eq!(f["inputMethod"], "MANUAL");
    assert_eq!(f["amount"], 18.5);
    assert_eq!(f["id"], "8");
    assert_eq!(request["financials"].as_array().unwrap().len(), 1);
    assert!(payment_rows(&app).contains(&("other".to_string(), 1850)));

    // The VAT ticket names the payment, the balance slip masks the code.
    assert_eq!(
        r.gift_voucher_prints
            .iter()
            .map(|p| (p.kind.as_str(), p.outcome))
            .collect::<Vec<_>>(),
        [("balance", SlipOutcome::Printed)]
    );
    printer.settle(3).await;
    let texts = printer.rendered().join("\n----\n");
    assert!(texts.contains("Bon cadeau"), "{texts}");
    assert!(texts.contains("SOLDE DU BON"), "{texts}");
    assert!(texts.contains("Solde: 31,50 EUR"), "{texts}");
    let code = voucher(&app, id).code;
    let slip = printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("SOLDE DU BON"))
        .unwrap();
    assert!(slip.contains(&format!("**** **** {}", &code[8..])));
    assert!(!slip.contains(&code[..8]));

    let conn = app.state.db.lock().unwrap();
    let moves = store::movements(&conn, id).unwrap();
    assert_eq!(
        moves
            .iter()
            .map(|m| (m.kind.as_str(), m.amount_cents))
            .collect::<Vec<_>>(),
        [("issue", 5000), ("redeem", -1850)]
    );
    assert_eq!(store::count_prints(&conn, id, "balance").unwrap(), 1);
    assert_eq!(store::count_prints(&conn, id, "original").unwrap(), 1);
}

#[tokio::test]
async fn a_voucher_used_to_the_last_cent_is_used_and_prints_no_balance_slip() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 1850).await;
    let order = dish_order(&app, PRICEY);
    let r = pay(&app, order, &[PaymentPart::gift_voucher(1850, id)]).await;
    assert!(r.success, "{:?}", r.error);
    assert_eq!(voucher(&app, id).balance_cents, 0);
    assert_eq!(r.gift_voucher_prints.len(), 0);
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, id).code).unwrap();
    assert_eq!(view.status, "used");
    // A used voucher can pay nothing more.
    let again = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(100, id), PaymentPart::cash()],
    )
    .await;
    assert_eq!(
        again.error.as_deref(),
        Some(errors::GIFT_VOUCHER_INSUFFICIENT)
    );
}

#[tokio::test]
async fn a_voucher_and_cash_split_the_bill_and_cash_alone_is_rounded() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 500).await;
    // 18,50 EUR: 5,00 on the voucher, the rest 13,50 in cash.
    let order = dish_order(&app, PRICEY);
    let r = pay(
        &app,
        order,
        &[PaymentPart::gift_voucher(500, id), PaymentPart::cash()],
    )
    .await;
    assert!(r.success, "{:?}", r.error);
    let request = request_of(&app, r.fiscal_event_id.unwrap());
    let types: Vec<(String, f64)> = request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["type"].as_str().unwrap().to_string(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        types,
        [
            ("VOUCHER_STORE".to_string(), 5.0),
            ("CASH".to_string(), 13.5)
        ]
    );
    assert_eq!(voucher(&app, id).balance_cents, 0);
}

#[tokio::test]
async fn two_vouchers_pay_one_sale() {
    let (app, _printer) = app().await;
    let (_, a, _) = sell_voucher(&app, 1000).await;
    let (_, b, _) = sell_voucher(&app, 2000).await;
    let order = dish_order(&app, PRICEY);
    let r = pay(
        &app,
        order,
        &[
            PaymentPart::gift_voucher(1000, a),
            PaymentPart::gift_voucher(850, b),
        ],
    )
    .await;
    assert!(r.success, "{:?}", r.error);
    assert_eq!(voucher(&app, a).balance_cents, 0);
    assert_eq!(voucher(&app, b).balance_cents, 1150);
    let request = request_of(&app, r.fiscal_event_id.unwrap());
    assert_eq!(request["financials"].as_array().unwrap().len(), 2);
    // Only the voucher with a balance left gets a balance slip.
    assert_eq!(r.gift_voucher_prints.len(), 1);
    assert_eq!(r.gift_voucher_prints[0].voucher_id, b);
}

#[tokio::test]
async fn a_voucher_worth_more_than_the_bill_pays_the_bill_and_keeps_the_balance() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    // The part is capped by the bill: more than the bill is refused, the
    // exact bill is a part of the bill and no change is given.
    let order = dish_order(&app, DISH);
    let over = pay(&app, order, &[PaymentPart::gift_voucher(5000, id)]).await;
    assert_eq!(over.error.as_deref(), Some(errors::PAYMENT_EXCEEDS_TOTAL));
    assert_eq!(voucher(&app, id).balance_cents, 5000);
    let ok = pay(&app, order, &[PaymentPart::gift_voucher(1000, id)]).await;
    assert!(ok.success, "{:?}", ok.error);
    assert_eq!(voucher(&app, id).balance_cents, 4000);
    let request = request_of(&app, ok.fiscal_event_id.unwrap());
    assert_eq!(request["financials"].as_array().unwrap().len(), 1);
    assert_eq!(request["financials"][0]["amount"], 10.0);
}

#[tokio::test]
async fn refused_vouchers_debit_nothing_and_sign_nothing() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 500).await;
    let events = app.fiscal_events().len();
    let order = dish_order(&app, DISH);
    for (parts, code) in [
        (
            vec![PaymentPart::gift_voucher(600, id), PaymentPart::cash()],
            errors::GIFT_VOUCHER_INSUFFICIENT,
        ),
        (
            vec![PaymentPart::gift_voucher(100, 999), PaymentPart::cash()],
            errors::GIFT_VOUCHER_NOT_FOUND,
        ),
        (
            vec![
                PaymentPart::gift_voucher(100, id),
                PaymentPart::gift_voucher(100, id),
                PaymentPart::cash(),
            ],
            errors::GIFT_VOUCHER_PAYMENT_INVALID,
        ),
        // A gift voucher part without a voucher.
        (
            vec![
                PaymentPart::of(PaymentMethod::GiftVoucher, 100),
                PaymentPart::cash(),
            ],
            errors::PAYMENT_INVALID,
        ),
    ] {
        let r = pay(&app, order, &parts).await;
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some(code));
    }
    assert_eq!(app.fiscal_events().len(), events);
    assert_eq!(voucher(&app, id).balance_cents, 500);
    assert_eq!(app.order_status(order).as_deref(), Some("open"));
}

#[tokio::test]
async fn an_expired_voucher_is_refused_but_can_still_be_looked_up() {
    let (app, _printer) = app().await;
    let (event, _, order) = sell_voucher(&app, 5000).await;
    // A voucher whose validity ended yesterday (rows are append-only, so
    // it is inserted that way).
    let yesterday =
        (pos_fiscal::event::now_in_belgium().date_naive() - chrono::Days::new(1)).to_string();
    let id = {
        let conn = app.state.db.lock().unwrap();
        store::insert_voucher(
            &conn,
            &store::NewVoucher {
                code: &pos_engine::commands::gift_vouchers::make_code(1_111_111_111),
                face_value_cents: 3000,
                issued_on: "2025-01-01",
                expires_on: &yesterday,
                training: false,
                issue_event_id: event,
                issue_order_id: order,
                issue_line_id: 9999,
            },
        )
        .unwrap()
    };
    let r = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, id)],
    )
    .await;
    assert_eq!(r.error.as_deref(), Some(errors::GIFT_VOUCHER_EXPIRED));
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, id).code).unwrap();
    assert_eq!(view.status, "expired");
    assert_eq!(view.balance_cents, 3000);
    // The last day is still valid.
    let today = pos_fiscal::event::now_in_belgium().date_naive().to_string();
    let last_day = {
        let conn = app.state.db.lock().unwrap();
        store::insert_voucher(
            &conn,
            &store::NewVoucher {
                code: &pos_engine::commands::gift_vouchers::make_code(2_222_222_222),
                face_value_cents: 3000,
                issued_on: "2025-01-01",
                expires_on: &today,
                training: false,
                issue_event_id: event,
                issue_order_id: order,
                issue_line_id: 9998,
            },
        )
        .unwrap()
    };
    let ok = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, last_day)],
    )
    .await;
    assert!(ok.success, "{:?}", ok.error);
}

#[tokio::test]
async fn training_vouchers_and_normal_vouchers_stay_apart() {
    let (app, _printer) = app().await;
    let (_, real, _) = sell_voucher(&app, 5000).await;

    app.state.config.lock().unwrap().training_mode = true;
    let (_, trained, _) = sell_voucher(&app, 4000).await;
    assert!(voucher(&app, trained).training);
    // A training sale leaves no payment row, a real one never uses a training voucher.
    let r = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, real)],
    )
    .await;
    assert_eq!(r.error.as_deref(), Some(errors::GIFT_VOUCHER_NOT_USABLE));
    let ok = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, trained)],
    )
    .await;
    assert!(ok.success, "{:?}", ok.error);
    assert_eq!(voucher(&app, trained).balance_cents, 3000);

    app.state.config.lock().unwrap().training_mode = false;
    let r = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, trained)],
    )
    .await;
    assert_eq!(r.error.as_deref(), Some(errors::GIFT_VOUCHER_NOT_USABLE));
    assert_eq!(voucher(&app, real).balance_cents, 5000);
}

#[tokio::test]
async fn lookup_flags_a_voucher_of_the_other_mode_as_not_usable() {
    let (app, _printer) = app().await;
    let (_, real, _) = sell_voucher(&app, 5000).await;
    app.state.config.lock().unwrap().training_mode = true;
    let (_, trained, _) = sell_voucher(&app, 4000).await;

    // Training mode: the training voucher is usable, the real one is not.
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, trained).code).unwrap();
    assert_eq!(view.status, "active");
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, real).code).unwrap();
    assert_eq!(view.status, "not_usable");
    assert_eq!(view.balance_cents, 5000, "the balance is still shown");

    // Normal mode: the other way round.
    app.state.config.lock().unwrap().training_mode = false;
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, trained).code).unwrap();
    assert_eq!(view.status, "not_usable");
    assert!(view.training);
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, real).code).unwrap();
    assert_eq!(view.status, "active");

    // The manager list keeps the real statuses.
    let listed = do_list_gift_vouchers(&app.state, None).unwrap();
    assert!(listed.iter().all(|v| v.status != "not_usable"));
}

// ── Refunds ────────────────────────────────────────────────────────

fn refund_item(menu_item_id: i64) -> Vec<RefundItem> {
    vec![RefundItem {
        line_id: None,
        menu_item_id,
        quantity: 1,
    }]
}

#[tokio::test]
async fn the_refund_of_an_unused_voucher_sale_cancels_it() {
    let (app, _printer) = app().await;
    let (event, id, _) = sell_voucher(&app, 5000).await;
    let r = do_refund_checkout(&app.state, event, refund_item(VOUCHER_ITEM), MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    let v = voucher(&app, id);
    assert!(v.cancelled);
    assert_eq!(v.balance_cents, 0);
    {
        let conn = app.state.db.lock().unwrap();
        let moves = store::movements(&conn, id).unwrap();
        assert_eq!(
            moves
                .iter()
                .map(|m| (m.kind.as_str(), m.amount_cents))
                .collect::<Vec<_>>(),
            [("issue", 5000), ("cancel", -5000)]
        );
    }
    let view = do_lookup_gift_voucher(&app.state, &voucher(&app, id).code).unwrap();
    assert_eq!(view.status, "cancelled");
    // It can no longer pay.
    let r = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(500, id), PaymentPart::cash()],
    )
    .await;
    assert_eq!(r.error.as_deref(), Some(errors::GIFT_VOUCHER_NOT_USABLE));
}

#[tokio::test]
async fn the_refund_of_a_used_voucher_sale_is_refused_and_signs_nothing() {
    let (app, _printer) = app().await;
    let (event, id, _) = sell_voucher(&app, 5000).await;
    let used = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(100, id), PaymentPart::cash()],
    )
    .await;
    assert!(used.success, "{:?}", used.error);
    let events = app.fiscal_events().len();
    let counter = app.counter_for(EventLabel::N);
    let r = do_refund_checkout(&app.state, event, refund_item(VOUCHER_ITEM), MANAGER.into()).await;
    let refused = match r {
        Ok(r) => r.error,
        Err(e) => Some(e),
    };
    assert_eq!(refused.as_deref(), Some(errors::GIFT_VOUCHER_USED));
    assert_eq!(app.fiscal_events().len(), events);
    assert_eq!(app.counter_for(EventLabel::N), counter);
    assert_eq!(voucher(&app, id).balance_cents, 4900);
    assert!(!voucher(&app, id).cancelled);
}

#[tokio::test]
async fn two_vouchers_on_one_sale_refund_one_at_a_time() {
    let (app, _printer) = app().await;
    let order = new_order(&app);
    do_add_gift_voucher_line(&app.state, order, 2000).unwrap();
    do_add_gift_voucher_line(&app.state, order, 2000).unwrap();
    let sale = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(sale.success, "{:?}", sale.error);
    let event = sale.fiscal_events_id();
    let [a, b] = <[store::VoucherRow; 2]>::try_from(vouchers(&app)).unwrap();
    // The first voucher is used.
    assert!(
        pay(
            &app,
            dish_order(&app, DISH),
            &[PaymentPart::gift_voucher(500, a.id), PaymentPart::cash()]
        )
        .await
        .success
    );
    // One refund: the unused voucher goes, the used one stays.
    let r = do_refund_checkout(&app.state, event, refund_item(VOUCHER_ITEM), MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert!(voucher(&app, b.id).cancelled);
    assert!(!voucher(&app, a.id).cancelled);
    // The second refund would need the used one: refused.
    let r = do_refund_checkout(&app.state, event, refund_item(VOUCHER_ITEM), MANAGER.into()).await;
    let refused = match r {
        Ok(r) => r.error,
        Err(e) => Some(e),
    };
    assert!(
        refused.as_deref() == Some(errors::GIFT_VOUCHER_USED)
            || refused
                .as_deref()
                .is_some_and(|e| e.starts_with("refund_exceeds_sold")),
        "{refused:?}"
    );
}

trait EventId {
    fn fiscal_events_id(&self) -> i64;
}

impl EventId for CheckoutResult {
    fn fiscal_events_id(&self) -> i64 {
        self.fiscal_event_id.unwrap()
    }
}

#[tokio::test]
async fn the_refund_of_a_sale_paid_by_voucher_is_cash_and_leaves_the_balance() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    let sale = pay(
        &app,
        dish_order(&app, PRICEY),
        &[PaymentPart::gift_voucher(1850, id)],
    )
    .await;
    assert!(sale.success, "{:?}", sale.error);
    let r = do_refund_checkout(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        refund_item(PRICEY),
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);
    let request = request_of(&app, r.fiscal_event_id.unwrap());
    assert_eq!(request["financials"][0]["type"], "CASH");
    assert_eq!(request["financials"][0]["amount"], -18.5);
    // The voucher is not re-credited.
    assert_eq!(voucher(&app, id).balance_cents, 3150);
}

// ── Reports ────────────────────────────────────────────────────────

#[tokio::test]
async fn the_x_report_totals_vat_x_and_the_voucher_payment_and_prints_its_label() {
    let (app, printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    let sale = pay(
        &app,
        dish_order(&app, PRICEY),
        &[PaymentPart::gift_voucher(1850, id)],
    )
    .await;
    assert!(sale.success, "{:?}", sale.error);
    printer.settle(3).await;
    let x = do_generate_x_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);

    let report = app.fiscal_events().pop().unwrap();
    let request: Value = serde_json::from_str(report.event_data_json.as_deref().unwrap()).unwrap();
    let payments: Vec<(String, f64)> = request["turnover"]["payments"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| {
            p["amount"].as_array().unwrap().iter().map(|a| {
                (
                    p["type"].as_str().unwrap().to_string(),
                    a["totalAmount"].as_f64().unwrap(),
                )
            })
        })
        .collect();
    assert!(
        payments.contains(&("VOUCHER_STORE".to_string(), 18.5)),
        "{payments:?}"
    );
    assert!(
        payments.contains(&("CASH".to_string(), 50.0)),
        "{payments:?}"
    );
    assert_vat_x_turnover(&request, 50.0);

    for _ in 0..40 {
        if printer
            .rendered()
            .iter()
            .any(|r| r.contains("Bon cadeau") && r.contains("RAPPORT"))
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let texts = printer.rendered();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("Bon cadeau") && t.contains("18,50")),
        "{texts:#?}"
    );
}

// ── Lookup, list, prints ───────────────────────────────────────────

#[tokio::test]
async fn lookup_reads_spaced_codes_and_shows_a_masked_code_with_its_movements() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    assert!(
        pay(
            &app,
            dish_order(&app, PRICEY),
            &[PaymentPart::gift_voucher(1850, id)]
        )
        .await
        .success
    );
    let code = voucher(&app, id).code;
    for typed in [code.clone(), spaced(&code), format!(" {code} ")] {
        let view = do_lookup_gift_voucher(&app.state, &typed).unwrap();
        assert_eq!(view.id, id);
        assert_eq!(view.code_masked, format!("**** **** {}", &code[8..]));
        assert_eq!(view.face_value_cents, 5000);
        assert_eq!(view.balance_cents, 3150);
        assert_eq!(view.status, "active");
        assert!(!view.training);
        assert_eq!(
            view.movements
                .iter()
                .map(|m| m.kind.as_str())
                .collect::<Vec<_>>(),
            ["issue", "redeem"]
        );
        assert!(view.movements[0].ticket.starts_with("N "));
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains(&code));
    }
}

#[tokio::test]
async fn lookup_checks_the_digits_and_throttles_failures() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    let code = voucher(&app, id).code;
    // Shape and check digits.
    assert_eq!(
        do_lookup_gift_voucher(&app.state, "1234").unwrap_err(),
        errors::GIFT_VOUCHER_CODE_INVALID
    );
    let mut wrong = code.clone();
    let last = wrong.pop().unwrap();
    wrong.push(if last == '0' { '1' } else { '0' });
    assert_eq!(
        do_lookup_gift_voucher(&app.state, &wrong).unwrap_err(),
        errors::GIFT_VOUCHER_CODE_INVALID
    );
    // A valid code nobody has.
    let unknown = pos_engine::commands::gift_vouchers::make_code(7_777_777_777);
    assert_eq!(
        do_lookup_gift_voucher(&app.state, &unknown).unwrap_err(),
        errors::GIFT_VOUCHER_NOT_FOUND
    );
    // Three failures so far; successes do not count.
    for _ in 0..10 {
        assert!(do_lookup_gift_voucher(&app.state, &code).is_ok());
    }
    assert_eq!(
        do_lookup_gift_voucher(&app.state, &unknown).unwrap_err(),
        errors::GIFT_VOUCHER_NOT_FOUND
    );
    assert_eq!(
        do_lookup_gift_voucher(&app.state, "0000").unwrap_err(),
        errors::GIFT_VOUCHER_CODE_INVALID
    );
    // The fifth failure was the last allowed: now even a good code waits.
    let err = do_lookup_gift_voucher(&app.state, &code).unwrap_err();
    assert!(
        err.starts_with("gift_voucher_lookup_throttled|seconds="),
        "{err}"
    );
}

#[tokio::test]
async fn the_admin_list_puts_outstanding_vouchers_first_and_filters() {
    let (app, _printer) = app().await;
    let (_, used, _) = sell_voucher(&app, 1850).await;
    let (_, open, _) = sell_voucher(&app, 5000).await;
    let (cancel_event, cancelled, _) = sell_voucher(&app, 2000).await;
    assert!(
        pay(
            &app,
            dish_order(&app, PRICEY),
            &[PaymentPart::gift_voucher(1850, used)]
        )
        .await
        .success
    );
    assert!(
        do_refund_checkout(
            &app.state,
            cancel_event,
            refund_item(VOUCHER_ITEM),
            MANAGER.into()
        )
        .await
        .unwrap()
        .success
    );

    let all = do_list_gift_vouchers(&app.state, None).unwrap();
    assert_eq!(
        all.iter()
            .map(|v| (v.id, v.status.as_str()))
            .collect::<Vec<_>>(),
        [(open, "active"), (used, "used"), (cancelled, "cancelled")]
    );
    for v in &all {
        assert!(v.code_masked.starts_with("**** **** "));
    }
    for (filter, expected) in [("active", open), ("used", used), ("cancelled", cancelled)] {
        let list = do_list_gift_vouchers(&app.state, Some(filter.into())).unwrap();
        assert_eq!(list.iter().map(|v| v.id).collect::<Vec<_>>(), [expected]);
    }
    assert_eq!(
        do_list_gift_vouchers(&app.state, Some("expired".into()))
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        do_list_gift_vouchers(&app.state, Some("all".into()))
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn the_voucher_slip_and_its_duplicate_print_with_the_code_and_dates() {
    let (app, printer) = app().await;
    let (event, id, _) = sell_voucher(&app, 5000).await;
    let v = voucher(&app, id);
    for _ in 0..40 {
        if printer.rendered().iter().any(|r| r.contains("BON CADEAU")) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let original = printer
        .rendered()
        .into_iter()
        .find(|r| r.contains("BON CADEAU"))
        .expect("the voucher slip");
    let one = pos_print::render::one_line(&original);
    assert!(one.contains("Valeur: 50,00 EUR"), "{one}");
    assert!(one.contains(&spaced(&v.code)), "{one}");
    assert!(one.contains("Valable jusqu'au"), "{one}");
    assert!(one.contains("sans rendu de monnaie"), "{one}");
    assert!(
        one.contains("CECI N'EST PAS UN TICKET DE CAISSE T.V.A."),
        "{one}"
    );
    assert!(
        one.contains(&format!(
            "Ticket: N {}",
            app.fiscal_events()
                .iter()
                .find(|e| e.id == event)
                .unwrap()
                .sequence_number
        )),
        "{one}"
    );
    assert!(!one.contains("DUPLICATA"));

    let outcome = do_print_gift_voucher(&app.state, id).await.unwrap();
    assert_eq!(outcome, SlipOutcome::Printed);
    for _ in 0..40 {
        if printer
            .rendered()
            .iter()
            .filter(|r| r.contains("BON CADEAU"))
            .count()
            >= 2
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let slips: Vec<String> = printer
        .rendered()
        .into_iter()
        .filter(|r| r.contains("BON CADEAU"))
        .collect();
    assert_eq!(slips.len(), 2);
    assert!(slips[1].contains("DUPLICATA"));
    let conn = app.state.db.lock().unwrap();
    assert_eq!(store::count_prints(&conn, id, "duplicate").unwrap(), 1);
    assert_eq!(store::count_prints(&conn, id, "original").unwrap(), 1);
}

#[tokio::test]
async fn a_printer_failure_never_undoes_the_sale() {
    let (app, _printer) = app().await;
    app.set_printer(DEAD_PRINTER);
    let order = new_order(&app);
    do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    let r = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(r.success, "{:?}", r.error);
    assert_eq!(vouchers(&app).len(), 1);
    assert_eq!(r.gift_voucher_prints.len(), 1);
    assert_eq!(r.gift_voucher_prints[0].outcome, SlipOutcome::Failed);
    // Nothing logged as printed: the duplicate is how it gets out.
    let id = vouchers(&app)[0].id;
    let conn = app.state.db.lock().unwrap();
    assert_eq!(store::count_prints(&conn, id, "original").unwrap(), 0);
}

#[tokio::test]
async fn the_voucher_tables_are_append_only() {
    let (app, _printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    let conn = app.state.db.lock().unwrap();
    for sql in [
        "UPDATE gift_vouchers SET face_value_cents = 1",
        "DELETE FROM gift_vouchers",
        "UPDATE gift_voucher_movements SET amount_cents = 0",
        "DELETE FROM gift_voucher_movements",
    ] {
        assert!(conn.execute(sql, []).is_err(), "{sql}");
    }
    assert_eq!(store::get(&conn, id).unwrap().unwrap().balance_cents, 5000);
}

#[tokio::test]
async fn the_vat_ticket_names_the_voucher_payment_and_the_layout_is_unchanged() {
    let (app, printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    assert!(
        pay(
            &app,
            dish_order(&app, PRICEY),
            &[PaymentPart::gift_voucher(1850, id)]
        )
        .await
        .success
    );
    for _ in 0..40 {
        if printer
            .rendered()
            .iter()
            .filter(|r| r.contains("INFORMATIONS FDM"))
            .count()
            >= 2
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let tickets: Vec<String> = printer
        .rendered()
        .into_iter()
        .filter(|r| r.contains("INFORMATIONS FDM"))
        .collect();
    assert_eq!(tickets.len(), 2);
    // The sale of the voucher: the X line; the use: the payment row.
    assert!(tickets[0].contains("Bon cadeau"));
    let row = tickets[1]
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .find(|l| l.starts_with("Bon cadeau"))
        .expect("payment row");
    assert_eq!(row, "Bon cadeau 18,50");
}

#[tokio::test]
async fn the_z_report_totals_vat_x_and_the_voucher_payment_and_prints_its_label() {
    let (app, printer) = app().await;
    let (_, id, _) = sell_voucher(&app, 5000).await;
    let sale = pay(
        &app,
        dish_order(&app, PRICEY),
        &[PaymentPart::gift_voucher(1850, id)],
    )
    .await;
    assert!(sale.success, "{:?}", sale.error);
    printer.settle(3).await;
    let z = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);

    // The turnover report is the first R of the Z.
    let report = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.event_label.as_deref() == Some("R"))
        .unwrap();
    let request: Value = serde_json::from_str(report.event_data_json.as_deref().unwrap()).unwrap();
    assert_vat_x_turnover(&request, 50.0);
    let vouchers: Vec<f64> = request["turnover"]["payments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["type"] == "VOUCHER_STORE")
        .flat_map(|p| p["amount"].as_array().unwrap().iter())
        .map(|a| a["totalAmount"].as_f64().unwrap())
        .collect();
    assert_eq!(vouchers, [18.5]);

    for _ in 0..40 {
        if printer
            .rendered()
            .iter()
            .any(|r| r.contains("Bon cadeau") && r.contains("18,50"))
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    let texts = printer.rendered();
    assert!(
        texts
            .iter()
            .any(|t| t.contains("Bon cadeau") && t.contains("18,50")),
        "{texts:#?}"
    );
}

// ── Switching the feature off ──────────────────────────────────────

fn off() -> GiftVoucherSettings {
    GiftVoucherSettings {
        enabled: false,
        ..GiftVoucherSettings::default()
    }
}

#[tokio::test]
async fn switching_off_is_refused_while_a_real_voucher_can_still_be_redeemed() {
    let (app, _printer) = app().await;
    // Nothing sold yet: the switch-off is allowed, then back on.
    assert!(do_set_gift_voucher_settings(&app.state, off()).is_ok());
    do_set_gift_voucher_settings(&app.state, on()).unwrap();

    let (_, id, _) = sell_voucher(&app, 5000).await;
    assert_eq!(
        do_set_gift_voucher_settings(&app.state, off()).unwrap_err(),
        "gift_vouchers_outstanding|count=1"
    );
    assert!(do_get_gift_voucher_settings(&app.state).unwrap().enabled);
    // Changing another setting while staying on is fine.
    assert!(
        do_set_gift_voucher_settings(
            &app.state,
            GiftVoucherSettings {
                validity_months: 24,
                ..on()
            }
        )
        .is_ok()
    );

    // A training voucher does not hold the switch.
    app.state.config.lock().unwrap().training_mode = true;
    sell_voucher(&app, 4000).await;
    app.state.config.lock().unwrap().training_mode = false;
    assert_eq!(
        do_set_gift_voucher_settings(&app.state, off()).unwrap_err(),
        "gift_vouchers_outstanding|count=1"
    );

    // Partly used: still held.
    let r = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, id)],
    )
    .await;
    assert!(r.success, "{:?}", r.error);
    assert_eq!(
        do_set_gift_voucher_settings(&app.state, off()).unwrap_err(),
        "gift_vouchers_outstanding|count=1"
    );
}

#[tokio::test]
async fn a_used_up_or_expired_voucher_does_not_hold_the_switch() {
    let (app, _printer) = app().await;
    let (event, id, order) = sell_voucher(&app, 1000).await;
    assert!(do_set_gift_voucher_settings(&app.state, off()).is_err());
    let r = pay(
        &app,
        dish_order(&app, DISH),
        &[PaymentPart::gift_voucher(1000, id)],
    )
    .await;
    assert!(r.success, "{:?}", r.error);
    assert_eq!(voucher(&app, id).balance_cents, 0);
    // Used up: the switch-off goes through.
    assert!(do_set_gift_voucher_settings(&app.state, off()).is_ok());
    do_set_gift_voucher_settings(&app.state, on()).unwrap();

    // An expired voucher with a balance does not hold it either.
    let yesterday =
        (pos_fiscal::event::now_in_belgium().date_naive() - chrono::Days::new(1)).to_string();
    {
        let conn = app.state.db.lock().unwrap();
        store::insert_voucher(
            &conn,
            &store::NewVoucher {
                code: &pos_engine::commands::gift_vouchers::make_code(3_333_333_333),
                face_value_cents: 3000,
                issued_on: "2025-01-01",
                expires_on: &yesterday,
                training: false,
                issue_event_id: event,
                issue_order_id: order,
                issue_line_id: 9997,
            },
        )
        .unwrap();
    }
    assert!(do_set_gift_voucher_settings(&app.state, off()).is_ok());
}

// ── A table order that sells a voucher in an earlier round ─────────

#[tokio::test]
async fn a_voucher_line_in_an_earlier_round_keeps_the_sale_voucher_free_of_voucher_payments() {
    let (app, _printer) = app().await;
    let (_, paying, _) = sell_voucher(&app, 5000).await;
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(5),
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    add(&app, order, DISH);
    do_add_gift_voucher_line(&app.state, order, 2000).unwrap();
    do_sign_table_order(&app.state, order).await.unwrap();
    add(&app, order, PRICEY);
    do_sign_table_order(&app.state, order).await.unwrap();

    // The order sells a voucher (its line sits in an earlier P): a voucher
    // cannot pay it, and the refused attempt debits nothing.
    let r = pay(&app, order, &[PaymentPart::gift_voucher(4850, paying)]).await;
    assert_eq!(
        r.error.as_deref(),
        Some(errors::GIFT_VOUCHER_PAYMENT_INVALID)
    );
    assert_eq!(voucher(&app, paying).balance_cents, 5000);

    // Paid by cash it issues its voucher once.
    let r = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(r.success, "{:?}", r.error);
    let all = vouchers(&app);
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].face_value_cents, 2000);
}

// ── The system product survives a HubRise catalog sync ─────────────

#[cfg(feature = "hubrise")]
#[tokio::test]
async fn the_voucher_is_still_sold_after_a_catalog_sync() {
    use pos_engine::commands::menu::do_get_menu_items;

    let (app, _printer) = app().await;
    {
        // A mirrored menu has external ids everywhere.
        let mut conn = app.state.db.lock().unwrap();
        conn.execute(
            "UPDATE menu_items SET external_id = 'T' || id WHERE external_id IS NULL",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE categories SET external_id = 'C' || id WHERE external_id IS NULL",
            [],
        )
        .unwrap();
        let outcome =
            pos_catalog::apply(&mut conn, "hubrise", &pos_catalog::testing::catalog()).unwrap();
        assert!(
            matches!(outcome, pos_catalog::ApplyOutcome::Applied { .. }),
            "{outcome:?}"
        );
    }
    let system = |app: &TestApp| -> (String, String, i64) {
        app.state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT external_id, vat_code, active FROM menu_items WHERE id = ?1",
                [VOUCHER_ITEM],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap()
    };
    assert_eq!(system(&app), ("pili:gift-voucher".into(), "X".into(), 0));

    // The sync deactivated every row it does not list, the voucher sells.
    let order = new_order(&app);
    let line = do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    assert_eq!(line.vat_code.as_str(), "X");
    assert_eq!(line.department_id, "GV");
    assert_eq!(line.menu_item_id, VOUCHER_ITEM);
    let r = pay(&app, order, &[PaymentPart::cash()]).await;
    assert!(r.success, "{:?}", r.error);
    let request = request_of(&app, r.fiscal_event_id.unwrap());
    let signed = &request["transaction"]["transactionLines"][0]["mainProduct"];
    assert_eq!(signed["vats"][0]["label"], "X");
    assert_eq!(signed["departmentId"], "GV");
    assert_eq!(vouchers(&app).len(), 1);

    // The grid never shows it.
    let shown = do_get_menu_items(&app.state, None, None).unwrap();
    assert!(shown.iter().all(|m| m.id != VOUCHER_ITEM));

    // Syncing the same catalog again leaves it untouched.
    {
        let mut conn = app.state.db.lock().unwrap();
        pos_catalog::apply(&mut conn, "hubrise", &pos_catalog::testing::catalog()).unwrap();
    }
    assert_eq!(system(&app), ("pili:gift-voucher".into(), "X".into(), 0));
    let order = new_order(&app);
    assert!(do_add_gift_voucher_line(&app.state, order, 1000).is_ok());
}

#[tokio::test]
async fn a_voucher_line_takes_neither_a_course_nor_a_kitchen_note() {
    let (app, _printer) = app().await;
    do_set_course_settings(
        &app.state,
        CourseSettings {
            enabled: true,
            category_courses: Vec::new(),
        },
    )
    .unwrap();
    let order = new_order(&app);
    let line = do_add_gift_voucher_line(&app.state, order, 5000).unwrap();
    assert_eq!(
        do_set_line_course(&app.state, line.id, 2).unwrap_err(),
        errors::GIFT_VOUCHER_LINE_NO_KITCHEN
    );
    for note in [Some("sans sauce".to_string()), None] {
        assert_eq!(
            do_set_line_note(&app.state, line.id, note)
                .await
                .unwrap_err(),
            errors::GIFT_VOUCHER_LINE_NO_KITCHEN
        );
    }
    // Nothing was stored on the line.
    let db = app.state.db.lock().unwrap();
    let note: Option<String> = db
        .query_row(
            "SELECT kitchen_note FROM order_lines WHERE id = ?1",
            [line.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(note, None);
}
