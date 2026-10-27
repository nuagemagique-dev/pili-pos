//! Where lane 4 (owner features: allergens, courses, gift vouchers) meets the till work of
//! the other lanes on android/int (decisions INT-D12 to INT-D21 in docs/android/DECISIONS.md):
//!
//! - the menu editor never shows or writes the POS's own rows (`pili:%`);
//! - a gift voucher line takes no discount, and an order discount leaves it whole;
//! - a sale that sold a gift voucher is never invoiced; a sale paid by a voucher is;
//! - a gift voucher payment line is never corrected (M132), and no line becomes one;
//! - the sales report counts the VAT X voucher line and still equals the Z;
//! - kitchen notes print on course slips; a note on a held line prints no MODIFICATION;
//! - `PRICE_CHANGE` entries are never held and never printed on a course slip;
//! - a table transfer carries no course in its signed request and keeps the held courses.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::auth::do_set_pin;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutResult, PaymentPart, do_checkout_split};
use pos_engine::commands::courses::{
    CategoryCourse, CourseSettings, do_list_held_courses, do_reprint_course_slip,
    do_send_next_course, do_set_course_settings,
};
use pos_engine::commands::discount::{
    DiscountInput, DiscountKind, DiscountReason, DiscountTarget, do_apply_discount,
};
use pos_engine::commands::errors;
use pos_engine::commands::gift_vouchers::{
    GiftVoucherSettings, do_add_gift_voucher_line, do_set_gift_voucher_settings,
};
use pos_engine::commands::invoice::{do_invoice_preview, do_list_invoiceable_sales};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_list_cart_lines,
    do_set_line_note,
};
use pos_engine::commands::payment_correction::{do_correct_payment, do_list_sale_payments};
use pos_engine::commands::report::do_generate_z_report;
use pos_engine::commands::sales_report::do_sales_report;
use pos_engine::commands::table_transfer::do_transfer_table;
use pos_engine::commands::tables::{do_list_tables, do_sign_table_order};
use pos_fiscal::payment::PaymentMethod;

/// A valid INSZ, manager, with a PIN.
const MANAGER: &str = "75061189702";
const PIN: &str = "4321";
const VAT: &str = "BE0308357159";

const SAKE: i64 = 1;
const EDAMAME: i64 = 2;
const MAKI: i64 = 3;
/// A 18,50 EUR dish, VAT B, in the mains.
const PRICEY: i64 = 4;
/// The system product of the gift vouchers (migration 63).
const VOUCHER_ITEM: i64 = 990_001;
const SYSTEM_CATEGORY: i64 = 990_000;

/// Categories 1 drinks (course 0), 2 starters (1), 3 mains (2). Gift vouchers on, a Peppol
/// section for the invoices, a manager with a PIN, a printer.
async fn app() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.enable_peppol_section();
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1), (3, 2, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 2, 450, 'B', 1, 1, 1),
                (3, 3, 1200, 'B', 1, 2, 1),
                (4, 3, 1850, 'B', 1, 3, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Edamame'),
                ('menu_item', 3, 'name', 'fr', 'Maki saumon'),
                ('menu_item', 4, 'name', 'fr', 'Grand plat');",
        )
        .unwrap();
    do_set_gift_voucher_settings(
        &app.state,
        GiftVoucherSettings {
            enabled: true,
            ..GiftVoucherSettings::default()
        },
    )
    .unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

fn courses_on(app: &TestApp) {
    do_set_course_settings(
        &app.state,
        CourseSettings {
            enabled: true,
            category_courses: vec![
                CategoryCourse {
                    category_id: 2,
                    course: 1,
                },
                CategoryCourse {
                    category_id: 3,
                    course: 2,
                },
            ],
        },
    )
    .unwrap();
}

fn order(app: &TestApp, kind: OrderType, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: kind,
            table_number: table,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id
}

fn takeaway(app: &TestApp) -> i64 {
    order(app, OrderType::Takeaway, None)
}

fn table(app: &TestApp, number: i32) -> i64 {
    order(app, OrderType::DineIn, Some(number))
}

fn add(app: &TestApp, order: i64, item: i64, quantity: i32) -> i64 {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: item,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap()
    .id
}

async fn pay(app: &TestApp, order: i64, parts: &[PaymentPart]) -> CheckoutResult {
    let r = do_checkout_split(&app.state, order, parts, None)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r
}

/// Sell a voucher of `cents` for cash: (sale event id, voucher id).
async fn sell_voucher(app: &TestApp, cents: i64) -> (i64, i64) {
    let o = takeaway(app);
    do_add_gift_voucher_line(&app.state, o, cents).unwrap();
    let r = pay(app, o, &[PaymentPart::cash()]).await;
    let id: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT MAX(id) FROM gift_vouchers", [], |r| r.get(0))
        .unwrap();
    (r.fiscal_event_id.unwrap(), id)
}

fn request_of(app: &TestApp, event_id: i64) -> Value {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .unwrap();
    serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap()
}

fn discount(target: DiscountTarget, kind: DiscountKind, value: Option<i64>) -> DiscountInput {
    DiscountInput {
        target,
        kind,
        value,
        reason_code: DiscountReason::CommercialGesture,
        reason_text: None,
    }
}

fn today(app: &TestApp) -> String {
    let hour = app.state.config.lock().unwrap().day_end_hour;
    pos_fiscal::event::now_in_belgium()
        .checked_sub_signed(chrono::Duration::hours(i64::from(hour)))
        .unwrap()
        .date_naive()
        .to_string()
}

// ── Menu editor and allergens: the POS's own rows ─────────────────

#[cfg(feature = "hubrise")]
mod menu_editor {
    use super::*;
    use pos_engine::commands::allergens::{SetAllergensInput, do_set_allergens};
    use pos_engine::commands::menu_admin::{
        SaveCategoryInput, SaveProductInput, do_menu_admin_get, do_menu_save_category,
        do_menu_save_product,
    };
    use std::collections::BTreeMap;

    fn names(fr: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("fr".to_string(), fr.to_string())])
    }

    #[tokio::test]
    async fn the_editor_never_lists_the_gift_voucher_product_or_its_category() {
        let (app, _printer) = app().await;
        let menu = do_menu_admin_get(&app.state).unwrap();
        assert!(menu.categories.iter().all(|c| c.id != SYSTEM_CATEGORY));
        assert!(menu.products.iter().all(|p| p.id != VOUCHER_ITEM));
        // The till's own rows are still there, for the vouchers.
        assert_eq!(menu.products.len(), 4);
        assert_eq!(menu.categories.len(), 3);
    }

    #[tokio::test]
    async fn the_editor_cannot_write_the_system_rows() {
        let (app, _printer) = app().await;
        let err = do_menu_save_category(
            &app.state,
            SaveCategoryInput {
                id: Some(SYSTEM_CATEGORY),
                names: names("Systeme"),
                sort_order: 0,
                color: None,
                active: true,
            },
        )
        .unwrap_err();
        assert!(err.contains("not_found"), "{err}");
        let product = |id: Option<i64>, category_id: i64| SaveProductInput {
            id,
            category_id,
            names: names("Bon"),
            price_cents: 1000,
            vat_code: "B".into(),
            vat_code_takeaway: None,
            kitchen_printer: false,
            code: None,
            open_price: false,
            sort_order: 0,
            active: true,
        };
        let err = do_menu_save_product(&app.state, product(Some(VOUCHER_ITEM), 1)).unwrap_err();
        assert!(err.contains("not_found"), "{err}");
        // Nor a new product in the system category.
        let err = do_menu_save_product(&app.state, product(None, SYSTEM_CATEGORY)).unwrap_err();
        assert!(err.contains("not_found"), "{err}");
        let (active, price): (i64, i64) = app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT active, price_cents FROM menu_items WHERE id = ?1",
                [VOUCHER_ITEM],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((active, price), (0, 0));
    }

    #[tokio::test]
    async fn the_gift_voucher_product_gets_no_allergen_list() {
        let (app, _printer) = app().await;
        let input = |id: i64| SetAllergensInput {
            entity_type: "menu_item".into(),
            entity_id: id,
            declared: true,
            contains: vec!["fish".into()],
            may_contain: vec![],
        };
        let err = do_set_allergens(&app.state, input(VOUCHER_ITEM)).unwrap_err();
        assert!(err.starts_with(errors::ALLERGEN_TARGET_NOT_FOUND), "{err}");
        assert!(do_set_allergens(&app.state, input(MAKI)).unwrap());
    }

    #[tokio::test]
    async fn the_editor_reads_the_declared_lists_of_its_products_hidden_ones_included() {
        let (app, _printer) = app().await;
        assert_eq!(do_menu_admin_get(&app.state).unwrap().allergens.len(), 0);
        // A hidden product keeps its list in the editor.
        app.state
            .db
            .lock()
            .unwrap()
            .execute("UPDATE menu_items SET active = 0 WHERE id = ?1", [EDAMAME])
            .unwrap();
        for (id, contains, traces) in [
            (MAKI, vec!["fish", "soybeans"], vec!["sesame"]),
            (EDAMAME, vec![], vec![]),
        ] {
            do_set_allergens(
                &app.state,
                SetAllergensInput {
                    entity_type: "menu_item".into(),
                    entity_id: id,
                    declared: true,
                    contains: contains.into_iter().map(String::from).collect(),
                    may_contain: traces.into_iter().map(String::from).collect(),
                },
            )
            .unwrap();
        }
        let lists = do_menu_admin_get(&app.state).unwrap().allergens;
        let got: Vec<(String, i64, Vec<String>, Vec<String>)> = lists
            .into_iter()
            .map(|a| (a.entity_type, a.entity_id, a.contains, a.may_contain))
            .collect();
        assert_eq!(
            got,
            vec![
                ("menu_item".into(), EDAMAME, vec![], vec![]),
                (
                    "menu_item".into(),
                    MAKI,
                    vec!["fish".into(), "soybeans".into()],
                    vec!["sesame".into()]
                ),
            ]
        );
        // Back to "no information": the entry leaves the list.
        do_set_allergens(
            &app.state,
            SetAllergensInput {
                entity_type: "menu_item".into(),
                entity_id: EDAMAME,
                declared: false,
                contains: vec![],
                may_contain: vec![],
            },
        )
        .unwrap();
        let lists = do_menu_admin_get(&app.state).unwrap().allergens;
        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].entity_id, MAKI);
    }
}

// ── Discounts ──────────────────────────────────────────────────────

#[tokio::test]
async fn a_voucher_line_cannot_be_offered() {
    let (app, _printer) = app().await;
    let o = takeaway(&app);
    let voucher = do_add_gift_voucher_line(&app.state, o, 2500).unwrap().id;
    let err = do_apply_discount(
        &app.state,
        o,
        discount(
            DiscountTarget::Line { line_id: voucher },
            DiscountKind::Free,
            None,
        ),
        MANAGER.into(),
        PIN.into(),
    )
    .await
    .unwrap_err();
    assert!(err.starts_with(errors::DISCOUNT_INVALID), "{err}");
    let lines = do_list_cart_lines(&app.state, o).unwrap();
    assert_eq!(lines[0].discount_cents, 0);
}

#[tokio::test]
async fn an_order_discount_leaves_the_voucher_line_whole_and_the_voucher_keeps_its_value() {
    let (app, _printer) = app().await;
    let o = takeaway(&app);
    let dish = add(&app, o, PRICEY, 1);
    let voucher = do_add_gift_voucher_line(&app.state, o, 2500).unwrap().id;
    let r = do_apply_discount(
        &app.state,
        o,
        discount(DiscountTarget::Order, DiscountKind::Percent, Some(10)),
        MANAGER.into(),
        PIN.into(),
    )
    .await
    .unwrap();
    // 10 % of the dish only: 1,85 EUR off 18,50 + 25,00.
    assert_eq!(r.order.total_cents, 1850 + 2500 - 185);
    let lines = do_list_cart_lines(&app.state, o).unwrap();
    let of = |id: i64| lines.iter().find(|l| l.line.id == id).unwrap();
    assert_eq!(of(dish).discount_cents, -185);
    assert_eq!(of(voucher).discount_cents, 0);
    assert_eq!(of(voucher).price_changes.len(), 0);

    let sale = pay(&app, o, &[PaymentPart::cash()]).await;
    let balance: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT SUM(amount_cents) FROM gift_voucher_movements",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(balance, 2500, "the voucher is worth its full line");
    let request = request_of(&app, sale.fiscal_event_id.unwrap());
    let x_line = request["transaction"]["transactionLines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["mainProduct"]["departmentId"] == "GV")
        .unwrap();
    let vats = x_line["mainProduct"]["vats"].as_array().unwrap();
    assert_eq!(vats.len(), 1);
    assert_eq!(vats[0]["label"], "X");
    assert!(
        vats[0]["priceChanges"]
            .as_array()
            .is_none_or(std::vec::Vec::is_empty),
        "{x_line}"
    );
}

// ── Invoices ───────────────────────────────────────────────────────

#[tokio::test]
async fn a_sale_that_sold_a_voucher_is_never_invoiced() {
    let (app, _printer) = app().await;
    let customer = app.save_customer(VAT, "Client Test SRL");
    let (voucher_sale, _) = sell_voucher(&app, 2500).await;
    let expected = format!("{}|event={voucher_sale}", errors::INVOICE_GIFT_VOUCHER_SALE);
    let preview = do_invoice_preview(&app.state, &[voucher_sale]).unwrap();
    assert_eq!(preview.problems, std::slice::from_ref(&expected));
    let err = pos_engine::commands::invoice::do_sign_invoice_fiscal(
        &app.state,
        vec![voucher_sale],
        customer,
        MANAGER.into(),
    )
    .await
    .unwrap_err();
    assert_eq!(err, expected);
    let listed = do_list_invoiceable_sales(&app.state, None, None).unwrap();
    assert!(listed.iter().all(|s| s.fiscal_event_id != voucher_sale));
}

#[tokio::test]
async fn a_sale_paid_with_a_voucher_is_invoiced_like_any_sale() {
    let (app, _printer) = app().await;
    let (_, voucher) = sell_voucher(&app, 2500).await;
    let o = takeaway(&app);
    add(&app, o, PRICEY, 1);
    let sale = pay(&app, o, &[PaymentPart::gift_voucher(1850, voucher)])
        .await
        .fiscal_event_id
        .unwrap();
    let preview = do_invoice_preview(&app.state, &[sale]).unwrap();
    assert!(preview.problems.is_empty(), "{:?}", preview.problems);
    assert_eq!(preview.total_cents, 1850);
    assert!(
        preview.vat.iter().all(|v| v.label != "X"),
        "{:?}",
        preview.vat
    );
}

// ── Payment correction (M132) ─────────────────────────────────────

async fn correct(app: &TestApp, event: i64, line: i32, method: PaymentMethod) -> Option<String> {
    let r = do_correct_payment(&app.state, event, line, method, MANAGER.into(), PIN.into())
        .await
        .unwrap();
    if r.success { None } else { r.error }
}

#[tokio::test]
async fn a_gift_voucher_payment_line_is_never_corrected() {
    let (app, _printer) = app().await;
    let (_, voucher) = sell_voucher(&app, 500).await;
    let o = takeaway(&app);
    add(&app, o, PRICEY, 1);
    // 5,00 EUR on the voucher, 13,50 in cash.
    let sale = pay(
        &app,
        o,
        &[PaymentPart::gift_voucher(500, voucher), PaymentPart::cash()],
    )
    .await
    .fiscal_event_id
    .unwrap();
    let payments = do_list_sale_payments(&app.state, sale).unwrap();
    assert!(payments.correctable, "the cash line can be corrected");
    assert_eq!(payments.lines[0].payment_type, "VOUCHER_STORE");
    let events = app.fiscal_events().len();

    assert_eq!(
        correct(&app, sale, 0, PaymentMethod::Card).await.as_deref(),
        Some(errors::PAYMENT_CORRECTION_GIFT_VOUCHER)
    );
    assert_eq!(
        correct(&app, sale, 1, PaymentMethod::GiftVoucher)
            .await
            .as_deref(),
        Some(errors::PAYMENT_CORRECTION_METHOD)
    );
    assert_eq!(app.fiscal_events().len(), events, "nothing signed");
    let balance: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT SUM(amount_cents) FROM gift_voucher_movements WHERE voucher_id = ?1",
            [voucher],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(balance, 0, "the voucher debit stays");
}

#[tokio::test]
async fn a_sale_paid_by_vouchers_alone_has_nothing_to_correct() {
    let (app, _printer) = app().await;
    let (_, voucher) = sell_voucher(&app, 2500).await;
    let o = takeaway(&app);
    add(&app, o, PRICEY, 1);
    let sale = pay(&app, o, &[PaymentPart::gift_voucher(1850, voucher)])
        .await
        .fiscal_event_id
        .unwrap();
    let payments = do_list_sale_payments(&app.state, sale).unwrap();
    assert!(!payments.correctable);
    assert_eq!(
        payments.refusal.as_deref(),
        Some(errors::PAYMENT_CORRECTION_GIFT_VOUCHER)
    );
}

#[tokio::test]
async fn the_cash_of_a_voucher_sale_is_never_corrected_to_meal_vouchers() {
    let (app, _printer) = app().await;
    let (voucher_sale, _) = sell_voucher(&app, 2500).await;
    let payments = do_list_sale_payments(&app.state, voucher_sale).unwrap();
    assert!(payments.correctable);
    assert_eq!(
        correct(&app, voucher_sale, 0, PaymentMethod::MealVoucher)
            .await
            .as_deref(),
        Some(errors::GIFT_VOUCHER_PAYMENT_INVALID)
    );
}

// ── Sales report ───────────────────────────────────────────────────

#[tokio::test]
async fn the_sales_report_counts_the_voucher_line_and_still_equals_the_z() {
    let (app, _printer) = app().await;
    let (_, voucher) = sell_voucher(&app, 2500).await;
    let o = takeaway(&app);
    add(&app, o, PRICEY, 1);
    pay(&app, o, &[PaymentPart::gift_voucher(1850, voucher)]).await;

    let day = today(&app);
    let report = do_sales_report(&app.state, day.clone(), day.clone()).unwrap();
    // Selling the voucher is VAT X turnover; paying with it is a payment, not a sale.
    assert_eq!(report.total_cents, 2500 + 1850);
    assert_eq!(report.ticket_count, 2);
    let gv = report
        .by_product
        .iter()
        .find(|p| p.department == "Bons cadeaux")
        .unwrap_or_else(|| panic!("{:?}", report.by_product));
    assert_eq!((gv.quantity_milli, gv.amount_cents), (1000, 2500));

    let z = do_generate_z_report(&app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);
    let report = do_sales_report(&app.state, day.clone(), day).unwrap();
    assert!(report.days[0].matches_z, "{:?}", report.days);
    assert_eq!(report.days[0].z_total_cents, Some(2500 + 1850));
}

// ── Courses, kitchen notes, price changes, transfers ──────────────

#[tokio::test]
async fn a_kitchen_note_prints_under_its_item_on_the_first_slip_and_at_the_release() {
    let (app, printer) = app().await;
    courses_on(&app);
    let o = table(&app, 3);
    let starter = add(&app, o, EDAMAME, 1);
    let main = add(&app, o, MAKI, 2);
    do_set_line_note(&app.state, starter, Some("sans sel".into()))
        .await
        .unwrap();
    do_set_line_note(&app.state, main, Some("sans wasabi".into()))
        .await
        .unwrap();

    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    assert!(slips[0].contains("1 x Edamame"), "{}", slips[0]);
    assert!(slips[0].contains("> sans sel"), "{}", slips[0]);
    assert!(!slips[0].contains("sans wasabi"), "held: {}", slips[0]);

    let next = do_send_next_course(&app.state, o).await.unwrap();
    assert_eq!((next.course, next.slip), (Some(2), SlipOutcome::Printed));
    let slips = printer.settle_slips(2).await;
    let release = &slips[1];
    assert!(release.contains("2 x Maki saumon"), "{release}");
    assert!(release.contains("> sans wasabi"), "{release}");
    assert!(!release.contains("sans sel"), "{release}");

    // The duplicate carries the note too.
    assert_eq!(
        do_reprint_course_slip(&app.state, o, 2).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(3).await;
    assert!(
        slips[2].contains("DUPLICATA") && slips[2].contains("> sans wasabi"),
        "{}",
        slips[2]
    );
}

#[tokio::test]
async fn a_note_on_a_held_line_prints_no_modification_and_goes_with_the_release() {
    let (app, printer) = app().await;
    courses_on(&app);
    let o = table(&app, 4);
    add(&app, o, SAKE, 1);
    let main = add(&app, o, MAKI, 1);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    // Held, never printed: the note waits for the release.
    assert_eq!(
        do_set_line_note(&app.state, main, Some("bien cuit".into()))
            .await
            .unwrap(),
        SlipOutcome::NothingToPrint
    );
    assert_eq!(printer.settle_slips(1).await.len(), 1, "no MODIFICATION");

    do_send_next_course(&app.state, o).await.unwrap();
    let slips = printer.settle_slips(2).await;
    assert!(
        slips[1].contains("1 x Maki saumon") && slips[1].contains("> bien cuit"),
        "{}",
        slips[1]
    );
    assert!(!slips[1].contains("MODIFICATION"), "{}", slips[1]);
}

#[tokio::test]
async fn price_change_entries_are_never_held_or_printed_on_a_course_slip() {
    let (app, printer) = app().await;
    courses_on(&app);
    let o = table(&app, 5);
    add(&app, o, SAKE, 1);
    add(&app, o, MAKI, 3);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    // The discount writes PRICE_CHANGE correction pairs on registered lines and signs a P.
    let r = do_apply_discount(
        &app.state,
        o,
        discount(DiscountTarget::Order, DiscountKind::Percent, Some(10)),
        MANAGER.into(),
        PIN.into(),
    )
    .await
    .unwrap();
    assert_eq!(r.kitchen_slip, SlipOutcome::NothingToPrint);
    assert_eq!(
        printer.settle_slips(1).await.len(),
        1,
        "no slip for a discount"
    );
    let unprinted: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM order_entries e
             LEFT JOIN kitchen_slip_entries k ON k.entry_id = e.id
             WHERE e.order_id = ?1 AND e.till_op = 'price_change' AND k.entry_id IS NULL",
            [o],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(unprinted, 0, "price change entries are consumed, not held");
    let held = do_list_held_courses(&app.state, o).unwrap();
    assert_eq!(
        held.iter()
            .map(|h| (h.course, h.lines, h.quantity))
            .collect::<Vec<_>>(),
        vec![(2, 1, 3)],
        "the discount neither adds nor removes held quantities"
    );

    do_send_next_course(&app.state, o).await.unwrap();
    let slips = printer.settle_slips(2).await;
    let release = &slips[1];
    assert!(release.contains("3 x Maki saumon"), "{release}");
    assert!(!release.contains("-3 x"), "{release}");
    assert_eq!(release.matches("Maki saumon").count(), 1, "{release}");
}

#[tokio::test]
async fn a_table_transfer_carries_no_course_and_keeps_the_held_courses() {
    let (app, printer) = app().await;
    courses_on(&app);
    let o = table(&app, 6);
    add(&app, o, EDAMAME, 1);
    add(&app, o, MAKI, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    assert_eq!(printer.settle_slips(1).await.len(), 1);
    let held_before = do_list_held_courses(&app.state, o).unwrap();
    assert_eq!(held_before.len(), 1);

    let moved = do_transfer_table(&app.state, o, 9).await.unwrap();
    assert_eq!((moved.from_table, moved.to_table), (6, 9));
    let transfer = app.fiscal_events().pop().unwrap();
    let request = transfer.event_data_json.as_deref().unwrap();
    for absent in [
        "course", "Course", "Plats", "Entrees", "Direct", "SUITE", "held", "kitchen",
    ] {
        assert!(!request.contains(absent), "{absent:?} in {request}");
    }

    assert_eq!(do_list_held_courses(&app.state, o).unwrap(), held_before);
    let tables = do_list_tables(&app.state, 10).unwrap();
    assert_eq!(
        tables[8].held_course,
        Some(2),
        "the tag moves with the order"
    );
    assert_eq!(tables[5].held_course, None);
    // The release still prints, under the new table.
    do_send_next_course(&app.state, o).await.unwrap();
    let slips = printer.settle_slips(3).await;
    let release = slips.iter().find(|s| s.contains("SUITE : Plats")).unwrap();
    assert!(
        release.contains("Table 9") && release.contains("2 x Maki saumon"),
        "{release}"
    );
}
