//! Parity with the statistics export of `pos-export`: both crates parse the archived
//! signed events with their own code (V12-D5, `pos-accounting` is stand-alone), so a
//! shared fixture of archived events pins the results they must agree on. The same
//! file is read by `crates/pos-export/tests/stats_build.rs`, with the same expected
//! values.

use pos_accounting::parse::{RawEvent, parse_sale};
use serde_json::Value;

const CASES: &str = include_str!("golden/parity_cases.json");

#[test]
fn the_parity_cases_give_the_expected_totals() {
    let cases: Vec<Value> = serde_json::from_str(CASES).expect("the fixture is JSON");
    assert!(cases.len() >= 4);
    for case in &cases {
        let name = case["name"].as_str().unwrap();
        let event = RawEvent {
            id: 1,
            label: "N".into(),
            request: case["request"].clone(),
            response: case["response"].clone(),
            fdm_datetime: Some("2026-10-04T10:00:00.000Z".into()),
        };
        let sale = parse_sale(&event, None).unwrap_or_else(|e| panic!("{name}: {e}"));
        let expected = &case["expected"];
        assert_eq!(sale.refund, expected["refund"].as_bool().unwrap(), "{name}");
        assert_eq!(
            sale.paid(),
            expected["paid_cents"].as_i64().unwrap(),
            "{name}"
        );
        let want = expected["vats"].as_array().unwrap();
        assert_eq!(sale.vats.len(), want.len(), "{name}");
        for (row, w) in sale.vats.iter().zip(want) {
            assert_eq!(row.label, w["label"].as_str().unwrap(), "{name}");
            assert_eq!(row.rate_bp, w["rate_bp"].as_i64().unwrap(), "{name}");
            assert_eq!(row.base, w["base"].as_i64().unwrap(), "{name}");
            assert_eq!(row.vat, w["vat"].as_i64().unwrap(), "{name}");
            assert_eq!(row.total, w["total"].as_i64().unwrap(), "{name}");
        }
    }
}
