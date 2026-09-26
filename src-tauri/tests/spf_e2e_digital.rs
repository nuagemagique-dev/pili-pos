//! Ticket delivery end to end: paper, email, or both (cert manual
//! §III.H), through the real checkout, refund and copy commands.
//!
//! The FDM is the official fixture replay, the printer a TCP sink that
//! records every job, and Scaleway TEM a wiremock server. The contract
//! proved here:
//!
//! - the medium is part of the signed request (`ticketMedium`);
//! - a refused choice (no email setup, bad address) stops before
//!   anything is signed;
//! - a DIGITAL ticket is only offered when allowed and the service is
//!   up, and falls back to printing the original when the email fails,
//!   so the customer always leaves with a ticket;
//! - the email carries the same text as the paper ticket plus the
//!   verification URL (POS spec v2.2 §6.5);
//! - an original is printed once (§8.1): a second print is refused.

mod common;

use common::*;

use moyun_pos::commands::errors;

use moyun_pos::commands::checkout::{CheckoutPaymentMethod, CheckoutResult, do_checkout_with};
use moyun_pos::commands::copy::do_copy_ticket_with;
use moyun_pos::commands::delivery::{
    self, ERR_ALREADY_PRINTED, ERR_DIGITAL_UNAVAILABLE, ERR_EMAIL_SENT, ERR_INVALID_EMAIL,
    EmailStatus, RequestedMedium, TicketDeliveryRequest,
};
use moyun_pos::commands::refund::{RefundItem, do_refund_checkout_with};
use pos_core::order::OrderType;
use pos_core::vat::VatCode;
use pos_db::queries::orders::NewOrderLine;
use pos_fiscal::types::{EventLabel, TicketMedium};
use wiremock::MockServer;

const TEST_INSZ: &str = "12345678901";
const CUSTOMER: &str = "client@example.be";

fn seed_open_order(app: &TestApp) -> i64 {
    let conn = app.state.db.lock().unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO users (insz, name, role, active) VALUES (?1, 'Anna', 'cashier', 1)",
        rusqlite::params![TEST_INSZ],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
         VALUES (1, 1, 1000, 'B', 0, 1)",
        [],
    )
    .unwrap();
    let order = pos_db::queries::orders::create_order(&conn, OrderType::DineIn, Some(1), TEST_INSZ)
        .unwrap();
    pos_db::queries::orders::add_order_line(
        &conn,
        NewOrderLine {
            order_id: order.id,
            menu_item_id: 1,
            quantity: 1,
            unit_price_cents: 1000,
            vat_code: VatCode::B,
            name: "Burger",
            modifiers_json: None,
            department_id: "1",
            department_name: "Cuisine",
        },
    )
    .unwrap();
    order.id
}

fn email(medium: RequestedMedium) -> TicketDeliveryRequest {
    TicketDeliveryRequest::email(medium, CUSTOMER)
}

/// An app with a printer sink and the M110 sale fixture mounted.
async fn app_with_printer() -> (TestApp, PrinterSink) {
    let app = TestApp::new().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    (app, printer)
}

/// Same, with email enabled through `tem`.
async fn app_with_mail(tem: &MockServer, allow_digital_only: bool) -> (TestApp, PrinterSink) {
    let (mut app, printer) = app_with_printer().await;
    app.enable_mail(tem, allow_digital_only);
    (app, printer)
}

async fn sell(app: &TestApp, request: Option<&TicketDeliveryRequest>) -> CheckoutResult {
    let order_id = seed_open_order(app);
    mock_from_fixture(app, &load_uc(&basic_set_root(), "M110")).await;
    do_checkout_with(&app.state, order_id, CheckoutPaymentMethod::Card, request)
        .await
        .expect("checkout returns a result")
}

// ── Paper ──────────────────────────────────────────────────────

#[tokio::test]
async fn paper_is_the_default_and_the_print_is_recorded() {
    let (app, printer) = app_with_printer().await;
    let result = sell(&app, None).await;
    assert!(result.success, "{:?}", result.error);

    let event_id = result.fiscal_event_id.expect("event id");
    let d = result.delivery.expect("delivery outcome");
    assert_eq!(d.medium, TicketMedium::Paper);
    assert_eq!(d.email, EmailStatus::NotRequested);
    assert!(d.paper_printed);
    assert!(!d.needs_paper);
    assert!(d.recipient_hint.is_none());

    assert_eq!(app.signed_ticket_medium(event_id), "PAPER");
    assert!(app.original_printed(event_id));
    assert!(app.deliveries().is_empty());
    assert_eq!(printer.settle(1).await.len(), 1);
}

#[tokio::test]
async fn an_explicit_paper_choice_needs_no_email_setup() {
    let (app, printer) = app_with_printer().await;
    let req = TicketDeliveryRequest {
        medium: RequestedMedium::Paper,
        email: Some("ignored@example.be".into()),
    };
    let result = sell(&app, Some(&req)).await;
    assert!(result.success, "{:?}", result.error);
    assert!(app.deliveries().is_empty());
    assert_eq!(printer.settle(1).await.len(), 1);
}

#[tokio::test]
async fn a_failed_print_asks_for_paper_and_records_nothing() {
    let app = TestApp::new().await;
    app.set_printer(DEAD_PRINTER);
    let result = sell(&app, None).await;
    assert!(
        result.success,
        "the sale is signed even if the printer is down"
    );

    let d = result.delivery.unwrap();
    assert!(!d.paper_printed);
    assert!(d.needs_paper, "the screen must offer to print the ticket");
    assert!(!app.original_printed(result.fiscal_event_id.unwrap()));
}

// ── Refusals before signing ────────────────────────────────────

#[tokio::test]
async fn email_without_setup_is_refused_before_signing() {
    let (app, printer) = app_with_printer().await;
    let order_id = seed_open_order(&app);
    for medium in [RequestedMedium::Digital, RequestedMedium::PaperDigital] {
        let result = do_checkout_with(
            &app.state,
            order_id,
            CheckoutPaymentMethod::Cash,
            Some(&email(medium)),
        )
        .await
        .unwrap();
        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some(ERR_DIGITAL_UNAVAILABLE));
    }
    assert_eq!(app.fdm.received_requests().await.unwrap().len(), 0);
    assert_eq!(app.counter_for(EventLabel::N), 0);
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    assert!(printer.settle(0).await.is_empty());
}

#[tokio::test]
async fn a_bad_address_is_refused_before_signing() {
    let tem = MockServer::start().await;
    let (app, _printer) = app_with_mail(&tem, true).await;
    let order_id = seed_open_order(&app);
    for bad in ["", "client", "client@", "@example.be", "a b@example.be"] {
        let req = TicketDeliveryRequest::email(RequestedMedium::PaperDigital, bad);
        let result = do_checkout_with(
            &app.state,
            order_id,
            CheckoutPaymentMethod::Cash,
            Some(&req),
        )
        .await
        .unwrap();
        assert!(!result.success, "{bad:?} must be refused");
        assert!(
            result
                .error
                .as_deref()
                .unwrap()
                .starts_with(ERR_INVALID_EMAIL),
            "{bad:?}: {:?}",
            result.error
        );
    }
    assert_eq!(app.fdm.received_requests().await.unwrap().len(), 0);
    assert_eq!(app.counter_for(EventLabel::N), 0);
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    assert!(tem_sent(&tem).await.is_empty());
}

// ── Paper + email ──────────────────────────────────────────────

#[tokio::test]
async fn paper_digital_prints_and_emails_the_same_ticket() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, false).await;

    let result = sell(&app, Some(&email(RequestedMedium::PaperDigital))).await;
    assert!(result.success, "{:?}", result.error);
    let event_id = result.fiscal_event_id.unwrap();
    let d = result.delivery.clone().unwrap();
    assert_eq!(d.medium, TicketMedium::PaperDigital);
    assert!(d.paper_printed);
    assert_eq!(d.email, EmailStatus::Sent);
    assert_eq!(
        d.recipient_hint.as_deref(),
        Some(pos_mail::address::mask(CUSTOMER).as_str())
    );
    assert_eq!(app.signed_ticket_medium(event_id), "PAPER_DIGITAL");
    assert_eq!(printer.settle(1).await.len(), 1);

    let sent = tem_sent(&tem).await;
    assert_eq!(sent.len(), 1);
    let mail = &sent[0];
    assert_eq!(mail["to"][0]["email"], CUSTOMER);
    assert_eq!(mail["project_id"], "project-1");
    let ticket = result.ticket.as_ref().unwrap();
    let paper_text = pos_mail::render::render_ticket_text(ticket);
    let text = mail["text"].as_str().unwrap();
    assert!(text.starts_with(&paper_text), "email body = paper ticket");
    let url = ticket.verification_url.as_deref().unwrap();
    assert!(text.contains(url), "the verification URL is in the text");
    assert!(
        mail["html"]
            .as_str()
            .unwrap()
            .contains(&format!("href=\"{url}\"")),
        "and a link in the HTML (§6.5)"
    );
    assert_eq!(mail["attachments"][0]["type"], "image/png");

    let rows = app.deliveries();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fiscal_event_id, event_id);
    assert_eq!(rows[0].medium, "PAPER_DIGITAL");
    assert_eq!(rows[0].status, "sent");
    assert_eq!(rows[0].provider_message_id.as_deref(), Some(TEM_MESSAGE_ID));
    assert!(
        rows[0].recipient.is_none(),
        "address cleared once sent (GDPR)"
    );
}

#[tokio::test]
async fn paper_digital_keeps_paper_and_retries_the_email_later() {
    let tem = MockServer::start().await;
    tem_fails(&tem, 503, 1).await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, false).await;

    let result = sell(&app, Some(&email(RequestedMedium::PaperDigital))).await;
    assert!(result.success);
    let d = result.delivery.unwrap();
    assert!(d.paper_printed);
    assert_eq!(d.email, EmailStatus::Queued);
    assert_eq!(printer.settle(1).await.len(), 1);

    let row = &app.deliveries()[0];
    assert_eq!(row.status, "pending");
    assert_eq!(row.attempts, 1);
    assert!(row.recipient.is_some(), "kept for the retry");

    // The retry is due (skip the backoff) and the worker sends it.
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE ticket_deliveries SET next_attempt_at = NULL", [])
        .unwrap();
    delivery::run_outbox_once(&app.state).await;
    let row = &app.deliveries()[0];
    assert_eq!(row.status, "sent");
    assert!(row.recipient.is_none());
    assert_eq!(tem_sent(&tem).await.len(), 2);
    assert_eq!(printer.settle(1).await.len(), 1, "no second paper ticket");
}

#[tokio::test]
async fn paper_digital_is_kept_while_the_email_service_is_down() {
    let tem = MockServer::start().await;
    let (app, _printer) = app_with_mail(&tem, true).await;
    // No probe recorded: the service isn't known to be up.
    let result = sell(&app, Some(&email(RequestedMedium::PaperDigital))).await;
    assert!(result.success);
    assert_eq!(
        app.signed_ticket_medium(result.fiscal_event_id.unwrap()),
        "PAPER_DIGITAL"
    );
}

// ── Email only ─────────────────────────────────────────────────

#[tokio::test]
async fn digital_only_emails_without_printing() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, true).await;
    app.state.mail.record_probe(true);

    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    assert!(result.success, "{:?}", result.error);
    let event_id = result.fiscal_event_id.unwrap();
    let d = result.delivery.unwrap();
    assert_eq!(d.medium, TicketMedium::Digital);
    assert_eq!(d.email, EmailStatus::Sent);
    assert!(!d.paper_printed);
    assert!(!d.needs_paper);
    assert_eq!(app.signed_ticket_medium(event_id), "DIGITAL");
    assert!(printer.settle(0).await.is_empty(), "nothing printed");
    assert!(!app.original_printed(event_id));
    assert_eq!(tem_sent(&tem).await.len(), 1);
}

#[tokio::test]
async fn digital_becomes_paper_digital_when_not_allowed() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, false).await;
    app.state.mail.record_probe(true);

    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    assert!(result.success);
    assert_eq!(
        app.signed_ticket_medium(result.fiscal_event_id.unwrap()),
        "PAPER_DIGITAL"
    );
    assert_eq!(printer.settle(1).await.len(), 1);
    assert_eq!(result.delivery.unwrap().email, EmailStatus::Sent);
}

#[tokio::test]
async fn digital_becomes_paper_digital_when_the_service_was_not_reachable() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, true).await;
    app.state.mail.record_probe(false);

    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    assert!(result.success);
    assert_eq!(
        app.signed_ticket_medium(result.fiscal_event_id.unwrap()),
        "PAPER_DIGITAL"
    );
    assert_eq!(printer.settle(1).await.len(), 1);
}

#[tokio::test]
async fn a_failed_digital_email_prints_the_original_once() {
    let tem = MockServer::start().await;
    tem_fails(&tem, 503, 10).await;
    let (app, printer) = app_with_mail(&tem, true).await;
    app.state.mail.record_probe(true);

    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    assert!(result.success);
    let event_id = result.fiscal_event_id.unwrap();
    let d = result.delivery.unwrap();
    assert_eq!(d.medium, TicketMedium::Digital, "signed as DIGITAL");
    assert!(d.paper_printed, "the customer gets the original on paper");
    assert!(!d.needs_paper);
    assert_eq!(
        d.email,
        EmailStatus::NotRequested,
        "paper replaces the email"
    );
    assert_eq!(printer.settle(1).await.len(), 1);
    assert!(app.original_printed(event_id));

    let row = &app.deliveries()[0];
    assert_eq!(row.status, "fallback_printed");
    assert!(row.fallback_printed);
    assert!(row.recipient.is_none(), "address cleared, no more retries");

    // The worker never emails a ticket that went out on paper.
    delivery::run_outbox_once(&app.state).await;
    assert_eq!(tem_sent(&tem).await.len(), 1);
    assert_eq!(app.deliveries()[0].status, "fallback_printed");
}

#[tokio::test]
async fn a_rejected_digital_email_also_falls_back_to_paper() {
    let tem = MockServer::start().await;
    tem_fails(&tem, 400, 10).await;
    let (app, printer) = app_with_mail(&tem, true).await;
    app.state.mail.record_probe(true);

    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    assert!(result.success);
    assert!(result.delivery.unwrap().paper_printed);
    assert_eq!(printer.settle(1).await.len(), 1);
}

#[tokio::test]
async fn digital_with_email_and_printer_down_keeps_trying_the_email() {
    let tem = MockServer::start().await;
    tem_fails(&tem, 503, 1).await;
    tem_accepts(&tem).await;
    let (app, _printer) = app_with_mail(&tem, true).await;
    app.set_printer(DEAD_PRINTER);
    app.state.mail.record_probe(true);

    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    assert!(result.success);
    let d = result.delivery.unwrap();
    assert!(!d.paper_printed);
    assert!(d.needs_paper, "the screen offers to print");
    assert_eq!(d.email, EmailStatus::Queued);
    assert_eq!(app.deliveries()[0].status, "pending", "not given up");

    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE ticket_deliveries SET next_attempt_at = NULL", [])
        .unwrap();
    delivery::run_outbox_once(&app.state).await;
    assert_eq!(app.deliveries()[0].status, "sent");
}

// ── Printing a missing original ────────────────────────────────

#[tokio::test]
async fn a_printed_original_can_not_be_printed_again() {
    let (app, printer) = app_with_printer().await;
    let result = sell(&app, None).await;
    let event_id = result.fiscal_event_id.unwrap();
    let ticket = result.ticket.unwrap();
    assert_eq!(printer.settle(1).await.len(), 1);

    let err = delivery::do_print_missing_original(&app.state, event_id, &ticket)
        .await
        .unwrap_err();
    assert_eq!(err, ERR_ALREADY_PRINTED);
    assert_eq!(printer.settle(1).await.len(), 1, "no duplicate original");
}

#[tokio::test]
async fn an_unprinted_original_prints_once() {
    let app = TestApp::new().await;
    app.set_printer(DEAD_PRINTER);
    let result = sell(&app, None).await;
    let event_id = result.fiscal_event_id.unwrap();
    let ticket = result.ticket.unwrap();

    // Still down: an error, and the original stays unprinted.
    assert!(
        delivery::do_print_missing_original(&app.state, event_id, &ticket)
            .await
            .unwrap_err()
            .starts_with("print_failed|detail=")
    );
    assert!(!app.original_printed(event_id));

    // Back up: prints once, then refuses.
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    delivery::do_print_missing_original(&app.state, event_id, &ticket)
        .await
        .unwrap();
    assert!(app.original_printed(event_id));
    assert_eq!(
        delivery::do_print_missing_original(&app.state, event_id, &ticket)
            .await
            .unwrap_err(),
        ERR_ALREADY_PRINTED
    );
    assert_eq!(printer.settle(1).await.len(), 1);
}

#[tokio::test]
async fn printing_the_missing_original_of_a_digital_ticket_stops_its_email() {
    let tem = MockServer::start().await;
    tem_fails(&tem, 503, 1).await;
    tem_accepts(&tem).await;
    let (app, _printer) = app_with_mail(&tem, true).await;
    app.set_printer(DEAD_PRINTER);
    app.state.mail.record_probe(true);
    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    let event_id = result.fiscal_event_id.unwrap();

    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    delivery::do_print_missing_original(&app.state, event_id, result.ticket.as_ref().unwrap())
        .await
        .unwrap();
    let row = &app.deliveries()[0];
    assert_eq!(row.status, "fallback_printed");
    assert!(row.recipient.is_none());

    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE ticket_deliveries SET next_attempt_at = NULL", [])
        .unwrap_or(0);
    delivery::run_outbox_once(&app.state).await;
    assert_eq!(tem_sent(&tem).await.len(), 1, "no email after the paper");
}

#[tokio::test]
async fn an_emailed_digital_ticket_can_not_be_printed_as_an_original() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, true).await;
    app.state.mail.record_probe(true);
    let result = sell(&app, Some(&email(RequestedMedium::Digital))).await;
    let event_id = result.fiscal_event_id.unwrap();

    let err =
        delivery::do_print_missing_original(&app.state, event_id, result.ticket.as_ref().unwrap())
            .await
            .unwrap_err();
    assert_eq!(err, ERR_EMAIL_SENT);
    assert!(printer.settle(0).await.is_empty());
}

#[tokio::test]
async fn printing_an_unknown_event_is_refused() {
    let (app, _printer) = app_with_printer().await;
    let result = sell(&app, None).await;
    let err =
        delivery::do_print_missing_original(&app.state, 9999, result.ticket.as_ref().unwrap())
            .await
            .unwrap_err();
    assert_eq!(err, errors::EVENT_NOT_FOUND);
}

// ── Refunds, copies, training ──────────────────────────────────

#[tokio::test]
async fn a_refund_ticket_can_be_emailed() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, false).await;
    let sale = sell(&app, None).await;
    let original = sale.fiscal_event_id.unwrap();

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M111")).await;
    let refund = do_refund_checkout_with(
        &app.state,
        original,
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
        Some(&email(RequestedMedium::PaperDigital)),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let refund_id = refund.fiscal_event_id.unwrap();
    assert_ne!(refund_id, original);
    assert_eq!(app.signed_ticket_medium(refund_id), "PAPER_DIGITAL");
    let d = refund.delivery.unwrap();
    assert!(d.paper_printed);
    assert_eq!(d.email, EmailStatus::Sent);
    assert_eq!(printer.settle(2).await.len(), 2, "sale + refund");
    let rows = app.deliveries();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fiscal_event_id, refund_id);
}

#[tokio::test]
async fn a_refund_with_a_bad_address_is_refused_before_signing() {
    let tem = MockServer::start().await;
    let (app, _printer) = app_with_mail(&tem, false).await;
    let sale = sell(&app, None).await;
    let fdm_calls = app.fdm.received_requests().await.unwrap().len();
    let refund = do_refund_checkout_with(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        vec![RefundItem {
            menu_item_id: 1,
            quantity: 1,
        }],
        TEST_INSZ.to_string(),
        Some(&TicketDeliveryRequest::email(
            RequestedMedium::PaperDigital,
            "nope",
        )),
    )
    .await
    .unwrap();
    assert!(!refund.success);
    assert_eq!(app.fdm.received_requests().await.unwrap().len(), fdm_calls);
    assert_eq!(app.fiscal_events().len(), 1, "only the sale");
}

#[tokio::test]
async fn a_copy_can_be_emailed_only() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, true).await;
    app.state.mail.record_probe(true);
    let sale = sell(&app, None).await;

    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M160")).await;
    let copy = do_copy_ticket_with(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        TEST_INSZ.to_string(),
        Some(&email(RequestedMedium::Digital)),
    )
    .await
    .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let copy_id = copy.fiscal_event_id.unwrap();
    assert_eq!(app.signed_ticket_medium(copy_id), "DIGITAL");
    assert!(copy.ticket.is_some());
    let d = copy.delivery.unwrap();
    assert_eq!(d.email, EmailStatus::Sent);
    assert!(!d.paper_printed);
    assert_eq!(printer.settle(1).await.len(), 1, "only the sale printed");
    assert_eq!(app.deliveries()[0].fiscal_event_id, copy_id);
    // The FDM returns no verification URL for a copy: the copy uses
    // the original sale's QR code (POS description v2.2 §6.5).
    let original_url = sale.ticket.as_ref().unwrap().verification_url.clone();
    assert!(original_url.is_some());
    assert_eq!(copy.ticket.as_ref().unwrap().verification_url, original_url);
    let sent = tem_sent(&tem).await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["attachments"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn a_paper_copy_is_printed_and_recorded() {
    let (app, printer) = app_with_printer().await;
    let sale = sell(&app, None).await;
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M160")).await;
    let copy = do_copy_ticket_with(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        TEST_INSZ.to_string(),
        None,
    )
    .await
    .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let copy_id = copy.fiscal_event_id.unwrap();
    assert_eq!(app.signed_ticket_medium(copy_id), "PAPER");
    assert!(copy.delivery.unwrap().paper_printed);
    assert!(app.original_printed(copy_id));
    assert_eq!(printer.settle(2).await.len(), 2);
}

#[tokio::test]
async fn a_training_ticket_email_keeps_its_training_marks() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, _printer) = app_with_mail(&tem, false).await;
    app.state.config.lock().unwrap().training_mode = true;
    let order_id = seed_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M171_")).await;
    let result = do_checkout_with(
        &app.state,
        order_id,
        CheckoutPaymentMethod::Cash,
        Some(&email(RequestedMedium::PaperDigital)),
    )
    .await
    .unwrap();
    assert!(result.success, "{:?}", result.error);
    let sent = tem_sent(&tem).await;
    assert_eq!(sent.len(), 1);
    assert!(sent[0]["text"].as_str().unwrap().contains("TRAINING"));
}

// ── Atomicity ──────────────────────────────────────────────────

#[tokio::test]
async fn a_failed_outbox_write_rolls_the_whole_sale_back() {
    let tem = MockServer::start().await;
    tem_accepts(&tem).await;
    let (app, printer) = app_with_mail(&tem, false).await;
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER poison_outbox BEFORE INSERT ON ticket_deliveries
             BEGIN SELECT RAISE(ABORT, 'disk full (synthetic)'); END;",
        )
        .unwrap();

    let order_id = seed_open_order(&app);
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let err = do_checkout_with(
        &app.state,
        order_id,
        CheckoutPaymentMethod::Cash,
        Some(&email(RequestedMedium::PaperDigital)),
    )
    .await
    .expect_err("the commit fails");
    assert!(err.contains("disk full"), "{err}");
    assert!(app.fiscal_events().is_empty(), "nothing archived");
    assert_eq!(app.counter_for(EventLabel::N), 0, "no gap");
    assert_eq!(app.order_status(order_id).as_deref(), Some("open"));
    assert!(printer.settle(0).await.is_empty());
    assert!(tem_sent(&tem).await.is_empty());

    // Once the fault is gone the retry signs the same ticket number.
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch("DROP TRIGGER poison_outbox;")
        .unwrap();
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M110")).await;
    let result = do_checkout_with(
        &app.state,
        order_id,
        CheckoutPaymentMethod::Cash,
        Some(&email(RequestedMedium::PaperDigital)),
    )
    .await
    .unwrap();
    assert!(result.success);
    assert_eq!(app.counter_for(EventLabel::N), 1);
    assert_eq!(app.deliveries().len(), 1);
}

// ── Mail status and listing ────────────────────────────────────

#[tokio::test]
async fn mail_status_reports_an_unconfigured_terminal() {
    let app = TestApp::new().await;
    let status = delivery::do_check_mail_status(&app.state).await.unwrap();
    assert!(!status.configured);
    assert!(!status.reachable);
    assert!(!status.allow_digital_only);
}

#[tokio::test]
async fn mail_status_probes_the_service() {
    let tem = MockServer::start().await;
    tem_probe(&tem, 200).await;
    let (app, _printer) = app_with_mail(&tem, true).await;
    let status = delivery::do_check_mail_status(&app.state).await.unwrap();
    assert!(status.configured);
    assert!(status.reachable);
    assert!(status.allow_digital_only);
    assert!(app.state.mail.recently_reachable());

    // Cached: a second call doesn't probe again.
    delivery::do_check_mail_status(&app.state).await.unwrap();
    let probes = tem
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .filter(|r| r.method.as_str() == "GET")
        .count();
    assert_eq!(probes, 1);
}

#[tokio::test]
async fn a_refused_key_makes_the_service_unreachable() {
    let tem = MockServer::start().await;
    tem_probe(&tem, 401).await;
    let (app, _printer) = app_with_mail(&tem, true).await;
    let status = delivery::do_check_mail_status(&app.state).await.unwrap();
    assert!(status.configured);
    assert!(!status.reachable);
    assert!(!app.state.mail.recently_reachable());
}

#[tokio::test]
async fn the_delivery_list_never_shows_the_address() {
    let tem = MockServer::start().await;
    tem_fails(&tem, 503, 1).await;
    let (app, _printer) = app_with_mail(&tem, false).await;
    sell(&app, Some(&email(RequestedMedium::PaperDigital))).await;

    let list = delivery::do_list_ticket_deliveries(&app.state, None).unwrap();
    assert_eq!(list.len(), 1);
    let json = serde_json::to_string(&list).unwrap();
    assert!(!json.contains(CUSTOMER), "{json}");
    assert_eq!(list[0].recipient_hint, pos_mail::address::mask(CUSTOMER));
    assert_eq!(list[0].status, "pending");
}

#[tokio::test]
async fn the_outbox_does_nothing_without_an_email_client() {
    let (app, _printer) = app_with_printer().await;
    sell(&app, None).await;
    delivery::run_outbox_once(&app.state).await;
    assert!(app.deliveries().is_empty());
}
