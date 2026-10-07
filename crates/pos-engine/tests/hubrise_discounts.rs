//! Discounts on online orders: each is spread over the product lines
//! (the cent to the largest share) and sent as a priceChange of scope
//! EVENT on every line it touches (spec v2.2, Advanced Set UC223).
//! `vats[].price` stays the price before it, `lineTotal` and
//! `transactionTotal` are after it, the payment is the net amount, and
//! a refund or a closing carries the same changes negated.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::hubrise::{HubriseMock, connected_app, drain_outbox, poller};
use common::{PrinterSink, TestApp};
use pos_core::user::UserRole;
use pos_engine::commands::checkout::PaymentPart;
use pos_engine::commands::errors;
use pos_engine::commands::refund::{RefundItem, do_refund_checkout_with};
use pos_engine::commands::report::do_generate_x_report;
use pos_engine::commands::webshop::{
    do_accept_webshop_order, do_close_cancelled_webshop_order, do_handover_webshop_order,
    do_refund_webshop_order,
};
use pos_hubrise::poller::{poll_once, start};
use pos_hubrise_mock::builders::template;
use serde_json::{Value, json};

const CASHIER: &str = "12345678958";
const MANAGER: &str = "85073003328";

struct World {
    app: TestApp,
    _db: tempfile::TempDir,
    mock: HubriseMock,
    _printer: PrinterSink,
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
        _printer: printer,
    }
}

impl World {
    /// Place `body` and let the POS pick it up.
    async fn order(&self, body: Value) -> (i64, String) {
        let hr = self.mock.inject_body(body);
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

    fn request(&self, event_id: i64) -> Value {
        let row = self
            .app
            .fiscal_events()
            .into_iter()
            .find(|e| e.id == event_id)
            .expect("event");
        serde_json::from_str(row.event_data_json.as_deref().unwrap()).unwrap()
    }

    fn last_request(&self, label: &str) -> Value {
        let row = self
            .app
            .fiscal_events()
            .into_iter()
            .rfind(|e| e.event_type == label)
            .expect("event");
        serde_json::from_str(row.event_data_json.as_deref().unwrap()).unwrap()
    }

    fn total_cents(&self, order_id: i64) -> i64 {
        self.app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT total_cents FROM orders WHERE id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap()
    }

    fn n_event(&self, order_id: i64) -> i64 {
        self.app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT n_event_id FROM webshop_orders WHERE order_id = ?1",
                [order_id],
                |r| r.get(0),
            )
            .unwrap()
    }
}

/// Deliveroo: 2 x Maki saumon (10,00) + 1 Coca (3,00), 2,30 off.
fn deliveroo_discounted() -> Value {
    pos_hubrise_mock::builders::OrderBuilder::new("Deliveroo", "delivery")
        .item("MAKI-SAUMON-12", "Maki saumon", 1000, 2)
        .item("COCA-33", "Coca-Cola 33cl", 300, 1)
        .discount("PROMO10", "Promo -10 %", 230)
        .paid_by("DELIVEROO", "Deliveroo")
        .build()
}

fn cents(v: &Value) -> i64 {
    (v.as_f64().expect("amount") * 100.0).round() as i64
}

/// (price before, Σ changes, lineTotal) of each transaction line.
fn lines(request: &Value) -> Vec<(i64, i64, i64)> {
    request["transaction"]["transactionLines"]
        .as_array()
        .expect("transaction lines")
        .iter()
        .map(|l| {
            let vat = &l["mainProduct"]["vats"][0];
            let changes: i64 = vat["priceChanges"]
                .as_array()
                .map(|c| c.iter().map(|c| cents(&c["amount"])).sum())
                .unwrap_or(0);
            (cents(&vat["price"]), changes, cents(&l["lineTotal"]))
        })
        .collect()
}

fn paid(request: &Value) -> i64 {
    request["financials"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| cents(&f["amount"]))
        .sum()
}

#[tokio::test]
async fn a_discounted_sale_sends_the_discount_as_price_changes() {
    let w = world().await;
    let (id, _) = w.order(deliveroo_discounted()).await;
    let state = {
        let db = w.app.state.db.lock().unwrap();
        pos_hubrise::webshop::load(&db, id).unwrap().unwrap()
    };
    assert!(state.attention.is_empty(), "{:?}", state.attention);

    let sale = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    let n = w.request(sale.fiscal_event_id);

    // 20,00 and 3,00 before, 2,00 and 0,30 off.
    let mut got = lines(&n);
    got.sort();
    assert_eq!(got, vec![(300, -30, 270), (2000, -200, 1800)]);
    assert_eq!(cents(&n["transaction"]["transactionTotal"]), 2070);
    assert_eq!(paid(&n), 2070, "the payment is what the customer paid");
    assert_eq!(w.total_cents(id), 2070);

    let change = n["transaction"]["transactionLines"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|l| l["mainProduct"]["vats"][0]["priceChanges"].get(0).cloned())
        .unwrap();
    assert_eq!(change["groupingId"], 1);
    assert_eq!(change["id"], "PROMO10");
    assert_eq!(change["name"], "Promo -10 %");
    assert_eq!(change["scope"], "EVENT");
    assert_eq!(change["type"], "PUBLIC");

    // The ticket shows the discount under each line it touches.
    let ticket = sale.sale.and_then(|s| s.ticket).expect("ticket");
    let promo: Vec<f64> = ticket
        .lines
        .iter()
        .filter(|l| l.description == "Promo -10 %")
        .map(|l| l.total)
        .collect();
    assert_eq!(promo.len(), 2);
    let printed: f64 = ticket.lines.iter().map(|l| l.total).sum();
    assert!((printed - 20.70).abs() < 1e-9, "{printed}");
    assert!((ticket.total - 20.70).abs() < 1e-9);
}

#[tokio::test]
async fn the_p_and_the_handover_n_carry_the_discount() {
    let w = world().await;
    let body = template("webshop_cash_collection")
        .unwrap()
        .discount("WELCOME", "Bienvenue", 100)
        .build();
    let (id, hr_id) = w.order(body).await;
    let p = do_accept_webshop_order(&w.app.state, id, None)
        .await
        .unwrap();
    assert_eq!(p.fiscal_state, "signed_p");
    let order = w.last_request("P");
    let before: i64 = lines(&order).iter().map(|l| l.0).sum();
    assert_eq!(
        cents(&order["transaction"]["transactionTotal"]),
        before - 100
    );

    // The customer pays the net amount, in cash.
    let net = w.total_cents(id);
    assert_eq!(net, before - 100);
    let sale = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
        .await
        .unwrap();
    assert!(sale.success, "{:?}", sale.error);
    let n = w.request(sale.fiscal_event_id.unwrap());
    assert_eq!(n["costCenter"]["reference"], hr_id.as_str());
    assert_eq!(cents(&n["transaction"]["transactionTotal"]), net);
    let changes: i64 = lines(&n).iter().map(|l| l.1).sum();
    assert_eq!(changes, -100);
}

#[tokio::test]
async fn a_cancelled_discounted_p_closes_at_zero() {
    let w = world().await;
    let body = template("webshop_cash_collection")
        .unwrap()
        .discount("WELCOME", "Bienvenue", 100)
        .build();
    let (id, hr_id) = w.order(body).await;
    assert!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .is_ok()
    );
    drain_outbox(&w.app).await;
    w.mock.update(&hr_id, json!({ "status": "cancelled" }));
    poll_once(&poller(&w.app)).await.unwrap();

    let closed = do_close_cancelled_webshop_order(&w.app.state, id)
        .await
        .unwrap();
    assert!(closed.success, "{:?}", closed.error);
    let n = w.request(closed.fiscal_event_id.unwrap());
    let got = lines(&n);
    // Every line and its correction, each discount and its reversal.
    assert_eq!(got.iter().map(|l| l.1).sum::<i64>(), 0);
    assert!(got.iter().any(|l| l.1 < 0) && got.iter().any(|l| l.1 > 0));
    assert_eq!(got.iter().map(|l| l.2).sum::<i64>(), 0);
    assert_eq!(cents(&n["transaction"]["transactionTotal"]), 0);
    assert_eq!(w.total_cents(id), 0);
}

#[tokio::test]
async fn a_refund_gives_back_the_net_amount() {
    let w = world().await;
    let (id, hr_id) = w.order(deliveroo_discounted()).await;
    assert!(
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .is_ok()
    );
    let sale = w.n_event(id);
    drain_outbox(&w.app).await;
    w.mock.update(&hr_id, json!({ "status": "cancelled" }));
    poll_once(&poller(&w.app)).await.unwrap();

    w.app.login_as(MANAGER, UserRole::Admin);
    // By product, a discounted line is refused: its share is the whole.
    let maki: i64 = w
        .app
        .state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT menu_item_id FROM order_lines WHERE order_id = ?1 ORDER BY id LIMIT 1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    let by_product = do_refund_checkout_with(
        &w.app.state,
        sale,
        vec![RefundItem {
            line_id: None,
            menu_item_id: maki,
            quantity: 1,
        }],
        MANAGER.into(),
        None,
    )
    .await;
    let refused = match by_product {
        Err(e) => e,
        Ok(r) => r.error.unwrap_or_default(),
    };
    assert!(
        refused.starts_with(errors::hubrise::REFUND_DISCOUNTED_LINE),
        "{refused}"
    );

    let refund = do_refund_webshop_order(&w.app.state, id, MANAGER, None)
        .await
        .unwrap();
    assert!(refund.success, "{:?}", refund.error);
    let back = w.request(refund.fiscal_event_id.unwrap());
    let mut got = lines(&back);
    got.sort();
    assert_eq!(got, vec![(-2000, 200, -1800), (-300, 30, -270)]);
    assert_eq!(cents(&back["transaction"]["transactionTotal"]), -2070);
    assert_eq!(paid(&back), -2070);

    // The day nets to zero, and the turnover lists the discount both
    // ways under its id.
    let x = do_generate_x_report(&w.app.state, MANAGER.into())
        .await
        .unwrap();
    assert!(x.success, "{:?}", x.error);
    let report = w.last_request("R");
    let turnover = &report["turnover"];
    let n_total: i64 = turnover["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|t| t["eventLabel"] == "N")
        .map(|t| cents(&t["amount"]))
        .sum();
    assert_eq!(n_total, 0);
    let promo = turnover["priceChanges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "PROMO10")
        .expect("the discount in the turnover");
    let (neg, pos): (i64, i64) = promo["amount"]
        .as_array()
        .unwrap()
        .iter()
        .fold((0, 0), |(n, p), a| {
            (n + cents(&a["negative"]), p + cents(&a["positive"]))
        });
    assert_eq!((neg, pos), (-230, 230));
}
