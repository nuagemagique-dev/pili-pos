//! The shell's half of the engine events: [`TauriEventSink`] hands each
//! [`EngineEvent`] to the webview under the topic the engine names.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock, PoisonError};

use pos_engine::{EngineEvent, EventSink};
use tauri::{AppHandle, Emitter};

/// Forwards the engine's events to the webview. The handle only exists
/// once Tauri's `setup` runs, so it is filled then ([`Self::set_handle`]);
/// the engine's loops start after that.
#[derive(Default)]
pub struct TauriEventSink {
    handle: OnceLock<AppHandle>,
    /// The topics whose last emit failed and was reported: a webview
    /// that reloads while the FDM status poller runs fails every tick,
    /// and one warning per outage is enough.
    failing: Mutex<HashSet<&'static str>>,
}

impl TauriEventSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// Give the sink the app handle it emits through. Set once; a
    /// second call keeps the first handle.
    pub fn set_handle(&self, handle: AppHandle) {
        let _ = self.handle.set(handle);
    }
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: EngineEvent) {
        let Some(handle) = self.handle.get() else {
            tracing::debug!(
                topic = event.topic(),
                "event before the window handle: dropped"
            );
            return;
        };
        let topic = event.topic();
        // Held across the emit: a failure and a success of the same topic
        // on two threads cannot then leave the topic marked as failing.
        let mut failing = self.failing.lock().unwrap_or_else(PoisonError::into_inner);
        // The payload types are the ones the UI has always received.
        let emitted = match event {
            EngineEvent::FdmSignMessages(messages) => handle.emit(topic, messages),
            EngineEvent::FdmStatus(health) => handle.emit(topic, health),
            EngineEvent::SessionLocked(e) => handle.emit(topic, e),
            EngineEvent::PresenceFailed(e) => handle.emit(topic, e),
            EngineEvent::WebshopOrderReceived(e)
            | EngineEvent::WebshopOrderUpdated(e)
            | EngineEvent::CatalogUpdated(e)
            | EngineEvent::CatalogRejected(e)
            | EngineEvent::CatalogAwaitingAdoption(e) => handle.emit(topic, e),
        };
        match emitted {
            // The webview may be reloading; the UI re-reads on its next
            // load. Warn on the first failure of a topic only.
            Err(e) if failing.insert(topic) => {
                tracing::warn!(topic, %e, "event not delivered to the webview");
            }
            Err(e) => tracing::debug!(topic, %e, "event not delivered to the webview"),
            Ok(()) => {
                failing.remove(topic);
            }
        }
    }
}
