//! The FFI integration tests: one test binary, one module per domain. The domain cards fill their
//! module (and `support.rs` grows helpers); they never add a second test binary.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup or conversion is the test's failure"
)]
#![expect(
    clippy::assert_is_empty,
    reason = "assert!(x.is_empty()) reads best for strings and lists of any type, and shows the value in the message"
)]
#![expect(
    clippy::too_many_lines,
    reason = "an end-to-end scenario per test reads top to bottom; splitting would hide the order"
)]
#![expect(
    clippy::fn_params_excessive_bools,
    reason = "test helpers take plain switches such as training mode or gate on/off"
)]

mod alerts;
mod allergens;
mod auth;
mod badges;
mod checkout;
mod common;
mod courses;
mod eft;
mod events;
mod export;
mod gift_vouchers;
mod helpers;
mod hubrise;
mod install;
mod invoice;
mod lifecycle;
mod menu_admin;
mod orders;
mod reports;
mod support;
