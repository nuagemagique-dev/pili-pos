//! The FFI integration tests: one test binary, one module per domain. The domain cards fill their
//! module (and `support.rs` grows helpers); they never add a second test binary.

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
mod scale;
mod support;
