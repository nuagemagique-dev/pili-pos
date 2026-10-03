//! Price changes against the SPF's own examples: the transaction the
//! POS builds for Advanced Set UC223 (a 50 % ticket discount spread
//! over two lines at B and A) is the published one.

use std::path::PathBuf;

use chrono::Utc;
use pos_core::config::PosConfig;
use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use pos_core::vat::VatCode;
use pos_fiscal::event::{Booking, build_sale_input};
use pos_fiscal::price_change::{self, LineChange, LineChanges};
use pos_fiscal::types::{PaymentType, PriceChangeScope, PriceChangeType};
use serde_json::Value;

fn uc223() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../docs/20260821_UC_AdvancedSet_POS_FDM_1.6/UC223_signSale_EventPriceChange/UC223_signSale_EventPriceChange_GQL-Variables.json",
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Numbers as f64: the published JSON writes 20 where serde writes 20.0.
fn numeric(v: &Value) -> Value {
    match v {
        Value::Number(n) => serde_json::json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(numeric).collect()),
        Value::Object(o) => Value::Object(o.iter().map(|(k, v)| (k.clone(), numeric(v))).collect()),
        other => other.clone(),
    }
}

fn line(
    id: i64,
    product: i64,
    name: &str,
    dept: (&str, &str),
    qty: i32,
    unit: i64,
    vat: VatCode,
) -> OrderLine {
    OrderLine {
        id,
        order_id: 1,
        menu_item_id: product,
        quantity: qty,
        unit_price_cents: unit,
        vat_code: vat,
        name: name.into(),
        modifiers_json: None,
        created_at: Utc::now(),
        department_id: dept.0.into(),
        department_name: dept.1.into(),
    }
}

fn discount(amount_cents: i64) -> LineChange {
    LineChange {
        grouping_id: 1,
        id: "10".into(),
        name: "Discount50P".into(),
        scope: PriceChangeScope::Event,
        change_type: PriceChangeType::Public,
        amount_cents,
    }
}

#[test]
fn uc223_event_discount_matches_the_published_transaction() {
    let lines = vec![
        line(
            1,
            20001,
            "Carpaccio Beef",
            ("20", "Starters"),
            2,
            2000,
            VatCode::B,
        ),
        line(
            2,
            10006,
            "Dry Martini",
            ("10", "Aperitifs"),
            1,
            1200,
            VatCode::A,
        ),
    ];
    // 50 % of the ticket, spread over the lines in proportion.
    let shares = price_change::distribute(&[4000, 1200], -2600);
    let changes: LineChanges = lines
        .iter()
        .zip(shares)
        .map(|(l, s)| (l.id, vec![discount(s)]))
        .collect();
    let order = Order {
        id: 1,
        table_number: None,
        order_type: OrderType::Takeaway,
        status: OrderStatus::Open,
        user_insz: "84022899837".into(),
        created_at: Utc::now(),
        closed_at: None,
        subtotal_cents: 2600,
        total_cents: 2600,
    };
    let booking = Booking {
        period_id: "p".into(),
        date: "2024-09-02".into(),
    };
    let net = price_change::net_total_cents(&lines, &changes);
    assert_eq!(net, 2600);
    let mut sale = build_sale_input(
        &PosConfig::default(),
        &order,
        &lines,
        5000,
        &booking,
        PaymentType::CardDebit,
        net,
        0,
        None,
    );
    price_change::apply(&mut sale.transaction, &lines, &changes);

    let ours = numeric(&serde_json::to_value(&sale.transaction).unwrap());
    let published = numeric(&uc223()["data"]["transaction"]);
    assert_eq!(
        ours, published,
        "\nours: {ours:#}\npublished: {published:#}"
    );
    assert_eq!(sale.financials[0].amount, 26.0);
}

#[test]
fn without_changes_nothing_changes() {
    let lines = vec![line(1, 1, "X", ("1", "D"), 3, 110, VatCode::B)];
    let order = Order {
        id: 1,
        table_number: None,
        order_type: OrderType::Takeaway,
        status: OrderStatus::Open,
        user_insz: "84022899837".into(),
        created_at: Utc::now(),
        closed_at: None,
        subtotal_cents: 330,
        total_cents: 330,
    };
    let booking = Booking {
        period_id: "p".into(),
        date: "2024-09-02".into(),
    };
    let plain = build_sale_input(
        &PosConfig::default(),
        &order,
        &lines,
        1,
        &booking,
        PaymentType::Cash,
        330,
        0,
        None,
    );
    let mut applied = plain.clone();
    price_change::apply(&mut applied.transaction, &lines, &LineChanges::new());
    assert_eq!(
        serde_json::to_value(&plain.transaction).unwrap(),
        serde_json::to_value(&applied.transaction).unwrap()
    );
}
