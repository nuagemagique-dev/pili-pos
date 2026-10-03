//! What the copy and refund tickets show (SPF example tickets of
//! 2026-09-24, POS description v2.2 §1.8, §6.5, §6.6).
//!
//! - a copy reproduces the original ticket, every field and its QR
//!   code included, adds COPIE, the copy event's control data and when
//!   it was made, and moves the original's control data under
//!   "Informations FDM d'origine";
//! - a copy of a VAT ticket never says it is not a VAT ticket; a copy
//!   of any other event does;
//! - a refund ticket shows the control data of the sale it reverses,
//!   and says REMBOURSEMENT once, under the VAT ticket title;
//! - a VAT ticket shows the sale total, the cash paid for it and the
//!   rounding as its own payment line, and the issuer's legal name;
//! - a pre-bill says "ADDITION PROVISOIRE" and shows no VAT.
//!
//! Events are signed by the admission image's mock FDM.

mod common;

use common::*;

use pili_pos::commands::checkout::{CheckoutPaymentMethod, CheckoutResult, do_checkout};
use pili_pos::commands::copy::do_copy_ticket;
use pili_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pili_pos::commands::refund::{RefundItem, do_refund_checkout};
use pili_pos::commands::tables::do_sign_pre_bill;
use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_fiscal::ticket::FiscalTicket;
use pos_fiscal::types::EventLabel;

const MANAGER: &str = "12345678958";
const INVALID_MENTION: &str = "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE";

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
                (1, 1, 304, 'A', 0, 1);",
        )
        .unwrap();
    app
}

async fn order(app: &TestApp, table: Option<i32>, quantity: i32) -> i64 {
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
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: id,
            menu_item_id: 1,
            quantity,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )
    .unwrap();
    id
}

/// 2 x 3,04 paid cash: 6,08 rounded to 6,10.
async fn cash_sale(app: &TestApp) -> CheckoutResult {
    let id = order(app, None, 2).await;
    let r = do_checkout(&app.state, id, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r
}

fn printed(ticket: &FiscalTicket) -> String {
    pos_print::render::render_plain_text(&pos_print::receipt::format_receipt(ticket))
}

/// How the ticket prints an event's counters (SPF example tickets).
fn counters(r: &pos_fiscal::response::FdmReference) -> String {
    format!(
        "Event: {:?}; eventCounter: {};\ntotalCounter: {};",
        r.event_label, r.event_counter, r.total_counter
    )
}

#[tokio::test]
async fn vi_copy_m160_a_copy_reproduces_the_original_sale() {
    let app = app().await;
    let sale = cash_sale(&app).await;
    let original = sale.ticket.clone().unwrap();

    let copy = do_copy_ticket(&app.state, sale.fiscal_event_id.unwrap(), MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let ticket = copy.ticket.unwrap();

    // Every field of the original ticket.
    assert_eq!(ticket.ticket_number, original.ticket_number);
    assert_eq!(ticket.pos_datetime, original.pos_datetime);
    assert_eq!(ticket.operator, original.operator);
    assert_eq!(ticket.ticket_type_label, original.ticket_type_label);
    assert_eq!(ticket.event_label, EventLabel::N);
    assert_eq!(ticket.lines.len(), original.lines.len());
    assert_eq!(ticket.total, original.total);
    assert_eq!(ticket.rounding, Some(0.02));
    assert_eq!(ticket.payments.len(), 1);
    assert_eq!(ticket.payments[0].method, original.payments[0].method);
    assert_eq!(ticket.payments[0].amount, original.payments[0].amount);
    assert_eq!(ticket.vat_breakdown.len(), original.vat_breakdown.len());
    // The copy gets no URL of its own: the original's QR code (§6.5).
    assert!(original.verification_url.is_some());
    assert_eq!(ticket.verification_url, original.verification_url);

    // The copy's control data, and the original's apart.
    assert_eq!(ticket.fdm_ref.as_ref().unwrap().event_label, EventLabel::C);
    let from = ticket
        .original
        .as_ref()
        .expect("the original's control data");
    let sold = original.fdm_ref.as_ref().unwrap();
    assert_eq!(counters(&from.fdm_ref), counters(sold));
    assert_eq!(from.fdm_ref.fdm_id, sold.fdm_id);
    assert_eq!(from.short_signature, original.short_signature);
    assert!(ticket.copy_made_at.is_some());
}

#[tokio::test]
async fn vi_copy_the_printed_copy_of_a_sale_is_a_vat_ticket_marked_copie() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let sale = cash_sale(&app).await;
    let sold = sale.ticket.as_ref().unwrap().fdm_ref.clone().unwrap();

    let copy = do_copy_ticket(&app.state, sale.fiscal_event_id.unwrap(), MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let text = printed(copy.ticket.as_ref().unwrap());
    for expected in [
        "TICKET DE CAISSE T.V.A.",
        "COPIE",
        "Informations FDM d'origine :",
        "Copie etablie le",
        &counters(&sold),
        &format!("fdmId: {};", sold.fdm_id),
    ] {
        assert!(
            text.contains(expected),
            "{expected:?} missing from:\n{text}"
        );
    }
    assert!(!one_line(&text).contains(INVALID_MENTION), "{text}");
    // Paper and email show the same thing.
    let receipts = printer.settle(2).await;
    assert!(receipts[1].contains("COPIE"), "{}", receipts[1]);
}

#[tokio::test]
async fn vi_copy_m161_a_copy_of_a_pre_bill_is_not_a_vat_ticket() {
    let app = app().await;
    let id = order(&app, Some(3), 1).await;
    do_sign_pre_bill(&app.state, id).await.unwrap();
    let pre_bill = app.fiscal_events().pop().unwrap();
    assert_eq!(pre_bill.event_type, "P");

    let copy = do_copy_ticket(&app.state, pre_bill.id, MANAGER.into())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let text = printed(copy.ticket.as_ref().unwrap());
    for expected in [
        "PRO FORMA",
        "COPIE",
        "Informations FDM d'origine :",
        INVALID_MENTION,
    ] {
        assert!(
            one_line(&text).contains(expected),
            "{expected:?} missing from:\n{text}"
        );
    }
}

#[tokio::test]
async fn vi_refund_m111_the_refund_ticket_shows_the_sale_it_reverses() {
    let app = app().await;
    let sale = cash_sale(&app).await;
    let sold = sale.ticket.as_ref().unwrap().fdm_ref.clone().unwrap();

    let refund = do_refund_checkout(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 2,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let ticket = refund.ticket.unwrap();
    let from = ticket.original.as_ref().expect("the sale's control data");
    assert_eq!(counters(&from.fdm_ref), counters(&sold));
    assert_eq!(
        from.short_signature,
        sale.ticket.as_ref().unwrap().short_signature
    );
    assert!(ticket.copy_made_at.is_none());

    let text = printed(&ticket);
    for expected in [
        "Informations FDM d'origine :",
        &counters(&sold),
        &format!("fdmId: {};", sold.fdm_id),
    ] {
        assert!(
            text.contains(expected),
            "{expected:?} missing from:\n{text}"
        );
    }
    assert!(!text.contains("COPIE"), "{text}");
}

#[tokio::test]
async fn vi_n_a_sale_ticket_has_no_original_block() {
    let app = app().await;
    let sale = cash_sale(&app).await;
    let ticket = sale.ticket.unwrap();
    assert!(ticket.original.is_none());
    let text = printed(&ticket);
    assert!(!text.contains("Informations FDM d'origine"), "{text}");
    assert!(!text.contains("COPIE"), "{text}");
}

#[tokio::test]
async fn vi_n_uc201_the_ticket_shows_the_sale_total_and_the_rounding_apart() {
    let app = app().await;
    let sale = cash_sale(&app).await;
    let text = printed(sale.ticket.as_ref().unwrap());
    let row = |label: &str| {
        text.lines()
            .find(|l| l.starts_with(label))
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| panic!("no {label:?} row in:\n{text}"))
    };
    assert_eq!(row("TOTAL :"), "TOTAL : €6,08");
    assert!(text.contains("Mode de paiement :"), "{text}");
    assert_eq!(row("Especes"), "Especes 6,08");
    assert_eq!(row("Arrondi"), "Arrondi 0,02");
    // Quantity, name, line total, VAT code.
    let product = row("2 ");
    assert!(product.ends_with("6,08 A"), "{product}");
}

#[tokio::test]
async fn vi_n_the_issuer_legal_name_is_printed() {
    let app = app().await;
    app.state.config.lock().unwrap().restaurant_legal_name = Some("Exemple Horeca SRL".into());
    let sale = cash_sale(&app).await;
    let text = printed(sale.ticket.as_ref().unwrap());
    assert!(
        text.contains("Denomination legale : Exemple Horeca SRL"),
        "{text}"
    );
}

#[tokio::test]
async fn vi_refund_says_remboursement_once() {
    let app = app().await;
    let sale = cash_sale(&app).await;
    let refund = do_refund_checkout(
        &app.state,
        sale.fiscal_event_id.unwrap(),
        vec![RefundItem {
            line_id: None,
            menu_item_id: 1,
            quantity: 1,
        }],
        MANAGER.into(),
    )
    .await
    .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let text = printed(refund.ticket.as_ref().unwrap());
    assert_eq!(text.matches("REMBOURSEMENT").count(), 1, "{text}");
    assert!(text.contains("TICKET DE CAISSE T.V.A."), "{text}");
}

#[tokio::test]
async fn vi_proforma_m123_the_pre_bill_says_addition_provisoire_without_vat() {
    let app = app().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    let id = order(&app, Some(2), 1).await;
    do_sign_pre_bill(&app.state, id).await.unwrap();
    let receipts = printer.settle(1).await;
    let pre_bill = receipts
        .iter()
        .find(|r| r.contains("PRO FORMA"))
        .expect("the pre-bill");
    for expected in ["ADDITION PROVISOIRE", "TOTAL :", INVALID_MENTION] {
        assert!(
            one_line(pre_bill).contains(expected),
            "{expected:?} missing from:\n{pre_bill}"
        );
    }
    assert!(
        !pre_bill.contains("% TVA"),
        "no VAT on a pre-bill:\n{pre_bill}"
    );
    assert!(!pre_bill.contains("Mode de paiement"), "{pre_bill}");
}
