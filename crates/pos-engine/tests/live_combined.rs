//! Online orders end to end, live on both sides: the orders come from
//! the real `HubRise` API (test location), and their events are signed
//! by a live FDM (the module2 cloud simulator).
//!
//! Ignored by default and never run in CI. Run it by hand, one test at
//! a time, after `scripts/live_hubrise_login.sh`:
//!
//! ```bash
//! scripts/live_combined.sh live_prepaid_order_signed_by_the_fdm_then_refunded
//! ```
//!
//! Each test places one order on `HubRise`'s free plan (5 a month), and
//! signs two events on the FDM. The variables of `live_fdm.rs` and
//! `live_hubrise.rs` apply.
//!
//! Only what holds on any FDM is asserted: each event is signed (its
//! response passed the POS's own response check), an N carries a short
//! signature, the cost center is the platform's or the webshop's with
//! the `HubRise` order as its reference, the refund cites the sale.

#![cfg(feature = "hubrise")]
#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]
#![expect(
    clippy::print_stderr,
    reason = "a live run reports a skipped run on stderr"
)]

use pos_engine_testkit as common;

use common::live_hubrise::{Live, LiveFdm, finish};
use common::{FiscalEventRow, live_url, record_as};
use pos_core::user::UserRole;
use pos_engine::commands::checkout::PaymentPart;
use pos_engine::commands::webshop::{
    do_accept_webshop_order, do_handover_webshop_order, do_refund_webshop_order,
};
use pos_hubrise_mock::builders::template;
use serde_json::{Value, json};

/// The operator module2's test setup knows (as in `live_fdm.rs`), a
/// manager: the refund needs one.
const INSZ: &str = "75061189702";

async fn live() -> Option<Live> {
    let Some(url) = live_url() else {
        eprintln!("PILI_LIVE_FDM_URL not set: skipping (see scripts/live_combined.sh)");
        return None;
    };
    let w = common::live_hubrise::live(LiveFdm::Live(url)).await?;
    w.app.login_as(INSZ, UserRole::Admin);
    Some(w)
}

/// The first value under `key`, anywhere in `value`.
fn find<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => map
            .get(key)
            .or_else(|| map.values().find_map(|v| find(v, key))),
        Value::Array(items) => items.iter().find_map(|v| find(v, key)),
        _ => None,
    }
}

fn request(e: &FiscalEventRow) -> Value {
    serde_json::from_str(e.event_data_json.as_deref().unwrap()).unwrap()
}

/// The event's cost center: `(type, reference)`.
fn cost_center(e: &FiscalEventRow) -> (String, String) {
    let request = request(e);
    let cc = find(&request, "costCenter").expect("a cost center");
    (
        cc["type"].as_str().unwrap().to_owned(),
        cc["reference"].as_str().unwrap().to_owned(),
    )
}

/// The payment types of the event's financial lines.
fn payment_types(e: &FiscalEventRow) -> Vec<String> {
    request(e)["financials"]
        .as_array()
        .expect("financial lines")
        .iter()
        .map(|p| p["type"].as_str().unwrap().to_owned())
        .collect()
}

fn signed_n(e: &FiscalEventRow) {
    assert_eq!(e.event_type, "N");
    assert!(
        e.short_signature.as_deref().is_some_and(|s| !s.is_empty()),
        "an N carries the FDM's short signature"
    );
}

#[tokio::test]
#[ignore = "live HubRise and live FDM, by hand: scripts/live_combined.sh"]
async fn live_prepaid_order_signed_by_the_fdm_then_refunded() {
    let Some(w) = live().await else { return };
    w.load_test_menu().await;
    let body = template("deliveroo_prepaid")
        .unwrap()
        .discount("PROMO10", "Promo -10 %", 200)
        .build();
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

    record_as("signSale_hubrise_prepaid");
    let accepted = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .expect("accept");
    w.app.save_ticket_numbers();
    assert_eq!(accepted.fiscal_state, "signed_n");
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "accepted");

    // The platform cancels; the POS finds it at its next start.
    w.set_status(&hr_id, "cancelled").await;
    w.start().await;
    assert_eq!(w.state_of(id).fiscal_state, "refund_pending");

    record_as("signSale_hubrise_refund");
    let refund = do_refund_webshop_order(&w.app.state, id, INSZ, None)
        .await
        .unwrap();
    w.app.save_ticket_numbers();
    assert!(refund.success, "{:?}", refund.error);
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "cancelled");

    let events = w.app.fiscal_events();
    let [sale, refunded] = events.as_slice() else {
        panic!("two events expected, got {events:?}");
    };
    signed_n(sale);
    signed_n(refunded);
    assert_eq!(
        cost_center(sale),
        ("PLATFORM".to_string(), hr_id.clone()),
        "the platform's order, by its HubRise id"
    );
    assert_ne!(payment_types(sale).len(), 0);
    assert!(
        payment_types(sale).iter().all(|t| t == "APP"),
        "{:?}",
        payment_types(sale)
    );
    assert!(
        find(&request(sale), "priceChanges").is_some(),
        "the discount goes as price changes"
    );
    // The refund cites the sale (FAQ Q301 a) and goes back by the app.
    let back = request(refunded);
    let cited = &back["fdmRef"];
    assert_eq!(cited["eventLabel"], "N", "{cited}");
    assert_eq!(cited["eventCounter"].as_i64(), sale.event_counter);
    assert_eq!(cited["totalCounter"].as_i64(), sale.total_counter);
    assert_ne!(payment_types(refunded).len(), 0);
    assert!(payment_types(refunded).iter().all(|t| t == "APP"));
    finish();
}

#[tokio::test]
#[ignore = "live HubRise and live FDM, by hand: scripts/live_combined.sh"]
async fn live_unpaid_order_registered_then_sold_by_the_fdm() {
    let Some(w) = live().await else { return };
    w.load_test_menu().await;
    let hr_id = w
        .place(template("webshop_cash_collection").unwrap().build())
        .await;
    w.start().await;
    let id = w.local(&hr_id);
    let state = w.state_of(id);
    assert!(state.attention.is_empty(), "{:?}", state.attention);

    record_as("signOrder_hubrise_unpaid");
    let accepted = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .expect("accept");
    w.app.save_ticket_numbers();
    assert_eq!(accepted.fiscal_state, "signed_p");
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "accepted");

    record_as("signSale_hubrise_handover");
    let sale = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    w.app.save_ticket_numbers();
    assert!(sale.success, "{:?}", sale.error);
    w.drain().await;
    assert_eq!(w.hubrise_status(&hr_id).await, "completed");

    let events = w.app.fiscal_events();
    let [order, sold] = events.as_slice() else {
        panic!("two events expected, got {events:?}");
    };
    assert_eq!(order.event_type, "P");
    signed_n(sold);
    // The N closes the P: same cost center, same reference (FAQ Q203 b).
    let registered = cost_center(order);
    assert_eq!(registered, ("WEBSHOP".to_string(), hr_id.clone()));
    assert_eq!(cost_center(sold), registered);
    assert_ne!(payment_types(sold).len(), 0);
    assert!(
        payment_types(sold).iter().all(|t| t == "CASH"),
        "{:?}",
        payment_types(sold)
    );
    finish();
}
