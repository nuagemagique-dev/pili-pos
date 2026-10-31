//! The installation's `HubRise` switch. The integration ships in every
//! build; this says whether it runs. Off, nothing reaches `HubRise`: the
//! poller and the outbox pass idle, and no connection is loaded.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shared by the app state and the background workers.
#[derive(Debug, Clone, Default)]
pub struct Switch(Arc<AtomicBool>);

impl Switch {
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "a one-flag switch: the bool is its whole API, shared with the engine"
    )]
    pub fn new(on: bool) -> Self {
        Self(Arc::new(AtomicBool::new(on)))
    }

    pub fn is_on(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "a one-flag switch: the bool is its whole API, shared with the engine"
    )]
    pub fn set(&self, on: bool) {
        self.0.store(on, Ordering::SeqCst);
    }
}
