//! Tests of the numbered takeaway facade (engine module `takeaway`): `get_takeaway_numbering`,
//! `set_takeaway_numbering`, `get_takeaway_tag`, `set_takeaway_customer`, and the number and
//! name on `list_open_takeaways`.
//!
//! (a) Contract: the FFI records are the engine's field by field; an unknown key is a contract
//! error.
//!
//! (b) Wiring on the seeded engine: the first takeaway of a new installation is E-61, the
//! manager's first number applies to the next one, the customer round trip, the refusals.

use pos_engine::commands::takeaway as engine;
use pos_ffi::PosError;
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::records::orders::{AddOrderLineInput, CreateOrderInput, OrderType};
use pos_ffi::records::takeaway::{TakeawayNumbering, TakeawayTag};
use serde_json::json;

use crate::support::{MANAGER_INSZ, MANAGER_PIN};

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_numbering_record_is_the_engines_field_by_field() {
    let input: engine::TakeawayNumbering =
        contract_inp(&TakeawayNumbering { first_number: 70 }).unwrap();
    assert_eq!(input.first_number, 70);
    let back: TakeawayNumbering = contract_out(&input).unwrap();
    assert_eq!(back.first_number, 70);
    let defaults: TakeawayNumbering = contract_out(&engine::TakeawayNumbering::default()).unwrap();
    assert_eq!(defaults.first_number, 61);
}

#[test]
fn the_tag_record_is_the_engines_field_by_field() {
    let tag = engine::TakeawayTag {
        order_id: 7,
        day: "2026-11-06".into(),
        number: 61,
        label: "E-61".into(),
        customer_name: Some("Dupont".into()),
        customer_phone: None,
    };
    let record: TakeawayTag = contract_out(&tag).unwrap();
    assert_eq!(record.order_id, 7);
    assert_eq!(record.label, "E-61");
    assert_eq!(record.customer_name.as_deref(), Some("Dupont"));
    assert_eq!(record.customer_phone, None);
}

#[test]
fn an_unknown_key_is_a_contract_error() {
    let mut value = serde_json::to_value(TakeawayNumbering { first_number: 70 }).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<TakeawayNumbering>(value).is_err());
}

// ───────────────────────────── (b) wiring ─────────────────────────────

async fn takeaway(e: &pos_ffi::PosEngine) -> i64 {
    e.create_order(CreateOrderInput {
        order_type: OrderType::Takeaway,
        table_number: None,
        user_insz: MANAGER_INSZ.into(),
    })
    .await
    .unwrap()
    .id
}

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_the_methods_refuse_and_a_cashier_does_not_set_the_first_number() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.get_takeaway_numbering().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.get_takeaway_tag(1).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_takeaway_customer(1, None, None).await.unwrap_err()).0,
        "not_logged_in"
    );
    e.login("75061189702".into(), Some("4444".into()))
        .await
        .unwrap();
    assert_eq!(e.get_takeaway_numbering().await.unwrap().first_number, 61);
    assert_eq!(
        coded(
            &e.set_takeaway_numbering(TakeawayNumbering { first_number: 70 })
                .await
                .unwrap_err()
        )
        .0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_numbers_the_setting_and_the_customer_round_trip() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    let first = takeaway(e).await;
    let tag = e.get_takeaway_tag(first).await.unwrap().unwrap();
    assert_eq!((tag.number, tag.label.as_str()), (61, "E-61"));

    // The first number applies to the next takeaway only.
    let stored = e
        .set_takeaway_numbering(TakeawayNumbering { first_number: 100 })
        .await
        .unwrap();
    assert_eq!(stored.first_number, 100);
    assert_eq!(e.get_takeaway_numbering().await.unwrap(), stored);
    let second = takeaway(e).await;
    assert_eq!(
        e.get_takeaway_tag(second).await.unwrap().unwrap().label,
        "E-100"
    );
    let err = e
        .set_takeaway_numbering(TakeawayNumbering { first_number: 0 })
        .await
        .unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "takeaway_number_invalid");
    assert_eq!(
        (params["min"].as_str(), params["max"].as_str()),
        ("1", "999")
    );

    // The customer, and the list of the open takeaways that carries the name.
    e.add_order_line(AddOrderLineInput {
        order_id: second,
        menu_item_id: 12007,
        quantity: 1,
        modifiers_json: None,
        unit_price_cents: None,
    })
    .await
    .unwrap();
    let named = e
        .set_takeaway_customer(
            second,
            Some("  Dupont ".into()),
            Some("0470 12 34 56".into()),
        )
        .await
        .unwrap();
    assert_eq!(named.customer_name.as_deref(), Some("Dupont"));
    assert_eq!(named.customer_phone.as_deref(), Some("0470 12 34 56"));
    let open = e.list_open_takeaways().await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].label.as_deref(), Some("E-100"));
    assert_eq!(open[0].number, Some(100));
    assert_eq!(open[0].customer_name.as_deref(), Some("Dupont"));

    let err = e
        .set_takeaway_customer(second, Some("x".repeat(31)), None)
        .await
        .unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "takeaway_name_invalid");
    assert_eq!(params["max"], "30");
    let err = e
        .set_takeaway_customer(second, None, Some("nope".into()))
        .await
        .unwrap_err();
    assert_eq!(coded(&err).0, "takeaway_phone_invalid");
    // The refusals left the customer alone; an empty name and phone clear it.
    assert_eq!(e.get_takeaway_tag(second).await.unwrap().unwrap(), named);
    let cleared = e
        .set_takeaway_customer(second, Some(String::new()), None)
        .await
        .unwrap();
    assert_eq!(
        (cleared.customer_name, cleared.customer_phone),
        (None, None)
    );
    assert_eq!(e.get_takeaway_tag(9999).await.unwrap(), None);
    e.shutdown().await.unwrap();
}
