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
    // The copy's own control data sits under the copy label.
    assert!(
        printed[2].contains("Informations FDM de la copie"),
        "{}",
        printed[2]
    );
    assert!(!printed[2].contains("Informations FDM d'origine"));
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

fn request_of(app: &TestApp, event_id: i64) -> serde_json::Value {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .unwrap();
    serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap()
}

/// A sale closed unpaid (event N with empty `financials`), signed by a manager.
async fn unpaid_sale(app: &TestApp, price_cents: i64) -> i64 {
    pos_engine::commands::auth::do_set_pin(&app.state, OPERATOR.into(), "4321".into(), None)
        .unwrap();
    let order = app.order_of(price_cents);
    let r = pos_engine::commands::unpaid::do_close_unpaid(&app.state, order, OPERATOR, "4321")
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

#[tokio::test]
async fn a_training_sale_is_not_a_sale_to_invoice() {
    let (app, customer) = app().await;
    app.state.config.lock().unwrap().training_mode = true;
    let training = app.sell_cash(1000).await;
    app.state.config.lock().unwrap().training_mode = false;
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == training)
        .unwrap();
    assert_eq!(event.event_type, "T");

    let expected = format!("{}|event={training}", errors::INVOICE_NOT_A_SALE);
    let err = do_sign_invoice_fiscal(&app.state, vec![training], customer, OPERATOR.into())
        .await
        .unwrap_err();
    assert_eq!(err, expected);
    let preview = do_invoice_preview(&app.state, &[training]).unwrap();
    assert_eq!(preview.problems, [expected]);
    assert!(preview.lines.is_empty());
    assert_eq!(app.counter_for(EventLabel::I), 0);
}

#[tokio::test]
async fn a_sale_closed_unpaid_is_not_a_sale_to_invoice() {
    let (app, customer) = app().await;
    let unpaid = unpaid_sale(&app, 1000).await;
    // It is a signed N, but nothing was paid: the request has no payment line.
    assert_eq!(
        request_of(&app, unpaid)["financials"],
        serde_json::json!([])
    );
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == unpaid)
        .unwrap();
    assert_eq!(event.event_type, "N");

    let expected = format!("{}|event={unpaid}", errors::INVOICE_NOT_A_SALE);
    let err = do_sign_invoice_fiscal(&app.state, vec![unpaid], customer, OPERATOR.into())
        .await
        .unwrap_err();
    assert_eq!(err, expected);
    let preview = do_invoice_preview(&app.state, &[unpaid]).unwrap();
    assert_eq!(preview.problems, [expected]);
    assert_eq!(app.counter_for(EventLabel::I), 0);

    let paid = app.sell_cash(500).await;
    let listed = do_list_invoiceable_sales(&app.state, None, None).unwrap();
    let ids: Vec<i64> = listed.iter().map(|s| s.fiscal_event_id).collect();
    assert_eq!(ids, [paid], "the unpaid sale is not offered");
}

#[tokio::test]
async fn the_list_of_sales_is_newest_first_and_searchable() {
    let (app, _) = app().await;
    let s1 = app.sell_cash(1000).await;
    app.state.config.lock().unwrap().training_mode = true;
    let training = app.sell_cash(700).await;
    app.state.config.lock().unwrap().training_mode = false;
    let s2 = app.sell_cash(1100).await;
    let unpaid = unpaid_sale(&app, 900).await;
    let s3 = app.sell_cash(1200).await;

    let list = |limit: Option<i64>, query: Option<&str>| -> Vec<i64> {
        do_list_invoiceable_sales(&app.state, limit, query.map(String::from))
            .unwrap()
            .iter()
            .map(|s| s.fiscal_event_id)
            .collect()
    };
    // Newest first; the training sale and the unpaid one are left out.
    assert_eq!(list(None, None), [s3, s2, s1]);
    assert!(!list(None, None).contains(&training));
    assert!(!list(None, None).contains(&unpaid));

    // The limit is clamped to 1..=200.
    assert_eq!(list(Some(2), None), [s3, s2]);
    assert_eq!(list(Some(0), None), [s3]);
    assert_eq!(list(Some(-7), None), [s3]);
    assert_eq!(list(Some(100_000), None), [s3, s2, s1]);

    // By ticket number: the prefix of one sale's number.
    let ticket = |id: i64| request_of(&app, id)["posFiscalTicketNo"].as_i64().unwrap();
    // (A year starts with 2, so a number starting with 1 cannot match a date.)
    let t1 = ticket(s1).to_string();
    assert!(t1.starts_with('1'), "{t1}");
    assert_eq!(list(None, Some(&t1)), [s1]);
    assert_eq!(list(None, Some(&ticket(s3).to_string())), [s3]);
    assert!(list(None, Some("999999")).is_empty());

    // By date: make the first sale an earlier day, then ask for each day.
    let date_of = |id: i64| request_of(&app, id)["posDateTime"].as_str().unwrap()[..10].to_string();
    let today = date_of(s3);
    {
        let db = app.state.db.lock().unwrap();
        // The archive is append-only for real use; this test needs a sale of another day,
        // so the guard is lifted in this throw-away database.
        db.execute("DROP TRIGGER fiscal_events_no_rewrite", [])
            .unwrap();
        db.execute(
            "UPDATE fiscal_events
             SET event_data_json = json_set(event_data_json, '$.posDateTime', '2020-01-15T10:00:00Z')
             WHERE id = ?1",
            [s1],
        )
        .expect("move the first sale to another day");
    }
    assert_eq!(list(None, Some(&today)), [s3, s2]);
    assert_eq!(list(None, Some("2020-01-15")), [s1]);
    assert_eq!(list(None, Some("2020-01")), [s1]);
    assert!(list(None, Some("1999")).is_empty());

    // LIKE wildcards are stripped: "%" and "_" alone mean "no filter", and "%-" followed by
    // the month would match every sale if it were a pattern.
    assert_eq!(list(None, Some("%")), [s3, s2, s1]);
    assert_eq!(list(None, Some("_")), [s3, s2, s1]);
    assert_eq!(list(None, Some("  ")), [s3, s2, s1]);
    let month = format!("%{}", &today[4..7]);
    assert!(list(None, Some(&month)).is_empty(), "{month}");
    assert_eq!(list(None, Some("2020%01")), [] as [i64; 0]);
}

#[tokio::test]
async fn a_customer_is_updated_by_id() {
    let (app, a) = app().await;
    const OTHER_VAT: &str = "BE0123456749";
    let b = app.save_customer(OTHER_VAT, "Autre Client SA");
    let form = |id: Option<i64>, vat: &str, name: &str, street: &str| B2bCustomerInput {
        id,
        vat_number: vat.into(),
        name: name.into(),
        street: street.into(),
        postal_code: "1000".into(),
        city: "Bruxelles".into(),
        country_code: String::new(),
    };
    let save = |f: B2bCustomerInput| do_save_b2b_customer(&app.state, f);

    // An unknown id is refused, nothing is created.
    assert_eq!(
        save(form(Some(999), VAT, "X", "Rue 1")).unwrap_err(),
        errors::CUSTOMER_NOT_FOUND
    );
    // B's number cannot be taken by A.
    assert_eq!(
        save(form(Some(a), OTHER_VAT, "X", "Rue 1")).unwrap_err(),
        format!("{}|field=vat_number", errors::CUSTOMER_INCOMPLETE)
    );
    assert_eq!(rows(&app, "SELECT COUNT(*) FROM b2b_customers"), 2);

    // A new name and address: same row, fields updated, operator recorded.
    let updated = save(form(Some(a), VAT, "Nouveau Nom", "Rue Neuve 9")).unwrap();
    assert_eq!(updated.id, a);
    assert_eq!(updated.name, "Nouveau Nom");
    assert_eq!(updated.street, "Rue Neuve 9");
    assert_eq!(updated.postal_code, "1000");
    assert_eq!(updated.city, "Bruxelles");
    let by: String = app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT updated_by FROM b2b_customers WHERE id = ?1",
            [a],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(by, OPERATOR);
    // B is untouched.
    let untouched = pos_db::queries::invoicing::get_customer(&app.state.db.lock().unwrap(), b)
        .unwrap()
        .unwrap();
    assert_eq!(untouched.name, "Autre Client SA");

    // A changed VAT number forgets what the network said about the old one.
    {
        let db = app.state.db.lock().unwrap();
        pos_db::queries::invoicing::set_customer_peppol_status(
            &db,
            a,
            "registered",
            "2026-10-01T10:00:00Z",
        )
        .unwrap();
    }
    let same = save(form(Some(a), VAT, "Nouveau Nom", "Rue Neuve 9")).unwrap();
    assert_eq!(same.peppol_status, "registered", "same number: cache kept");
    let moved = save(form(Some(a), "BE0203201340", "Nouveau Nom", "Rue Neuve 9")).unwrap();
    assert_eq!(moved.id, a);
    assert_eq!(moved.vat_number, "BE0203201340");
    assert_eq!(moved.peppol_status, "unknown");
    assert_eq!(moved.peppol_checked_at, None);
}
