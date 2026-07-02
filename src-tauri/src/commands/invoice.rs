//! Peppol B2B e-invoice IPC commands.
//!
//! From 2026-01-01 every Belgian VAT-registered B2B invoice must be
//! delivered as a structured electronic document over the Peppol
//! network (BIS Billing 3.0). This command finalizes a B2B sale that
//! the operator has already rung up as a normal sale:
//!
//! 1. Build the UBL 2.1 invoice from the line items + buyer details
//!    collected at checkout.
//! 2. Submit it to the configured Access Point (no live AP wired up
//!    yet — see `pos-peppol::access_point::LoggingAccessPoint` for
//!    the dev no-op, `StorecoveAccessPoint` for the production stub).
//! 3. Persist the UBL + AP response in `peppol_deliveries` for the
//!    audit trail. Same retention rule as `fiscal_events`: append-only.
//!
//! ## Scope at submission time
//!
//! - Single-VAT-category invoices (the most common Tokyo Sushi Bar
//!   B2B case: a colleague-restaurant ordering takeaway for a staff
//!   meal). Multi-VAT and credit-note variants are stub'd in
//!   `pos-peppol` but not surfaced through IPC yet.
//! - No live AP — the cert dossier declares Peppol as "ready, AP
//!   procurement in progress" since AP onboarding has its own
//!   contractual timeline independent of SPF Finances certification.
//!
//! The command is intentionally synchronous in the AP submission:
//! Tokyo Sushi Bar's B2B volume is so low (< 1 invoice/week) that
//! enqueue-and-drain isn't worth the complexity. If the AP is down
//! the operator gets an error and can re-trigger from the order
//! detail screen.

use pos_peppol::ubl::{
    self, Address, Amount, Country, CustomerParty, EndpointId, InvoiceUbl, Item,
    LegalMonetaryTotal, LineItem, Party, PartyName, PartyTaxScheme, Price, Quantity, SupplierParty,
    TaxCategory, TaxScheme, TaxTotal, vat_label_to_bis,
};
use pos_peppol::{AccessPoint, LoggingAccessPoint};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::state::AppState;

/// One line on the B2B invoice as supplied by the UI. Quantities are
/// "each" by default; weight-based items are out of scope until we
/// wire `unit_kind` through. Prices are TVA-included euros (the UBL
/// emitter converts to ex-VAT internally).
#[derive(Debug, Clone, Deserialize)]
pub struct B2bLine {
    pub name: String,
    pub quantity: f64,
    pub unit_price_incl_vat: f64,
    /// SCE-2.0 VAT label: A/B/C/D/X.
    pub vat_label: String,
}

/// Buyer details collected at checkout. Operator enters them once per
/// B2B invoice — there's no customer-account system at this scale.
#[derive(Debug, Clone, Deserialize)]
pub struct BuyerInput {
    pub vat_number: String,
    pub name: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    /// ISO 3166-1 alpha-2, e.g. "BE", "FR".
    pub country_code: String,
}

/// Supplier identification copied verbatim from PosConfig. Kept inline
/// here (rather than re-reading from the DB) because the UI already
/// has these values to display on the receipt, and threading them
/// avoids an extra DB hit in the synchronous submit path.
#[derive(Debug, Clone, Deserialize)]
pub struct SupplierInput {
    pub name: String,
    pub vat_number: String,
    pub street: String,
    pub postal_code: String,
    pub city: String,
    pub country_code: String,
}

/// What the UI displays after the AP returns. The fiscal_event_id ties
/// this delivery back to the underlying SCE-2.0 invoice event so the
/// audit trail is bidirectional.
#[derive(Debug, Clone, Serialize)]
pub struct B2bInvoiceResult {
    pub delivery_id: i64,
    pub ap_vendor: String,
    pub message_id: Option<String>,
    pub status: String,
    pub ubl_preview: String,
}

fn build_ubl(
    invoice_no: &str,
    issue_date: &str,
    supplier: &SupplierInput,
    buyer: &BuyerInput,
    lines: &[B2bLine],
) -> Result<InvoiceUbl, String> {
    if lines.is_empty() {
        return Err("invoice must have at least one line".to_string());
    }

    let mut total_ex_vat = 0.0_f64;
    let mut total_vat = 0.0_f64;
    let mut ubl_lines: Vec<LineItem> = Vec::with_capacity(lines.len());
    for (idx, l) in lines.iter().enumerate() {
        let (cat_id, percent_str) = vat_label_to_bis(&l.vat_label).map_err(|e| e.to_string())?;
        let rate: f64 = percent_str.parse().unwrap_or(0.0);
        let incl = l.unit_price_incl_vat * l.quantity;
        let ex = if rate > 0.0 {
            incl / (1.0 + rate / 100.0)
        } else {
            incl
        };
        let vat = incl - ex;
        total_ex_vat += ex;
        total_vat += vat;

        ubl_lines.push(LineItem {
            id: (idx + 1).to_string(),
            quantity: Quantity {
                unit_code: "EA".to_string(),
                value: l.quantity,
            },
            line_extension_amount: Amount {
                currency_id: "EUR".to_string(),
                value: ex,
            },
            item: Item {
                name: l.name.clone(),
                tax_category: TaxCategory {
                    id: cat_id.to_string(),
                    percent: percent_str.to_string(),
                    scheme: TaxScheme {
                        id: "VAT".to_string(),
                    },
                },
            },
            price: Price {
                amount: Amount {
                    currency_id: "EUR".to_string(),
                    value: if l.quantity > 0.0 {
                        ex / l.quantity
                    } else {
                        0.0
                    },
                },
            },
        });
    }

    let due_date = ubl::due_date_30_days(issue_date).map_err(|e| e.to_string())?;

    let supplier_party = Party {
        endpoint_id: EndpointId {
            scheme_id: "0208".to_string(),
            value: supplier.vat_number.clone(),
        },
        name: PartyName {
            name: supplier.name.clone(),
        },
        postal_address: Address {
            street: supplier.street.clone(),
            postal_code: supplier.postal_code.clone(),
            city: supplier.city.clone(),
            country: Country {
                code: supplier.country_code.clone(),
            },
        },
        tax_scheme: PartyTaxScheme {
            vat_no: supplier.vat_number.clone(),
            scheme: TaxScheme {
                id: "VAT".to_string(),
            },
        },
    };

    let customer_party = Party {
        endpoint_id: EndpointId {
            scheme_id: "0208".to_string(),
            value: buyer.vat_number.clone(),
        },
        name: PartyName {
            name: buyer.name.clone(),
        },
        postal_address: Address {
            street: buyer.street.clone(),
            postal_code: buyer.postal_code.clone(),
            city: buyer.city.clone(),
            country: Country {
                code: buyer.country_code.clone(),
            },
        },
        tax_scheme: PartyTaxScheme {
            vat_no: buyer.vat_number.clone(),
            scheme: TaxScheme {
                id: "VAT".to_string(),
            },
        },
    };

    Ok(InvoiceUbl {
        id: invoice_no.to_string(),
        issue_date: issue_date.to_string(),
        due_date,
        invoice_type_code: "380".to_string(),
        document_currency_code: "EUR".to_string(),
        supplier: SupplierParty {
            party: supplier_party,
        },
        customer: CustomerParty {
            party: customer_party,
        },
        lines: ubl_lines,
        tax_total: TaxTotal {
            tax_amount: Amount {
                currency_id: "EUR".to_string(),
                value: total_vat,
            },
        },
        legal_monetary_total: LegalMonetaryTotal {
            line_extension_amount: Amount {
                currency_id: "EUR".to_string(),
                value: total_ex_vat,
            },
            tax_exclusive_amount: Amount {
                currency_id: "EUR".to_string(),
                value: total_ex_vat,
            },
            tax_inclusive_amount: Amount {
                currency_id: "EUR".to_string(),
                value: total_ex_vat + total_vat,
            },
            payable_amount: Amount {
                currency_id: "EUR".to_string(),
                value: total_ex_vat + total_vat,
            },
        },
    })
}

/// Issue a B2B invoice via Peppol. The caller is expected to have
/// already produced and signed the SCE-2.0 invoice fiscal event via
/// the standard `signInvoice` flow — this command does NOT re-emit
/// the fiscal event. It only takes the resulting `fiscal_event_id`
/// and the buyer's details, builds the UBL, and ships it.
#[tauri::command]
pub async fn issue_b2b_invoice(
    state: State<'_, AppState>,
    fiscal_event_id: i64,
    invoice_no: String,
    issue_date: String,
    supplier: SupplierInput,
    buyer: BuyerInput,
    lines: Vec<B2bLine>,
) -> Result<B2bInvoiceResult, String> {
    do_issue_b2b_invoice(
        state.inner(),
        fiscal_event_id,
        invoice_no,
        issue_date,
        supplier,
        buyer,
        lines,
    )
    .await
}

pub async fn do_issue_b2b_invoice(
    state: &AppState,
    fiscal_event_id: i64,
    invoice_no: String,
    issue_date: String,
    supplier: SupplierInput,
    buyer: BuyerInput,
    lines: Vec<B2bLine>,
) -> Result<B2bInvoiceResult, String> {
    let ubl_doc = build_ubl(&invoice_no, &issue_date, &supplier, &buyer, &lines)?;
    let xml = ubl::to_ubl_xml(&ubl_doc).map_err(|e| e.to_string())?;

    // Default to the logging AP until the operator wires Storecove/
    // Unifiedpost credentials. The cert build keeps this path so the
    // intake reviewer can see the full flow without us shipping live
    // AP credentials in the kiosk image.
    let ap: Box<dyn AccessPoint> = Box::new(LoggingAccessPoint);
    let ap_vendor = ap.vendor().to_string();
    let endpoint_id_value = format!("0208:{}", buyer.vat_number);

    let delivery_id: i64 = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "INSERT INTO peppol_deliveries
                (fiscal_event_id, invoice_no, customer_endpoint_id, ap_vendor, ubl_xml, status)
             VALUES (?, ?, ?, ?, ?, 'queued')
             RETURNING id",
            rusqlite::params![
                fiscal_event_id,
                invoice_no,
                endpoint_id_value,
                ap_vendor,
                xml,
            ],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?
    };

    match ap.submit(&xml).await {
        Ok(receipt) => {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE peppol_deliveries
                 SET status='accepted', ap_message_id=?, ap_response_json=?, accepted_at=?
                 WHERE id=?",
                rusqlite::params![
                    receipt.message_id,
                    receipt.raw_response,
                    receipt.accepted_at,
                    delivery_id,
                ],
            )
            .map_err(|e| e.to_string())?;
            Ok(B2bInvoiceResult {
                delivery_id,
                ap_vendor,
                message_id: Some(receipt.message_id),
                status: "accepted".to_string(),
                ubl_preview: xml,
            })
        }
        Err(e) => {
            let msg = e.to_string();
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE peppol_deliveries
                 SET status='failed', error_msg=?
                 WHERE id=?",
                rusqlite::params![msg, delivery_id],
            )
            .map_err(|e| e.to_string())?;
            Err(msg)
        }
    }
}
