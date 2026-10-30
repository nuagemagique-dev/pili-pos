//! Checkout scale link: the Bizerba "Checkout-Dialog 06" protocol as a
//! Dibal DPOS-400 speaks it (protocol 24 in its test menu).
//!
//! - [`frame`]: the records as bytes, both ways.
//! - [`session`]: one weighing as a state machine, without clock or I/O.
//! - [`driver`]: runs a session over TCP.
//!
//! The spec is Bizerba's 1997 "Description of Protocol - Checkout-Dialog
//! 06"; nothing here has met a real scale yet (see
//! `docs/internal/design/BALANCE_DIBAL_FR.md`).

pub mod driver;
pub mod frame;
pub mod session;

pub use driver::{DriverConfig, ProbeOutcome, ScaleDriver, WeighOutcome};
pub use frame::{CheckValue, Message, PriceDigits, WeightRecord};
pub use session::{CheckEvent, Failure, Progress, Reading, WeighRequest};
