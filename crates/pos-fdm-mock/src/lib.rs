//! Mock FDM (SCE 2.0 GraphQL) and mock Scaleway TEM, for development,
//! the admission image, screenshots and the certification evidence.
//!
//! Each [`app`] is one FDM: its own counters per POS, its own settings
//! and request log. Like the module2 cloud simulator, it can require a
//! bearer token and a list of allowed POS, and its errors and warnings
//! can be switched at run time (`/mock/control`) as on the simulator's
//! Developer tab; `/mock/requests` lists the last calls, like its Logs.
//!
//! Its answers have the shape module2's simulator FDM02000425 (FDM
//! software 2.2.0.5) gave on 2026-09-27, recorded in
//! `fixtures/module2/` and checked by `tests/module2_shape.rs`.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha1::Digest;

pub mod storecove;
mod tem;
pub mod tls;

pub use tem::ROUTE as TEM_ROUTE;

/// The FDM's counters for one POS (POS spec v2.2 §2.4, arrêté
/// ministériel n° 64): one per event label, plus the total.
#[derive(Default)]
struct Counters {
    per_label: HashMap<&'static str, i64>,
    total: i64,
}

impl Counters {
    /// Advance the counter of `label` and the total counter, together.
    fn next(&mut self, label: &'static str) -> (i64, i64) {
        let event = self.per_label.entry(label).or_insert(0);
        *event += 1;
        self.total += 1;
        (*event, self.total)
    }
}

/// What the mock simulates. Start-up values come from the environment
/// (`MOCK_FDM_ERROR`, `MOCK_FDM_WARNING`, `MOCK_FDM_BUFFER_PCT`,
/// `MOCK_FDM_INITIALIZED`, `MOCK_FDM_TOKEN`, `MOCK_FDM_ALLOWED_POS`,
/// `MOCK_FDM_PLACEHOLDER_SIGNATURES`);
/// `POST /mock/control` replaces them without a restart.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Control {
    /// Error codes: shown by Status; the critical ones refuse every
    /// sign request (BUFFER_FULL, FDM_LOCKED, ...).
    pub errors: Vec<String>,
    /// Warning codes: shown by Status and in every sign response.
    pub warnings: Vec<String>,
    /// Buffer use in %. Absent: follows the codes (100 when full, 82.53
    /// when near full, else 1.2).
    pub buffer_pct: Option<f64>,
    /// Absent: false when errors are set.
    pub initialized: Option<bool>,
    /// Bearer token required on every request. Absent: none.
    pub token: Option<String>,
    /// POS allowed to sign. Absent: any.
    pub allowed_pos: Option<Vec<String>>,
    /// Sign like module2's online simulator: a `dummy_` placeholder
    /// signature and a short signature not derived from it.
    pub placeholder_signatures: bool,
}

impl Control {
    fn from_env() -> Self {
        let codes = |key: &str| parse_codes(key).unwrap_or_default();
        Self {
            errors: codes("MOCK_FDM_ERROR"),
            warnings: codes("MOCK_FDM_WARNING"),
            buffer_pct: std::env::var("MOCK_FDM_BUFFER_PCT")
                .ok()
                .and_then(|v| v.parse().ok()),
            initialized: std::env::var("MOCK_FDM_INITIALIZED")
                .ok()
                .map(|v| v != "false"),
            token: std::env::var("MOCK_FDM_TOKEN")
                .ok()
                .filter(|t| !t.trim().is_empty()),
            allowed_pos: parse_codes("MOCK_FDM_ALLOWED_POS"),
            placeholder_signatures: std::env::var("MOCK_FDM_PLACEHOLDER_SIGNATURES")
                .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true")),
        }
    }

    fn buffer_pct(&self) -> f64 {
        let has = |code: &str| self.errors.iter().chain(&self.warnings).any(|c| c == code);
        self.buffer_pct.unwrap_or(if has("BUFFER_FULL") {
            100.0
        } else if has("BUFFER_NEAR_FULL") {
            82.53
        } else {
            1.2
        })
    }

    fn initialized(&self) -> bool {
        self.initialized.unwrap_or(self.errors.is_empty())
    }
}

/// One call, as `GET /mock/requests` lists it.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoggedCall {
    at: String,
    operation: String,
    request: serde_json::Value,
    response: serde_json::Value,
}

/// Calls kept by the request log.
const LOG_SIZE: usize = 100;

/// The 5 key fields of a mutation (POS description v2.2, communication
/// rules): posId, posDateTime, terminalId, eventLabel, posFiscalTicketNo.
type MutationKey = (String, String, String, &'static str, i64);

#[derive(Default)]
struct FdmState {
    control: Control,
    counters: HashMap<String, Counters>,
    /// Every signed mutation: its input and its answer, by key fields.
    signed: HashMap<MutationKey, (serde_json::Value, serde_json::Value)>,
    log: VecDeque<LoggedCall>,
}

fn mutation_key(mutation: &str, is_training: bool, data: &serde_json::Value) -> MutationKey {
    let text = |field: &str| {
        data.get(field)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };
    (
        text("posId"),
        text("posDateTime"),
        text("terminalId"),
        event_label(mutation, is_training),
        data.get("posFiscalTicketNo")
            .and_then(|v| v.as_i64())
            .unwrap_or_default(),
    )
}

type Shared = Arc<Mutex<FdmState>>;

/// The mock FDM (GraphQL on `/graphql`, controls on `/mock/*`) and the
/// mock email API ([`TEM_ROUTE`]), as served by the `pos-fdm-mock`
/// binary. Tests mount it on a local port to drive the real POS client
/// against it. Its settings start from the environment.
pub fn app() -> Router {
    app_with(Control::from_env())
}

/// Same as [`app`], with explicit settings.
pub fn app_with(control: Control) -> Router {
    app_with_storecove(control, storecove::StorecoveMock::new())
}

/// Same as [`app_with`], with the Storecove mock (Peppol access point, routes under
/// [`storecove::PREFIX`]) answering from `storecove`, so a test can script and read it.
pub fn app_with_storecove(control: Control, storecove: storecove::StorecoveMock) -> Router {
    let state: Shared = Arc::new(Mutex::new(FdmState {
        control,
        ..Default::default()
    }));
    Router::new()
        .route("/graphql", post(handle_graphql))
        .route("/mock/control", get(get_control).post(set_control))
        .route("/mock/requests", get(get_requests).delete(clear_requests))
        .with_state(state)
        .merge(tem::router())
        .merge(storecove::router(storecove))
}

fn lock(state: &Shared) -> std::sync::MutexGuard<'_, FdmState> {
    state.lock().unwrap_or_else(|e| e.into_inner())
}

/// The settings, the token shown only as set or not.
async fn get_control(State(state): State<Shared>) -> Json<serde_json::Value> {
    let mut control = serde_json::to_value(&lock(&state).control).unwrap_or_default();
    if control["token"].is_string() {
        control["token"] = serde_json::json!("(set)");
    }
    Json(control)
}

async fn set_control(
    State(state): State<Shared>,
    Json(control): Json<Control>,
) -> Json<serde_json::Value> {
    println!("[mock] control: {control:?}");
    lock(&state).control = control;
    Json(serde_json::json!({ "ok": true }))
}

async fn get_requests(State(state): State<Shared>) -> Json<Vec<LoggedCall>> {
    Json(lock(&state).log.iter().cloned().collect())
}

async fn clear_requests(State(state): State<Shared>) -> Json<serde_json::Value> {
    lock(&state).log.clear();
    Json(serde_json::json!({ "ok": true }))
}

#[derive(Deserialize, Serialize)]
struct GqlRequest {
    query: String,
    #[serde(default)]
    variables: serde_json::Value,
}

async fn handle_graphql(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(req): Json<GqlRequest>,
) -> Json<serde_json::Value> {
    let operation = extract_mutation_name(&req.query);
    let mut fdm = lock(&state);
    let response = answer(&mut fdm, &headers, &operation, &req);
    if fdm.log.len() == LOG_SIZE {
        fdm.log.pop_front();
    }
    fdm.log.push_back(LoggedCall {
        at: chrono::Utc::now().to_rfc3339(),
        operation,
        request: serde_json::to_value(&req).unwrap_or_default(),
        response: response.clone(),
    });
    Json(response)
}

/// What the mock reports as its FDM software version.
const FDM_SW_VERSION: &str = "MOCK-FDM-1.0";

/// The mock's serial number: 11 characters, as the spec's fdmId
/// (example `SPF01987654`).
const FDM_ID: &str = "MCK00000001";

/// module2 adds an `info` object next to `data` (and next to GraphQL
/// validation errors); the POS ignores it.
fn with_info(mut body: serde_json::Value) -> serde_json::Value {
    body["info"] = serde_json::json!({ "https": false, "version": FDM_SW_VERSION });
    body
}

/// The first root field of a GraphQL document (past an alias), with
/// its 1-based line and column.
fn first_root_field(query: &str) -> Option<(String, usize, usize)> {
    let open = query.find('{')?;
    let mut at = open + 1;
    let ident_at = |from: usize| -> Option<(usize, String)> {
        let start = from + query[from..].find(|c: char| !c.is_whitespace())?;
        let name: String = query[start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        (!name.is_empty()).then_some((start, name))
    };
    let (mut start, mut name) = ident_at(at)?;
    at = start + name.len();
    if query[at..].trim_start().starts_with(':') {
        let colon = at + query[at..].find(':')?;
        (start, name) = ident_at(colon + 1)?;
    }
    let before = &query[..start];
    let line = before.matches('\n').count() + 1;
    let column = start - before.rfind('\n').map_or(0, |n| n + 1) + 1;
    Some((name, line, column))
}

/// The sign mutations the mock knows.
const SIGN_MUTATIONS: [&str; 15] = [
    "signSale",
    "signOrder",
    "signPreBill",
    "signWorkIn",
    "signWorkOut",
    "signCopy",
    "signInvoice",
    "signMoneyInOut",
    "signDrawerOpen",
    "signPaymentCorrection",
    "signCostCenterChange",
    "signReportTurnoverX",
    "signReportTurnoverZ",
    "signReportUserX",
    "signReportUserZ",
];

/// A GraphQL validation error for a root field the schema does not
/// have, worded like module2's (graphene) server. The status field is
/// spelled `status` there, as in the Q100/Q110 fixtures.
fn unknown_field(query: &str) -> Option<serde_json::Value> {
    let (field, line, column) = first_root_field(query)?;
    let is_mutation = query.trim_start().starts_with("mutation");
    let known = if is_mutation {
        SIGN_MUTATIONS.contains(&field.as_str())
    } else {
        field == "status"
    };
    if known {
        return None;
    }
    let root = if is_mutation { "Mutation" } else { "Query" };
    let hint = if field.eq_ignore_ascii_case("status") && !is_mutation {
        " Did you mean \"status\"?"
    } else {
        ""
    };
    Some(with_info(serde_json::json!({ "errors": [{
        "message": format!("Cannot query field \"{field}\" on type \"{root}\".{hint}"),
        "locations": [{ "line": line, "column": column }],
    }] })))
}

/// module2's check of `posDateTime`: seconds, at most milliseconds,
/// and the Belgian offset of that moment.
fn pos_date_time_is_valid(value: &str) -> bool {
    let Ok(at) = chrono::DateTime::parse_from_rfc3339(value) else {
        return false;
    };
    let rest = &value[19..];
    let (digits, offset) = match rest.strip_prefix('.') {
        Some(fraction) => {
            let n = fraction.chars().take_while(char::is_ascii_digit).count();
            (n, &fraction[n..])
        }
        None => (0, rest),
    };
    let expected = if brussels_summer_time(at.with_timezone(&chrono::Utc)) {
        "+02:00"
    } else {
        "+01:00"
    };
    digits <= 3 && offset == expected
}

/// EU summer time: from 01:00 UTC on the last Sunday of March to 01:00
/// UTC on the last Sunday of October.
fn brussels_summer_time(at: chrono::DateTime<chrono::Utc>) -> bool {
    use chrono::Datelike;
    let last_sunday = |month: u32| {
        let last = chrono::NaiveDate::from_ymd_opt(at.year(), month, 31).unwrap();
        let back = chrono::Days::new(u64::from(last.weekday().num_days_from_sunday()));
        (last - back).and_hms_opt(1, 0, 0).unwrap().and_utc()
    };
    at >= last_sunday(3) && at < last_sunday(10)
}

/// module2's refusal of an ill-formed `posDateTime`.
fn invalid_pos_date_time() -> serde_json::Value {
    invalid_request(
        "posDateTime is not a valid ISO 8601 date-time string. +01:00 is only allowed during winter time, +02:00 is only allowed during summer time.",
    )
}

/// An `INVALID_REQUEST` refusal: module2 uses that code for every
/// field it rejects (FDM software 2.1.1.6).
fn invalid_request(message: &str) -> serde_json::Value {
    serde_json::json!({ "errors": [{
        "message": message,
        "extensions": {
            "category": code_category("INVALID_REQUEST"),
            "code": "INVALID_REQUEST",
            "showPos": code_show_pos("INVALID_REQUEST"),
        },
    }] })
}

/// Amounts: multiples of 0.01 (POS description v2.2, "multiple de
/// 0.01" / "deux décimales").
const CENT_FIELDS: [&str; 13] = [
    "amount",
    "lineTotal",
    "transactionTotal",
    "price",
    "rate",
    "taxableAmount",
    "vatAmount",
    "totalAmount",
    "normalAmount",
    "negativeCorrections",
    "positiveCorrections",
    "negative",
    "positive",
];

/// Quantities and unit prices: multiples of 0.0001.
const TEN_THOUSANDTH_FIELDS: [&str; 2] = ["quantity", "unitPrice"];

fn is_multiple_of(value: f64, steps_per_unit: f64) -> bool {
    let scaled = value * steps_per_unit;
    (scaled - scaled.round()).abs() < 1e-6
}

/// The field checks module2's simulator makes on a sign input (its
/// changelog, FDM software 2.1.0.4 to 2.1.2.0, and the POS
/// description v2.2): amounts in cents, quantities and unit prices in
/// ten-thousandths, `subProducts` null on a single product and present
/// on a composite one, a `negQuantityReason` on every negative
/// quantity, a `costCenter` on `signOrder`, and every cited `fdmRef`
/// with an 11-character `fdmId` and a UTC `fdmDateTime`. The first
/// problem found, as the refusal's message.
fn sign_input_problem(mutation: &str, data: &serde_json::Value) -> Option<String> {
    if mutation == "signOrder" && data.get("costCenter").is_none_or(|c| c.is_null()) {
        return Some("costCenter is required for signOrder.".into());
    }
    let mut problem = None;
    check_fields(data, "data", &mut problem);
    problem
}

fn check_fields(value: &serde_json::Value, path: &str, problem: &mut Option<String>) {
    if problem.is_some() {
        return;
    }
    match value {
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                let here = format!("{path}.{key}");
                if let Some(number) = item.as_f64() {
                    let steps = if CENT_FIELDS.contains(&key.as_str()) {
                        100.0
                    } else if TEN_THOUSANDTH_FIELDS.contains(&key.as_str()) {
                        10_000.0
                    } else {
                        continue;
                    };
                    if !is_multiple_of(number, steps) {
                        *problem = Some(format!("{here} must be a multiple of {}.", 1.0 / steps));
                        return;
                    }
                    continue;
                }
                if key == "fdmRef" && !item.is_null() {
                    check_fdm_ref(item, &here, problem);
                } else if key == "fdmRefs" {
                    for (i, fdm_ref) in item.as_array().into_iter().flatten().enumerate() {
                        check_fdm_ref(fdm_ref, &format!("{here}[{i}]"), problem);
                    }
                }
                check_fields(item, &here, problem);
            }
            check_line(map, path, problem);
            check_product(map, path, problem);
        }
        serde_json::Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                check_fields(item, &format!("{path}[{i}]"), problem);
            }
        }
        _ => {}
    }
}

/// A transaction line: `subProducts` null on SINGLE_PRODUCT, present
/// on COMPOSITE_PRODUCT.
fn check_line(
    line: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    problem: &mut Option<String>,
) {
    let sub = line.get("subProducts").filter(|s| !s.is_null());
    let message = match line.get("lineType").and_then(|t| t.as_str()) {
        Some("SINGLE_PRODUCT") if sub.is_some() => "must be null for lineType SINGLE_PRODUCT",
        Some("COMPOSITE_PRODUCT") if sub.is_none() => "is required for lineType COMPOSITE_PRODUCT",
        _ => return,
    };
    problem.get_or_insert(format!("{path}.subProducts {message}."));
}

/// A product with a negative quantity says why.
fn check_product(
    product: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    problem: &mut Option<String>,
) {
    let negative = product
        .get("quantity")
        .and_then(|q| q.as_f64())
        .is_some_and(|q| q < 0.0);
    let reason = product
        .get("negQuantityReason")
        .is_some_and(|r| !r.is_null());
    if negative && !reason {
        problem.get_or_insert(format!(
            "{path}.negQuantityReason is required when quantity is negative."
        ));
    }
}

/// A cited event: an 11-character `fdmId`, a UTC `fdmDateTime`.
fn check_fdm_ref(fdm_ref: &serde_json::Value, path: &str, problem: &mut Option<String>) {
    if problem.is_some() {
        return;
    }
    let fdm_id = fdm_ref.get("fdmId").and_then(|v| v.as_str()).unwrap_or("");
    if fdm_id.chars().count() != 11 {
        *problem = Some(format!("{path}.fdmId must be 11 characters long."));
        return;
    }
    let at = fdm_ref
        .get("fdmDateTime")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !at.ends_with('Z') || chrono::DateTime::parse_from_rfc3339(at).is_err() {
        *problem = Some(format!(
            "{path}.fdmDateTime must be an RFC 3339 date-time in UTC (Z)."
        ));
    }
}

/// The FDM's answer to one GraphQL request.
fn answer(
    fdm: &mut FdmState,
    headers: &HeaderMap,
    operation: &str,
    req: &GqlRequest,
) -> serde_json::Value {
    let control = fdm.control.clone();
    if let Some(token) = &control.token {
        let sent = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        if sent != Some(token.as_str()) {
            // module2: no code, no `data`, no `info`.
            return serde_json::json!({ "errors": [{ "message": "Not a valid Token" }] });
        }
    }

    if let Some(refusal) = unknown_field(&req.query) {
        return refusal;
    }

    if operation == "Status" {
        return status_response(&req.query, &control);
    }

    // A critical error refuses every sign request (as the real FDM).
    let critical: Vec<String> = control
        .errors
        .iter()
        .filter(|c| is_critical(c))
        .cloned()
        .collect();
    if !critical.is_empty() {
        return error_response(&control.errors);
    }

    let is_training = req
        .variables
        .get("isTraining")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // M184 - the end-of-day Z carries BOTH Z mutations in one GraphQL
    // document (`$turnover` + `$user` variables). Each gets its own
    // counters, like a real FDM signing two events back to back.
    let parts: Vec<(&str, serde_json::Value)> =
        if req.query.contains("signReportTurnoverZ") && req.query.contains("signReportUserZ") {
            vec![
                (
                    "signReportTurnoverZ",
                    req.variables.get("turnover").cloned().unwrap_or_default(),
                ),
                (
                    "signReportUserZ",
                    req.variables.get("user").cloned().unwrap_or_default(),
                ),
            ]
        } else {
            vec![(
                operation,
                req.variables.get("data").cloned().unwrap_or_default(),
            )]
        };

    if let Some(allowed) = &control.allowed_pos {
        let unknown = parts.iter().any(|(_, data)| {
            let pos_id = data.get("posId").and_then(|v| v.as_str()).unwrap_or("");
            !allowed.iter().any(|a| a == pos_id)
        });
        if unknown {
            return error_response(&["UNKNOWN_POS".to_string()]);
        }
    }

    let bad_date = parts.iter().any(|(_, data)| {
        data.get("posDateTime")
            .and_then(|v| v.as_str())
            .is_some_and(|at| !pos_date_time_is_valid(at))
    });
    if bad_date {
        return invalid_pos_date_time();
    }
    if let Some(problem) = parts
        .iter()
        .find_map(|(mutation, data)| sign_input_problem(mutation, data))
    {
        return invalid_request(&problem);
    }

    // Same key fields: the same content gets its first answer back,
    // other content is refused (POS description v2.2, communication
    // rules).
    let keys: Vec<MutationKey> = parts
        .iter()
        .map(|(mutation, data)| mutation_key(mutation, is_training, data))
        .collect();
    let conflict = parts.iter().zip(&keys).any(|((_, data), key)| {
        fdm.signed
            .get(key)
            .is_some_and(|(signed, _)| signed != data)
    });
    if conflict {
        return invalid_request(
            "A mutation with the same key fields but other values was already processed.",
        );
    }

    let mut data_map = serde_json::Map::new();
    for ((mutation, data), key) in parts.into_iter().zip(keys) {
        if let Some((_, answer)) = fdm.signed.get(&key) {
            let mut answer = answer.clone();
            answer["warnings"] = codes_to_messages(vec!["DUPLICATE_REQUEST".into()], "FR");
            println!("[dup] {mutation} #{}", key.4);
            data_map.insert(mutation.to_string(), answer);
            continue;
        }
        let pos_id = data
            .get("posId")
            .and_then(|v| v.as_str())
            .unwrap_or("CFOD0061234567")
            .to_string();
        let counters = fdm.counters.entry(pos_id).or_default();
        let mut result = build_sign_result(mutation, is_training, &data, counters);
        result.buffer_capacity_used = control.buffer_pct();
        if control.placeholder_signatures {
            as_simulator_signs(&mut result);
        }
        result.warnings = codes_to_messages(control.warnings.clone(), "FR");
        println!(
            "[{}] {mutation} #{} (total: {})",
            result.fdm_ref.event_label, result.fdm_ref.event_counter, result.fdm_ref.total_counter
        );
        let answer = serde_json::to_value(&result).unwrap_or_default();
        fdm.signed.insert(key, (data, answer.clone()));
        data_map.insert(mutation.to_string(), answer);
    }
    with_info(serde_json::json!({ "data": data_map }))
}

/// Replace a signature by the simulator's kind: `dummy_` and 38
/// base64 characters, with a short signature (N only) that is 40 hex
/// characters but not derived from it, as module2's simulator
/// FDM02000425 (FDM software 2.2.0.5) answered on 2026-09-27.
fn as_simulator_signs(result: &mut SignResult) {
    let seed = result.digital_signature.clone().into_bytes();
    let seed = seed.as_slice();
    let mut raw = sha1::Sha1::digest(seed).to_vec();
    raw.extend_from_slice(&sha1::Sha1::digest([seed, b"-more"].concat())[..8]);
    result.digital_signature = format!(
        "dummy_{}",
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(raw)
    );
    if result.short_signature.is_some() {
        let unrelated = sha1::Sha1::digest([seed, b"-short"].concat());
        result.short_signature = Some(hex::encode(unrelated));
    }
}

/// The label of the event a mutation signs. Every event signed in
/// training mode is a T event, whatever the mutation (UC M170 work-in,
/// M171 sale and M172 copy all answer T).
fn event_label(mutation_name: &str, is_training: bool) -> &'static str {
    match mutation_name {
        _ if is_training => "T",
        "signSale" => "N",
        "signOrder" | "signPreBill" | "signCostCenterChange" => "P",
        "signWorkIn" | "signWorkOut" => "S",
        "signCopy" => "C",
        "signInvoice" => "I",
        "signMoneyInOut" | "signDrawerOpen" | "signPaymentCorrection" => "F",
        "signReportTurnoverX" | "signReportTurnoverZ" | "signReportUserX" | "signReportUserZ" => {
            "R"
        }
        _ => "N",
    }
}

/// Build one signed result for a mutation, advancing the counters.
fn build_sign_result(
    mutation_name: &str,
    is_training: bool,
    data: &serde_json::Value,
    counters: &mut Counters,
) -> SignResult {
    let event_label = event_label(mutation_name, is_training);

    let event_op = match mutation_name {
        "signSale" => "SALE",
        "signOrder" => "ORDER",
        "signPreBill" => "PRE_BILL",
        "signWorkIn" => "WORK_IN",
        "signWorkOut" => "WORK_OUT",
        "signCopy" => "COPY",
        "signInvoice" => "INVOICE",
        "signMoneyInOut" => "MONEY_IN_OUT",
        "signDrawerOpen" => "DRAWER_OPEN",
        "signPaymentCorrection" => "PAYMENT_CORRECTION",
        "signCostCenterChange" => "COST_CENTER_CHANGE",
        "signReportTurnoverX" => "REPORT_TURNOVER_X",
        "signReportTurnoverZ" => "REPORT_TURNOVER_Z",
        "signReportUserX" => "REPORT_USER_X",
        "signReportUserZ" => "REPORT_USER_Z",
        _ => "SALE",
    };

    let (ec, tc) = counters.next(event_label);

    let pos_id = data
        .get("posId")
        .and_then(|v| v.as_str())
        .unwrap_or("CFOD0061234567");
    let ticket_no = data
        .get("posFiscalTicketNo")
        .and_then(|v| v.as_i64())
        .unwrap_or(ec);
    let pos_dt = data
        .get("posDateTime")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let terminal_id = data.get("terminalId").and_then(|v| v.as_str());
    let device_id = data
        .get("deviceId")
        .and_then(|v| v.as_str())
        .unwrap_or("DEV001");

    let sig_raw = format!("mock-sig-{ec}-{tc}-{}", chrono::Utc::now().timestamp());
    let digital_sig = base64::engine::general_purpose::STANDARD.encode(sig_raw.as_bytes());

    let short_sig = if event_label == "N" {
        let hash = sha1::Sha1::digest(sig_raw.as_bytes());
        Some(hex::encode(hash))
    } else {
        None
    };

    let verification_url = if event_label == "N" {
        Some(format!("https://www.gs2.be/M{ec:05}{tc:05}AbCdEfGhI"))
    } else {
        None
    };

    // VAT for sales; none on a training event (M171B fixture, module2).
    let vat_calc = if event_label == "N" {
        data.get("transaction").map(compute_vat_calc)
    } else {
        None
    };

    SignResult {
        pos_id: pos_id.to_string(),
        pos_fiscal_ticket_no: ticket_no,
        pos_date_time: pos_dt.to_string(),
        terminal_id: terminal_id.map(|s| s.to_string()),
        device_id: device_id.to_string(),
        event_operation: event_op.to_string(),
        fdm_ref: FdmRef {
            fdm_id: FDM_ID.to_string(),
            fdm_date_time: fdm_date_time(),
            event_label: event_label.to_string(),
            event_counter: ec,
            total_counter: tc,
        },
        fdm_sw_version: FDM_SW_VERSION.to_string(),
        digital_signature: digital_sig,
        short_signature: short_sig,
        verification_url,
        vat_calc,
        buffer_capacity_used: 1.2,
        warnings: serde_json::json!([]),
        informations: serde_json::json!([]),
        // module2 and the official fixtures send no footer lines.
        footer: Vec::new(),
    }
}

/// The FDM's clock as module2 gives it: UTC, seconds, `Z`.
fn fdm_date_time() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn extract_mutation_name(query: &str) -> String {
    // The status field (`status`, as module2 and the Q100/Q110
    // fixtures spell it) routes to the status handler.
    if query.contains("status(") {
        return "Status".to_string();
    }
    // Find pattern: mutationName( or mutationName(
    let mutations = [
        "signSale",
        "signOrder",
        "signPreBill",
        "signWorkIn",
        "signWorkOut",
        "signCopy",
        "signInvoice",
        "signMoneyInOut",
        "signDrawerOpen",
        "signPaymentCorrection",
        "signCostCenterChange",
        "signReportTurnoverX",
        "signReportTurnoverZ",
        "signReportUserX",
        "signReportUserZ",
        "Status",
    ];
    for m in mutations {
        if query.contains(m) {
            return m.to_string();
        }
    }
    "unknown".to_string()
}

fn compute_vat_calc(txn: &serde_json::Value) -> Vec<VatCalcItem> {
    let mut totals: std::collections::HashMap<String, f64> = std::collections::HashMap::new();

    if let Some(lines) = txn.get("transactionLines").and_then(|v| v.as_array()) {
        for line in lines {
            if let Some(vats) = line
                .get("mainProduct")
                .and_then(|p| p.get("vats"))
                .and_then(|v| v.as_array())
            {
                for vat in vats {
                    let label = vat.get("label").and_then(|v| v.as_str()).unwrap_or("B");
                    let price = vat.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    *totals.entry(label.to_string()).or_default() += price;
                }
            }
        }
    }

    let rates: std::collections::HashMap<&str, f64> =
        [("A", 21.0), ("B", 12.0), ("C", 6.0), ("D", 0.0), ("X", 0.0)].into();

    totals
        .iter()
        .map(|(label, &total)| {
            let rate = rates.get(label.as_str()).copied().unwrap_or(0.0);
            let taxable = if rate > 0.0 {
                (total / (1.0 + rate / 100.0) * 100.0).round() / 100.0
            } else {
                total
            };
            let vat_amount = ((total - taxable) * 100.0).round() / 100.0;
            VatCalcItem {
                label: label.clone(),
                rate,
                taxable_amount: taxable,
                vat_amount,
                total_amount: total,
                out_of_scope: label == "X",
            }
        })
        .collect()
}

/// All status-field occurrences in the query, in source order, as
/// `(response_key, language)`. Handles both spec spellings and GraphQL
/// aliases, so a Q110-style multi-language query
/// (`status(language: EN) … messagesFR: status(language: FR) …`)
/// yields one data key per alias.
fn status_occurrences(query: &str) -> Vec<(String, String)> {
    let mut found: Vec<(usize, String, String)> = Vec::new();
    for field in ["Status(", "status("] {
        for (pos, _) in query.match_indices(field) {
            // "fdmStatus: Status(" also contains "tatus(" inside the
            // alias - only accept occurrences where the char before
            // the match is not alphanumeric (i.e. a real field name).
            if pos > 0
                && query[..pos]
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_alphanumeric())
            {
                continue;
            }
            let before = query[..pos].trim_end();
            let key = match before.strip_suffix(':') {
                Some(prefix) => prefix
                    .split_whitespace()
                    .last()
                    .unwrap_or(field.trim_end_matches('('))
                    .to_string(),
                None => field.trim_end_matches('(').to_string(),
            };
            let rest = &query[pos..];
            let lang = rest
                .split_once("language:")
                .and_then(|(_, r)| r.split(')').next())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "FR".to_string());
            found.push((pos, key, lang));
        }
    }
    found.sort_by_key(|(pos, _, _)| *pos);
    found.into_iter().map(|(_, k, l)| (k, l)).collect()
}

fn status_response(query: &str, control: &Control) -> serde_json::Value {
    let occurrences = status_occurrences(query);
    let occurrences = if occurrences.is_empty() {
        vec![("Status".to_string(), "FR".to_string())]
    } else {
        occurrences
    };

    // A full buffer is also nearly full (module2 lists both).
    let mut warnings = control.warnings.clone();
    if control.errors.iter().any(|c| c == "BUFFER_FULL")
        && !warnings.iter().any(|c| c == "BUFFER_NEAR_FULL")
    {
        warnings.push("BUFFER_NEAR_FULL".to_string());
    }
    let mut data = serde_json::Map::new();
    for (key, lang) in occurrences {
        data.insert(
            key,
            serde_json::json!({
                "device": {
                    "fdmId": FDM_ID,
                    "fdmSwVersion": FDM_SW_VERSION,
                    "fdmDateTime": fdm_date_time(),
                    // Coherent with the codes, so §V evidence is
                    // self-consistent (BUFFER_NEAR_FULL means > 70%).
                    "bufferCapacityUsed": control.buffer_pct()
                },
                "initialized": control.initialized(),
                "informations": [],
                "warnings": codes_to_messages(warnings.clone(), &lang),
                "errors": codes_to_messages(control.errors.clone(), &lang)
            }),
        );
    }
    with_info(serde_json::json!({ "data": data }))
}

fn parse_codes(env_key: &str) -> Option<Vec<String>> {
    let raw = std::env::var(env_key).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(
        trimmed
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

fn codes_to_messages(codes: Vec<String>, lang: &str) -> serde_json::Value {
    let items: Vec<serde_json::Value> = codes
        .into_iter()
        .map(|code| {
            serde_json::json!({
                "message": code_to_human(&code, lang),
                "extensions": {
                    "category": code_category(&code),
                    "code": code,
                    "showPos": code_show_pos(&code),
                }
            })
        })
        .collect();
    serde_json::Value::Array(items)
}

/// `extensions.category` per the POS spec v2.2 is the message ORIGIN
/// (`SPF_FOD` | `FDM` | `OTHER`), not its severity - severity is
/// implied by which array (warnings/errors/informations) carries the
/// message. The category is the one of the tb_messages table (SPF_FOD
/// where it names two).
fn code_category(code: &str) -> &'static str {
    match code {
        "CLIENT_CERT_NEAR_EXPIRATION"
        | "CLIENT_CERT_EXPIRED"
        | "RTC_SYNC_FAILED"
        | "UPDATE_URLS_FAILED"
        | "UPDATE_TRUST_CERT_FAILED"
        | "UPDATE_TASK_LIST_FAILED"
        | "TASK_FEEDBACK_FAILED"
        | "NOP_FAILED"
        | "BUFFER_NEAR_FULL"
        | "SERVER_CERT_RESOLVE_FAILED"
        | "TRANSACTION_UPLOAD_FAILED"
        | "INITIALIZATION_FAILED"
        | "RTC_NOT_INITIALIZED"
        | "CORRUPT_RECORD_ENCOUNTERED"
        | "UPDATE_PARAMS_FAILED"
        | "UPDATE_CLIENT_CERT_FAILED"
        | "UPDATE_VAT_RATES_FAILED"
        | "UPDATE_POS_ALLOWLIST_FAILED"
        | "TOO_MANY_MEMORY_ERRORS"
        | "BUFFER_FULL"
        | "FDM_LOCKED"
        | "UNDEFINED_OTHER"
        | "UPDATE_POS_VATNO"
        | "UPDATE_POS_ESTNO" => "SPF_FOD",
        // POS description v2.2 tb_messages: raised by the FDM itself.
        "DUPLICATE_REQUEST"
        | "UNAUTHORIZED"
        | "INVALID_REQUEST"
        | "INTERNAL_ERROR"
        | "UNDEFINED_ERROR"
        | "FDM_NOT_OPERATIONAL"
        | "UNKNOWN_POS" => "FDM",
        _ => "OTHER",
    }
}

/// `showPos` - display obligation per the §V table of the
/// certification manual (affichage M/O/N).
fn code_show_pos(code: &str) -> &'static str {
    match code {
        // Mandatory display
        "CLIENT_CERT_EXPIRED"
        | "BUFFER_NEAR_FULL"
        | "BUFFER_FULL"
        | "FDM_LOCKED"
        | "FDM_NOT_OPERATIONAL"
        | "UNKNOWN_POS"
        | "UPDATE_POS_VATNO"
        | "UPDATE_POS_ESTNO" => "MANDATORY",
        // Never displayed on the POS
        "RTC_NOT_INITIALIZED"
        | "CORRUPT_RECORD_ENCOUNTERED"
        | "UPDATE_PARAMS_FAILED"
        | "UPDATE_CLIENT_CERT_FAILED"
        | "UPDATE_VAT_RATES_FAILED"
        | "UPDATE_POS_ALLOWLIST_FAILED"
        | "DUPLICATE_REQUEST"
        | "TOO_MANY_MEMORY_ERRORS" => "NEVER",
        // Everything else in the table is optional display
        _ => "OPTIONAL",
    }
}

/// Human message for a code, localized like a real FDM would (Q110).
/// The frequently-demoed codes carry full NL/DE/EN translations; the
/// long tail falls back to the FR table, which is what the intake
/// examiner reads anyway.
fn code_to_human(code: &str, lang: &str) -> &'static str {
    match (code, lang) {
        ("BUFFER_NEAR_FULL", "NL") => "Het buffergebruik van de FDM overschrijdt 70%",
        ("BUFFER_NEAR_FULL", "DE") => "Die Pufferauslastung des FDM übersteigt 70%",
        ("BUFFER_NEAR_FULL", "EN") => "FDM buffer usage exceeds 70%",
        ("BUFFER_FULL", "NL") => "De buffer van de FDM is vol",
        ("BUFFER_FULL", "DE") => "Der Puffer des FDM ist voll",
        ("BUFFER_FULL", "EN") => "The FDM buffer is full",
        ("CLIENT_CERT_EXPIRED", "NL") => "Het certificaat van de klant is vervallen",
        ("CLIENT_CERT_EXPIRED", "DE") => "Das Zertifikat des Kunden ist abgelaufen",
        ("CLIENT_CERT_EXPIRED", "EN") => "The client certificate has expired",
        ("FDM_NOT_OPERATIONAL", "NL") => "De FDM werkt niet",
        ("FDM_NOT_OPERATIONAL", "DE") => "Das FDM funktioniert nicht",
        ("FDM_NOT_OPERATIONAL", "EN") => "The FDM is not operational",
        ("FDM_LOCKED", "NL") => "De FDM is vergrendeld",
        ("FDM_LOCKED", "DE") => "Das FDM ist gesperrt",
        ("FDM_LOCKED", "EN") => "The FDM is locked",
        _ => code_to_human_fr(code),
    }
}

fn code_to_human_fr(code: &str) -> &'static str {
    match code {
        "CLIENT_CERT_NEAR_EXPIRATION" => "Le certificat du client expire dans moins de 4 mois",
        "CLIENT_CERT_EXPIRED" => "Le certificat du client a expiré",
        "RTC_SYNC_FAILED" => "Le FDM n'a pas pu synchroniser l'horloge en temps réel",
        "UPDATE_URLS_FAILED" => "Le FDM n'a pas pu mettre à jour les URLs",
        "UPDATE_TRUST_CERT_FAILED" => "Le FDM échoue à mettre à jour les certificats de confiance",
        "UPDATE_TASK_LIST_FAILED" => "Le FDM n'a pas pu télécharger la liste de tâches",
        "TASK_FEEDBACK_FAILED" => "Le FDM n'a pas pu signaler le résultat de la tâche",
        "NOP_FAILED" => "Le FDM n'a pas pu poster un message NOP",
        "BUFFER_NEAR_FULL" => "L'utilisation du buffer du FDM dépasse 70%",
        "SERVER_CERT_RESOLVE_FAILED" => "Le FDM n'a pas pu résoudre le certificat du serveur",
        "TRANSACTION_UPLOAD_FAILED" => {
            "Le FDM a tenté plus de 10 fois de poster une transaction sans succès"
        }
        "INITIALIZATION_FAILED" => "L'initialisation du FDM a échoué",
        "BUFFER_FULL" => "Le buffer du FDM est plein",
        "FDM_LOCKED" => "Le FDM est verrouillé",
        "UNAUTHORIZED" => "Le FDM a reçu des requêtes non-autorisées",
        "INVALID_REQUEST" => "Le FDM n'a pas pu traiter la requête",
        "INTERNAL_ERROR" => "Le FDM a rencontré un problème technique",
        "UNDEFINED_ERROR" => "Le FDM a rencontré une erreur indéfinie",
        "UNDEFINED_OTHER" => "Toute autre erreur rencontrée",
        "FDM_NOT_OPERATIONAL" => "Le FDM ne fonctionne pas",
        "UNKNOWN_POS" => {
            "Le FDM a reçu un message d'un POS non repris dans la liste d'autorisation"
        }
        "UPDATE_POS_VATNO" => {
            "Le numéro de TVA du POS ne correspond pas au numéro de TVA connu par le FDM"
        }
        "UPDATE_POS_ESTNO" => {
            "Le numéro d'établissement du POS ne correspond pas à celui connu par le FDM"
        }
        _ => "Erreur FDM",
    }
}

/// Codes that stop the FDM from signing.
fn is_critical(code: &str) -> bool {
    matches!(
        code,
        "BUFFER_FULL"
            | "FDM_LOCKED"
            | "FDM_NOT_OPERATIONAL"
            | "UNKNOWN_POS"
            | "UNAUTHORIZED"
            | "INVALID_REQUEST"
            | "INTERNAL_ERROR"
            | "UNDEFINED_ERROR"
    )
}

/// A refusal shaped as the POS description v2.2 error example: free
/// text in `message`, the code in `extensions`.
fn error_response(codes: &[String]) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = codes
        .iter()
        .map(|code| {
            serde_json::json!({
                "message": code_to_human(code, "FR"),
                "extensions": {
                    "category": code_category(code),
                    "code": code,
                    "showPos": code_show_pos(code),
                },
            })
        })
        .collect();
    // No `data` key on a refusal, as module2.
    serde_json::json!({ "errors": messages })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SignResult {
    pos_id: String,
    pos_fiscal_ticket_no: i64,
    pos_date_time: String,
    terminal_id: Option<String>,
    device_id: String,
    event_operation: String,
    fdm_ref: FdmRef,
    fdm_sw_version: String,
    digital_signature: String,
    short_signature: Option<String>,
    verification_url: Option<String>,
    vat_calc: Option<Vec<VatCalcItem>>,
    buffer_capacity_used: f64,
    warnings: serde_json::Value,
    informations: serde_json::Value,
    footer: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FdmRef {
    fdm_id: String,
    fdm_date_time: String,
    event_label: String,
    event_counter: i64,
    total_counter: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VatCalcItem {
    label: String,
    rate: f64,
    taxable_amount: f64,
    vat_amount: f64,
    total_amount: f64,
    out_of_scope: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mutation_names_are_recognised() {
        for m in [
            "signSale",
            "signOrder",
            "signPreBill",
            "signWorkIn",
            "signWorkOut",
            "signCopy",
            "signMoneyInOut",
            "signDrawerOpen",
            "signReportTurnoverX",
            "signReportTurnoverZ",
            "signReportUserX",
            "signReportUserZ",
        ] {
            let q = format!(
                "mutation($data: X!, $isTraining: Boolean!) {{ {m}(data: $data, isTraining: $isTraining) {{ posId }} }}"
            );
            assert_eq!(extract_mutation_name(&q), m);
        }
        assert_eq!(
            extract_mutation_name("query { status(language: FR) { initialized } }"),
            "Status"
        );
        assert_eq!(
            extract_mutation_name("query { status(language: NL) { initialized } }"),
            "Status"
        );
        assert_eq!(extract_mutation_name("query { nothing }"), "unknown");
    }

    #[test]
    fn labels_follow_the_mutation_and_training_is_always_t() {
        for (m, label) in [
            ("signSale", "N"),
            ("signOrder", "P"),
            ("signPreBill", "P"),
            ("signWorkIn", "S"),
            ("signWorkOut", "S"),
            ("signCopy", "C"),
            ("signMoneyInOut", "F"),
            ("signDrawerOpen", "F"),
            ("signReportTurnoverZ", "R"),
            ("signReportUserX", "R"),
        ] {
            let r = build_sign_result(m, false, &json!({}), &mut Counters::default());
            assert_eq!(r.fdm_ref.event_label, label, "{m}");
            let t = build_sign_result(m, true, &json!({}), &mut Counters::default());
            assert_eq!(t.fdm_ref.event_label, "T", "{m} in training");
            assert!(t.short_signature.is_none() && t.verification_url.is_none());
        }
    }

    #[test]
    fn only_n_events_carry_a_short_signature_and_a_verification_url() {
        let n = build_sign_result("signSale", false, &json!({}), &mut Counters::default());
        let sig = n.short_signature.expect("N has a short signature");
        assert_eq!(sig.len(), 40);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(n.verification_url.unwrap().starts_with("https://"));
        for m in ["signOrder", "signCopy", "signWorkIn", "signReportTurnoverX"] {
            let r = build_sign_result(m, false, &json!({}), &mut Counters::default());
            assert!(r.short_signature.is_none(), "{m}");
            assert!(r.verification_url.is_none(), "{m}");
        }
    }

    fn work_in(ticket_no: i64, insz: &str) -> GqlRequest {
        GqlRequest {
            query: "mutation($data: WorkInOutInput!, $isTraining: Boolean!) { signWorkIn(data: $data, isTraining: $isTraining) { posId } }".into(),
            variables: json!({ "isTraining": false, "data": {
                "posId": "CPOS0031234567",
                "posFiscalTicketNo": ticket_no,
                "posDateTime": "2026-07-03T12:34:56+02:00",
                "terminalId": "T1",
                "deviceId": "D1",
                "employeeId": insz,
            }}),
        }
    }

    #[test]
    fn the_same_request_again_gets_its_first_answer() {
        let mut fdm = FdmState::default();
        let first = answer(&mut fdm, &HeaderMap::new(), "signWorkIn", &work_in(1, "1"));
        let again = answer(&mut fdm, &HeaderMap::new(), "signWorkIn", &work_in(1, "1"));
        let (a, b) = (&first["data"]["signWorkIn"], &again["data"]["signWorkIn"]);
        assert_eq!(a["fdmRef"], b["fdmRef"]);
        assert_eq!(a["digitalSignature"], b["digitalSignature"]);
        assert_eq!(b["warnings"][0]["extensions"]["code"], "DUPLICATE_REQUEST");
        assert_eq!(b["warnings"][0]["extensions"]["showPos"], "NEVER");

        let next = answer(&mut fdm, &HeaderMap::new(), "signWorkIn", &work_in(2, "1"));
        assert_eq!(next["data"]["signWorkIn"]["fdmRef"]["eventCounter"], 2);
    }

    #[test]
    fn the_same_key_fields_with_other_content_are_refused() {
        let mut fdm = FdmState::default();
        answer(&mut fdm, &HeaderMap::new(), "signWorkIn", &work_in(1, "1"));
        let other = answer(&mut fdm, &HeaderMap::new(), "signWorkIn", &work_in(1, "2"));
        assert_eq!(other["errors"][0]["extensions"]["code"], "INVALID_REQUEST");
        assert!(other.get("data").is_none());
    }

    #[test]
    fn counters_are_per_label_with_one_total() {
        let mut c = Counters::default();
        assert_eq!(c.next("S"), (1, 1));
        assert_eq!(c.next("N"), (1, 2));
        assert_eq!(c.next("S"), (2, 3), "the S counter skips the N event");
    }

    #[test]
    fn the_request_identification_is_echoed() {
        let r = build_sign_result(
            "signSale",
            false,
            &json!({
                "posId": "CPOS0031234567",
                "posFiscalTicketNo": 42,
                "posDateTime": "2026-07-03T12:34:56+02:00",
                "terminalId": "T1",
                "deviceId": "00:1b:44:11:3a:b7",
            }),
            &mut Counters::default(),
        );
        assert_eq!(r.pos_id, "CPOS0031234567");
        assert_eq!(r.pos_fiscal_ticket_no, 42);
        assert_eq!(r.pos_date_time, "2026-07-03T12:34:56+02:00");
        assert_eq!(r.terminal_id.as_deref(), Some("T1"));
        assert_eq!(r.device_id, "00:1b:44:11:3a:b7");
        assert_eq!(r.event_operation, "SALE");
    }

    #[test]
    fn vat_is_computed_per_label_from_vat_inclusive_prices() {
        let txn = json!({ "transactionLines": [
            { "mainProduct": { "vats": [{ "label": "A", "price": 24.0 }] } },
            { "mainProduct": { "vats": [{ "label": "B", "price": 28.0 }] } },
            { "mainProduct": { "vats": [{ "label": "B", "price": 5.6 }] } },
            { "mainProduct": { "vats": [{ "label": "X", "price": 3.0 }] } },
        ]});
        let mut calc = compute_vat_calc(&txn);
        calc.sort_by(|a, b| a.label.cmp(&b.label));
        let rows: Vec<(&str, f64, f64, f64, bool)> = calc
            .iter()
            .map(|v| {
                (
                    v.label.as_str(),
                    v.total_amount,
                    v.taxable_amount,
                    v.vat_amount,
                    v.out_of_scope,
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("A", 24.0, 19.83, 4.17, false),
                ("B", 33.6, 30.0, 3.6, false),
                ("X", 3.0, 3.0, 0.0, true),
            ]
        );
    }

    #[test]
    fn status_aliases_each_get_their_language() {
        let q = "query { status(language: EN) { initialized } messagesFR: status(language: FR) { initialized } messagesNL: Status(language: NL) { initialized } }";
        assert_eq!(
            status_occurrences(q),
            vec![
                ("status".to_string(), "EN".to_string()),
                ("messagesFR".to_string(), "FR".to_string()),
                ("messagesNL".to_string(), "NL".to_string()),
            ]
        );
        // An alias that itself ends in "Status" is not a second field.
        assert_eq!(
            status_occurrences("query { fdmStatus: Status(language: DE) { initialized } }"),
            vec![("fdmStatus".to_string(), "DE".to_string())]
        );
        assert_eq!(
            status_occurrences("query { Status { initialized } }"),
            vec![],
            "no parenthesis, no match: the handler falls back to FR"
        );
    }

    #[test]
    fn messages_are_localised_with_a_french_fallback() {
        let msgs = codes_to_messages(vec!["BUFFER_FULL".into(), "RTC_SYNC_FAILED".into()], "NL");
        assert_eq!(msgs[0]["message"], "De buffer van de FDM is vol");
        assert_eq!(msgs[0]["extensions"]["showPos"], "MANDATORY");
        assert_eq!(msgs[0]["extensions"]["category"], "SPF_FOD");
        assert_eq!(
            msgs[1]["message"],
            "Le FDM n'a pas pu synchroniser l'horloge en temps réel"
        );
        assert_eq!(msgs[1]["extensions"]["showPos"], "OPTIONAL");
        assert_eq!(code_show_pos("DUPLICATE_REQUEST"), "NEVER");
        assert_eq!(code_category("SOMETHING_ELSE"), "OTHER");
        assert_eq!(code_to_human("SOMETHING_ELSE", "DE"), "Erreur FDM");
    }

    fn fdm(control: Control) -> FdmState {
        FdmState {
            control,
            ..Default::default()
        }
    }

    fn sale(pos_id: &str) -> GqlRequest {
        GqlRequest {
            query: "mutation($data: SaleInput!, $isTraining: Boolean!) { signSale(data: $data, isTraining: $isTraining) { posId } }".into(),
            variables: json!({ "data": { "posId": pos_id }, "isTraining": false }),
        }
    }

    fn bearer(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        h
    }

    fn error_code(response: &serde_json::Value) -> &str {
        response["errors"][0]["extensions"]["code"]
            .as_str()
            .unwrap_or("")
    }

    /// A signSale with `data`, answered by a default mock.
    fn sign_sale(data: serde_json::Value) -> serde_json::Value {
        let req = GqlRequest {
            query: sale("x").query,
            variables: json!({ "data": data, "isTraining": false }),
        };
        answer(
            &mut fdm(Control::default()),
            &HeaderMap::new(),
            "signSale",
            &req,
        )
    }

    fn line(line_type: &str, quantity: f64, sub: Option<serde_json::Value>) -> serde_json::Value {
        json!({
            "lineType": line_type,
            "mainProduct": { "quantity": quantity, "unitPrice": 12.5, "vats": [{ "label": "A", "price": 12.5 }] },
            "subProducts": sub,
            "lineTotal": 12.5,
        })
    }

    fn with_lines(lines: serde_json::Value) -> serde_json::Value {
        json!({ "posId": "CPOS0031234567", "transaction": { "transactionLines": lines, "transactionTotal": 12.5 } })
    }

    fn refusal(response: &serde_json::Value) -> String {
        assert_eq!(error_code(response), "INVALID_REQUEST", "{response}");
        response["errors"][0]["message"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn a_well_formed_sale_is_signed() {
        let signed = sign_sale(with_lines(json!([line("SINGLE_PRODUCT", 2.0, None)])));
        assert!(signed["data"]["signSale"].is_object(), "{signed}");
    }

    #[test]
    fn amounts_are_in_cents_and_quantities_in_ten_thousandths() {
        let mut data = with_lines(json!([line("SINGLE_PRODUCT", 1.0, None)]));
        data["transaction"]["transactionTotal"] = json!(12.505);
        assert!(refusal(&sign_sale(data)).contains("transactionTotal must be a multiple of 0.01"));

        let signed = sign_sale(with_lines(json!([line("SINGLE_PRODUCT", 0.1234, None)])));
        assert!(signed["data"].is_object(), "4 decimals are fine: {signed}");
        let refused = sign_sale(with_lines(json!([line("SINGLE_PRODUCT", 0.12345, None)])));
        assert!(refusal(&refused).contains("quantity must be a multiple of 0.0001"));
    }

    #[test]
    fn sub_products_follow_the_line_type() {
        let single = sign_sale(with_lines(json!([line(
            "SINGLE_PRODUCT",
            1.0,
            Some(json!([]))
        )])));
        assert!(refusal(&single).contains("must be null for lineType SINGLE_PRODUCT"));
        let composite = sign_sale(with_lines(json!([line("COMPOSITE_PRODUCT", 1.0, None)])));
        assert!(refusal(&composite).contains("is required for lineType COMPOSITE_PRODUCT"));
    }

    #[test]
    fn a_negative_quantity_needs_a_reason() {
        let refused = sign_sale(with_lines(json!([line("SINGLE_PRODUCT", -1.0, None)])));
        assert!(refusal(&refused).contains("negQuantityReason is required"));
        let mut refund = line("SINGLE_PRODUCT", -1.0, None);
        refund["mainProduct"]["negQuantityReason"] = json!("REFUND");
        assert!(sign_sale(with_lines(json!([refund])))["data"].is_object());
    }

    #[test]
    fn a_cited_event_has_a_spec_fdm_ref() {
        let cite = |fdm_id: &str, at: &str| {
            let mut data = with_lines(json!([line("SINGLE_PRODUCT", 1.0, None)]));
            data["fdmRefs"] = json!([{ "fdmId": fdm_id, "fdmDateTime": at, "eventLabel": "N", "eventCounter": 1, "totalCounter": 1 }]);
            sign_sale(data)
        };
        assert!(cite("FDM02000425", "2026-09-27T17:17:29Z")["data"].is_object());
        assert!(
            refusal(&cite("CBX0MOCK0001", "2026-09-27T17:17:29Z")).contains("fdmId must be 11")
        );
        assert!(refusal(&cite("FDM02000425", "2026-09-27T19:17:29+02:00")).contains("fdmDateTime"));
    }

    #[test]
    fn an_order_has_a_cost_center() {
        let order = |cost_center: serde_json::Value| {
            let req = GqlRequest {
                query: "mutation($data: OrderInput!, $isTraining: Boolean!) { signOrder(data: $data, isTraining: $isTraining) { posId } }".into(),
                variables: json!({ "data": { "posId": "CPOS0031234567", "costCenter": cost_center }, "isTraining": false }),
            };
            answer(
                &mut fdm(Control::default()),
                &HeaderMap::new(),
                "signOrder",
                &req,
            )
        };
        assert!(refusal(&order(json!(null))).contains("costCenter is required"));
        assert!(order(json!({ "id": "4", "type": "TABLE" }))["data"].is_object());
    }

    #[test]
    fn a_required_token_must_be_sent() {
        let mut f = fdm(Control {
            token: Some("secret".into()),
            ..Default::default()
        });
        let req = sale("CPOS0031234567");
        // As module2: a bare message, no code.
        let refused = answer(&mut f, &HeaderMap::new(), "signSale", &req);
        assert_eq!(
            refused,
            json!({ "errors": [{ "message": "Not a valid Token" }] })
        );
        let wrong = answer(&mut f, &bearer("other"), "signSale", &req);
        assert_eq!(wrong, refused);
        let signed = answer(&mut f, &bearer("secret"), "signSale", &req);
        assert_eq!(signed["data"]["signSale"]["posId"], "CPOS0031234567");
    }

    #[test]
    fn placeholder_signatures_look_like_the_simulator_ones() {
        let mut f = fdm(Control {
            placeholder_signatures: true,
            ..Default::default()
        });
        let signed = answer(
            &mut f,
            &HeaderMap::new(),
            "signSale",
            &sale("CPOS0031234567"),
        );
        let sale = &signed["data"]["signSale"];
        let sig = sale["digitalSignature"].as_str().unwrap();
        let rest = sig.strip_prefix("dummy_").expect("dummy_ prefix");
        assert_eq!(rest.len(), 38, "{sig}");
        let short = sale["shortSignature"].as_str().unwrap();
        assert_eq!(short.len(), 40);
        assert!(short.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn only_allowed_pos_may_sign() {
        let mut f = fdm(Control {
            allowed_pos: Some(vec!["CPOS0031234567".into()]),
            ..Default::default()
        });
        let refused = answer(
            &mut f,
            &HeaderMap::new(),
            "signSale",
            &sale("CFOD0061234567"),
        );
        assert_eq!(error_code(&refused), "UNKNOWN_POS");
        let signed = answer(
            &mut f,
            &HeaderMap::new(),
            "signSale",
            &sale("CPOS0031234567"),
        );
        assert!(signed["data"]["signSale"].is_object());
    }

    #[test]
    fn a_critical_error_refuses_signing_in_the_spec_shape() {
        let mut f = fdm(Control {
            errors: vec!["BUFFER_FULL".into()],
            ..Default::default()
        });
        let r = answer(
            &mut f,
            &HeaderMap::new(),
            "signSale",
            &sale("CPOS0031234567"),
        );
        assert_eq!(r["data"], serde_json::Value::Null);
        assert_eq!(r["errors"][0]["message"], "Le buffer du FDM est plein");
        assert_eq!(
            r["errors"][0]["extensions"],
            json!({ "category": "SPF_FOD", "code": "BUFFER_FULL", "showPos": "MANDATORY" })
        );
    }

    #[test]
    fn warnings_ride_along_with_every_signature() {
        let mut f = fdm(Control {
            warnings: vec!["BUFFER_NEAR_FULL".into()],
            ..Default::default()
        });
        let r = answer(
            &mut f,
            &HeaderMap::new(),
            "signSale",
            &sale("CPOS0031234567"),
        );
        let result = &r["data"]["signSale"];
        assert_eq!(
            result["warnings"][0]["extensions"]["code"],
            "BUFFER_NEAR_FULL"
        );
        assert_eq!(result["bufferCapacityUsed"], 82.53);
    }

    #[test]
    fn each_pos_has_its_own_counters() {
        let mut f = fdm(Control::default());
        let counter = |f: &mut FdmState, pos: &str, ticket_no: i64| {
            let mut request = sale(pos);
            request.variables["data"]["posFiscalTicketNo"] = json!(ticket_no);
            answer(f, &HeaderMap::new(), "signSale", &request)["data"]["signSale"]["fdmRef"]
                ["totalCounter"]
                .as_i64()
                .unwrap()
        };
        assert_eq!(counter(&mut f, "CPOS0031234567", 1), 1);
        assert_eq!(counter(&mut f, "CPOS0031234567", 2), 2);
        assert_eq!(counter(&mut f, "CFOD0061234567", 1), 1);
    }

    #[tokio::test]
    async fn the_control_api_switches_errors_without_a_restart() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::ServiceExt;

        let app = app_with(Control::default());
        let post = |uri: &str, body: serde_json::Value| {
            Request::post(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap()
        };
        let sign = || {
            post(
                "/graphql",
                serde_json::to_value(sale("CPOS0031234567")).unwrap(),
            )
        };
        let body = |r: axum::response::Response| async move {
            let bytes = axum::body::to_bytes(r.into_body(), usize::MAX)
                .await
                .unwrap();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        };

        let ok = body(app.clone().oneshot(sign()).await.unwrap()).await;
        assert!(ok["data"]["signSale"].is_object());
        app.clone()
            .oneshot(post("/mock/control", json!({ "errors": ["FDM_LOCKED"] })))
            .await
            .unwrap();
        let locked = body(app.clone().oneshot(sign()).await.unwrap()).await;
        assert_eq!(error_code(&locked), "FDM_LOCKED");

        let log = body(
            app.clone()
                .oneshot(Request::get("/mock/requests").body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        let ops: Vec<_> = log
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["operation"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ops, ["signSale", "signSale"]);
    }
}
