//! A card payment through the real terminal drivers (`pos-eft`)
//! against the terminal mock (`pos-eft-mock`, served in-process), from
//! the terminal call to the signed sale, for each protocol: Worldline
//! Terminal API, CCV Payment API, OPI on the LAN. The FDM is the
//! admission image's mock, which signs for real.
//!
//! The other EFT suites script the terminal (`eft_payments`,
//! `eft_signing`); this one checks that what the drivers read from the
//! wire reaches the signed payment line.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;
use pos_core::config::{PaymentTerminalConfig, PaymentTerminalKind};
use pos_eft_mock::{Behaviour, Mock, Next};
use pos_engine::commands::checkout::{PaymentPart, do_checkout_split};
use pos_engine::commands::eft::{do_eft_check, do_eft_pay};
use serde_json::Value;

const MANAGER: &str = "12345678958";

fn config(kind: PaymentTerminalKind, mock: &Mock, device_port: u16) -> PaymentTerminalConfig {
    let cloud = kind != PaymentTerminalKind::CcvOpi;
    PaymentTerminalConfig {
        kind,
        base_url: cloud.then(|| mock.url()),
        api_key_file: None,
        merchant_id: Some("UMID-1".into()),
        terminal_id: Some("UTID-1".into()),
        integrator_id: Some("pili-tests".into()),
        management_system_id: Some("08000000002".into()),
        access_protocol: None,
        address: (!cloud).then(|| mock.opi_addr().to_string()),
        device_port: (!cloud).then_some(device_port),
        timeout_seconds: 30,
    }
}

/// A free local port for the OPI channel 1.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn app_with(kind: PaymentTerminalKind) -> (TestApp, Mock) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let mock = Mock::start();
    let port = free_port();
    mock.set_device_port(port);
    mock.set_default(Next {
        delay_ms: 50,
        ..Next::default()
    });
    app.state.config.lock().unwrap().payment_terminal = Some(config(kind, &mock, port));
    (app, mock)
}

fn card_line(app: &TestApp, event_id: i64) -> Value {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .expect("the event");
    let request: Value = serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap();
    let lines = request["financials"].as_array().unwrap().clone();
    assert_eq!(lines.len(), 1, "one card line, no rounding: {lines:?}");
    lines[0].clone()
}

async fn sells_through(kind: PaymentTerminalKind) {
    let (app, mock) = app_with(kind).await;
    let order = app.order_of(1870);
    let payment = do_eft_pay(&app.state, order, 1870).await.unwrap();
    assert_eq!(payment.status, "approved", "{kind:?}: {:?}", payment.detail);
    assert_eq!(payment.brand.as_deref(), Some("Bancontact"), "{kind:?}");
    assert_eq!(payment.card_type.as_deref(), Some("debit"), "{kind:?}");
    assert_eq!(mock.payments().len(), 1, "{kind:?}: one charge");

    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(1870, payment.id)],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{kind:?}: {:?}", sale.error);
    let line = card_line(&app, sale.fiscal_event_id.unwrap());
    assert_eq!(line["type"], "CARD_DEBIT", "{kind:?}");
    assert_eq!(line["inputMethod"], "AUTOMATIC", "{kind:?}");
    assert_eq!(line["provider"], "Bancontact", "{kind:?}");
    assert_eq!(line["amount"], 18.7, "{kind:?}");
    assert_eq!(app.order_status(order).as_deref(), Some("closed"));
}

#[tokio::test]
async fn a_worldline_terminal_payment_is_signed() {
    sells_through(PaymentTerminalKind::WorldlineTerminalApi).await;
}

#[tokio::test]
async fn a_ccv_cloud_terminal_payment_is_signed() {
    sells_through(PaymentTerminalKind::CcvPaymentApi).await;
}

#[tokio::test]
async fn an_opi_terminal_payment_is_signed() {
    sells_through(PaymentTerminalKind::CcvOpi).await;
}

#[tokio::test]
async fn a_visa_refusal_and_a_lost_answer_never_sign_a_card() {
    let (app, mock) = app_with(PaymentTerminalKind::WorldlineTerminalApi).await;
    let order = app.order_of(990);
    mock.next(Next {
        result: Behaviour::Decline,
        brand: "Visa".into(),
        delay_ms: 50,
        approved_cents: None,
    });
    let declined = do_eft_pay(&app.state, order, 990).await.unwrap();
    assert_eq!(declined.status, "declined");
    assert!(app.fiscal_events().is_empty(), "nothing signed");

    // The answer is lost: unknown, then the terminal is asked again.
    mock.next(Next {
        result: Behaviour::Timeout,
        brand: "Visa".into(),
        delay_ms: 50,
        approved_cents: None,
    });
    let lost = do_eft_pay(&app.state, order, 990).await.unwrap();
    assert_eq!(lost.status, "unknown", "{:?}", lost.detail);
    let checked = do_eft_check(&app.state, lost.id).await.unwrap();
    assert_eq!(checked.status, "approved", "{:?}", checked.detail);
    // The mock's receipt says "VISA CREDIT": the type comes from the
    // terminal, not from the brand alone.
    assert_eq!(checked.card_type.as_deref(), Some("credit"));
    let sale = do_checkout_split(
        &app.state,
        order,
        &[PaymentPart::terminal(990, checked.id)],
        None,
    )
    .await
    .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let line = card_line(&app, sale.fiscal_event_id.unwrap());
    assert_eq!(line["type"], "CARD_CREDIT");
    assert_eq!(line["id"], "6");
    assert_eq!(line["provider"], "Visa");
}
