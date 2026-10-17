//! UC4121 (invoice of three sales) and UC4122 (invoice of a refund and two sales) of the
//! Advanced Set 1.6, replayed field by field against the published signInvoice requests.
//!
//! The sales of both use payment type CUSTOMER CREDIT (customer accounts are not part of
//! the product), so the sales here are cash sales; the invoice request does not depend on how the
//! sales were paid. The invoice is compared like every other fixture: business fields equal
//! (`costCenter.type`, `customerVatNo`, the labels of `fdmRefs`) and the identification fields
//! by format; the invoice number is checked for its year and sequence format, not its value.

use pos_engine_testkit as common;

use common::*;
use serde_json::Value;

use pos_engine::commands::invoice::{do_invoice_preview, do_sign_invoice_fiscal};
use pos_engine::commands::refund::{RefundItem, do_refund_checkout};
use pos_fiscal::types::EventLabel;

const OPERATOR: &str = "84022899837";

async fn app(invoice_uc: &str) -> (TestApp, Value, i64) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let fixture = load_uc(&advanced_set_root(), invoice_uc);
    let vars = read_variables(&fixture)["data"].clone();
    let customer = app.save_customer(
        vars["customerVatNo"].as_str().unwrap(),
        vars["costCenter"]["reference"].as_str().unwrap(),
    );
    (app, vars, customer)
}

/// Our archived I request against the fixture's.
fn assert_invoice_matches(uc: &str, app: &TestApp, fixture: &Value, sales: &[i64]) {
    let refs = sales.len();
    let events = app.fiscal_events();
    let ours: Value = serde_json::from_str(
        events
            .iter()
            .rev()
            .find(|e| e.event_type == "I")
            .unwrap()
            .event_data_json
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    let mut found = Vec::new();
    diff(
        "",
        &business_view(&ours),
        &business_view(fixture),
        &mut found,
    );
    assert!(
        found.is_empty(),
        "{uc}: request differs from the fixture: {found:#?}"
    );
    assert_eq!(ours["fdmRefs"].as_array().unwrap().len(), refs);
    // Each ref is the FDM's own answer for the matching sale, in order.
    app.assert_fdm_refs_of(&ours, sales);
    let no = ours["invoiceNo"].as_str().unwrap();
    assert_eq!(no.len(), 8, "{no}");
    assert!(no.chars().all(|c| c.is_ascii_digit()), "{no}");
    assert_eq!(ours["costCenter"]["type"], fixture["costCenter"]["type"]);
    assert_eq!(
        ours["costCenter"]["reference"],
        fixture["costCenter"]["reference"]
    );
    assert_eq!(ours["customerVatNo"], fixture["customerVatNo"]);
    app.dump_evidence(uc);
}

#[tokio::test]
async fn uc4121_an_invoice_of_three_sales() {
    let (app, fixture, customer) = app("UC4121CustomerInvoice/UC4121_4").await;
    assert_eq!(fixture["fdmRefs"].as_array().unwrap().len(), 3);
    // Carpaccio and wine, two dry Martini, two dame blanche.
    let sales = [
        app.sell_cash(8000).await,
        app.sell_cash(2400).await,
        app.sell_cash(2304).await,
    ];

    let preview = do_invoice_preview(&app.state, &sales).unwrap();
    assert!(preview.problems.is_empty(), "{:?}", preview.problems);
    assert_eq!(preview.total_cents, 8000 + 2400 + 2304);

    let result = do_sign_invoice_fiscal(&app.state, sales.to_vec(), customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(app.counter_for(EventLabel::I), 1);
    assert_invoice_matches("UC4121", &app, &fixture, &sales);
}

#[tokio::test]
async fn uc4122_an_invoice_of_a_refund_and_two_sales() {
    let (app, fixture, customer) = app("UC4122_Creditnote/UC4122_4").await;
    let labels: Vec<&str> = fixture["fdmRefs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["eventLabel"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["N", "N", "N"], "the refund is an N too");

    // The scenario precondition: the sold ticket (4 dame blanche) "had already been invoiced",
    // so the original sale is invoiced first, then refunded (the refund is a negative N), then
    // two other sales are made.
    let original = app.sell_cash(4608).await;
    let first_invoice =
        do_sign_invoice_fiscal(&app.state, vec![original], customer, OPERATOR.into())
            .await
            .unwrap();
    assert!(first_invoice.success, "{:?}", first_invoice.error);
    assert_eq!(app.counter_for(EventLabel::I), 1);
    // The precondition holds: the original sale is now on an invoice and cannot be invoiced again.
    let again = do_invoice_preview(&app.state, &[original]).unwrap();
    assert!(
        again
            .problems
            .iter()
            .any(|p| p.starts_with("invoice_sale_already_invoiced")),
        "the original sale must be reported as already invoiced: {:?}",
        again.problems
    );
    let item: i64 = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT menu_item_id FROM order_lines
             WHERE order_id = (SELECT order_id FROM fiscal_events WHERE id = ?1)",
            [original],
            |r| r.get(0),
        )
        .unwrap();
    let items = vec![RefundItem {
        line_id: None,
        menu_item_id: item,
        quantity: 1,
    }];
    let refund = do_refund_checkout(&app.state, original, items, OPERATOR.to_string())
        .await
        .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let refund_event = refund.fiscal_event_id.expect("refund event");
    let first = app.sell_cash(4800).await;
    let second = app.sell_cash(12000).await;

    // The invoice of the refund and the two sales: the refund counts negative.
    let ids = vec![refund_event, first, second];
    let invoiced = ids.clone();
    let preview = do_invoice_preview(&app.state, &ids).unwrap();
    assert!(preview.problems.is_empty(), "{:?}", preview.problems);
    assert!(preview.sales.iter().any(|s| s.is_refund));
    assert_eq!(preview.total_cents, -4608 + 4800 + 12000);

    let result = do_sign_invoice_fiscal(&app.state, ids, customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    // Two invoices in all: the one of the original sale, then this one (the last I event).
    assert_eq!(app.counter_for(EventLabel::I), 2);
    assert_invoice_matches("UC4122", &app, &fixture, &invoiced);
}
