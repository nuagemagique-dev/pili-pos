//! Online orders HubRise cancels or changes after the POS got them.
//! Before acceptance nothing was signed and nothing is. After an N, the
//! order waits for a manager's refund (an N citing the sale, paid back
//! APP or ONLINE); after a P, its closing corrects every line and signs
//! an N at 0,00 EUR. Nothing is ever signed on HubRise's word alone.

#![cfg(feature = "hubrise")]

mod common;

use common::hubrise::{HubriseMock, connected_app, drain_outbox, poller};
use common::{PrinterSink, TestApp};
use pili_pos::commands::checkout::PaymentPart;
use pili_pos::commands::errors;
use pili_pos::commands::webshop::{
    do_accept_webshop_order, do_close_cancelled_webshop_order, do_handover_webshop_order,
    do_refund_webshop_order, recover_webshop_orders,
};
use pos_core::user::UserRole;
use pos_hubrise::poller::{poll_once, start};
use serde_json::{Value, json};

const CASHIER: &str = "12345678958";
const MANAGER: &str = "85073003328";

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
    app.login_as(CASHIER, UserRole::Cashier);
    World {
        app,
        _db: db,
        mock,
        printer,
    }
}

impl World {
    async fn order(&self, template: &str) -> (i64, String) {
        let hr = self.mock.inject(template);
        let hr_id = hr["id"].as_str().unwrap().to_owned();
        self.poll().await;
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

    async fn poll(&self) {
        poll_once(&poller(&self.app)).await.unwrap();
    }

    /// An accepted order, then cancelled by the platform.
    async fn cancelled_after_acceptance(&self, template: &str) -> (i64, String) {
        let (id, hr_id) = self.order(template).await;
        do_accept_webshop_order(&self.app.state, id, None)
            .await
            .unwrap();
        drain_outbox(&self.app).await;
        self.mock.update(&hr_id, json!({ "status": "cancelled" }));
        self.poll().await;
        (id, hr_id)
    }

    fn state_of(&self, order_id: i64) -> pos_hubrise::webshop::WebshopState {
        let db = self.app.state.db.lock().unwrap();
        pos_hubrise::webshop::load(&db, order_id).unwrap().unwrap()
    }

    fn column(&self, order_id: i64, column: &str) -> Option<i64> {
        self.app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                &format!("SELECT {column} FROM webshop_orders WHERE order_id = ?1"),
                [order_id],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn request(&self, event_id: i64) -> Value {
        let row = self
            .app
            .fiscal_events()
            .into_iter()
            .find(|e| e.id == event_id)
            .expect("event");
        serde_json::from_str(row.event_data_json.as_deref().unwrap()).unwrap()
    }

    fn labels(&self) -> Vec<String> {
        self.app
            .fiscal_events()
            .into_iter()
            .map(|e| e.event_type)
            .collect()
    }

    /// Point the POS at a closed port; returns the FDM to restore.
    fn fdm_down(&self) -> String {
        let mut config = self.app.state.config.lock().unwrap();
        std::mem::replace(&mut config.fdm_url, "http://127.0.0.1:9/graphql".into())
    }

    fn fdm_up(&self, url: String) {
        self.app.state.config.lock().unwrap().fdm_url = url;
    }

    fn manager(&self) {
        self.app.login_as(MANAGER, UserRole::Admin);
    }
}

fn quantities(request: &Value) -> Vec<f64> {
    request["transaction"]["transactionLines"]
        .as_array()
        .expect("transaction lines")
        .iter()
        .map(|l| l["mainProduct"]["quantity"].as_f64().unwrap())
        .collect()
}

fn payment_types(request: &Value) -> Vec<String> {
    request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["type"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn cancelled_before_acceptance_signs_nothing() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    w.mock.update(&hr_id, json!({ "status": "cancelled" }));
    w.poll().await;
    assert_eq!(w.state_of(id).fiscal_state, "voided");
    assert!(w.labels().is_empty());
    assert_eq!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap_err(),
        errors::hubrise::ALREADY_HANDLED
    );
}

#[tokio::test]
async fn cancelled_after_the_n_waits_for_a_manager_and_signs_nothing_alone() {
    let w = world().await;
    let (id, _) = w.cancelled_after_acceptance("deliveroo_prepaid").await;
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");
    assert_eq!(w.labels(), vec!["N"], "only the sale");
    // Polling again changes nothing.
    w.poll().await;
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");
    assert_eq!(w.labels(), vec!["N"]);
}

#[tokio::test]
async fn the_managers_refund_cites_the_sale_and_goes_back_by_app() {
    let w = world().await;
    let (id, _) = w.cancelled_after_acceptance("deliveroo_prepaid").await;
    let sale = w.column(id, "n_event_id").unwrap();
    let kicks = w.printer.drawer_kicks();
    w.manager();

    let refund = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
        .await
        .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let refund_id = refund.fiscal_event_id.unwrap();
    assert_eq!(w.state_of(id).fiscal_state, "refunded");
    assert_eq!(w.column(id, "refund_event_id"), Some(refund_id));

    let sold = w.request(sale);
    let back = w.request(refund_id);
    assert!(back["fdmRef"].is_object(), "the refund cites the sale");
    assert_eq!(payment_types(&back), vec!["APP"]);
    // Every line, each negated.
    let mut negated: Vec<f64> = quantities(&sold).iter().map(|q| -q).collect();
    let mut refunded = quantities(&back);
    negated.sort_by(f64::total_cmp);
    refunded.sort_by(f64::total_cmp);
    assert_eq!(refunded, negated);
    assert_eq!(w.labels(), vec!["N", "N"]);

    // The day's takings net to zero; no money left the drawer.
    let net: i64 = w
        .app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT COALESCE(SUM(amount_cents), 0) FROM payments",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(net, 0);
    w.printer.settle(1).await;
    assert_eq!(w.printer.drawer_kicks(), kicks, "the platform pays back");
}

#[tokio::test]
async fn a_webshop_refund_goes_back_online() {
    let w = world().await;
    let (id, _) = w.cancelled_after_acceptance("webshop_online").await;
    w.manager();
    let refund = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
        .await
        .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    assert_eq!(
        payment_types(&w.request(refund.fiscal_event_id.unwrap())),
        vec!["ONLINE"]
    );
}

#[tokio::test]
async fn a_refund_needs_a_manager_and_is_made_once() {
    let w = world().await;
    let (id, _) = w.cancelled_after_acceptance("ubereats_prepaid").await;
    let refused = do_refund_webshop_order(&w.app.state, id, CASHIER, None)
        .await
        .unwrap();
    assert!(!refused.success);
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");

    w.manager();
    assert!(
        do_refund_webshop_order(&w.app.state, id, MANAGER, None)
            .await
            .unwrap()
            .success
    );
    let again = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
        .await
        .unwrap();
    assert_eq!(again.error.as_deref(), Some(errors::hubrise::NOT_CANCELLED));
    assert_eq!(w.labels(), vec!["N", "N"]);
}

#[tokio::test]
async fn only_a_cancelled_signed_order_is_refunded() {
    let w = world().await;
    let (pending, _) = w.order("deliveroo_prepaid").await;
    let (signed, _) = w.order("ubereats_prepaid").await;
    do_accept_webshop_order(&w.app.state, signed, None)
        .await
        .unwrap();
    w.manager();
    for id in [pending, signed] {
        let r = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
            .await
            .unwrap();
        assert_eq!(r.error.as_deref(), Some(errors::hubrise::NOT_CANCELLED));
    }
    assert_eq!(w.labels(), vec!["N"]);
}

#[tokio::test]
async fn an_fdm_failure_leaves_the_refund_to_do() {
    let w = world().await;
    let (id, _) = w.cancelled_after_acceptance("deliveroo_prepaid").await;
    w.manager();
    let fdm = w.fdm_down();
    let failed = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
        .await
        .unwrap();
    assert!(!failed.success);
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");
    w.fdm_up(fdm);
    assert!(
        do_refund_webshop_order(&w.app.state, id, MANAGER, None)
            .await
            .unwrap()
            .success
    );
}

#[tokio::test]
async fn cancelled_after_the_p_is_closed_by_corrections_and_an_n_at_zero() {
    let w = world().await;
    let (id, hr_id) = w
        .cancelled_after_acceptance("webshop_cash_collection")
        .await;
    assert_eq!(w.state_of(id).fiscal_state, "cancel_pending");
    // Nothing to collect any more.
    let handover = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert_eq!(
        handover.error.as_deref(),
        Some(errors::hubrise::NOT_AWAITING_PAYMENT)
    );

    let closed = do_close_cancelled_webshop_order(&w.app.state, id)
        .await
        .unwrap();
    assert!(closed.success, "{:?}", closed.error);
    assert_eq!(w.state_of(id).fiscal_state, "closed");
    assert_eq!(w.labels(), vec!["P", "N"]);
    let n = w.request(closed.fiscal_event_id.unwrap());
    assert_eq!(n["costCenter"]["reference"], hr_id.as_str(), "closes the P");
    // Every addition, then its correction: 0,00 EUR.
    let q = quantities(&n);
    assert!(q.iter().any(|q| *q > 0.0) && q.iter().any(|q| *q < 0.0));
    assert_eq!(q.iter().sum::<f64>(), 0.0);
    assert_eq!(
        n["transaction"]["transactionTotal"].as_f64(),
        Some(0.0),
        "{n}"
    );
    assert!(closed.ticket.is_some(), "its ticket is printed");
    assert_eq!(w.app.order_status(id).as_deref(), Some("closed"));
    // HubRise already knows: it cancelled.
    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&hr_id)["status"], "cancelled");
}

#[tokio::test]
async fn a_failed_closing_is_retried_without_correcting_twice() {
    let w = world().await;
    let (id, _) = w.cancelled_after_acceptance("webshop_cash_delivery").await;
    let fdm = w.fdm_down();
    let failed = do_close_cancelled_webshop_order(&w.app.state, id)
        .await
        .unwrap();
    assert!(!failed.success);
    assert_eq!(w.state_of(id).fiscal_state, "cancel_pending");
    w.fdm_up(fdm);
    let closed = do_close_cancelled_webshop_order(&w.app.state, id)
        .await
        .unwrap();
    assert!(closed.success, "{:?}", closed.error);
    let q = quantities(&w.request(closed.fiscal_event_id.unwrap()));
    assert_eq!(q.iter().sum::<f64>(), 0.0);
    assert_eq!(
        q.iter().filter(|q| **q < 0.0).count(),
        q.iter().filter(|q| **q > 0.0).count(),
        "one correction per line"
    );
}

#[tokio::test]
async fn a_change_after_signing_is_flagged_not_signed() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    drain_outbox(&w.app).await;
    // The platform drops the drink after the sale was signed.
    let drink = w.mock.order(&hr_id)["items"][1]["id"].clone();
    w.mock.update(
        &hr_id,
        json!({ "items": [{ "id": drink, "deleted": true }] }),
    );
    w.poll().await;

    let state = w.state_of(id);
    assert_eq!(state.fiscal_state, "signed_n");
    assert!(
        state
            .attention
            .contains(&pos_hubrise::webshop::Attention::ChangedAfterSigning)
    );
    assert_eq!(w.labels(), vec!["N"]);
}

#[tokio::test]
async fn a_cancellation_while_signing_skips_the_accepted_status() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    // HubRise cancels between the claim and the FDM's answer.
    w.mock.update(&hr_id, json!({ "status": "cancelled" }));
    {
        let db = w.app.state.db.lock().unwrap();
        db.execute(
            "UPDATE webshop_orders SET fiscal_state = 'signing' WHERE order_id = ?1",
            [id],
        )
        .unwrap();
    }
    w.poll().await;
    {
        let db = w.app.state.db.lock().unwrap();
        db.execute(
            "UPDATE webshop_orders SET fiscal_state = 'pending' WHERE order_id = ?1",
            [id],
        )
        .unwrap();
    }
    let accepted = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    assert_eq!(accepted.fiscal_state, "signed_n");
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");
    drain_outbox(&w.app).await;
    assert_eq!(w.mock.order(&hr_id)["status"], "cancelled");
}

#[tokio::test]
async fn an_interrupted_refund_or_closing_is_repaired_at_boot() {
    let w = world().await;
    let (refunded, _) = w.cancelled_after_acceptance("deliveroo_prepaid").await;
    let (unrefunded, _) = w.cancelled_after_acceptance("ubereats_prepaid").await;
    let (closed, _) = w
        .cancelled_after_acceptance("webshop_cash_collection")
        .await;
    let (unclosed, _) = w.cancelled_after_acceptance("webshop_cash_delivery").await;
    w.manager();
    let refund = do_refund_webshop_order(&w.app.state, refunded, MANAGER, None)
        .await
        .unwrap();
    let closing = do_close_cancelled_webshop_order(&w.app.state, closed)
        .await
        .unwrap();
    assert!(refund.success && closing.success);
    {
        // As if the POS died between the FDM's answers and recording
        // them, and while starting the two others.
        let db = w.app.state.db.lock().unwrap();
        db.execute_batch(&format!(
            "UPDATE webshop_orders SET fiscal_state = 'signing', refund_event_id = NULL
                WHERE order_id = {refunded};
             UPDATE webshop_orders SET fiscal_state = 'signing', n_event_id = NULL
                WHERE order_id = {closed};
             UPDATE webshop_orders SET fiscal_state = 'signing'
                WHERE order_id IN ({unrefunded}, {unclosed});"
        ))
        .unwrap();
        assert_eq!(recover_webshop_orders(&db).unwrap(), 4);
    }
    assert_eq!(w.state_of(refunded).fiscal_state, "refunded");
    assert_eq!(
        w.column(refunded, "refund_event_id"),
        refund.fiscal_event_id
    );
    assert_eq!(w.state_of(unrefunded).fiscal_state, "refund_pending");
    assert_eq!(w.state_of(closed).fiscal_state, "closed");
    assert_eq!(w.column(closed, "n_event_id"), closing.fiscal_event_id);
    assert_eq!(w.state_of(unclosed).fiscal_state, "cancel_pending");
}

#[tokio::test]
async fn a_cancellation_while_the_pos_was_off_is_found_at_start() {
    let w = world().await;
    let (id, hr_id) = w.order("deliveroo_prepaid").await;
    do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    drain_outbox(&w.app).await;
    assert_eq!(w.app.order_status(id).as_deref(), Some("closed"));
    // Cancelled while the POS was off: its event is lost.
    w.mock.update(&hr_id, json!({ "status": "cancelled" }));
    w.mock
        .state
        .inner
        .lock()
        .unwrap()
        .clients
        .get_mut(common::hubrise::POS_TOKEN)
        .unwrap()
        .events
        .clear();
    w.poll().await;
    assert_eq!(w.state_of(id).fiscal_state, "signed_n");

    start(&poller(&w.app)).await.unwrap();
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");
    assert_eq!(w.labels(), vec!["N"], "nothing signed on its own");
}
