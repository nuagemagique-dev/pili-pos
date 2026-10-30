//! The sale by weight through the scale simulator: the line comes from the
//! reading, is signed in kilograms, printed with its weight and refunded
//! whole. Events are signed by the admission image's mock FDM.

use pos_engine_testkit as common;

use common::*;

use pos_core::config::{ScaleCheckValue, ScaleConfig};
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::auth::do_set_pin;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::courses::{
    CategoryCourse, CourseSettings, do_send_next_course, do_set_course_settings, do_set_line_course,
};
use pos_engine::commands::discount::{
    DiscountInput, DiscountKind, DiscountReason, DiscountTarget, do_apply_discount,
};
use pos_engine::commands::errors;
use pos_engine::commands::hubrise_push::build_hubrise_order_body;
use pos_engine::commands::invoice::do_invoice_preview;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order, do_remove_order_line,
    do_update_order_line_quantity,
};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_engine::commands::report::{
    do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use pos_engine::commands::sales_report::do_sales_report;
use pos_engine::commands::scale::{
    ScaleWeighInput, do_get_scale_status, do_scale_cancel_weighing, do_scale_start_weighing,
};
use pos_engine::commands::settings::do_set_training_mode;
use pos_engine::commands::table_transfer::do_transfer_table;
use pos_engine::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_scale_mock::MockScale;

const OPERATOR: &str = "12345678958";
const BOWL: i64 = 7;
const SOUP: i64 = 8;
const PER_KILO: i64 = 1990;
const MANAGER_PIN: &str = "4321";

/// An app with a bowl sold by weight, a soup by the piece, and a scale.
async fn app_with_scale() -> (TestApp, MockScale) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(&format!(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer,
                                     sort_order, active, sold_by_weight, scale_tare_grams) VALUES
                ({BOWL}, 1, {PER_KILO}, 'B', 1, 0, 1, 1, 30),
                ({SOUP}, 1, 450, 'B', 1, 1, 1, 0, NULL);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', {BOWL}, 'name', 'fr', 'Bol malatang sur place'),
                ('menu_item', {SOUP}, 'name', 'fr', 'Soupe');"
        ))
        .unwrap();
    let scale = MockScale::new("4711F336");
    let address = scale.spawn().await;
    app.state.config.lock().unwrap().scale = Some(ScaleConfig {
        tcp_address: address.to_string(),
        price_digits: 5,
        check_values: vec![ScaleCheckValue {
            checksum: "4711".into(),
            correction: "F336".into(),
        }],
        weigh_timeout_ms: 5000,
    });
    (app, scale)
}

fn order(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id
}

async fn weigh(app: &TestApp, order_id: i64) -> Result<pos_core::order::OrderLine, String> {
    do_scale_start_weighing(
        &app.state,
        ScaleWeighInput {
            order_id,
            menu_item_id: BOWL,
        },
    )
    .await
}

fn weighings(app: &TestApp) -> Vec<(i64, i64, i64, Option<i64>)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare(
            "SELECT weight_grams, unit_price_cents, amount_cents, tare_grams
             FROM scale_weighings ORDER BY id",
        )
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn order_total(app: &TestApp, order_id: i64) -> i64 {
    let db = app.state.db.lock().unwrap();
    pos_db::queries::orders::get_order(&db, order_id)
        .unwrap()
        .total_cents
}

fn sale_event(app: &TestApp) -> i64 {
    app.fiscal_events().pop().expect("an event").id
}

fn last_sale_line(app: &TestApp) -> serde_json::Value {
    let event = app.fiscal_events().pop().expect("an event");
    let data: serde_json::Value = serde_json::from_str(&event.event_data_json.unwrap()).unwrap();
    data["transaction"]["transactionLines"][0].clone()
}

#[tokio::test]
async fn a_weighing_adds_the_line_the_scale_returned() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    let line = weigh(&app, order_id).await.unwrap();

    assert_eq!(line.quantity, 1);
    assert_eq!(line.weight_grams, Some(612));
    assert_eq!(line.unit_price_cents, PER_KILO);
    assert_eq!(line.weighed_amount_cents, Some(1218));
    assert_eq!(line.line_total(), 1218);
    assert_eq!(order_total(&app, order_id), 1218);

    // The audit row, and the tare and text the scale got.
    assert_eq!(weighings(&app), vec![(612, PER_KILO, 1218, Some(30))]);
    assert_eq!(scale.state().tare_grams, Some(30));
    assert_eq!(scale.state().text.as_deref(), Some("Bol malatang "));

    // The cart shows it.
    let cart = pos_engine::commands::order::do_list_cart_lines(&app.state, order_id).unwrap();
    assert_eq!(cart[0].line.weight_grams, Some(612));
}

#[tokio::test]
async fn a_product_sold_by_weight_never_comes_from_the_screen() {
    let (app, _scale) = app_with_scale().await;
    let order_id = order(&app);
    let add = |menu_item_id, unit_price_cents| {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id,
                menu_item_id,
                quantity: 1,
                modifiers_json: None,
                unit_price_cents,
            },
        )
    };
    assert_eq!(add(BOWL, None).unwrap_err(), errors::LINE_NEEDS_SCALE);
    assert_eq!(add(BOWL, Some(1)).unwrap_err(), errors::LINE_NEEDS_SCALE);
    // And the reverse: a piece has nothing to weigh.
    let refused = do_scale_start_weighing(
        &app.state,
        ScaleWeighInput {
            order_id,
            menu_item_id: SOUP,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(refused, errors::SCALE_NOT_SOLD_BY_WEIGHT);
}

#[tokio::test]
async fn without_a_scale_nothing_can_be_weighed() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    app.state.config.lock().unwrap().scale = None;
    scale.load(612);
    assert_eq!(
        weigh(&app, order_id).await.unwrap_err(),
        errors::SCALE_NOT_CONFIGURED
    );
    assert_eq!(scale.state().weighings, 0);
    assert!(!do_get_scale_status(&app.state).await.unwrap().configured);
}

#[tokio::test]
async fn a_refused_reading_adds_no_line() {
    for (grams, code) in [
        (10, errors::SCALE_BELOW_MIN),
        (-1, errors::SCALE_UNDERLOAD),
        (20_000, errors::SCALE_OVERLOAD),
    ] {
        let (app, scale) = app_with_scale().await;
        let order_id = order(&app);
        scale.load(grams);
        assert_eq!(weigh(&app, order_id).await.unwrap_err(), code);
        assert_eq!(order_total(&app, order_id), 0);
        assert!(weighings(&app).is_empty());
    }
}

#[tokio::test]
async fn the_same_bowl_is_not_counted_twice() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    assert_eq!(
        weigh(&app, order_id).await.unwrap_err(),
        errors::SCALE_NO_MOTION
    );
    assert_eq!(order_total(&app, order_id), 1218);
    scale.load(500);
    weigh(&app, order_id).await.unwrap();
    assert_eq!(order_total(&app, order_id), 1218 + 995);
}

#[tokio::test]
async fn a_scale_price_that_disagrees_is_refused() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    scale.with(|s| s.price_error_cents = 7);
    assert_eq!(
        weigh(&app, order_id).await.unwrap_err(),
        errors::SCALE_PRICE_MISMATCH
    );
    assert!(weighings(&app).is_empty());
}

#[tokio::test]
async fn cancelling_adds_nothing() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    scale.with(|s| s.motion = 1_000_000);
    let weighing = weigh(&app, order_id);
    let cancel = async {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        do_scale_cancel_weighing(&app.state).unwrap();
    };
    let (result, ()) = tokio::join!(weighing, cancel);
    assert_eq!(result.unwrap_err(), errors::SCALE_CANCELLED);
    assert_eq!(order_total(&app, order_id), 0);
}

#[tokio::test]
async fn the_software_check_is_logged_and_a_refusal_blocks_the_scale() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    scale.require_check();
    let line = weigh(&app, order_id).await.unwrap();
    assert_eq!(line.weight_grams, Some(612));
    let status = do_get_scale_status(&app.state).await.unwrap();
    assert!(status.configured && status.connected && !status.busy);
    assert_eq!(status.check, "ok");

    // No value that passes: nothing is weighed, and nothing gets around it.
    {
        let mut config = app.state.config.lock().unwrap();
        config.scale.as_mut().unwrap().check_values[0].checksum = "0000".into();
    }
    scale.load(300);
    scale.require_check();
    assert_eq!(
        weigh(&app, order_id).await.unwrap_err(),
        errors::SCALE_CHECK_FAILED
    );
    assert_eq!(
        do_get_scale_status(&app.state).await.unwrap().check,
        "refused"
    );
    let log: Vec<String> = {
        let db = app.state.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT event FROM scale_check_log ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert!(log.contains(&"requested".to_string()));
    assert!(log.contains(&"passed".to_string()));
    assert!(log.contains(&"refused".to_string()));
    let audit_writes = app
        .state
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM scale_check_log", []);
    assert!(audit_writes.is_err(), "the check log is append-only");
}

#[tokio::test]
async fn a_weighed_line_has_no_plus_or_minus_and_is_corrected_whole() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    let line = weigh(&app, order_id).await.unwrap();
    assert_eq!(
        do_update_order_line_quantity(&app.state, line.id, 2).unwrap_err(),
        errors::LINE_WEIGHED_FIXED
    );
    do_remove_order_line(&app.state, line.id).unwrap();
    assert_eq!(order_total(&app, order_id), 0);
    // The journal keeps the weighing and its correction, both with their weight.
    let db = app.state.db.lock().unwrap();
    let entries = pos_db::queries::orders::get_order_entries(&db, order_id, false).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].quantity, -1);
    assert_eq!(entries[1].weight_grams, Some(612));
    assert_eq!(entries[1].line_total(), -1218);
    assert_eq!(entries[1].fiscal_quantity(), -0.612);
}

#[tokio::test]
async fn the_sale_signs_kilograms_and_the_ticket_prints_the_weight() {
    let (app, scale) = app_with_scale().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let line = last_sale_line(&app);
    let product = &line["mainProduct"];
    assert_eq!(product["quantity"], serde_json::json!(0.612));
    assert_eq!(product["quantityType"], "KILOGRAM");
    assert_eq!(product["unitPrice"], serde_json::json!(19.9));
    assert_eq!(product["vats"][0]["price"], serde_json::json!(12.18));
    assert_eq!(line["lineTotal"], serde_json::json!(12.18));
    assert_eq!(printer.settle(1).await.len(), 1);
    let printed = printer.rendered().join("\n");
    assert!(printed.contains("0,612 kg"), "{printed}");
    assert!(printed.contains("19,90 €/kg"), "{printed}");
    let narrow = printer
        .rendered_for(pos_core::config::PaperWidth::Mm58)
        .join("\n");
    assert!(narrow.contains("0,612 kg"), "{narrow}");
}

#[tokio::test]
async fn a_weighing_is_refunded_whole_and_once() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    scale.load(500);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let lines = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&db, order_id).unwrap()
    };
    let item = |line_id, quantity| RefundItem {
        menu_item_id: BOWL,
        line_id: Some(line_id),
        quantity,
    };
    let sale = sale_event(&app);
    let refund =
        |items: Vec<RefundItem>| do_refund_checkout(&app.state, sale, items, OPERATOR.to_string());

    // A part of a weighing cannot go back.
    let refused = refund(vec![item(lines[0].id, 2)]).await.unwrap_err();
    assert!(
        refused.starts_with(errors::REFUND_WEIGHED_LINE_WHOLE),
        "{refused}"
    );

    // The first one goes back with the weight and price signed.
    let done = refund(vec![item(lines[0].id, 1)]).await.unwrap();
    assert!(done.success, "{:?}", done.error);
    let product = last_sale_line(&app)["mainProduct"].clone();
    assert_eq!(product["quantity"], serde_json::json!(-0.612));
    assert_eq!(product["quantityType"], "KILOGRAM");
    assert_eq!(product["negQuantityReason"], "REFUND");
    assert_eq!(product["vats"][0]["price"], serde_json::json!(-12.18));

    // Not twice.
    let again = refund(vec![item(lines[0].id, 1)]).await.unwrap_err();
    assert!(again.starts_with(errors::REFUND_EXCEEDS_SOLD), "{again}");

    // The other weighing still can, once.
    let other = refund(vec![item(lines[1].id, 1)]).await.unwrap();
    assert!(other.success, "{:?}", other.error);
    let again = refund(vec![item(lines[1].id, 1)]).await.unwrap_err();
    assert!(again.starts_with(errors::REFUND_EXCEEDS_SOLD), "{again}");
}

#[tokio::test]
async fn a_weighing_under_half_a_kilo_counts_as_refunded_too() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(278);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let line_id = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&db, order_id).unwrap()[0].id
    };
    let sale = sale_event(&app);
    let refund = || {
        do_refund_checkout(
            &app.state,
            sale,
            vec![RefundItem {
                menu_item_id: BOWL,
                line_id: Some(line_id),
                quantity: 1,
            }],
            OPERATOR.to_string(),
        )
    };
    let first = refund().await.unwrap();
    assert!(first.success, "{:?}", first.error);
    let second = refund().await.unwrap_err();
    assert!(
        second.starts_with(errors::REFUND_EXCEEDS_SOLD),
        "the double refund guard saw -0.278 as 0: {second}"
    );
}

#[tokio::test]
async fn a_till_discount_comes_off_the_amount_of_the_scale() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    assert_eq!(order_total(&app, order_id), 1218);

    do_set_pin(&app.state, OPERATOR.into(), MANAGER_PIN.into(), None).unwrap();
    let discounted = do_apply_discount(
        &app.state,
        order_id,
        DiscountInput {
            target: DiscountTarget::Order,
            kind: DiscountKind::Amount,
            value: Some(218),
            reason_code: DiscountReason::CommercialGesture,
            reason_text: None,
        },
        OPERATOR.into(),
        MANAGER_PIN.into(),
    )
    .await
    .unwrap();
    assert_eq!(discounted.order.total_cents, 1000);

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let line = last_sale_line(&app);
    assert_eq!(line["mainProduct"]["quantity"], serde_json::json!(0.612));
    assert_eq!(line["lineTotal"], serde_json::json!(10.0));
}

#[tokio::test]
async fn the_z_counts_a_sale_its_correction_and_its_refund_in_money() {
    let (app, scale) = app_with_scale().await;

    // A weighing removed before the sale nets to nothing.
    let order_id = order(&app);
    scale.load(612);
    let removed = weigh(&app, order_id).await.unwrap();
    do_remove_order_line(&app.state, removed.id).unwrap();
    scale.load(278);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let sale = sale_event(&app);
    assert_eq!(order_total(&app, order_id), 553);

    let lines = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&db, order_id).unwrap()
    };
    let kept = lines.iter().find(|l| l.quantity == 1).unwrap().id;
    let refund = do_refund_checkout(
        &app.state,
        sale,
        vec![RefundItem {
            menu_item_id: BOWL,
            line_id: Some(kept),
            quantity: 1,
        }],
        OPERATOR.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);

    let z = do_generate_z_report(&app.state, OPERATOR.into())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);
    // The Z signs the turnover report, then the user report.
    let events = app.fiscal_events();
    let turnover: serde_json::Value =
        serde_json::from_str(events[events.len() - 2].event_data_json.as_deref().unwrap()).unwrap();
    let net: f64 = turnover["turnover"]["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["eventLabel"] == "N")
        .map(|t| t["amount"].as_f64().unwrap())
        .sum();
    assert_eq!(net, 0.0, "{turnover}");
}

#[tokio::test]
async fn the_sales_report_shows_the_kilos_of_a_weighed_product() {
    let (app, scale) = app_with_scale().await;
    for grams in [612, 622] {
        let order_id = order(&app);
        scale.load(grams);
        weigh(&app, order_id).await.unwrap();
        let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
            .await
            .unwrap();
        assert!(result.success, "{:?}", result.error);
    }
    let today = chrono::Utc::now()
        .with_timezone(&chrono_tz::Europe::Brussels)
        .date_naive()
        .to_string();
    let report = do_sales_report(&app.state, today.clone(), today).unwrap();
    let bowl = &report.by_product[0];
    assert!(bowl.weighed);
    assert_eq!(bowl.quantity_milli, 1234);
    assert_eq!(bowl.amount_cents, 1218 + 1238);
}

#[tokio::test]
async fn an_invoice_preview_keeps_the_kilos() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let preview = do_invoice_preview(&app.state, &[sale_event(&app)]).unwrap();
    assert_eq!(preview.lines[0].quantity_milli, 612);
    assert_eq!(preview.lines[0].total_cents, 1218);
}

#[tokio::test]
async fn a_pre_bill_then_the_sale_sign_the_same_kilos_on_a_table() {
    let (app, scale) = app_with_scale().await;
    let order_id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(1),
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id;
    scale.load(278);
    weigh(&app, order_id).await.unwrap();

    do_sign_pre_bill(&app.state, order_id).await.unwrap();
    let pre_bill = app.fiscal_events().pop().unwrap();
    assert_eq!(pre_bill.event_type, "P");
    let data: serde_json::Value = serde_json::from_str(&pre_bill.event_data_json.unwrap()).unwrap();
    let product = &data["transaction"]["transactionLines"][0]["mainProduct"];
    assert_eq!(product["quantityType"], "KILOGRAM");
    assert_eq!(product["quantity"], serde_json::json!(0.278));

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let product = &last_sale_line(&app)["mainProduct"];
    assert_eq!(product["quantityType"], "KILOGRAM");
    assert_eq!(product["quantity"], serde_json::json!(0.278));
}

#[tokio::test]
async fn a_training_sale_signs_the_kilos_too() {
    let (app, scale) = app_with_scale().await;
    do_set_training_mode(&app.state, true).unwrap();
    let order_id = order(&app);
    scale.load(278);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let product = &last_sale_line(&app)["mainProduct"];
    assert_eq!(product["quantityType"], "KILOGRAM");
    assert_eq!(product["quantity"], serde_json::json!(0.278));
}

#[tokio::test]
async fn the_hubrise_echo_sends_the_kilos_and_the_price_per_kilo() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(278);
    let line = weigh(&app, order_id).await.unwrap();
    let order = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order(&db, order_id).unwrap()
    };
    let body = build_hubrise_order_body(&order, &[line], &[("card", 553)], &[]);
    assert_eq!(body["items"][0]["quantity"], "0.278");
    assert_eq!(body["items"][0]["price"], "19.90 EUR");
}

// ── The rest of the order flow ─────────────────────────────────────

fn table_order(app: &TestApp, table: i32) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(table),
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id
}

fn add_soups(app: &TestApp, order_id: i64, quantity: i32) -> i64 {
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: SOUP,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap()
    .id
}

/// The product lines of every signed event that has any.
fn signed_products(app: &TestApp) -> Vec<(String, serde_json::Value)> {
    app.fiscal_events()
        .into_iter()
        .flat_map(|event| {
            let data: serde_json::Value =
                serde_json::from_str(event.event_data_json.as_deref().unwrap_or("{}")).unwrap();
            let lines = data["transaction"]["transactionLines"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            lines
                .into_iter()
                .map(move |l| (event.event_type.clone(), l["mainProduct"].clone()))
        })
        .collect()
}

#[tokio::test]
async fn a_weighing_goes_to_the_kitchen_as_one_bowl() {
    let (app, scale) = app_with_scale().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.kitchen_slip, Some(SlipOutcome::Printed));
    let slips = printer.settle_slips(1).await;
    assert!(slips[0].contains("1 x Bol malatang"), "{}", slips[0]);
}

#[tokio::test]
async fn a_weighing_waits_for_its_course_and_keeps_its_kilos() {
    let (app, scale) = app_with_scale().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    do_set_course_settings(
        &app.state,
        CourseSettings {
            enabled: true,
            category_courses: vec![CategoryCourse {
                category_id: 1,
                course: 1,
            }],
        },
    )
    .unwrap();
    let order_id = table_order(&app, 1);
    add_soups(&app, order_id, 1);
    scale.load(278);
    let bowl = weigh(&app, order_id).await.unwrap();
    do_set_line_course(&app.state, bowl.id, 2).unwrap();

    assert_eq!(
        do_sign_table_order(&app.state, order_id).await.unwrap(),
        SlipOutcome::Printed
    );
    let slips = printer.settle_slips(1).await;
    assert!(slips[0].contains("1 x Soupe"), "{}", slips[0]);
    assert!(!slips[0].contains("1 x Bol"), "{}", slips[0]);

    do_send_next_course(&app.state, order_id).await.unwrap();
    let slips = printer.settle_slips(2).await;
    assert!(slips[1].contains("1 x Bol malatang"), "{}", slips[1]);

    let bowl_lines: Vec<_> = signed_products(&app)
        .into_iter()
        .filter(|(_, p)| p["quantityType"] == "KILOGRAM")
        .collect();
    assert_eq!(bowl_lines.len(), 1);
    assert_eq!(bowl_lines[0].0, "P");
    assert_eq!(bowl_lines[0].1["quantity"], serde_json::json!(0.278));
}

#[tokio::test]
async fn a_table_transfer_keeps_the_weighing() {
    let (app, scale) = app_with_scale().await;
    let order_id = table_order(&app, 1);
    scale.load(278);
    let before = weigh(&app, order_id).await.unwrap();

    let moved = do_transfer_table(&app.state, order_id, 2).await.unwrap();
    assert_eq!((moved.from_table, moved.to_table), (1, 2));
    let after = {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::orders::get_order_lines(&db, order_id).unwrap()
    };
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].id, before.id);
    assert_eq!(after[0].weight_grams, Some(278));
    assert_eq!(weighings(&app).len(), 1);

    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let products = signed_products(&app);
    assert!(!products.is_empty());
    for (event, product) in &products {
        assert_eq!(product["quantityType"], "KILOGRAM", "{event}: {product}");
        assert_eq!(
            product["quantity"].as_f64().unwrap().abs(),
            0.278,
            "{event}: {product}"
        );
    }
}

#[tokio::test]
async fn a_weighing_and_soups_on_one_ticket_go_back_each_by_its_own_rule() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    let bowl = weigh(&app, order_id).await.unwrap();
    let soups = add_soups(&app, order_id, 3);
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let sale = sale_event(&app);
    let sold = signed_products(&app);
    assert_eq!(sold[0].1["quantityType"], "KILOGRAM");
    assert_eq!(sold[1].1["quantityType"], "PIECE");
    assert_eq!(sold[1].1["quantity"], serde_json::json!(3.0));

    // A part of the soups may go back, with the whole weighing.
    let done = do_refund_checkout(
        &app.state,
        sale,
        vec![
            RefundItem {
                menu_item_id: BOWL,
                line_id: Some(bowl.id),
                quantity: 1,
            },
            RefundItem {
                menu_item_id: SOUP,
                line_id: Some(soups),
                quantity: 2,
            },
        ],
        OPERATOR.into(),
    )
    .await
    .unwrap();
    assert!(done.success, "{:?}", done.error);
    let products = signed_products(&app);
    let refund = &products[products.len() - 2..];
    assert_eq!(refund[0].1["quantity"], serde_json::json!(-0.612));
    assert_eq!(refund[0].1["quantityType"], "KILOGRAM");
    assert_eq!(refund[1].1["quantity"], serde_json::json!(-2.0));
    assert_eq!(refund[1].1["quantityType"], "PIECE");

    // The third soup still can; the weighing no more.
    let again = do_refund_checkout(
        &app.state,
        sale,
        vec![RefundItem {
            menu_item_id: SOUP,
            line_id: Some(soups),
            quantity: 1,
        }],
        OPERATOR.into(),
    )
    .await
    .unwrap();
    assert!(again.success, "{:?}", again.error);
}

#[tokio::test]
async fn the_x_and_user_reports_count_a_weighing_in_money() {
    let (app, scale) = app_with_scale().await;
    let order_id = order(&app);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);

    let x = do_generate_x_report(&app.state, OPERATOR.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    let data: serde_json::Value = serde_json::from_str(
        app.fiscal_events()
            .pop()
            .unwrap()
            .event_data_json
            .as_deref()
            .unwrap(),
    )
    .unwrap();
    let net: f64 = data["turnover"]["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["eventLabel"] == "N")
        .map(|t| t["amount"].as_f64().unwrap())
        .sum();
    assert_eq!(net, 12.18, "{data}");

    let user = do_generate_user_x_report(&app.state, OPERATOR.into())
        .await
        .unwrap();
    assert!(user.success, "{:?}", user.error);
}

#[tokio::test]
async fn a_copy_and_a_pre_bill_print_the_weight_on_both_papers() {
    let (app, scale) = app_with_scale().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let order_id = table_order(&app, 1);
    scale.load(612);
    weigh(&app, order_id).await.unwrap();

    do_sign_pre_bill(&app.state, order_id).await.unwrap();
    let pre_bill = printer.settle(1).await.len();
    assert!(pre_bill >= 1);
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let sale = sale_event(&app);
    let copy = do_copy_ticket(&app.state, sale, OPERATOR.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    printer.settle(pre_bill + 2).await;

    for paper in [
        pos_core::config::PaperWidth::Mm80,
        pos_core::config::PaperWidth::Mm58,
    ] {
        let tickets: Vec<_> = printer
            .rendered_for(paper)
            .into_iter()
            .filter(|t| t.contains("Bol malatang") && !t.contains("1 x Bol"))
            .collect();
        assert!(tickets.len() >= 3, "{paper:?}: {tickets:#?}");
        for ticket in &tickets {
            assert!(ticket.contains("0,612 kg"), "{paper:?}: {ticket}");
            assert!(ticket.contains("19,90 €/kg"), "{paper:?}: {ticket}");
        }
    }
}

#[tokio::test]
async fn an_installation_without_a_scale_signs_pieces_only() {
    let (app, _scale) = app_with_scale().await;
    app.state.config.lock().unwrap().scale = None;
    let order_id = order(&app);
    add_soups(&app, order_id, 2);
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let product = &last_sale_line(&app)["mainProduct"];
    assert_eq!(product["quantityType"], "PIECE");
    assert_eq!(product["quantity"], serde_json::json!(2.0));
    assert!(weighings(&app).is_empty());
}
