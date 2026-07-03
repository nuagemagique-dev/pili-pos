use std::sync::atomic::{AtomicI64, Ordering};

use axum::{Json, Router, routing::post};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha1::Digest;

static EVENT_COUNTER: AtomicI64 = AtomicI64::new(0);
static TOTAL_COUNTER: AtomicI64 = AtomicI64::new(0);

#[tokio::main]
async fn main() {
    let app = Router::new().route("/graphql", post(handle_graphql));

    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Mock FDM listening on http://localhost:{port}/graphql");
    println!("Press Ctrl+C to stop");
    axum::serve(listener, app).await.unwrap();
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

    let event_label = match mutation_name.as_str() {
        "signSale" => {
            if is_training {
                "T"
            } else {
                "N"
            }
        }
        "signOrder" | "signPreBill" | "signCostCenterChange" => "P",
        "signWorkIn" | "signWorkOut" => "S",
        "signCopy" => "C",
        "signInvoice" => "I",
        "signMoneyInOut" | "signDrawerOpen" | "signPaymentCorrection" => "F",
        "signReportTurnoverX" | "signReportTurnoverZ" | "signReportUserX" | "signReportUserZ" => {
            "R"
        }
        "Status" => return Json(status_response(&req.query)),
        _ => "N",
    };

    let event_op = match mutation_name.as_str() {
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

    let ec = EVENT_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
    let tc = TOTAL_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;

    // Extract fields from variables.data
    let data = req.variables.get("data").cloned().unwrap_or_default();
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

    let result = SignResult {
        pos_id: pos_id.to_string(),
        pos_fiscal_ticket_no: ticket_no,
        pos_date_time: pos_dt.to_string(),
        terminal_id: terminal_id.map(|s| s.to_string()),
        device_id: device_id.to_string(),
        event_operation: event_op.to_string(),
        fdm_ref: FdmRef {
            fdm_id: "BMC0MOCK0001".to_string(),
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
    };

    println!("[{event_label}] {mutation_name} #{ec} (total: {tc})");

    let mut data_map = serde_json::Map::new();
    data_map.insert(
        mutation_name.clone(),
        serde_json::to_value(&result).unwrap(),
    );

    Json(serde_json::json!({ "data": data_map }))
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
                    "fdmId": "BMC0MOCK0001",
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
