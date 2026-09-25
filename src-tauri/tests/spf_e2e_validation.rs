//! Input validation and auth checks across the command layer: unknown
//! or inactive menu items, non-positive quantities, empty/zero-total
//! checkouts, unknown ids (order / fiscal event), and — for the one
//! command that actually authenticates anyone — wrong user / PIN.
//!
//! `login`/`create_user`/`set_pin` previously had no `do_*` inner
//! function (unlike every other command module), so they couldn't be
//! driven from an integration test at all. This suite is also the
//! regression test for the minimal refactor in `commands/auth.rs` that
//! added them, following the same pattern used everywhere else in the
//! command layer.

mod common;

use common::*;

use moyun_pos::commands::auth::{CreateUserInput, do_create_user, do_login, do_set_pin};
use moyun_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use moyun_pos::commands::copy::do_copy_ticket;
use moyun_pos::commands::order::{AddOrderLineInput, do_add_order_line};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout};
use moyun_pos::commands::tables::{do_sign_pre_bill, do_sign_table_order};
use pos_core::order::OrderType;
use pos_db::queries::orders::create_order;

/// Two mod-97-valid Belgian INSZ numbers, distinct from `TEST_INSZ`
/// used by the other suites (which is deliberately NOT a valid INSZ,
/// since none of those commands validate the format).
const VALID_INSZ_1: &str = "12345678958";
const VALID_INSZ_2: &str = "12345678987";

fn seed_catalogue(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1), (2, 1, 0);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
            (1, 1, 1000, 'B', 0, 1),
            (2, 2, 1000, 'B', 0, 1); -- category 2 is inactive
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
            (3, 1, 1000, 'B', 1, 0); -- item itself inactive",
    )
    .unwrap();
}

// ── login ────────────────────────────────────────────────────────────

#[tokio::test]
async fn login_rejects_a_malformed_insz() {
    let app = TestApp::new().await;
    let result = do_login(&app.state, "not-an-insz".to_string(), None);
    assert!(result.is_err());
}

#[tokio::test]
async fn login_rejects_an_unknown_user() {
    let app = TestApp::new().await;
    let result = do_login(&app.state, VALID_INSZ_1.to_string(), None);
    assert!(
        result.is_err(),
        "a well-formed but unregistered INSZ must be refused"
    );
}

#[tokio::test]
async fn login_succeeds_without_a_pin_when_none_is_set() {
    let app = TestApp::new().await;
    do_create_user(
        &app.state,
        CreateUserInput {
            insz: VALID_INSZ_1.to_string(),
            name: "No Pin".to_string(),
            role: "waiter".to_string(),
        },
    )
    .unwrap();

    let user = do_login(&app.state, VALID_INSZ_1.to_string(), None).unwrap();
    assert_eq!(user.insz, VALID_INSZ_1);
    assert!(!user.has_pin);
}

#[tokio::test]
async fn login_requires_the_pin_once_one_is_set() {
    let app = TestApp::new().await;
    do_create_user(
        &app.state,
        CreateUserInput {
            insz: VALID_INSZ_1.to_string(),
            name: "Pinned".to_string(),
            role: "cashier".to_string(),
        },
    )
    .unwrap();
    do_set_pin(&app.state, VALID_INSZ_1.to_string(), "1234".to_string()).unwrap();

    let no_pin = do_login(&app.state, VALID_INSZ_1.to_string(), None);
    assert!(no_pin.is_err(), "PIN required but not supplied");

    let wrong_pin = do_login(
        &app.state,
        VALID_INSZ_1.to_string(),
        Some("0000".to_string()),
    );
    assert!(wrong_pin.is_err(), "wrong PIN must be refused");

    let right_pin = do_login(
        &app.state,
        VALID_INSZ_1.to_string(),
        Some("1234".to_string()),
    );
    assert!(right_pin.is_ok(), "{:?}", right_pin.err());
    assert!(right_pin.unwrap().has_pin);
}

#[tokio::test]
async fn set_pin_rejects_non_4_digit_values() {
    let app = TestApp::new().await;
    do_create_user(
        &app.state,
        CreateUserInput {
            insz: VALID_INSZ_1.to_string(),
            name: "Test".to_string(),
            role: "waiter".to_string(),
        },
    )
    .unwrap();

    assert!(do_set_pin(&app.state, VALID_INSZ_1.to_string(), "123".to_string()).is_err());
    assert!(do_set_pin(&app.state, VALID_INSZ_1.to_string(), "12345".to_string()).is_err());
    assert!(do_set_pin(&app.state, VALID_INSZ_1.to_string(), "abcd".to_string()).is_err());
    assert!(do_set_pin(&app.state, VALID_INSZ_1.to_string(), "12a4".to_string()).is_err());
}

#[tokio::test]
async fn create_user_rejects_a_malformed_insz_and_an_unknown_role() {
    let app = TestApp::new().await;
    let bad_insz = do_create_user(
        &app.state,
        CreateUserInput {
            insz: "123".to_string(),
            name: "Test".to_string(),
            role: "waiter".to_string(),
        },
    );
    assert!(bad_insz.is_err());

    let bad_role = do_create_user(
        &app.state,
        CreateUserInput {
            insz: VALID_INSZ_2.to_string(),
            name: "Test".to_string(),
            role: "manager".to_string(), // not a real UserRole
        },
    );
    assert!(bad_role.is_err());
}

// ── add_order_line: catalogue + quantity guards ─────────────────────

#[tokio::test]
async fn add_order_line_rejects_an_unknown_menu_item() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        create_order(&conn, OrderType::DineIn, Some(1), VALID_INSZ_1)
            .unwrap()
            .id
    };
    let result = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: 9999,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn add_order_line_rejects_an_item_whose_category_is_inactive() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        create_order(&conn, OrderType::DineIn, Some(1), VALID_INSZ_1)
            .unwrap()
            .id
    };
    let result = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: 2, // active item, inactive category
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn add_order_line_rejects_an_inactive_item() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        create_order(&conn, OrderType::DineIn, Some(1), VALID_INSZ_1)
            .unwrap()
            .id
    };
    let result = do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id,
            menu_item_id: 3, // active category, inactive item
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    );
    assert!(result.is_err());
}

#[tokio::test]
async fn add_order_line_rejects_zero_and_negative_quantities() {
    let app = TestApp::new().await;
    seed_catalogue(&app);
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        create_order(&conn, OrderType::DineIn, Some(1), VALID_INSZ_1)
            .unwrap()
            .id
    };
    for quantity in [0, -1, -5] {
        let result = do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id,
                menu_item_id: 1,
                quantity,
                modifiers_json: None,
                unit_price_cents: None,
            },
        );
        assert!(result.is_err(), "quantity {quantity} must be rejected");
    }
}

// ── checkout / refund / copy / P: unknown ids, empty input ──────────

#[tokio::test]
async fn checkout_of_an_unknown_order_id_errors() {
    let app = TestApp::new().await;
    let result = do_checkout(&app.state, 999_999, CheckoutPaymentMethod::Cash).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn checkout_of_an_empty_order_is_refused_without_calling_the_fdm() {
    let app = TestApp::new().await;
    let order_id = {
        let conn = app.state.db.lock().unwrap();
        create_order(&conn, OrderType::DineIn, Some(1), VALID_INSZ_1)
            .unwrap()
            .id
    };
    // No mock mounted: any FDM call would panic the wiremock server
    // with an unmatched-request error and fail the test.
    let result = do_checkout(&app.state, order_id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(!result.success);
    assert!(
        result
            .error
            .as_deref()
            .is_some_and(|e| e.contains("no lines"))
    );
}

#[tokio::test]
async fn refund_of_an_unknown_fiscal_event_errors() {
    let app = TestApp::new().await;
    let result = do_refund_checkout(
        &app.state,
        999_999,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        VALID_INSZ_1.to_string(),
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn refund_with_no_items_is_refused_without_calling_the_fdm() {
    let app = TestApp::new().await;
    // Any fiscal_event_id works here: the empty-items guard runs first.
    let result = do_refund_checkout(&app.state, 1, vec![], VALID_INSZ_1.to_string())
        .await
        .unwrap();
    assert!(!result.success);
}

#[tokio::test]
async fn copy_of_an_unknown_fiscal_event_errors() {
    let app = TestApp::new().await;
    let result = do_copy_ticket(&app.state, 999_999, VALID_INSZ_1.to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn table_order_and_pre_bill_of_an_unknown_order_id_error() {
    let app = TestApp::new().await;
    assert!(do_sign_table_order(&app.state, 999_999).await.is_err());
    assert!(do_sign_pre_bill(&app.state, 999_999).await.is_err());
}
