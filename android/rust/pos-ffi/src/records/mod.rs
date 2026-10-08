//! The FFI records (`#[derive(uniffi::Record)]`), one file per domain, mirroring the engine's serde
//! shapes; `convert::out` and `convert::inp` move values between the two.

pub mod auth;
pub mod checkout;
pub mod common;
pub mod hubrise;
pub mod orders;
pub mod reports;
