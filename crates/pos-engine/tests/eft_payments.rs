//! A card payment taken on the linked terminal, from the request to the
//! sale that settles it (`commands::eft`, `commands::checkout`).
//!
//! The terminal is scripted (`pos_eft::scripted`), the FDM is the
//! admission image's mock, which signs for real.
//!
//! - an approved payment is signed as an AUTOMATIC card line with the
//!   card type and brand the terminal returned, and marked used with
//!   the sale's event;
//! - nothing is charged for a sale that cannot be signed (FDM down);
//! - a payment that waits for its answer blocks every other payment and
//!   the sale of its order until the terminal is asked again;
//! - an approved payment is never forgotten: the sale uses it, for its
//!   exact amount, or is refused.

use std::sync::Arc;

use pos_engine_testkit as common;

use common::*;
use pos_eft::scripted::{Call, Script};
use pos_eft::{CardType, EftError, Outcome};
use pos_engine::commands::checkout::{PaymentPart, do_checkout, do_checkout_split};
use pos_engine::commands::eft::{
    EftPayment, PaymentTerminalStatus, do_check_payment_terminal, do_eft_abort, do_eft_check,
    do_eft_pay, do_list_eft_payments, do_list_unsettled_card_payments,
};
use pos_engine::commands::errors;
use pos_engine::commands::order::{
    AddOrderLineInput, do_add_order_line, do_remove_order_line, do_update_order_line_quantity,
};
use pos_engine::commands::settings::do_get_settings;
use serde_json::Value;

const MANAGER: &str = "12345678958";

async fn app() -> (TestApp, Arc<Script>) {
    let mut app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let script = app.link_terminal();
    (app, script)
}

/// Charge `cents` of `order`, the terminal approving it.
async fn approved(
    app: &TestApp,
    script: &Script,
    order: i64,
    cents: i64,
    brand: &str,
    card: CardType,
) -> EftPayment {
    script.on_pay(Ok(approval(cents, brand, card)));
    let payment = do_eft_pay(&app.state, order, cents).await.unwrap();
    assert_eq!(payment.status, "approved");
    payment
}

/// The request of the sale `event_id`, as it was signed.
fn request_of(app: &TestApp, event_id: i64) -> Value {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .expect("the event");
    serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap()
}

fn payment_rows(app: &TestApp) -> Vec<(String, i64)> {
    let db = app.state.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT method, amount_cents FROM payments ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[tokio::test]
async fn an_approved_payment_is_signed_as_an_automatic_card_line_and_marked_used() {
    let (app, script) = app().await;
    let order = app.order_of(2500);
    let payment = approved(&app, &script, order, 2500, "Bancontact", CardType::Debit).await;
    assert_eq!(payment.order_id, order);
    assert_eq!(payment.amount_cents, 2500);
    assert_eq!(payment.card_type.as_deref(), Some("debit"));
    assert_eq!(payment.brand.as_deref(), Some("Bancontact"));
    assert_eq!(payment.terminal_reference.as_deref(), Some("TXN-2500"));
    assert_eq!(payment.masked_pan.as_deref(), Some("XXXX XXXX XXXX 1234"));
    assert!(!payment.used);
    // What the checkout screen shows, also after a restart.
    assert_eq!(
        do_list_eft_payments(&app.state, order).unwrap(),
        vec![payment.clone()]
    );

    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(2500, payment.id)],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let event_id = sale.fiscal_event_id.unwrap();

    let request = request_of(&app, event_id);
    let lines = request["financials"].as_array().unwrap();
    assert_eq!(lines.len(), 1, "no rounding on a card payment");
    let card = &lines[0];
    assert_eq!(card["id"], "2");
    assert_eq!(card["name"], "Card");
    assert_eq!(card["type"], "CARD_DEBIT");
    assert_eq!(card["inputMethod"], "AUTOMATIC");
    assert_eq!(card["provider"], "Bancontact");
    assert_eq!(card["reference"], "TXN-2500");
    assert_eq!(card["amountType"], "PAYMENT");
    assert_eq!(card["amount"], 25.0);

    let rows = app.eft_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].used_event_id, Some(event_id));
    assert_eq!(app.order_status(order).as_deref(), Some("closed"));
    assert_eq!(payment_rows(&app), [("card".to_string(), 2500)]);
    assert!(
        do_list_eft_payments(&app.state, order).unwrap().is_empty(),
        "a settled payment is not listed any more"
    );
    // The reference sent to the terminal is the one the row keeps.
    assert_eq!(
        script.calls(),
        vec![Call::Pay {
            reference: rows[0].reference.clone(),
            amount_cents: 2500
        }]
    );
}

#[tokio::test]
async fn a_credit_card_has_its_own_line_and_the_cash_rest_alone_is_rounded() {
    let (app, script) = app().await;
    let order = app.order_of(1486);
    let payment = approved(&app, &script, order, 1000, "Visa", CardType::Credit).await;
    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(1000, payment.id), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let request = request_of(&app, sale.fiscal_event_id.unwrap());
    let summary: Vec<_> = request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["id"].as_str().unwrap().to_string(),
                f["type"].as_str().unwrap().to_string(),
                f["inputMethod"].as_str().unwrap().to_string(),
                f["amountType"].as_str().unwrap().to_string(),
                f["amount"].as_f64().unwrap(),
            )
        })
        .collect();
    let row = |id: &str, t: &str, m: &str, a: &str, x: f64| {
        (
            id.to_string(),
            t.to_string(),
            m.to_string(),
            a.to_string(),
            x,
        )
    };
    assert_eq!(
        summary,
        [
            row("6", "CARD_CREDIT", "AUTOMATIC", "PAYMENT", 10.0),
            row("1", "CASH", "MANUAL", "PAYMENT", 4.86),
            row("1", "CASH", "MANUAL", "ROUNDING", -0.01),
        ]
    );
    assert_eq!(request["financials"][0]["provider"], "Visa");
    assert!(request["financials"][1].get("provider").is_none());
    assert_eq!(
        payment_rows(&app),
        [("card".to_string(), 1000), ("cash".to_string(), 485)]
    );
}

#[tokio::test]
async fn a_card_the_terminal_does_not_type_is_signed_card_unknown() {
    let (app, script) = app().await;
    let order = app.order_of(900);
    let payment = approved(&app, &script, order, 900, "Mastercard", CardType::Unknown).await;
    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(900, payment.id)],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let card = &request_of(&app, sale.fiscal_event_id.unwrap())["financials"][0];
    assert_eq!(card["type"], "CARD_UNKNOWN");
    assert_eq!(card["id"], "7");
    assert_eq!(card["provider"], "Mastercard");
}

#[tokio::test]
async fn a_card_typed_by_hand_is_unchanged_with_a_terminal_linked() {
    let (app, script) = app().await;
    let order = app.order_of(700);
    let sale = do_checkout(
        &app.state,
        order,
        pos_engine::commands::checkout::CheckoutPaymentMethod::Card,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let card = &request_of(&app, sale.fiscal_event_id.unwrap())["financials"][0];
    assert_eq!(
        (
            card["id"].as_str(),
            card["name"].as_str(),
            card["type"].as_str()
        ),
        (Some("2"), Some("Card"), Some("CARD_DEBIT"))
    );
    assert_eq!(card["inputMethod"], "MANUAL");
    assert!(card.get("provider").is_none() && card.get("reference").is_none());
    assert!(script.calls().is_empty());
}

#[tokio::test]
async fn a_declined_payment_charges_nothing_and_cannot_be_used() {
    let (app, script) = app().await;
    let order = app.order_of(1200);
    script.on_pay(Ok(Outcome::Declined {
        reason: "insufficient funds".into(),
    }));
    let payment = do_eft_pay(&app.state, order, 1200).await.unwrap();
    assert_eq!(payment.status, "declined");
    assert_eq!(payment.detail.as_deref(), Some("insufficient funds"));
    assert!(payment.card_type.is_none());

    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(1200, payment.id)],
        None,
    )
    .await
    .unwrap();
    assert!(!sale.success);
    assert_eq!(sale.error.as_deref(), Some(errors::EFT_PAYMENT_NOT_USABLE));
    assert!(app.fiscal_events().is_empty());
    // Nothing is waiting: the operator can ask again, or take cash.
    assert!(do_list_eft_payments(&app.state, order).unwrap().is_empty());
    script.on_pay(Ok(approval(1200, "Maestro", CardType::Debit)));
    assert_eq!(
        do_eft_pay(&app.state, order, 1200).await.unwrap().status,
        "approved"
    );
}

#[tokio::test]
async fn a_cancelled_payment_is_final_and_nothing_is_forgotten() {
    let (app, script) = app().await;
    let order = app.order_of(1200);
    script.on_pay(Ok(Outcome::Cancelled));
    let payment = do_eft_pay(&app.state, order, 1200).await.unwrap();
    assert_eq!(payment.status, "cancelled");
    // A final answer is returned as it is when asked about again.
    let again = do_eft_check(&app.state, payment.id).await.unwrap();
    assert_eq!(again, payment);
    assert_eq!(script.calls().len(), 1, "no call for a final answer");
    let cash = do_checkout_split(&app.state, order, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(cash.success, "{:?}", cash.error);
}

#[tokio::test]
async fn nothing_is_charged_when_the_fdm_is_down() {
    // The default FDM of a TestApp answers nothing: no sale can be signed.
    let mut app = TestApp::new().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let script = app.link_terminal();
    let order = app.order_of(1200);
    let error = do_eft_pay(&app.state, order, 1200).await.unwrap_err();
    assert!(error.starts_with("FDM"), "{error}");
    assert!(script.calls().is_empty(), "the terminal was never asked");
    assert!(app.eft_rows().is_empty(), "and nothing was stored");
}

#[tokio::test]
async fn nothing_is_charged_while_a_sign_request_waits_for_the_fdm() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signSale");
    let mut app = TestApp::new().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let script = app.link_terminal();
    let first = app.order_of(500);
    let lost = do_checkout(
        &app.state,
        first,
        pos_engine::commands::checkout::CheckoutPaymentMethod::Cash,
    )
    .await
    .unwrap();
    assert!(!lost.success);
    assert!(app.open_pending().is_some());

    let second = app.order_of(1200);
    let error = do_eft_pay(&app.state, second, 1200).await.unwrap_err();
    assert!(error.contains(errors::FDM_ANSWER_PENDING), "{error}");
    assert!(script.calls().is_empty());
}

#[tokio::test]
async fn a_blocking_fdm_condition_is_not_operational_either() {
    let (app, script) = app().await;
    // module2's buffer is full: the status says so, no sale could be signed.
    app.fdm.reset().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::body_string_contains("status(language"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": { "fdmStatus": {
                "device": { "fdmId": "FDM1", "fdmSwVersion": "2", "fdmDateTime": "2026-10-12T10:00:00.000Z", "bufferCapacityUsed": 100.0 },
                "initialized": true,
                "informations": [],
                "warnings": [],
                "errors": [ { "message": "full", "extensions": { "code": "BUFFER_FULL", "showPos": "MANDATORY" } } ]
            } }
        })))
        .mount(&app.fdm)
        .await;
    app.state.config.lock().unwrap().fdm_url = format!("{}/graphql", app.fdm.uri());
    let order = app.order_of(1200);
    let error = do_eft_pay(&app.state, order, 1200).await.unwrap_err();
    assert!(error.starts_with("FDM"), "{error}");
    assert!(script.calls().is_empty());
}

#[tokio::test]
async fn no_terminal_no_payment() {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let order = app.order_of(1200);
    assert_eq!(
        do_eft_pay(&app.state, order, 1200).await.unwrap_err(),
        errors::EFT_NOT_CONFIGURED
    );
    assert_eq!(do_get_settings(&app.state).unwrap().payment_terminal, None);
    assert_eq!(
        do_eft_abort(&app.state).await.unwrap_err(),
        errors::EFT_NOT_CONFIGURED
    );
}

#[tokio::test]
async fn the_settings_say_which_terminal_is_linked() {
    let (app, _script) = app().await;
    assert_eq!(
        do_get_settings(&app.state)
            .unwrap()
            .payment_terminal
            .as_deref(),
        Some("ccv_opi")
    );
    let json = serde_json::to_value(do_get_settings(&app.state).unwrap()).unwrap();
    assert_eq!(json["paymentTerminal"], "ccv_opi");
    app.state.config.lock().unwrap().payment_terminal = None;
    let json = serde_json::to_value(do_get_settings(&app.state).unwrap()).unwrap();
    assert!(
        json.get("paymentTerminal").is_none(),
        "absent when not linked"
    );
}

#[tokio::test]
async fn the_amount_is_positive_and_within_what_is_still_due() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    for cents in [0, -100, 1001] {
        assert_eq!(
            do_eft_pay(&app.state, order, cents).await.unwrap_err(),
            errors::EFT_AMOUNT_INVALID,
            "{cents}"
        );
    }
    assert!(script.calls().is_empty());
    // A part payment is fine, then only the rest is due.
    approved(&app, &script, order, 600, "Bancontact", CardType::Debit).await;
    assert_eq!(
        do_eft_pay(&app.state, order, 401).await.unwrap_err(),
        errors::EFT_AMOUNT_INVALID
    );
    approved(&app, &script, order, 400, "Bancontact", CardType::Debit).await;
    assert_eq!(
        do_eft_pay(&app.state, order, 1).await.unwrap_err(),
        errors::EFT_AMOUNT_INVALID,
        "nothing is left to pay"
    );
}

#[tokio::test]
async fn an_order_is_paid_in_two_terminal_payments_and_both_must_be_used() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    let a = approved(&app, &script, order, 600, "Bancontact", CardType::Debit).await;
    let b = approved(&app, &script, order, 400, "Visa", CardType::Credit).await;
    // One left out: a charged card is never forgotten.
    let forgotten = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(600, a.id), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        forgotten.error.as_deref(),
        Some(errors::EFT_PAYMENT_NOT_USABLE)
    );
    let sale = do_checkout_split(
        &app.state,
        order,
        &[
            PaymentPart::terminal(600, a.id),
            PaymentPart::terminal(400, b.id),
        ],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let event = sale.fiscal_event_id.unwrap();
    assert!(
        app.eft_rows()
            .iter()
            .all(|r| r.used_event_id == Some(event))
    );
    let ids: Vec<_> = request_of(&app, event)["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["2", "6"]);
}

#[tokio::test]
async fn an_approved_payment_cannot_be_left_out_of_the_sale() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    approved(&app, &script, order, 1000, "Bancontact", CardType::Debit).await;
    for parts in [
        vec![PaymentPart::cash()],
        vec![PaymentPart::of(
            pos_engine::commands::checkout::CheckoutPaymentMethod::Card,
            1000,
        )],
    ] {
        let sale = do_checkout_split(&app.state, order, &parts, None)
            .await
            .unwrap();
        assert_eq!(sale.error.as_deref(), Some(errors::EFT_PAYMENT_NOT_USABLE));
    }
    // Nor can the order be closed unpaid with a card charged.
    pos_engine::commands::auth::do_set_pin(&app.state, MANAGER.into(), "4321".into(), None)
        .unwrap();
    let unpaid = pos_engine::commands::unpaid::do_close_unpaid(&app.state, order, MANAGER, "4321")
        .await
        .unwrap();
    assert_eq!(
        unpaid.error.as_deref(),
        Some(errors::EFT_PAYMENT_NOT_USABLE)
    );
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.order_status(order).as_deref(), Some("open"));
}

#[tokio::test]
async fn a_part_must_name_a_usable_payment_of_this_order_for_its_exact_amount() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    let other = app.order_of(1000);
    let theirs = approved(&app, &script, other, 1000, "Bancontact", CardType::Debit).await;
    let mine = approved(&app, &script, order, 600, "Bancontact", CardType::Debit).await;

    let refused = |parts: Vec<PaymentPart>| {
        let app = &app;
        async move {
            let r = do_checkout_split(&app.state, order, &parts, None)
                .await
                .unwrap();
            assert_eq!(
                r.error.as_deref(),
                Some(errors::EFT_PAYMENT_NOT_USABLE),
                "{parts:?}"
            );
        }
    };
    // Another order's payment.
    refused(vec![
        PaymentPart::terminal(1000, theirs.id),
        PaymentPart::cash(),
    ])
    .await;
    // Nothing of that id.
    refused(vec![PaymentPart::terminal(600, 9999), PaymentPart::cash()]).await;
    // Another amount.
    refused(vec![
        PaymentPart::terminal(500, mine.id),
        PaymentPart::cash(),
    ])
    .await;
    // Not a card part.
    refused(vec![
        PaymentPart {
            eft_payment_id: Some(mine.id),
            ..PaymentPart::of(
                pos_engine::commands::checkout::CheckoutPaymentMethod::MealVoucher,
                600,
            )
        },
        PaymentPart::cash(),
    ])
    .await;
    // The same payment twice.
    refused(vec![
        PaymentPart::terminal(600, mine.id),
        PaymentPart::terminal(600, mine.id),
    ])
    .await;
    assert!(app.fiscal_events().is_empty());
    let ok = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(600, mine.id), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert!(ok.success, "{:?}", ok.error);
    // Used: it cannot settle a second sale.
    let again = do_checkout_split(
        &app.state,
        other,
        &[PaymentPart::terminal(600, mine.id), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert_eq!(again.error.as_deref(), Some(errors::EFT_PAYMENT_NOT_USABLE));
}

#[tokio::test]
async fn a_lost_answer_blocks_everything_until_the_terminal_is_asked_again() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.on_pay(Ok(Outcome::Unknown {
        detail: "no answer within 180 s".into(),
    }));
    let payment = do_eft_pay(&app.state, order, 1500).await.unwrap();
    assert_eq!(payment.status, "unknown");
    assert_eq!(payment.detail.as_deref(), Some("no answer within 180 s"));
    assert_eq!(do_list_eft_payments(&app.state, order).unwrap().len(), 1);

    // No second payment, on this order or another.
    let other = app.order_of(800);
    for o in [order, other] {
        assert_eq!(
            do_eft_pay(&app.state, o, 100).await.unwrap_err(),
            errors::EFT_PENDING
        );
    }
    // And no sale of the order: the card may be charged.
    let sale = do_checkout_split(&app.state, order, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert_eq!(sale.error.as_deref(), Some(errors::EFT_PENDING));
    assert!(app.fiscal_events().is_empty());
    // Another order's sale is not concerned.
    assert!(
        do_checkout_split(&app.state, other, &[PaymentPart::cash()], None)
            .await
            .unwrap()
            .success
    );

    // Still not known: still blocked, the detail follows.
    script.on_status(Ok(Outcome::Unknown {
        detail: "still pending at the provider".into(),
    }));
    let still = do_eft_check(&app.state, payment.id).await.unwrap();
    assert_eq!(still.status, "unknown");
    assert_eq!(
        still.detail.as_deref(),
        Some("still pending at the provider")
    );
    assert_eq!(
        do_eft_pay(&app.state, order, 100).await.unwrap_err(),
        errors::EFT_PENDING
    );

    // The terminal tells: approved. The sale can go.
    script.on_status(Ok(approval(1500, "Maestro", CardType::Debit)));
    let checked = do_eft_check(&app.state, payment.id).await.unwrap();
    assert_eq!(checked.status, "approved");
    assert_eq!(checked.brand.as_deref(), Some("Maestro"));
    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(1500, payment.id)],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let reference = app.eft_rows()[0].reference.clone();
    assert!(script.calls().contains(&Call::Status {
        reference: reference.clone()
    }));
}

#[tokio::test]
async fn a_checked_payment_that_was_not_charged_frees_the_till() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.on_pay(Ok(Outcome::Unknown {
        detail: "timeout".into(),
    }));
    let payment = do_eft_pay(&app.state, order, 1500).await.unwrap();
    script.on_status(Ok(Outcome::Declined {
        reason: "card removed".into(),
    }));
    let checked = do_eft_check(&app.state, payment.id).await.unwrap();
    assert_eq!(checked.status, "declined");
    let cash = do_checkout_split(&app.state, order, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(cash.success, "{:?}", cash.error);
}

#[tokio::test]
async fn an_approval_for_another_amount_is_not_signed() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.on_pay(Ok(approval(1400, "Bancontact", CardType::Debit)));
    let payment = do_eft_pay(&app.state, order, 1500).await.unwrap();
    assert_eq!(payment.status, "unknown", "left for a person to sort out");
    assert!(payment.detail.unwrap().contains("14.00"));
}

#[tokio::test]
async fn a_checked_approval_for_another_amount_stays_unknown_and_blocks() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.on_pay(Ok(Outcome::Unknown {
        detail: "timeout".into(),
    }));
    let payment = do_eft_pay(&app.state, order, 1500).await.unwrap();
    // status() cannot compare with the request: the engine does.
    script.on_status(Ok(approval(1600, "Bancontact", CardType::Debit)));
    let checked = do_eft_check(&app.state, payment.id).await.unwrap();
    assert_eq!(checked.status, "unknown");
    assert_eq!(
        checked.detail.as_deref(),
        Some("approved amount 16.00 differs from 15.00, check the terminal")
    );
    assert!(checked.card_type.is_none() && checked.brand.is_none());
    let sale = do_checkout_split(&app.state, order, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert_eq!(sale.error.as_deref(), Some(errors::EFT_PENDING));
    // The right amount, when the terminal says so, is accepted.
    script.on_status(Ok(approval(1500, "Bancontact", CardType::Debit)));
    assert_eq!(
        do_eft_check(&app.state, payment.id).await.unwrap().status,
        "approved"
    );
}

#[tokio::test]
async fn a_request_the_terminal_never_got_is_kept_as_not_sent_with_its_error() {
    for (error, code) in [
        (EftError::Busy, errors::EFT_BUSY.to_string()),
        (
            EftError::Unreachable("connection refused".into()),
            format!("{}|detail=connection refused", errors::EFT_UNREACHABLE),
        ),
        (
            EftError::Rejected("unknown terminal".into()),
            format!("{}|detail=unknown terminal", errors::EFT_REJECTED),
        ),
        (
            EftError::Config("api key unreadable".into()),
            format!("{}|detail=api key unreadable", errors::EFT_REJECTED),
        ),
    ] {
        let (app, script) = app().await;
        let order = app.order_of(1500);
        script.on_pay(Err(error));
        assert_eq!(do_eft_pay(&app.state, order, 1500).await.unwrap_err(), code);
        let rows = app.eft_rows();
        assert_eq!(rows.len(), 1, "the request is kept");
        assert_eq!(rows[0].status.as_str(), "not_sent");
        assert!(rows[0].detail.is_some());
        // Nothing waits: the operator can try again at once.
        script.on_pay(Ok(approval(1500, "Bancontact", CardType::Debit)));
        assert_eq!(
            do_eft_pay(&app.state, order, 1500).await.unwrap().status,
            "approved"
        );
    }
}

#[tokio::test]
async fn a_slow_payment_can_be_aborted_from_the_screen() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.set_pay_delay(std::time::Duration::from_millis(400));
    script.on_pay(Ok(Outcome::Cancelled));

    let pay = do_eft_pay(&app.state, order, 1500);
    let abort = async {
        // Once the payment is out, abort it.
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if app
                .eft_rows()
                .first()
                .is_some_and(|r| r.status.as_str() == "sent")
            {
                break;
            }
        }
        // The abort goes to the instance running the payment.
        let running = app
            .state
            .eft_in_flight
            .lock()
            .unwrap()
            .as_ref()
            .map(|(reference, _)| reference.clone());
        assert_eq!(running, Some(app.eft_rows()[0].reference.clone()));
        do_eft_abort(&app.state).await
    };
    let (payment, aborted) = tokio::join!(pay, abort);
    aborted.unwrap();
    assert_eq!(payment.unwrap().status, "cancelled");
    let reference = app.eft_rows()[0].reference.clone();
    assert_eq!(
        script.calls(),
        vec![
            Call::Pay {
                reference: reference.clone(),
                amount_cents: 1500
            },
            Call::Abort { reference }
        ]
    );
    assert!(app.state.eft_in_flight.lock().unwrap().is_none());
    // Nothing in flight any more: an abort is a no-op.
    do_eft_abort(&app.state).await.unwrap();
    assert_eq!(script.calls().len(), 2);
}

#[tokio::test]
async fn the_order_must_be_open() {
    let (app, script) = app().await;
    let order = app.order_of(500);
    assert!(
        do_checkout_split(&app.state, order, &[PaymentPart::cash()], None)
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        do_eft_pay(&app.state, order, 500).await.unwrap_err(),
        errors::ORDER_NOT_OPEN
    );
    assert_eq!(
        do_eft_pay(&app.state, 987654, 500).await.unwrap_err(),
        errors::ORDER_NOT_OPEN
    );
    assert!(script.calls().is_empty());
}

#[tokio::test]
async fn who_may_ask_for_a_payment_and_who_may_look() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    app.login_as("12345678959", pos_core::user::UserRole::Waiter);
    assert_eq!(
        do_eft_pay(&app.state, order, 500).await.unwrap_err(),
        errors::FORBIDDEN
    );
    assert_eq!(
        do_eft_abort(&app.state).await.unwrap_err(),
        errors::FORBIDDEN
    );
    // A waiter can look at what waits.
    assert!(do_list_eft_payments(&app.state, order).unwrap().is_empty());

    app.login_as("12345678960", pos_core::user::UserRole::Cashier);
    approved(&app, &script, order, 500, "Bancontact", CardType::Debit).await;
    app.login_as("12345678959", pos_core::user::UserRole::Waiter);
    assert_eq!(do_list_eft_payments(&app.state, order).unwrap().len(), 1);
    // Any logged-in user checks a payment (a final one needs no call).
    let id = app.eft_rows()[0].id;
    assert_eq!(
        do_eft_check(&app.state, id).await.unwrap().status,
        "approved"
    );

    app.logout();
    assert_eq!(
        do_eft_pay(&app.state, order, 100).await.unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_list_eft_payments(&app.state, order).unwrap_err(),
        errors::NOT_LOGGED_IN
    );
    assert_eq!(
        do_eft_check(&app.state, id).await.unwrap_err(),
        errors::NOT_LOGGED_IN
    );
}

#[tokio::test]
async fn an_unknown_payment_id_is_not_usable() {
    let (app, _script) = app().await;
    assert_eq!(
        do_eft_check(&app.state, 4242).await.unwrap_err(),
        errors::EFT_PAYMENT_NOT_USABLE
    );
}

#[tokio::test]
async fn a_bad_payment_terminal_section_is_reported_by_the_config_check() {
    let (app, _script) = app().await;
    assert!(
        do_get_settings(&app.state)
            .unwrap()
            .config_problems
            .is_empty()
    );
    {
        let mut config = app.state.config.lock().unwrap();
        let terminal = config.payment_terminal.as_mut().unwrap();
        terminal.address = Some("not-an-address".into());
        terminal.timeout_seconds = 5;
    }
    let problems = do_get_settings(&app.state).unwrap().config_problems;
    assert_eq!(problems.len(), 2, "{problems:?}");
    assert!(problems.iter().all(|p| p.starts_with("payment_terminal.")));
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with("payment_terminal.address"))
    );
    assert!(
        problems
            .iter()
            .any(|p| p.starts_with("payment_terminal.timeout_seconds"))
    );
}

#[tokio::test]
async fn a_training_sale_never_charges_a_real_card() {
    let (app, script) = app().await;
    let order = app.order_of(1250);
    app.state.config.lock().unwrap().training_mode = true;
    let err = do_eft_pay(&app.state, order, 1250).await.unwrap_err();
    assert_eq!(err, errors::EFT_TRAINING);
    assert!(script.calls().is_empty(), "the terminal was never asked");
    assert!(app.eft_rows().is_empty(), "nothing stored");
}

// ── Reachability of the linked terminal ────────────────────────

#[tokio::test]
async fn the_terminal_check_of_an_unlinked_till_probes_nothing() {
    let mut app = TestApp::new().await;
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert_eq!(
        status,
        PaymentTerminalStatus {
            configured: false,
            kind: None,
            connected: false,
            busy: false,
            detail: None,
        }
    );
    // Still no script to call, even once linked and then unlinked.
    let script = app.link_terminal();
    app.state.config.lock().unwrap().payment_terminal = None;
    assert!(
        !do_check_payment_terminal(&app.state)
            .await
            .unwrap()
            .configured
    );
    assert!(script.calls().is_empty());
}

#[tokio::test]
async fn a_reachable_terminal_is_connected() {
    let (app, script) = app().await;
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert_eq!(
        status,
        PaymentTerminalStatus {
            configured: true,
            kind: Some("ccv_opi".into()),
            connected: true,
            busy: false,
            detail: None,
        }
    );
    assert_eq!(script.calls(), vec![Call::Check]);
    // Nothing was stored, nothing charged.
    assert!(app.eft_rows().is_empty());
}

#[tokio::test]
async fn an_unreachable_terminal_says_why() {
    let (app, script) = app().await;
    script.on_check(Err(EftError::Unreachable("connection refused".into())));
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert!(status.configured && !status.connected && !status.busy);
    assert_eq!(status.detail.as_deref(), Some("connection refused"));
    // The next probe finds it again.
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert!(status.connected && status.detail.is_none());
}

#[tokio::test]
async fn a_terminal_busy_elsewhere_still_counts_as_there() {
    let (app, script) = app().await;
    script.on_check(Err(EftError::Busy));
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert!(status.connected && status.busy);
    assert!(status.detail.is_none());
}

#[tokio::test]
async fn the_check_works_before_login_and_in_training() {
    let (app, script) = app().await;
    app.state.config.lock().unwrap().training_mode = true;
    *app.state.session.lock().unwrap() = None;
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert!(status.connected);
    assert_eq!(script.calls(), vec![Call::Check]);
}

#[tokio::test]
async fn no_probe_while_a_payment_runs() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.set_pay_delay(std::time::Duration::from_millis(400));
    script.on_pay(Ok(approval(1500, "Bancontact", CardType::Debit)));

    let pay = do_eft_pay(&app.state, order, 1500);
    let check = async {
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if app
                .eft_rows()
                .first()
                .is_some_and(|r| r.status.as_str() == "sent")
            {
                break;
            }
        }
        // Past the stored row the guard is taken within a moment.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        do_check_payment_terminal(&app.state).await.unwrap()
    };
    let (payment, status) = tokio::join!(pay, check);
    assert_eq!(payment.unwrap().status, "approved");
    assert!(status.configured && status.connected && status.busy);
    assert!(status.detail.is_none());
    let calls = script.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(matches!(calls[0], Call::Pay { .. }));
    // The line is free again: the next probe goes through.
    let status = do_check_payment_terminal(&app.state).await.unwrap();
    assert!(status.connected && !status.busy);
    assert_eq!(script.calls().last(), Some(&Call::Check));
}

#[tokio::test]
async fn the_probe_guard_does_not_block_an_abort() {
    let (app, script) = app().await;
    let order = app.order_of(1500);
    script.set_pay_delay(std::time::Duration::from_millis(400));
    script.on_pay(Ok(Outcome::Cancelled));

    let pay = do_eft_pay(&app.state, order, 1500);
    let abort = async {
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if app
                .eft_rows()
                .first()
                .is_some_and(|r| r.status.as_str() == "sent")
            {
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        // The line is held by the payment: the abort answers anyway.
        assert!(app.state.eft_line.try_lock().is_err());
        tokio::time::timeout(
            std::time::Duration::from_millis(300),
            do_eft_abort(&app.state),
        )
        .await
        .expect("the abort must not wait for the line")
    };
    let (payment, aborted) = tokio::join!(pay, abort);
    aborted.unwrap();
    assert_eq!(payment.unwrap().status, "cancelled");
    assert!(app.state.eft_line.try_lock().is_ok());
}

/// A new product at `price_cents` added to `order`; the id of its line.
fn add_line(app: &TestApp, order: i64, price_cents: i64) -> i64 {
    let item = {
        let db = app.state.db.lock().unwrap();
        db.execute(
            "INSERT INTO menu_items (category_id, price_cents, vat_code, sort_order, active)
             VALUES (1, ?1, 'B', 0, 1)",
            [price_cents],
        )
        .unwrap();
        db.last_insert_rowid()
    };
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: item,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap()
    .id
}

fn total_of(app: &TestApp, order: i64) -> i64 {
    pos_engine::commands::order::do_get_order(&app.state, order)
        .unwrap()
        .total_cents
}

fn first_line_of(app: &TestApp, order: i64) -> i64 {
    pos_engine::commands::order::do_list_order_lines(&app.state, order).unwrap()[0].id
}

#[tokio::test]
async fn a_line_cannot_be_removed_under_what_the_card_was_charged() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    let big = first_line_of(&app, order);
    let small = add_line(&app, order, 400);
    approved(&app, &script, order, 1000, "Bancontact", CardType::Debit).await;

    // 1400 - 1000 = 400 < 1000: refused, nothing changed.
    assert_eq!(
        do_remove_order_line(&app.state, big).unwrap_err(),
        "eft_below_charged|amount=10,00"
    );
    assert_eq!(total_of(&app, order), 1400);
    assert_eq!(
        pos_engine::commands::order::do_list_order_lines(&app.state, order)
            .unwrap()
            .len(),
        2
    );
    // 1400 - 400 = 1000: the card is still covered to the cent.
    do_remove_order_line(&app.state, small).unwrap();
    assert_eq!(total_of(&app, order), 1000);
    let payment = &do_list_eft_payments(&app.state, order).unwrap()[0];
    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(1000, payment.id)],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    assert!(app.eft_rows().iter().all(|r| r.used_event_id.is_some()));
}

#[tokio::test]
async fn a_quantity_cannot_be_lowered_under_what_the_card_was_charged() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    let line = first_line_of(&app, order);
    // More is always fine.
    do_update_order_line_quantity(&app.state, line, 2).unwrap();
    approved(&app, &script, order, 1500, "Visa", CardType::Credit).await;

    assert_eq!(
        do_update_order_line_quantity(&app.state, line, 1).unwrap_err(),
        "eft_below_charged|amount=15,00"
    );
    assert_eq!(total_of(&app, order), 2000);
    // Adding lines is always fine, and lowering is while 1500 is covered.
    let extra = add_line(&app, order, 600);
    do_update_order_line_quantity(&app.state, line, 1).unwrap();
    assert_eq!(total_of(&app, order), 1600);
    do_remove_order_line(&app.state, extra).unwrap_err();
    assert_eq!(total_of(&app, order), 1600);
}

#[tokio::test]
async fn the_order_is_frozen_while_a_card_payment_waits_for_its_answer() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    let line = first_line_of(&app, order);
    let extra = add_line(&app, order, 400);
    script.on_pay(Ok(Outcome::Unknown {
        detail: "no answer within 180 s".into(),
    }));
    let payment = do_eft_pay(&app.state, order, 500).await.unwrap();
    assert_eq!(payment.status, "unknown");

    for result in [
        do_remove_order_line(&app.state, extra),
        do_update_order_line_quantity(&app.state, line, 3),
    ] {
        assert_eq!(result.unwrap_err(), errors::EFT_EDIT_PENDING);
    }
    assert_eq!(total_of(&app, order), 1400);
    // Once the terminal answered, the amount is final: the edit is
    // judged against it.
    script.on_status(Ok(approval(500, "Bancontact", CardType::Debit)));
    assert_eq!(
        do_eft_check(&app.state, payment.id).await.unwrap().status,
        "approved"
    );
    do_remove_order_line(&app.state, extra).unwrap();
    assert_eq!(total_of(&app, order), 1000);
}

#[tokio::test]
async fn an_order_without_a_card_payment_is_edited_freely() {
    let (app, script) = app().await;
    let order = app.order_of(1000);
    let line = first_line_of(&app, order);
    let extra = add_line(&app, order, 400);
    do_remove_order_line(&app.state, extra).unwrap();
    do_update_order_line_quantity(&app.state, line, 2).unwrap();
    // A declined card charged nothing.
    script.on_pay(Ok(Outcome::Declined {
        reason: "refused".into(),
    }));
    assert_eq!(
        do_eft_pay(&app.state, order, 2000).await.unwrap().status,
        "declined"
    );
    do_update_order_line_quantity(&app.state, line, 1).unwrap();
    do_remove_order_line(&app.state, line).unwrap();
    assert_eq!(total_of(&app, order), 0);
}

#[tokio::test]
async fn a_charged_card_is_listed_until_a_sale_settles_it() {
    let (app, script) = app().await;
    assert!(
        do_list_unsettled_card_payments(&app.state)
            .unwrap()
            .is_empty()
    );
    let order = app.order_of(1000);
    // Nothing charged yet: a declined card is not money taken.
    script.on_pay(Ok(Outcome::Declined {
        reason: "refused".into(),
    }));
    do_eft_pay(&app.state, order, 1000).await.unwrap();
    assert!(
        do_list_unsettled_card_payments(&app.state)
            .unwrap()
            .is_empty()
    );

    let payment = approved(&app, &script, order, 600, "Bancontact", CardType::Debit).await;
    let other = app.order_of(500);
    let listed = do_list_unsettled_card_payments(&app.state).unwrap();
    assert_eq!(listed.len(), 1);
    let one = &listed[0];
    assert_eq!(one.payment_id, payment.id);
    assert_eq!(one.order_id, order);
    assert_eq!(one.order_type, "takeaway");
    assert_eq!(one.table_number, None);
    assert_eq!(one.amount_cents, 600);
    assert_eq!(one.brand.as_deref(), Some("Bancontact"));
    assert!(!one.created_at.is_empty());

    // Another order's sale does not settle it; its own sale does.
    assert!(
        do_checkout_split(&app.state, other, &[PaymentPart::cash()], None)
            .await
            .unwrap()
            .success
    );
    assert_eq!(
        do_list_unsettled_card_payments(&app.state).unwrap().len(),
        1
    );
    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(600, payment.id), PaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    assert!(
        do_list_unsettled_card_payments(&app.state)
            .unwrap()
            .is_empty()
    );
}
