//! The event stream: engine events reach Kotlin through a broadcast channel and an
//! [`EventStream`] object whose `next()` is awaited in a coroutine.

use std::sync::Arc;

use pos_engine::{EngineEvent, EventSink};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

use crate::convert;
use crate::error::PosError;
use crate::records::common::{
    CallbackEvent, FdmHealth, PresenceFailed, SessionLocked, SignMessages,
};

/// What the UI is told outside a command call: one variant per engine topic, plus the stream's own
/// `Resync` marker (PD57). The payloads are the records of `records::common`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, uniffi::Enum)]
pub enum PosEvent {
    /// Topic `fdm-sign-messages`: the warnings and informations the FDM attached to a signed event.
    FdmSignMessages { messages: SignMessages },
    /// Topic `fdm-status`: the FDM status monitor checked the FDM.
    FdmStatus { health: FdmHealth },
    /// Topic `session-locked`: the session was locked, the UI goes to the login screen.
    SessionLocked { locked: SessionLocked },
    /// Topic `presence-failed`: the clock-in queued by a login was not registered.
    PresenceFailed { failure: PresenceFailed },
    /// Topic `webshop-order-received`: an online order arrived.
    WebshopOrderReceived { event: CallbackEvent },
    /// Topic `webshop-order-updated`: an online order changed.
    WebshopOrderUpdated { event: CallbackEvent },
    /// Topic `catalog-updated`: the menu was updated from the HubRise catalog.
    CatalogUpdated { event: CallbackEvent },
    /// Topic `catalog-rejected`: the catalog has issues, the menu stayed as it was.
    CatalogRejected { event: CallbackEvent },
    /// Topic `catalog-awaiting-adoption`: first sync over a hand-entered menu, a manager decides.
    CatalogAwaitingAdoption { event: CallbackEvent },
    /// The receiver lagged and `missed` events were dropped, or an event payload could not be
    /// converted: the UI must re-fetch its state.
    Resync { missed: i64 },
}

impl PosEvent {
    /// The engine's topic string of this event (`"resync"` for [`PosEvent::Resync`], which has no
    /// engine topic). For tests and logs; not exported.
    pub fn topic(&self) -> &'static str {
        match self {
            PosEvent::FdmSignMessages { .. } => "fdm-sign-messages",
            PosEvent::FdmStatus { .. } => "fdm-status",
            PosEvent::SessionLocked { .. } => "session-locked",
            PosEvent::PresenceFailed { .. } => "presence-failed",
            PosEvent::WebshopOrderReceived { .. } => "webshop-order-received",
            PosEvent::WebshopOrderUpdated { .. } => "webshop-order-updated",
            PosEvent::CatalogUpdated { .. } => "catalog-updated",
            PosEvent::CatalogRejected { .. } => "catalog-rejected",
            PosEvent::CatalogAwaitingAdoption { .. } => "catalog-awaiting-adoption",
            PosEvent::Resync { .. } => "resync",
        }
    }
}

/// Convert every payload; the first failure is the caller's to handle.
fn convert_event(event: EngineEvent) -> Result<PosEvent, PosError> {
    Ok(match event {
        EngineEvent::FdmSignMessages(m) => PosEvent::FdmSignMessages {
            messages: convert::out(&m)?,
        },
        EngineEvent::FdmStatus(h) => PosEvent::FdmStatus {
            health: convert::out(&h)?,
        },
        EngineEvent::SessionLocked(l) => PosEvent::SessionLocked {
            locked: convert::out(&l)?,
        },
        EngineEvent::PresenceFailed(f) => PosEvent::PresenceFailed {
            failure: convert::out(&f)?,
        },
        EngineEvent::WebshopOrderReceived(e) => PosEvent::WebshopOrderReceived {
            event: CallbackEvent::from_engine(&e)?,
        },
        EngineEvent::WebshopOrderUpdated(e) => PosEvent::WebshopOrderUpdated {
            event: CallbackEvent::from_engine(&e)?,
        },
        EngineEvent::CatalogUpdated(e) => PosEvent::CatalogUpdated {
            event: CallbackEvent::from_engine(&e)?,
        },
        EngineEvent::CatalogRejected(e) => PosEvent::CatalogRejected {
            event: CallbackEvent::from_engine(&e)?,
        },
        EngineEvent::CatalogAwaitingAdoption(e) => PosEvent::CatalogAwaitingAdoption {
            event: CallbackEvent::from_engine(&e)?,
        },
    })
}

/// The FFI form of an engine event. A payload that fails conversion is logged and replaced by
/// `Resync { missed: 1 }` (PD57), so the UI re-fetches instead of missing the change silently.
pub fn from_engine(event: EngineEvent) -> PosEvent {
    let topic = event.topic();
    or_resync(topic, convert_event(event))
}

/// A converted event, or the resync that stands for one that could not be converted.
fn or_resync(topic: &str, converted: Result<PosEvent, PosError>) -> PosEvent {
    match converted {
        Ok(event) => event,
        Err(err) => {
            tracing::error!(topic, error = %err, "event payload not convertible: asking for a resync");
            PosEvent::Resync { missed: 1 }
        }
    }
}

/// The engine's [`EventSink`]: converts and broadcasts. It never blocks.
pub struct FfiSink {
    pub(crate) tx: broadcast::Sender<PosEvent>,
}

impl FfiSink {
    /// A sink on a channel of its own with one subscriber already on it. For tests outside this
    /// crate that plug the sink into an engine state they build themselves.
    #[doc(hidden)]
    pub fn with_stream(capacity: usize) -> (Arc<FfiSink>, Arc<EventStream>) {
        let (tx, rx) = broadcast::channel(capacity);
        (Arc::new(FfiSink { tx }), EventStream::new(rx))
    }
}

impl EventSink for FfiSink {
    fn emit(&self, event: EngineEvent) {
        // No receiver is not an error: nobody is listening yet.
        let _ = self.tx.send(from_engine(event));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_conversion_failure_becomes_a_resync() {
        let failed = Err(PosError::internal("contract: SignMessages: broken"));
        assert_eq!(
            or_resync("fdm-sign-messages", failed),
            PosEvent::Resync { missed: 1 }
        );
    }

    #[test]
    fn a_converted_event_is_kept() {
        let kept = or_resync("x", Ok(PosEvent::Resync { missed: 9 }));
        assert_eq!(kept, PosEvent::Resync { missed: 9 });
    }
}
