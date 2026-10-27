//! An order whose P or N waits for the FDM's answer cannot change: a
//! line added meanwhile would never be covered by a signed event. Every
//! path that adds a line refuses with `ORDER_SIGN_PENDING`.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::errors;
use pos_engine::commands::modifiers::{AddWithModifiersInput, do_add_order_line_with_modifiers};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};

const OPERATOR: &str = "12345678958";

async fn app() -> (TestApp, LossyFdm) {
    let fdm = LossyFdm::start().await;
    let app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 1200, 'B', 1, 0, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Maki saumon');",
        )
        .unwrap();
    (app, fdm)
}

/// A dine-in order with one line, its sale left waiting for the FDM.
async fn pending_order(app: &TestApp, fdm: &LossyFdm) -> i64 {
    fdm.lose_first("signSale");
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(1),
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id;
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    let _ = do_checkout(&app.state, order, CheckoutPaymentMethod::Card).await;
    order
}

#[tokio::test]
async fn a_plain_line_is_refused_while_the_sale_waits() {
    let (app, fdm) = app().await;
    let order = pending_order(&app, &fdm).await;
    let added = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    );
    assert_eq!(
        added.map(|l| l.id),
        Err(errors::ORDER_SIGN_PENDING.to_string())
    );
}

#[tokio::test]
async fn a_line_with_options_is_refused_while_the_sale_waits() {
    let (app, fdm) = app().await;
    let order = pending_order(&app, &fdm).await;
    let added = do_add_order_line_with_modifiers(
        &app.state,
        &AddWithModifiersInput {
            order_id: order,
            menu_item_id: 1,
            quantity: 1,
            modifier_ids: vec![],
        },
    );
    assert_eq!(
        added.map(|l| l.id),
        Err(errors::ORDER_SIGN_PENDING.to_string())
    );
}
