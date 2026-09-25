//! Mock FDM (Checkbox-compatible GraphQL) and mock Scaleway TEM, for
//! development, the admission image and screenshots. Counters are
//! process-wide, like one physical FDM.

use std::collections::HashMap;
use std::sync::Mutex;

use axum::{Json, Router, routing::post};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha1::Digest;

mod tem;

pub use tem::ROUTE as TEM_ROUTE;

/// The FDM's counters (POS spec v2.2 §2.4, arrêté ministériel n° 64):
/// one per event label, plus the total of every event.
#[derive(Default)]
struct Counters {
    per_label: HashMap<&'static str, i64>,
    total: i64,
}

static COUNTERS: Mutex<Option<Counters>> = Mutex::new(None);

/// Advance the counter of `label` and the total counter, together.
fn next_counters(label: &'static str) -> (i64, i64) {
    let mut guard = COUNTERS.lock().unwrap_or_else(|e| e.into_inner());
    let c = guard.get_or_insert_with(Counters::default);
    let event = c.per_label.entry(label).or_insert(0);
    *event += 1;
    c.total += 1;
    (*event, c.total)
}

/// The mock FDM (GraphQL on `/graphql`) and the mock email API
/// ([`TEM_ROUTE`]), as served by the `pos-fdm-mock` binary. Tests
/// mount it on a local port to drive the real POS client against it.
pub fn app() -> Router {
    Router::new()
        .route("/graphql", post(handle_graphql))
        .merge(tem::router())
}

#[derive(Deserialize)]
struct GqlRequest {
    query: String,
    #[serde(default)]
    variables: serde_json::Value,
}

async fn handle_graphql(Json(req): Json<GqlRequest>) -> Json<serde_json::Value> {
    let mutation_name = extract_mutation_name(&req.query);
    let is_training = req
        .variables
        .get("isTraining")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Cert demo: inject FDM warnings/errors via env vars MOCK_FDM_WARNING /
    // MOCK_FDM_ERROR (comma-separated codes accepted). Allows capturing the
    // POS UI response for §V of the certification manual without rebuilding.
    if let Some(payload) = inject_status_error(&mutation_name) {
        return Json(payload);
    }

    if mutation_name == "Status" {
        return Json(status_response(&req.query));
    }

    // M184 — the end-of-day Z carries BOTH Z mutations in one GraphQL
    // document (`$turnover` + `$user` variables). Each gets its own
    // counters, like a real FDM signing two events back to back.
    if req.query.contains("signReportTurnoverZ") && req.query.contains("signReportUserZ") {
        let turnover = req.variables.get("turnover").cloned().unwrap_or_default();
        let user = req.variables.get("user").cloned().unwrap_or_default();
        let r1 = build_sign_result("signReportTurnoverZ", is_training, &turnover);
        let r2 = build_sign_result("signReportUserZ", is_training, &user);
        println!(
            "[R+R] signReportTurnoverZ+signReportUserZ #{}+#{} (M184)",
            r1.fdm_ref.event_counter, r2.fdm_ref.event_counter
        );
        return Json(serde_json::json!({ "data": {
            "signReportTurnoverZ": r1,
            "signReportUserZ": r2,
        }}));
    }

    let data = req.variables.get("data").cloned().unwrap_or_default();
    let result = build_sign_result(&mutation_name, is_training, &data);

    println!(
        "[{}] {mutation_name} #{} (total: {})",
        result.fdm_ref.event_label, result.fdm_ref.event_counter, result.fdm_ref.total_counter
    );

    let mut data_map = serde_json::Map::new();
    data_map.insert(
        mutation_name.clone(),
        serde_json::to_value(&result).unwrap(),
    );

    Json(serde_json::json!({ "data": data_map }))
}

/// Build one signed result for a mutation, advancing the counters.
fn build_sign_result(
    mutation_name: &str,
    is_training: bool,
    data: &serde_json::Value,
) -> SignResult {
    // Every event signed in training mode is a T event, whatever the
    // mutation (UC M170 work-in, M171 sale and M172 copy all answer T).
    let event_label = match mutation_name {
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
    };

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

    let (ec, tc) = next_counters(event_label);

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

    // Build VAT calc for sales
    let vat_calc = if event_label == "N" || event_label == "T" {
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
            fdm_id: "CBX0MOCK0001".to_string(),
            fdm_date_time: chrono::Utc::now().to_rfc3339(),
            event_label: event_label.to_string(),
            event_counter: ec,
            total_counter: tc,
        },
        fdm_sw_version: "MOCK-FDM-1.0".to_string(),
        digital_signature: digital_sig,
        short_signature: short_sig,
        verification_url,
        vat_calc,
        buffer_capacity_used: 1.2,
        warnings: None,
        informations: None,
        footer: vec!["www.systemedecaisseenregistreuse.be".to_string()],
    }
}

fn extract_mutation_name(query: &str) -> String {
    // Both spec spellings of the status field (SDL says `Status`, the
    // Q100/Q110 UC fixtures use `status`) route to the status handler.
    if query.contains("Status(") || query.contains("status(") {
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
            // alias — only accept occurrences where the char before
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

fn status_response(query: &str) -> serde_json::Value {
    let occurrences = status_occurrences(query);
    let occurrences = if occurrences.is_empty() {
        vec![("Status".to_string(), "FR".to_string())]
    } else {
        occurrences
    };

    // Keep the reported buffer usage coherent with injected buffer
    // codes so §V evidence is self-consistent (a BUFFER_NEAR_FULL
    // warning alongside a 1.2% buffer reading would contradict the
    // >70% threshold the message claims).
    let injected: Vec<String> = parse_codes("MOCK_FDM_WARNING")
        .into_iter()
        .chain(parse_codes("MOCK_FDM_ERROR"))
        .flatten()
        .collect();
    let default_buffer_pct = if injected.iter().any(|c| c == "BUFFER_FULL") {
        100.0
    } else if injected.iter().any(|c| c == "BUFFER_NEAR_FULL") {
        82.53
    } else {
        1.2
    };
    let buffer_pct: f64 = std::env::var("MOCK_FDM_BUFFER_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default_buffer_pct);
    let has_errors = parse_codes("MOCK_FDM_ERROR").is_some();
    let initialized = !has_errors
        || std::env::var("MOCK_FDM_INITIALIZED")
            .ok()
            .as_deref()
            .map(|v| v != "false")
            .unwrap_or(true);

    let mut data = serde_json::Map::new();
    for (key, lang) in occurrences {
        let warnings = parse_codes("MOCK_FDM_WARNING").map(|c| codes_to_messages(c, &lang));
        let errors = parse_codes("MOCK_FDM_ERROR").map(|c| codes_to_messages(c, &lang));
        data.insert(
            key,
            serde_json::json!({
                "device": {
                    "fdmId": "CBX0MOCK0001",
                    "fdmSwVersion": "MOCK-FDM-1.0",
                    "fdmDateTime": chrono::Utc::now().to_rfc3339(),
                    "bufferCapacityUsed": buffer_pct
                },
                "initialized": initialized,
                "informations": null,
                "warnings": warnings,
                "errors": errors
            }),
        );
    }
    serde_json::json!({ "data": data })
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
/// (`SPF_FOD` | `FDM` | `OTHER`), not its severity — severity is
/// implied by which array (warnings/errors/informations) carries the
/// message. All tb_messages codes are SPF_FOD-defined.
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
        | "DUPLICATE_REQUEST"
        | "TOO_MANY_MEMORY_ERRORS"
        | "BUFFER_FULL"
        | "FDM_LOCKED"
        | "UNAUTHORIZED"
        | "INVALID_REQUEST"
        | "INTERNAL_ERROR"
        | "UNDEFINED_ERROR"
        | "UNDEFINED_OTHER"
        | "FDM_NOT_OPERATIONAL"
        | "UNKNOWN_POS" => "SPF_FOD",
        _ => "OTHER",
    }
}

/// `showPos` — display obligation per the §V table of the
/// certification manual (affichage M/O/N).
fn code_show_pos(code: &str) -> &'static str {
    match code {
        // Mandatory display
        "CLIENT_CERT_EXPIRED"
        | "BUFFER_NEAR_FULL"
        | "BUFFER_FULL"
        | "FDM_LOCKED"
        | "FDM_NOT_OPERATIONAL"
        | "UNKNOWN_POS" => "MANDATORY",
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
        _ => "Erreur FDM",
    }
}

/// When `MOCK_FDM_ERROR` is set with critical codes, intercept sign mutations
/// and return a GraphQL error envelope (mirroring real FDM behaviour). Used
/// for cert evidence captures (§V).
fn inject_status_error(mutation_name: &str) -> Option<serde_json::Value> {
    if mutation_name == "Status" {
        return None;
    }
    let errors = parse_codes("MOCK_FDM_ERROR")?;
    let critical = errors.iter().any(|c| {
        matches!(
            c.as_str(),
            "BUFFER_FULL"
                | "FDM_LOCKED"
                | "FDM_NOT_OPERATIONAL"
                | "UNKNOWN_POS"
                | "UNAUTHORIZED"
                | "INVALID_REQUEST"
                | "INTERNAL_ERROR"
                | "UNDEFINED_ERROR"
        )
    });
    if !critical {
        return None;
    }
    let messages: Vec<serde_json::Value> = errors
        .iter()
        .map(|code| {
            serde_json::json!({
                "message": code_to_human(code, "FR"),
                "extensions": { "code": code },
            })
        })
        .collect();
    Some(serde_json::json!({
        "data": null,
        "errors": messages,
    }))
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
    warnings: Option<serde_json::Value>,
    informations: Option<serde_json::Value>,
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
            extract_mutation_name("query { Status(language: FR) { initialized } }"),
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
            let r = build_sign_result(m, false, &json!({}));
            assert_eq!(r.fdm_ref.event_label, label, "{m}");
            let t = build_sign_result(m, true, &json!({}));
            assert_eq!(t.fdm_ref.event_label, "T", "{m} in training");
            assert!(t.short_signature.is_none() && t.verification_url.is_none());
        }
    }

    #[test]
    fn only_n_events_carry_a_short_signature_and_a_verification_url() {
        let n = build_sign_result("signSale", false, &json!({}));
        let sig = n.short_signature.expect("N has a short signature");
        assert_eq!(sig.len(), 40);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(n.verification_url.unwrap().starts_with("https://"));
        for m in ["signOrder", "signCopy", "signWorkIn", "signReportTurnoverX"] {
            let r = build_sign_result(m, false, &json!({}));
            assert!(r.short_signature.is_none(), "{m}");
            assert!(r.verification_url.is_none(), "{m}");
        }
    }

    #[test]
    fn counters_are_per_label_with_one_total() {
        // Process-wide, like one FDM: compare increments, not values.
        let (s1, t1) = next_counters("S");
        let (n1, t2) = next_counters("N");
        let (s2, t3) = next_counters("S");
        assert_eq!(s2, s1 + 1, "the S counter skips the N event");
        assert!(t2 > t1 && t3 > t2);
        assert!(n1 >= 1);
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
}
