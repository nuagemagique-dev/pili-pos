//! Library facade for the pili-pos binary.
//!
//! Tauri's `#[tauri::command]` lives in a binary crate, but integration
//! tests under `tests/` only link the library half of a Cargo package.
//! The fiscal core is the `pos-engine` crate; the Tauri-bound half
//! (`ipc`, `paths`, `cli`) stays here.

pub mod cli;
pub mod ipc;
pub mod paths;

// Temporary: the integration tests still reach the engine as
// `pili_pos::{commands, engine, state}`. P1a-T6 removes these re-exports
// once the tests have moved to the engine crate.
pub use pos_engine::{commands, engine, state};
