//! Facade of the B2B invoice commands (engine module `invoice`): one method per Tauri
//! command, same parameter names, one engine call each.

use pos_engine::commands::invoice;

use crate::convert;
use crate::core::PosEngine;
use crate::error::PosError;
use crate::records::invoice::{
    B2bCustomerInput, Customer, CustomerLookup, InvoicePreview, InvoiceResult, InvoiceSummary,
    InvoiceableSale, PeppolDeliveries,
};

// The closures name the engine function at the call, which `check_command_parity.py` looks for.
#[uniffi::export]
impl PosEngine {
    /// Recent signed sales a cashier can put on an invoice.
    pub async fn list_invoiceable_sales(
        &self,
        limit: Option<i64>,
        query: Option<String>,
    ) -> Result<Vec<InvoiceableSale>, PosError> {
        let sales = self
            .blocking(move |st| invoice::do_list_invoiceable_sales(st, limit, query))
            .await?;
        convert::out(&sales)
    }

    /// What an invoice of these sales would contain, without signing.
    pub async fn invoice_preview(
        &self,
        sale_event_ids: Vec<i64>,
    ) -> Result<InvoicePreview, PosError> {
        let preview = self
            .blocking(move |st| invoice::do_invoice_preview(st, &sale_event_ids))
            .await?;
        convert::out(&preview)
    }

    /// Check a VAT number and ask the Peppol network about it (3 s at most).
    pub async fn lookup_b2b_customer(
        &self,
        vat_number: String,
    ) -> Result<CustomerLookup, PosError> {
        let lookup = self
            .spawn(move |engine| async move {
                invoice::do_lookup_b2b_customer(&engine, &vat_number).await
            })
            .await?;
        convert::out(&lookup)
    }

    /// Known customers whose VAT number or name starts with `query`.
    pub async fn search_b2b_customers(&self, query: String) -> Result<Vec<Customer>, PosError> {
        let customers = self
            .blocking(move |st| invoice::do_search_b2b_customers(st, &query))
            .await?;
        convert::out(&customers)
    }

    /// Create or update a customer.
    pub async fn save_b2b_customer(
        &self,
        customer: B2bCustomerInput,
    ) -> Result<Customer, PosError> {
        let customer: invoice::B2bCustomerInput = convert::inp(&customer)?;
        let saved = self
            .blocking(move |st| invoice::do_save_b2b_customer(st, customer))
            .await?;
        convert::out(&saved)
    }

    /// M150: sign the invoice (event I) of the chosen sales for a saved customer.
    pub async fn sign_invoice_fiscal(
        &self,
        sale_event_ids: Vec<i64>,
        customer_id: i64,
        employee_insz: String,
    ) -> Result<InvoiceResult, PosError> {
        let result = self
            .spawn(move |engine| async move {
                invoice::do_sign_invoice_fiscal(&engine, sale_event_ids, customer_id, employee_insz)
                    .await
            })
            .await?;
        convert::out(&result)
    }

    /// The newest invoices, with the state of their Peppol delivery.
    pub async fn list_invoices(&self, limit: Option<i64>) -> Result<Vec<InvoiceSummary>, PosError> {
        let invoices = self
            .blocking(move |st| invoice::do_list_invoices(st, limit))
            .await?;
        convert::out(&invoices)
    }

    /// The Peppol panel (administrators only).
    pub async fn list_peppol_deliveries(
        &self,
        limit: Option<i64>,
    ) -> Result<PeppolDeliveries, PosError> {
        let panel = self
            .blocking(move |st| invoice::do_list_peppol_deliveries(st, limit))
            .await?;
        convert::out(&panel)
    }

    /// Put a parked delivery back in the queue (administrators only).
    pub async fn retry_peppol_delivery(&self, delivery_id: i64) -> Result<(), PosError> {
        self.blocking(move |st| invoice::do_retry_peppol_delivery(st, delivery_id))
            .await
    }
}
