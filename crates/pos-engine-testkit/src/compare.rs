//! Field-by-field comparison of a request pili-pos built with an
//! official fixture's request: the business content only (products,
//! quantities, prices, VAT, totals, payments, cost-center type,
//! referenced ticket, ticket medium), not the installation's identity.

use serde_json::{Value, json};

/// The business content of a request, independent of the installation.
pub fn business_view(d: &Value) -> Value {
    let mut view = serde_json::Map::new();
    view.insert("ticketMedium".into(), d["ticketMedium"].clone());
    if let Some(t) = d.get("transaction") {
        view.insert("transaction".into(), transaction_view(t));
    }
    // A table transfer: each side's cost center type and transaction.
    if let Some(transfer) = d.get("transfer") {
        let side = |name: &str| -> Value {
            Value::Array(
                transfer[name]
                    .as_array()
                    .map_or(&[][..], Vec::as_slice)
                    .iter()
                    .map(|item| {
                        json!({
                            "costCenter.type": item["costCenter"]["type"],
                            "transaction": transaction_view(&item["transaction"]),
                        })
                    })
                    .collect(),
            )
        };
        view.insert(
            "transfer".into(),
            json!({ "from": side("from"), "to": side("to") }),
        );
    }
    if let Some(f) = d.get("financials").and_then(Value::as_array) {
        // The order of the payment lines carries no meaning (a split
        // sale may list cash first or last): compare them sorted.
        let mut lines: Vec<Value> = f
            .iter()
            .map(|p| json!({ "type": p["type"], "amount": p["amount"], "amountType": p["amountType"] }))
            .collect();
        lines.sort_by_key(|l| (l["type"].to_string(), l["amountType"].to_string()));
        view.insert("financials".into(), Value::Array(lines));
    }
    if let Some(cc) = d.get("costCenter") {
        view.insert("costCenter.type".into(), cc["type"].clone());
    }
    if let Some(r) = d.get("fdmRef") {
        view.insert("fdmRef".into(), r.clone());
    }
    // An invoice (M150, UC4121, UC4122): the customer and the labels of the events it
    // references (the counters are the answers of the FDM that signed the sales).
    if let Some(vat) = d.get("customerVatNo") {
        view.insert("customerVatNo".into(), vat.clone());
    }
    if let Some(refs) = d.get("fdmRefs").and_then(Value::as_array) {
        let labels: Vec<Value> = refs.iter().map(|r| r["eventLabel"].clone()).collect();
        view.insert("fdmRefLabels".into(), Value::Array(labels));
    }
    if d.get("drawer").is_some() {
        view.insert("drawer".into(), json!(true));
    }
    Value::Object(view)
}

/// The business content of a `transaction` object.
fn transaction_view(t: &Value) -> Value {
    let lines: Vec<Value> = t["transactionLines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let p = &l["mainProduct"];
            json!({
                "lineType": l["lineType"],
                "productId": p["productId"],
                "departmentId": p["departmentId"],
                "quantity": p["quantity"],
                "quantityType": p["quantityType"],
                "negQuantityReason": p.get("negQuantityReason").cloned().unwrap_or(Value::Null),
                "unitPrice": p["unitPrice"],
                "vats": p["vats"].as_array().unwrap().iter()
                    .map(|v| json!({
                        "label": v["label"],
                        "price": v["price"],
                        "priceChanges": price_changes_view(v),
                    }))
                    .collect::<Vec<_>>(),
                "lineTotal": l["lineTotal"],
            })
        })
        .collect();
    json!({ "transactionLines": lines, "transactionTotal": t["transactionTotal"] })
}

/// The price changes of a VAT entry as `[{scope, type, amount}]`; the
/// id and the name are free text and only their presence is checked
/// (by the caller's `check_identification`-style tests), the
/// `groupingId` is the POS's own counter. Null when the entry has none.
fn price_changes_view(vat: &Value) -> Value {
    match vat.get("priceChanges").and_then(Value::as_array) {
        Some(changes) if !changes.is_empty() => Value::Array(
            changes
                .iter()
                .map(|c| json!({ "scope": c["scope"], "type": c["type"], "amount": c["amount"] }))
                .collect(),
        ),
        _ => Value::Null,
    }
}

pub fn diff(path: &str, ours: &Value, fixture: &Value, out: &mut Vec<(String, Value, Value)>) {
    match (ours, fixture) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let child = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                diff(
                    &child,
                    a.get(k).unwrap_or(&Value::Null),
                    b.get(k).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                diff(&format!("{path}[{i}]"), x, y, out);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            if (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() > 0.005 {
                out.push((path.to_string(), ours.clone(), fixture.clone()));
            }
        }
        _ if ours != fixture => out.push((path.to_string(), ours.clone(), fixture.clone())),
        _ => {}
    }
}
