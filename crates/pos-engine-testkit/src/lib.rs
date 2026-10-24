//! Shared e2e test harness for SPF Finances UC replay.
//!
//! Two responsibilities:
//!
//! 1. **TestApp** - an in-memory `AppState` paired with a wiremock
//!    server that pretends to be the FDM. Test code calls `TestApp::new()`
//!    then drives the Tauri command inner functions
//!    (`commands::checkout::do_checkout`, etc.) against the resulting
//!    state. Assertions are made against the SQLite DB.
//!
//! 2. **Fixture loading** - walks `docs/20260821_UC_BasicSet_POS_FDM_1.8/`
//!    and `docs/20260821_UC_Reports_1.3/`, returns `(uc_id, variables_path,
//!    response_path)` tuples plus helpers to read each. Loosely modelled
//!    on `crates/pos-fiscal/tests/uc_fixtures.rs::discover_fixtures` but
//!    parameterised on the root directory so it serves both UC sets.

// Each integration-test binary links this library but uses a
// different subset of it, so dead-code analysis misfires.
#![allow(dead_code)]

pub mod compare;
pub mod devices;
pub mod eft;
pub mod fdm_drop;
pub mod fixtures;
pub mod harness;
pub mod hubrise;
pub mod live;
pub mod live_hubrise;

#[allow(unused_imports)]
pub use compare::*;
#[allow(unused_imports)]
pub use devices::*;
#[allow(unused_imports)]
pub use eft::*;
#[allow(unused_imports)]
pub use fdm_drop::*;
#[allow(unused_imports)]
pub use fixtures::*;
#[allow(unused_imports)]
pub use harness::*;
#[allow(unused_imports)]
pub use live::*;
#[allow(unused_imports)]
pub use pos_eft;
#[allow(unused_imports)]
pub use pos_print::render::one_line;
