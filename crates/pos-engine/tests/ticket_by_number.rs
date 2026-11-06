//! Find a ticket by the number printed on it (UX audit R2), for the copy and the refund.
//!
//! - a number finds its sale in the whole archive, whatever the recent lists hold;
//! - a number nobody holds, zero and a negative number find nothing;
//! - each label counts its own numbers: a sale and a pre-bill with one number both come back;
//! - a sale that cannot be refunded comes back with the very error the refund gives:
//!   everything refunded, a refund, an unpaid sale, training mode;
//! - the command is the manager's, like the copy.
//!
//! Events are signed by the admission image's mock FDM.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::auth::do_set_pin;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::copy::do_list_copyable_tickets;
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::refund::{
    ERR_REFUND_IN_TRAINING, ERR_REFUND_OF_REFUND, RefundItem, TicketLookup,
    do_find_ticket_by_number, do_list_refundable_events, do_refund_checkout,
};
use pos_engine::commands::tables::do_sign_pre_bill;
use pos_engine::commands::unpaid::do_close_unpaid;

/// A manager (refunds and copies are theirs), with a well-formed INSZ.
const MANAGER: &str = "12345678958";
const SERVER: &str = "84022899837";
const PIN: &str = "4321";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, UserRole::Admin);
    do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (1, 1, 1200, 'A', 0, 1),
                (2, 1, 2800, 'B', 1, 1);",
        )
        .unwrap();
    app
}

/// An open order of 2 x item 1 and 1 x item 2, for a table or (None) a takeaway.
fn order(app: &TestApp, table: Option<i32>) -> i64 {
    let id = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: if table.is_some() {
                OrderType::DineIn
            } else {
                OrderType::Takeaway
            },
            table_number: table,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    for (item, quantity) in [(1, 2), (2, 1)] {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: id,
                menu_item_id: item,
                quantity,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    id
}

/// A sale paid by card; returns its fiscal event id.
async fn sale(app: &TestApp) -> i64 {
    let id = order(app, None);
    let r = do_checkout(&app.state, id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

/// The ticket number printed on event `event`.
fn number_of(app: &TestApp, event: i64) -> i64 {
    app.fiscal_events()
        .iter()
        .find(|e| e.id == event)
        .unwrap()
        .sequence_number
}

fn find(app: &TestApp, number: i64) -> TicketLookup {
    do_find_ticket_by_number(&app.state, number).unwrap()
}

/// What the refund of `lines` of `event` answers: its error, if any.
async fn refund_error(app: &TestApp, event: i64, lines: &[(i64, i32)]) -> Option<String> {
    let items = lines
        .iter()
        .map(|&(menu_item_id, quantity)| RefundItem {
            line_id: None,
            menu_item_id,
            quantity,
        })
        .collect();
    match do_refund_checkout(&app.state, event, items, MANAGER.into()).await {
        Ok(r) if r.success => None,
        Ok(r) => r.error,
        Err(e) => Some(e),
    }
}

#[tokio::test]
async fn a_printed_number_finds_its_sale_for_the_copy_and_the_refund() {
    let app = app().await;
    let event = sale(&app).await;
    let number = number_of(&app, event);

    let lookup = find(&app, number);
    assert_eq!(lookup.number, number);
    assert_eq!(lookup.matches.len(), 1);
    let found = &lookup.matches[0];
    assert_eq!(found.fiscal_event_id, event);
    assert_eq!(found.event_label, "N");
    assert_eq!(found.ticket_number, number);
    assert_eq!(found.total_cents, 5200);
    assert!(!found.is_refund);
    assert_eq!(found.refund_blocked, None);
    let copyable = found.copyable.as_ref().expect("the customer got a ticket");
    assert_eq!(copyable.fiscal_event_id, event);
    let refundable = found.refundable.as_ref().expect("the sale can go back");
    assert_eq!(refundable.fiscal_event_id, event);
    assert_eq!(refundable.total_cents, 5200);
    assert!(refundable.order_id > 0);

    // Same facts as the two lists the tab shows.
    let listed = do_list_refundable_events(&app.state, None).unwrap();
    assert_eq!(listed[0].order_id, refundable.order_id);
    let copies = do_list_copyable_tickets(&app.state, None).unwrap();
    assert_eq!(copies[0].created_at, copyable.created_at);
}

#[tokio::test]
async fn a_number_finds_a_sale_older_than_the_recent_lists() {
    let app = app().await;
    let oldest = sale(&app).await;
    for _ in 0..3 {
        sale(&app).await;
    }
    // A list limited to one sale holds one row; the number reaches the oldest sale anyway.
    assert_eq!(
        do_list_refundable_events(&app.state, Some(1))
            .unwrap()
            .len(),
        1
    );
    let lookup = find(&app, number_of(&app, oldest));
    assert_eq!(lookup.matches.len(), 1);
    assert_eq!(lookup.matches[0].fiscal_event_id, oldest);
    assert!(lookup.matches[0].refundable.is_some());
}

#[tokio::test]
async fn a_number_nobody_holds_finds_nothing() {
    let app = app().await;
    let event = sale(&app).await;
    let number = number_of(&app, event);
    assert!(find(&app, number + 500).matches.is_empty());
    assert!(find(&app, 0).matches.is_empty());
    assert!(find(&app, -4).matches.is_empty());
    assert!(find(&app, i64::MAX).matches.is_empty());
    // The answer keeps the number asked for.
    assert_eq!(find(&app, number + 500).number, number + 500);
}

#[tokio::test]
async fn a_sale_and_a_pre_bill_with_one_number_both_come_back() {
    let app = app().await;
    let table_order = order(&app, Some(4));
    do_sign_pre_bill(&app.state, table_order).await.unwrap();
    // The table order took P number 1, the pre-bill P number 2.
    let pre_bill_number = app
        .fiscal_events()
        .iter()
        .filter(|e| e.event_type == "P")
        .map(|e| e.sequence_number)
        .max()
        .unwrap();
    assert_eq!(pre_bill_number, 2);
    // Each label counts its own numbers: the second sale is N number 2 as well.
    sale(&app).await;
    let event = sale(&app).await;
    let number = number_of(&app, event);
    assert_eq!(number, pre_bill_number);
    let lookup = find(&app, number);
    let labels: Vec<_> = lookup
        .matches
        .iter()
        .map(|m| m.event_label.as_str())
        .collect();
    assert_eq!(lookup.matches.len(), 2, "{labels:?}");
    assert!(labels.contains(&"N") && labels.contains(&"P"), "{labels:?}");
    let pre = lookup
        .matches
        .iter()
        .find(|m| m.event_label == "P")
        .unwrap();
    assert!(pre.copyable.is_some(), "a pre-bill can be copied");
    assert!(pre.refundable.is_none() && pre.refund_blocked.is_none());
    assert_eq!(pre.table_number, Some(4));
    // Newest first.
    assert!(lookup.matches[0].fiscal_event_id > lookup.matches[1].fiscal_event_id);
}

#[tokio::test]
async fn a_sale_fully_refunded_comes_back_with_the_reason_but_stays_copyable() {
    let app = app().await;
    let event = sale(&app).await;
    let number = number_of(&app, event);

    // A part refunded: something can still go back.
    assert_eq!(refund_error(&app, event, &[(1, 1)]).await, None);
    let lookup = find(&app, number);
    let sale_match = lookup
        .matches
        .iter()
        .find(|m| m.fiscal_event_id == event)
        .unwrap();
    assert!(sale_match.refundable.is_some());
    assert_eq!(sale_match.refund_blocked, None);

    // The rest refunded: nothing is left.
    assert_eq!(refund_error(&app, event, &[(1, 1), (2, 1)]).await, None);
    let lookup = find(&app, number);
    let sale_match = lookup
        .matches
        .iter()
        .find(|m| m.fiscal_event_id == event)
        .unwrap();
    assert!(sale_match.refundable.is_none());
    assert_eq!(
        sale_match.refund_blocked.as_deref(),
        Some(errors::REFUND_ALREADY_DONE)
    );
    assert!(sale_match.copyable.is_some(), "the copy stays possible");
    // The refund itself says no too (the quantities exceed what is left).
    let refused = refund_error(&app, event, &[(1, 1)]).await.unwrap();
    assert!(
        refused.starts_with(errors::REFUND_EXCEEDS_SOLD),
        "{refused}"
    );
}

#[tokio::test]
async fn a_refund_ticket_is_found_and_is_not_refundable_with_the_refund_code() {
    let app = app().await;
    let event = sale(&app).await;
    assert_eq!(refund_error(&app, event, &[(2, 1)]).await, None);
    let refund_event = app
        .fiscal_events()
        .iter()
        .filter(|e| e.event_type == "N")
        .map(|e| e.id)
        .max()
        .unwrap();
    assert_ne!(refund_event, event);

    let found = find(&app, number_of(&app, refund_event))
        .matches
        .into_iter()
        .find(|m| m.fiscal_event_id == refund_event)
        .unwrap();
    assert!(found.is_refund);
    assert_eq!(found.total_cents, -2800);
    assert!(found.refundable.is_none());
    assert_eq!(found.refund_blocked.as_deref(), Some(ERR_REFUND_OF_REFUND));
    // The refund of a refund gives the same code.
    assert_eq!(
        refund_error(&app, refund_event, &[(2, 1)]).await.as_deref(),
        Some(ERR_REFUND_OF_REFUND)
    );
}

#[tokio::test]
async fn a_sale_closed_unpaid_is_not_refundable() {
    let app = app().await;
    let id = order(&app, Some(7));
    let closed = do_close_unpaid(&app.state, id, MANAGER, PIN).await.unwrap();
    assert!(closed.success, "{:?}", closed.error);
    let event = closed.fiscal_event_id.unwrap();

    let found = find(&app, number_of(&app, event))
        .matches
        .into_iter()
        .find(|m| m.fiscal_event_id == event)
        .unwrap();
    assert!(found.refundable.is_none());
    assert_eq!(
        found.refund_blocked.as_deref(),
        Some(errors::REFUND_UNPAID_SALE)
    );
    assert_eq!(
        refund_error(&app, event, &[(1, 1)]).await.as_deref(),
        Some(errors::REFUND_UNPAID_SALE)
    );
}

#[tokio::test]
async fn in_training_mode_a_real_sale_is_not_refundable() {
    let app = app().await;
    let event = sale(&app).await;
    app.state.config.lock().unwrap().training_mode = true;

    let found = find(&app, number_of(&app, event))
        .matches
        .into_iter()
        .find(|m| m.fiscal_event_id == event)
        .unwrap();
    assert!(found.refundable.is_none());
    assert_eq!(
        found.refund_blocked.as_deref(),
        Some(ERR_REFUND_IN_TRAINING)
    );
    assert_eq!(
        refund_error(&app, event, &[(1, 1)]).await.as_deref(),
        Some(ERR_REFUND_IN_TRAINING)
    );
}

#[tokio::test]
async fn only_a_manager_may_look_a_ticket_up() {
    let app = app().await;
    let event = sale(&app).await;
    let number = number_of(&app, event);
    app.login_as(SERVER, UserRole::Waiter);
    assert!(do_find_ticket_by_number(&app.state, number).is_err());
    app.logout();
    assert!(do_find_ticket_by_number(&app.state, number).is_err());
}

#[tokio::test]
async fn the_lookup_reads_and_signs_nothing() {
    let app = app().await;
    let event = sale(&app).await;
    let events = app.fiscal_events().len();
    let orders: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0))
        .unwrap();
    for number in [number_of(&app, event), 999, 0] {
        find(&app, number);
    }
    assert_eq!(app.fiscal_events().len(), events);
    let after: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM orders", [], |r| r.get(0))
        .unwrap();
    assert_eq!(after, orders);
}
