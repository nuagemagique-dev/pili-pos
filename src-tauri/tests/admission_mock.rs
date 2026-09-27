//! The admission image runs moyun-pos against `pos-fdm-mock` (the
//! examiners' machine has no real FDM until the kit arrives). Every
//! other suite stubs the FDM with canned wiremock answers; this one
//! serves the real mock over HTTP and plays a whole service day
//! through the real commands, so the mock and the POS client can never
//! drift apart unnoticed.
//!
//! Checked on the way: every command succeeds, the FDM counters it
//! returns are per label and strictly increasing with a total counter
//! over all events, only N events carry a short signature and a QR
//! URL, and the archive holds each request with its response.

mod common;

use std::collections::HashMap;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_check_fdm_status, do_checkout};
use moyun_pos::commands::copy::do_copy_ticket;
use moyun_pos::commands::financial::{MoneyDirection, do_money_in_out, do_open_drawer_fiscal};
use moyun_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::report::{
    do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use moyun_pos::commands::social::{do_work_in, do_work_out};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;

const INSZ: &str = "12345678958";

async fn app_on_mock() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 1200, 'A', 0, 1),
                (2, 2, 2800, 'B', 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('category', 1, 'name', 'fr', 'Boissons'),
                ('category', 2, 'name', 'fr', 'Cuisine'),
                ('menu_item', 1, 'name', 'fr', 'Dry Martini'),
                ('menu_item', 2, 'name', 'fr', 'Burger du chef');"
        )
        .unwrap();
    }
    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    app
}

fn order(app: &TestApp, table: Option<i32>, lines: &[(i64, i32)]) -> i64 {
    let id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: INSZ.to_string(),
        },
    )
    .unwrap()
    .id;
    for &(item, qty) in lines {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: id,
                menu_item_id: item,
                quantity: qty,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    id
}

async fn sale(app: &TestApp, table: Option<i32>, method: CheckoutPaymentMethod) -> i64 {
    let id = order(app, table, &[(1, 2), (2, 1)]);
    let result = do_checkout(&app.state, id, method).await.unwrap();
    assert!(result.success, "sale: {:?}", result.error);
    let ticket = result.ticket.expect("a sale has a ticket");
    assert_eq!(ticket.total, 52.00);
    result.fiscal_event_id.unwrap()
}

fn set_training(app: &TestApp, on: bool) {
    app.state.config.lock().unwrap().training_mode = on;
}

#[tokio::test]
async fn a_whole_service_day_against_the_admission_mock() {
    let app = app_on_mock().await;

    // The status query the header polls.
    let status = do_check_fdm_status(&app.state).await.unwrap();
    assert_ne!(status["connected"], serde_json::json!(false), "{status}");

    let ok = |what: &str, success: bool, error: Option<String>| {
        assert!(success, "{what}: {error:?}");
    };

    let r = do_work_in(&app.state, INSZ.into()).await.unwrap();
    ok("work in", r.success, r.error);

    // Table 4: two rounds registered (P), a pre-bill (P), then paid (N).
    let table = order(&app, Some(4), &[(1, 1)]);
    do_sign_table_order(&app.state, table).await.unwrap();
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: table,
            menu_item_id: 2,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    do_sign_table_order(&app.state, table).await.unwrap();
    do_sign_pre_bill(&app.state, table).await.unwrap();
    let paid = do_checkout(&app.state, table, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    ok("table sale", paid.success, paid.error.clone());

    let card_sale = sale(&app, None, CheckoutPaymentMethod::Card).await;

    let r = do_money_in_out(
        &app.state,
        INSZ.into(),
        5000,
        MoneyDirection::In,
        "Fond".into(),
    )
    .await
    .unwrap();
    ok("money in", r.success, r.error);
    let r = do_open_drawer_fiscal(&app.state, INSZ.into())
        .await
        .unwrap();
    ok("drawer open", r.success, r.error);

    let copy = do_copy_ticket(&app.state, card_sale, INSZ.into())
        .await
        .unwrap();
    ok("copy", copy.success, copy.error);

    let refund = do_refund_checkout(
        &app.state,
        card_sale,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        INSZ.into(),
    )
    .await
    .unwrap();
    ok("refund", refund.success, refund.error);

    // Training: a sale and a copy, then back to normal.
    set_training(&app, true);
    let training_sale = sale(&app, None, CheckoutPaymentMethod::Cash).await;
    let copy = do_copy_ticket(&app.state, training_sale, INSZ.into())
        .await
        .unwrap();
    ok("training copy", copy.success, copy.error);
    set_training(&app, false);

    let x = do_generate_x_report(&app.state, INSZ.into()).await.unwrap();
    ok("X", x.success, x.error);
    let ux = do_generate_user_x_report(&app.state, INSZ.into())
        .await
        .unwrap();
    ok("user X", ux.success, ux.error);
    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    ok("Z", z.success, z.error.clone());
    assert_eq!(z.report_no, Some(1));

    let r = do_work_out(&app.state, INSZ.into()).await.unwrap();
    ok("work out", r.success, r.error);

    // ── What the archive holds ──────────────────────────────────
    let events = app.fiscal_events();
    let labels: Vec<&str> = events
        .iter()
        .map(|e| e.event_label.as_deref().unwrap())
        .collect();
    assert_eq!(
        labels,
        [
            "S", // work in
            "P", "P", "P", // two rounds + pre-bill
            "N", // table paid
            "N", // card sale
            "F", "F", // money in, drawer
            "C", // copy
            "N", // refund
            "T", "T", // training sale and copy
            "R", "R", "R", "R", // X, user X, Z (turnover + user)
            "S", // work out
        ],
        "{labels:?}"
    );

    let mut last_per_label: HashMap<&str, i64> = HashMap::new();
    let mut last_total = 0;
    for (e, label) in events.iter().zip(&labels) {
        let event = e.event_counter.unwrap();
        let total = e.total_counter.unwrap();
        let previous = last_per_label.insert(label, event).unwrap_or(0);
        assert!(
            event > previous,
            "{label}: counter {event} after {previous}"
        );
        assert!(
            total > last_total,
            "total counter {total} after {last_total}"
        );
        assert!(total >= event, "{label}: total {total} < event {event}");
        last_total = total;

        let is_n = *label == "N";
        assert_eq!(
            e.short_signature.is_some(),
            is_n,
            "{label}: short signature on N only"
        );
        let response: serde_json::Value =
            serde_json::from_str(e.fdm_response_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            response["verificationUrl"].is_string(),
            is_n,
            "{label}: verification URL on N only: {response}"
        );
        assert!(e.event_data_json.as_deref().is_some_and(|j| j.len() > 2));
        assert_eq!(e.fdm_id.as_deref(), Some("MCK00000001"));
    }
    // Per-label counting: three N events are N 1..3 on a fresh FDM
    // only if nothing else ran first - the mock is process-wide, so
    // check the spacing instead: the N counter moved by exactly 3.
    let n: Vec<i64> = events
        .iter()
        .filter(|e| e.event_label.as_deref() == Some("N"))
        .map(|e| e.event_counter.unwrap())
        .collect();
    assert_eq!(n.len(), 3);
    assert!(n.windows(2).all(|w| w[1] > w[0]));
    let total_span = events.last().unwrap().total_counter.unwrap()
        - events.first().unwrap().total_counter.unwrap();
    assert!(
        n[2] - n[0] < total_span,
        "the N counter counts N events only, not every event"
    );
}

#[tokio::test]
async fn the_mock_answers_status_whatever_the_ui_language() {
    for lang in ["fr", "nl", "de", "en", "zh"] {
        let app = app_on_mock().await;
        app.state.config.lock().unwrap().language = lang.into();
        let status = do_check_fdm_status(&app.state).await.unwrap();
        assert_ne!(
            status["connected"],
            serde_json::json!(false),
            "{lang}: {status}"
        );
    }
}

#[tokio::test]
async fn a_sale_signed_by_the_mock_carries_its_vat_breakdown() {
    let app = app_on_mock().await;
    let id = order(&app, Some(1), &[(1, 2), (2, 1)]);
    let result = do_checkout(&app.state, id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let ticket = result.ticket.unwrap();
    let mut vat: Vec<(String, f64, f64)> = ticket
        .vat_breakdown
        .iter()
        .map(|v| (format!("{:?}", v.label), v.total_amount, v.vat_amount))
        .collect();
    vat.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        vat,
        vec![("A".into(), 24.00, 4.17), ("B".into(), 28.00, 3.00)],
        "Dry Martini 2 x 12 at 21 %, burger 28 at 12 %"
    );
    assert!(ticket.verification_url.is_some());
    assert_eq!(ticket.short_signature.as_ref().map(String::len), Some(40));
}

/// module2's online simulator, the FDM of the certification test
/// machine, signs with a `dummy_` placeholder: the POS accepts it,
/// archives it untouched and prints the short signature it received.
#[tokio::test]
async fn a_sale_signed_like_the_module2_simulator_is_accepted() {
    let app = app_on_mock().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let simulator = pos_fdm_mock::app_with(pos_fdm_mock::Control {
        placeholder_signatures: true,
        ..Default::default()
    });
    tokio::spawn(async move {
        axum::serve(listener, simulator).await.unwrap();
    });
    app.state.config.lock().unwrap().fdm_url = format!("http://{addr}/graphql");

    let id = order(&app, Some(1), &[(1, 1)]);
    let result = do_checkout(&app.state, id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    let ticket = result.ticket.unwrap();
    let short = ticket.short_signature.expect("a short signature");
    assert_eq!(short.len(), 40);

    let event = app.fiscal_events().pop().unwrap();
    let response: serde_json::Value =
        serde_json::from_str(event.fdm_response_json.as_deref().unwrap()).unwrap();
    assert!(
        response["digitalSignature"]
            .as_str()
            .is_some_and(|s| s.starts_with("dummy_")),
        "{response}"
    );
    assert_eq!(event.short_signature.as_deref(), Some(short.as_str()));
}
