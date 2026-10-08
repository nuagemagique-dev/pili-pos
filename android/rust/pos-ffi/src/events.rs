//! The event stream: engine events reach Kotlin through a broadcast channel and an
//! [`EventStream`] object whose `next()` is awaited in a coroutine.

use std::sync::Arc;

use pos_engine::{EngineEvent, EventSink};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

/// What the UI is told outside a command call. The topic variants come with the events card; for
/// now only the stream's own marker exists.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, uniffi::Enum)]
pub enum PosEvent {
    /// The receiver lagged and `missed` events were dropped: the UI must re-fetch its state.
    Resync { missed: i64 },
}

/// The FFI form of an engine event. `None`: not forwarded (yet).
pub(crate) fn from_engine(event: EngineEvent) -> Option<PosEvent> {
    tracing::trace!(topic = event.topic(), "engine event not forwarded yet");
    None
}

/// The engine's [`EventSink`]: converts and broadcasts. It never blocks.
pub(crate) struct FfiSink {
    pub(crate) tx: broadcast::Sender<PosEvent>,
}

impl EventSink for FfiSink {
    fn emit(&self, event: EngineEvent) {
        if let Some(event) = from_engine(event) {
            // No receiver is not an error: nobody is listening yet.
            let _ = self.tx.send(event);
        }
    }
}

/// One subscriber's view of the events. Kotlin: `EventStream.next()` (suspend) until it answers
/// `null`, which means the engine is gone.
#[derive(uniffi::Object)]
pub struct EventStream {
    rx: tokio::sync::Mutex<broadcast::Receiver<PosEvent>>,
}

impl EventStream {
    pub(crate) fn new(rx: broadcast::Receiver<PosEvent>) -> Arc<Self> {
        Arc::new(Self {
            rx: tokio::sync::Mutex::new(rx),
        })
    }
}

#[uniffi::export]
impl EventStream {
    /// The next event; `None` when the engine is gone (channel closed). On lag it answers
    /// `Resync { missed }`, after which the stream continues from the oldest retained event.
    pub async fn next(&self) -> Option<PosEvent> {
        let mut rx = self.rx.lock().await;
        match rx.recv().await {
            Ok(event) => Some(event),
            Err(RecvError::Lagged(missed)) => Some(PosEvent::Resync {
                missed: i64::try_from(missed).unwrap_or(i64::MAX),
            }),
            Err(RecvError::Closed) => None,
        }
    }
}

#[cfg(test)]
pub(crate) fn publish_for_tests(engine: &crate::core::PosEngine, event: PosEvent) {
    let tx = engine.events_tx_for_tests().expect("engine still open");
    let _ = tx.send(event);
}
