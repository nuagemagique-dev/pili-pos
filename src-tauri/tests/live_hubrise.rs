//! The HubRise integration against the real HubRise API, on a test
//! location, with the FDM mock signing.
//!
//! Ignored by default and never run in CI. Run it by hand, one test at
//! a time, after `scripts/live_hubrise_login.sh`:
//!
//! ```bash
//! scripts/live_hubrise.sh live_catalog_is_mirrored
//! ```
//!
//! The test location is on HubRise's free plan: 5 orders a month.
//! Each order test places one order.
//!
//! | Variable | Default | Purpose |
//! |---|---|---|
//! | `PILI_LIVE_HUBRISE_API` | `https://api.hubrise.com` | HubRise API |
//! | `PILI_LIVE_PAUSE_MS` | `1000` | least gap between two calls |
//! | `PILI_LIVE_HUBRISE_MAX_REQUESTS` | `40` | calls per run |
//! | `PILI_LIVE_RECORD_DIR` | unset | where each call is recorded, scrubbed |
//!
//! The suite's own calls (loading the test menu, placing and cancelling
//! orders) use the same token as the POS: HubRise sends no event to the
//! client that caused it, so the POS finds the orders the way it does
//! after a restart (`poller::start`: backfill, then a re-read of the
//! orders it knows).

#![cfg(feature = "hubrise")]

mod common;

use common::TestApp;
use common::hubrise::{drain_outbox, poller};
use common::live_hubrise::{LiveToken, live_client, live_hubrise_app, live_hubrise_requests};
use pili_pos::commands::checkout::PaymentPart;
use pili_pos::commands::webshop::{
    do_accept_webshop_order, do_handover_webshop_order, do_refund_webshop_order,
};
use pos_core::user::UserRole;
use pos_hubrise::poller::start;
use pos_hubrise_mock::builders::{seed_catalog_builder, template};
use serde_json::{Value, json};

const CASHIER: &str = "12345678958";
const MANAGER: &str = "85073003328";

struct Live {
    app: TestApp,
    _db: tempfile::TempDir,
    token: LiveToken,
    hubrise: pos_hubrise::client::Client,
}

async fn live() -> Option<Live> {
    let (app, db, token, proxy) = live_hubrise_app().await?;
    eprintln!(
        "live HubRise: {} / {} (location {})",
        token.account_name, token.location_name, token.location_id
    );
    // Never the real restaurant: the test location's name says so.
    assert!(
        token.location_name.to_uppercase().contains("TEST"),
        "refusing to run on location {:?}: not a test location",
        token.location_name
    );
    app.use_admission_mock().await;
    app.login_as(CASHIER, UserRole::Cashier);
    let hubrise = live_client(&app, &token, &proxy);
    Some(Live {
        app,
        _db: db,
        token,
        hubrise,
    })
}

impl Live {
    /// Load the test menu (the mock's seed catalog) into the location's
    /// catalog, whole.
    async fn load_test_menu(&self) {
        let catalog = self.token.catalog_id.as_deref().unwrap();
        let body =
            json!({ "name": self.token.catalog_name, "data": seed_catalog_builder().build_json() });
        let _: Value = self
            .hubrise
            .put_json(&format!("/catalogs/{catalog}"), &body)
            .await
            .expect("load the test menu");
    }

    /// What the POS does at start: callback, catalog, backfill.
    async fn start(&self) {
        start(&poller(&self.app)).await.expect("poller start");
    }

    async fn place(&self, body: Value) -> String {
        let order: Value = self
            .hubrise
            .post_json(
                &format!("/locations/{}/orders", self.token.location_id),
                &body,
            )
            .await
            .expect("place the order");
        order["id"].as_str().unwrap().to_owned()
    }

    async fn hubrise_status(&self, id: &str) -> String {
        let order: Value = self
            .hubrise
            .get_json(&format!(
                "/locations/{}/orders/{id}",
                self.token.location_id
            ))
            .await
            .unwrap();
        order["status"].as_str().unwrap().to_owned()
    }

    async fn set_status(&self, id: &str, status: &str) {
        let _: Value = self
            .hubrise
            .patch_json(
                &format!("/locations/{}/orders/{id}", self.token.location_id),
                &json!({ "status": status }),
            )
            .await
            .unwrap();
    }

    fn local(&self, hubrise_id: &str) -> i64 {
        self.app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM orders WHERE hubrise_order_id = ?1",
                [hubrise_id],
                |r| r.get(0),
            )
            .expect("the POS has the order")
    }

    fn state_of(&self, order_id: i64) -> pos_hubrise::webshop::WebshopState {
        let db = self.app.state.db.lock().unwrap();
        pos_hubrise::webshop::load(&db, order_id).unwrap().unwrap()
    }

    fn labels(&self) -> Vec<String> {
        self.app
            .fiscal_events()
            .into_iter()
            .map(|e| e.event_type)
            .collect()
    }

    async fn drain(&self) {
        let report = drain_outbox(&self.app).await;
        eprintln!("outbox pass: {report:?}");
    }
}

fn finish() {
    eprintln!("{} live HubRise calls", live_hubrise_requests());
}

#[tokio::test]
#[ignore = "live HubRise, by hand: scripts/live_hubrise.sh"]
async fn live_catalog_is_mirrored() {
    let Some(w) = live().await else { return };
    w.load_test_menu().await;
    w.start().await;

    let db = w.app.state.db.lock().unwrap();
    let row = |sku: &str| -> (String, String, i64) {
        db.query_row(
            "SELECT vat_code, vat_code_takeaway, active FROM menu_items WHERE external_id = ?1",
            [sku],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or_else(|e| panic!("{sku}: {e}"))
    };
    assert_eq!(row("MAKI-SAUMON-12"), ("B".into(), "C".into(), 1), "food");
    assert_eq!(row("ASAHI-33"), ("A".into(), "A".into(), 1), "alcohol");
    assert_eq!(row("charge:DELIVERY").2, 1, "the delivery charge");
    let registered: bool = db
        .query_row(
            "SELECT callback_registered_at IS NOT NULL FROM hubrise_connection",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    assert!(registered, "the passive callback is registered");
    drop(db);
    finish();
}

#[tokio::test]
#[ignore = "live HubRise, by hand: scripts/live_hubrise.sh"]
async fn live_prepaid_order_signed_then_cancelled_and_refunded() {
    let Some(w) = live().await else { return };
    w.load_test_menu().await;
    let body = template("deliveroo_prepaid")
        .unwrap()
        .discount("PROMO10", "Promo -10 %", 200)
        .build();
    // The payment covers the discounted total.
    let body = {
        let mut b = body;
        let total = pos_hubrise_mock::builders::order_total_cents(&b);
        b["payments"] = json!([{ "ref": "DELIVEROO", "name": "Deliveroo",
            "amount": pos_hubrise_types::money::format_cents(total) }]);
        b
    };
    let hr_id = w.place(body).await;
    w.start().await;
    let id = w.local(&hr_id);
    let state = w.state_of(id);
    assert!(state.attention.is_empty(), "{:?}", state.attention);
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "received");

    let accepted = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .expect("accept");
    assert_eq!(accepted.fiscal_state, "signed_n");
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "accepted");

    // The platform cancels; the POS finds it at its next start.
    w.set_status(&hr_id, "cancelled").await;
    w.start().await;
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");

    w.app.login_as(MANAGER, UserRole::Admin);
    let refund = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
        .await
        .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    assert_eq!(w.labels(), vec!["N", "N"]);
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "cancelled");
    finish();
}

#[tokio::test]
#[ignore = "live HubRise, by hand: scripts/live_hubrise.sh"]
async fn live_unpaid_order_registered_then_collected() {
    let Some(w) = live().await else { return };
    w.load_test_menu().await;
    let hr_id = w
        .place(template("webshop_cash_collection").unwrap().build())
        .await;
    w.start().await;
    let id = w.local(&hr_id);
    let state = w.state_of(id);
    assert!(state.attention.is_empty(), "{:?}", state.attention);

    let accepted = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .expect("accept");
    assert_eq!(accepted.fiscal_state, "signed_p");
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "accepted");

    let sale = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    assert_eq!(w.labels(), vec!["P", "N"]);
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "completed");
    finish();
}
