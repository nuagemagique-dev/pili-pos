//! The copy picker lists what a customer can ask a copy of (manual
//! §III, event C): sales, refunds, pre-bills and training tickets the
//! FDM signed. Order registrations, clock-ins, cash movements and
//! copies themselves are never offered.
//!
//! Events are signed by the admission image's mock FDM.

mod common;

use common::*;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::copy::{do_copy_ticket, do_list_copyable_tickets};
use moyun_pos::commands::financial::{MoneyDirection, do_money_in_out};
use moyun_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::social::do_work_in;
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_core::user::UserRole;

/// A manager, with a well-formed INSZ.
const MANAGER: &str = "12345678958";

async fn app() -> TestApp {
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
                (1, 1, 1200, 'A', 0, 1),
                (2, 1, 2800, 'B', 1, 1);",
        )
        .unwrap();
    app
}

async fn order(app: &TestApp, table: Option<i32>, lines: &[(i64, i32)]) -> i64 {
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
    for &(menu_item_id, quantity) in lines {
        do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: id,
                menu_item_id,
                quantity,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
    }
    id
}

async fn sale(app: &TestApp, lines: &[(i64, i32)]) -> i64 {
    let id = order(app, None, lines).await;
    let r = do_checkout(&app.state, id, CheckoutPaymentMethod::Card)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

fn listed(app: &TestApp) -> Vec<(String, i64)> {
    do_list_copyable_tickets(&app.state, None)
        .unwrap()
        .into_iter()
        .map(|t| (t.event_label, t.total_cents))
        .collect()
}

#[tokio::test]
async fn the_picker_offers_every_ticket_a_customer_received_and_nothing_else() {
    let app = app().await;
    do_work_in(&app.state, MANAGER.into()).await.unwrap();
    let sold = sale(&app, &[(1, 2), (2, 1)]).await; // 52,00

    // A table: its order registration (P, no ticket), then a pre-bill.
    let table = order(&app, Some(4), &[(2, 1)]).await;
    do_sign_table_order(&app.state, table).await.unwrap();
    do_sign_pre_bill(&app.state, table).await.unwrap();

    let r = do_refund_checkout(
        &app.state,
        sold,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);
    let r = do_money_in_out(
        &app.state,
        MANAGER.into(),
        5000,
        MoneyDirection::In,
        "float".into(),
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);

    assert_eq!(
        listed(&app),
        [
            ("N".to_string(), -1200),
            ("P".into(), 2800),
            ("N".into(), 5200)
        ],
        "refund, pre-bill, sale: newest first, each with its own total"
    );

    // Copies are signed, and are never offered for copying themselves.
    let r = do_copy_ticket(&app.state, sold, MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(listed(&app).len(), 3);
}

#[tokio::test]
async fn every_listed_ticket_can_be_copied() {
    let app = app().await;
    sale(&app, &[(1, 1)]).await;
    let table = order(&app, Some(2), &[(1, 1), (2, 2)]).await;
    do_sign_pre_bill(&app.state, table).await.unwrap();

    for ticket in do_list_copyable_tickets(&app.state, None).unwrap() {
        let r = do_copy_ticket(&app.state, ticket.fiscal_event_id, MANAGER.into())
            .await
            .unwrap();
        assert!(
            r.success,
            "{} #{}: {:?}",
            ticket.event_label, ticket.ticket_number, r.error
        );
        assert_eq!(app.fiscal_events().pop().unwrap().event_type, "C");
    }
}

#[tokio::test]
async fn training_tickets_are_listed_as_training() {
    let app = app().await;
    app.state.config.lock().unwrap().training_mode = true;
    sale(&app, &[(2, 1)]).await;
    let tickets = do_list_copyable_tickets(&app.state, None).unwrap();
    assert_eq!(tickets.len(), 1);
    assert_eq!(tickets[0].event_label, "T");

    let r = do_copy_ticket(&app.state, tickets[0].fiscal_event_id, MANAGER.into())
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(
        app.fiscal_events().pop().unwrap().event_type,
        "T",
        "a training copy is T"
    );
}

#[tokio::test]
async fn only_a_manager_lists_the_tickets() {
    let app = app().await;
    sale(&app, &[(1, 1)]).await;
    for role in [UserRole::Cashier, UserRole::Waiter] {
        app.login_as("84022899837", role);
        assert!(
            do_list_copyable_tickets(&app.state, None).is_err(),
            "{role:?}"
        );
    }
    app.logout();
    assert!(do_list_copyable_tickets(&app.state, None).is_err());
}
