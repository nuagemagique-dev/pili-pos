use serde::Serialize;

use crate::types::*;

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

/// The five-field tuple the FDM uses to detect a duplicate or conflicting
/// request (FDM spec v1.4 §3 idempotency rules). If the POS retries with
/// the exact same `IdempotencyKey`, the FDM returns the cached response
/// and does NOT advance its internal counters. Two requests sharing this
/// key but differing in *payload* are refused — the FDM treats that as a
/// conflict.
///
/// The key is constructed from `CommonFields` plus the event label
/// (which lives on the FDM response side, not the request side, so we
/// pass it explicitly). The order of fields here matches the spec.
// We can't derive `Hash` here because `EventLabel` doesn't implement
// it. `PartialEq + Eq` are enough for the FDM idempotency contract
// (the FDM dedupes by exact field equality, not by hash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyKey<'a> {
    pub pos_id: &'a str,
    pub pos_date_time: &'a str,
    pub terminal_id: Option<&'a str>,
    pub event_label: EventLabel,
    pub pos_fiscal_ticket_no: i64,
}

impl<'a> IdempotencyKey<'a> {
    /// Build an idempotency key from `CommonFields` and the event label.
    /// The label is taken from the spec-defined static mapping (e.g.
    /// `SaleInput` → `N`, `OrderInput` → `P`, …) — see the helpers on
    /// [`SaleInput::idempotency_key`] etc. for the per-event-type
    /// convenience constructors.
    pub fn new(common: &'a CommonFields, event_label: EventLabel) -> Self {
        Self {
            pos_id: &common.pos_id,
            pos_date_time: &common.pos_date_time,
            terminal_id: common.terminal_id.as_deref(),
            event_label,
            pos_fiscal_ticket_no: common.pos_fiscal_ticket_no,
        }
    }

    /// Single-line debug-style rendering used for log lines so a tail of
    /// the request log can be grepped by `pos_fiscal_ticket_no` quickly.
    ///
    /// Format (stable for log parsers):
    /// `posId=…,posDt=…,terminalId=…,label=N,ticketNo=1000`
    pub fn render(&self) -> String {
        format!(
            "posId={},posDt={},terminalId={},label={:?},ticketNo={}",
            self.pos_id,
            self.pos_date_time,
            self.terminal_id.unwrap_or(""),
            self.event_label,
            self.pos_fiscal_ticket_no
        )
    }
}

// Per-event-type idempotency-key constructors. We expose them as
// methods so callers can write `sale_input.idempotency_key()` without
// having to remember which event label maps to which input. The
// labels are fixed by spec POS v2.2.

impl SaleInput {
    /// N event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::N)
    }
}

impl OrderInput {
    /// P event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::P)
    }
}

impl PreBillInput {
    /// P event (pre-bill is the second flavour of P).
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::P)
    }
}

impl CostCenterChangeInput {
    /// P event (cost-center change is the third flavour of P).
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::P)
    }
}

impl WorkInOutInput {
    /// S event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::S)
    }
}

impl InvoiceInput {
    /// I event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::I)
    }
}

impl MoneyInOutInput {
    /// F event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::F)
    }
}

impl DrawerOpenInput {
    /// F event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::F)
    }
}

impl PaymentCorrectionInput {
    /// F event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::F)
    }
}

impl CopyInput {
    /// C event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::C)
    }
}

impl ReportTurnoverXInput {
    /// R event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::R)
    }
}

impl ReportTurnoverZInput {
    /// R event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::R)
    }
}

impl ReportUserXInput {
    /// R event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::R)
    }
}

impl ReportUserZInput {
    /// R event.
    pub fn idempotency_key(&self) -> IdempotencyKey<'_> {
        IdempotencyKey::new(&self.common, EventLabel::R)
    }
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

// ── Tests for the idempotency-key contract ─────────────────────

#[cfg(test)]
mod idempotency_tests {
    use super::*;

    fn common(pos_id: &str, ticket_no: i64, dt: &str, terminal: Option<&str>) -> CommonFields {
        CommonFields {
            language: Language::Fr,
            vat_no: "BE0499999960".to_string(),
            est_no: "2123456749".to_string(),
            pos_id: pos_id.to_string(),
            pos_fiscal_ticket_no: ticket_no,
            pos_date_time: dt.to_string(),
            pos_sw_version: "1.0.0".to_string(),
            terminal_id: terminal.map(str::to_string),
            device_id: "DEV001".to_string(),
            booking_period_id: "bp-1".to_string(),
            booking_date: "2026-05-14".to_string(),
            ticket_medium: TicketMedium::Paper,
            employee_id: "85073000148".to_string(),
        }
    }

    /// Two semantically-identical requests must produce equal keys.
    /// This is what the FDM uses to return a cached response on retry.
    #[test]
    fn identical_keys_compare_equal() {
        let c1 = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T1"));
        let c2 = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T1"));
        let k1 = IdempotencyKey::new(&c1, EventLabel::N);
        let k2 = IdempotencyKey::new(&c2, EventLabel::N);
        assert_eq!(k1, k2);
    }

    /// Differing ticket numbers → different keys (no dedup).
    #[test]
    fn different_ticket_no_differs() {
        let c1 = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T1"));
        let c2 = common("CPOS01", 43, "2026-05-14T10:00:00+02:00", Some("T1"));
        assert_ne!(
            IdempotencyKey::new(&c1, EventLabel::N),
            IdempotencyKey::new(&c2, EventLabel::N)
        );
    }

    /// Differing event labels → different keys, even at the same
    /// ticket number (per-event-label counters mean tied numbers are
    /// legal across labels).
    #[test]
    fn different_event_label_differs() {
        let c = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T1"));
        assert_ne!(
            IdempotencyKey::new(&c, EventLabel::N),
            IdempotencyKey::new(&c, EventLabel::P)
        );
    }

    /// Differing terminal IDs → different keys (multi-terminal setups
    /// share a single POS but each terminal's stream is independent).
    #[test]
    fn different_terminal_differs() {
        let c1 = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T1"));
        let c2 = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T2"));
        assert_ne!(
            IdempotencyKey::new(&c1, EventLabel::N),
            IdempotencyKey::new(&c2, EventLabel::N)
        );
    }

    /// Per-event-type idempotency-key convenience methods select the
    /// correct event label for each input shape (POS v2.2 §253-376).
    /// If anyone ever flips the wrong label by mistake, this catches it.
    #[test]
    fn per_event_type_keys_map_to_correct_label() {
        let c = common("CPOS01", 1, "2026-05-14T10:00:00+02:00", None);

        let sale = SaleInput {
            common: c.clone(),
            fdm_ref: None,
            cost_center: None,
            transaction: TransactionInput {
                transaction_lines: vec![],
                transaction_total: 0.0,
            },
            financials: vec![],
        };
        assert_eq!(sale.idempotency_key().event_label, EventLabel::N);

        let order = OrderInput {
            common: c.clone(),
            cost_center: CostCenterInput {
                id: "T1".into(),
                cost_center_type: CostCenterType::Table,
                reference: "ref".into(),
                cost_center: None,
            },
            transaction: TransactionInput {
                transaction_lines: vec![],
                transaction_total: 0.0,
            },
        };
        assert_eq!(order.idempotency_key().event_label, EventLabel::P);

        let work_in = WorkInOutInput { common: c.clone() };
        assert_eq!(work_in.idempotency_key().event_label, EventLabel::S);

        let drawer_open = DrawerOpenInput {
            common: c.clone(),
            drawer: None,
        };
        assert_eq!(drawer_open.idempotency_key().event_label, EventLabel::F);
    }

    /// `render()` is grep-friendly — log parsers can recover every
    /// field from a single line.
    #[test]
    fn rendered_key_is_log_grep_friendly() {
        let c = common("CPOS01", 42, "2026-05-14T10:00:00+02:00", Some("T1"));
        let k = IdempotencyKey::new(&c, EventLabel::N);
        let s = k.render();
        assert!(s.contains("posId=CPOS01"));
        assert!(s.contains("ticketNo=42"));
        assert!(s.contains("label=N"));
        assert!(s.contains("terminalId=T1"));
        assert!(s.contains("posDt=2026-05-14T10:00:00+02:00"));
    }
}
