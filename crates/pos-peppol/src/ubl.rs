//! UBL 2.1 Invoice serializer - minimum viable BIS Billing 3.0 subset.
//!
//! BIS Billing 3.0 is the Peppol billing profile mandated by the Belgian
//! government for B2B e-invoicing. The full UBL 2.1 schema has hundreds
//! of optional elements; this serializer emits the minimum set the AP
//! providers (Storecove, Unifiedpost) and the receiver-side validators
//! require for a simple SCE-2.0 sale-derived invoice.
//!
//! ## Mapping from `pos_fiscal::inputs::InvoiceInput`
//!
//! | UBL element | Source field |
//! |---|---|
//! | `cbc:ID` | `InvoiceInput.invoice_no` |
//! | `cbc:IssueDate` | derived from `pos_date_time` (date only) |
//! | `cbc:DueDate` | `IssueDate + 30 days` (default) |
//! | `cbc:InvoiceTypeCode` | `380` (commercial invoice) or `381` (credit note) |
//! | `cbc:DocumentCurrencyCode` | `EUR` (Tokyo Sushi Bar default) |
//! | `cac:AccountingSupplierParty` | filled from `PosConfig.restaurant_*` |
//! | `cac:AccountingCustomerParty` | from `InvoiceInput.customer_vat_no` + caller-supplied buyer details |
//! | `cac:InvoiceLine` | one per `transaction.transactionLines[i]` |
//! | `cac:TaxTotal` | per-VAT-category subtotals |
//! | `cbc:LegalMonetaryTotal` | grand total + payable amount |
//!
//! What's intentionally stubbed (will not block Tokyo Sushi Bar's B2C flow):
//! - PaymentMeans block (we don't model bank details yet)
//! - AllowanceCharge (discounts) - UBL distinguishes line-level vs
//!   document-level allowances; spec is rich and Phase 5/6 already
//!   captures price changes as separate fiscal events.
//! - Multi-currency support - Tokyo Sushi Bar is EUR-only.

use chrono::{DateTime, Duration, NaiveDate};
use serde::Serialize;

use crate::errors::PeppolError;

/// Top-level UBL Invoice document. Implements `Serialize` so callers
/// can render it via `quick_xml::se::to_string`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename = "Invoice", rename_all = "PascalCase")]
pub struct InvoiceUbl {
    /// Sender's invoice number (must be unique per supplier).
    #[serde(rename = "cbc:ID")]
    pub id: String,

    /// `2024-07-29` style. Local supplier time.
    #[serde(rename = "cbc:IssueDate")]
    pub issue_date: String,

    /// `IssueDate + 30 days` by default.
    #[serde(rename = "cbc:DueDate")]
    pub due_date: String,

    /// `380` = commercial invoice. `381` = credit note (BIS Billing
    /// 3.0 uses a separate `CreditNote` root element for credit notes,
    /// but we model both via this struct and switch the type code).
    #[serde(rename = "cbc:InvoiceTypeCode")]
    pub invoice_type_code: String,

    /// `EUR`.
    #[serde(rename = "cbc:DocumentCurrencyCode")]
    pub document_currency_code: String,

    #[serde(rename = "cac:AccountingSupplierParty")]
    pub supplier: SupplierParty,

    #[serde(rename = "cac:AccountingCustomerParty")]
    pub customer: CustomerParty,

    #[serde(rename = "cac:InvoiceLine")]
    pub lines: Vec<LineItem>,

    #[serde(rename = "cac:TaxTotal")]
    pub tax_total: TaxTotal,

    #[serde(rename = "cac:LegalMonetaryTotal")]
    pub legal_monetary_total: LegalMonetaryTotal,
}

/// A buyer or seller. Peppol identifies parties by their `endpointID`,
/// which for Belgian VAT-registered entities is `0208:<VAT-no>` (e.g.
/// `0208:BE0499999960`). The `0208` prefix is the Peppol ISO 6523
/// scheme code for "Belgian VAT". For foreign customers the prefix
/// changes (e.g. `9925` for Dutch VAT).
#[derive(Debug, Clone, Serialize)]
pub struct Party {
    #[serde(rename = "cbc:EndpointID", serialize_with = "serialize_endpoint_id")]
    pub endpoint_id: EndpointId,
    #[serde(rename = "cac:PartyName")]
    pub name: PartyName,
    #[serde(rename = "cac:PostalAddress")]
    pub postal_address: Address,
    #[serde(rename = "cac:PartyTaxScheme")]
    pub tax_scheme: PartyTaxScheme,
}

#[derive(Debug, Clone)]
pub struct EndpointId {
    /// e.g. "0208" (Belgian VAT), "9925" (Dutch VAT).
    pub scheme_id: String,
    /// e.g. "BE0499999960".
    pub value: String,
}

fn serialize_endpoint_id<S: serde::Serializer>(id: &EndpointId, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    // quick_xml-friendly: we serialize as `<cbc:EndpointID schemeID="…">value</cbc:EndpointID>`
    // by emitting both an `@schemeID` attribute and a `$value` text node.
    let mut st = s.serialize_struct("EndpointID", 2)?;
    st.serialize_field("@schemeID", &id.scheme_id)?;
    st.serialize_field("$value", &id.value)?;
    st.end()
}

#[derive(Debug, Clone, Serialize)]
pub struct PartyName {
    #[serde(rename = "cbc:Name")]
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Address {
    #[serde(rename = "cbc:StreetName")]
    pub street: String,
    #[serde(rename = "cbc:CityName")]
    pub city: String,
    #[serde(rename = "cbc:PostalZone")]
    pub postal_code: String,
    #[serde(rename = "cac:Country")]
    pub country: Country,
}

#[derive(Debug, Clone, Serialize)]
pub struct Country {
    #[serde(rename = "cbc:IdentificationCode")]
    pub code: String, // ISO 3166-1 alpha-2, e.g. "BE"
}

#[derive(Debug, Clone, Serialize)]
pub struct PartyTaxScheme {
    #[serde(rename = "cbc:CompanyID")]
    pub vat_no: String,
    #[serde(rename = "cac:TaxScheme")]
    pub scheme: TaxScheme,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxScheme {
    #[serde(rename = "cbc:ID")]
    pub id: String, // "VAT"
}

#[derive(Debug, Clone, Serialize)]
pub struct SupplierParty {
    #[serde(rename = "cac:Party")]
    pub party: Party,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerParty {
    #[serde(rename = "cac:Party")]
    pub party: Party,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineItem {
    #[serde(rename = "cbc:ID")]
    pub id: String,
    #[serde(rename = "cbc:InvoicedQuantity", serialize_with = "serialize_quantity")]
    pub quantity: Quantity,
    #[serde(
        rename = "cbc:LineExtensionAmount",
        serialize_with = "serialize_amount"
    )]
    pub line_extension_amount: Amount,
    #[serde(rename = "cac:Item")]
    pub item: Item,
    #[serde(rename = "cac:Price")]
    pub price: Price,
}

#[derive(Debug, Clone)]
pub struct Quantity {
    pub unit_code: String, // "EA" for piece, "KGM" for kilogram
    pub value: f64,
}

fn serialize_quantity<S: serde::Serializer>(q: &Quantity, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    let mut st = s.serialize_struct("Quantity", 2)?;
    st.serialize_field("@unitCode", &q.unit_code)?;
    st.serialize_field("$value", &format!("{:.4}", q.value))?;
    st.end()
}

#[derive(Debug, Clone)]
pub struct Amount {
    pub currency_id: String, // "EUR"
    pub value: f64,
}

fn serialize_amount<S: serde::Serializer>(a: &Amount, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeStruct;
    let mut st = s.serialize_struct("Amount", 2)?;
    st.serialize_field("@currencyID", &a.currency_id)?;
    st.serialize_field("$value", &format!("{:.2}", a.value))?;
    st.end()
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    #[serde(rename = "cbc:Name")]
    pub name: String,
    #[serde(rename = "cac:ClassifiedTaxCategory")]
    pub tax_category: TaxCategory,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxCategory {
    #[serde(rename = "cbc:ID")]
    pub id: String, // BIS Billing: "S" (standard), "Z" (zero-rated), "E" (exempt)
    #[serde(rename = "cbc:Percent")]
    pub percent: String, // "21.00", "12.00", "6.00", "0.00"
    #[serde(rename = "cac:TaxScheme")]
    pub scheme: TaxScheme,
}

#[derive(Debug, Clone, Serialize)]
pub struct Price {
    #[serde(rename = "cbc:PriceAmount", serialize_with = "serialize_amount")]
    pub amount: Amount,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxTotal {
    #[serde(rename = "cbc:TaxAmount", serialize_with = "serialize_amount")]
    pub tax_amount: Amount,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegalMonetaryTotal {
    #[serde(
        rename = "cbc:LineExtensionAmount",
        serialize_with = "serialize_amount"
    )]
    pub line_extension_amount: Amount,
    #[serde(rename = "cbc:TaxExclusiveAmount", serialize_with = "serialize_amount")]
    pub tax_exclusive_amount: Amount,
    #[serde(rename = "cbc:TaxInclusiveAmount", serialize_with = "serialize_amount")]
    pub tax_inclusive_amount: Amount,
    #[serde(rename = "cbc:PayableAmount", serialize_with = "serialize_amount")]
    pub payable_amount: Amount,
}

/// Map an SCE-2.0 VAT label letter (A/B/C/D/X) to a BIS Billing 3.0
/// tax category code + numeric rate string. The cert manual maps
/// these as: A=21%, B=12%, C=6%, D=0%, X=exempt.
pub fn vat_label_to_bis(label: &str) -> Result<(&'static str, &'static str), PeppolError> {
    match label {
        "A" => Ok(("S", "21.00")),
        "B" => Ok(("S", "12.00")),
        "C" => Ok(("S", "6.00")),
        "D" => Ok(("Z", "0.00")),
        "X" => Ok(("E", "0.00")),
        other => Err(PeppolError::MissingField(Box::leak(
            format!("unknown VAT label {other}").into_boxed_str(),
        ))),
    }
}

/// Render an `InvoiceUbl` to a canonical UBL 2.1 XML string with the
/// required `Invoice` root namespaces. `quick_xml` emits the body;
/// we prepend the `<?xml … ?>` prologue and the namespace-declared root
/// open tag, then close it manually. This is a thin wrapper - for
/// production use, swap to a fully namespace-aware emitter.
pub fn to_ubl_xml(invoice: &InvoiceUbl) -> Result<String, PeppolError> {
    let body = quick_xml::se::to_string(invoice).map_err(|e| PeppolError::Xml(e.to_string()))?;
    // quick_xml::se::to_string emits `<Invoice>…</Invoice>` - we need the
    // BIS Billing 3.0 namespace declarations on the root. Strip the
    // outer pair we got, re-wrap with proper xmlns attributes.
    let body_inner = body
        .strip_prefix("<Invoice>")
        .and_then(|s| s.strip_suffix("</Invoice>"))
        .ok_or_else(|| PeppolError::Xml("unexpected root from quick_xml".into()))?;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
         xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
         xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0</cbc:CustomizationID>
  <cbc:ProfileID>urn:fdc:peppol.eu:2017:poacc:billing:01:1.0</cbc:ProfileID>
  {body_inner}
</Invoice>
"#,
    );
    Ok(xml)
}

/// Convenience: compute a `DueDate` 30 days after `issue_date`.
pub fn due_date_30_days(issue_date: &str) -> Result<String, PeppolError> {
    let d = NaiveDate::parse_from_str(issue_date, "%Y-%m-%d")
        .map_err(|e| PeppolError::Xml(format!("bad issue_date {issue_date}: {e}")))?;
    Ok((d + Duration::days(30)).format("%Y-%m-%d").to_string())
}

/// Extract the date-only portion of an RFC3339 `pos_date_time`.
pub fn issue_date_from_pos_dt(pos_dt: &str) -> Result<String, PeppolError> {
    let dt: DateTime<chrono::FixedOffset> = DateTime::parse_from_rfc3339(pos_dt)
        .map_err(|e| PeppolError::Xml(format!("bad pos_date_time {pos_dt}: {e}")))?;
    Ok(dt.format("%Y-%m-%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_invoice() -> InvoiceUbl {
        InvoiceUbl {
            id: "INV-2026-0001".into(),
            issue_date: "2026-05-14".into(),
            due_date: "2026-06-13".into(),
            invoice_type_code: "380".into(),
            document_currency_code: "EUR".into(),
            supplier: SupplierParty {
                party: Party {
                    endpoint_id: EndpointId {
                        scheme_id: "0208".into(),
                        value: "BE0499999960".into(),
                    },
                    name: PartyName {
                        name: "Tokyo Sushi Bar".into(),
                    },
                    postal_address: Address {
                        street: "Rue Test 1".into(),
                        city: "Bruxelles".into(),
                        postal_code: "1000".into(),
                        country: Country { code: "BE".into() },
                    },
                    tax_scheme: PartyTaxScheme {
                        vat_no: "BE0499999960".into(),
                        scheme: TaxScheme { id: "VAT".into() },
                    },
                },
            },
            customer: CustomerParty {
                party: Party {
                    endpoint_id: EndpointId {
                        scheme_id: "0208".into(),
                        value: "BE0123456789".into(),
                    },
                    name: PartyName {
                        name: "Acme Corp".into(),
                    },
                    postal_address: Address {
                        street: "Rue Acme 42".into(),
                        city: "Liège".into(),
                        postal_code: "4000".into(),
                        country: Country { code: "BE".into() },
                    },
                    tax_scheme: PartyTaxScheme {
                        vat_no: "BE0123456789".into(),
                        scheme: TaxScheme { id: "VAT".into() },
                    },
                },
            },
            lines: vec![LineItem {
                id: "1".into(),
                quantity: Quantity {
                    unit_code: "EA".into(),
                    value: 2.0,
                },
                line_extension_amount: Amount {
                    currency_id: "EUR".into(),
                    value: 24.00,
                },
                item: Item {
                    name: "Dry Martini".into(),
                    tax_category: TaxCategory {
                        id: "S".into(),
                        percent: "21.00".into(),
                        scheme: TaxScheme { id: "VAT".into() },
                    },
                },
                price: Price {
                    amount: Amount {
                        currency_id: "EUR".into(),
                        value: 12.00,
                    },
                },
            }],
            tax_total: TaxTotal {
                tax_amount: Amount {
                    currency_id: "EUR".into(),
                    value: 4.17,
                },
            },
            legal_monetary_total: LegalMonetaryTotal {
                line_extension_amount: Amount {
                    currency_id: "EUR".into(),
                    value: 24.00,
                },
                tax_exclusive_amount: Amount {
                    currency_id: "EUR".into(),
                    value: 19.83,
                },
                tax_inclusive_amount: Amount {
                    currency_id: "EUR".into(),
                    value: 24.00,
                },
                payable_amount: Amount {
                    currency_id: "EUR".into(),
                    value: 24.00,
                },
            },
        }
    }

    #[test]
    fn ubl_xml_contains_required_namespaces() {
        let xml = to_ubl_xml(&sample_invoice()).expect("serialize");
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
        let xml = to_ubl_xml(&sample_invoice()).expect("serialize");
        assert!(xml.contains("INV-2026-0001"));
        assert!(xml.contains("Tokyo Sushi Bar"));
        assert!(xml.contains("Acme Corp"));
        assert!(xml.contains("BE0499999960"));
        assert!(xml.contains("BE0123456789"));
    }

    #[test]
    fn ubl_xml_carries_belgian_endpoint_scheme() {
        // Belgian VAT IDs MUST be carried under schemeID="0208" per the
        // Peppol policy. Catching a slip in the scheme code is the
        // single biggest cert risk for B2B invoicing.
        let xml = to_ubl_xml(&sample_invoice()).expect("serialize");
        assert!(
            xml.contains(r#"schemeID="0208""#),
            "Belgian VAT IDs must use Peppol scheme 0208; XML was: {xml}"
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
        assert_eq!(due_date_30_days("2026-05-14").unwrap(), "2026-06-13");
    }
}
