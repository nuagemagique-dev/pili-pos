use std::sync::atomic::{AtomicI64, Ordering};

use axum::{Json, Router, routing::post};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha1::Digest;

static EVENT_COUNTER: AtomicI64 = AtomicI64::new(0);
static TOTAL_COUNTER: AtomicI64 = AtomicI64::new(0);

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/graphql", post(handle_graphql));

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
    let is_training = req.variables
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
        "signSale" => if is_training { "T" } else { "N" },
        "signOrder" | "signPreBill" | "signCostCenterChange" => "P",
        "signWorkIn" | "signWorkOut" => "S",
        "signCopy" => "C",
        "signInvoice" => "I",
        "signMoneyInOut" | "signDrawerOpen" | "signPaymentCorrection" => "F",
        "signReportTurnoverX" | "signReportTurnoverZ" | "signReportUserX" | "signReportUserZ" => "R",
        "Status" => return Json(status_response()),
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
    let pos_id = data.get("posId").and_then(|v| v.as_str()).unwrap_or("CFOD0061234567");
    let ticket_no = data.get("posFiscalTicketNo").and_then(|v| v.as_i64()).unwrap_or(ec);
    let pos_dt = data.get("posDateTime").and_then(|v| v.as_str()).unwrap_or("");
    let terminal_id = data.get("terminalId").and_then(|v| v.as_str());
    let device_id = data.get("deviceId").and_then(|v| v.as_str()).unwrap_or("DEV001");

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
    data_map.insert(mutation_name.clone(), serde_json::to_value(&result).unwrap());

    Json(serde_json::json!({ "data": data_map }))
}

fn extract_mutation_name(query: &str) -> String {
    // Find pattern: mutationName( or mutationName(
    let mutations = [
        "signSale", "signOrder", "signPreBill", "signWorkIn", "signWorkOut",
        "signCopy", "signInvoice", "signMoneyInOut", "signDrawerOpen",
        "signPaymentCorrection", "signCostCenterChange",
        "signReportTurnoverX", "signReportTurnoverZ",
        "signReportUserX", "signReportUserZ", "Status",
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
            if let Some(vats) = line.get("mainProduct")
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

    totals.iter().map(|(label, &total)| {
        let rate = rates.get(label.as_str()).copied().unwrap_or(0.0);
        let taxable = if rate > 0.0 { (total / (1.0 + rate / 100.0) * 100.0).round() / 100.0 } else { total };
        let vat_amount = ((total - taxable) * 100.0).round() / 100.0;
        VatCalcItem {
            label: label.clone(),
            rate,
            taxable_amount: taxable,
            vat_amount,
            total_amount: total,
            out_of_scope: label == "X",
        }
    }).collect()
}

fn status_response() -> serde_json::Value {
    let warnings = parse_codes("MOCK_FDM_WARNING").map(codes_to_messages);
    let errors = parse_codes("MOCK_FDM_ERROR").map(codes_to_messages);
    let buffer_pct: f64 = std::env::var("MOCK_FDM_BUFFER_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.2);
    let initialized = errors.is_none()
        || std::env::var("MOCK_FDM_INITIALIZED")
            .ok()
            .as_deref()
            .map(|v| v != "false")
            .unwrap_or(true);

    serde_json::json!({
        "data": {
            "Status": {
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
            }
        }
    })
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

fn codes_to_messages(codes: Vec<String>) -> serde_json::Value {
    let items: Vec<serde_json::Value> = codes
        .into_iter()
        .map(|code| {
            let (category, show_pos) = code_severity(&code);
            serde_json::json!({
                "message": code_to_human(&code),
                "extensions": {
                    "category": category,
                    "code": code,
                    "showPos": show_pos,
                }
            })
        })
        .collect();
    serde_json::Value::Array(items)
}

fn code_severity(code: &str) -> (&'static str, &'static str) {
    // (category, showPos) — values match FDM spec v1.4 §V table.
    match code {
        "BUFFER_NEAR_FULL"
        | "BUFFER_FULL"
        | "FDM_LOCKED"
        | "FDM_NOT_OPERATIONAL"
        | "UNKNOWN_POS"
        | "CLIENT_CERT_EXPIRED" => ("ERROR", "MANDATORY"),
        "RTC_NOT_INITIALIZED"
        | "CORRUPT_RECORD_ENCOUNTERED"
        | "UPDATE_PARAMS_FAILED"
        | "UPDATE_CLIENT_CERT_FAILED"
        | "UPDATE_VAT_RATES_FAILED"
        | "UPDATE_POS_ALLOWLIST_FAILED"
        | "DUPLICATE_REQUEST"
        | "TOO_MANY_MEMORY_ERRORS" => ("ERROR", "NEVER"),
        _ => ("WARNING", "OPTIONAL"),
    }
}

fn code_to_human(code: &str) -> &'static str {
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
        "TRANSACTION_UPLOAD_FAILED" => "Le FDM a tenté plus de 10 fois de poster une transaction sans succès",
        "INITIALIZATION_FAILED" => "L'initialisation du FDM a échoué",
        "BUFFER_FULL" => "Le buffer du FDM est plein",
        "FDM_LOCKED" => "Le FDM est verrouillé",
        "UNAUTHORIZED" => "Le FDM a reçu des requêtes non-autorisées",
        "INVALID_REQUEST" => "Le FDM n'a pas pu traiter la requête",
        "INTERNAL_ERROR" => "Le FDM a rencontré un problème technique",
        "UNDEFINED_ERROR" => "Le FDM a rencontré une erreur indéfinie",
        "UNDEFINED_OTHER" => "Toute autre erreur rencontrée",
        "FDM_NOT_OPERATIONAL" => "Le FDM ne fonctionne pas",
        "UNKNOWN_POS" => "Le FDM a reçu un message d'un POS non repris dans la liste d'autorisation",
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
                "message": code_to_human(code),
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
