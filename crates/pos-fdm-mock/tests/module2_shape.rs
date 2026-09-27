//! The mock answers in the shape of module2's simulator.
//!
//! `fixtures/module2/` holds what the simulator FDM02000425 (FDM
//! software 2.2.0.5) answered on 2026-09-27 (`scripts/live_fdm.sh`
//! with `MOYUN_LIVE_RECORD_DIR`, see its README). Each recorded request
//! is sent to the mock, set up as the simulator was, and both answers
//! must have the same shape: the same keys, the same JSON types, the
//! same HTTP status. Values that must differ (signatures, times,
//! counters, texts) are not compared.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use pos_fdm_mock::{Control, app_with};
use serde_json::Value;
use tower::ServiceExt;

/// Calls where the mock keeps the spec's behaviour on purpose.
const KNOWN_DIFFERENCES: [&str; 2] = [
    // The Q110 multi-language status (four `status` fields in one
    // document, the official request included): the simulator answers
    // HTTP 500 with an empty body. The mock answers as the spec says.
    "status_multi_lang",
    "status_multi_lang_official",
];

const TOKEN: &str = "recorded-token";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/module2")
}

fn read(name: &str) -> Value {
    let raw = std::fs::read_to_string(fixtures().join(name)).unwrap();
    serde_json::from_str(&raw).unwrap()
}

/// Every key path of `value` with its JSON type; an array is described
/// by its first item.
fn shape(value: &Value, path: String, out: &mut BTreeMap<String, &'static str>) {
    let kind = match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    match value {
        Value::Object(map) => {
            for (key, item) in map {
                shape(item, format!("{path}.{key}"), out);
            }
        }
        Value::Array(items) => {
            if let Some(first) = items.first() {
                shape(first, format!("{path}[]"), out);
            }
        }
        _ => {}
    }
    out.insert(path, kind);
}

fn shape_of(value: &Value) -> BTreeMap<String, &'static str> {
    let mut out = BTreeMap::new();
    shape(value, "$".into(), &mut out);
    out
}

/// The mock set up as the simulator was for `label`.
fn control_for(label: &str) -> Control {
    let mut control = Control {
        token: Some(TOKEN.into()),
        allowed_pos: Some(vec!["CPOS0031234567".into()]),
        placeholder_signatures: true,
        ..Default::default()
    };
    if label.ends_with("_buffer_full") {
        control.errors = vec!["BUFFER_FULL".into()];
    } else if label.ends_with("_buffer_near_full") {
        control.warnings = vec!["BUFFER_NEAR_FULL".into()];
    }
    control
}

#[tokio::test]
async fn the_mock_answers_like_the_module2_simulator() {
    let calls = std::fs::read_to_string(fixtures().join("calls.jsonl")).unwrap();
    let mut compared = 0;
    let mut differences = Vec::new();
    for line in calls.lines().filter(|l| !l.trim().is_empty()) {
        let call: Value = serde_json::from_str(line).unwrap();
        let label = call["label"].as_str().unwrap();
        if KNOWN_DIFFERENCES.contains(&label) {
            continue;
        }
        let request = read(&format!("{label}_request.json"));
        let recorded = read(&format!("{label}_response.json"));

        let mut http = Request::post("/graphql").header("content-type", "application/json");
        if call["token"].as_bool().unwrap() {
            let token = if label.contains("wrong_token") {
                "wrong"
            } else {
                TOKEN
            };
            http = http.header("authorization", format!("Bearer {token}"));
        }
        let response = app_with(control_for(label))
            .oneshot(http.body(Body::from(request.to_string())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let mock: Value = serde_json::from_slice(&bytes).unwrap();

        let recorded_status = call["httpStatus"].as_u64().unwrap() as u16;
        if status != StatusCode::from_u16(recorded_status).unwrap() {
            differences.push(format!("{label}: HTTP {status}, module2 {recorded_status}"));
        }
        let (want, got) = (shape_of(&recorded), shape_of(&mock));
        for path in want.keys().chain(got.keys()).collect::<BTreeSet<_>>() {
            if want.get(path) != got.get(path) {
                differences.push(format!(
                    "{label}: {path} is {:?} on module2, {:?} on the mock",
                    want.get(path),
                    got.get(path)
                ));
            }
        }
        compared += 1;
    }
    assert!(compared >= 20, "only {compared} recorded calls compared");
    assert!(
        differences.is_empty(),
        "the mock differs from module2:\n{}",
        differences.join("\n")
    );
}
