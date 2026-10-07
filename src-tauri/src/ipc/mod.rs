//! The Tauri command wrappers: one call each into `crate::commands`.

/// The state object the wrappers manage (a later step points it at the engine).
pub type Managed = crate::state::AppState;

pub mod auth;
pub mod checkout;
pub mod copy;
pub mod delivery;
pub mod financial;
#[cfg(feature = "hubrise")]
pub mod hubrise;
#[cfg(feature = "hubrise")]
pub mod integrations;
#[cfg(feature = "peppol")]
pub mod invoice;
pub mod menu;
#[cfg(feature = "hubrise")]
pub mod modifiers;
pub mod order;
pub mod print;
pub mod refund;
pub mod report;
pub mod settings;
pub mod social;
pub mod support;
pub mod tables;
pub mod unpaid;
#[cfg(feature = "hubrise")]
pub mod webshop;
