//! UBL 2.1 Invoice serializer, Peppol BIS Billing 3.0 (V11-D11).
//!
//! The document is built from VAT-inclusive amounts in **i64 eurocents** (no f64 money
//! anywhere) and written by hand in the element order of the UBL 2.1 schema.
//!
//! ## How the sums are made
//!
//! - **Line net amount** = round(VAT-inclusive line amount / (1 + rate)), half away from
//!   zero. A refunded line keeps its negative quantity and net amount; its unit price stays
//!   positive (BR-27).
//! - **One `TaxSubtotal` per VAT label**: taxable amount = the sum of its line net amounts,
//!   minus its allowances, plus its charges (BR-S-08); tax amount = round(taxable x rate)
//!   (BR-CO-17).
//! - **Event-level price changes** (a discount over the whole ticket) become document
//!   `AllowanceCharge`s that carry their tax category. Line-level changes are already in the
//!   line amount.
//! - Totals: line extension = sum of line nets (BR-CO-10); tax exclusive = line extension
//!   minus allowances plus charges (BR-CO-13); tax inclusive = tax exclusive plus the tax
//!   total (BR-CO-15); **`PrepaidAmount`** = the VAT-inclusive total of the tickets the
//!   invoice covers; **`PayableRoundingAmount`** absorbs the cent differences so that
//!   `PayableAmount` is `0.00` (BR-CO-16): the sales were paid at the till.
//! - `BuyerReference` is the invoice number, there is no `DueDate`.
//!
//! ## VAT labels
//!
//! A, B, C are category `S` at 21, 12 and 6 percent, D is `Z`, X is `E` with the exemption
//! reason code `vatex-eu-o` (not subject to VAT), BR-E-10 asking for a reason.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use chrono::{DateTime, Duration, NaiveDate};

use crate::errors::PeppolError;

/// The Peppol BIS Billing 3.0 customization and profile ids.
pub const CUSTOMIZATION_ID: &str =
    "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0";
pub const PROFILE_ID: &str = "urn:fdc:peppol.eu:2017:poacc:billing:01:1.0";

/// A seller or a buyer, as the caller knows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    /// Peppol scheme of the endpoint, `0208` for a Belgian enterprise number.
    pub endpoint_scheme: String,
    /// The endpoint value: the 10-digit enterprise number for scheme `0208`.
    pub endpoint_value: String,
    pub name: String,
    pub address: Address,
    /// The VAT number with its country prefix, e.g. `BE0308357159`.
    pub vat_no: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub street: String,
    pub postal_code: String,
    pub city: String,
    /// ISO 3166-1 alpha-2.
    pub country_code: String,
}

/// One product line of a covered ticket, VAT inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLine {
    pub name: String,
    /// Quantity in thousandths (2 pieces = 2000, 0.350 kg = 350). Negative on a refund.
    pub quantity_milli: i64,
    /// UN/ECE unit code: `H87` piece, `KGM` kilogram, `MTR` metre, `LTR` litre, `HUR` hour.
    pub unit_code: String,
    /// The line amount after its own price changes, VAT inclusive, in cents. Negative on a
    /// refund.
    pub incl_cents: i64,
    /// SCE 2.0 VAT label A, B, C, D or X.
    pub vat_label: String,
}

/// A price change of the whole ticket. Negative = discount (an allowance), positive = a
/// charge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAdjustment {
    pub reason: String,
    pub incl_cents: i64,
    pub vat_label: String,
}

/// Everything the UBL needs, in cents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceSource {
    pub invoice_no: String,
    /// `YYYY-MM-DD`.
    pub issue_date: String,
    pub supplier: Party,
    pub customer: Party,
    pub lines: Vec<SourceLine>,
    pub adjustments: Vec<SourceAdjustment>,
    /// The VAT-inclusive total of the covered tickets (product lines and price changes,
    /// payment rounding excluded).
    pub prepaid_cents: i64,
    pub note: Option<String>,
}

/// A UBL invoice line, computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineItem {
    pub id: String,
    pub name: String,
    pub quantity_milli: i64,
    pub unit_code: String,
    pub net_cents: i64,
    /// Unit price in 1/10000 euro, never negative.
    pub price_micro: i64,
    pub tax: TaxCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxCategory {
    /// BIS category code: `S`, `Z` or `E`.
    pub id: &'static str,
    /// Rate in percent.
    pub percent: i64,
    /// The label it came from.
    pub label: String,
}

/// A document-level allowance or charge, computed (amount is positive, the flag says which).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowanceCharge {
    pub is_charge: bool,
    pub reason: String,
    pub net_cents: i64,
    pub tax: TaxCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxSubtotal {
    pub tax: TaxCategory,
    pub taxable_cents: i64,
    pub tax_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonetaryTotal {
    pub line_extension_cents: i64,
    pub tax_exclusive_cents: i64,
    pub tax_inclusive_cents: i64,
    pub allowance_total_cents: i64,
    pub charge_total_cents: i64,
    pub prepaid_cents: i64,
    pub payable_rounding_cents: i64,
    pub payable_cents: i64,
}

/// A computed UBL invoice, ready for [`to_ubl_xml`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceUbl {
    pub id: String,
    pub issue_date: String,
    /// `380` = commercial invoice.
    pub invoice_type_code: String,
    pub document_currency_code: String,
    pub buyer_reference: String,
    pub note: Option<String>,
    pub supplier: Party,
    pub customer: Party,
    pub allowance_charges: Vec<AllowanceCharge>,
    pub tax_subtotals: Vec<TaxSubtotal>,
    pub tax_total_cents: i64,
    pub totals: MonetaryTotal,
    pub lines: Vec<LineItem>,
}

/// Map an SCE-2.0 VAT label letter (A/B/C/D/X) to a BIS Billing 3.0
/// tax category code + numeric rate string. A=21%, B=12%, C=6%, D=0% (zero rated),
/// X=exempt (not subject to VAT).
pub fn vat_label_to_bis(label: &str) -> Result<(&'static str, &'static str), PeppolError> {
    match label {
        "A" => Ok(("S", "21.00")),
        "B" => Ok(("S", "12.00")),
        "C" => Ok(("S", "6.00")),
        "D" => Ok(("Z", "0.00")),
        "X" => Ok(("E", "0.00")),
        other => Err(PeppolError::Xml(format!("unknown VAT label {other}"))),
    }
}

fn tax_category(label: &str) -> Result<TaxCategory, PeppolError> {
    let (id, percent) = vat_label_to_bis(label)?;
    let percent: i64 = percent
        .split('.')
        .next()
        .and_then(|p| p.parse().ok())
        .ok_or_else(|| PeppolError::Xml(format!("bad rate for label {label}")))?;
    Ok(TaxCategory {
        id,
        percent,
        label: label.to_string(),
    })
}

/// `num / den` rounded half away from zero (`den > 0`).
fn round_div(num: i64, den: i64) -> i64 {
    debug_assert!(den > 0);
    let half = den / 2;
    if num >= 0 {
        (num + half) / den
    } else {
        -((-num + half) / den)
    }
}

/// VAT-inclusive cents to net cents at `percent`.
fn net_of(incl_cents: i64, percent: i64) -> i64 {
    round_div(incl_cents * 100, 100 + percent)
}

impl InvoiceUbl {
    /// Compute the document from its source. Fails on an unknown VAT label, a line without
    /// quantity or an empty invoice.
    pub fn build(source: &InvoiceSource) -> Result<Self, PeppolError> {
        if source.lines.is_empty() {
            return Err(PeppolError::MissingField("invoice lines"));
        }
        let mut lines = Vec::with_capacity(source.lines.len());
        let mut taxable: BTreeMap<String, (TaxCategory, i64)> = BTreeMap::new();
        for (i, l) in source.lines.iter().enumerate() {
            if l.quantity_milli == 0 {
                return Err(PeppolError::MissingField("line quantity"));
            }
            let tax = tax_category(&l.vat_label)?;
            let net = net_of(l.incl_cents, tax.percent);
            // price = |net| / |quantity|, in 1/10000 euro: net_cents * 100 * 1000 / qty_milli
            let price_micro = round_div(net.abs() * 100_000, l.quantity_milli.abs());
            taxable
                .entry(l.vat_label.clone())
                .or_insert((tax.clone(), 0))
                .1 += net;
            lines.push(LineItem {
                id: (i + 1).to_string(),
                name: l.name.clone(),
                quantity_milli: l.quantity_milli,
                unit_code: l.unit_code.clone(),
                net_cents: net,
                price_micro,
                tax,
            });
        }
        let mut allowance_charges = Vec::new();
        let (mut allowance_total, mut charge_total) = (0, 0);
        for a in &source.adjustments {
            if a.incl_cents == 0 {
                continue;
            }
            let tax = tax_category(&a.vat_label)?;
            let net = net_of(a.incl_cents, tax.percent);
            let is_charge = a.incl_cents > 0;
            if is_charge {
                charge_total += net;
            } else {
                allowance_total += -net;
            }
            // The taxable amount of the category moves by the signed net amount.
            taxable
                .entry(a.vat_label.clone())
                .or_insert((tax.clone(), 0))
                .1 += net;
            allowance_charges.push(AllowanceCharge {
                is_charge,
                reason: a.reason.clone(),
                net_cents: net.abs(),
                tax,
            });
        }
        let tax_subtotals: Vec<TaxSubtotal> = taxable
            .into_values()
            .map(|(tax, taxable_cents)| TaxSubtotal {
                tax_cents: round_div(taxable_cents * tax.percent, 100),
                tax,
                taxable_cents,
            })
            .collect();
        let line_extension: i64 = lines.iter().map(|l| l.net_cents).sum();
        let tax_exclusive = line_extension - allowance_total + charge_total;
        let tax_total: i64 = tax_subtotals.iter().map(|t| t.tax_cents).sum();
        let tax_inclusive = tax_exclusive + tax_total;
        let rounding = source.prepaid_cents - tax_inclusive;
        let totals = MonetaryTotal {
            line_extension_cents: line_extension,
            tax_exclusive_cents: tax_exclusive,
            tax_inclusive_cents: tax_inclusive,
            allowance_total_cents: allowance_total,
            charge_total_cents: charge_total,
            prepaid_cents: source.prepaid_cents,
            payable_rounding_cents: rounding,
            // BR-CO-16: payable = tax inclusive - prepaid + rounding
            payable_cents: tax_inclusive - source.prepaid_cents + rounding,
        };
        Ok(Self {
            id: source.invoice_no.clone(),
            issue_date: source.issue_date.clone(),
            invoice_type_code: "380".to_string(),
            document_currency_code: "EUR".to_string(),
            buyer_reference: source.invoice_no.clone(),
            note: source.note.clone(),
            supplier: source.supplier.clone(),
            customer: source.customer.clone(),
            allowance_charges,
            tax_subtotals,
            tax_total_cents: tax_total,
            totals,
            lines,
        })
    }
}

// ── XML ────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// `1234` -> `12.34`, `-5` -> `-0.05`.
pub fn money(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", cents.abs() / 100, cents.abs() % 100)
}

fn quantity(milli: i64) -> String {
    let sign = if milli < 0 { "-" } else { "" };
    format!("{sign}{}.{:03}", milli.abs() / 1000, milli.abs() % 1000)
}

fn price(micro: i64) -> String {
    format!("{}.{:04}", micro / 10_000, micro % 10_000)
}

struct Xml {
    out: String,
    depth: usize,
}

impl Xml {
    fn pad(&mut self) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }

    fn open(&mut self, name: &str) {
        self.pad();
        let _ = writeln!(self.out, "<{name}>");
        self.depth += 1;
    }

    fn close(&mut self, name: &str) {
        self.depth -= 1;
        self.pad();
        let _ = writeln!(self.out, "</{name}>");
    }

    fn text(&mut self, name: &str, value: &str) {
        self.pad();
        let _ = writeln!(self.out, "<{name}>{}</{name}>", esc(value));
    }

    fn attr_text(&mut self, name: &str, attr: &str, attr_value: &str, value: &str) {
        self.pad();
        let _ = writeln!(
            self.out,
            "<{name} {attr}=\"{}\">{}</{name}>",
            esc(attr_value),
            esc(value)
        );
    }

    fn amount(&mut self, name: &str, cents: i64) {
        self.attr_text(name, "currencyID", "EUR", &money(cents));
    }
}

fn write_party(x: &mut Xml, wrapper: &str, p: &Party) {
    x.open(wrapper);
    x.open("cac:Party");
    x.attr_text(
        "cbc:EndpointID",
        "schemeID",
        &p.endpoint_scheme,
        &p.endpoint_value,
    );
    x.open("cac:PartyName");
    x.text("cbc:Name", &p.name);
    x.close("cac:PartyName");
    x.open("cac:PostalAddress");
    x.text("cbc:StreetName", &p.address.street);
    x.text("cbc:CityName", &p.address.city);
    x.text("cbc:PostalZone", &p.address.postal_code);
    x.open("cac:Country");
    x.text("cbc:IdentificationCode", &p.address.country_code);
    x.close("cac:Country");
    x.close("cac:PostalAddress");
    x.open("cac:PartyTaxScheme");
    x.text("cbc:CompanyID", &p.vat_no);
    x.open("cac:TaxScheme");
    x.text("cbc:ID", "VAT");
    x.close("cac:TaxScheme");
    x.close("cac:PartyTaxScheme");
    x.open("cac:PartyLegalEntity");
    x.text("cbc:RegistrationName", &p.name);
    x.close("cac:PartyLegalEntity");
    x.close("cac:Party");
    x.close(wrapper);
}

/// The `TaxCategory` block (document level: with exemption reason when `E`).
fn write_tax_category(x: &mut Xml, name: &str, t: &TaxCategory, with_reason: bool) {
    x.open(name);
    x.text("cbc:ID", t.id);
    x.text("cbc:Percent", &format!("{}.00", t.percent));
    if with_reason && t.id == "E" {
        x.text("cbc:TaxExemptionReasonCode", "vatex-eu-o");
        x.text("cbc:TaxExemptionReason", "Not subject to VAT");
    }
    x.open("cac:TaxScheme");
    x.text("cbc:ID", "VAT");
    x.close("cac:TaxScheme");
    x.close(name);
}

/// Render an [`InvoiceUbl`] to the UBL 2.1 Invoice document (Peppol BIS Billing 3.0).
pub fn to_ubl_xml(invoice: &InvoiceUbl) -> Result<String, PeppolError> {
    if invoice.id.trim().is_empty() {
        return Err(PeppolError::MissingField("invoice number"));
    }
    let mut x = Xml {
        out: String::new(),
        depth: 1,
    };
    x.out
        .push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.out.push_str(
        "<Invoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\"\n         \
         xmlns:cac=\"urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2\"\n         \
         xmlns:cbc=\"urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2\">\n",
    );
    x.text("cbc:CustomizationID", CUSTOMIZATION_ID);
    x.text("cbc:ProfileID", PROFILE_ID);
    x.text("cbc:ID", &invoice.id);
    x.text("cbc:IssueDate", &invoice.issue_date);
    x.text("cbc:InvoiceTypeCode", &invoice.invoice_type_code);
    if let Some(note) = &invoice.note {
        x.text("cbc:Note", note);
    }
    x.text("cbc:DocumentCurrencyCode", &invoice.document_currency_code);
    x.text("cbc:BuyerReference", &invoice.buyer_reference);
    write_party(&mut x, "cac:AccountingSupplierParty", &invoice.supplier);
    write_party(&mut x, "cac:AccountingCustomerParty", &invoice.customer);
    for a in &invoice.allowance_charges {
        x.open("cac:AllowanceCharge");
        x.text(
            "cbc:ChargeIndicator",
            if a.is_charge { "true" } else { "false" },
        );
        x.text("cbc:AllowanceChargeReason", &a.reason);
        x.amount("cbc:Amount", a.net_cents);
        write_tax_category(&mut x, "cac:TaxCategory", &a.tax, false);
        x.close("cac:AllowanceCharge");
    }
    x.open("cac:TaxTotal");
    x.amount("cbc:TaxAmount", invoice.tax_total_cents);
    for s in &invoice.tax_subtotals {
        x.open("cac:TaxSubtotal");
        x.amount("cbc:TaxableAmount", s.taxable_cents);
        x.amount("cbc:TaxAmount", s.tax_cents);
        write_tax_category(&mut x, "cac:TaxCategory", &s.tax, true);
        x.close("cac:TaxSubtotal");
    }
    x.close("cac:TaxTotal");
    let t = &invoice.totals;
    x.open("cac:LegalMonetaryTotal");
    x.amount("cbc:LineExtensionAmount", t.line_extension_cents);
    x.amount("cbc:TaxExclusiveAmount", t.tax_exclusive_cents);
    x.amount("cbc:TaxInclusiveAmount", t.tax_inclusive_cents);
    if t.allowance_total_cents != 0 {
        x.amount("cbc:AllowanceTotalAmount", t.allowance_total_cents);
    }
    if t.charge_total_cents != 0 {
        x.amount("cbc:ChargeTotalAmount", t.charge_total_cents);
    }
    x.amount("cbc:PrepaidAmount", t.prepaid_cents);
    if t.payable_rounding_cents != 0 {
        x.amount("cbc:PayableRoundingAmount", t.payable_rounding_cents);
    }
    x.amount("cbc:PayableAmount", t.payable_cents);
    x.close("cac:LegalMonetaryTotal");
    for l in &invoice.lines {
        x.open("cac:InvoiceLine");
        x.text("cbc:ID", &l.id);
        x.attr_text(
            "cbc:InvoicedQuantity",
            "unitCode",
            &l.unit_code,
            &quantity(l.quantity_milli),
        );
        x.amount("cbc:LineExtensionAmount", l.net_cents);
        x.open("cac:Item");
        x.text("cbc:Name", &l.name);
        write_tax_category(&mut x, "cac:ClassifiedTaxCategory", &l.tax, false);
        x.close("cac:Item");
        x.open("cac:Price");
        x.attr_text(
            "cbc:PriceAmount",
            "currencyID",
            "EUR",
            &price(l.price_micro),
        );
        x.close("cac:Price");
        x.close("cac:InvoiceLine");
    }
    x.out.push_str("</Invoice>\n");
    Ok(x.out)
}

/// Extract the date-only portion of an RFC3339 `pos_date_time`.
pub fn issue_date_from_pos_dt(pos_dt: &str) -> Result<String, PeppolError> {
    let dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(pos_dt)
        .map_err(|e| PeppolError::Xml(format!("bad pos_date_time {pos_dt}: {e}")))?;
    Ok(dt.format("%Y-%m-%d").to_string())
}

/// `issue_date` plus `days`, for callers that print a due date.
pub fn date_plus_days(issue_date: &str, days: i64) -> Result<String, PeppolError> {
    let d = NaiveDate::parse_from_str(issue_date, "%Y-%m-%d")
        .map_err(|e| PeppolError::Xml(format!("bad issue_date {issue_date}: {e}")))?;
    Ok((d + Duration::days(days)).format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(name: &str, vat: &str, value: &str) -> Party {
        Party {
            endpoint_scheme: "0208".into(),
            endpoint_value: value.into(),
            name: name.into(),
            address: Address {
                street: "Rue Test 1".into(),
                postal_code: "4000".into(),
                city: "Liège".into(),
                country_code: "BE".into(),
            },
            vat_no: vat.into(),
        }
    }

    fn source(
        lines: Vec<SourceLine>,
        adjustments: Vec<SourceAdjustment>,
        prepaid: i64,
    ) -> InvoiceSource {
        InvoiceSource {
            invoice_no: "20260001".into(),
            issue_date: "2026-10-17".into(),
            supplier: party("Restaurant Exemple", "BE0499999960", "0499999960"),
            customer: party("Acme & Fils <SA>", "BE0308357159", "0308357159"),
            lines,
            adjustments,
            prepaid_cents: prepaid,
            note: Some("Déjà payé".into()),
        }
    }

    fn line(name: &str, qty_milli: i64, incl: i64, label: &str) -> SourceLine {
        SourceLine {
            name: name.into(),
            quantity_milli: qty_milli,
            unit_code: "H87".into(),
            incl_cents: incl,
            vat_label: label.into(),
        }
    }

    /// The BIS rules that can be checked on the numbers: BR-CO-10, 13, 15, 16, 17 and BR-S-08.
    fn assert_bis_sums(u: &InvoiceUbl) {
        let t = &u.totals;
        // BR-CO-10
        assert_eq!(
            t.line_extension_cents,
            u.lines.iter().map(|l| l.net_cents).sum::<i64>()
        );
        // BR-CO-13
        assert_eq!(
            t.tax_exclusive_cents,
            t.line_extension_cents - t.allowance_total_cents + t.charge_total_cents
        );
        // BR-CO-14: the tax total is the sum of the subtotals
        assert_eq!(
            u.tax_total_cents,
            u.tax_subtotals.iter().map(|s| s.tax_cents).sum::<i64>()
        );
        // BR-CO-15
        assert_eq!(
            t.tax_inclusive_cents,
            t.tax_exclusive_cents + u.tax_total_cents
        );
        // BR-CO-16
        assert_eq!(
            t.payable_cents,
            t.tax_inclusive_cents - t.prepaid_cents + t.payable_rounding_cents
        );
        for s in &u.tax_subtotals {
            // BR-CO-17
            let exact = s.taxable_cents * s.tax.percent;
            assert_eq!(s.tax_cents, round_div(exact, 100), "{:?}", s.tax);
            // BR-S-08: taxable = lines + charges - allowances of the category
            let lines: i64 = u
                .lines
                .iter()
                .filter(|l| l.tax.label == s.tax.label)
                .map(|l| l.net_cents)
                .sum();
            let adj: i64 = u
                .allowance_charges
                .iter()
                .filter(|a| a.tax.label == s.tax.label)
                .map(|a| {
                    if a.is_charge {
                        a.net_cents
                    } else {
                        -a.net_cents
                    }
                })
                .sum();
            assert_eq!(s.taxable_cents, lines + adj, "BR-S-08 {:?}", s.tax);
        }
    }

    #[test]
    fn a_two_label_invoice_has_one_subtotal_per_label_and_pays_to_zero() {
        // 2 x 12.10 at 21 % (24.20 incl) and 1 x 11.20 at 12 % = 35.40 incl... 24.20 + 11.20
        let u = InvoiceUbl::build(&source(
            vec![
                line("Biere", 2000, 2420, "A"),
                line("Sushi", 1000, 1120, "B"),
            ],
            vec![],
            3540,
        ))
        .unwrap();
        assert_eq!(u.lines[0].net_cents, 2000);
        assert_eq!(u.lines[0].price_micro, 100_000); // 10.0000 each
        assert_eq!(u.lines[1].net_cents, 1000);
        assert_eq!(u.tax_subtotals.len(), 2);
        assert_eq!(u.tax_subtotals[0].tax.label, "A");
        assert_eq!(
            (
                u.tax_subtotals[0].taxable_cents,
                u.tax_subtotals[0].tax_cents
            ),
            (2000, 420)
        );
        assert_eq!(
            (
                u.tax_subtotals[1].taxable_cents,
                u.tax_subtotals[1].tax_cents
            ),
            (1000, 120)
        );
        assert_eq!(u.totals.tax_inclusive_cents, 3540);
        assert_eq!(u.totals.payable_cents, 0);
        assert_eq!(u.totals.payable_rounding_cents, 0);
        assert_bis_sums(&u);
    }

    #[test]
    fn rounding_differences_go_to_the_payable_rounding_amount() {
        // Three lines of 10.00 incl at 21 %: net 8.26 each (8.2645...), taxable 24.78, tax 5.20,
        // tax inclusive 29.98 while the tickets said 30.00.
        let u = InvoiceUbl::build(&source(
            vec![
                line("a", 1000, 1000, "A"),
                line("b", 1000, 1000, "A"),
                line("c", 1000, 1000, "A"),
            ],
            vec![],
            3000,
        ))
        .unwrap();
        assert_eq!(u.totals.line_extension_cents, 2478);
        assert_eq!(u.tax_total_cents, 520);
        assert_eq!(u.totals.tax_inclusive_cents, 2998);
        assert_eq!(u.totals.payable_rounding_cents, 2);
        assert_eq!(u.totals.payable_cents, 0);
        assert_bis_sums(&u);
        let xml = to_ubl_xml(&u).unwrap();
        assert!(xml.contains(
            "<cbc:PayableRoundingAmount currencyID=\"EUR\">0.02</cbc:PayableRoundingAmount>"
        ));
        assert!(xml.contains("<cbc:PayableAmount currencyID=\"EUR\">0.00</cbc:PayableAmount>"));
    }

    #[test]
    fn an_event_level_discount_is_a_document_allowance_with_its_category() {
        // 50.00 incl at 12 % with a 5.00 discount (incl) on the whole ticket.
        let u = InvoiceUbl::build(&source(
            vec![line("Menu", 1000, 5000, "B")],
            vec![SourceAdjustment {
                reason: "Remise".into(),
                incl_cents: -500,
                vat_label: "B".into(),
            }],
            4500,
        ))
        .unwrap();
        assert_eq!(u.allowance_charges.len(), 1);
        let a = &u.allowance_charges[0];
        assert!(!a.is_charge);
        assert_eq!((a.net_cents, a.tax.id, a.tax.percent), (446, "S", 12)); // 500/1.12 = 446.4
        assert_eq!(u.totals.allowance_total_cents, 446);
        assert_eq!(u.totals.tax_exclusive_cents, 4464 - 446); // line net 4464.28 -> 4464
        assert_eq!(u.tax_subtotals[0].taxable_cents, 4464 - 446);
        assert_eq!(u.totals.prepaid_cents, 4500);
        assert_bis_sums(&u);
        let xml = to_ubl_xml(&u).unwrap();
        assert!(xml.contains("<cbc:ChargeIndicator>false</cbc:ChargeIndicator>"));
        assert!(xml.contains(
            "<cbc:AllowanceTotalAmount currencyID=\"EUR\">4.46</cbc:AllowanceTotalAmount>"
        ));
        // A charge flips the indicator and lands in the charge total.
        let u = InvoiceUbl::build(&source(
            vec![line("Menu", 1000, 5000, "B")],
            vec![SourceAdjustment {
                reason: "Service".into(),
                incl_cents: 300,
                vat_label: "B".into(),
            }],
            5300,
        ))
        .unwrap();
        assert!(u.allowance_charges[0].is_charge);
        assert_eq!(u.totals.charge_total_cents, 268);
        assert_bis_sums(&u);
    }

    #[test]
    fn a_refund_line_keeps_a_negative_quantity_and_a_positive_price() {
        let u = InvoiceUbl::build(&source(
            vec![
                line("Sushi", 2000, 2240, "B"),
                line("Sushi (remboursé)", -1000, -1120, "B"),
            ],
            vec![],
            1120,
        ))
        .unwrap();
        assert_eq!(u.lines[1].net_cents, -1000);
        assert_eq!(u.lines[1].price_micro, 100_000);
        assert_eq!(u.totals.line_extension_cents, 1000);
        assert_eq!(u.totals.payable_cents, 0);
        assert_bis_sums(&u);
        let xml = to_ubl_xml(&u).unwrap();
        assert!(xml.contains(">-1.000</cbc:InvoicedQuantity>"));
        assert!(xml.contains(
            "<cbc:LineExtensionAmount currencyID=\"EUR\">-10.00</cbc:LineExtensionAmount>"
        ));
    }

    #[test]
    fn zero_rated_and_exempt_labels_map_and_exempt_carries_a_reason() {
        let u = InvoiceUbl::build(&source(
            vec![
                line("Eau", 1000, 200, "D"),
                line("Caution", 1000, 500, "X"),
                line("Vin", 1000, 1210, "A"),
            ],
            vec![],
            1910,
        ))
        .unwrap();
        let ids: Vec<(&str, i64)> = u
            .tax_subtotals
            .iter()
            .map(|s| (s.tax.id, s.tax.percent))
            .collect();
        assert_eq!(ids, vec![("S", 21), ("Z", 0), ("E", 0)]);
        assert_eq!(u.tax_subtotals[1].tax_cents, 0);
        assert_bis_sums(&u);
        let xml = to_ubl_xml(&u).unwrap();
        // BR-E-10: a reason on the exempt subtotal only.
        assert_eq!(xml.matches("cbc:TaxExemptionReasonCode").count(), 2); // open + close tag
    }

    #[test]
    fn bad_input_is_refused() {
        assert!(InvoiceUbl::build(&source(vec![], vec![], 0)).is_err());
        assert!(InvoiceUbl::build(&source(vec![line("a", 0, 100, "A")], vec![], 100)).is_err());
        assert!(InvoiceUbl::build(&source(vec![line("a", 1000, 100, "Q")], vec![], 100)).is_err());
        let mut u =
            InvoiceUbl::build(&source(vec![line("a", 1000, 100, "A")], vec![], 100)).unwrap();
        u.id = " ".into();
        assert!(to_ubl_xml(&u).is_err());
    }

    #[test]
    fn the_xml_has_the_bis_header_the_belgian_endpoint_and_escaped_text() {
        let u =
            InvoiceUbl::build(&source(vec![line("Biere", 2000, 2420, "A")], vec![], 2420)).unwrap();
        let xml = to_ubl_xml(&u).unwrap();
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Invoice xmlns="));
        for needle in [
            "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2",
            "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2",
            "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2",
            "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0",
            "<cbc:EndpointID schemeID=\"0208\">0308357159</cbc:EndpointID>",
            "<cbc:EndpointID schemeID=\"0208\">0499999960</cbc:EndpointID>",
            "<cbc:BuyerReference>20260001</cbc:BuyerReference>",
            "<cbc:PrepaidAmount currencyID=\"EUR\">24.20</cbc:PrepaidAmount>",
            "Acme &amp; Fils &lt;SA&gt;",
        ] {
            assert!(xml.contains(needle), "{needle}\n{xml}");
        }
        assert!(!xml.contains("DueDate"));
        assert!(!xml.contains("0208:BE"));
    }

    #[test]
    fn element_order_follows_the_ubl_schema() {
        let u = InvoiceUbl::build(&source(
            vec![line("a", 1000, 121, "A")],
            vec![SourceAdjustment {
                reason: "r".into(),
                incl_cents: -10,
                vat_label: "A".into(),
            }],
            111,
        ))
        .unwrap();
        let xml = to_ubl_xml(&u).unwrap();
        let order = [
            "cbc:CustomizationID",
            "cbc:ProfileID",
            "cbc:ID>",
            "cbc:IssueDate",
            "cbc:InvoiceTypeCode",
            "cbc:Note",
            "cbc:DocumentCurrencyCode",
            "cbc:BuyerReference",
            "cac:AccountingSupplierParty",
            "cac:AccountingCustomerParty",
            "cac:AllowanceCharge",
            "cac:TaxTotal",
            "cac:LegalMonetaryTotal",
            "cac:InvoiceLine",
        ];
        let mut last = 0;
        for tag in order {
            let at = xml[last..]
                .find(tag)
                .unwrap_or_else(|| panic!("{tag} out of order"));
            last += at;
        }
    }

    #[test]
    fn a_golden_two_line_document() {
        let u = InvoiceUbl::build(&source(
            vec![
                line("Biere", 2000, 2420, "A"),
                line("Sushi", 1000, 1120, "B"),
            ],
            vec![],
            3540,
        ))
        .unwrap();
        let xml = to_ubl_xml(&u).unwrap();
        if std::env::var_os("PILI_UPDATE_GOLDEN").is_some() {
            std::fs::write(
                concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden_two_labels.xml"),
                &xml,
            )
            .unwrap();
        }
        let golden = include_str!("../tests/golden_two_labels.xml");
        assert_eq!(
            xml, golden,
            "regenerate the golden after reviewing the change:\n{xml}"
        );
    }

    #[test]
    fn money_and_date_helpers() {
        assert_eq!(money(5), "0.05");
        assert_eq!(money(-5), "-0.05");
        assert_eq!(money(123456), "1234.56");
        assert_eq!(quantity(350), "0.350");
        assert_eq!(price(12_345), "1.2345");
        assert_eq!(
            issue_date_from_pos_dt("2026-05-14T13:00:00+02:00").unwrap(),
            "2026-05-14"
        );
        assert_eq!(date_plus_days("2026-05-14", 30).unwrap(), "2026-06-13");
        assert!(vat_label_to_bis("Q").is_err());
        assert_eq!(vat_label_to_bis("X").unwrap(), ("E", "0.00"));
    }

    // The five tests below keep the names (and the intent) of the tests of the first UBL
    // writer, which the cents-based writer of V11-T1 replaced: the inventory of tests never
    // loses a name, and the old assertions still hold on the new API.

    fn sample_xml() -> String {
        let u =
            InvoiceUbl::build(&source(vec![line("Biere", 2000, 2420, "A")], vec![], 2420)).unwrap();
        to_ubl_xml(&u).unwrap()
    }

    #[test]
    fn ubl_xml_contains_required_namespaces() {
        let xml = sample_xml();
        assert!(xml.contains("urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"));
        assert!(
            xml.contains(
                "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
            )
        );
        assert!(
            xml.contains("urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2")
        );
        assert!(xml.contains("urn:cen.eu:en16931:2017#compliant"));
    }

    #[test]
    fn ubl_xml_contains_invoice_number_and_parties() {
        let xml = sample_xml();
        assert!(xml.contains("20260001"));
        assert!(xml.contains("Restaurant Exemple"));
        assert!(xml.contains("Acme &amp; Fils &lt;SA&gt;"));
        assert!(xml.contains("BE0499999960"));
        assert!(xml.contains("BE0308357159"));
    }

    #[test]
    fn ubl_xml_carries_belgian_endpoint_scheme() {
        // Belgian parties MUST be carried under schemeID="0208" per the Peppol policy.
        let xml = sample_xml();
        assert!(
            xml.contains(r#"schemeID="0208""#),
            "Belgian parties must use Peppol scheme 0208; XML was: {xml}"
        );
    }

    #[test]
    fn vat_label_mapping_covers_all_sce_codes() {
        assert_eq!(vat_label_to_bis("A").unwrap(), ("S", "21.00"));
        assert_eq!(vat_label_to_bis("B").unwrap(), ("S", "12.00"));
        assert_eq!(vat_label_to_bis("C").unwrap(), ("S", "6.00"));
        assert_eq!(vat_label_to_bis("D").unwrap(), ("Z", "0.00"));
        assert_eq!(vat_label_to_bis("X").unwrap(), ("E", "0.00"));
        assert!(vat_label_to_bis("Q").is_err());
    }

    #[test]
    fn date_helpers_round_trip() {
        assert_eq!(
            issue_date_from_pos_dt("2026-05-14T13:00:00+02:00").unwrap(),
            "2026-05-14"
        );
        assert_eq!(date_plus_days("2026-05-14", 30).unwrap(), "2026-06-13");
    }
}
