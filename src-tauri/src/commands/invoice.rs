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

use pos_fiscal::archive::{self, ArchiveParams};
use pos_fiscal::counter::FiscalCounter;
use pos_fiscal::event::build_invoice_input;
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::response::FdmReference;
use pos_fiscal::types::EventLabel;

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

/// What the UI receives after the FDM signs the I event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceSignResult {
    pub success: bool,
    /// Row id of the archived I event — pass to `issue_b2b_invoice`.
    pub fiscal_event_id: Option<i64>,
    pub invoice_no: Option<String>,
    pub error: Option<String>,
}

/// M150 — sign the SCE 2.0 invoice fiscal event (I). Consolidates one
/// or more prior sales (their archived FDM references become the
/// `fdmRefs` of the invoice, as in the published M150 fixture), sends
/// `signInvoice` to the FDM and archives the I event. The Peppol/UBL
/// delivery is a separate step (`issue_b2b_invoice`).
#[tauri::command]
pub async fn sign_invoice_fiscal(
    state: State<'_, AppState>,
    sale_fiscal_event_ids: Vec<i64>,
    customer_vat_no: String,
    employee_insz: String,
) -> Result<InvoiceSignResult, String> {
    do_sign_invoice_fiscal(
        state.inner(),
        sale_fiscal_event_ids,
        customer_vat_no,
        employee_insz,
    )
    .await
}

pub async fn do_sign_invoice_fiscal(
    state: &AppState,
    sale_fiscal_event_ids: Vec<i64>,
    customer_vat_no: String,
    employee_insz: String,
) -> Result<InvoiceSignResult, String> {
    if sale_fiscal_event_ids.is_empty() {
        return Err("invoice must reference at least one sale".to_string());
    }

    let (config, bp_id, fdm_refs, invoice_no, ticket_number) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let config = state.config.lock().map_err(|e| e.to_string())?.clone();
        let bp = state
            .booking_period_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone();

        // Rebuild the FDM reference of every underlying sale from its
        // archived response.
        let mut refs = Vec::with_capacity(sale_fiscal_event_ids.len());
        for id in &sale_fiscal_event_ids {
            let event = pos_db::queries::fiscal::get_fiscal_event(&db, *id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("fiscal event {id} not found"))?;
            let label_str = event
                .event_label
                .ok_or_else(|| format!("event {id} has no FDM label"))?;
            let label = match label_str.as_str() {
                "N" => EventLabel::N,
                other => {
                    return Err(format!(
                        "event {id} is {other}, only N sales are invoiceable"
                    ));
                }
            };
            refs.push(FdmReference {
                fdm_id: event
                    .fdm_id
                    .ok_or_else(|| format!("event {id} has no FDM id"))?,
                fdm_date_time: event
                    .fdm_datetime
                    .ok_or_else(|| format!("event {id} has no FDM datetime"))?,
                event_label: label,
                event_counter: event
                    .event_counter
                    .ok_or_else(|| format!("event {id} has no event counter"))?,
                total_counter: event
                    .total_counter
                    .ok_or_else(|| format!("event {id} has no total counter"))?,
            });
        }

        // Invoice numbering: <year><seq 4 digits>, sequential over the
        // archived I events (the M150 fixture uses "20240005").
        let issued: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM fiscal_events WHERE event_type = 'I'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let invoice_no = format!("{}{:04}", chrono::Utc::now().format("%Y"), issued + 1);

        // Peek — the counter only commits together with the archive,
        // so an FDM failure never creates a sequence gap.
        let tn = FiscalCounter::peek_next(&db, EventLabel::I).map_err(|e| e.to_string())?;
        (config, bp, refs, invoice_no, tn)
    };

    let input = build_invoice_input(
        &config,
        ticket_number,
        &employee_insz,
        &bp_id,
        &invoice_no,
        &customer_vat_no,
        &fdm_refs,
    );
    let request_json = serde_json::to_string(&input).map_err(|e| e.to_string())?;

    let fdm = FdmClient::new(&config.fdm_url);
    let sign_result = match fdm.sign_invoice(input, config.training_mode).await {
        Ok(r) => r,
        Err(e) => {
            return Ok(InvoiceSignResult {
                success: false,
                fiscal_event_id: None,
                invoice_no: None,
                error: Some(format!("FDM refused signInvoice: {e}")),
            });
        }
    };

    let fiscal_event_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
        let committed = FiscalCounter::next(&tx, EventLabel::I).map_err(|e| e.to_string())?;
        debug_assert_eq!(committed, ticket_number, "I counter drifted");
        let id = archive::archive_event(
            &tx,
            ArchiveParams {
                event_type: EventLabel::I,
                sequence_number: ticket_number,
                order_id: None,
                pos_id: &config.pos_id,
                terminal_id: &config.terminal_id,
                user_insz: &employee_insz,
                request_json: &request_json,
                sign_result: Some(&sign_result),
            },
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        id
    };

    Ok(InvoiceSignResult {
        success: true,
        fiscal_event_id: Some(fiscal_event_id),
        invoice_no: Some(invoice_no),
        error: None,
    })
}

/// Issue a B2B invoice via Peppol. The caller is expected to have
/// already produced and signed the SCE-2.0 invoice fiscal event via
/// `sign_invoice_fiscal` above — this command does NOT re-emit the
/// fiscal event. It only takes the resulting `fiscal_event_id`
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
