//! Belgian Peppol B2B e-invoicing (V11-D1: in every build).
//!
//! From 2026-01-01 a B2B invoice between Belgian VAT-registered parties is a structured
//! electronic invoice delivered over the Peppol network (BIS Billing 3.0). This crate holds
//! the three pieces the engine's invoice commands (`pos_engine::commands::invoice`) need:
//!
//! 1. [`participant`]: offline VAT checks and the `0208:` participant id.
//! 2. [`ubl`]: the UBL 2.1 Invoice document, built and summed in i64 eurocents.
//! 3. [`access_point`]: the [`AccessPoint`] trait with the Storecove API v2 implementation
//!    ([`StorecoveAccessPoint`], tested against a local mock) and the logging provider
//!    ([`LoggingAccessPoint`]). Failures are classified for the delivery queue
//!    ([`PeppolError`]).
//!
//! The crate does no queueing and no I/O of its own beyond the access point's HTTP calls:
//! the queue, the retries and the audit rows live in the engine.

pub mod access_point;
pub mod errors;
pub mod participant;
pub mod ubl;

pub use access_point::{
    AccessPoint, DeliveryReceipt, LOOKUP_TIMEOUT, LoggingAccessPoint, LookupResult, SUBMIT_TIMEOUT,
    StorecoveAccessPoint,
};
pub use errors::PeppolError;
pub use participant::{VatNumber, be_participant_id, parse_vat_number};
pub use ubl::{Address, InvoiceSource, InvoiceUbl, LineItem, Party, to_ubl_xml};
