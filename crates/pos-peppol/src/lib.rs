//! Belgian Peppol B2B e-invoicing.
//!
//! From 2026-01-01 every B2B invoice issued by a Belgian VAT-registered
//! seller must be a **structured electronic invoice** delivered via the
//! Peppol network (Royal Decree, OECD/Peppol BIS Billing 3.0 schema).
//! This crate covers two responsibilities:
//!
//! 1. **`ubl` module** — serialize an SCE-2.0 [`pos_fiscal::inputs::InvoiceInput`]
//!    (the fiscal record we ALREADY produce when the operator chooses
//!    "B2B invoice" at checkout) into a UBL 2.1 Invoice XML document
//!    compliant with the Peppol BIS Billing 3.0 profile.
//! 2. **`access_point` module** — `AccessPoint` trait describing how
//!    we hand the signed UBL to a Peppol AP for AS4 transport to the
//!    receiver. Two implementations are sketched: `StorecoveAccessPoint`
//!    (simplest API, JSON-over-HTTPS) and `UnifiedpostAccessPoint`.
//!
//! ## Scope of this crate at submission time
//!
//! - **UBL serialization**: minimum-viable BIS Billing 3.0. Produces
//!   valid XML for simple line-item invoices with a single VAT category.
//!   Multi-VAT and credit-note variants are stub'd.
//! - **AccessPoint**: the trait is wired and there is a no-op
//!   `LoggingAccessPoint` for tests. Live providers require a contract,
//!   an API key and a sender `endpointID` registration on the Peppol network,
//!   which is operator-side paperwork tracked in
//!   `docs/SUBMISSION_ARTIFACTS/PEPPOL_AP_DECISION.md`.
//!
//! Tokyo Sushi Bar is B2C-dominant; B2B invoices are exceptional. So
//! the crate exists to satisfy the spec without blocking the main
//! dine-in flow on Peppol-AP procurement.

pub mod access_point;
pub mod errors;
pub mod ubl;

pub use access_point::{AccessPoint, DeliveryReceipt, LoggingAccessPoint};
pub use errors::PeppolError;
pub use ubl::{InvoiceUbl, Party, Address, LineItem};
