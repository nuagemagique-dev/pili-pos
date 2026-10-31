//! The Tauri command wrappers: one call each into `pos_engine::commands`.
//!
//! A wrapper that calls a synchronous `do_*` is `#[tauri::command(async)]`, so it runs
//! off the main thread. It still blocks a tokio worker while it waits for the database
//! mutex: an accepted trade-off, since a command holds the lock for a few milliseconds
//! and the pool is sized for the UI's handful of concurrent calls.

/// The state object the wrappers manage: the engine (it derefs to the state).
pub type Managed = pos_engine::engine::Engine;

pub mod alerts;
pub mod allergens;
pub mod auth;
pub mod badges;
pub mod checkout;
pub mod copy;
pub mod courses;
pub mod delivery;
#[cfg(feature = "hubrise")]
pub mod discount;
pub mod eft;
pub mod export;
pub mod fdm_monitor;
pub mod financial;
pub mod gift_vouchers;
#[cfg(feature = "hubrise")]
pub mod hubrise;
#[cfg(feature = "hubrise")]
pub mod integrations;
pub mod invoice;
pub mod menu;
pub mod menu_admin;
#[cfg(feature = "hubrise")]
pub mod modifiers;
pub mod order;
pub mod print;
pub mod refund;
pub mod report;
pub mod scale;
pub mod session;
pub mod settings;
pub mod social;
pub mod support;
pub mod tables;
pub mod unpaid;
#[cfg(feature = "hubrise")]
pub mod webshop;
