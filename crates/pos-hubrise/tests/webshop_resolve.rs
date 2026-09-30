//! Online orders resolved against a POS menu that mirrors the mock's
//! seed catalog: every template, and every reason an order is held.

use pos_core::vat::VatCode;
use pos_hubrise::webshop::{
    Attention, Channel, LineKind, PaymentMode, Warning, ingest, load, re_resolve_pending, resolve,
    set_channel,
};
use pos_hubrise_mock::builders::{OrderBuilder, order_total_cents, seed_catalog, template};
use pos_hubrise_types::{Catalog, CatalogData, Order};
use rusqlite::Connection;
use serde_json::{Value, json};

fn menu(data: CatalogData) -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    pos_db::migrate::run_migrations(&conn).unwrap();
    let catalog = Catalog {
        id: "cat".into(),
        location_id: None,
        name: "Menu".into(),
        created_at: None,
        data: Some(data),
    };
    let outcome = pos_hubrise::catalog_map::apply(&mut conn, &catalog).unwrap();
    assert!(
        matches!(outcome, pos_catalog::ApplyOutcome::Applied { .. }),
        "{outcome:?}"
    );
    conn
}

fn seeded() -> Connection {
    menu(seed_catalog())
}

/// A HubRise order as the API returns it: ids, status, total.
fn as_order(mut body: Value) -> Order {
    let total = order_total_cents(&body);
    body["id"] = json!("hr-1");
    body["location_id"] = json!("loc");
    body["status"] = json!("new");
    body["created_at"] = json!("2026-09-30T12:00:00+02:00");
    if body.get("total").is_none() {
        body["total"] = json!(pos_hubrise_types::money::format_cents(total));
    }
    serde_json::from_value(body).unwrap()
}

fn templated(name: &str) -> Order {
    as_order(template(name).unwrap().build())
}

fn codes(att: &[Attention]) -> Vec<String> {
    att.iter()
        .map(|a| {
            serde_json::to_value(a).unwrap()["code"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect()
}

#[test]
fn a_deliveroo_order_is_prepaid_by_app_with_takeaway_vat() {
    let conn = seeded();
    let r = resolve(&conn, &templated("deliveroo_prepaid")).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert_eq!(r.payment_mode, Some(PaymentMode::Prepaid));
    assert_eq!(r.channel.unwrap().payment_type, "APP");
    assert_eq!(r.lines.len(), 2);
    let maki = &r.lines[0];
    assert_eq!(maki.name, "Maki saumon 12 pièces (Soja sucrée)");
    assert_eq!((maki.quantity, maki.unit_price_cents), (2, 1050));
    assert_eq!(maki.vat_code, VatCode::C, "delivered food");
    assert_eq!(r.lines[1].vat_code, VatCode::C, "soft drink taken away");
    assert!(r.warnings.is_empty());
}

#[test]
fn an_uber_eats_order_is_prepaid_by_app() {
    let conn = seeded();
    let r = resolve(&conn, &templated("ubereats_prepaid")).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert_eq!(r.payment_mode, Some(PaymentMode::Prepaid));
    assert_eq!(r.lines[0].unit_price_cents, 990, "roll with avocado");
}

#[test]
fn the_webshop_paid_online_is_prepaid_online() {
    let conn = seeded();
    let r = resolve(&conn, &templated("webshop_online")).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert_eq!(r.channel.unwrap().payment_type, "ONLINE");
    assert_eq!(r.lines[1].vat_code, VatCode::A, "beer stays 21 %");
}

#[test]
fn cash_at_collection_is_unpaid() {
    let conn = seeded();
    let r = resolve(&conn, &templated("webshop_cash_collection")).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert_eq!(r.payment_mode, Some(PaymentMode::Unpaid));
}

#[test]
fn cash_at_the_door_is_unpaid_and_the_delivery_charge_is_a_line() {
    let conn = seeded();
    let r = resolve(&conn, &templated("webshop_cash_delivery")).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert_eq!(r.payment_mode, Some(PaymentMode::Unpaid));
    let fee = r.lines.last().unwrap();
    assert_eq!(fee.kind, LineKind::Charge);
    assert_eq!((fee.unit_price_cents, fee.vat_code), (250, VatCode::C));
}

#[test]
fn an_unknown_channel_is_held_and_a_mapping_releases_it() {
    let conn = seeded();
    let order = templated("eat_in");
    let r = resolve(&conn, &order).unwrap();
    assert_eq!(codes(&r.attention), vec!["unknown_channel"]);
    assert_eq!(r.lines[0].vat_code, VatCode::B, "food eaten in");
    assert_eq!(r.lines[1].vat_code, VatCode::A);

    ingest(&conn, &order).unwrap();
    let fixed = set_channel(
        &conn,
        &Channel {
            channel: "Table QR".into(),
            kind: "platform".into(),
            payment_type: "APP".into(),
            charge_policy: "include".into(),
        },
    )
    .unwrap();
    assert_eq!(fixed, 1);
}

#[test]
fn an_unknown_product_is_held_until_the_catalog_has_it() {
    let conn = seeded();
    let body = OrderBuilder::new("Deliveroo", "delivery")
        .item("GYOZA-6", "Gyoza", 550, 1)
        .paid_by("DELIVEROO", "Deliveroo")
        .build();
    let order = as_order(body);
    let ingested = ingest(&conn, &order).unwrap();
    assert!(ingested.catalog_miss);
    let state = load(&conn, ingested.order_id).unwrap().unwrap();
    assert_eq!(state.resolution, "needs_review");
    assert_eq!(codes(&state.attention), vec!["unknown_product"]);

    // The catalog gains the product: the menu then resolves the order.
    let mut json = pos_hubrise_mock::builders::seed_catalog_builder()
        .product(
            "GYOZA",
            "MAKI",
            "Gyoza",
            pos_hubrise_mock::builders::FOOD,
            &[("GYOZA-6", None, 550, &[])],
        )
        .build_json();
    json["charges"] = json!([]);
    let mut conn2 = conn;
    let catalog = Catalog {
        id: "cat".into(),
        location_id: None,
        name: "Menu".into(),
        created_at: None,
        data: Some(serde_json::from_value(json).unwrap()),
    };
    pos_hubrise::catalog_map::apply(&mut conn2, &catalog).unwrap();
    assert_eq!(re_resolve_pending(&conn2).unwrap(), 1);
    let state = load(&conn2, ingested.order_id).unwrap().unwrap();
    assert_eq!(state.resolution, "ok");
}

#[test]
fn an_unknown_option_or_charge_is_held() {
    let conn = seeded();
    let body = OrderBuilder::new("Website", "delivery")
        .item("MISO-1", "Soupe miso", 350, 1)
        .option("Sauce", "Wasabi", "WASABI", 0)
        .charge("TIP", "Pourboire", 100)
        .build();
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert_eq!(
        codes(&r.attention),
        vec!["unknown_option", "unknown_charge"]
    );
}

#[test]
fn an_item_without_ref_is_held() {
    let conn = seeded();
    let mut body = OrderBuilder::new("Website", "collection")
        .item("MISO-1", "Soupe miso", 350, 1)
        .build();
    body["items"][0].as_object_mut().unwrap().remove("sku_ref");
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert_eq!(codes(&r.attention), vec!["item_without_ref"]);
}

#[test]
fn a_vat_rate_other_than_the_menus_is_held() {
    let conn = seeded();
    // Food eaten in is 12 %, not 6 %.
    let body = OrderBuilder::new("Website", "eat_in")
        .item("MISO-1", "Soupe miso", 350, 1)
        .tax_rate("6.0")
        .build();
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert_eq!(
        r.attention,
        vec![Attention::VatMismatch {
            at: "MISO-1".into(),
            order: "6.0".into(),
            menu: VatCode::B,
        }]
    );
}

#[test]
fn a_platform_markup_is_a_warning_not_a_hold() {
    let conn = seeded();
    let body = OrderBuilder::new("Deliveroo", "delivery")
        .item("MISO-1", "Soupe miso", 420, 1)
        .paid_by("DELIVEROO", "Deliveroo")
        .build();
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert_eq!(
        r.warnings,
        vec![Warning::PriceDiffers {
            at: "MISO-1".into(),
            catalog_cents: 350,
            charged_cents: 420,
        }]
    );
    assert_eq!(
        r.lines[0].unit_price_cents, 420,
        "the charged price is signed"
    );
}

#[test]
fn lines_that_do_not_add_up_are_held() {
    let conn = seeded();
    let mut body = template("webshop_online").unwrap().build();
    body["total"] = json!("99.00 EUR");
    body["payments"][0]["amount"] = json!("99.00 EUR");
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert_eq!(codes(&r.attention), vec!["total_mismatch"]);
}

#[test]
fn a_discount_is_held_for_now() {
    let conn = seeded();
    let body = OrderBuilder::new("Deliveroo", "delivery")
        .item("MISO-1", "Soupe miso", 350, 2)
        .discount("PROMO", "Promo", 100)
        .paid_by("DELIVEROO", "Deliveroo")
        .build();
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert_eq!(codes(&r.attention), vec!["discount"]);
}

#[test]
fn a_partial_or_mixed_payment_is_held() {
    let conn = seeded();
    let partial = OrderBuilder::new("Website", "collection")
        .item("MISO-1", "Soupe miso", 350, 2)
        .payment("ONLINE", "En ligne", 300)
        .build();
    let r = resolve(&conn, &as_order(partial)).unwrap();
    assert_eq!(r.payment_mode, None);
    assert_eq!(
        r.attention,
        vec![Attention::PaymentUnclear {
            paid_cents: 300,
            total_cents: 700
        }]
    );

    let mixed = OrderBuilder::new("Website", "collection")
        .item("MISO-1", "Soupe miso", 350, 2)
        .payment("ONLINE", "En ligne", 400)
        .payment("CASH", "Espèces", 300)
        .build();
    let r = resolve(&conn, &as_order(mixed)).unwrap();
    assert_eq!(codes(&r.attention), vec!["payment_unclear"]);
}

#[test]
fn a_missing_service_type_is_held() {
    let conn = seeded();
    let mut body = OrderBuilder::new("Website", "collection")
        .item("MISO-1", "Soupe miso", 350, 1)
        .build();
    body.as_object_mut().unwrap().remove("service_type");
    let r = resolve(&conn, &as_order(body)).unwrap();
    assert_eq!(codes(&r.attention), vec!["missing_service_type"]);
}

#[test]
fn an_excluded_charge_is_not_a_line_and_still_adds_up() {
    let conn = seeded();
    set_channel(
        &conn,
        &Channel {
            channel: "website".into(),
            kind: "webshop".into(),
            payment_type: "ONLINE".into(),
            charge_policy: "exclude".into(),
        },
    )
    .unwrap();
    let r = resolve(&conn, &templated("webshop_cash_delivery")).unwrap();
    assert!(r.is_ok(), "{:?}", r.attention);
    assert!(r.lines.iter().all(|l| l.kind == LineKind::Product));
}

#[test]
fn a_cancellation_before_acceptance_voids_the_order() {
    let conn = seeded();
    let mut order = templated("deliveroo_prepaid");
    let first = ingest(&conn, &order).unwrap();
    assert!(first.pending);
    order.status = pos_hubrise_types::OrderStatus::Cancelled;
    let again = ingest(&conn, &order).unwrap();
    assert!(!again.pending);
    assert_eq!(again.order_id, first.order_id);
    let state = load(&conn, first.order_id).unwrap().unwrap();
    assert_eq!(state.fiscal_state, "voided");
    let status: String = conn
        .query_row(
            "SELECT status FROM orders WHERE id = ?1",
            [first.order_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "voided");
}

#[test]
fn a_signed_order_is_not_resolved_again() {
    let conn = seeded();
    let order = templated("deliveroo_prepaid");
    let first = ingest(&conn, &order).unwrap();
    conn.execute(
        "UPDATE webshop_orders SET fiscal_state = 'signed_n' WHERE order_id = ?1",
        [first.order_id],
    )
    .unwrap();
    let before = load(&conn, first.order_id).unwrap().unwrap();
    let mut changed = order.clone();
    changed.items.clear();
    let again = ingest(&conn, &changed).unwrap();
    assert!(!again.pending);
    // The signed lines stand; the change is flagged for staff.
    let after = load(&conn, first.order_id).unwrap().unwrap();
    assert_eq!(after.lines, before.lines);
    assert_eq!(after.resolution, before.resolution);
    assert_eq!(after.attention, vec![Attention::ChangedAfterSigning]);
}

#[test]
fn every_template_ingests_once() {
    let conn = seeded();
    for name in pos_hubrise_mock::builders::TEMPLATES {
        let mut order = templated(name);
        order.id = format!("hr-{name}");
        let a = ingest(&conn, &order).unwrap();
        let b = ingest(&conn, &order).unwrap();
        assert_eq!(a.order_id, b.order_id, "{name}");
    }
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM webshop_orders", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows as usize, pos_hubrise_mock::builders::TEMPLATES.len());
}
