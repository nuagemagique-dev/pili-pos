//! The Tauri command wrappers: one call each into `pos_engine::commands`.

/// The state object the wrappers manage: the engine (it derefs to the state).
pub type Managed = pos_engine::engine::Engine;

pub mod alerts;
pub mod allergens;
pub mod auth;
pub mod checkout;
pub mod copy;
pub mod delivery;
pub mod eft;
pub mod export;
pub mod fdm_monitor;
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
pub mod session;
pub mod settings;
pub mod social;
pub mod support;
pub mod tables;
pub mod unpaid;
#[cfg(feature = "hubrise")]
pub mod webshop;
