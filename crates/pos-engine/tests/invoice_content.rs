//! What an invoice contains (V11-D3): the lines, the price changes and the VAT are read from
//! the archive of the signed sales, never supplied by the screen. Two sales with an A line and
//! a B line, a discount on a line and a discount on the whole ticket, signed by the admission
//! mock FDM: the preview, the stored `vat_json` and the invoices journal view must agree with
//! the `vatCalc` the FDM answered.

#![cfg(feature = "hubrise")]

use pos_engine_testkit as common;

use common::*;

use pos_core::order::OrderType;
use pos_core::user::UserRole;
use pos_db::queries::price_changes::{NewPriceChange, add_to_line};
use pos_engine::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pos_engine::commands::invoice::{do_invoice_preview, do_sign_invoice_fiscal};
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use serde_json::Value;

const OPERATOR: &str = "75061189702";
const VAT: &str = "BE0308357159";
const ITEM_A: i64 = 1;
const ITEM_B: i64 = 2;

fn change<'a>(
    id: &'a str,
    name: &'a str,
    scope: &'a str,
    grouping: i32,
    cents: i64,
) -> NewPriceChange<'a> {
    NewPriceChange {
        grouping_id: grouping,
        change_id: id,
        name,
        scope,
        change_type: "PUBLIC",
        amount_cents: cents,
    }
}

/// One sale paid in cash: `lines` are (item, quantity, changes of the line).
async fn sell(app: &TestApp, lines: &[(i64, i32, Vec<NewPriceChange<'_>>)]) -> i64 {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
    .id;
    for (item, quantity, changes) in lines {
        let line = do_add_order_line(
            &app.state,
            AddOrderLineInput {
                order_id: order,
                menu_item_id: *item,
                quantity: *quantity,
                modifiers_json: None,
                unit_price_cents: None,
            },
        )
        .unwrap();
        if !changes.is_empty() {
            add_to_line(&app.state.db.lock().unwrap(), line.id, changes).unwrap();
        }
    }
    let r = do_checkout(&app.state, order, CheckoutPaymentMethod::Cash)
        .await
        .unwrap();
    assert!(r.success, "{:?}", r.error);
    r.fiscal_event_id.unwrap()
}

/// What the FDM answered as `vatCalc` for a sale, per label, in cents: (base, vat, total).
fn fdm_vat(app: &TestApp, event_id: i64) -> Vec<(String, i64, i64, i64)> {
    let event = app
        .fiscal_events()
        .into_iter()
        .find(|e| e.id == event_id)
        .unwrap();
    let response: Value = serde_json::from_str(&event.fdm_response_json.unwrap()).unwrap();
    let cents = |v: &Value| (v.as_f64().unwrap() * 100.0).round() as i64;
    response["vatCalc"]
        .as_array()
        .expect("the FDM answers vatCalc")
        .iter()
        .map(|v| {
            (
                v["label"].as_str().unwrap().to_string(),
                cents(&v["taxableAmount"]),
                cents(&v["vatAmount"]),
                cents(&v["totalAmount"]),
            )
        })
        .collect()
}

#[tokio::test]
async fn an_invoice_is_built_from_the_archive_of_its_sales() {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, UserRole::Admin);
    app.state
        .db
        .lock()
        .unwrap()
        .execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active)
                VALUES (1, 1, 1000, 'A', 0, 1), (2, 1, 500, 'B', 1, 1);",
        )
        .unwrap();
    let customer = app.save_customer(VAT, "Client Test SRL");

    // Sale 1: a beer (A) with a 1.00 discount on the line and a 1.00 discount of the ticket,
    // and a dish (B) with 0.50 of the same ticket discount.
    let first = sell(
        &app,
        &[
            (
                ITEM_A,
                1,
                vec![
                    change("L1", "Promo biere", "LINE", 1, -100),
                    change("E1", "Remise ticket", "EVENT", 2, -100),
                ],
            ),
            (
                ITEM_B,
                1,
                vec![change("E1", "Remise ticket", "EVENT", 2, -50)],
            ),
        ],
    )
    .await;
    // Sale 2: two beers with 0.60 of the same ticket discount, and a dish with no change.
    let second = sell(
        &app,
        &[
            (
                ITEM_A,
                2,
                vec![change("E1", "Remise ticket", "EVENT", 2, -60)],
            ),
            (ITEM_B, 1, vec![]),
        ],
    )
    .await;

    let preview = do_invoice_preview(&app.state, &[first, second]).unwrap();
    assert!(preview.problems.is_empty(), "{:?}", preview.problems);

    // Product lines first (the LINE discount is inside its line), then one row per reason and
    // VAT label for the ticket discounts, summed over the two sales.
    let rows: Vec<(String, i64, i64, String)> = preview
        .lines
        .iter()
        .map(|l| {
            (
                l.description.clone(),
                l.quantity_milli,
                l.total_cents,
                l.vat_label.clone(),
            )
        })
        .collect();
    let name_a = preview.lines[0].description.clone();
    let name_b = preview.lines[1].description.clone();
    assert_eq!(
        rows,
        [
            (name_a.clone(), 1000, 900, "A".to_string()),
            (name_b.clone(), 1000, 500, "B".to_string()),
            (name_a, 2000, 2000, "A".to_string()),
            (name_b, 1000, 500, "B".to_string()),
            ("Remise ticket".to_string(), 1000, -160, "A".to_string()),
            ("Remise ticket".to_string(), 1000, -50, "B".to_string()),
        ]
    );
    let expected_total = 900 + 500 + 2000 + 500 - 160 - 50;
    assert_eq!(preview.total_cents, expected_total);
    assert_eq!(
        preview.total_cents,
        preview.lines.iter().map(|l| l.total_cents).sum::<i64>()
    );
    assert_eq!(
        preview.sales.iter().map(|s| s.total_cents).sum::<i64>(),
        expected_total
    );

    // The VAT per label is the sum of what the FDM computed for each sale.
    let mut expected: std::collections::BTreeMap<String, (i64, i64, i64)> = Default::default();
    for event in [first, second] {
        for (label, base, vat, total) in fdm_vat(&app, event) {
            let e = expected.entry(label).or_default();
            e.0 += base;
            e.1 += vat;
            e.2 += total;
        }
    }
    let got: std::collections::BTreeMap<String, (i64, i64, i64)> = preview
        .vat
        .iter()
        .map(|v| (v.label.clone(), (v.base_cents, v.vat_cents, v.total_cents)))
        .collect();
    assert_eq!(got, expected);
    assert_eq!(
        preview
            .vat
            .iter()
            .map(|v| v.rate_hundredths)
            .collect::<Vec<_>>(),
        [2100, 1200]
    );
    // (The VAT is the FDM's own figure and is shown as it answered: the admission mock
    // computes `vatCalc` from the prices before price changes, so its totals are compared
    // with each other here, never with the net total of the lines.)

    // Signed: the invoice stores the same figures and the journal view exposes them.
    let signed = do_sign_invoice_fiscal(&app.state, vec![first, second], customer, OPERATOR.into())
        .await
        .unwrap();
    assert!(signed.success, "{:?}", signed.error);
    let no = signed.invoice_no.unwrap();
    let db = app.state.db.lock().unwrap();
    let (total, vat_json): (i64, String) = db
        .query_row(
            "SELECT total_cents, vat_json FROM invoices WHERE invoice_no = ?1",
            [&no],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(total, expected_total);
    let stored: Value = serde_json::from_str(&vat_json).unwrap();
    for (label, (base, vat, total)) in &expected {
        assert_eq!(stored[label]["base"], *base, "{vat_json}");
        assert_eq!(stored[label]["vat"], *vat, "{vat_json}");
        assert_eq!(stored[label]["total"], *total, "{vat_json}");
    }
    let (base_a, vat_a, base_b, vat_b): (i64, i64, i64, i64) = db
        .query_row(
            "SELECT base_a_cents, vat_a_cents, base_b_cents, vat_b_cents
             FROM v_invoice_journal WHERE invoice_no = ?1",
            [&no],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!((base_a, vat_a), (expected["A"].0, expected["A"].1));
    assert_eq!((base_b, vat_b), (expected["B"].0, expected["B"].1));
}
