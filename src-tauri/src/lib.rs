//! Library facade for the pili-pos binary.
//!
//! The fiscal core is the `pos-engine` crate (its integration tests live
//! in `crates/pos-engine/tests`); the Tauri-bound half (`ipc`, `paths`,
//! `cli`) stays here.

pub mod cli;
pub mod events;
pub mod ipc;
pub mod paths;
