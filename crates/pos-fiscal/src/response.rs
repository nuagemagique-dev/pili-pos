use serde::{Deserialize, Serialize};

use crate::types::*;

// ── FDM Response Types ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignResult {
    pub pos_id: String,
    pub pos_fiscal_ticket_no: i64,
    pub pos_date_time: String,
    pub terminal_id: Option<String>,
    pub device_id: String,
    pub event_operation: EventOperation,
    pub fdm_ref: FdmReference,
    pub fdm_sw_version: String,
    pub digital_signature: String,
    pub short_signature: Option<String>,
    pub verification_url: Option<String>,
    pub vat_calc: Option<Vec<VatCalcItem>>,
    pub buffer_capacity_used: f64,
    pub warnings: Option<Vec<MessageItem>>,
    pub informations: Option<Vec<MessageItem>>,
    pub footer: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FdmReference {
    pub fdm_id: String,
    pub fdm_date_time: String,
    pub event_label: EventLabel,
    pub event_counter: i64,
    pub total_counter: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VatCalcItem {
    pub label: VatLabel,
    pub rate: f64,
    pub taxable_amount: f64,
    pub vat_amount: f64,
    pub total_amount: f64,
    pub out_of_scope: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageItem {
    pub message: String,
    pub locations: Option<Vec<LocationItem>>,
    pub extensions: ExtensionItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationItem {
    pub line: i32,
    pub column: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionItem {
    pub category: MessageCategory,
    pub code: FdmWarningCode,
    pub data: Option<Vec<DataItem>>,
    pub show_pos: Display,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataItem {
    pub name: String,
    pub value: String,
}

// ── FDM Status Query Response ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FdmStatus {
    pub device: FdmDevice,
    pub initialized: bool,
    pub informations: Option<Vec<MessageItem>>,
    pub warnings: Option<Vec<MessageItem>>,
    pub errors: Option<Vec<MessageItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FdmDevice {
    pub fdm_id: String,
    pub fdm_sw_version: String,
    pub fdm_date_time: String,
    pub buffer_capacity_used: f64,
}

// ── GraphQL Response Wrapper ───────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQlResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraphQlError {
    pub message: String,
    pub locations: Option<Vec<LocationItem>>,
    pub extensions: Option<serde_json::Value>,
}

impl std::fmt::Display for GraphQlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
