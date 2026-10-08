//! pili-pos Android FFI: the UniFFI facade over `pos-engine`.
//!
//! One [`PosEngine`] object owns the engine and its own Tokio runtime. Every method is async and
//! awaits a join handle from that runtime (never `async_runtime = "tokio"`), errors are the
//! [`PosError`] enum, events come from an [`EventStream`]. The shape is the one proven by spike S2
//! (`docs/android/spikes/S2.md`); the domain cards add the facades under `api/` and the records
//! under `records/` without touching `Cargo.toml` or `Cargo.lock` (PD59).
//!
//! Platform code and process-wide state (logger, panic hook) live in `logging.rs` only.

uniffi::setup_scaffolding!();

pub mod api;
pub mod convert;
pub mod core;
pub mod error;
pub mod events;
pub mod helpers;
pub mod install;
pub mod logging;
pub mod records;

pub use crate::core::{EngineOptions, EngineTimersMs, PosEngine, pos_sw_version};
pub use crate::error::PosError;
pub use crate::events::{EventStream, PosEvent};
pub use crate::logging::init_logging;
