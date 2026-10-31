//! Maps a local POS `Order + OrderLines` to a HubRise-compatible
//! JSON body for `POST /v1/locations/:loc/orders`. Lives in the Tauri
//! command layer (not in the pos-hubrise crate) because it depends on
//! `pos_core::order::*` types, and pos-hubrise must stay core-agnostic
//! to avoid circular workspace dependencies.

use pos_core::order::{Order, OrderLine, OrderType};
use serde_json::{Value, json};

/// Format an integer cents amount as `"12.50 EUR"` - the wire format
/// `HubRise` expects for monetary values.
fn cents_to_hubrise(cents: i64) -> String {
    pos_hubrise_types::money::format_cents(cents)
}

/// The discounts of a till sale for the echo: one per discount name and
/// grouping, the amount taken off (positive), from the consolidated
/// changes of its lines.
pub(crate) fn order_discounts(
    changes: &pos_fiscal::price_change::LineChanges,
) -> Vec<(String, i64)> {
    let mut by_group: std::collections::BTreeMap<(i32, String), i64> =
        std::collections::BTreeMap::default();
    for change in changes.values().flatten() {
        if change.change_type == pos_fiscal::types::PriceChangeType::Public {
            *by_group
                .entry((change.grouping_id, change.name.clone()))
                .or_default() -= change.amount_cents;
        }
    }
    by_group
        .into_iter()
        .filter(|(_, cents)| *cents != 0)
        .map(|((_, name), cents)| (name, cents))
        .collect()
}

/// Build the JSON body for `POST /v1/locations/:loc/orders`.
///
/// The order is pushed as `status: "completed"` because POS orders
/// are only pushed AFTER successful checkout (FDM signed + paid).
/// `HubRise` doesn't need to advance them through a state machine.
/// `payments` are the sale's payments, `(name, cents)`, one per method.
pub fn build_hubrise_order_body(
    order: &Order,
    lines: &[OrderLine],
    payments: &[(&str, i64)],
    discounts: &[(String, i64)],
) -> Value {
    let service_type = match order.order_type {
        OrderType::DineIn => "eat_in",
        OrderType::Takeaway => "collection",
    };

    let items: Vec<Value> = lines
        .iter()
        .map(|l| {
            json!({
                "product_name": l.name,
                "price": cents_to_hubrise(l.unit_price_cents),
                "quantity": match l.weight_grams {
                    Some(_) => format!("{:.3}", l.fiscal_quantity()),
                    None => l.quantity.to_string(),
                },
                "tax_rate": match l.vat_code.as_str() {
                    "A" => "21",
                    "B" => "12",
                    "C" => "6",
                    // "D" (exempt) and anything unknown.
                    _ => "0",
                },
            })
        })
        .collect();

    let total = cents_to_hubrise(order.total_cents);

    let payments: Vec<Value> = payments
        .iter()
        .map(|(name, cents)| json!({ "name": name, "amount": cents_to_hubrise(*cents) }))
        .collect();

    let mut body = json!({
        "status": "completed",
        "service_type": service_type,
        "channel": "POS",
        "connection_name": "pili-pos",
        "items": items,
        "total": total,
        "payments": payments,
        "created_at": order.created_at.to_rfc3339(),
    });

    // The till's discounts, so the echoed order's total matches its
    // payments: `price_off` is the amount taken off (positive).
    if !discounts.is_empty() {
        body["discounts"] = json!(
            discounts
                .iter()
                .map(|(name, cents)| json!({ "name": name, "price_off": cents_to_hubrise(*cents) }))
                .collect::<Vec<_>>()
        );
    }

    // Dine-in: encode the table number as a collection_code so
    // tsb-service / tsb-dashboard can display "Table 5" in its UI.
    if let Some(table) = order.table_number {
        body["collection_code"] = json!(format!("T{table}"));
    }

    body
}
