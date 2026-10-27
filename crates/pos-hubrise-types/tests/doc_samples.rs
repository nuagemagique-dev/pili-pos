//! The JSON samples of the `HubRise` API documentation (copied to `tests/fixtures/`)
//! parse into the wire types, keep their `ref` fields, and survive a
//! round trip.

use pos_hubrise_types::money::parse_cents;
use pos_hubrise_types::{CatalogData, OptionList, Order, OrderStatus, Product, ServiceType, Sku};
use serde_json::Value;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn the_retrieved_order_keeps_every_ref() {
    let order: Order = serde_json::from_str(&fixture("order_retrieve_doc.json")).unwrap();
    assert_eq!(order.ref_.as_deref(), Some("17654321"));
    assert_eq!(order.status, OrderStatus::InDelivery);
    assert_eq!(order.service_type, Some(ServiceType::Delivery));
    assert_eq!(order.items.len(), 2);
    assert_eq!(order.items[0].sku_ref.as_deref(), Some("3"));
    assert_eq!(order.items[1].options[0].ref_.as_deref(), Some("31"));
    assert_eq!(order.items[1].options[0].quantity, 1);
    assert!(order.items[0].deal_line.is_some());
    assert_eq!(order.deals["0"].ref_.as_deref(), Some("10"));
    assert_eq!(order.discounts[0].ref_.as_deref(), Some("45"));
    assert_eq!(order.charges[0].ref_.as_deref(), Some("DEL"));
    assert_eq!(order.payments[0].ref_.as_deref(), Some("CSH"));
    assert!(order.delivery.is_some());
    assert_eq!(parse_cents(order.total.as_deref().unwrap()), Ok(1890));
}

#[test]
fn the_created_order_parses_with_its_refs() {
    // The create body has no id / location / created_at: add them as
    // HubRise does on creation.
    let mut body: Value = serde_json::from_str(&fixture("order_create_doc.json")).unwrap();
    body["id"] = "o1".into();
    body["location_id"] = "l1".into();
    body["created_at"] = "2021-06-24T17:07:53+02:00".into();
    let order: Order = serde_json::from_value(body).unwrap();
    assert_eq!(order.items[0].sku_ref.as_deref(), Some("MAR-SM"));
    assert_eq!(order.items[0].options[0].ref_.as_deref(), Some("BBQ"));
    assert_eq!(order.payments[0].ref_.as_deref(), Some("PP"));
}

#[test]
fn a_listed_order_with_few_fields_parses() {
    let orders: Vec<Order> = serde_json::from_str(&fixture("order_list_doc.json")).unwrap();
    assert_eq!(orders[0].status, OrderStatus::New);
    assert_eq!(orders[0].private_ref.as_deref(), Some("3345"));
}

#[test]
fn the_order_round_trips_with_ref_on_the_wire() {
    let raw = fixture("order_retrieve_doc.json");
    let order: Order = serde_json::from_str(&raw).unwrap();
    let back = serde_json::to_value(&order).unwrap();
    assert_eq!(back["ref"], "17654321");
    assert_eq!(back["items"][1]["options"][0]["ref"], "31");
    assert!(back.get("ref_").is_none(), "no ref_ on the wire");
    let again: Order = serde_json::from_value(back).unwrap();
    assert_eq!(again.items[1].options[0].ref_.as_deref(), Some("31"));
}

#[test]
fn nulls_count_as_empty() {
    let order: Order = serde_json::from_value(serde_json::json!({
        "id": "o1", "location_id": "l1", "status": "new",
        "created_at": "2021-06-24T17:07:53+02:00",
        "coupon_codes": null, "items": [{
            "product_name": "X", "price": "1.00 EUR", "quantity": "1",
            "options": null, "deleted": null
        }],
        "discounts": null, "charges": null, "payments": null, "deals": null
    }))
    .unwrap();
    assert_eq!(order.coupon_codes.len(), 0);
    assert!(!order.items[0].deleted);
    assert!(order.items[0].options.is_empty());
}

#[test]
fn the_catalog_upload_sample_parses() {
    let body: Value = serde_json::from_str(&fixture("catalog_create_doc.json")).unwrap();
    let data: CatalogData = serde_json::from_value(body["data"].clone()).unwrap();
    assert_eq!(data.categories[1].parent_ref.as_deref(), Some("1"));
    assert_eq!(
        data.products[0].skus[0].ref_field.as_deref(),
        Some("TS_DUAL")
    );
    assert_eq!(data.products[0].skus[0].option_list_refs, vec!["TES_COL"]);
    assert_eq!(data.option_lists[0].options.len(), 2);
}

#[test]
fn a_product_carries_its_tax_rates() {
    let p: Product = serde_json::from_str(&fixture("product_upload_doc.json")).unwrap();
    let tax = p.tax_rate.expect("tax_rate");
    assert_eq!(tax.delivery.as_deref(), Some("20.0"));
    assert_eq!(tax.collection.as_deref(), Some("5.5"));
    assert_eq!(tax.eat_in.as_deref(), Some("5.5"));

    let retrieved: Product = serde_json::from_str(&fixture("product_retrieve_doc.json")).unwrap();
    let tax = retrieved.tax_rate.expect("tax_rate");
    assert_eq!(tax.eat_in, None, "null eat_in stays unset");
    assert_eq!(retrieved.skus[0].option_list_ids, vec!["e2sfj"]);
}

#[test]
fn a_sku_carries_restrictions_and_price_overrides() {
    let sku: Sku = serde_json::from_str(&fixture("sku_upload_doc.json")).unwrap();
    assert!(sku.restrictions.is_some());
    assert_eq!(sku.price_overrides[0].price, "12.30 EUR");
    assert_eq!(sku.tags, vec!["hidden"]);
    let back = serde_json::to_value(&sku).unwrap();
    assert_eq!(back["restrictions"]["end_time"], "13:30");
    assert_eq!(back["price_overrides"][0]["variant_refs"][0], "1");
}

#[test]
fn a_disabled_restriction_takes_an_element_off_sale() {
    use pos_hubrise_types::Restrictions;
    let off: Option<Restrictions> =
        serde_json::from_value(serde_json::json!({"enabled": false})).unwrap();
    let on: Option<Restrictions> =
        serde_json::from_value(serde_json::json!({"end_time": "13:30"})).unwrap();
    assert!(!Restrictions::is_enabled(&off));
    assert!(Restrictions::is_enabled(&on));
    assert!(Restrictions::is_enabled(&None));
}

#[test]
fn an_option_list_with_null_max_parses() {
    let list: OptionList = serde_json::from_str(&fixture("option_list_retrieve_doc.json")).unwrap();
    assert_eq!(list.max_selections, None);
    assert_eq!(list.options[0].ref_field.as_deref(), Some("BBQ"));
}
