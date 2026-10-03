//! The mock answers in the shape of the real HubRise API.
//!
//! `fixtures/hubrise/` holds what api.hubrise.com answered on
//! 2026-10-03 for a test location (`scripts/live_hubrise.sh` with
//! `PILI_LIVE_RECORD_DIR`, customer details and names scrubbed). Two
//! checks:
//!
//! - every recorded answer parses into the POS's HubRise types;
//! - each call replayed on the mock answers with no field the real API
//!   lacks, of the same JSON type where both have a value. The real API
//!   has more fields than the mock; the mock never invents one.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use pos_hubrise_mock::app;
use pos_hubrise_mock::state::{MockState, POS_TOKEN, WEBSHOP_TOKEN};
use serde_json::{Value, json};
use tower::ServiceExt;

const LOCATION: &str = "pili-dev-loc-1";
const CATALOG: &str = "pili-dev-cat-1";

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/hubrise")
}

/// Every recorded call, `(scenario/file, call)`.
fn recorded() -> Vec<(String, Value)> {
    let mut all = Vec::new();
    for scenario in std::fs::read_dir(fixtures()).unwrap() {
        let scenario = scenario.unwrap().path();
        let mut files: Vec<_> = std::fs::read_dir(&scenario)
            .unwrap()
            .map(|f| f.unwrap().path())
            .collect();
        files.sort();
        for f in files {
            let call: Value = serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
            let name = format!(
                "{}/{}",
                scenario.file_name().unwrap().to_string_lossy(),
                f.file_name().unwrap().to_string_lossy()
            );
            all.push((name, call));
        }
    }
    assert!(!all.is_empty(), "no fixtures");
    all
}

fn find(scenario: &str, method: &str, path_has: &str, pick: impl Fn(&Value) -> bool) -> Value {
    recorded()
        .into_iter()
        .filter(|(name, _)| name.starts_with(scenario))
        .map(|(_, call)| call)
        .find(|c| {
            c["method"] == method && c["path"].as_str().unwrap().contains(path_has) && pick(c)
        })
        .unwrap_or_else(|| panic!("no recorded {method} {path_has} in {scenario}"))
}

#[test]
fn every_real_answer_parses_into_the_pos_types() {
    for (name, call) in recorded() {
        let path = call["path"].as_str().unwrap().split('?').next().unwrap();
        let response = call["response"].clone();
        let parsed = if path.starts_with("/v1/catalogs/") {
            serde_json::from_value::<pos_hubrise_types::Catalog>(response).map(drop)
        } else if path == "/v1/callback" {
            serde_json::from_value::<pos_hubrise_types::callback::CallbackRegistration>(response)
                .map(drop)
        } else if path.ends_with("/orders") && call["method"] == "GET" {
            serde_json::from_value::<Vec<pos_hubrise_types::Order>>(response).map(drop)
        } else if path.contains("/orders") {
            serde_json::from_value::<pos_hubrise_types::Order>(response).map(drop)
        } else {
            panic!("{name}: unclassified call {path}");
        };
        if let Err(e) = parsed {
            panic!("{name}: {e}");
        }
    }
}

/// Every key path of `value` with the JSON types it holds; an array's
/// items are described under `path[]`.
fn shape(value: &Value) -> BTreeMap<String, BTreeSet<&'static str>> {
    fn kind(v: &Value) -> &'static str {
        match v {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
    fn walk(v: &Value, at: String, out: &mut BTreeMap<String, BTreeSet<&'static str>>) {
        out.entry(at.clone()).or_default().insert(kind(v));
        match v {
            Value::Object(map) => {
                for (k, v) in map {
                    walk(v, format!("{at}.{k}"), out);
                }
            }
            Value::Array(items) => {
                for v in items {
                    walk(v, format!("{at}[]"), out);
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeMap::new();
    walk(value, String::new(), &mut out);
    out
}

/// What the mock has that the real answer lacks, or types that clash
/// where both hold a value.
fn invented(mock: &Value, real: &Value) -> Vec<String> {
    let real = shape(real);
    let mut out = Vec::new();
    for (path, kinds) in shape(mock) {
        let Some(real_kinds) = real.get(&path) else {
            out.push(format!("{path}: not in the real answer"));
            continue;
        };
        let mock_values: BTreeSet<_> = kinds.into_iter().filter(|k| *k != "null").collect();
        let real_values: BTreeSet<_> = real_kinds
            .iter()
            .copied()
            .filter(|k| *k != "null")
            .collect();
        if !mock_values.is_empty()
            && !real_values.is_empty()
            && mock_values.is_disjoint(&real_values)
        {
            out.push(format!(
                "{path}: mock {mock_values:?}, real {real_values:?}"
            ));
        }
    }
    out
}

struct Mock(axum::Router);

impl Mock {
    fn new() -> Self {
        Self(app(Arc::new(MockState::new_seeded())))
    }

    async fn send(&self, method: &str, uri: &str, token: &str, body: Option<&Value>) -> Value {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header("X-Access-Token", token)
            .header("Content-Type", "application/json");
        let req = match body {
            Some(b) => req.body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let resp = self.0.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert!(status.is_success(), "{method} {uri}: {status}");
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    }
}

#[tokio::test]
async fn the_mock_invents_no_field() {
    let mock = Mock::new();
    let mut found = Vec::new();
    let mut check = |what: &str, mock: &Value, real: &Value| {
        found.extend(
            invented(mock, real)
                .into_iter()
                .map(|d| format!("{what} {d}")),
        );
    };

    // The callback, registered then read.
    let real = find("catalog_mirror", "POST", "/v1/callback", |_| true);
    let answer = mock
        .send("POST", "/v1/callback", POS_TOKEN, Some(&real["request"]))
        .await;
    check("POST /callback", &answer, &real["response"]);
    // Read once registered (the first run's GET came before it).
    let real = find("prepaid_cancel_refund", "GET", "/v1/callback", |_| true);
    let answer = mock.send("GET", "/v1/callback", POS_TOKEN, None).await;
    check("GET /callback", &answer, &real["response"]);

    // The catalog, replaced then read.
    let real = find("catalog_mirror", "PUT", "/v1/catalogs/", |_| true);
    let answer = mock
        .send(
            "PUT",
            &format!("/v1/catalogs/{CATALOG}"),
            POS_TOKEN,
            Some(&real["request"]),
        )
        .await;
    check("PUT /catalogs", &answer, &real["response"]);
    let real = find("catalog_mirror", "GET", "/v1/catalogs/", |_| true);
    let answer = mock
        .send("GET", &format!("/v1/catalogs/{CATALOG}"), POS_TOKEN, None)
        .await;
    check("GET /catalogs", &answer, &real["response"]);

    // An order: placed, listed, read, its status changed.
    for scenario in ["prepaid_cancel_refund", "unpaid_collected"] {
        let real = find(scenario, "POST", "/orders", |_| true);
        let placed = mock
            .send(
                "POST",
                &format!("/v1/locations/{LOCATION}/orders"),
                WEBSHOP_TOKEN,
                Some(&real["request"]),
            )
            .await;
        check(
            &format!("{scenario} POST /orders"),
            &placed,
            &real["response"],
        );
        let id = placed["id"].as_str().unwrap();

        let real = find(scenario, "GET", "/orders", |c| {
            c["response"].as_array().is_some_and(|a| !a.is_empty())
        });
        let listed = mock
            .send(
                "GET",
                &format!("/v1/locations/{LOCATION}/orders?status=new"),
                POS_TOKEN,
                None,
            )
            .await;
        check(
            &format!("{scenario} GET /orders"),
            &listed,
            &real["response"],
        );

        let real = find(scenario, "GET", "/orders/", |_| true);
        let read = mock
            .send(
                "GET",
                &format!("/v1/locations/{LOCATION}/orders/{id}"),
                POS_TOKEN,
                None,
            )
            .await;
        check(
            &format!("{scenario} GET /orders/:id"),
            &read,
            &real["response"],
        );

        let real = find(scenario, "PATCH", "/orders/", |_| true);
        let patched = mock
            .send(
                "PATCH",
                &format!("/v1/locations/{LOCATION}/orders/{id}"),
                POS_TOKEN,
                Some(&json!({ "status": "received" })),
            )
            .await;
        check(
            &format!("{scenario} PATCH /orders/:id"),
            &patched,
            &real["response"],
        );
    }

    assert!(found.is_empty(), "the mock differs:\n{}", found.join("\n"));
}
