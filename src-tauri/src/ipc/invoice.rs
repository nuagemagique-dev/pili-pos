//! B2B invoices (event I, M150), their customers and the Peppol delivery queue.
//! Each wrapper is one engine call (`scripts/check_ipc_wrappers.py`).

use tauri::State;

use pos_engine::commands::invoice::{
    B2bCustomerInput, Customer, CustomerLookup, InvoicePreview, InvoiceResult, InvoiceSummary,
    InvoiceableSale, PeppolDeliveries,
};

use super::Managed;

/// Recent signed sales a cashier can put on an invoice.
#[tauri::command(async)]
pub fn list_invoiceable_sales(
    state: State<'_, Managed>,
    limit: Option<i64>,
    query: Option<String>,
) -> Result<Vec<InvoiceableSale>, String> {
    pos_engine::commands::invoice::do_list_invoiceable_sales(state.inner(), limit, query)
}

/// What an invoice of these sales would contain, without signing.
#[tauri::command(async)]
pub fn invoice_preview(
    state: State<'_, Managed>,
    sale_event_ids: Vec<i64>,
) -> Result<InvoicePreview, String> {
    pos_engine::commands::invoice::do_invoice_preview(state.inner(), &sale_event_ids)
}

/// Check a VAT number and ask the Peppol network about it (3 s at most).
#[tauri::command]
pub async fn lookup_b2b_customer(
    state: State<'_, Managed>,
    vat_number: String,
) -> Result<CustomerLookup, String> {
    pos_engine::commands::invoice::do_lookup_b2b_customer(state.inner(), &vat_number).await
}

/// Known customers whose VAT number or name starts with `query`.
#[tauri::command(async)]
pub fn search_b2b_customers(
    state: State<'_, Managed>,
    query: String,
) -> Result<Vec<Customer>, String> {
    pos_engine::commands::invoice::do_search_b2b_customers(state.inner(), &query)
}

/// Create or update a customer.
#[tauri::command(async)]
pub fn save_b2b_customer(
    state: State<'_, Managed>,
    customer: B2bCustomerInput,
) -> Result<Customer, String> {
    pos_engine::commands::invoice::do_save_b2b_customer(state.inner(), customer)
}

/// M150: sign the invoice (event I) of the chosen sales for a saved customer, record it, print
/// it and queue its Peppol delivery.
#[tauri::command]
pub async fn sign_invoice_fiscal(
    state: State<'_, Managed>,
    sale_event_ids: Vec<i64>,
    customer_id: i64,
    employee_insz: String,
) -> Result<InvoiceResult, String> {
    pos_engine::commands::invoice::do_sign_invoice_fiscal(
        state.inner(),
        sale_event_ids,
        customer_id,
        employee_insz,
    )
    .await
}

/// The newest invoices, with the state of their Peppol delivery.
#[tauri::command(async)]
pub fn list_invoices(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<Vec<InvoiceSummary>, String> {
    pos_engine::commands::invoice::do_list_invoices(state.inner(), limit)
}

/// The Peppol panel (admins): setup, queue counts and deliveries.
#[tauri::command(async)]
pub fn list_peppol_deliveries(
    state: State<'_, Managed>,
    limit: Option<i64>,
) -> Result<PeppolDeliveries, String> {
    pos_engine::commands::invoice::do_list_peppol_deliveries(state.inner(), limit)
}

/// Put a parked delivery back in the queue (admins).
#[tauri::command(async)]
pub fn retry_peppol_delivery(state: State<'_, Managed>, delivery_id: i64) -> Result<(), String> {
    pos_engine::commands::invoice::do_retry_peppol_delivery(state.inner(), delivery_id)
}
