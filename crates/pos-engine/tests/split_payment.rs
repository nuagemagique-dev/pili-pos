//! Paying a sale in several parts (UX audit P3): friends who split the bill, in cash or not.
//!
//! The SPF rule (POS detailed description v2.2, section 7.1, "L'exploitant arrondi uniquement
//! les paiements en especes", step 2): "les paiements en especes sont traites. Ceux-ci
//! apparaissent comme une seule ligne de paiement sur le ticket, ARRONDIE", and "le total des
//! arrondis ne peut etre superieur a 2 centimes d'euro". So the signed request carries ONE cash
//! payment line and ONE rounding line, whatever the number of friends who paid in cash. The
//! screens let several friends pay in cash (each with their own amount given and change) and
//! send ONE cash part without amount, which pays what is left; the engine accepts that request
//! unchanged and still refuses a request with two cash parts.
//!
//! The split by dishes is a screen matter: the units checked for a part never reach the engine,
//! the payment list is the same as for equal parts (see `ui/e2e/checkout-dishes.spec.ts`).

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_core::order::OrderType;
use pos_engine::commands::checkout::{CheckoutPaymentMethod::Card, PaymentPart, do_checkout_split};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_fiscal::payment::{PaymentPart as FiscalPart, settle};

const MANAGER: &str = "12345678958";
/// A dish at 10,03: three of them are a bill of 30,09, three friends at 10,03.
const SUSHI: i64 = 31001;
/// A dish at 4,60.
const SOUP: i64 = 31002;

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
                (31001, 1, 1003, 'B', 0, 1),
                (31002, 1, 460, 'B', 1, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('category', 1, 'name', 'fr', 'Plats'),
                ('menu_item', 31001, 'name', 'fr', 'Sushi Mix'),
                ('menu_item', 31002, 'name', 'fr', 'Soupe Miso');",
        )
        .unwrap();
    app
}

fn sale_of(app: &TestApp, items: &[(i64, i32)]) -> i64 {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: MANAGER.into(),
        },
    )
    .unwrap()
    .id;
    for &(item, quantity) in items {
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
        .unwrap();
    }
    order
}

/// `(type, amountType, amount)` of the financials of the last signed event.
fn financials(app: &TestApp) -> Vec<(String, String, f64)> {
    let event = app.fiscal_events().pop().expect("the signed sale");
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["type"].as_str().unwrap().to_string(),
                f["amountType"].as_str().unwrap().to_string(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect()
}

/// The cash the signed sale asks for, in cents: the CASH payment line plus its ROUNDING line.
fn signed_cash_due_cents(app: &TestApp) -> i64 {
    let cents = |x: f64| format!("{x:.2}").replace('.', "").parse::<i64>().unwrap();
    financials(app)
        .iter()
        .filter(|f| f.0 == "CASH")
        .map(|f| cents(f.2))
        .sum()
}

/// What each friend hands over, from the engine's own rounding: `settle` of the running cash
/// total gives the cash due so far, and a friend hands over the difference with the cash due
/// before their part. The Compose checkout asks the engine the same way (`cashDueAfter`) and the
/// web one mirrors it in `cashDueCents` (`ui/src/utils/dishSplit.ts`, whose unit test uses the
/// same fixtures), so the amounts shown to the friends are the ones the signed line adds up to.
fn dues_from_the_engine(cash_parts: &[i64]) -> Vec<i64> {
    let (mut taken, mut before) = (0, 0);
    cash_parts
        .iter()
        .map(|part| {
            taken += part;
            let due = settle(taken, &[FiscalPart::cash()])
                .unwrap()
                .cash_due_cents();
            let hands_over = due - before;
            before = due;
            hands_over
        })
        .collect()
}

fn row(t: &str, a: &str, x: f64) -> (String, String, f64) {
    (t.to_string(), a.to_string(), x)
}

/// Three friends pay 10,03 each in cash (the screen shows 10,05, 10,00 and 10,05 to hand over,
/// the rounding of the one cash line shared between them): the request is the same as for one
/// cash payment of 30,09, ONE cash line and ONE rounding of +0,01.
#[tokio::test]
async fn three_friends_in_cash_sign_one_cash_line() {
    let app = app().await;
    let order = sale_of(&app, &[(SUSHI, 3)]);
    // What the screens send: one cash part without amount, whatever the number of cash parts.
    let r = do_checkout_split(&app.state, order, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);

    let fin = financials(&app);
    assert_eq!(
        fin,
        [row("CASH", "PAYMENT", 30.09), row("CASH", "ROUNDING", 0.01)]
    );
    assert_eq!(
        fin.iter()
            .filter(|f| f.0 == "CASH" && f.1 == "PAYMENT")
            .count(),
        1
    );
    // The three friends hand over 10,05 + 10,00 + 10,05, and that adds up to what the signed
    // sale asks for: the cash line with its rounding.
    let dues = dues_from_the_engine(&[1003, 1003, 1003]);
    assert_eq!(dues, [1005, 1000, 1005]);
    assert_eq!(dues.iter().sum::<i64>(), signed_cash_due_cents(&app));
}

/// Cash, card, then cash again on a bill of 34,69 (30,09 of sushi and 4,60 of soup): friend 1
/// pays 11,57 in cash, friend 2 11,56 by card, friend 3 11,56 in cash. The two cash parts are
/// ONE cash line of 23,13 rounded once to 23,15: friend 1 hands over 11,55 (11,57 rounded) and
/// friend 3 11,60, the rounding of the whole line shared between them (11,55 + 11,60 = 23,15).
#[tokio::test]
async fn cash_card_cash_signs_one_cash_line_rounded_once() {
    let app = app().await;
    let order = sale_of(&app, &[(SUSHI, 3), (SOUP, 1)]);
    let r = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::of(Card, 1156), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert!(r.success, "{:?}", r.error);

    let fin = financials(&app);
    assert_eq!(
        fin,
        [
            row("CARD_DEBIT", "PAYMENT", 11.56),
            row("CASH", "PAYMENT", 23.13),
            row("CASH", "ROUNDING", 0.02),
        ]
    );
    // What the two friends hand over adds up to the cash line with its rounding.
    let dues = dues_from_the_engine(&[1157, 1156]);
    assert_eq!(dues, [1155, 1160]);
    assert_eq!(dues.iter().sum::<i64>(), signed_cash_due_cents(&app));
}

/// The spec wants ONE cash line: a request with two cash parts is still refused, the screens
/// merge them.
#[tokio::test]
async fn two_cash_parts_in_the_request_are_refused() {
    let app = app().await;
    let order = sale_of(&app, &[(SUSHI, 3)]);
    let r = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::cash(), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert!(!r.success);
    assert_eq!(r.error.as_deref(), Some("payment_invalid"));
    // Nothing was signed.
    assert!(app.fiscal_events().is_empty());
}
