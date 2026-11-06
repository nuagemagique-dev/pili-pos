//! One manager PIN per order for discounts (UX audit P8).
//!
//! A manager who confirmed a discount, an offered item or an offered line
//! with their PIN gives the next ones on the same order without the PIN for
//! two minutes. The grant is the engine's: bound to the logged-in user, the
//! order and an expiry, dropped by the lock, the logout, the end of a shift,
//! an order switch and the sale. Every use is written in the audit trail
//! `till_discounts` with `pin_reused`. "Client parti sans payer", the payment
//! correction and the remote support keep asking the PIN every time.
//!
//! Nothing here changes what is signed: the last test compares the archived
//! requests of a sale with and without the grant.
//!
//! The expiry runs on a paused tokio clock: `advance` is the fake clock.

#![cfg(feature = "hubrise")]
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::time::Duration;

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::discount::{
    DiscountInput, DiscountKind, DiscountReason, DiscountResult, DiscountTarget, do_apply_discount,
    do_remove_discount,
};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::session::{
    LeaveReason, do_leave_order, do_lock_session, do_set_active_order,
};

const MANAGER: &str = "12345678958";
const OTHER_MANAGER: &str = "94060242576";
const WAITER: &str = "84022899837";
const PIN: &str = "4321";
const OTHER_PIN: &str = "1111";

/// Catalogue: 1 Sake 6,50 A, 2 Maki 12,00 B, 3 Eau 3,00 A.
async fn world() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OTHER_MANAGER, UserRole::Admin);
    app.login_as(MANAGER, UserRole::Admin);
    pos_engine::commands::auth::do_set_pin(&app.state, MANAGER.into(), PIN.into(), None).unwrap();
    pos_engine::commands::auth::do_set_pin(
        &app.state,
        OTHER_MANAGER.into(),
        OTHER_PIN.into(),
        Some(PIN.into()),
    )
    .unwrap();
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1),
                (2, 1, 1200, 'B', 1, 1, 1),
                (3, 1, 300, 'A', 0, 2, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake'),
                ('menu_item', 2, 'name', 'fr', 'Maki saumon'),
                ('menu_item', 3, 'name', 'fr', 'Eau plate');",
        )
        .unwrap();
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

fn takeaway(app: &TestApp) -> i64 {
    do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id
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

fn free_line(line_id: i64) -> DiscountInput {
    DiscountInput {
        target: DiscountTarget::Line { line_id },
        kind: DiscountKind::Free,
        value: None,
        reason_code: DiscountReason::StaffMeal,
        reason_text: None,
    }
}

fn order_percent(value: i64) -> DiscountInput {
    DiscountInput {
        target: DiscountTarget::Order,
        kind: DiscountKind::Percent,
        value: Some(value),
        reason_code: DiscountReason::CommercialGesture,
        reason_text: None,
    }
}

async fn with_pin(
    app: &TestApp,
    order: i64,
    input: DiscountInput,
) -> Result<DiscountResult, String> {
    do_apply_discount(&app.state, order, input, MANAGER.into(), Some(PIN.into())).await
}

async fn without_pin(
    app: &TestApp,
    order: i64,
    input: DiscountInput,
) -> Result<DiscountResult, String> {
    do_apply_discount(&app.state, order, input, MANAGER.into(), None).await
}

/// `(action, manager, pin_reused)` of the order's audit rows, oldest first.
fn audit(app: &TestApp, order: i64) -> Vec<(String, String, bool)> {
    let db = app.state.db.lock().unwrap();
    pos_db::queries::till_discounts::list(&db, order)
        .unwrap()
        .into_iter()
        .map(|r| (r.action, r.manager_insz, r.pin_reused))
        .collect()
}

fn total(app: &TestApp, order: i64) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT total_cents FROM orders WHERE id = ?1",
            [order],
            |r| r.get(0),
        )
        .unwrap()
}

/// Two lines to offer, a PIN-confirmed offer on the first: the grant is open.
async fn granted_order(app: &TestApp) -> (i64, i64, i64) {
    let o = takeaway(app);
    let sake = add(app, o, 1, 1);
    let maki = add(app, o, 2, 1);
    let first = with_pin(app, o, free_line(sake)).await.unwrap();
    assert!(!first.pin_reused);
    (o, sake, maki)
}

#[tokio::test]
async fn a_typed_pin_covers_the_next_offers_of_the_order() {
    let (app, _p) = world().await;
    let (o, _sake, maki) = granted_order(&app).await;

    let second = without_pin(&app, o, free_line(maki)).await.unwrap();
    assert!(second.pin_reused);
    // The grant keeps running from the typed PIN: about two minutes left.
    let left = second.pin_grant_ms.unwrap();
    assert!((100_000..=120_000).contains(&left), "{left}");
    assert_eq!(total(&app, o), 0);

    // A removal rides the grant too, and a new offer after it as well.
    let removed = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap();
    assert!(removed.pin_reused);
    assert_eq!(total(&app, o), 1200);
    let again = without_pin(&app, o, free_line(maki)).await.unwrap();
    assert!(again.pin_reused);

    // Every use is in the audit trail, with the flag.
    let m = MANAGER.to_string();
    assert_eq!(
        audit(&app, o),
        vec![
            ("apply".into(), m.clone(), false),
            ("apply".into(), m.clone(), true),
            ("remove".into(), m.clone(), true),
            ("apply".into(), m, true),
        ]
    );
}

#[tokio::test]
async fn a_discount_on_the_whole_order_rides_the_grant_too() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 1);
    add(&app, o, 2, 1);
    // The first offer opens the grant; the order discount of the next one
    // is refused while a free line exists (one change at a time), so remove
    // the offer without the PIN, then discount the order.
    with_pin(&app, o, free_line(sake)).await.unwrap();
    do_remove_discount(&app.state, o, Some(sake), MANAGER.into(), None)
        .await
        .unwrap();
    let done = without_pin(&app, o, order_percent(10)).await.unwrap();
    assert!(done.pin_reused);
    assert_eq!(total(&app, o), 1850 - 185);
}

#[tokio::test]
async fn without_a_grant_the_pin_is_required_and_nothing_is_written() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 1);

    let err = without_pin(&app, o, free_line(sake)).await.unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);
    let err = do_remove_discount(&app.state, o, None, MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);
    assert_eq!(audit(&app, o).len(), 0);
    assert_eq!(total(&app, o), 650);

    // A waiter or a cashier is still told they may not, grant or not.
    for role in [UserRole::Waiter, UserRole::Cashier] {
        app.login_as(WAITER, role);
        let err = do_apply_discount(&app.state, o, free_line(sake), WAITER.into(), None)
            .await
            .unwrap_err();
        assert_eq!(err, errors::FORBIDDEN, "{role:?}");
    }
}

#[tokio::test]
async fn a_wrong_typed_pin_is_not_rescued_by_the_grant() {
    let (app, _p) = world().await;
    let (o, _sake, maki) = granted_order(&app).await;
    let err = do_apply_discount(
        &app.state,
        o,
        free_line(maki),
        MANAGER.into(),
        Some("0000".into()),
    )
    .await
    .unwrap_err();
    assert_eq!(err, errors::PIN_WRONG);
    assert_eq!(audit(&app, o).len(), 1);
}

#[tokio::test]
async fn the_grant_runs_out_after_two_minutes_on_the_clock() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 1);
    let maki = add(&app, o, 2, 1);
    let eau = add(&app, o, 3, 1);
    with_pin(&app, o, free_line(sake)).await.unwrap();

    // From here the clock is ours: no network is involved on a takeaway.
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(119)).await;
    let near = without_pin(&app, o, free_line(maki)).await.unwrap();
    assert!(near.pin_reused);
    // About one second left (a few real milliseconds passed before `pause`).
    let left = near.pin_grant_ms.unwrap();
    assert!(
        (1..=1_000).contains(&left),
        "the reuse did not extend it: {left}"
    );

    tokio::time::advance(Duration::from_secs(2)).await;
    let err = without_pin(&app, o, free_line(eau)).await.unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);

    // Typing the PIN again opens a new grant for two more minutes.
    let renewed = with_pin(&app, o, free_line(eau)).await.unwrap();
    assert!(!renewed.pin_reused);
    assert!(renewed.pin_grant_ms.unwrap() >= 119_000);
    assert_eq!(
        audit(&app, o).iter().map(|r| r.2).collect::<Vec<_>>(),
        vec![false, true, false]
    );
}

#[tokio::test]
async fn another_order_does_not_ride_the_grant() {
    let (app, _p) = world().await;
    let (_o, _s, _m) = granted_order(&app).await;
    let other = takeaway(&app);
    let sake = add(&app, other, 1, 1);
    let err = without_pin(&app, other, free_line(sake)).await.unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);
    assert_eq!(audit(&app, other).len(), 0);
}

#[tokio::test]
async fn another_manager_does_not_ride_the_grant() {
    let (app, _p) = world().await;
    let (o, _sake, maki) = granted_order(&app).await;
    app.login_as(OTHER_MANAGER, UserRole::Admin);
    let err = do_apply_discount(&app.state, o, free_line(maki), OTHER_MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);
    // Their own PIN works, and it is theirs: the first manager's grant is gone.
    let done = do_apply_discount(
        &app.state,
        o,
        free_line(maki),
        OTHER_MANAGER.into(),
        Some(OTHER_PIN.into()),
    )
    .await
    .unwrap();
    assert!(!done.pin_reused);
    app.login_as(MANAGER, UserRole::Admin);
    let err = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);
}

#[tokio::test]
async fn a_lock_a_logout_and_a_new_login_end_the_grant() {
    let (app, _p) = world().await;

    let (o, _sake, maki) = granted_order(&app).await;
    do_lock_session(&app.state).await.unwrap();
    app.login_as(MANAGER, UserRole::Admin);
    let err = without_pin(&app, o, free_line(maki)).await.unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED, "after the lock");

    // Typed again, then a logout through the engine.
    with_pin(&app, o, free_line(maki)).await.unwrap();
    pos_engine::commands::auth::do_logout(&app.state).unwrap();
    app.login_as(MANAGER, UserRole::Admin);
    let err = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED, "after the logout");

    // A real login (PIN) starts without a grant, even for the same manager.
    do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), Some(PIN.into()))
        .await
        .unwrap();
    with_pin(&app, o, free_line(maki)).await.unwrap();
    pos_engine::commands::auth::do_login(&app.state, MANAGER.into(), Some(PIN.into())).unwrap();
    let err = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED, "after a new login");
}

#[tokio::test]
async fn the_end_of_a_shift_ends_the_grant() {
    let (app, _p) = world().await;
    let (o, _sake, maki) = granted_order(&app).await;
    let result =
        pos_engine::commands::social::do_end_shift(&app.state, MANAGER.into(), Some(PIN.into()))
            .await
            .unwrap();
    // The FDM answer does not matter: the session is closed in every case.
    let _ = result;
    assert!(app.state.session.lock().unwrap().is_none());
    app.login_as(MANAGER, UserRole::Admin);
    let err = without_pin(&app, o, free_line(maki)).await.unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED);
}

#[tokio::test]
async fn an_order_switch_ends_the_grant_but_the_same_order_and_the_kitchen_do_not() {
    let (app, _p) = world().await;
    let (o, _sake, maki) = granted_order(&app).await;
    let other = takeaway(&app);
    add(&app, other, 1, 1);

    // The screen shows the order again, or sends it to the kitchen: kept.
    do_set_active_order(&app.state, Some(o)).unwrap();
    do_leave_order(&app.state, o, LeaveReason::SendToKitchen)
        .await
        .unwrap();
    do_set_active_order(&app.state, Some(o)).unwrap();
    assert!(
        without_pin(&app, o, free_line(maki))
            .await
            .unwrap()
            .pin_reused
    );

    // Another order on screen: gone, and it does not come back with the first.
    do_set_active_order(&app.state, Some(other)).unwrap();
    do_set_active_order(&app.state, Some(o)).unwrap();
    let err = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(
        err,
        errors::PIN_REQUIRED,
        "after another order was on screen"
    );

    // Leaving the order for another screen also ends it.
    do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), Some(PIN.into()))
        .await
        .unwrap();
    with_pin(&app, o, free_line(maki)).await.unwrap();
    do_leave_order(&app.state, o, LeaveReason::Navigate)
        .await
        .unwrap();
    let err = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED, "after leaving the order");

    // No order on screen at all.
    do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), Some(PIN.into()))
        .await
        .unwrap();
    with_pin(&app, o, free_line(maki)).await.unwrap();
    do_set_active_order(&app.state, None).unwrap();
    let err = do_remove_discount(&app.state, o, Some(maki), MANAGER.into(), None)
        .await
        .unwrap_err();
    assert_eq!(err, errors::PIN_REQUIRED, "with no order on screen");
}

#[tokio::test]
async fn the_sale_ends_the_grant() {
    let (app, _p) = world().await;
    let o = takeaway(&app);
    let sake = add(&app, o, 1, 1);
    add(&app, o, 2, 1);
    with_pin(&app, o, free_line(sake)).await.unwrap();
    let now = tokio::time::Instant::now();
    assert!(app.state.session_rules.pin_grants().covers(MANAGER, o, now));

    let sold = do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(sold.success, "{:?}", sold.error);
    assert!(
        !app.state
            .session_rules
            .pin_grants()
            .covers(MANAGER, o, tokio::time::Instant::now()),
        "the sold order keeps no grant"
    );
}

#[tokio::test]
async fn the_other_pin_actions_keep_asking_for_the_pin_every_time() {
    let (app, _p) = world().await;
    let (o, _sake, _maki) = granted_order(&app).await;
    let now = tokio::time::Instant::now();
    assert!(app.state.session_rules.pin_grants().covers(MANAGER, o, now));

    // "Client parti sans payer": the PIN or nothing, and the order stays open.
    let unpaid = pos_engine::commands::unpaid::do_close_unpaid(&app.state, o, MANAGER, "")
        .await
        .unwrap();
    assert!(!unpaid.success);
    assert_eq!(unpaid.error.as_deref(), Some(errors::PIN_WRONG));

    // A payment correction (the PIN is checked before anything is looked up).
    let correction = pos_engine::commands::payment_correction::do_correct_payment(
        &app.state,
        1,
        0,
        pos_fiscal::payment::PaymentMethod::Cash,
        MANAGER.into(),
        String::new(),
    )
    .await
    .unwrap();
    assert!(!correction.success);
    assert_eq!(correction.error.as_deref(), Some(errors::PIN_WRONG));

    // The remote support.
    let err = pos_engine::commands::support::do_start_remote_support(
        &app.state,
        MANAGER.into(),
        String::new(),
    )
    .unwrap_err();
    assert_eq!(err, errors::PIN_WRONG);

    // And the grant was not spent by any of it.
    assert!(
        app.state
            .session_rules
            .pin_grants()
            .covers(MANAGER, o, tokio::time::Instant::now())
    );
}

#[tokio::test]
async fn the_grant_changes_nothing_that_is_signed() {
    // The same two offers and the same sale, once with a PIN each time and
    // once on the grant: the archived requests are the same bar their own
    // identifiers, and none of them mentions the PIN.
    async fn sale(second_pin: Option<&str>) -> Vec<String> {
        let (app, _p) = world().await;
        let o = takeaway(&app);
        let sake = add(&app, o, 1, 1);
        let maki = add(&app, o, 2, 1);
        with_pin(&app, o, free_line(sake)).await.unwrap();
        do_apply_discount(
            &app.state,
            o,
            free_line(maki),
            MANAGER.into(),
            second_pin.map(Into::into),
        )
        .await
        .unwrap();
        let sold = do_checkout(&app.state, o, CheckoutPaymentMethod::Cash)
            .await
            .unwrap();
        assert!(sold.success, "{:?}", sold.error);
        app.fiscal_events()
            .into_iter()
            .filter_map(|e| e.event_data_json)
            .collect()
    }
    let typed = sale(Some(PIN)).await;
    let reused = sale(None).await;
    assert_eq!(typed.len(), reused.len());
    for text in typed.iter().chain(&reused) {
        let lower = text.to_lowercase();
        assert!(
            !lower.contains("pin_reused") && !lower.contains("pinreused"),
            "{text}"
        );
    }
    // Same sold lines and payments in the signed N.
    let n_lines = |texts: &[String]| -> (Vec<serde_json::Value>, serde_json::Value) {
        let n = texts
            .iter()
            .map(|t| serde_json::from_str::<serde_json::Value>(t).unwrap())
            .find(|v| v["transaction"]["transactionLines"].is_array())
            .expect("the signed sale");
        (
            n["transaction"]["transactionLines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l["lineTotal"].clone())
                .collect(),
            n["transaction"]["payments"].clone(),
        )
    };
    assert_eq!(n_lines(&typed), n_lines(&reused));
}
