use serde::Serialize;

use crate::types::{
    CostCenterType, EventLabel, InOut, InputMethod, Language, MoneyInOutLineType,
    NegQuantityReason, PaymentLineType, PaymentType, PriceChangeScope, PriceChangeType,
    QuantityType, TicketMedium, TransactionLineType, VatLabel,
};

// ── Shared Input Objects ───────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FdmReferenceInput {
    pub fdm_id: String,
    pub fdm_date_time: String,
    pub event_label: EventLabel,
    pub event_counter: i64,
    pub total_counter: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerInput {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostCenterInput {
    pub id: String,
    #[serde(rename = "type")]
    pub cost_center_type: CostCenterType,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<Box<CostCenterInput>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceChangeInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping_id: Option<i32>,
    pub id: String,
    pub name: String,
    pub scope: PriceChangeScope,
    #[serde(rename = "type")]
    pub price_change_type: PriceChangeType,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VatInput {
    pub label: VatLabel,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_changes: Option<Vec<PriceChangeInput>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gtin: Option<String>,
    pub product_id: String,
    pub product_name: String,
    pub department_id: String,
    pub department_name: String,
    pub quantity: f64,
    pub quantity_type: QuantityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub neg_quantity_reason: Option<NegQuantityReason>,
    pub unit_price: f64,
    pub vats: Vec<VatInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionLineInput {
    pub line_type: TransactionLineType,
    pub main_product: ProductInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_products: Option<Vec<ProductInput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<CostCenterInput>,
    pub line_total: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInput {
    pub transaction_lines: Vec<TransactionLineInput>,
    pub transaction_total: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignCurrencyInput {
    pub amount: f64,
    pub iso: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentLineInput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub input_method: InputMethod,
    pub amount: f64,
    pub amount_type: PaymentLineType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_currency: Option<ForeignCurrencyInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawer: Option<DrawerInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoneyInOutLineInput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    pub input_method: InputMethod,
    pub amount: f64,
    pub amount_type: MoneyInOutLineType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_currency: Option<ForeignCurrencyInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawer: Option<DrawerInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferItemInput {
    pub cost_center: CostCenterInput,
    pub transaction: TransactionInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferInput {
    pub from: Vec<TransferItemInput>,
    pub to: Vec<TransferItemInput>,
}

// ── Common fields shared by all event-specific inputs ──────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonFields {
    pub language: Language,
    pub vat_no: String,
    pub est_no: String,
    pub pos_id: String,
    pub pos_fiscal_ticket_no: i64,
    pub pos_date_time: String,
    pub pos_sw_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    pub device_id: String,
    pub booking_period_id: String,
    pub booking_date: String,
    pub ticket_medium: TicketMedium,
    pub employee_id: String,
}

// ── Event-Specific Input Objects ───────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaleInput {
    #[serde(flatten)]
    pub common: CommonFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fdm_ref: Option<FdmReferenceInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<CostCenterInput>,
    pub transaction: TransactionInput,
    pub financials: Vec<PaymentLineInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub cost_center: CostCenterInput,
    pub transaction: TransactionInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreBillInput {
    #[serde(flatten)]
    pub common: CommonFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<CostCenterInput>,
    pub transaction: TransactionInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CostCenterChangeInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub transfer: TransferInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkInOutInput {
    #[serde(flatten)]
    pub common: CommonFields,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub invoice_no: String,
    pub customer_vat_no: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<CostCenterInput>,
    pub fdm_refs: Vec<FdmReferenceInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoneyInOutInput {
    #[serde(flatten)]
    pub common: CommonFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawer: Option<DrawerInput>,
    pub financials: Vec<MoneyInOutLineInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawerOpenInput {
    #[serde(flatten)]
    pub common: CommonFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drawer: Option<DrawerInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentCorrectionInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub financials: Vec<PaymentLineInput>,
    pub fdm_ref: FdmReferenceInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub fdm_ref: FdmReferenceInput,
}

// ── Report-specific inputs ─────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PosDeviceInput {
    pub pos_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_id: Option<String>,
    pub first_pos_date_time: String,
    pub last_pos_date_time: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FdmDeviceInput {
    pub fdm_id: String,
    pub first_fdm_date_time: String,
    pub first_total_counter: i64,
    pub last_fdm_date_time: String,
    pub last_total_counter: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventTotalInput {
    pub event_label: EventLabel,
    pub ticket_count: i64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentTotalInput {
    pub department_id: String,
    pub department_name: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VatTotalInput {
    pub label: VatLabel,
    pub rate: f64,
    pub taxable_amount: f64,
    pub vat_amount: f64,
    pub total_amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentTotalAmountInput {
    #[serde(rename = "type")]
    pub amount_type: PaymentLineType,
    pub normal_amount: f64,
    pub negative_corrections: f64,
    pub positive_corrections: f64,
    pub corrections_count: i64,
    pub total_amount: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal_foreign: Option<Vec<ForeignCurrencyInput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrections_foreign: Option<Vec<ForeignCurrencyInput>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentTotalInput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub payment_type: PaymentType,
    pub amount: Vec<PaymentTotalAmountInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NegQuantityTotalInput {
    pub neg_quantity_reason: NegQuantityReason,
    pub neg_quantity_count: i64,
    pub ticket_count: i64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceChangeVatTotalInput {
    pub label: VatLabel,
    pub negative: f64,
    pub positive: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceChangeTotalInput {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub price_change_type: PriceChangeType,
    pub amount: Vec<PriceChangeVatTotalInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceTotalInput {
    pub invoice_no: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnoverInput {
    pub transactions: Vec<EventTotalInput>,
    pub departments: Vec<DepartmentTotalInput>,
    pub vats: Vec<VatTotalInput>,
    pub payments: Vec<PaymentTotalInput>,
    #[serde(default)]
    pub drawers_open_count: i64,
    pub neg_quantities: Vec<NegQuantityTotalInput>,
    pub price_changes: Vec<PriceChangeTotalInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoices: Option<Vec<InvoiceTotalInput>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InOutItemInput {
    pub pos_date_time: String,
    pub in_out: InOut,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserItemInput {
    pub employee_id: String,
    pub total_amount: f64,
    pub first_pos_date_time: String,
    pub last_pos_date_time: String,
    pub social_events: Vec<InOutItemInput>,
    pub payments: Vec<PaymentTotalInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTurnoverXInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub pos_devices: Vec<PosDeviceInput>,
    pub fdm_devices: Vec<FdmDeviceInput>,
    pub turnover: TurnoverInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTurnoverZInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub report_no: i64,
    pub report_booking_date: String,
    pub pos_devices: Vec<PosDeviceInput>,
    pub fdm_devices: Vec<FdmDeviceInput>,
    pub turnover: TurnoverInput,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportUserXInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub pos_devices: Vec<PosDeviceInput>,
    pub fdm_devices: Vec<FdmDeviceInput>,
    pub users: Vec<UserItemInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportUserZInput {
    #[serde(flatten)]
    pub common: CommonFields,
    pub report_no: i64,
    pub report_booking_date: String,
    pub pos_devices: Vec<PosDeviceInput>,
    pub fdm_devices: Vec<FdmDeviceInput>,
    pub users: Vec<UserItemInput>,
}
