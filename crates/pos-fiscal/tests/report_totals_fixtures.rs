//! The SPF Reports v1.3 scenario, replayed through `report_totals`:
//! the eight input tickets T1–T8 (their published requests and FDM
//! responses) must add up to the totals of the published reports
//! M180–M184.
//!
//! Two differences are intended and asserted as such:
//! - P tickets: the scenario registers four P events (orders T1, T5, T6
//!   and pre-bill T7, 250 €); the published report counts three
//!   (198 €, T1 left out). The spec defines ticketCount as the number of
//!   events of the label during the bookingDate (§2.2.2 turnover), so
//!   pili-pos reports 4 and 250 €.
//! - Payment names: the turnover report of the fixture names the methods
//!   "Contant" and "Mastercard", while the sales (and the fixture's own
//!   user report) use "CONTANT" and "Master Card". The report carries
//!   the names of the sales.
//!
//! `fdmDevices` times are compared within a few seconds: the fixture
//! rounds them (and its first one is the POS time of T1, 7 s before the
//! FDM signed it), while pili-pos copies the FDM's own `fdmDateTime`.

use std::path::PathBuf;

use pos_fiscal::report_totals::{self, ReportEvent};
use pos_fiscal::types::EventLabel;
use serde_json::{Value, json};

fn reports_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/20260821_UC_Reports_1.3")
}

fn read_json(dir: &str, suffix: &str) -> Value {
    let dir = reports_root().join(dir);
    let path = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with(suffix))
        .unwrap_or_else(|| panic!("no *{suffix} in {}", dir.display()));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
}

/// T1–T8 as archived: the request's `data` and the mutation result.
fn scenario() -> Vec<ReportEvent> {
    [
        ("T1", EventLabel::P),
        ("T2", EventLabel::N),
        ("T3", EventLabel::N),
        ("T4", EventLabel::N),
        ("T5", EventLabel::P),
        ("T6", EventLabel::P),
        ("T7", EventLabel::P),
        ("T8", EventLabel::N),
    ]
    .into_iter()
    .map(|(t, label)| {
        let dir = format!("InputReport/{t}");
        let request = read_json(&dir, "GQL-Variables.json")["data"].clone();
        let response = read_json(&dir, "_Response.json")["data"]
            .as_object()
            .unwrap()
            .values()
            .next()
            .unwrap()
            .clone();
        assert_eq!(
            response["fdmRef"]["eventLabel"],
            json!(label),
            "{t} is signed as {label:?}"
        );
        ReportEvent {
            label,
            request,
            response,
            registered_negatives: Vec::new(),
            invoice_amount: None,
        }
    })
    .collect()
}

fn to_json<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}

/// Numbers compared as amounts (JSON 24 == 24.0).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() < 0.005
        }
        (Value::Object(x), Value::Object(y)) => {
            let keys: std::collections::BTreeSet<_> = x.keys().chain(y.keys()).collect();
            keys.into_iter().all(|k| {
                same(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                )
            })
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        _ => a == b,
    }
}

fn assert_same(what: &str, ours: &Value, fixture: &Value) {
    assert!(
        same(ours, fixture),
        "{what} differs\n pili-pos: {ours:#}\n fixture:   {fixture:#}"
    );
}

/// The turnover of the published reports, with the intended differences
/// applied (see the module comment).
fn expected_turnover(fixture: &Value) -> Value {
    let mut t = fixture.clone();
    t["transactions"][1] = json!({ "eventLabel": "P", "ticketCount": 4, "amount": 250 });
    t["payments"][0]["name"] = json!("CONTANT");
    t["payments"][1]["name"] = json!("Master Card");
    t
}

#[test]
fn the_published_p_count_leaves_out_the_first_order() {
    // Guards the first intended difference: if SPF corrects the fixture,
    // `expected_turnover` must stop patching it.
    let fixture = &read_json("M180_signReportTurnoverX", "GQL-Variables.json")["data"]["turnover"];
    assert_eq!(
        fixture["transactions"][1],
        json!({ "eventLabel": "P", "ticketCount": 3, "amount": 198 })
    );
    let p: Vec<_> = scenario()
        .into_iter()
        .filter(|e| e.label == EventLabel::P)
        .collect();
    assert_eq!(p.len(), 4);
}

#[test]
fn m180_turnover_x() {
    let fixture = &read_json("M180_signReportTurnoverX", "GQL-Variables.json")["data"];
    let events = scenario();
    let ours = to_json(&report_totals::turnover(&events));
    assert_same("turnover", &ours, &expected_turnover(&fixture["turnover"]));
    assert_same(
        "posDevices",
        &to_json(&report_totals::pos_devices(&events)),
        &fixture["posDevices"],
    );
    assert_fdm_devices(
        &to_json(&report_totals::fdm_devices(&events)),
        &fixture["fdmDevices"],
    );
}

fn assert_fdm_devices(ours: &Value, fixture: &Value) {
    let (ours, fixture) = (ours.as_array().unwrap(), fixture.as_array().unwrap());
    assert_eq!(ours.len(), fixture.len(), "fdmDevices");
    for (o, f) in ours.iter().zip(fixture) {
        for key in ["fdmId", "firstTotalCounter", "lastTotalCounter"] {
            assert_eq!(o[key], f[key], "fdmDevices.{key}");
        }
        for key in ["firstFdmDateTime", "lastFdmDateTime"] {
            let at =
                |v: &Value| chrono::DateTime::parse_from_rfc3339(v[key].as_str().unwrap()).unwrap();
            let gap = (at(o) - at(f)).num_seconds().abs();
            assert!(gap <= 10, "fdmDevices.{key}: {} vs {}", o[key], f[key]);
        }
    }
}

#[test]
fn m181_turnover_z_has_the_same_totals() {
    let fixture = &read_json("M181_signReportTurnoverZ", "GQL-Variables.json")["data"];
    let ours = to_json(&report_totals::turnover(&scenario()));
    assert_same("turnover", &ours, &expected_turnover(&fixture["turnover"]));
}

#[test]
fn m182_user_x() {
    let fixture = &read_json("M182_signReportUserX", "GQL-Variables.json")["data"];
    let ours = to_json(&report_totals::users(&scenario()));
    assert_same("users", &ours, &fixture["users"]);
}

#[test]
fn m183_user_z_has_the_same_users() {
    let fixture = &read_json("M183_signReportUserZ", "GQL-Variables.json")["data"];
    let ours = to_json(&report_totals::users(&scenario()));
    assert_same("users", &ours, &fixture["users"]);
}

#[test]
fn m184_combined_z() {
    let fixture = read_json("M184_signReportTurnoverZUserZ", "GQL-Variables.json");
    let events = scenario();
    assert_same(
        "turnover",
        &to_json(&report_totals::turnover(&events)),
        &expected_turnover(&fixture["turnover"]["turnover"]),
    );
    assert_same(
        "users",
        &to_json(&report_totals::users(&events)),
        &fixture["user"]["users"],
    );
}
