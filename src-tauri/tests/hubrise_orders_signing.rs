//! Online orders signed by the FDM: prepaid ones end in an N at
//! acceptance (APP or ONLINE), unpaid ones in a P at acceptance then an
//! N at handover with the same cost center; rejected ones are never
//! signed. Against the mock FDM, the HubRise mock and a recording
//! printer.

#![cfg(feature = "hubrise")]

mod common;

use common::hubrise::{HubriseMock, connected_app, drain_outbox, outbox_due_now, poller};
use common::{PrinterSink, TestApp};
use pili_pos::commands::checkout::{CheckoutPaymentMethod, PaymentPart, do_checkout_split};
use pili_pos::commands::errors;
use pili_pos::commands::webshop::{
    do_accept_webshop_order, do_complete_webshop_order, do_handover_webshop_order,
    do_reject_webshop_order, recover_webshop_orders,
};
use pos_hubrise::poller::{poll_once, start};
use serde_json::Value;

const INSZ: &str = "12345678958";

struct World {
    app: TestApp,
    _db: tempfile::TempDir,
    mock: HubriseMock,
    printer: PrinterSink,
}

async fn world() -> World {
    let (app, db, mock) = connected_app().await;
    app.use_admission_mock().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    start(&poller(&app)).await.unwrap();
    app.login_as(INSZ, pos_core::user::UserRole::Cashier);
    World {
        app,
        _db: db,
        mock,
        printer,
    }
}

impl World {
    /// Inject a template and let the POS fetch it: the local order id
    /// and the HubRise id.
    async fn order(&self, template: &str) -> (i64, String) {
        let hr = self.mock.inject(template);
        let hr_id = hr["id"].as_str().unwrap().to_owned();
        poll_once(&poller(&self.app)).await.unwrap();
        let id = self
            .app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM orders WHERE hubrise_order_id = ?1",
                [&hr_id],
                |r| r.get(0),
            )
            .unwrap();
        (id, hr_id)
    }

    fn state_of(&self, order_id: i64) -> pos_hubrise::webshop::WebshopState {
        let db = self.app.state.db.lock().unwrap();
        pos_hubrise::webshop::load(&db, order_id).unwrap().unwrap()
    }

    /// The signed request of an event, as sent to the FDM.
    fn request(&self, event_id: i64) -> Value {
        let row = self
            .app
            .fiscal_events()
            .into_iter()
            .find(|e| e.id == event_id)
            .expect("event");
        serde_json::from_str(row.event_data_json.as_deref().unwrap()).unwrap()
    }

    fn events(&self) -> Vec<(String, Option<i64>)> {
        self.app
            .fiscal_events()
            .into_iter()
            .map(|e| (e.event_type, e.order_id))
            .collect()
    }

    /// The API path of the location's orders.
    fn orders_path(&self) -> String {
        let loc = self
            .mock
            .state
            .inner
            .lock()
            .unwrap()
            .default_location_id
            .clone();
        format!("/v1/locations/{loc}/orders")
    }

    /// Statuses the POS has sent HubRise for `hr_id`, in order.
    fn patches(&self, hr_id: &str) -> Vec<Value> {
        self.mock
            .requests("PATCH", &format!("{}/{hr_id}", self.orders_path()))
            .into_iter()
            .filter_map(|r| r.body)
            .collect()
    }
}

fn cost_center(request: &Value) -> (&str, &str, &str) {
    let cc = &request["costCenter"];
    (
        cc["type"].as_str().unwrap_or_default(),
        cc["id"].as_str().unwrap_or_default(),
        cc["reference"].as_str().unwrap_or_default(),
    )
}

fn payment_types(request: &Value) -> Vec<String> {
    request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["type"].as_str().unwrap().to_owned())
        .collect()
}

fn vat_labels(request: &Value) -> Vec<String> {
    request["transaction"]["transactionLines"]
        .as_array()
        .expect("transaction lines")
        .iter()
        .map(|l| {
            l["mainProduct"]["vats"][0]["label"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect()
}

#[tokio::test]
async fn a_prepaid_deliveroo_order_is_one_n_paid_by_app() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    let result = do_accept_webshop_order(&w.app.state, id, Some("2026-09-30T12:30:00+02:00"))
        .await
        .unwrap();
    assert_eq!(result.fiscal_state, "signed_n");

    assert_eq!(w.events(), vec![("N".to_string(), Some(id))]);
    let request = w.request(result.fiscal_event_id);
    assert_eq!(
        cost_center(&request),
        ("PLATFORM", "DELIVEROO", hr_id.as_str())
    );
    assert_eq!(payment_types(&request), vec!["APP"]);
    let row = &w.app.fiscal_events()[0];
    assert_eq!(
        row.user_insz, INSZ,
        "signed in the accepting employee's name"
    );

    let state = w.state_of(id);
    assert_eq!(state.fiscal_state, "signed_n");
    // The VAT ticket and the kitchen ticket printed; no bar slip.
    assert!(result.sale.as_ref().unwrap().ticket.is_some());
    assert!(result.kitchen_printed);
    assert!(w.printer.bar_slips().is_empty());

    drain_outbox(&w.app).await;
    let order = w.mock.order(&hr_id);
    assert_eq!(order["status"], "accepted");
    assert_eq!(order["confirmed_time"], "2026-09-30T12:30:00+02:00");
    // No echo of the sale to HubRise: it came from there.
    assert!(w.mock.requests("POST", &w.orders_path()).is_empty());
    let queued: i64 = w
        .app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM hubrise_outbox WHERE kind = 'create_order'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(queued, 0);
}

#[tokio::test]
async fn delivered_food_is_c_and_the_webshop_pays_online() {
    let w = world().await;
    let (id, hr_id) = w.order("webshop_online").await;
    let result = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    let request = w.request(result.fiscal_event_id);
    assert_eq!(
        cost_center(&request),
        ("WEBSHOP", "WEBSITE", hr_id.as_str())
    );
    assert_eq!(payment_types(&request), vec!["ONLINE"]);
    assert_eq!(vat_labels(&request), vec!["C", "A"]);
}

#[tokio::test]
async fn food_eaten_in_is_b() {
    let w = world().await;
    pili_pos::commands::hubrise::do_set_channel(&w.app.state, &{
        w.app.login_as(INSZ, pos_core::user::UserRole::Admin);
        pos_hubrise::webshop::Channel {
            channel: "Table QR".into(),
            kind: "platform".into(),
            payment_type: "APP".into(),
            charge_policy: "include".into(),
        }
    })
    .unwrap();
    let (id, _) = w.order("eat_in").await;
    let result = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    assert_eq!(
        vat_labels(&w.request(result.fiscal_event_id)),
        vec!["B", "A"]
    );
}

#[tokio::test]
async fn a_double_tap_signs_once() {
    let w = world().await;
    let (id, _) = w.order("ubereats_prepaid").await;
    let (a, b) = tokio::join!(
        do_accept_webshop_order(&w.app.state, id, None),
        do_accept_webshop_order(&w.app.state, id, None)
    );
    let ok = [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count();
    assert_eq!(ok, 1);
    let err = a.err().or(b.err()).unwrap();
    assert_eq!(err, errors::hubrise::ALREADY_HANDLED);
    assert_eq!(w.events().len(), 1);
    assert_eq!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap_err(),
        errors::hubrise::ALREADY_HANDLED
    );
}

#[tokio::test]
async fn a_duplicate_event_or_a_restart_does_not_sign_again() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    // The platform updates the order; the POS fetches it again.
    w.mock.update(
        &hr_id,
        serde_json::json!({ "customer_notes": "Sonnez deux fois" }),
    );
    poll_once(&poller(&w.app)).await.unwrap();
    start(&poller(&w.app)).await.unwrap();
    {
        let db = w.app.state.db.lock().unwrap();
        assert_eq!(recover_webshop_orders(&db).unwrap(), 0);
    }
    assert_eq!(w.events().len(), 1);
    assert_eq!(w.state_of(id).fiscal_state, "signed_n");
}

#[tokio::test]
async fn an_order_to_check_cannot_be_accepted() {
    let w = world().await;
    let (id, _) = w.order("eat_in").await; // unknown channel
    assert_eq!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap_err(),
        errors::hubrise::NEEDS_REVIEW
    );
    assert!(w.events().is_empty());
    assert_eq!(w.state_of(id).fiscal_state, "pending");
}

#[tokio::test]
async fn without_fdm_nothing_is_signed_nor_sent_and_a_retry_works() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    let fdm_url = w.app.state.config.lock().unwrap().fdm_url.clone();
    w.app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:9/graphql".into();

    assert!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .is_err()
    );
    assert!(w.events().is_empty());
    assert_eq!(w.state_of(id).fiscal_state, "pending");
    drain_outbox(&w.app).await;
    assert!(!w.patches(&hr_id).iter().any(|p| p["status"] == "accepted"));

    w.app.state.config.lock().unwrap().fdm_url = fdm_url;
    let result = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    assert_eq!(result.fiscal_state, "signed_n");
    assert_eq!(w.events().len(), 1);
    // The lines were written once, by the first try.
    let lines: i64 = w
        .app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM order_lines WHERE order_id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(lines, 2);
}

#[tokio::test]
async fn a_hubrise_outage_after_signing_is_caught_up_by_the_outbox() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    drain_outbox(&w.app).await; // the `received` acknowledgement
    w.mock.fault(
        "PATCH",
        &format!("{}/{hr_id}", w.orders_path()),
        500,
        Some(1),
    );
    do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    let first = drain_outbox(&w.app).await;
    assert_eq!(first.retried, 1);
    assert_eq!(w.mock.order(&hr_id)["status"], "received");

    outbox_due_now(&w.app);
    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&hr_id)["status"], "accepted");
    assert_eq!(w.events().len(), 1);
}

#[tokio::test]
async fn an_unpaid_order_is_a_p_then_an_n_at_handover_with_the_same_cost_center() {
    let w = world().await;
    let (id, hr_id) = w.order("webshop_cash_collection").await;
    let accepted = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    assert_eq!(accepted.fiscal_state, "signed_p");
    let p = w.request(accepted.fiscal_event_id);
    assert_eq!(cost_center(&p), ("WEBSHOP", "WEBSITE", hr_id.as_str()));
    assert_eq!(w.events(), vec![("P".to_string(), Some(id))]);
    assert_eq!(w.app.order_status(id).as_deref(), Some("open"));

    let drawer_before = w.printer.drawer_kicks();
    let sale = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let n = w.request(sale.fiscal_event_id.unwrap());
    assert_eq!(cost_center(&n), cost_center(&p), "the N closes the P");
    assert_eq!(payment_types(&n), vec!["CASH"]);
    assert_eq!(
        w.events().iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
        vec!["P", "N"]
    );
    assert_eq!(w.state_of(id).fiscal_state, "signed_n");
    w.printer.settle(1).await;
    assert!(
        w.printer.drawer_kicks() > drawer_before,
        "cash opens the drawer"
    );

    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&hr_id)["status"], "completed");
}

#[tokio::test]
async fn only_an_accepted_unpaid_order_is_collected_and_never_by_app() {
    let w = world().await;
    let (pending, _) = w.order("webshop_cash_delivery").await;
    let r = do_handover_webshop_order(&w.app.state, pending, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert_eq!(
        r.error.as_deref(),
        Some(errors::hubrise::NOT_AWAITING_PAYMENT)
    );

    do_accept_webshop_order(&w.app.state, pending, None)
        .await
        .unwrap();
    let r = do_handover_webshop_order(
        &w.app.state,
        pending,
        &[PaymentPart::of(CheckoutPaymentMethod::App, 1600)],
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        r.error.as_deref(),
        Some(errors::hubrise::PAYMENT_NOT_AT_TILL)
    );
    assert_eq!(w.events().len(), 1, "only the P");
}

#[tokio::test]
async fn a_rejected_order_is_never_signed() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    do_reject_webshop_order(&w.app.state, id).unwrap();
    assert!(w.events().is_empty());
    assert_eq!(w.state_of(id).fiscal_state, "rejected");
    assert_eq!(w.app.order_status(id).as_deref(), Some("voided"));
    assert_eq!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap_err(),
        errors::hubrise::ALREADY_HANDLED
    );
    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&hr_id)["status"], "rejected");
}

#[tokio::test]
async fn an_order_to_check_can_be_rejected() {
    let w = world().await;
    let (id, _) = w.order("eat_in").await;
    do_reject_webshop_order(&w.app.state, id).unwrap();
    assert_eq!(w.state_of(id).fiscal_state, "rejected");
}

#[tokio::test]
async fn not_in_training_mode_and_not_by_a_waiter() {
    let w = world().await;
    let (id, _) = w.order("deliveroo_prepaid").await;
    w.app.state.config.lock().unwrap().training_mode = true;
    assert_eq!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap_err(),
        errors::hubrise::TRAINING
    );
    w.app.state.config.lock().unwrap().training_mode = false;
    w.app.login_as(INSZ, pos_core::user::UserRole::Waiter);
    assert!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .is_err()
    );
    assert!(do_reject_webshop_order(&w.app.state, id).is_err());
    assert!(w.events().is_empty());
}

#[tokio::test]
async fn the_till_never_sells_an_online_order_nor_takes_app_payments() {
    let w = world().await;
    let (id, _) = w.order("webshop_cash_collection").await;
    do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    let r = do_checkout_split(&w.app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(!r.success);
    let r = do_checkout_split(
        &w.app.state,
        id,
        &[PaymentPart::of(CheckoutPaymentMethod::Online, 100)],
        None,
    )
    .await
    .unwrap();
    assert!(!r.success);
    assert_eq!(w.events().len(), 1, "only the P");
}

#[tokio::test]
async fn a_crash_while_signing_is_repaired_at_boot() {
    let w = world().await;
    let (signed, signed_hr) = w.order("deliveroo_prepaid").await;
    let (unsigned, _) = w.order("ubereats_prepaid").await;
    do_accept_webshop_order(&w.app.state, signed, None)
        .await
        .unwrap();
    {
        // As if the POS died between the FDM's answer and recording it.
        let db = w.app.state.db.lock().unwrap();
        db.execute_batch(&format!(
            "UPDATE webshop_orders SET fiscal_state = 'signing', n_event_id = NULL
                WHERE order_id = {signed};
             UPDATE webshop_orders SET fiscal_state = 'signing' WHERE order_id = {unsigned};
             DELETE FROM hubrise_outbox WHERE body_json LIKE '%\"accepted\"%';"
        ))
        .unwrap();
        assert_eq!(recover_webshop_orders(&db).unwrap(), 2);
    }
    assert_eq!(w.state_of(signed).fiscal_state, "signed_n");
    assert_eq!(w.state_of(unsigned).fiscal_state, "pending");
    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&signed_hr)["status"], "accepted");
    assert_eq!(w.events().len(), 1);
}

#[tokio::test]
async fn a_prepaid_order_is_completed_once_without_signing_again() {
    let w = world().await;
    let (prepaid, prepaid_hr) = w.order("deliveroo_prepaid").await;
    let (unpaid, _) = w.order("webshop_cash_collection").await;
    for id in [prepaid, unpaid] {
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap();
    }
    drain_outbox(&w.app).await;
    // The unpaid order is completed by its handover sale, not here.
    assert_eq!(
        do_complete_webshop_order(&w.app.state, unpaid).unwrap_err(),
        errors::hubrise::NOT_AWAITING_PAYMENT
    );
    do_complete_webshop_order(&w.app.state, prepaid).unwrap();
    assert_eq!(
        do_complete_webshop_order(&w.app.state, prepaid).unwrap_err(),
        errors::hubrise::ALREADY_HANDLED
    );
    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&prepaid_hr)["status"], "completed");
    assert_eq!(w.events().len(), 2, "the N and the P, nothing more");
}
