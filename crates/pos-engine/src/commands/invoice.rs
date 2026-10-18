//! B2B invoices (event I, M150) and the customers they are made out to.
//!
//! An invoice is made from sales already paid at the till: the cashier picks signed N events
//! and a customer, the engine reads the products, VAT and totals from the archive (V11-D3,
//! the screen never supplies lines), numbers the invoice (V11-D5), signs the I event
//! (`signInvoice`, `fdmRefs` of the sales, `costCenter` of type CUSTOMER, V11-D6), records the
//! invoice rows in the same transaction as the I archive (`PendingContext::Invoice`, see
//! `signing.rs`), prints the invoice (`pos_print::invoice`) and, when the installation has a
//! `peppol` section and the customer is Belgian, queues the UBL document for delivery
//! (`peppol_queue.rs`, V11-D12).
//!
//! Rules, one by one:
//!
//! - one sale is on at most one invoice (V11-D4); a refund N may be invoiced (UC4122);
//!   training events, copies, orders and sales closed unpaid are refused;
//! - no invoice in training mode (V11-D7);
//! - the VAT number is checked offline (Belgian check digits, V11-D10); the Peppol lookup is
//!   cached on the customer and never blocks the invoice: a failed or slow lookup is
//!   `unknown`;
//! - invoices are issued by a cashier or an admin, the Peppol panel is for admins (V11-D8).

use std::collections::BTreeMap;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use pos_core::config::PosConfig;
use pos_db::queries::fiscal::FiscalEventRef;
use pos_db::queries::invoicing as q;
use pos_fiscal::event::build_invoice_input;
use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::inputs::CostCenterInput;
use pos_fiscal::response::{FdmReference, SignResult};
use pos_fiscal::ticket::{ReceiptLanguage, is_unpaid_sale};
use pos_fiscal::types::{CostCenterType, EventLabel};
use pos_peppol::parse_vat_number;
use pos_peppol::ubl::{self, InvoiceSource, SourceAdjustment, SourceLine};
use pos_print::invoice::{
    InvoiceControl, InvoiceLinePrint, InvoiceParty, InvoicePrint, InvoiceSaleRef, InvoiceVatRow,
};

use crate::commands::access::{self, Access};
use crate::commands::errors::{self, coded, incomplete};
use crate::commands::signing::{self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared};
use crate::state::AppState;

pub use q::{Customer, Delivery};

/// Invoices a screen lists at once, by default.
const DEFAULT_LIMIT: i64 = 30;
/// Customers a search returns at most.
const SEARCH_LIMIT: i64 = 20;
/// A Peppol lookup older than this is made again by the queue (V11-D10).
const LOOKUP_FRESH_DAYS: i64 = 30;
/// Longest customer name: the `costCenter.reference` limit of the FDM.
const NAME_MAX_CHARS: usize = 600;

pub(crate) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

// ── Types the screens see ─────────────────────────────────────────

/// A signed sale a cashier can put on an invoice.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceableSale {
    pub fiscal_event_id: i64,
    pub ticket_number: i64,
    pub pos_date_time: String,
    /// Negative for a refund.
    pub total_cents: i64,
    pub fdm_id: String,
    pub event_counter: i64,
    pub total_counter: i64,
    pub is_refund: bool,
    /// The number of the invoice this sale is on, if any.
    pub invoiced_by: Option<String>,
}

/// One row of a preview: a product line, or a price change of the whole ticket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewLine {
    pub description: String,
    /// Quantity in thousandths (2 pieces = 2000).
    pub quantity_milli: i64,
    pub total_cents: i64,
    pub vat_label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewVat {
    pub label: String,
    /// Hundredths of a percent (21 % = 2100).
    pub rate_hundredths: i64,
    pub base_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
}

/// What an invoice would contain, without signing anything.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePreview {
    pub sales: Vec<InvoiceableSale>,
    pub lines: Vec<PreviewLine>,
    pub vat: Vec<PreviewVat>,
    pub total_cents: i64,
    /// Coded errors (`invoice_sale_already_invoiced|event=..|invoice=..`, ...). The invoice
    /// can be signed only when there are none.
    pub problems: Vec<String>,
}

/// What a customer form holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2bCustomerInput {
    /// Set to edit a known customer.
    #[serde(default)]
    pub id: Option<i64>,
    pub vat_number: String,
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    /// ISO 3166-1 alpha-2; the country of the VAT number when empty.
    #[serde(default)]
    pub country_code: String,
}

/// The answer to "who is this VAT number".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerLookup {
    /// The number in its canonical spelling (`BE0308357159`).
    pub vat_number: String,
    pub country_code: String,
    /// The customer already known under this number.
    pub customer: Option<Customer>,
    /// `registered`, `not_registered` or `unknown` (offline, slow, or not asked).
    pub peppol_status: String,
    /// The access point answered just now.
    pub peppol_checked: bool,
    /// Only Belgian customers are delivered over Peppol (V11-D9).
    pub deliverable: bool,
}

/// What `sign_invoice_fiscal` returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceResult {
    pub success: bool,
    /// Archive id of the signed I event.
    pub fiscal_event_id: Option<i64>,
    pub invoice_no: Option<String>,
    /// The invoice reached the printer.
    pub printed: bool,
    /// `queued` when a Peppol delivery was queued, else `None`.
    pub delivery_status: Option<String>,
    pub error: Option<String>,
}

/// Old name of [`InvoiceResult`], kept for the M150 test.
pub type InvoiceSignResult = InvoiceResult;

impl InvoiceResult {
    fn failed(error: impl Into<String>) -> Self {
        Self {
            success: false,
            fiscal_event_id: None,
            invoice_no: None,
            printed: false,
            delivery_status: None,
            error: Some(error.into()),
        }
    }
}

/// One invoice in the list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSummary {
    pub id: i64,
    pub invoice_no: String,
    pub fiscal_event_id: i64,
    pub issue_date: String,
    pub customer_name: String,
    pub customer_vat_number: String,
    pub total_cents: i64,
    pub sale_event_ids: Vec<i64>,
    /// Status of the Peppol delivery: `queued`, `accepted`, `failed`; `None` when none was
    /// queued.
    pub delivery_status: Option<String>,
    pub delivery_id: Option<i64>,
}

/// The Peppol panel: how the installation is set up, the queue counts and the rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeppolDeliveries {
    pub configured: bool,
    pub provider: Option<String>,
    pub sender_id: Option<String>,
    pub queued: i64,
    pub accepted: i64,
    pub failed: i64,
    pub deliveries: Vec<Delivery>,
}

// ── Reading the archive ───────────────────────────────────────────

/// A JSON amount (multiple of 0.01) in eurocents.
fn cents(v: &serde_json::Value) -> i64 {
    (v.as_f64().unwrap_or(0.0) * 100.0).round() as i64
}

#[derive(Debug, Clone)]
pub(crate) struct DocLine {
    pub name: String,
    pub quantity_milli: i64,
    pub unit_code: &'static str,
    pub incl_cents: i64,
    pub vat_label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DocAdjustment {
    pub reason: String,
    pub vat_label: String,
    pub incl_cents: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct DocVat {
    pub label: String,
    pub rate_hundredths: i64,
    pub base_cents: i64,
    pub vat_cents: i64,
    pub total_cents: i64,
}

/// One sale (event N), read from its archive.
#[derive(Debug, Clone)]
pub(crate) struct SaleDoc {
    pub event_id: i64,
    pub ticket_number: i64,
    pub pos_date_time: String,
    pub fdm_ref: FdmReference,
    pub total_cents: i64,
    pub lines: Vec<DocLine>,
    pub adjustments: Vec<DocAdjustment>,
    pub vat: Vec<DocVat>,
}

impl SaleDoc {
    fn is_refund(&self) -> bool {
        self.total_cents < 0
    }
}

fn unit_code(quantity_type: &str) -> &'static str {
    match quantity_type {
        "KILOGRAM" => "KGM",
        "METER" => "MTR",
        "LITRE" => "LTR",
        "HOUR" => "HUR",
        _ => "H87",
    }
}

/// The rate of a VAT label, in hundredths of a percent (SCE 2.0: A 21, B 12, C 6, D and X 0).
fn label_rate_hundredths(label: &str) -> i64 {
    match label {
        "A" => 2100,
        "B" => 1200,
        "C" => 600,
        _ => 0,
    }
}

/// Read a signed sale from the archive: its product lines, its price changes, the VAT the
/// FDM computed. Refuses what is not an invoiceable sale.
pub(crate) fn read_sale(event: &FiscalEventRef) -> Result<SaleDoc, String> {
    let not_a_sale = || coded(errors::INVOICE_NOT_A_SALE, &[("event", &event.id)]);
    if event.event_type != "N" || event.event_label.as_deref() != Some("N") {
        return Err(not_a_sale());
    }
    let request: serde_json::Value =
        serde_json::from_str(&event.event_data_json).map_err(errors::internal)?;
    if is_unpaid_sale(&request) {
        return Err(not_a_sale());
    }
    let fdm_ref = FdmReference {
        fdm_id: event.fdm_id.clone().ok_or_else(|| incomplete("fdmId"))?,
        fdm_date_time: event
            .fdm_datetime
            .clone()
            .ok_or_else(|| incomplete("fdmDateTime"))?,
        event_label: EventLabel::N,
        event_counter: event
            .event_counter
            .ok_or_else(|| incomplete("eventCounter"))?,
        total_counter: event
            .total_counter
            .ok_or_else(|| incomplete("totalCounter"))?,
    };

    let mut lines = Vec::new();
    // (change id, name, label) -> amount of the changes that apply to the whole ticket.
    let mut event_changes: BTreeMap<(String, String, String), i64> = BTreeMap::new();
    for line in request["transaction"]["transactionLines"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let product = &line["mainProduct"];
        let name = product["productName"].as_str().unwrap_or_default();
        let quantity_milli = (product["quantity"].as_f64().unwrap_or(1.0) * 1000.0).round() as i64;
        let unit = unit_code(product["quantityType"].as_str().unwrap_or("PIECE"));
        let vats = product["vats"].as_array().cloned().unwrap_or_default();
        let split = vats.len() > 1;
        for vat in &vats {
            let label = vat["label"].as_str().unwrap_or_default().to_string();
            let mut incl = cents(&vat["price"]);
            for change in vat["priceChanges"].as_array().into_iter().flatten() {
                let amount = cents(&change["amount"]);
                if change["scope"].as_str() == Some("EVENT") {
                    let key = (
                        change["id"].as_str().unwrap_or_default().to_string(),
                        change["name"].as_str().unwrap_or_default().to_string(),
                        label.clone(),
                    );
                    *event_changes.entry(key).or_insert(0) += amount;
                } else {
                    incl += amount;
                }
            }
            lines.push(DocLine {
                // A product sold under several VAT labels is shown once per label.
                name: if split {
                    format!("{name} ({label})")
                } else {
                    name.to_string()
                },
                quantity_milli: if split { 1000 } else { quantity_milli },
                unit_code: if split { "H87" } else { unit },
                incl_cents: incl,
                vat_label: label,
            });
        }
    }
    let adjustments: Vec<DocAdjustment> = event_changes
        .into_iter()
        .filter(|(_, amount)| *amount != 0)
        .map(|((_, reason, vat_label), incl_cents)| DocAdjustment {
            reason,
            vat_label,
            incl_cents,
        })
        .collect();
    let total_cents = lines.iter().map(|l| l.incl_cents).sum::<i64>()
        + adjustments.iter().map(|a| a.incl_cents).sum::<i64>();

    // VAT as the FDM computed it for this ticket.
    let response: serde_json::Value = event
        .fdm_response_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or(serde_json::Value::Null);
    let mut vat: Vec<DocVat> = response["vatCalc"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|v| DocVat {
            label: v["label"].as_str().unwrap_or_default().to_string(),
            rate_hundredths: (v["rate"].as_f64().unwrap_or(0.0) * 100.0).round() as i64,
            base_cents: cents(&v["taxableAmount"]),
            vat_cents: cents(&v["vatAmount"]),
            total_cents: cents(&v["totalAmount"]),
        })
        .collect();
    if vat.is_empty() {
        // The FDM always returns it for an N; derived from the lines should it ever be missing.
        let mut by_label: BTreeMap<String, i64> = BTreeMap::new();
        for l in &lines {
            *by_label.entry(l.vat_label.clone()).or_insert(0) += l.incl_cents;
        }
        for a in &adjustments {
            *by_label.entry(a.vat_label.clone()).or_insert(0) += a.incl_cents;
        }
        vat = by_label
            .into_iter()
            .map(|(label, total)| {
                let rate = label_rate_hundredths(&label);
                let base =
                    (total * 10_000 + (10_000 + rate) / 2 * total.signum()) / (10_000 + rate);
                DocVat {
                    label,
                    rate_hundredths: rate,
                    base_cents: base,
                    vat_cents: total - base,
                    total_cents: total,
                }
            })
            .collect();
    }

    Ok(SaleDoc {
        event_id: event.id,
        ticket_number: request["posFiscalTicketNo"].as_i64().unwrap_or_default(),
        pos_date_time: request["posDateTime"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        fdm_ref,
        total_cents,
        lines,
        adjustments,
        vat,
    })
}

/// The sales an invoice covers.
#[derive(Debug, Clone)]
pub(crate) struct InvoiceDoc {
    pub sales: Vec<SaleDoc>,
}

impl InvoiceDoc {
    pub fn total_cents(&self) -> i64 {
        self.sales.iter().map(|s| s.total_cents).sum()
    }

    /// Product lines, then the price changes of the whole tickets, aggregated.
    fn adjustments(&self) -> Vec<DocAdjustment> {
        let mut sum: BTreeMap<(String, String), i64> = BTreeMap::new();
        for a in self.sales.iter().flat_map(|s| &s.adjustments) {
            *sum.entry((a.reason.clone(), a.vat_label.clone()))
                .or_insert(0) += a.incl_cents;
        }
        sum.into_iter()
            .filter(|(_, v)| *v != 0)
            .map(|((reason, vat_label), incl_cents)| DocAdjustment {
                reason,
                vat_label,
                incl_cents,
            })
            .collect()
    }

    /// The FDM's VAT, summed per label.
    pub fn vat(&self) -> Vec<DocVat> {
        let mut sum: BTreeMap<String, DocVat> = BTreeMap::new();
        for v in self.sales.iter().flat_map(|s| &s.vat) {
            let e = sum.entry(v.label.clone()).or_insert_with(|| DocVat {
                label: v.label.clone(),
                rate_hundredths: v.rate_hundredths,
                base_cents: 0,
                vat_cents: 0,
                total_cents: 0,
            });
            e.base_cents += v.base_cents;
            e.vat_cents += v.vat_cents;
            e.total_cents += v.total_cents;
        }
        sum.into_values().collect()
    }

    /// `{"A": {"base": n, "vat": n, "total": n}, ...}` as stored with the invoice.
    pub fn vat_json(&self) -> String {
        let mut map = serde_json::Map::new();
        for v in self.vat() {
            map.insert(
                v.label.clone(),
                serde_json::json!({"base": v.base_cents, "vat": v.vat_cents, "total": v.total_cents}),
            );
        }
        serde_json::Value::Object(map).to_string()
    }

    fn print_lines(&self) -> Vec<InvoiceLinePrint> {
        let mut out: Vec<InvoiceLinePrint> = self
            .sales
            .iter()
            .flat_map(|s| &s.lines)
            .map(|l| InvoiceLinePrint {
                quantity_milli: l.quantity_milli,
                description: l.name.clone(),
                total_cents: l.incl_cents,
                vat_label: l.vat_label.clone(),
            })
            .collect();
        out.extend(self.adjustments().into_iter().map(|a| InvoiceLinePrint {
            quantity_milli: 1000,
            description: a.reason,
            total_cents: a.incl_cents,
            vat_label: a.vat_label,
        }));
        out
    }

    fn preview_lines(&self) -> Vec<PreviewLine> {
        self.print_lines()
            .into_iter()
            .map(|l| PreviewLine {
                description: l.description,
                quantity_milli: l.quantity_milli,
                total_cents: l.total_cents,
                vat_label: l.vat_label,
            })
            .collect()
    }
}

fn sale_ref(s: &SaleDoc) -> InvoiceSaleRef {
    InvoiceSaleRef {
        ticket_number: s.ticket_number,
        pos_date_time: s.pos_date_time.clone(),
        fdm_id: s.fdm_ref.fdm_id.clone(),
        event_label: "N".to_string(),
        event_counter: s.fdm_ref.event_counter,
        total_counter: s.fdm_ref.total_counter,
        total_cents: s.total_cents,
    }
}

fn invoiceable(s: &SaleDoc, invoiced_by: Option<String>) -> InvoiceableSale {
    InvoiceableSale {
        fiscal_event_id: s.event_id,
        ticket_number: s.ticket_number,
        pos_date_time: s.pos_date_time.clone(),
        total_cents: s.total_cents,
        fdm_id: s.fdm_ref.fdm_id.clone(),
        event_counter: s.fdm_ref.event_counter,
        total_counter: s.fdm_ref.total_counter,
        is_refund: s.is_refund(),
        invoiced_by,
    }
}

fn label_name(label: EventLabel) -> String {
    serde_json::to_value(label)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// The FDM control data of a signed event, without its signature.
pub(crate) fn control_of(result: &SignResult) -> InvoiceControl {
    InvoiceControl {
        fdm_id: result.fdm_ref.fdm_id.clone(),
        fdm_date_time: result.fdm_ref.fdm_date_time.clone(),
        event_label: label_name(result.fdm_ref.event_label),
        event_counter: result.fdm_ref.event_counter,
        total_counter: result.fdm_ref.total_counter,
        pos_fiscal_ticket_no: result.pos_fiscal_ticket_no,
        pos_date_time: result.pos_date_time.clone(),
    }
}

// ── Customers ──────────────────────────────────────────────────────

fn party_of(c: &Customer) -> InvoiceParty {
    InvoiceParty {
        name: c.name.clone(),
        street: c.street.clone(),
        postal_code: c.postal_code.clone(),
        city: c.city.clone(),
        country_code: c.country_code.clone(),
        vat_number: c.vat_number.clone(),
    }
}

/// A customer form, checked: the VAT number offline, every address field present.
fn validate_customer(input: &B2bCustomerInput) -> Result<q::NewCustomer, String> {
    let vat = parse_vat_number(&input.vat_number)
        .map_err(|_| errors::CUSTOMER_VAT_INVALID.to_string())?;
    let field = |name: &str, value: &str| -> Result<String, String> {
        let v = value.trim();
        if v.is_empty() || v.chars().count() > NAME_MAX_CHARS {
            return Err(coded(errors::CUSTOMER_INCOMPLETE, &[("field", &name)]));
        }
        Ok(v.to_string())
    };
    let country = if input.country_code.trim().is_empty() {
        vat.country.clone()
    } else {
        input.country_code.trim().to_uppercase()
    };
    let country_ok = country.len() == 2
        && country.bytes().all(|b| b.is_ascii_uppercase())
        // A Belgian number goes with a Belgian address, and the other way round.
        && (vat.is_belgian() == (country == "BE"));
    if !country_ok {
        return Err(coded(
            errors::CUSTOMER_INCOMPLETE,
            &[("field", &"country_code")],
        ));
    }
    Ok(q::NewCustomer {
        vat_number: vat.canonical,
        name: field("name", &input.name)?,
        street: field("street", &input.street)?,
        postal_code: field("postal_code", &input.postal_code)?,
        city: field("city", &input.city)?,
        country_code: country,
    })
}

/// Create or update a customer (a cashier may). A new customer under a VAT number already
/// known updates that customer.
pub fn do_save_b2b_customer(state: &AppState, input: B2bCustomerInput) -> Result<Customer, String> {
    let user = access::operator(state, Access::Cashier)?;
    let new = validate_customer(&input)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let existing = q::find_customer_by_vat(&db, &new.vat_number).map_err(errors::internal)?;
    let id = match (input.id, existing) {
        (Some(id), other) => {
            if q::get_customer(&db, id)
                .map_err(errors::internal)?
                .is_none()
            {
                return Err(errors::CUSTOMER_NOT_FOUND.to_string());
            }
            // Another customer holds that number.
            if other.is_some_and(|o| o.id != id) {
                return Err(coded(
                    errors::CUSTOMER_INCOMPLETE,
                    &[("field", &"vat_number")],
                ));
            }
            q::update_customer(&db, id, &new, &user.insz).map_err(errors::internal)?;
            id
        }
        (None, Some(known)) => {
            q::update_customer(&db, known.id, &new, &user.insz).map_err(errors::internal)?;
            known.id
        }
        (None, None) => q::insert_customer(&db, &new, &user.insz).map_err(errors::internal)?,
    };
    q::get_customer(&db, id)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::CUSTOMER_NOT_FOUND.to_string())
}

/// Known customers whose VAT number or name starts with `query` (at most 20).
pub fn do_search_b2b_customers(state: &AppState, query: &str) -> Result<Vec<Customer>, String> {
    access::operator(state, Access::Cashier)?;
    let db = state.db.lock().map_err(errors::internal)?;
    q::search_customers(&db, query, SEARCH_LIMIT).map_err(errors::internal)
}

/// Check a typed VAT number offline, find the customer known under it and ask the Peppol
/// network whether it can receive invoices (3 seconds at most; V11-D10). A failed or slow
/// lookup answers `unknown` and never raises: the invoice does not wait for it.
pub async fn do_lookup_b2b_customer(
    state: &AppState,
    vat_number: &str,
) -> Result<CustomerLookup, String> {
    access::operator(state, Access::Cashier)?;
    let vat = parse_vat_number(vat_number).map_err(|_| errors::CUSTOMER_VAT_INVALID.to_string())?;
    let customer = {
        let db = state.db.lock().map_err(errors::internal)?;
        q::find_customer_by_vat(&db, &vat.canonical).map_err(errors::internal)?
    };
    let cached = customer
        .as_ref()
        .map(|c| c.peppol_status.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut out = CustomerLookup {
        vat_number: vat.canonical.clone(),
        country_code: vat.country.clone(),
        customer: customer.clone(),
        peppol_status: cached,
        peppol_checked: false,
        deliverable: vat.is_belgian(),
    };
    let Some(participant) = vat.participant_id() else {
        return Ok(out);
    };
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let Some(peppol) = config.peppol.as_ref() else {
        return Ok(out);
    };
    let answer = match crate::commands::peppol_queue::access_point(state, peppol) {
        Ok(ap) => tokio::time::timeout(pos_peppol::LOOKUP_TIMEOUT, ap.lookup(&participant)).await,
        Err(e) => {
            tracing::warn!(%e, "peppol lookup: no access point");
            return Ok(out);
        }
    };
    match answer {
        Ok(Ok(result)) => {
            out.peppol_status = result.as_str().to_string();
            out.peppol_checked = true;
            if let Some(c) = &customer {
                let db = state.db.lock().map_err(errors::internal)?;
                q::set_customer_peppol_status(&db, c.id, result.as_str(), &now_rfc3339())
                    .map_err(errors::internal)?;
                out.customer = q::get_customer(&db, c.id).map_err(errors::internal)?;
            }
        }
        Ok(Err(e)) => {
            tracing::info!(%e, "peppol lookup failed: status unknown");
            out.peppol_status = "unknown".to_string();
        }
        Err(_) => {
            tracing::info!("peppol lookup timed out: status unknown");
            out.peppol_status = "unknown".to_string();
        }
    }
    Ok(out)
}

// ── Listing and preview ───────────────────────────────────────────

fn load_sale(db: &rusqlite::Connection, id: i64) -> Result<SaleDoc, String> {
    let event = pos_db::queries::fiscal::get_fiscal_event(db, id)
        .map_err(errors::internal)?
        .ok_or_else(|| coded(errors::INVOICE_SALE_NOT_FOUND, &[("event", &id)]))?;
    read_sale(&event)
}

/// Recent signed sales a cashier can invoice, newest first. `query` keeps the tickets whose
/// number or date starts with it.
pub fn do_list_invoiceable_sales(
    state: &AppState,
    limit: Option<i64>,
    query: Option<String>,
) -> Result<Vec<InvoiceableSale>, String> {
    access::operator(state, Access::Cashier)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let ids = q::recent_sales(
        &db,
        limit.unwrap_or(DEFAULT_LIMIT).clamp(1, 200),
        &query.unwrap_or_default(),
    )
    .map_err(errors::internal)?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        // A sale whose archive cannot be read is left out of the list.
        let Ok(sale) = load_sale(&db, id) else {
            continue;
        };
        let by = q::invoice_of_sale(&db, id).map_err(errors::internal)?;
        out.push(invoiceable(&sale, by));
    }
    Ok(out)
}

/// The sales to invoice, checked one by one: the valid ones, and the coded problems.
fn check_sales(
    db: &rusqlite::Connection,
    ids: &[i64],
) -> Result<(Vec<SaleDoc>, Vec<String>), String> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let mut problems = Vec::new();
    let mut sales = Vec::new();
    if ids.is_empty() {
        problems.push(errors::INVOICE_NO_SALES.to_string());
    }
    for id in ids {
        match load_sale(db, id) {
            Ok(sale) => match q::invoice_of_sale(db, id).map_err(errors::internal)? {
                Some(no) => problems.push(coded(
                    errors::INVOICE_SALE_ALREADY_INVOICED,
                    &[("event", &id), ("invoice", &no)],
                )),
                None => sales.push(sale),
            },
            Err(e) => problems.push(e),
        }
    }
    Ok((sales, problems))
}

/// What an invoice of these sales would contain: lines, VAT, total, and the problems that
/// would stop it. Signs nothing.
pub fn do_invoice_preview(
    state: &AppState,
    sale_event_ids: &[i64],
) -> Result<InvoicePreview, String> {
    access::operator(state, Access::Cashier)?;
    let training = state.config.lock().map_err(errors::internal)?.training_mode;
    let db = state.db.lock().map_err(errors::internal)?;
    let (sales, mut problems) = check_sales(&db, sale_event_ids)?;
    if training {
        problems.insert(0, errors::INVOICE_TRAINING_MODE.to_string());
    }
    let doc = InvoiceDoc { sales };
    Ok(InvoicePreview {
        sales: doc.sales.iter().map(|s| invoiceable(s, None)).collect(),
        lines: doc.preview_lines(),
        vat: doc
            .vat()
            .into_iter()
            .map(|v| PreviewVat {
                label: v.label,
                rate_hundredths: v.rate_hundredths,
                base_cents: v.base_cents,
                vat_cents: v.vat_cents,
                total_cents: v.total_cents,
            })
            .collect(),
        total_cents: doc.total_cents(),
        problems,
    })
}

// ── The print and the UBL of an invoice ───────────────────────────

/// What identifies an invoice, whether it is about to be signed or was signed long ago.
struct InvoiceFacts<'a> {
    invoice_no: &'a str,
    issue_date: &'a str,
    customer: &'a InvoiceParty,
    doc: &'a InvoiceDoc,
}

fn build_print(
    state: &AppState,
    config: &PosConfig,
    operator: &str,
    facts: &InvoiceFacts<'_>,
    control: Option<InvoiceControl>,
) -> InvoicePrint {
    InvoicePrint {
        lang: ReceiptLanguage::from_code(&config.language),
        seller_name: config.restaurant_name.clone(),
        seller_legal_name: config.legal_name().to_string(),
        seller_address: config.restaurant_address.clone(),
        seller_vat: config.restaurant_vat.clone(),
        seller_est_no: config.restaurant_est_no.clone(),
        customer: facts.customer.clone(),
        invoice_no: facts.invoice_no.to_string(),
        issue_date: facts.issue_date.to_string(),
        sales: facts.doc.sales.iter().map(sale_ref).collect(),
        lines: facts.doc.print_lines(),
        vat: facts
            .doc
            .vat()
            .into_iter()
            .map(|v| InvoiceVatRow {
                label: v.label,
                rate_hundredths: v.rate_hundredths,
                base_cents: v.base_cents,
                vat_cents: v.vat_cents,
                total_cents: v.total_cents,
            })
            .collect(),
        total_cents: facts.doc.total_cents(),
        operator: operator.to_string(),
        pos_id: config.pos_id.clone(),
        pos_sw_version: state.identity.to_string(),
        terminal_id: config.terminal_id.clone(),
        control,
        copy: None,
    }
}

/// The UBL document of an invoice, for a Belgian customer.
fn build_ubl(
    config: &PosConfig,
    peppol: &pos_core::config::PeppolConfig,
    facts: &InvoiceFacts<'_>,
) -> Result<String, String> {
    let customer_vat = parse_vat_number(&facts.customer.vat_number)
        .map_err(|_| errors::CUSTOMER_VAT_INVALID.to_string())?;
    let customer_participant = customer_vat
        .participant_id()
        .ok_or_else(|| errors::CUSTOMER_VAT_INVALID.to_string())?;
    let (_, customer_digits) = customer_participant
        .split_once(':')
        .ok_or_else(|| errors::CUSTOMER_VAT_INVALID.to_string())?;
    let supplier_digits = peppol
        .sender_id
        .split_once(':')
        .map_or(peppol.sender_id.as_str(), |(_, v)| v);
    let party = |scheme: &str, value: &str, p: &InvoiceParty, vat: &str| ubl::Party {
        endpoint_scheme: scheme.to_string(),
        endpoint_value: value.to_string(),
        name: p.name.clone(),
        address: ubl::Address {
            street: p.street.clone(),
            postal_code: p.postal_code.clone(),
            city: p.city.clone(),
            country_code: p.country_code.clone(),
        },
        vat_no: vat.to_string(),
    };
    let supplier_party = InvoiceParty {
        name: config.legal_name().to_string(),
        street: peppol.supplier_street.clone(),
        postal_code: peppol.supplier_postal_code.clone(),
        city: peppol.supplier_city.clone(),
        country_code: peppol.supplier_country.clone(),
        vat_number: config.restaurant_vat.clone(),
    };
    let lines = facts
        .doc
        .sales
        .iter()
        .flat_map(|s| &s.lines)
        .map(|l| SourceLine {
            name: l.name.clone(),
            quantity_milli: l.quantity_milli,
            unit_code: l.unit_code.to_string(),
            incl_cents: l.incl_cents,
            vat_label: l.vat_label.clone(),
        })
        .collect();
    let adjustments = facts
        .doc
        .adjustments()
        .into_iter()
        .map(|a| SourceAdjustment {
            reason: a.reason,
            incl_cents: a.incl_cents,
            vat_label: a.vat_label,
        })
        .collect();
    let refs: Vec<String> = facts
        .doc
        .sales
        .iter()
        .map(|s| {
            format!(
                "{}-N{}/{}",
                s.fdm_ref.fdm_id, s.fdm_ref.event_counter, s.fdm_ref.total_counter
            )
        })
        .collect();
    let source = InvoiceSource {
        invoice_no: facts.invoice_no.to_string(),
        issue_date: facts.issue_date.to_string(),
        supplier: party(
            "0208",
            supplier_digits,
            &supplier_party,
            &config.restaurant_vat,
        ),
        customer: party(
            "0208",
            customer_digits,
            facts.customer,
            &customer_vat.canonical,
        ),
        lines,
        adjustments,
        prepaid_cents: facts.doc.total_cents(),
        note: Some(format!("Already paid. Tickets: {}", refs.join(", "))),
    };
    let invoice = ubl::InvoiceUbl::build(&source).map_err(errors::internal)?;
    ubl::to_ubl_xml(&invoice).map_err(errors::internal)
}

/// How the Peppol delivery of an invoice is queued, stored with the signing request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PendingPeppol {
    pub participant: String,
    pub ubl_xml: String,
    pub ap_vendor: String,
    /// What the cache says: `unknown` makes the queue ask the network first.
    pub lookup_status: String,
}

/// The lookup status a new delivery starts with: the cached one while it is fresh, else
/// `unknown` (V11-D10).
fn effective_lookup_status(c: &Customer) -> String {
    let fresh = c
        .peppol_checked_at
        .as_deref()
        .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
        .is_some_and(|t| {
            Utc::now().signed_duration_since(t) < chrono::Duration::days(LOOKUP_FRESH_DAYS)
        });
    if fresh {
        c.peppol_status.clone()
    } else {
        "unknown".to_string()
    }
}

fn vendor_of(provider: &str) -> String {
    match provider {
        "storecove" => "storecove".to_string(),
        _ => "logging-only (no live delivery)".to_string(),
    }
}

// ── Signing ───────────────────────────────────────────────────────

/// M150: sign the invoice (event I) of `sale_event_ids` for `customer_id`, record it, print
/// it and queue its Peppol delivery. Refusals before anything is signed are errors; an FDM
/// that does not sign is `success: false`, nothing recorded and no number used.
pub async fn do_sign_invoice_fiscal(
    state: &AppState,
    sale_event_ids: Vec<i64>,
    customer_id: i64,
    employee_insz: String,
) -> Result<InvoiceResult, String> {
    access::operator_named(state, &employee_insz, Access::Cashier)?;
    if state.config.lock().map_err(errors::internal)?.training_mode {
        return Err(errors::INVOICE_TRAINING_MODE.to_string());
    }
    let mut ids = sale_event_ids;
    ids.sort_unstable();
    ids.dedup();
    if ids.is_empty() {
        return Err(errors::INVOICE_NO_SALES.to_string());
    }
    let op = Op::new(Kind::Invoice, format!("{ids:?}|{customer_id}"));
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // This invoice, sent earlier, has been signed (and printed): nothing left to do.
        Ok(Begin::AlreadyDone(recovered)) => {
            let (invoice_no, event_id) = match &recovered.context {
                PendingContext::Invoice { invoice_no, .. } => {
                    (Some(invoice_no.clone()), recovered.committed.event_ids[0])
                }
                _ => (None, recovered.committed.event_ids[0]),
            };
            let delivery_status = delivery_status_of(state, event_id);
            return Ok(InvoiceResult {
                success: true,
                fiscal_event_id: Some(event_id),
                invoice_no,
                printed: recovered.follow_up.invoice_printed.unwrap_or(false),
                delivery_status,
                error: None,
            });
        }
        Err(e) => return Ok(InvoiceResult::failed(e)),
    };

    let config = state.config.lock().map_err(errors::internal)?.clone();
    // Before the database lock: the name lookup takes it too.
    let operator = super::operator_name(state, &employee_insz);
    let prepared = {
        let db = state.db.lock().map_err(errors::internal)?;
        let bp = super::booking(&db, &guard, &config)?;
        let customer = q::get_customer(&db, customer_id)
            .map_err(errors::internal)?
            .ok_or_else(|| errors::CUSTOMER_NOT_FOUND.to_string())?;
        let vat = parse_vat_number(&customer.vat_number)
            .map_err(|_| errors::CUSTOMER_VAT_INVALID.to_string())?;
        // Since 2026-01-01 a B2B invoice to a Belgian VAT-registered customer goes through
        // Peppol: without a `peppol` section the invoice is not compliant, so it is refused
        // before anything is signed (V11-D55).
        if vat.is_belgian() && config.peppol.is_none() {
            return Err(errors::INVOICE_PEPPOL_REQUIRED.to_string());
        }

        // The sales, each checked again under the lock.
        let (checked, problems) = check_sales(&db, &ids)?;
        if let Some(first) = problems.into_iter().next() {
            return Err(first);
        }
        let doc = InvoiceDoc { sales: checked };
        let fdm_refs: Vec<FdmReference> = doc.sales.iter().map(|s| s.fdm_ref.clone()).collect();

        // The next ticket number of the I counter, and the continuous invoice number. Both
        // are used only when the signed event is archived (no gap on an FDM failure).
        let ticket_number = signing::peek(&db, &guard, EventLabel::I)?;
        let invoice_seq = q::next_invoice_seq(&db).map_err(errors::internal)?;
        let mut input = build_invoice_input(
            &config,
            &state.identity,
            ticket_number,
            &employee_insz,
            &bp,
            "",
            &vat.canonical,
            &fdm_refs,
        );
        let issue_date = input
            .common
            .pos_date_time
            .get(..10)
            .unwrap_or_default()
            .to_string();
        let invoice_no = format!(
            "{}{:04}",
            issue_date.get(..4).unwrap_or("0000"),
            invoice_seq
        );
        input.invoice_no = invoice_no.clone();
        input.cost_center = Some(CostCenterInput {
            id: format!("C{}", customer.id),
            cost_center_type: CostCenterType::Customer,
            reference: customer.name.chars().take(NAME_MAX_CHARS).collect(),
            cost_center: None,
        });
        let request =
            SignRequest::new(SignMutation::Invoice, &input, false).map_err(errors::internal)?;
        let request_json = serde_json::to_string(&input).map_err(errors::internal)?;

        let party = party_of(&customer);
        let facts = InvoiceFacts {
            invoice_no: &invoice_no,
            issue_date: &issue_date,
            customer: &party,
            doc: &doc,
        };
        let print = build_print(state, &config, &operator, &facts, None);
        // The Peppol delivery: only with a `peppol` section and a Belgian customer.
        let peppol = match (&config.peppol, vat.participant_id()) {
            (Some(pc), Some(participant)) => Some(PendingPeppol {
                participant,
                ubl_xml: build_ubl(&config, pc, &facts)?,
                ap_vendor: vendor_of(&pc.provider),
                lookup_status: effective_lookup_status(&customer),
            }),
            _ => None,
        };
        let context = PendingContext::Invoice {
            invoice_no: invoice_no.clone(),
            invoice_seq,
            sale_event_ids: ids.clone(),
            customer_id: customer.id,
            customer_json: serde_json::to_string(&party).map_err(errors::internal)?,
            issue_date: issue_date.clone(),
            total_cents: doc.total_cents(),
            vat_json: doc.vat_json(),
            print: Box::new(print),
            peppol,
        };
        Prepared {
            op,
            request,
            label: EventLabel::I,
            first_ticket_no: ticket_number,
            events: vec![ArchiveSpec {
                order_id: None,
                user_insz: employee_insz.clone(),
                request_json,
                sent_entries: None,
            }],
            order_id: None,
            user_insz: employee_insz.clone(),
            context,
        }
    };
    let context = prepared.context.clone();
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(InvoiceResult::failed(failure.error)),
    };
    drop(guard);

    // Print (and wake the Peppol queue) with no lock held.
    let done = signing::follow_up(state, &config, &context, &committed, false).await;
    let invoice_no = match &context {
        PendingContext::Invoice { invoice_no, .. } => Some(invoice_no.clone()),
        _ => None,
    };
    Ok(InvoiceResult {
        success: true,
        fiscal_event_id: Some(committed.event_id()),
        invoice_no,
        printed: done.invoice_printed.unwrap_or(false),
        delivery_status: delivery_status_of(state, committed.event_id()),
        error: None,
    })
}

fn delivery_status_of(state: &AppState, event_id: i64) -> Option<String> {
    let db = state.db.lock().ok()?;
    q::invoice_of_event(&db, event_id)
        .ok()
        .flatten()
        .and_then(|i| i.delivery_status)
}

/// The invoice rows and the delivery row, in the archive transaction of the I event.
pub(crate) fn record_in_tx(
    tx: &rusqlite::Connection,
    event_id: i64,
    created_by: &str,
    context: &PendingContext,
) -> Result<(), String> {
    let PendingContext::Invoice {
        invoice_no,
        invoice_seq,
        sale_event_ids,
        customer_id,
        customer_json,
        issue_date,
        total_cents,
        vat_json,
        peppol,
        ..
    } = context
    else {
        return Ok(());
    };
    q::insert_invoice(
        tx,
        &q::NewInvoice {
            invoice_seq: *invoice_seq,
            invoice_no,
            fiscal_event_id: event_id,
            customer_id: Some(*customer_id),
            customer_json,
            issue_date,
            total_cents: *total_cents,
            vat_json,
            created_by,
            sale_event_ids,
        },
    )
    .map_err(errors::internal)?;
    if let Some(p) = peppol {
        q::insert_delivery(
            tx,
            &q::NewDelivery {
                fiscal_event_id: event_id,
                invoice_no,
                customer_endpoint_id: &p.participant,
                ap_vendor: &p.ap_vendor,
                ubl_xml: &p.ubl_xml,
                lookup_status: &p.lookup_status,
                now: &now_rfc3339(),
            },
        )
        .map_err(errors::internal)?;
    }
    Ok(())
}

/// Print an invoice (or its copy) on the ticket printer. `false` without a printer or when
/// printing fails: the invoice stands, a copy reprints it.
pub(crate) async fn print_invoice(
    state: &AppState,
    config: &PosConfig,
    print: &InvoicePrint,
) -> bool {
    let Some(addr) = config.printer_address.as_deref() else {
        return false;
    };
    let bytes = pos_print::invoice::format_invoice(print);
    match pos_print::transport::send_to_printer_with(addr, &bytes, state.timers.printer_status_read)
        .await
    {
        Ok(status) => {
            if !status.paper_ok {
                tracing::warn!(?status.warnings, "printer: paper issue after invoice print");
            }
            true
        }
        Err(e) => {
            tracing::warn!(%e, "invoice print failed (non-fatal)");
            false
        }
    }
}

// ── Rebuilding an invoice from the archive ────────────────────────

/// The invoice whose I event is `event`, rebuilt from its rows and the archive of its sales:
/// the print of the original (what a copy reproduces).
pub(crate) fn print_of_event(
    state: &AppState,
    config: &PosConfig,
    event: &FiscalEventRef,
) -> Result<InvoicePrint, String> {
    let (invoice, doc) = {
        let db = state.db.lock().map_err(errors::internal)?;
        let invoice = q::invoice_of_event(&db, event.id)
            .map_err(errors::internal)?
            .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?;
        let mut sales = Vec::new();
        for id in &invoice.sale_event_ids {
            sales.push(load_sale(&db, *id)?);
        }
        (invoice, InvoiceDoc { sales })
    };
    let customer: InvoiceParty =
        serde_json::from_str(&invoice.customer_json).map_err(errors::internal)?;
    let control = event
        .fdm_response_json
        .as_deref()
        .and_then(|j| serde_json::from_str::<SignResult>(j).ok())
        .map(|r| control_of(&r));
    let operator = super::operator_name(state, &event.user_insz);
    let facts = InvoiceFacts {
        invoice_no: &invoice.invoice_no,
        issue_date: &invoice.issue_date,
        customer: &customer,
        doc: &doc,
    };
    Ok(build_print(state, config, &operator, &facts, control))
}

/// Build the UBL document of an invoice signed without a delivery row (Peppol set up after
/// the invoice, or a delivery to redo) and queue it. `Ok(None)` for a customer who cannot be
/// delivered over Peppol (not Belgian); an existing delivery is returned as it is.
pub fn do_issue_b2b_invoice(state: &AppState, fiscal_event_id: i64) -> Result<Option<i64>, String> {
    access::operator(state, Access::Admin)?;
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let peppol = config
        .peppol
        .clone()
        .ok_or_else(|| errors::PEPPOL_NOT_CONFIGURED.to_string())?;
    let db = state.db.lock().map_err(errors::internal)?;
    let invoice = q::invoice_of_event(&db, fiscal_event_id)
        .map_err(errors::internal)?
        .ok_or_else(|| errors::EVENT_NOT_FOUND.to_string())?;
    if let Some(id) = invoice.delivery_id {
        return Ok(Some(id));
    }
    let customer: InvoiceParty =
        serde_json::from_str(&invoice.customer_json).map_err(errors::internal)?;
    let Some(participant) = parse_vat_number(&customer.vat_number)
        .map_err(|_| errors::CUSTOMER_VAT_INVALID.to_string())?
        .participant_id()
    else {
        return Ok(None);
    };
    let mut sales = Vec::new();
    for id in &invoice.sale_event_ids {
        sales.push(load_sale(&db, *id)?);
    }
    let doc = InvoiceDoc { sales };
    let facts = InvoiceFacts {
        invoice_no: &invoice.invoice_no,
        issue_date: &invoice.issue_date,
        customer: &customer,
        doc: &doc,
    };
    let ubl_xml = build_ubl(&config, &peppol, &facts)?;
    let id = q::insert_delivery(
        &db,
        &q::NewDelivery {
            fiscal_event_id,
            invoice_no: &invoice.invoice_no,
            customer_endpoint_id: &participant,
            ap_vendor: &vendor_of(&peppol.provider),
            ubl_xml: &ubl_xml,
            lookup_status: "unknown",
            now: &now_rfc3339(),
        },
    )
    .map_err(errors::internal)?;
    drop(db);
    state.peppol_wake.notify_one();
    Ok(Some(id))
}

// ── Lists and the Peppol panel ────────────────────────────────────

pub fn do_list_invoices(
    state: &AppState,
    limit: Option<i64>,
) -> Result<Vec<InvoiceSummary>, String> {
    access::operator(state, Access::Cashier)?;
    let db = state.db.lock().map_err(errors::internal)?;
    let rows = q::list_invoices(&db, limit.unwrap_or(DEFAULT_LIMIT).clamp(1, 200))
        .map_err(errors::internal)?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let customer: serde_json::Value =
                serde_json::from_str(&r.customer_json).unwrap_or_default();
            InvoiceSummary {
                id: r.id,
                invoice_no: r.invoice_no,
                fiscal_event_id: r.fiscal_event_id,
                issue_date: r.issue_date,
                customer_name: customer["name"].as_str().unwrap_or_default().to_string(),
                customer_vat_number: customer["vat_number"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                total_cents: r.total_cents,
                sale_event_ids: r.sale_event_ids,
                delivery_status: r.delivery_status,
                delivery_id: r.delivery_id,
            }
        })
        .collect())
}

/// The Peppol panel: setup, queue counts and the newest deliveries (admins).
pub fn do_list_peppol_deliveries(
    state: &AppState,
    limit: Option<i64>,
) -> Result<PeppolDeliveries, String> {
    access::operator(state, Access::Admin)?;
    let config = state.config.lock().map_err(errors::internal)?.clone();
    let db = state.db.lock().map_err(errors::internal)?;
    let (queued, accepted, failed) = q::queue_counts(&db).map_err(errors::internal)?;
    Ok(PeppolDeliveries {
        configured: config.peppol.is_some(),
        provider: config.peppol.as_ref().map(|p| p.provider.clone()),
        sender_id: config.peppol.as_ref().map(|p| p.sender_id.clone()),
        queued,
        accepted,
        failed,
        deliveries: q::list_deliveries(&db, limit.unwrap_or(50).clamp(1, 200))
            .map_err(errors::internal)?,
    })
}

/// Put a parked delivery back in the queue (admins). The recipient is looked up again.
pub fn do_retry_peppol_delivery(state: &AppState, delivery_id: i64) -> Result<(), String> {
    access::operator(state, Access::Admin)?;
    if state
        .config
        .lock()
        .map_err(errors::internal)?
        .peppol
        .is_none()
    {
        return Err(errors::PEPPOL_NOT_CONFIGURED.to_string());
    }
    {
        let db = state.db.lock().map_err(errors::internal)?;
        let row = q::get_delivery(&db, delivery_id)
            .map_err(errors::internal)?
            .ok_or_else(|| errors::PEPPOL_DELIVERY_NOT_FOUND.to_string())?;
        if row.status != "failed" {
            return Err(errors::PEPPOL_DELIVERY_NOT_PARKED.to_string());
        }
        q::requeue(&db, delivery_id, &now_rfc3339()).map_err(errors::internal)?;
        q::set_delivery_lookup(&db, delivery_id, "unknown").map_err(errors::internal)?;
    }
    state.peppol_wake.notify_one();
    Ok(())
}

#[cfg(test)]
mod tests {
    //! The reading of an archived sale (V11-D3) on hand-made archives: cases the till cannot
    //! produce (weighed goods, a product under two VAT labels, an FDM answer without `vatCalc`).

    use serde_json::{Value, json};

    use super::*;

    fn event(request: &Value, response: Option<Value>) -> FiscalEventRef {
        FiscalEventRef {
            id: 11,
            event_type: "N".into(),
            sequence_number: 1,
            order_id: Some(1),
            fdm_id: Some("FDM001".into()),
            fdm_datetime: Some("20261017T120000".into()),
            event_label: Some("N".into()),
            event_counter: Some(5),
            total_counter: Some(9),
            short_signature: None,
            qr_data: None,
            fdm_response_json: response.map(|r| r.to_string()),
            user_insz: "75061189702".into(),
            event_data_json: request.to_string(),
        }
    }

    fn request(lines: Value, total: f64) -> Value {
        json!({
            "posFiscalTicketNo": 7,
            "posDateTime": "2026-10-17T12:00:00Z",
            "financials": [{"type": "CASH", "name": "Cash", "amount": total}],
            "transaction": {"transactionTotal": total, "transactionLines": lines},
        })
    }

    fn product(name: &str, quantity: f64, unit: &str, vats: Value) -> Value {
        json!({"mainProduct": {
            "productName": name, "quantity": quantity, "quantityType": unit, "vats": vats
        }})
    }

    #[test]
    fn the_vat_falls_back_to_the_lines_when_the_fdm_sent_no_vat_calc() {
        let req = request(
            json!([
                product("Plat", 1.0, "PIECE", json!([{"label": "B", "price": 10.0}])),
                product(
                    "Biere",
                    1.0,
                    "PIECE",
                    json!([{"label": "A", "price": 12.1}])
                ),
                product("Eau", 1.0, "PIECE", json!([{"label": "D", "price": 2.0}])),
            ]),
            24.1,
        );
        for response in [None, Some(json!({})), Some(json!({"vatCalc": []}))] {
            let sale = read_sale(&event(&req, response)).unwrap();
            let by = |l: &str| sale.vat.iter().find(|v| v.label == l).unwrap().clone();
            // 10.00 at 12 %: base = round(1000 / 1.12) = 893, VAT 107.
            let b = by("B");
            assert_eq!(
                (b.rate_hundredths, b.base_cents, b.vat_cents, b.total_cents),
                (1200, 893, 107, 1000)
            );
            let a = by("A");
            assert_eq!(
                (a.rate_hundredths, a.base_cents, a.vat_cents, a.total_cents),
                (2100, 1000, 210, 1210)
            );
            let d = by("D");
            assert_eq!((d.base_cents, d.vat_cents, d.total_cents), (200, 0, 200));
            assert_eq!(sale.vat.len(), 3);
            assert_eq!(sale.total_cents, 2410);
        }
    }

    #[test]
    fn a_refund_falls_back_with_the_same_rounding_on_the_negative_side() {
        let req = request(
            json!([product(
                "Plat",
                -1.0,
                "PIECE",
                json!([{"label": "B", "price": -10.0}])
            )]),
            -10.0,
        );
        let sale = read_sale(&event(&req, None)).unwrap();
        let b = &sale.vat[0];
        assert_eq!(
            (b.base_cents, b.vat_cents, b.total_cents),
            (-893, -107, -1000)
        );
        assert!(sale.is_refund());
    }

    #[test]
    fn the_fdm_vat_calc_wins_over_the_lines() {
        let req = request(
            json!([product(
                "Plat",
                1.0,
                "PIECE",
                json!([{"label": "B", "price": 10.0}])
            )]),
            10.0,
        );
        // Deliberately not what the lines would give: the FDM's figures are the ones kept.
        let response = json!({"vatCalc": [
            {"label": "B", "rate": 12.0, "taxableAmount": 8.9, "vatAmount": 1.1, "totalAmount": 10.0}
        ]});
        let sale = read_sale(&event(&req, Some(response))).unwrap();
        let b = &sale.vat[0];
        assert_eq!(
            (b.rate_hundredths, b.base_cents, b.vat_cents, b.total_cents),
            (1200, 890, 110, 1000)
        );
    }

    #[test]
    fn quantities_and_unit_codes_come_from_the_archive() {
        let req = request(
            json!([
                product(
                    "Riz",
                    0.35,
                    "KILOGRAM",
                    json!([{"label": "C", "price": 3.5}])
                ),
                product("Huile", 1.5, "LITRE", json!([{"label": "C", "price": 6.0}])),
                product("Ruban", 2.0, "METER", json!([{"label": "A", "price": 4.0}])),
                product(
                    "Conseil",
                    0.5,
                    "HOUR",
                    json!([{"label": "A", "price": 20.0}])
                ),
                product("Pain", 3.0, "PIECE", json!([{"label": "C", "price": 6.0}])),
            ]),
            39.5,
        );
        let sale = read_sale(&event(&req, None)).unwrap();
        let got: Vec<(&str, i64, &str)> = sale
            .lines
            .iter()
            .map(|l| (l.name.as_str(), l.quantity_milli, l.unit_code))
            .collect();
        assert_eq!(
            got,
            [
                ("Riz", 350, "KGM"),
                ("Huile", 1500, "LTR"),
                ("Ruban", 2000, "MTR"),
                ("Conseil", 500, "HUR"),
                ("Pain", 3000, "H87"),
            ]
        );
    }

    #[test]
    fn a_product_under_two_vat_labels_is_one_row_per_label() {
        let req = request(
            json!([product(
                "Menu",
                2.0,
                "PIECE",
                json!([{"label": "A", "price": 5.0}, {"label": "B", "price": 7.0}])
            )]),
            12.0,
        );
        let sale = read_sale(&event(&req, None)).unwrap();
        let got: Vec<(&str, i64, &str, i64, &str)> = sale
            .lines
            .iter()
            .map(|l| {
                (
                    l.name.as_str(),
                    l.quantity_milli,
                    l.unit_code,
                    l.incl_cents,
                    l.vat_label.as_str(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("Menu (A)", 1000, "H87", 500, "A"),
                ("Menu (B)", 1000, "H87", 700, "B"),
            ]
        );
    }

    #[test]
    fn line_changes_go_into_the_line_and_event_changes_become_adjustments() {
        let req = request(
            json!([
                product(
                    "Plat",
                    1.0,
                    "PIECE",
                    json!([{"label": "B", "price": 10.0, "priceChanges": [
                        {"id": "L", "name": "Promo ligne", "scope": "LINE", "amount": -1.0},
                        {"id": "E", "name": "Remise", "scope": "EVENT", "amount": -0.5}
                    ]}])
                ),
                product(
                    "Biere",
                    1.0,
                    "PIECE",
                    json!([{"label": "A", "price": 4.0, "priceChanges": [
                        {"id": "E", "name": "Remise", "scope": "EVENT", "amount": -0.25}
                    ]}])
                ),
            ]),
            12.25,
        );
        let sale = read_sale(&event(&req, None)).unwrap();
        // The LINE change is part of the line; the EVENT ones are separate rows per label.
        assert_eq!(sale.lines[0].incl_cents, 900);
        assert_eq!(sale.lines[1].incl_cents, 400);
        let adj: Vec<(&str, &str, i64)> = sale
            .adjustments
            .iter()
            .map(|a| (a.reason.as_str(), a.vat_label.as_str(), a.incl_cents))
            .collect();
        assert_eq!(adj, [("Remise", "A", -25), ("Remise", "B", -50)]);
        assert_eq!(sale.total_cents, 900 + 400 - 25 - 50);
    }
}
