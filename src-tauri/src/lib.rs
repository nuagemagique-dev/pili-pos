//! Library facade for the moyun-pos binary.
//!
//! Tauri's `#[tauri::command]` lives in a binary crate, but integration
//! tests under `tests/` only link the library half of a Cargo package.
//! This file exposes the modules tests need (`commands`, `state`) so
//! the SPF e2e harness can drive the inner `do_*` functions added in
//! Phase 1 without spinning up the Tauri runtime.
//!
//! Production code (`main.rs`) keeps consuming these modules via the
//! crate name (`use moyun_pos::commands;` / `use moyun_pos::state;`),
//! producing the exact same binary as before.

pub mod cli;
pub mod commands;
pub mod paths;
pub mod state;
