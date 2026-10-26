//! The shell's half of the engine events: [`TauriEventSink`] hands each
//! [`EngineEvent`] to the webview under the topic the engine names.

use std::sync::OnceLock;

use pos_engine::{EngineEvent, EventSink};
use tauri::{AppHandle, Emitter};

/// Forwards the engine's events to the webview. The handle only exists
/// once Tauri's `setup` runs, so it is filled then ([`Self::set_handle`]);
/// the engine's loops start after that.
#[derive(Default)]
pub struct TauriEventSink {
    handle: OnceLock<AppHandle>,
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
        if let Err(e) = emitted {
            // The webview may be reloading; the UI re-reads on its next load.
            tracing::warn!(topic, %e, "event not delivered to the webview");
        }
    }
}
