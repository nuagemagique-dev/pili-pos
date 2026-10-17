//! The Peppol delivery queue (V11-D12) against the Storecove mock of `pos-fdm-mock`: accepted,
//! a 503 then accepted after the backoff, a refusal parked, a rejected key parked with the pass
//! stopped, a recipient who is not registered parked, a retry, an offline access point, nothing
//! queued without a `peppol` section or for a foreign customer, and an immutable attempt log.
//! No real vendor is called.

use std::time::Duration;

use chrono::Utc;
use pos_engine_testkit as common;

use common::*;

use pos_db::queries::invoicing as q;
use pos_engine::commands::errors;
use pos_engine::commands::invoice::{
    do_list_peppol_deliveries, do_lookup_b2b_customer, do_retry_peppol_delivery,
    do_sign_invoice_fiscal,
};
use pos_engine::commands::peppol_queue::{backoff, run_pass_at};
use pos_fdm_mock::storecove::StorecoveMock;

const OPERATOR: &str = "75061189702";
const VAT: &str = "BE0308357159";
const PARTICIPANT: &str = "0208:0308357159";

struct Rig {
    app: TestApp,
    mock: StorecoveMock,
    customer: i64,
    _dir: tempfile::TempDir,
}

async fn rig() -> Rig {
    let app = TestApp::new().await;
    let (mock, dir) = app.use_admission_mock_with_peppol().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let customer = app.save_customer(VAT, "Client Test SRL");
    Rig {
        app,
        mock,
        customer,
        _dir: dir,
    }
}

impl Rig {
    /// Sell and invoice one sale; returns the delivery id.
    async fn invoice(&self) -> i64 {
        let sale = self.app.sell_cash(1000).await;
        let result =
            do_sign_invoice_fiscal(&self.app.state, vec![sale], self.customer, OPERATOR.into())
                .await
                .unwrap();
        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.delivery_status.as_deref(), Some("queued"));
        let db = self.app.state.db.lock().unwrap();
        q::list_deliveries(&db, 1).unwrap()[0].id
    }

    fn delivery(&self, id: i64) -> q::Delivery {
        let db = self.app.state.db.lock().unwrap();
        q::get_delivery(&db, id).unwrap().unwrap()
    }

    fn log(&self, id: i64) -> Vec<(i64, String, Option<i64>, Option<String>)> {
        let db = self.app.state.db.lock().unwrap();
        q::delivery_log(&db, id).unwrap()
    }
}

/// A moment just after the delivery was queued.
fn soon() -> chrono::DateTime<Utc> {
    Utc::now() + chrono::Duration::seconds(2)
}

#[tokio::test]
async fn a_queued_invoice_is_delivered() {
    let r = rig().await;
    let id = r.invoice().await;
    assert_eq!(r.delivery(id).status, "queued");
    assert_eq!(r.delivery(id).customer_endpoint_id, PARTICIPANT);

    assert_eq!(run_pass_at(&r.app.state, soon()).await, 1);
    let d = r.delivery(id);
    assert_eq!(d.status, "accepted", "{d:?}");
    assert_eq!(d.attempts, 1);
    assert!(d.ap_message_id.as_deref().unwrap().starts_with("mock-guid"));
    assert_eq!(d.lookup_status, "registered");
    assert_eq!(r.log(id).len(), 1);
    assert_eq!(r.log(id)[0].1, "accepted");

    // The document the access point got: the invoice, to the right recipient, for the right
    // legal entity, with the API key.
    let submissions = r.mock.submissions();
    assert_eq!(submissions.len(), 1);
    assert_eq!(submissions[0].bearer.as_deref(), Some("test-key"));
    assert_eq!(submissions[0].body["legalEntityId"], 4242);
    let xml = r.mock.submitted_xml(0).unwrap();
    assert!(
        xml.contains(&format!("<cbc:ID>{}</cbc:ID>", d.invoice_no)),
        "{xml}"
    );
    assert!(xml.contains("0308357159"), "{xml}");
    assert!(
        xml.contains("<cbc:PrepaidAmount currencyID=\"EUR\">10.00</cbc:PrepaidAmount>"),
        "{xml}"
    );

    // An accepted delivery is not sent again.
    assert_eq!(run_pass_at(&r.app.state, soon()).await, 0);
    assert_eq!(r.mock.submissions().len(), 1);
}

#[tokio::test]
async fn a_transient_failure_is_retried_after_the_backoff() {
    let r = rig().await;
    let id = r.invoice().await;
    r.mock.script_submissions(&[503, 200]);

    let t0 = soon();
    assert_eq!(run_pass_at(&r.app.state, t0).await, 1);
    let d = r.delivery(id);
    assert_eq!(d.status, "queued");
    assert_eq!(d.attempts, 1);
    assert!(d.error_msg.as_deref().unwrap().contains("503"), "{d:?}");
    let next = chrono::DateTime::parse_from_rfc3339(d.next_attempt_at.as_deref().unwrap()).unwrap();
    // The stored time is cut to the second.
    let wait = (next.with_timezone(&Utc) - t0).num_seconds();
    assert!(
        (backoff(1).as_secs() as i64 - 1..=backoff(1).as_secs() as i64).contains(&wait),
        "{wait}"
    );

    // Too early: nothing is due.
    assert_eq!(
        run_pass_at(&r.app.state, t0 + chrono::Duration::seconds(10)).await,
        0
    );
    // On time: delivered.
    assert_eq!(
        run_pass_at(&r.app.state, t0 + chrono::Duration::seconds(32)).await,
        1
    );
    let d = r.delivery(id);
    assert_eq!(d.status, "accepted");
    assert_eq!(d.attempts, 2);
    let log = r.log(id);
    assert_eq!(
        log.iter().map(|l| l.1.as_str()).collect::<Vec<_>>(),
        ["transient", "accepted"]
    );
    assert_eq!(log[0].2, Some(503));
}

#[tokio::test]
async fn a_refusal_is_parked_and_a_retry_puts_it_back() {
    let r = rig().await;
    let id = r.invoice().await;
    r.mock.script_submissions(&[422]);

    run_pass_at(&r.app.state, soon()).await;
    let d = r.delivery(id);
    assert_eq!(d.status, "failed", "{d:?}");
    assert!(d.parked_at.is_some());
    assert!(d.error_msg.as_deref().unwrap().contains("422"));
    // Parked: no more attempts by itself.
    assert_eq!(
        run_pass_at(&r.app.state, soon() + chrono::Duration::hours(2)).await,
        0
    );

    let panel = do_list_peppol_deliveries(&r.app.state, None).unwrap();
    assert!(panel.configured);
    assert_eq!((panel.queued, panel.accepted, panel.failed), (0, 0, 1));

    do_retry_peppol_delivery(&r.app.state, id).unwrap();
    assert_eq!(r.delivery(id).status, "queued");
    assert_eq!(
        run_pass_at(&r.app.state, soon() + chrono::Duration::seconds(5)).await,
        1
    );
    assert_eq!(r.delivery(id).status, "accepted");
    // The first attempt stays in the log.
    assert_eq!(
        r.log(id).iter().map(|l| l.1.as_str()).collect::<Vec<_>>(),
        ["rejected", "accepted"]
    );

    // Only a parked delivery is retried.
    assert_eq!(
        do_retry_peppol_delivery(&r.app.state, id).unwrap_err(),
        errors::PEPPOL_DELIVERY_NOT_PARKED
    );
    assert_eq!(
        do_retry_peppol_delivery(&r.app.state, 999).unwrap_err(),
        errors::PEPPOL_DELIVERY_NOT_FOUND
    );
}

#[tokio::test]
async fn a_rejected_key_parks_the_delivery_and_stops_the_pass() {
    let r = rig().await;
    let first = r.invoice().await;
    let second = r.invoice().await;
    r.mock.script_submissions(&[401]);

    assert_eq!(run_pass_at(&r.app.state, soon()).await, 1);
    assert_eq!(r.delivery(first).status, "failed");
    assert_eq!(r.log(first)[0].1, "unauthorized");
    // The second one was not even tried: every call would have failed the same way.
    assert_eq!(r.delivery(second).status, "queued");
    assert_eq!(r.delivery(second).attempts, 0);
    assert_eq!(r.mock.submissions().len(), 1);
}

#[tokio::test]
async fn a_recipient_who_is_not_registered_is_parked_without_sending() {
    let r = rig().await;
    r.mock.set_not_registered(PARTICIPANT);
    let id = r.invoice().await;

    run_pass_at(&r.app.state, soon()).await;
    let d = r.delivery(id);
    assert_eq!(d.status, "failed");
    assert_eq!(d.lookup_status, "not_registered");
    assert!(r.mock.submissions().is_empty());
    assert_eq!(r.log(id)[0].1, "not_registered");
}

#[tokio::test]
async fn an_offline_lookup_is_unknown_and_the_invoice_is_still_signed() {
    let r = rig().await;
    // The lookup fails: the answer is "unknown", not an error.
    r.mock.script_lookups(&[500]);
    let lookup = do_lookup_b2b_customer(&r.app.state, "be 0308.357.159")
        .await
        .unwrap();
    assert_eq!(lookup.vat_number, VAT);
    assert_eq!(lookup.peppol_status, "unknown");
    assert!(!lookup.peppol_checked);
    assert!(lookup.deliverable);
    assert_eq!(lookup.customer.unwrap().id, r.customer);
    // A working lookup is cached on the customer.
    let lookup = do_lookup_b2b_customer(&r.app.state, VAT).await.unwrap();
    assert_eq!(lookup.peppol_status, "registered");
    assert!(lookup.peppol_checked);
    assert_eq!(lookup.customer.unwrap().peppol_status, "registered");
    // A slow access point is cut at 3 seconds.
    r.mock.set_delay(Some(Duration::from_secs(4)));
    let started = std::time::Instant::now();
    let lookup = do_lookup_b2b_customer(&r.app.state, VAT).await.unwrap();
    assert!(started.elapsed() < Duration::from_secs(4));
    assert!(!lookup.peppol_checked);
    r.mock.set_delay(None);

    // The access point is gone: the invoice is signed anyway and the delivery waits.
    r.app
        .state
        .config
        .lock()
        .unwrap()
        .peppol
        .as_mut()
        .unwrap()
        .base_url = Some("http://127.0.0.1:1/storecove".to_string());
    let id = r.invoice().await;
    run_pass_at(&r.app.state, soon()).await;
    let d = r.delivery(id);
    assert_eq!(d.status, "queued");
    assert_eq!(d.attempts, 1);
    assert_eq!(r.log(id)[0].1, "transient");
}

#[tokio::test]
async fn nothing_is_queued_without_a_peppol_section() {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let customer = app.save_customer(VAT, "Client Test SRL");
    let sale = app.sell_cash(1000).await;

    let result = do_sign_invoice_fiscal(&app.state, vec![sale], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.delivery_status, None);
    let panel = do_list_peppol_deliveries(&app.state, None).unwrap();
    assert!(!panel.configured);
    assert!(panel.deliveries.is_empty());
    assert_eq!(run_pass_at(&app.state, soon()).await, 0);
    assert_eq!(
        do_retry_peppol_delivery(&app.state, 1).unwrap_err(),
        errors::PEPPOL_NOT_CONFIGURED
    );
}

#[tokio::test]
async fn a_foreign_customer_is_not_queued() {
    let r = rig().await;
    let foreign = r.app.save_customer("FR12345678901", "Client Francais SARL");
    let sale = r.app.sell_cash(1000).await;
    let result = do_sign_invoice_fiscal(&r.app.state, vec![sale], foreign, OPERATOR.into())
        .await
        .unwrap();
    assert!(result.success, "{:?}", result.error);
    assert_eq!(result.delivery_status, None);
    assert!(
        do_list_peppol_deliveries(&r.app.state, None)
            .unwrap()
            .deliveries
            .is_empty()
    );
    let lookup = do_lookup_b2b_customer(&r.app.state, "FR12345678901")
        .await
        .unwrap();
    assert!(!lookup.deliverable);
    assert!(!lookup.peppol_checked);
}

#[tokio::test]
async fn the_attempt_log_cannot_be_changed() {
    let r = rig().await;
    let id = r.invoice().await;
    run_pass_at(&r.app.state, soon()).await;
    assert_eq!(r.log(id).len(), 1);
    let db = r.app.state.db.lock().unwrap();
    assert!(
        db.execute("UPDATE peppol_delivery_log SET outcome = 'x'", [])
            .is_err()
    );
    assert!(db.execute("DELETE FROM peppol_delivery_log", []).is_err());
    assert!(
        db.execute("UPDATE invoices SET total_cents = 1", [])
            .is_err()
    );
    assert!(db.execute("DELETE FROM invoice_sales", []).is_err());
}
