//! Tauri-free engine of pili-pos.
//!
//! Everything the kiosk does that is not a window: opening the database,
//! signing events with the FDM, tickets, reports, HubRise, settings. The
//! Linux shell `src-tauri` wraps these functions as Tauri commands and,
//! from phase P2, the Android FFI `android/rust/pos-ffi` calls the same
//! functions. There is no platform conditional compilation here: platform differences
//! are [`EngineConfig`] inputs (paths, configuration source).

pub mod background;
pub mod commands;
pub mod engine;
pub mod events;
pub mod install;
pub mod state;

pub use engine::{ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers};
pub use events::{EngineEvent, EventSink, NullSink};
pub use pos_fiscal::http_tls::{HttpTls, HttpTlsError};
pub use pos_fiscal::identity::{IdentityError, SoftwareIdentity};
