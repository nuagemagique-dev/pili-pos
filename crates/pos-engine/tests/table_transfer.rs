//! Moving an open order to another table (event P, signCostCenterChange,
//! Basic Set M122): the refusals, the order of the events, the table that
//! changes only when the move is signed (also when its answer comes
//! late), the ghost order voided on the target, the price changes that
//! travel with their line, the notice for the kitchen, training and
//! access.

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::report::close_overdue_day;
use pos_engine::commands::table_transfer::do_transfer_table;
use pos_engine::commands::tables::{do_list_configured_tables, do_sign_table_order};
use pos_fiscal::types::EventLabel;

const OPERATOR: &str = "12345678958";

/// Catalogue: 1 = Sake (bar), 2 = Maki (kitchen). 25 tables.
async fn app_with(fdm: &LossyFdm) -> TestApp {
    let app = TestApp::new().await;
    app.use_lossy_fdm(fdm);
    seed(&app);
    app
}

fn seed(app: &TestApp) {
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 1, 1200, 'B', 1, 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Maki saumon');",
        )
        .unwrap();
}

fn order(app: &TestApp, order_type: OrderType, table: Option<i32>) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type,
            table_number: table,
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id
}

fn table_order(app: &TestApp, table: i32) -> i64 {
    order(app, OrderType::DineIn, Some(table))
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

fn table_of(app: &TestApp, order: i64) -> Option<i32> {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT table_number FROM orders WHERE id = ?1",
            [order],
            |r| r.get(0),
        )
        .unwrap()
}

fn requests(app: &TestApp) -> Vec<Value> {
    app.fiscal_events()
        .iter()
        .filter_map(|e| e.event_data_json.as_deref())
        .map(|j| serde_json::from_str(j).unwrap())
        .collect()
}

fn operations(fdm: &LossyFdm) -> Vec<String> {
    fdm.sent().into_iter().map(|s| s.op).collect()
}

async fn refused(app: &TestApp, order: i64, to: i32) -> String {
    do_transfer_table(&app.state, order, to)
        .await
        .expect_err("the transfer is refused")
}

// ── Refusals ───────────────────────────────────────────────────────

#[tokio::test]
async fn every_refusal_signs_nothing_and_moves_nothing() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);

    // The same table.
    assert_eq!(refused(&app, o, 4).await, errors::TABLE_TRANSFER_SAME_TABLE);
    // Outside 1 to 999, and outside the plan (25 tables).
    for to in [0, -3, 26, 1000] {
        assert_eq!(
            refused(&app, o, to).await,
            errors::TABLE_TRANSFER_TARGET_INVALID,
            "table {to}"
        );
    }
    // A target with an open order that has lines.
    let busy = table_order(&app, 7);
    add(&app, busy, 2, 1);
    assert_eq!(
        refused(&app, o, 7).await,
        errors::TABLE_TRANSFER_TARGET_BUSY
    );
    // An order without a line.
    let empty = table_order(&app, 9);
    assert_eq!(refused(&app, empty, 10).await, errors::ORDER_NO_LINES);
    // A takeaway is not on a table.
    let takeaway = order(&app, OrderType::Takeaway, None);
    add(&app, takeaway, 1, 1);
    assert_eq!(
        refused(&app, takeaway, 3).await,
        errors::TABLE_TRANSFER_NOT_A_TABLE
    );
    // An online order is not on a table of the room.
    let online = table_order(&app, 12);
    add(&app, online, 1, 1);
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE orders SET hubrise_order_id = 'ONLINE-1' WHERE id = ?1",
            [online],
        )
        .unwrap();
    assert_eq!(
        refused(&app, online, 13).await,
        errors::TABLE_TRANSFER_NOT_A_TABLE
    );
    // An order that is not open.
    let closed = table_order(&app, 14);
    add(&app, closed, 1, 1);
    assert!(
        do_checkout(&app.state, closed, CheckoutPaymentMethod::Cash)
            .await
            .unwrap()
            .success
    );
    let events = app.fiscal_events().len();
    assert_eq!(refused(&app, closed, 15).await, errors::ORDER_NOT_OPEN);
    assert_eq!(refused(&app, 9999, 15).await, errors::ORDER_NOT_OPEN);

    assert_eq!(app.fiscal_events().len(), events, "no event for a refusal");
    assert_eq!(table_of(&app, o), Some(4));
    assert!(
        fdm.sent_op("signCostCenterChange").is_empty(),
        "{:?}",
        operations(&fdm)
    );
}

#[tokio::test]
async fn a_registration_waiting_for_the_fdm_is_resolved_before_the_move() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signOrder");
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    // The registration of the order has no answer yet.
    assert!(do_sign_table_order(&app.state, o).await.is_err());

    // The next command resolves the open request first, so the order is
    // no longer waiting: the move goes through and carries it.
    do_transfer_table(&app.state, o, 8).await.unwrap();
    assert_eq!(table_of(&app, o), Some(8));
}

#[tokio::test]
async fn a_target_with_only_a_ghost_order_is_free_and_the_ghost_is_voided() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    let ghost = table_order(&app, 6);

    let moved = do_transfer_table(&app.state, o, 6).await.unwrap();
    assert_eq!((moved.from_table, moved.to_table), (4, 6));

    assert_eq!(table_of(&app, o), Some(6));
    assert_eq!(app.order_status(ghost).as_deref(), Some("voided"));
    assert_eq!(app.order_status(o).as_deref(), Some("open"));
}

// ── Signing ────────────────────────────────────────────────────────

#[tokio::test]
async fn unsent_entries_are_registered_first_then_the_table_moves() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 4);
    add(&app, o, 1, 2);
    do_sign_table_order(&app.state, o).await.unwrap();
    // One more line after the first registration.
    add(&app, o, 2, 1);

    let moved = do_transfer_table(&app.state, o, 11).await.unwrap();
    assert_eq!(moved.kitchen_slip, SlipOutcome::Printed);
    assert_eq!(moved.transfer_slip, SlipOutcome::Printed);

    assert_eq!(
        operations(&fdm),
        ["signOrder", "signOrder", "signCostCenterChange"]
    );
    let events = app.fiscal_events();
    assert!(events.iter().all(|e| e.event_type == "P"));
    let requests = requests(&app);
    // The second order carries only what was not registered yet.
    let lines = requests[1]["transaction"]["transactionLines"]
        .as_array()
        .unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["mainProduct"]["productName"], "Maki saumon");
    // The move carries every line, on the old and the new table.
    let transfer = &requests[2]["transfer"];
    let ids = |side: &str| -> Vec<String> {
        transfer[side][0]["transaction"]["transactionLines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                l["mainProduct"]["productName"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect()
    };
    assert_eq!(ids("from"), ["Sake", "Maki saumon"]);
    assert_eq!(ids("to"), ["Sake", "Maki saumon"]);
    assert_eq!(transfer["from"][0]["costCenter"]["id"], "T04");
    assert_eq!(transfer["to"][0]["costCenter"]["id"], "T11");
    assert_eq!(requests[2]["ticketMedium"], "NONE");
    assert_eq!(table_of(&app, o), Some(11));
    // The move adds no journal entry: nothing is sent again.
    assert_eq!(
        do_sign_table_order(&app.state, o).await.unwrap(),
        SlipOutcome::NothingToPrint
    );
    assert_eq!(app.fiscal_events().len(), 3);

    // First slip at table 4, the new line at the old table, then the
    // notice with both tables and no amount.
    let slips = printer.settle_slips(4).await;
    assert_eq!(slips.len(), 3, "{slips:?}");
    let text: Vec<String> = slips
        .iter()
        .map(|s| pos_print::render::render_plain_text(s.as_bytes()))
        .collect();
    assert!(text[1].contains("Table 4") && text[1].contains("1 x Maki saumon"));
    assert!(text[2].contains("TRANSFERT DE TABLE"), "{}", text[2]);
    assert!(text[2].contains("Table 4 -> Table 11"), "{}", text[2]);
    assert!(!text[2].contains("EUR") && !text[2].contains(",00"));
}

#[tokio::test]
async fn the_sale_after_a_move_is_on_the_new_table_with_the_same_reference() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 1);
    add(&app, o, 1, 2);
    do_transfer_table(&app.state, o, 5).await.unwrap();

    let sale = do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);

    let requests = requests(&app);
    let (order, transfer, sale) = (&requests[0], &requests[1], &requests[2]);
    assert_eq!(order["costCenter"]["id"], "T01");
    assert_eq!(sale["costCenter"]["id"], "T05");
    assert_eq!(
        sale["costCenter"]["reference"],
        order["costCenter"]["reference"]
    );
    assert_eq!(
        transfer["transfer"]["to"][0]["costCenter"]["reference"],
        order["costCenter"]["reference"]
    );
    // The sale sends the order's journal once: 2 Sake.
    let lines = sale["transaction"]["transactionLines"].as_array().unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["mainProduct"]["quantity"], 2.0);
    // Tables view: nothing left on table 1 or 5.
    let tables = do_list_configured_tables(&app.state).unwrap();
    assert!(tables.iter().all(|t| t.status == "free"));
}

#[tokio::test]
async fn the_move_is_counted_on_a_training_label() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    app.state.config.lock().unwrap().training_mode = true;
    let o = table_order(&app, 2);
    add(&app, o, 1, 1);

    do_transfer_table(&app.state, o, 3).await.unwrap();

    let events = app.fiscal_events();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.event_type == "T"), "{events:?}");
    assert_eq!(app.counter_for(EventLabel::T), 2);
    assert_eq!(app.counter_for(EventLabel::P), 0);
    assert_eq!(table_of(&app, o), Some(3));
}

#[tokio::test]
async fn a_waiter_moves_a_table() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 2);
    add(&app, o, 1, 1);
    app.login_as("85073003328", pos_core::user::UserRole::Waiter);

    do_transfer_table(&app.state, o, 3).await.unwrap();

    assert_eq!(table_of(&app, o), Some(3));
    // The move is signed in the waiter's name.
    let requests = requests(&app);
    assert_eq!(requests.last().unwrap()["employeeId"], "85073003328");
}

#[tokio::test]
async fn without_a_session_nothing_moves() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 2);
    add(&app, o, 1, 1);
    app.logout();
    assert_eq!(refused(&app, o, 3).await, errors::NOT_LOGGED_IN);
    assert_eq!(table_of(&app, o), Some(2));
}

// ── FDM failures ───────────────────────────────────────────────────

#[tokio::test]
async fn with_the_fdm_down_nothing_is_signed_and_the_table_stays() {
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();

    let error = refused(&app, o, 8).await;

    assert!(error.starts_with("FDM error"), "{error}");
    assert!(app.fiscal_events().is_empty());
    assert_eq!(table_of(&app, o), Some(4));
    assert_eq!(app.counter_for(EventLabel::P), 0);
}

#[tokio::test]
async fn a_late_answer_moves_the_table_when_the_request_is_sent_again() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signCostCenterChange");
    let app = app_with(&fdm).await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);

    let first = refused(&app, o, 8).await;
    assert!(
        first.contains(errors::FDM_ANSWER_PENDING),
        "answer awaited: {first}"
    );
    // The order was registered; the table has not moved yet.
    assert_eq!(table_of(&app, o), Some(4));
    assert!(app.open_pending().is_some());
    // The kitchen still got the slip of the entries the FDM signed.
    assert_eq!(printer.settle_slips(1).await.len(), 1);

    // The same request again: the FDM returns its first answer, the
    // move is archived once and the table changes.
    let moved = do_transfer_table(&app.state, o, 8).await.unwrap();
    assert_eq!((moved.from_table, moved.to_table), (4, 8));
    assert_eq!(table_of(&app, o), Some(8));
    let sent = fdm.sent_op("signCostCenterChange");
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].body, sent[1].body, "the same bytes");
    assert_eq!(app.fiscal_events().len(), 2);
    assert!(app.open_pending().is_none());
    let slips = printer.settle_slips(2).await;
    assert!(slips.iter().any(|s| s.contains("TRANSFERT DE TABLE")));
}

#[tokio::test]
async fn the_background_tick_moves_the_table_of_a_late_answer() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signCostCenterChange");
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    assert!(do_transfer_table(&app.state, o, 8).await.is_err());
    assert_eq!(table_of(&app, o), Some(4));

    // The tick also runs at boot.
    assert_eq!(close_overdue_day(&app.state).await, Ok(false));

    assert_eq!(table_of(&app, o), Some(8));
    assert_eq!(app.fiscal_events().len(), 2);
}

#[tokio::test]
async fn a_ghost_on_the_target_is_voided_by_a_late_answer_too() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signCostCenterChange");
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    let ghost = table_order(&app, 8);
    assert!(do_transfer_table(&app.state, o, 8).await.is_err());
    assert_eq!(app.order_status(ghost).as_deref(), Some("open"));

    close_overdue_day(&app.state).await.unwrap();

    assert_eq!(table_of(&app, o), Some(8));
    assert_eq!(app.order_status(ghost).as_deref(), Some("voided"));
}

#[tokio::test]
async fn a_late_answer_never_puts_two_open_orders_on_one_table() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signCostCenterChange");
    let app = app_with(&fdm).await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    assert!(do_transfer_table(&app.state, o, 8).await.is_err());
    // While the answer is awaited, guests sit at table 8 and order.
    let taken = table_order(&app, 8);
    add(&app, taken, 2, 1);

    // The same request again: the move is archived (fiscal truth) but
    // the order stays on its source table.
    let error = refused(&app, o, 8).await;

    assert_eq!(error, errors::TABLE_TRANSFER_NOT_MOVED);
    assert_eq!(table_of(&app, o), Some(4));
    assert_eq!(table_of(&app, taken), Some(8));
    assert_eq!(app.order_status(o).as_deref(), Some("open"));
    assert_eq!(app.order_status(taken).as_deref(), Some("open"));
    assert_eq!(app.fiscal_events().len(), 2, "order + move archived once");
    assert!(app.open_pending().is_none());
    // No "table moved" notice for the kitchen.
    let slips = printer.settle_slips(1).await;
    assert!(!slips.iter().any(|s| s.contains("TRANSFERT DE TABLE")));
}

#[tokio::test]
async fn the_background_tick_keeps_the_order_put_when_the_target_was_taken() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signCostCenterChange");
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 1, 1);
    assert!(do_transfer_table(&app.state, o, 8).await.is_err());
    let taken = table_order(&app, 8);
    add(&app, taken, 2, 1);

    close_overdue_day(&app.state).await.unwrap();

    assert_eq!(table_of(&app, o), Some(4));
    assert_eq!(table_of(&app, taken), Some(8));
    assert_eq!(app.order_status(taken).as_deref(), Some("open"));
    assert_eq!(app.fiscal_events().len(), 2);
    assert!(app.open_pending().is_none());
}

// ── Price changes travel with their line ───────────────────────────

#[cfg(feature = "hubrise")]
#[tokio::test]
async fn price_changes_are_carried_negated_on_the_old_table_with_the_same_grouping() {
    use pos_db::queries::price_changes::{NewPriceChange, add_to_line};
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    let line = add(&app, o, 2, 2);
    {
        let db = app.state.db.lock().unwrap();
        add_to_line(
            &db,
            line,
            &[NewPriceChange {
                grouping_id: 7,
                change_id: "promo",
                name: "Promo -10 %",
                scope: "EVENT",
                change_type: "PUBLIC",
                amount_cents: -240,
            }],
        )
        .unwrap();
    }

    do_transfer_table(&app.state, o, 5).await.unwrap();

    let requests = requests(&app);
    let transfer = &requests.last().unwrap()["transfer"];
    let change = |side: &str| -> Value {
        transfer[side][0]["transaction"]["transactionLines"][0]["mainProduct"]["vats"][0]
            ["priceChanges"][0]
            .clone()
    };
    let (from, to) = (change("from"), change("to"));
    assert_eq!(to["amount"], -2.4);
    assert_eq!(from["amount"], 2.4);
    for key in ["id", "name", "scope", "type", "groupingId"] {
        assert_eq!(from[key], to[key], "{key}");
    }
    assert_eq!(to["groupingId"], 7);
    // 2 x 12,00 - 2,40 on the new table, the opposite on the old one.
    assert_eq!(transfer["to"][0]["transaction"]["transactionTotal"], 21.6);
    assert_eq!(
        transfer["from"][0]["transaction"]["transactionTotal"],
        -21.6
    );
}

// ── Reports ────────────────────────────────────────────────────────

#[tokio::test]
async fn an_x_report_after_a_move_counts_it_once_under_p_and_its_negative_lines() {
    use pos_engine::commands::report::do_generate_x_report;
    let fdm = LossyFdm::start().await;
    let app = app_with(&fdm).await;
    let o = table_order(&app, 4);
    add(&app, o, 2, 1);
    do_transfer_table(&app.state, o, 5).await.unwrap();

    let x = do_generate_x_report(&app.state, OPERATOR.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);

    let request = fdm.sent_op("signReportTurnoverX")[0].variables()["data"].clone();
    let data = request.get("turnover").cloned().unwrap_or(request);
    // The order registration and the move are two P events; the move's
    // lines come to nothing (from and to cancel out).
    let p = data["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["eventLabel"] == "P")
        .unwrap()
        .clone();
    assert_eq!(p["ticketCount"], 2);
    assert_eq!(p["amount"], 12.0, "12,00 ordered, 0 moved");
    let moved = data["negQuantities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["negQuantityReason"] == "COST_CENTER_CHANGE")
        .expect("the move's negative line is reported")
        .clone();
    assert_eq!(moved["negQuantityCount"], 1);
    assert_eq!(moved["ticketCount"], 1);
    assert_eq!(moved["amount"], -12.0);
}
