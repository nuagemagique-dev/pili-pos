//! The FFI integration tests: one test binary, one module per domain. The domain cards fill their
//! module (and `support.rs` grows helpers); they never add a second test binary.

mod auth;
mod badges;
mod checkout;
mod common;
mod eft;
mod events;
mod export;
mod helpers;
mod hubrise;
mod install;
mod invoice;
mod lifecycle;
mod menu_admin;
mod orders;
mod reports;
mod support;
