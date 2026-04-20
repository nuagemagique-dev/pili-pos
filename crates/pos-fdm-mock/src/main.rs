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
    serde_json::json!({
        "data": {
            "Status": {
                "device": {
                    "fdmId": "BMC0MOCK0001",
                    "fdmSwVersion": "MOCK-FDM-1.0",
                    "fdmDateTime": chrono::Utc::now().to_rfc3339(),
                    "bufferCapacityUsed": 1.2
                },
                "initialized": true,
                "informations": null,
                "warnings": null,
                "errors": null
            }
        }
    })
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
