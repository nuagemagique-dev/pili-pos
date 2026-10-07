use tauri::State;

use pos_engine::commands::invoice::{
    B2bInvoiceResult, B2bLine, BuyerInput, InvoiceSignResult, SupplierInput,
};

use super::Managed;

/// M150 - sign the SCE 2.0 invoice fiscal event (I). Consolidates one
/// or more prior sales (their archived FDM references become the
/// `fdmRefs` of the invoice, as in the published M150 fixture), sends
/// `signInvoice` to the FDM and archives the I event. The Peppol/UBL
/// delivery is a separate step (`issue_b2b_invoice`).
#[tauri::command]
pub async fn sign_invoice_fiscal(
    state: State<'_, Managed>,
    sale_fiscal_event_ids: Vec<i64>,
    customer_vat_no: String,
    employee_insz: String,
) -> Result<InvoiceSignResult, String> {
    pos_engine::commands::invoice::do_sign_invoice_fiscal(
        state.inner(),
        sale_fiscal_event_ids,
        customer_vat_no,
        employee_insz,
    )
    .await
}

/// Issue a B2B invoice via Peppol. The caller is expected to have
/// already produced and signed the SCE-2.0 invoice fiscal event via
/// `sign_invoice_fiscal` above - this command does NOT re-emit the
/// fiscal event. It only takes the resulting `fiscal_event_id`
/// and the buyer's details, builds the UBL, and ships it.
#[tauri::command]
pub async fn issue_b2b_invoice(
    state: State<'_, Managed>,
    fiscal_event_id: i64,
    invoice_no: String,
    issue_date: String,
    supplier: SupplierInput,
    buyer: BuyerInput,
    lines: Vec<B2bLine>,
) -> Result<B2bInvoiceResult, String> {
    pos_engine::commands::invoice::do_issue_b2b_invoice(
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
