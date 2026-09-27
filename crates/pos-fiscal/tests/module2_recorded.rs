//! What module2's simulator really answered (recorded on 2026-09-27 in
//! `crates/pos-fdm-mock/fixtures/module2/`) goes through the POS's own
//! parsing and response check: every signed event is accepted against
//! the request it answers, every status parses, and every refusal keeps
//! its code where the POS reads it.

use std::path::PathBuf;

use pos_fiscal::response::{FdmStatus, SignResult};
use pos_fiscal::response_check::{ExpectedEvent, check_sign_result};
use serde_json::Value;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pos-fdm-mock/fixtures/module2")
}

fn read(name: &str) -> Value {
    let raw = std::fs::read_to_string(fixtures().join(name)).unwrap();
    serde_json::from_str(&raw).unwrap()
}

/// `(label, request, response)` of every recorded call.
fn recorded() -> Vec<(String, Value, Value)> {
    std::fs::read_to_string(fixtures().join("calls.jsonl"))
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let call: Value = serde_json::from_str(line).unwrap();
            let label = call["label"].as_str().unwrap().to_string();
            let request = read(&format!("{label}_request.json"));
            let response = read(&format!("{label}_response.json"));
            (label, request, response)
        })
        .collect()
}

#[test]
fn every_event_module2_signed_passes_the_response_check() {
    let mut signed = 0;
    for (label, request, response) in recorded() {
        let Some(data) = response.get("data").and_then(Value::as_object) else {
            continue;
        };
        let variables = &request["variables"];
        let is_training = variables["isTraining"].as_bool().unwrap_or(false);
        for (mutation, value) in data {
            if !mutation.starts_with("sign") {
                continue;
            }
            let result: SignResult = serde_json::from_value(value.clone())
                .unwrap_or_else(|e| panic!("{label} {mutation}: {e}"));
            let input = match mutation.as_str() {
                "signReportTurnoverZ" if variables.get("turnover").is_some() => {
                    &variables["turnover"]
                }
                "signReportUserZ" if variables.get("user").is_some() => &variables["user"],
                _ => &variables["data"],
            };
            let expected = ExpectedEvent::for_mutation(mutation, is_training).unwrap();
            check_sign_result(input, expected, &result)
                .unwrap_or_else(|e| panic!("{label} {mutation}: {e}"));
            signed += 1;
        }
    }
    // The operator day (15 events, the Z counts two), one sale with a
    // warning, and the rest of the Basic Set (8 events).
    assert_eq!(signed, 24);
}

#[test]
fn every_status_module2_gave_parses() {
    let mut parsed = 0;
    for (label, _, response) in recorded() {
        let Some(status) = response.pointer("/data/fdmStatus") else {
            continue;
        };
        let status: FdmStatus =
            serde_json::from_value(status.clone()).unwrap_or_else(|e| panic!("{label}: {e}"));
        assert_eq!(status.device.fdm_id, "FDM02000425", "{label}");
        parsed += 1;
    }
    assert_eq!(parsed, 4);
}

/// A refusal carries its code in `extensions.code`, except the token
/// refusal and GraphQL validation errors, which only have a message.
#[test]
fn every_refusal_module2_gave_keeps_its_code() {
    let codes: Vec<(String, Option<String>)> = recorded()
        .into_iter()
        .filter_map(|(label, _, response)| {
            let error = response.pointer("/errors/0")?.clone();
            let code = error
                .pointer("/extensions/code")
                .and_then(Value::as_str)
                .map(str::to_string);
            Some((label, code))
        })
        .collect();
    let code_of = |label: &str| {
        codes
            .iter()
            .find(|(l, _)| l == label)
            .unwrap_or_else(|| panic!("{label} not recorded"))
            .1
            .clone()
    };
    assert_eq!(
        code_of("signSale_buffer_full").as_deref(),
        Some("BUFFER_FULL")
    );
    assert_eq!(
        code_of("signWorkIn_unknown_pos").as_deref(),
        Some("UNKNOWN_POS")
    );
    assert_eq!(
        code_of("signWorkIn_microseconds").as_deref(),
        Some("INVALID_REQUEST")
    );
    assert_eq!(code_of("status_no_token"), None);
    assert_eq!(code_of("Status_capitalized"), None);
}
