//! The rules of B2B invoices (event I, M150, v1.1): one invoice per sale, no invoice in
//! training mode, only normal sales, a continuous numbering, a crash between the FDM's answer
//! and the commit, the Z report's `invoices[]` and the copy of an invoice. The FDM is the
//! admission mock (`pos-fdm-mock`), which signs like the real one.

use pos_engine_testkit as common;

use common::*;

use pos_engine::commands::copy::do_copy_ticket;
use pos_engine::commands::errors;
use pos_engine::commands::invoice::{
    B2bCustomerInput, do_invoice_preview, do_list_invoiceable_sales, do_list_invoices,
    do_save_b2b_customer, do_sign_invoice_fiscal,
};
use pos_engine::commands::report::do_generate_z_report;
use pos_engine::commands::tables::do_sign_table_order;
use pos_fiscal::types::EventLabel;

/// A valid INSZ (the default test operator has none).
const OPERATOR: &str = "75061189702";
const VAT: &str = "BE0308357159";

async fn app() -> (TestApp, i64) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let customer = app.save_customer(VAT, "Client Test SRL");
    (app, customer)
}

fn rows(app: &TestApp, sql: &str) -> i64 {
    app.state
        .db
        .lock()
        .unwrap()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}

/// An open dine-in order of one product, for a table order (event P).
fn dine_in_order(app: &TestApp) -> i64 {
    let db = app.state.db.lock().unwrap();
    db.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order, active) VALUES (1, 0, 1)",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO menu_items (category_id, price_cents, vat_code, sort_order, active)
         VALUES (1, 700, 'B', 0, 1)",
        [],
    )
    .unwrap();
    let item = db.last_insert_rowid();
    let order = pos_db::queries::orders::create_order(
        &db,
        pos_core::order::OrderType::DineIn,
        Some(1),
        OPERATOR,
    )
    .unwrap();
    pos_db::queries::orders::add_order_line(
        &db,
        pos_db::queries::orders::NewOrderLine {
            order_id: order.id,
            menu_item_id: item,
            quantity: 1,
            unit_price_cents: 700,
            vat_code: pos_core::vat::VatCode::B,
            name: "Test",
            modifiers_json: None,
            department_id: "1",
            department_name: "Test",
        },
    )
    .unwrap();
    order.id
}

#[tokio::test]
async fn a_sale_is_on_one_invoice_only() {
    let (app, customer) = app().await;
    let sale = app.sell_cash(1000).await;
    let other = app.sell_cash(500).await;

    let first = do_sign_invoice_fiscal(&app.state, vec![sale], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(first.success, "{:?}", first.error);

    // The same sale again, alone or with another one: refused, nothing signed.
    for ids in [vec![sale], vec![sale, other]] {
        let err = do_sign_invoice_fiscal(&app.state, ids, customer, OPERATOR.into())
            .await
            .unwrap_err();
        assert!(
            err.starts_with(errors::INVOICE_SALE_ALREADY_INVOICED),
            "{err}"
        );
        assert!(err.contains(first.invoice_no.as_deref().unwrap()), "{err}");
    }
    assert_eq!(app.counter_for(EventLabel::I), 1);

    // The preview says so too, and the list shows who invoiced the sale.
    let preview = do_invoice_preview(&app.state, &[sale]).unwrap();
    assert_eq!(preview.problems.len(), 1);
    let listed = do_list_invoiceable_sales(&app.state, None, None).unwrap();
    let by = |id: i64| {
        listed
            .iter()
            .find(|s| s.fiscal_event_id == id)
            .unwrap()
            .invoiced_by
            .clone()
    };
    assert_eq!(by(sale), first.invoice_no);
    assert_eq!(by(other), None);

    // The other sale can still be invoiced.
    let second = do_sign_invoice_fiscal(&app.state, vec![other], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(second.success, "{:?}", second.error);
}

#[tokio::test]
async fn no_invoice_in_training_mode() {
    let (app, customer) = app().await;
    let sale = app.sell_cash(1000).await;
    app.state.config.lock().unwrap().training_mode = true;

    let err = do_sign_invoice_fiscal(&app.state, vec![sale], customer, OPERATOR.into())
        .await
        .unwrap_err();
    assert_eq!(err, errors::INVOICE_TRAINING_MODE);
    let preview = do_invoice_preview(&app.state, &[sale]).unwrap();
    assert_eq!(preview.problems[0], errors::INVOICE_TRAINING_MODE);
    assert_eq!(app.counter_for(EventLabel::I), 0);
    assert_eq!(app.counter_for(EventLabel::T), 0);
}

#[tokio::test]
async fn only_normal_sales_can_be_invoiced() {
    let (app, customer) = app().await;
    let sale = app.sell_cash(1000).await;
    // A table order (event P) and a copy (event C) are not sales.
    let order = dine_in_order(&app);
    do_sign_table_order(&app.state, order).await.unwrap();
    let copy = do_copy_ticket(&app.state, sale, OPERATOR.to_string())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    let events = app.fiscal_events();
    let id_of = |t: &str| events.iter().find(|e| e.event_type == t).unwrap().id;

    for (id, label) in [(id_of("P"), "P"), (id_of("C"), "C")] {
        let err = do_sign_invoice_fiscal(&app.state, vec![id], customer, OPERATOR.into())
            .await
            .unwrap_err();
        assert!(
            err.starts_with(errors::INVOICE_NOT_A_SALE),
            "{label}: {err}"
        );
    }
    let err = do_sign_invoice_fiscal(&app.state, vec![9999], customer, OPERATOR.into())
        .await
        .unwrap_err();
    assert!(err.starts_with(errors::INVOICE_SALE_NOT_FOUND), "{err}");
    let err = do_sign_invoice_fiscal(&app.state, vec![], customer, OPERATOR.into())
        .await
        .unwrap_err();
    assert_eq!(err, errors::INVOICE_NO_SALES);
    let err = do_sign_invoice_fiscal(&app.state, vec![sale], 12345, OPERATOR.into())
        .await
        .unwrap_err();
    assert_eq!(err, errors::CUSTOMER_NOT_FOUND);
    assert_eq!(app.counter_for(EventLabel::I), 0);
}

#[tokio::test]
async fn the_numbering_is_continuous_across_an_fdm_failure() {
    let (app, customer) = app().await;
    let a = app.sell_cash(1000).await;
    let b = app.sell_cash(1000).await;
    let c = app.sell_cash(1000).await;

    let first = do_sign_invoice_fiscal(&app.state, vec![a], customer, OPERATOR.into())
        .await
        .unwrap();
    // The FDM goes away: the invoice fails, its number is not used.
    let url = app.state.config.lock().unwrap().fdm_url.clone();
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".to_string();
    let failed = do_sign_invoice_fiscal(&app.state, vec![b], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(!failed.success);
    assert_eq!(failed.invoice_no, None);
    app.state.config.lock().unwrap().fdm_url = url;
    let third = do_sign_invoice_fiscal(&app.state, vec![c], customer, OPERATOR.into())
        .await
        .unwrap();

    let year = &first.invoice_no.as_deref().unwrap()[..4];
    assert!(year.starts_with("20"), "{year}");
    assert_eq!(first.invoice_no, Some(format!("{year}0001")));
    assert_eq!(third.invoice_no, Some(format!("{year}0002")));
    // The failed sale is still free to invoice.
    assert_eq!(rows(&app, "SELECT COUNT(*) FROM invoices"), 2);
    assert_eq!(rows(&app, "SELECT COUNT(*) FROM invoice_sales"), 2);
    let listed = do_list_invoices(&app.state, None).unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].invoice_no, format!("{year}0002"));
}

#[tokio::test]
async fn a_lost_answer_is_recovered_with_its_number_and_rows() {
    let app = TestApp::new().await;
    let fdm = LossyFdm::start().await;
    app.use_lossy_fdm(&fdm);
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let customer = app.save_customer(VAT, "Client Test SRL");
    let sale = app.sell_cash(1000).await;

    fdm.lose_first("signInvoice");
    let lost = do_sign_invoice_fiscal(&app.state, vec![sale], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(!lost.success, "the answer was lost");
    assert_eq!(rows(&app, "SELECT COUNT(*) FROM invoices"), 0);
    assert!(
        app.open_pending().is_some(),
        "the signed request stays open"
    );

    // The same command again sends the same request, archives it and finishes the job.
    let again = do_sign_invoice_fiscal(&app.state, vec![sale], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(again.success, "{:?}", again.error);
    assert_eq!(app.counter_for(EventLabel::I), 1);
    assert_eq!(rows(&app, "SELECT COUNT(*) FROM invoices"), 1);
    assert_eq!(rows(&app, "SELECT COUNT(*) FROM invoice_sales"), 1);
    assert!(again.invoice_no.is_some());
    assert!(app.open_pending().is_none());
}

#[tokio::test]
async fn the_z_report_lists_the_invoice_with_the_sum_of_its_sales() {
    let (app, customer) = app().await;
    let a = app.sell_cash(1000).await;
    let b = app.sell_cash(550).await;
    let invoice = do_sign_invoice_fiscal(&app.state, vec![a, b], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(invoice.success, "{:?}", invoice.error);

    let z = do_generate_z_report(&app.state, OPERATOR.to_string())
        .await
        .unwrap();
    assert!(z.success, "{:?}", z.error);
    let events = app.fiscal_events();
    let request = events
        .iter()
        .filter(|e| e.event_type == "R")
        .map(|e| {
            serde_json::from_str::<serde_json::Value>(e.event_data_json.as_ref().unwrap()).unwrap()
        })
        .find(|r| r["turnover"].get("invoices").is_some())
        .expect("the turnover Z carries invoices[]");
    let invoices = request["turnover"]["invoices"]
        .as_array()
        .expect("invoices[]");
    assert_eq!(invoices.len(), 1, "{request}");
    assert_eq!(
        invoices[0]["invoiceNo"].as_str(),
        invoice.invoice_no.as_deref()
    );
    assert_eq!(invoices[0]["amount"].as_f64(), Some(15.5));
}

#[tokio::test]
async fn an_invoice_has_a_copy() {
    let (app, customer) = app().await;
    let devices = PrinterSink::start().await;
    app.set_printer(&devices.addr);
    let sale = app.sell_cash(1000).await;
    let invoice = do_sign_invoice_fiscal(&app.state, vec![sale], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(invoice.success && invoice.printed, "{invoice:?}");
    let event = invoice.fiscal_event_id.unwrap();

    let copy = do_copy_ticket(&app.state, event, OPERATOR.to_string())
        .await
        .unwrap();
    assert!(copy.success, "{:?}", copy.error);
    assert_eq!(app.counter_for(EventLabel::C), 1);

    // The sale's ticket, the invoice, the invoice's copy.
    let printed = devices.settle(3).await;
    assert_eq!(printed.len(), 3, "{printed:?}");
    let no = invoice.invoice_no.unwrap();
    assert!(printed[1].contains(&no) && printed[2].contains(&no));
    assert!(printed[2].contains("COPIE"), "{}", printed[2]);
    assert!(!printed[1].contains("COPIE"));
}

#[tokio::test]
async fn a_customer_form_is_checked() {
    let (app, _) = app().await;
    let form = |vat: &str, name: &str, country: &str| B2bCustomerInput {
        id: None,
        vat_number: vat.into(),
        name: name.into(),
        street: "Rue 1".into(),
        postal_code: "4000".into(),
        city: "Liege".into(),
        country_code: country.into(),
    };
    let save = |f: B2bCustomerInput| do_save_b2b_customer(&app.state, f);
    assert_eq!(
        save(form("BE0308357158", "X", "")).unwrap_err(),
        errors::CUSTOMER_VAT_INVALID
    );
    assert!(
        save(form(VAT, "  ", ""))
            .unwrap_err()
            .starts_with(errors::CUSTOMER_INCOMPLETE)
    );
    // A Belgian number with a foreign address, and the other way round.
    assert!(
        save(form(VAT, "X", "FR"))
            .unwrap_err()
            .starts_with(errors::CUSTOMER_INCOMPLETE)
    );
    assert!(
        save(form("FR12345678901", "X", "BE"))
            .unwrap_err()
            .starts_with(errors::CUSTOMER_INCOMPLETE)
    );
    // The same number saved twice is one customer, updated.
    let first = save(form("BE 0308.357.159", "Old name", "")).unwrap();
    let second = save(form(VAT, "New name", "BE")).unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(second.name, "New name");
}
