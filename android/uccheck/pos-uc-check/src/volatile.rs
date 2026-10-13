//! What two honest runs of the same scenario may differ in.
//!
//! One constant table per kind of data, each entry with its reason. A
//! path is volatile only when its value comes from the clock, from a
//! generated identifier, from the installation's identity or from the
//! FDM's own arithmetic over those. A business field (a product, a
//! quantity, a price, a VAT amount, a payment, a counter) is never in
//! this table: a real difference there is an ESCALATE, not a new entry
//! (P7D8). The same table is documented in `docs/android/UC.md`.

use serde_json::Value;

/// Keys removed from a request, wherever they sit in the JSON.
pub const VOLATILE_KEYS: &[(&str, &str)] = &[
    (
        "posDateTime",
        "time of the request, read from the clock when the event is built",
    ),
    (
        "bookingDate",
        "calendar day of the run, so two runs on two days would differ",
    ),
    (
        "bookingPeriodId",
        "identifier generated when a booking period opens (a random UUID)",
    ),
    (
        "posId",
        "installation identity: the Android bundle and the host run may name different test posIds",
    ),
    (
        "terminalId",
        "installation identity, set by the provisioning of each side",
    ),
    (
        "deviceId",
        "installation identity: the Android engine takes the handset's id, the host takes pos.json",
    ),
    (
        "posSwVersion",
        "the product string of each side (pili-pos/x.y.z against pili-pos-android/x.y.z); the sw-version check proves it",
    ),
    (
        "digitalSignature",
        "signature the FDM derives from the clock and the counters",
    ),
    ("shortSignature", "hash of the digital signature"),
    ("verificationUrl", "carries the FDM's signature and date"),
];

/// Suffix of keys that hold a point in time (`fdmDateTime` of a copied
/// ticket's `fdmRef`, `reportDateTime`, ...).
pub const VOLATILE_KEY_SUFFIX: (&str, &str) = (
    "DateTime",
    "a point in time, read from the clock of the side that built or signed it",
);

/// Is this object key volatile?
pub fn is_volatile_key(key: &str) -> bool {
    VOLATILE_KEYS.iter().any(|(k, _)| *k == key) || key.ends_with(VOLATILE_KEY_SUFFIX.0)
}

/// A copy of `value` with every volatile key removed, at any depth.
pub fn strip_volatile(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(k, _)| !is_volatile_key(k))
                .map(|(k, v)| (k.clone(), strip_volatile(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(strip_volatile).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn volatile_keys_are_removed_at_every_depth() {
        let request = json!({
            "query": "mutation { signSale }",
            "variables": { "data": {
                "posDateTime": "2026-10-10T10:00:00.000+02:00",
                "posId": "CPOS0031234567",
                "posFiscalTicketNo": 3,
                "fdmRef": { "eventLabel": "N", "fdmDateTime": "2026-10-10T09:00:00+02:00", "eventCounter": 1 },
                "transaction": { "transactionLines": [{ "mainProduct": { "unitPrice": 2.5 } }] },
            } },
        });
        let stripped = strip_volatile(&request);
        assert_eq!(
            stripped,
            json!({
                "query": "mutation { signSale }",
                "variables": { "data": {
                    "posFiscalTicketNo": 3,
                    "fdmRef": { "eventLabel": "N", "eventCounter": 1 },
                    "transaction": { "transactionLines": [{ "mainProduct": { "unitPrice": 2.5 } }] },
                } },
            })
        );
    }

    #[test]
    fn a_business_field_is_never_volatile() {
        for key in [
            "unitPrice",
            "quantity",
            "lineTotal",
            "vats",
            "price",
            "amount",
            "eventCounter",
            "totalCounter",
            "eventLabel",
            "posFiscalTicketNo",
            "productId",
            "financials",
            "ticketMedium",
        ] {
            assert!(!is_volatile_key(key), "{key} must stay compared");
        }
    }

    #[test]
    fn every_entry_of_the_table_has_a_reason() {
        for (key, reason) in VOLATILE_KEYS {
            assert!(!key.is_empty() && reason.len() > 10, "{key}");
        }
        assert!(VOLATILE_KEY_SUFFIX.1.len() > 10);
    }

    #[test]
    fn a_date_time_suffix_is_volatile() {
        assert!(is_volatile_key("fdmDateTime"));
        assert!(is_volatile_key("reportDateTime"));
        assert!(!is_volatile_key("dateTimeless"));
    }
}
