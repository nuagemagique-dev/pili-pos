//! The B2B invoice records (V11-D1): the FFI mirror of `pos_engine::commands::invoice`
//! and of the customer and delivery rows of `pos_db::queries::invoicing`.

/// A saved B2B customer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Customer {
    pub id: i64,
    pub vat_number: String,
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country_code: String,
    /// `unknown`, `registered` or `not_registered`.
    pub peppol_status: String,
    pub peppol_checked_at: Option<String>,
}

/// A customer form.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct B2bCustomerInput {
    /// Set to edit a known customer.
    pub id: Option<i64>,
    pub vat_number: String,
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    /// ISO 3166-1 alpha-2; the country of the VAT number when empty.
    pub country_code: String,
}

/// A signed sale that can go on an invoice.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvoiceableSale {
    pub fiscal_event_id: i64,
    pub ticket_number: i64,
    pub pos_date_time: String,
    pub total_cents: i64,
    pub fdm_id: String,
    pub event_counter: i64,
    pub total_counter: i64,
    pub is_refund: bool,
    pub invoiced_by: Option<String>,
}

/// One row of an invoice preview.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewLine {
    pub description: String,
    /// Thousandths (2 pieces = 2000).
    pub quantity_milli: i64,
    pub total_cents: i64,
    pub vat_label: String,
}

/// One VAT row of an invoice preview.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewVat {
    pub label: String,
    /// Hundredths of a percent (21 % = 2100).
    pub rate_hundredths: i64,
    pub base_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
}

/// What an invoice would contain.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvoicePreview {
    pub sales: Vec<InvoiceableSale>,
    pub lines: Vec<PreviewLine>,
    pub vat: Vec<PreviewVat>,
    pub total_cents: i64,
    /// Coded errors that stop the invoice.
    pub problems: Vec<String>,
}

/// The answer to a VAT number lookup.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CustomerLookup {
    pub vat_number: String,
    pub country_code: String,
    pub customer: Option<Customer>,
    /// `registered`, `not_registered` or `unknown`.
    pub peppol_status: String,
    pub peppol_checked: bool,
    pub deliverable: bool,
}

/// What `sign_invoice_fiscal` returns.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvoiceResult {
    pub success: bool,
    pub fiscal_event_id: Option<i64>,
    pub invoice_no: Option<String>,
    pub printed: bool,
    /// `queued` when a Peppol delivery was queued.
    pub delivery_status: Option<String>,
    pub error: Option<String>,
}

/// One invoice of the list.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvoiceSummary {
    pub id: i64,
    pub invoice_no: String,
    pub fiscal_event_id: i64,
    pub issue_date: String,
    pub customer_name: String,
    pub customer_vat_number: String,
    pub total_cents: i64,
    pub sale_event_ids: Vec<i64>,
    pub delivery_status: Option<String>,
    pub delivery_id: Option<i64>,
}

/// One Peppol delivery.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Delivery {
    pub id: i64,
    pub fiscal_event_id: Option<i64>,
    pub invoice_no: String,
    pub customer_endpoint_id: String,
    pub ap_vendor: String,
    pub ap_message_id: Option<String>,
    /// `queued`, `accepted` or `failed`.
    pub status: String,
    pub error_msg: Option<String>,
    pub attempts: i64,
    pub next_attempt_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub parked_at: Option<String>,
    pub accepted_at: Option<String>,
    pub created_at: String,
    pub lookup_status: String,
}

/// The Peppol panel.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PeppolDeliveries {
    pub configured: bool,
    pub provider: Option<String>,
    pub sender_id: Option<String>,
    pub queued: i64,
    pub accepted: i64,
    pub failed: i64,
    pub deliveries: Vec<Delivery>,
}
