//! HubRise client for tsb-pos.
//!
//! Single "cloud" crate encapsulating all HubRise traffic for the
//! Rust/Tauri POS: OAuth connection, 30-second event polling,
//! dispatcher for order/catalog events, and PATCH status updates.
//!
//! Strictly isolated from `pos-fiscal` — dine-in orders never reach
//! this crate. See `plans/witty-sleeping-nygaard.md` for rationale.

pub mod catalog_apply;
pub mod catalog_mapper;
pub mod client;
pub mod dispatcher;
pub mod errors;
pub mod oauth;
pub mod outbox;
pub mod poller;
pub mod push;
pub mod token_store;

pub use client::Client;
pub use errors::HubriseError;
pub use oauth::OAuthConfig;
pub use token_store::StoredConnection;
