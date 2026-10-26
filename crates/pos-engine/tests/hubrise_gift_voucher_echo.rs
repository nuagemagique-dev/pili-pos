//! What the `HubRise` echo of a till sale does with gift vouchers
//! (V12-D31): a sale that sells a voucher is not echoed, a voucher
//! payment goes out as `HubRise`'s generic "Other" payment.

#![cfg(feature = "hubrise")]
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::TestApp;
use common::hubrise::{connected_app, drain_outbox, poller};
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, PaymentPart, do_checkout_split};
use pos_engine::commands::gift_vouchers::{
    GiftVoucherSettings, do_add_gift_voucher_line, do_set_gift_voucher_settings,
};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_hubrise::poller::start;

const INSZ: &str = "12345678958";
const ORDERS: &str = "/v1/locations/pili-dev-loc-1/orders";

async fn app() -> (TestApp, tempfile::TempDir, common::hubrise::HubriseMock) {
    let (app, dir, mock) = connected_app().await;
    app.use_admission_mock().await;
    start(&poller(&app)).await.unwrap();
    app.login_as(INSZ, UserRole::Admin);
    do_set_gift_voucher_settings(
        &app.state,
        GiftVoucherSettings {
            enabled: true,
            ..GiftVoucherSettings::default()
        },
    )
    .unwrap();
    (app, dir, mock)
}

fn order(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: INSZ.into(),
        },
    )
    .unwrap()
    .id
}

fn outbox_rows(app: &TestApp) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT COUNT(*) FROM hubrise_outbox", [], |r| r.get(0))
        .unwrap()
}

/// Two miso soups (7,00 EUR).
fn add_soups(app: &TestApp, order_id: i64) {
    let miso: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT id FROM menu_items WHERE external_id = 'MISO-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: miso,
            quantity: 2,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
}

async fn sell_voucher(app: &TestApp, cents: i64) -> i64 {
    let o = order(app);
    do_add_gift_voucher_line(&app.state, o, cents).unwrap();
    let r = do_checkout_split(
        &app.state,
        o,
        &[PaymentPart::of(CheckoutPaymentMethod::Card, cents)],
        None,
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);
    app.state
        .db
        .lock()
        .unwrap()
        .query_row("SELECT MAX(id) FROM gift_vouchers", [], |r| r.get(0))
        .unwrap()
}

#[tokio::test]
async fn a_sale_that_sells_a_voucher_is_not_echoed() {
    let (app, _db, mock) = app().await;
    sell_voucher(&app, 5000).await;
    assert_eq!(outbox_rows(&app), 0, "nothing queued");
    drain_outbox(&app).await;
    assert!(mock.requests("POST", ORDERS).is_empty(), "no order posted");

    // Control: the same till, a plain sale, is echoed.
    let o = order(&app);
    add_soups(&app, o);
    let r = do_checkout_split(&app.state, o, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(outbox_rows(&app), 1);
    assert_eq!(drain_outbox(&app).await.sent, 1);
    assert_eq!(mock.requests("POST", ORDERS).len(), 1);
}

#[tokio::test]
async fn a_voucher_payment_is_echoed_as_other() {
    let (app, _db, mock) = app().await;
    let voucher = sell_voucher(&app, 5000).await;

    let o = order(&app);
    add_soups(&app, o);
    let r = do_checkout_split(
        &app.state,
        o,
        &[PaymentPart::gift_voucher(500, voucher), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);
    assert_eq!(drain_outbox(&app).await.sent, 1);

    let posts = mock.requests("POST", ORDERS);
    assert_eq!(posts.len(), 1, "one echo, the voucher sale added none");
    let body = posts[0].body.as_ref().unwrap();
    let payments = body["payments"].as_array().unwrap();
    assert_eq!(payments.len(), 2, "{payments:?}");
    assert_eq!(payments[0]["name"], "Other");
    assert_eq!(payments[0]["amount"], "5.00 EUR");
    assert_eq!(payments[1]["name"], "Cash");
    assert_eq!(payments[1]["amount"], "2.00 EUR");
    assert_eq!(body["total"], "7.00 EUR");
}
