//! An operator day signed by a live FDM (the module2 cloud simulator).
//!
//! Ignored by default and never run in CI. Run it by hand, one test at
//! a time:
//!
//! ```bash
//! scripts/live_fdm.sh            # loads .env.live, see the script
//! ```
//!
//! | Variable | Default | Purpose |
//! |---|---|---|
//! | `MOYUN_LIVE_FDM_URL`      | -                | FDM GraphQL endpoint (required, else every test returns) |
//! | `MOYUN_LIVE_FDM_TOKEN`    | unset            | bearer token, handed to the app through `fdm_token_file` |
//! | `MOYUN_LIVE_POS_ID`       | `CPOS0031234567` | posId the FDM allows |
//! | `MOYUN_LIVE_SIMULATOR_VERSION` | unset       | `Simulator-Version` header (module2 simulator; unset = latest) |
//! | `MOYUN_LIVE_PAUSE_MS`     | `1000`           | least gap between two calls |
//! | `MOYUN_LIVE_MAX_REQUESTS` | `60`             | calls per run, past it nothing is forwarded |
//! | `UC_EVIDENCE_DIR`         | unset            | where each signed request and response is written |
//!
//! Only what holds on any FDM is asserted: every command succeeds (so
//! each response passed the POS's own response check), counters grow
//! per label and in total, N events alone carry a short signature and a
//! verification URL, and a refund or a copy cites the event it refers
//! to. No counter or amount from the official fixtures.

mod common;

use std::collections::HashMap;

use common::*;
use serde_json::Value;

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

/// The test setup's operator (UC_TestSetup_20240806).
const INSZ: &str = "75061189702";

async fn live_app() -> Option<TestApp> {
    let Some(url) = live_url() else {
        eprintln!("MOYUN_LIVE_FDM_URL not set: skipping (see scripts/live_fdm.sh)");
        return None;
    };
    let app = TestApp::live(url).await;
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
                ('menu_item', 2, 'name', 'fr', 'Burger du chef');",
        )
        .unwrap();
    }
    app.login_as(INSZ, pos_core::user::UserRole::Admin);
    Some(app)
}

/// A step that signed (or failed to sign) an event: keep the ticket
/// numbers at once, so a failure later in the run never reuses them.
fn signed(app: &TestApp, what: &str, success: bool, error: Option<String>) {
    app.save_ticket_numbers();
    assert!(
        success,
        "{what}: {error:?} (after {} live calls)",
        live_requests()
    );
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

async fn status_is_operational(app: &TestApp) {
    let status = do_check_fdm_status(&app.state).await.unwrap();
    assert_ne!(status["connected"], Value::Bool(false), "{status}");
}

/// The request as the POS sent it (the variables' `data`).
fn request(event: &FiscalEventRow) -> Value {
    let raw: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    raw.get("data").cloned().unwrap_or(raw)
}

/// The two requests the gate's first check costs: a status and a sale.
#[tokio::test]
#[ignore]
async fn live_fdm_status_and_one_sale() {
    let Some(app) = live_app().await else { return };
    status_is_operational(&app).await;

    let id = order(&app, None, &[(1, 1)]);
    let sale = do_checkout(&app.state, id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    signed(&app, "sale", sale.success, sale.error.clone());
    let ticket = sale.ticket.expect("a sale has a ticket");
    assert!(ticket.verification_url.is_some());
    assert_eq!(ticket.short_signature.as_ref().map(String::len), Some(40));

    app.dump_evidence("LIVE_sale");
    eprintln!("{} live calls", live_requests());
}

/// One operator day, every event signed by the live FDM.
#[tokio::test]
#[ignore]
async fn live_fdm_operator_day() {
    let Some(app) = live_app().await else { return };
    status_is_operational(&app).await;

    let r = do_work_in(&app.state, INSZ.into()).await.unwrap();
    signed(&app, "work in (M140)", r.success, r.error);

    // Table 4: an order (M121), its pre-bill (M123), then paid (M110).
    let table = order(&app, Some(4), &[(1, 2), (2, 1)]);
    let r = do_sign_table_order(&app.state, table).await;
    signed(&app, "table order (M121)", r.is_ok(), r.err());
    let r = do_sign_pre_bill(&app.state, table).await;
    signed(&app, "pre-bill (M123)", r.is_ok(), r.err());
    let paid = do_checkout(&app.state, table, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    signed(&app, "sale (M110)", paid.success, paid.error.clone());
    let sale = paid.fiscal_event_id.unwrap();

    let refund = do_refund_checkout(
        &app.state,
        sale,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        INSZ.into(),
    )
    .await
    .unwrap();
    signed(&app, "refund (M111)", refund.success, refund.error);

    let copy = do_copy_ticket(&app.state, sale, INSZ.into()).await.unwrap();
    signed(&app, "copy (M160)", copy.success, copy.error);

    let r = do_money_in_out(
        &app.state,
        INSZ.into(),
        5000,
        MoneyDirection::In,
        "Fond".into(),
    )
    .await
    .unwrap();
    signed(&app, "money in (M130)", r.success, r.error);
    let r = do_open_drawer_fiscal(&app.state, INSZ.into())
        .await
        .unwrap();
    signed(&app, "drawer open (M131)", r.success, r.error);
    let r = do_money_in_out(
        &app.state,
        INSZ.into(),
        7400,
        MoneyDirection::Declaration,
        "Comptage".into(),
    )
    .await
    .unwrap();
    signed(&app, "cash count (M133)", r.success, r.error);

    // Training (M171), then back to normal.
    app.state.config.lock().unwrap().training_mode = true;
    let id = order(&app, None, &[(1, 1)]);
    let training = do_checkout(&app.state, id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    signed(
        &app,
        "training sale (M171)",
        training.success,
        training.error,
    );
    app.state.config.lock().unwrap().training_mode = false;

    let x = do_generate_x_report(&app.state, INSZ.into()).await.unwrap();
    signed(&app, "X (M180)", x.success, x.error);
    let ux = do_generate_user_x_report(&app.state, INSZ.into())
        .await
        .unwrap();
    signed(&app, "user X (M182)", ux.success, ux.error);
    let z = do_generate_z_report(&app.state, INSZ.into()).await.unwrap();
    signed(&app, "Z (M184)", z.success, z.error);

    let r = do_work_out(&app.state, INSZ.into()).await.unwrap();
    signed(&app, "work out (M141)", r.success, r.error);

    app.dump_evidence("LIVE_day");

    // ── What the live FDM answered ──────────────────────────────
    let events = app.fiscal_events();
    let labels: Vec<&str> = events
        .iter()
        .map(|e| e.event_label.as_deref().unwrap())
        .collect();
    assert_eq!(
        labels,
        [
            "S", // work in
            "P", "P", // order, pre-bill
            "N", "N", // sale, refund
            "C", // copy
            "F", "F", "F", // money in, drawer, count
            "T", // training sale
            "R", "R", "R", "R", // X, user X, Z (turnover + user)
            "S", // work out
        ],
        "{labels:?}"
    );

    let fdm_id = events[0].fdm_id.clone().expect("fdmId");
    let mut last_per_label: HashMap<&str, i64> = HashMap::new();
    let mut last_total = 0;
    for (e, label) in events.iter().zip(&labels) {
        assert_eq!(e.fdm_id.as_ref(), Some(&fdm_id), "{label}: one FDM");
        let event = e.event_counter.unwrap();
        let total = e.total_counter.unwrap();
        if let Some(previous) = last_per_label.insert(label, event) {
            assert!(
                event > previous,
                "{label}: counter {event} after {previous}"
            );
        }
        assert!(
            total > last_total,
            "total counter {total} after {last_total}"
        );
        last_total = total;

        let is_n = *label == "N";
        assert_eq!(
            e.short_signature.is_some(),
            is_n,
            "{label}: short signature on N only"
        );
        let response: Value =
            serde_json::from_str(e.fdm_response_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            response["verificationUrl"].is_string(),
            is_n,
            "{label}: verification URL on N only: {response}"
        );
    }

    // The refund and the copy cite the sale the FDM signed.
    let sale_event = &events[3];
    for (what, e) in [("refund", &events[4]), ("copy", &events[5])] {
        let cited = &request(e)["fdmRef"];
        assert_eq!(cited["fdmId"], Value::String(fdm_id.clone()), "{what}");
        assert_eq!(
            cited["totalCounter"].as_i64(),
            sale_event.total_counter,
            "{what} cites the sale: {cited}"
        );
    }
    eprintln!("{} live calls", live_requests());
}
