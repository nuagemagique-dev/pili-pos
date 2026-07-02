//! Canonical JSON serialization for reproducible FDM communication.
//!
//! The SCE 2.0 FDM derives its digital signature from the **canonical** form
//! of the JSON it signs (FDM spec v1.4 §2.4.1 "JSON canonique pour le hachage
//! reproductible"). Although our POS does not sign anything itself — the FDM
//! does — sending canonical JSON on the wire gives us three concrete
//! benefits:
//!
//! 1. **Deterministic request bytes.** Two checkouts with identical inputs
//!    produce byte-identical request bodies, which makes the idempotency
//!    key calculation (`{posId, posDateTime, terminalId, eventLabel,
//!    posFiscalTicketNo}`) verifiable end-to-end.
//! 2. **Diff-friendly logs.** The cert dossier wants reproducible request
//!    captures (Phase 11 rehearsal log); canonical serialization makes
//!    `git diff` between two runs read as the genuine semantic change
//!    instead of noise from field-order shuffling.
//! 3. **Future-proofing.** When we eventually need to compute a request
//!    hash for our own ID/retry table (Phase 10), `to_canonical` is the
//!    correct primitive.
//!
//! ## Canonicalization rules
//!
//! - Object keys are sorted by Unicode codepoint at every level.
//! - No insignificant whitespace anywhere.
//! - Numbers are normalized in `serde_json`'s default shortest form (`1.0`
//!   stays `1.0`, `1.00` becomes `1.0`, integers stay integers).
//! - Strings are JSON-escaped in `serde_json`'s default minimal form.
//! - Arrays preserve their element order (the spec also requires this).
//!
//! Internally we round-trip through `serde_json::Value`. `serde_json::Map`
//! is a `BTreeMap<String, Value>` (unless the `preserve_order` feature is
//! enabled, which the workspace does NOT enable), so re-serializing the
//! `Value` yields keys in lexicographic-byte order — which for ASCII keys
//! is identical to Unicode codepoint order. The SCE 2.0 spec only uses
//! ASCII field names, so this is equivalent.

use serde::Serialize;

/// Serialize any `Serialize` value to canonical JSON: sorted keys, no
/// whitespace.
///
/// Equivalent to `serde_json::to_string` *except* that nested objects are
/// alphabetised. This is deterministic — calling it twice on the same
/// value yields byte-identical output (verified by
/// `tests::canonical_is_deterministic`).
pub fn to_canonical<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize + ?Sized,
{
    let json_value: serde_json::Value = serde_json::to_value(value)?;
    serde_json::to_string(&json_value)
}

/// Same as [`to_canonical`] but returns the intermediate `serde_json::Value`
/// alongside the string. Useful in tests that want to assert on shape
/// without re-parsing.
pub fn to_canonical_with_value<T>(
    value: &T,
) -> Result<(String, serde_json::Value), serde_json::Error>
where
    T: Serialize + ?Sized,
{
    let json_value: serde_json::Value = serde_json::to_value(value)?;
    let s = serde_json::to_string(&json_value)?;
    Ok((s, json_value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    /// Two structs with the *same* fields declared in *different* orders.
    /// Canonical serialization must collapse them to the same bytes.
    #[derive(Serialize)]
    struct OrderA {
        zulu: i32,
        alpha: i32,
        mike: String,
    }

    #[derive(Serialize)]
    struct OrderB {
        alpha: i32,
        mike: String,
        zulu: i32,
    }

    #[test]
    fn canonical_sorts_top_level_keys() {
        let s = to_canonical(&OrderA {
            zulu: 1,
            alpha: 2,
            mike: "x".into(),
        })
        .unwrap();
        assert_eq!(s, r#"{"alpha":2,"mike":"x","zulu":1}"#);
    }

    /// The whole point: byte-identical output regardless of declaration order.
    #[test]
    fn canonical_is_field_order_invariant() {
        let a = to_canonical(&OrderA {
            zulu: 1,
            alpha: 2,
            mike: "x".into(),
        })
        .unwrap();
        let b = to_canonical(&OrderB {
            alpha: 2,
            mike: "x".into(),
            zulu: 1,
        })
        .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn canonical_sorts_nested_objects() {
        #[derive(Serialize)]
        struct Outer {
            outer_z: bool,
            outer_a: Inner,
        }
        #[derive(Serialize)]
        struct Inner {
            inner_z: bool,
            inner_a: bool,
        }
        let s = to_canonical(&Outer {
            outer_z: true,
            outer_a: Inner {
                inner_z: false,
                inner_a: true,
            },
        })
        .unwrap();
        assert_eq!(
            s,
            r#"{"outer_a":{"inner_a":true,"inner_z":false},"outer_z":true}"#
        );
    }

    #[test]
    fn canonical_preserves_array_order() {
        // Arrays are SEQUENCES — their element order is semantic, so it
        // must be preserved. Only object keys get sorted.
        let v = serde_json::json!({"items": ["zulu", "alpha", "mike"]});
        let s = to_canonical(&v).unwrap();
        assert_eq!(s, r#"{"items":["zulu","alpha","mike"]}"#);
    }

    #[test]
    fn canonical_is_deterministic() {
        // Determinism guarantee: serializing the same value twice yields
        // the exact same byte string. This is what makes the canonical
        // form usable as an input to idempotency-key hashing.
        let v = serde_json::json!({
            "z": [3, 2, 1],
            "a": {"b": 1, "a": 2, "c": [{"y": 1, "x": 2}]},
            "m": null,
            "k": "literal \"quote\" here",
        });
        let s1 = to_canonical(&v).unwrap();
        let s2 = to_canonical(&v).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn canonical_no_whitespace_in_output() {
        let v = serde_json::json!({"a": [1, 2, {"nested": true}]});
        let s = to_canonical(&v).unwrap();
        assert!(!s.contains(' '), "canonical output must not contain spaces");
        assert!(
            !s.contains('\n'),
            "canonical output must not contain newlines"
        );
        assert!(!s.contains('\t'), "canonical output must not contain tabs");
    }

    #[test]
    fn canonical_handles_unicode_keys() {
        // Codepoint order != byte order for non-ASCII strings, but SCE 2.0
        // only uses ASCII field names. We still verify the function doesn't
        // panic on Unicode keys so the dossier captures any future drift.
        let v = serde_json::json!({"é": 1, "a": 2, "z": 3});
        let s = to_canonical(&v).unwrap();
        // serde_json sorts via BTreeMap on String which compares as bytes;
        // for these ASCII-only first chars the order is a < z < é (the é
        // is multi-byte starting with 0xC3, which sorts after 0x7A 'z').
        assert_eq!(s, r#"{"a":2,"z":3,"é":1}"#);
    }

    /// Property test: 50 random structurally-varied inputs must each
    /// produce identical bytes on repeated serialization.
    #[test]
    fn canonical_property_random_inputs_deterministic() {
        for seed in 0..50 {
            // Build a synthetic Value that varies with `seed` so we cover
            // different nesting/array/null shapes without pulling in a
            // proptest crate.
            let v = synth_value(seed);
            let a = to_canonical(&v).unwrap();
            let b = to_canonical(&v).unwrap();
            assert_eq!(a, b, "non-deterministic at seed {seed}");
        }
    }

    fn synth_value(seed: u32) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        // Pick a handful of keys whose ordering changes across seeds.
        let keys = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"];
        for (i, _k) in keys.iter().enumerate() {
            // Mix of types to exercise the number / string / null / array
            // / nested-object branches.
            let v = match (seed.wrapping_add(i as u32)) % 5 {
                0 => serde_json::Value::Null,
                1 => serde_json::json!(((seed as i64) * (i as i64 + 1))),
                2 => serde_json::json!(format!("v{seed}-{i}")),
                3 => serde_json::json!([seed, i]),
                _ => serde_json::json!({"nested_z": seed, "nested_a": i}),
            };
            // Insert in REVERSE so we exercise the sorting code path.
            obj.insert(keys[keys.len() - 1 - i].to_string(), v);
        }
        serde_json::Value::Object(obj)
    }
}
