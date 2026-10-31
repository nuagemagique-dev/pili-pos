//! Tests of the gift vouchers facade (engine module `gift_vouchers)`: `get_gift_voucher_settings`,
//! `set_gift_voucher_settings`, `add_gift_voucher_line`, `lookup_gift_voucher`,
//! `list_gift_vouchers`, `print_gift_voucher`, and the new `PaymentPart` and `CheckoutResult` fields.
//!
//! (a) Contract: the FFI records are the engine's field by field; an unknown key is a contract
//! error; a payment part without a voucher id still reads (the field defaults).
//!
//! (b) Wiring on the seeded engine: the setting round trip and its refusals, a voucher sold,
//! looked up, listed, paid with and printed again.

use pos_engine::commands::gift_vouchers as engine;
use pos_ffi::records::common::{
    GiftVoucherPrint, PaymentMethod, PaymentPart, SlipOutcome, VatCode, contract_inp, contract_out,
};
use pos_ffi::records::gift_vouchers::{GiftVoucherMovement, GiftVoucherSettings, GiftVoucherView};
use pos_ffi::records::orders::{CreateOrderInput, OrderType};
use pos_ffi::{PosEngine, PosError};
use serde_json::json;

use crate::support::{MANAGER_INSZ, MANAGER_PIN};

/// Dry Martini of the seed (10,00 EUR is not its price: it costs 12,00).
const MARTINI: i64 = 10006;

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn on() -> GiftVoucherSettings {
    GiftVoucherSettings {
        enabled: true,
        validity_months: 24,
        max_amount_cents: 20_000,
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_records_are_the_engines_field_by_field() {
    let input: engine::GiftVoucherSettings = contract_inp(&on()).unwrap();
    assert!(input.enabled);
    assert_eq!(
        (input.validity_months, input.max_amount_cents),
        (24, 20_000)
    );
    let back: GiftVoucherSettings = contract_out(&input).unwrap();
    assert_eq!(back, on());

    let view = engine::GiftVoucherView {
        id: 4,
        code_masked: "**** **** 9095".into(),
        face_value_cents: 5000,
        balance_cents: 3150,
        issued_on: "2026-10-17".into(),
        expires_on: "2027-10-17".into(),
        status: "active".into(),
        training: false,
        movements: vec![engine::GiftVoucherMovement {
            kind: "redeem".into(),
            amount_cents: -1850,
            date: "2026-10-17".into(),
            ticket: "N 12".into(),
        }],
    };
    let record: GiftVoucherView = contract_out(&view).unwrap();
    assert_eq!(record.code_masked, "**** **** 9095");
    assert_eq!(
        record.movements,
        vec![GiftVoucherMovement {
            kind: "redeem".into(),
            amount_cents: -1850,
            date: "2026-10-17".into(),
            ticket: "N 12".into()
        }]
    );

    let print: GiftVoucherPrint = contract_out(&engine::GiftVoucherPrint {
        voucher_id: 4,
        kind: "balance".into(),
        outcome: pos_engine::commands::bar_slip::SlipOutcome::Printed,
    })
    .unwrap();
    assert_eq!(
        print,
        GiftVoucherPrint {
            voucher_id: 4,
            kind: "balance".into(),
            outcome: SlipOutcome::Printed
        }
    );
}

#[test]
fn an_unknown_key_is_a_contract_error_and_the_part_field_defaults() {
    let mut value = serde_json::to_value(on()).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<GiftVoucherSettings>(value).is_err());

    let part: PaymentPart =
        serde_json::from_value(json!({"method": "gift_voucher", "amount_cents": 500})).unwrap();
    assert_eq!(part.method, PaymentMethod::GiftVoucher);
    assert_eq!(part.gift_voucher_id, None);
    let named: pos_fiscal::payment::PaymentPart = contract_inp(&PaymentPart {
        method: PaymentMethod::GiftVoucher,
        amount_cents: Some(500),
        eft_payment_id: None,
        gift_voucher_id: Some(7),
    })
    .unwrap();
    assert_eq!(
        named,
        pos_fiscal::payment::PaymentPart::gift_voucher(500, 7)
    );
}

// ───────────────────────────── (b) wiring ─────────────────────────────

async fn manager(e: &PosEngine) {
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();
}

async fn takeaway(e: &PosEngine) -> i64 {
    e.create_order(CreateOrderInput {
        order_type: OrderType::Takeaway,
        table_number: None,
        user_insz: MANAGER_INSZ.into(),
    })
    .await
    .unwrap()
    .id
}

fn cash() -> PaymentPart {
    PaymentPart {
        method: PaymentMethod::Cash,
        amount_cents: None,
        eft_payment_id: None,
        gift_voucher_id: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse_and_a_cashier_does_not_set() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.get_gift_voucher_settings().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_gift_voucher_settings(on()).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.add_gift_voucher_line(1, 2000).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(
            &e.lookup_gift_voucher("123456789095".into())
                .await
                .unwrap_err()
        )
        .0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.list_gift_vouchers(None).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.print_gift_voucher(1).await.unwrap_err()).0,
        "not_logged_in"
    );
    e.login("75061189702".into(), Some("4444".into()))
        .await
        .unwrap();
    assert!(!e.get_gift_voucher_settings().await.unwrap().enabled);
    assert_eq!(
        coded(&e.set_gift_voucher_settings(on()).await.unwrap_err()).0,
        "forbidden"
    );
    assert_eq!(
        coded(&e.list_gift_vouchers(None).await.unwrap_err()).0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_setting_round_trips_and_its_refusals_are_coded() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;
    let defaults = e.get_gift_voucher_settings().await.unwrap();
    assert_eq!(
        defaults,
        GiftVoucherSettings {
            enabled: false,
            validity_months: 12,
            max_amount_cents: 50_000
        }
    );
    assert_eq!(e.set_gift_voucher_settings(on()).await.unwrap(), on());
    assert_eq!(e.get_gift_voucher_settings().await.unwrap(), on());
    let mut bad = on();
    bad.validity_months = 6;
    let err = e.set_gift_voucher_settings(bad).await.unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "gift_voucher_settings_invalid");
    assert_eq!(params["field"], "validity_months");
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_voucher_is_sold_listed_used_and_printed_again() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    manager(e).await;
    let order = takeaway(e).await;
    assert_eq!(
        coded(&e.add_gift_voucher_line(order, 5000).await.unwrap_err()).0,
        "gift_vouchers_disabled"
    );
    e.set_gift_voucher_settings(on()).await.unwrap();
    let err = e.add_gift_voucher_line(order, 30_000).await.unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "gift_voucher_amount_invalid");
    assert_eq!(params["max"], "200,00");
    let line = e.add_gift_voucher_line(order, 5000).await.unwrap();
    assert_eq!((line.quantity, line.unit_price_cents), (1, 5000));
    assert_eq!(line.vat_code, VatCode::X);
    assert_eq!(line.department_id, "GV");
    assert_eq!(
        coded(&e.update_order_line_quantity(line.id, 2).await.unwrap_err()).0,
        "gift_voucher_line_fixed"
    );

    let sale = e.checkout(order, vec![cash()], None).await.unwrap();
    assert!(sale.success, "{:?}", sale.error);
    assert_eq!(sale.gift_voucher_prints.len(), 1);
    assert_eq!(sale.gift_voucher_prints[0].kind, "voucher");

    // Switching off is refused while the sold voucher can still be redeemed.
    let mut off = on();
    off.enabled = false;
    let err = e.set_gift_voucher_settings(off).await.unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "gift_vouchers_outstanding");
    assert_eq!(params["count"], "1");
    assert!(e.get_gift_voucher_settings().await.unwrap().enabled);

    let listed = e.list_gift_vouchers(None).await.unwrap();
    assert_eq!(listed.len(), 1);
    let voucher = &listed[0];
    assert_eq!(
        (voucher.face_value_cents, voucher.balance_cents),
        (5000, 5000)
    );
    assert_eq!(voucher.status, "active");
    assert!(voucher.code_masked.starts_with("**** **** "));
    // A well-formed code nobody has, and a malformed one.
    assert_eq!(
        coded(
            &e.lookup_gift_voucher("000000009797".into())
                .await
                .unwrap_err()
        )
        .0,
        "gift_voucher_not_found"
    );
    assert_eq!(
        coded(&e.lookup_gift_voucher("12".into()).await.unwrap_err()).0,
        "gift_voucher_code_invalid"
    );

    // The voucher pays a Dry Martini (12,00 EUR) in part and the rest is cash.
    let second = takeaway(e).await;
    e.add_order_line(pos_ffi::records::orders::AddOrderLineInput {
        order_id: second,
        menu_item_id: MARTINI,
        quantity: 1,
        modifiers_json: None,
        unit_price_cents: None,
    })
    .await
    .unwrap();
    let paid = e
        .checkout(
            second,
            vec![
                PaymentPart {
                    method: PaymentMethod::GiftVoucher,
                    amount_cents: Some(500),
                    eft_payment_id: None,
                    gift_voucher_id: Some(voucher.id),
                },
                cash(),
            ],
            None,
        )
        .await
        .unwrap();
    assert!(paid.success, "{:?}", paid.error);
    assert_eq!(paid.gift_voucher_prints[0].kind, "balance");
    let after = e.list_gift_vouchers(Some("active".into())).await.unwrap();
    assert_eq!(after[0].balance_cents, 4500);

    // A duplicate prints on the test printer.
    assert_eq!(
        e.print_gift_voucher(voucher.id).await.unwrap(),
        SlipOutcome::Printed
    );
    assert_eq!(
        coded(&e.print_gift_voucher(9999).await.unwrap_err()).0,
        "gift_voucher_not_found"
    );
    e.shutdown().await.unwrap();
}
