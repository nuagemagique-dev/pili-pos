//! What the engine tells its shell: [`EngineEvent`]s handed to an
//! [`EventSink`] the shell provides in [`crate::EngineConfig`].
//!
//! The Linux shell forwards each event to the webview under its
//! [`EngineEvent::topic`]; the Android FFI (phase P2) pulls them
//! instead. The topics and payloads are the ones the UI has always
//! listened to.

#[cfg(feature = "hubrise")]
use pos_hubrise::dispatcher::DispatchOutcome;
#[cfg(feature = "hubrise")]
use pos_hubrise_types::CallbackEvent;

/// An event the engine raises on its own, outside any command call.
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// The warnings and informations the FDM attached to a signed
    /// event (POS description v2.2, Message).
    FdmSignMessages(pos_fiscal::fdm_client::SignMessages),
    /// The FDM status monitor checked the FDM (`commands::fdm_monitor`,
    /// planner decision PD36): its classified health.
    FdmStatus(crate::commands::fdm_monitor::FdmHealth),
    /// The session was locked (`commands::session`): the UI goes to the
    /// login screen.
    SessionLocked(crate::commands::session::SessionLocked),
    /// The clock-in queued by a login was not registered
    /// (`commands::social`): the UI shows the presence toast.
    PresenceFailed(crate::commands::social::PresenceFailed),
    /// An online order arrived.
    #[cfg(feature = "hubrise")]
    WebshopOrderReceived(CallbackEvent),
    /// An online order changed.
    #[cfg(feature = "hubrise")]
    WebshopOrderUpdated(CallbackEvent),
    /// The menu was updated from the HubRise catalog.
    #[cfg(feature = "hubrise")]
    CatalogUpdated(CallbackEvent),
    /// The catalog has issues; the menu stayed as it was.
    #[cfg(feature = "hubrise")]
    CatalogRejected(CallbackEvent),
    /// First sync over a hand-entered menu: waiting for a manager.
    #[cfg(feature = "hubrise")]
    CatalogAwaitingAdoption(CallbackEvent),
}

impl EngineEvent {
    /// The name the UI listens to.
    pub fn topic(&self) -> &'static str {
        match self {
            EngineEvent::FdmSignMessages(_) => "fdm-sign-messages",
            EngineEvent::FdmStatus(_) => "fdm-status",
            EngineEvent::SessionLocked(_) => "session-locked",
            EngineEvent::PresenceFailed(_) => "presence-failed",
            #[cfg(feature = "hubrise")]
            EngineEvent::WebshopOrderReceived(_) => "webshop-order-received",
            #[cfg(feature = "hubrise")]
            EngineEvent::WebshopOrderUpdated(_) => "webshop-order-updated",
            #[cfg(feature = "hubrise")]
            EngineEvent::CatalogUpdated(_) => "catalog-updated",
            #[cfg(feature = "hubrise")]
            EngineEvent::CatalogRejected(_) => "catalog-rejected",
            #[cfg(feature = "hubrise")]
            EngineEvent::CatalogAwaitingAdoption(_) => "catalog-awaiting-adoption",
        }
    }
}

/// The event the HubRise poller's outcome stands for; `None` for an
/// outcome the UI is not told about (`Ignored`).
#[cfg(feature = "hubrise")]
pub fn event_for_outcome(outcome: DispatchOutcome, event: CallbackEvent) -> Option<EngineEvent> {
    match outcome {
        DispatchOutcome::OrderCreated => Some(EngineEvent::WebshopOrderReceived(event)),
        DispatchOutcome::OrderUpdated => Some(EngineEvent::WebshopOrderUpdated(event)),
        DispatchOutcome::CatalogUpdated => Some(EngineEvent::CatalogUpdated(event)),
        DispatchOutcome::CatalogRejected => Some(EngineEvent::CatalogRejected(event)),
        DispatchOutcome::CatalogAwaitingAdoption => {
            Some(EngineEvent::CatalogAwaitingAdoption(event))
        }
        DispatchOutcome::Ignored => None,
    }
}

/// Where the engine sends its events. Called from the background tasks
/// and from the FDM client, so it must not block.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: EngineEvent);
}

/// A sink that drops every event (tests, headless runs).
#[derive(Debug, Clone, Copy, Default)]
pub struct NullSink;

impl EventSink for NullSink {
    fn emit(&self, _event: EngineEvent) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign_messages() -> pos_fiscal::fdm_client::SignMessages {
        pos_fiscal::fdm_client::SignMessages {
            mutation: "signOrder".to_string(),
            booking_date: None,
            warnings: vec![],
            informations: vec![],
        }
    }

    #[cfg(feature = "hubrise")]
    fn callback() -> CallbackEvent {
        CallbackEvent {
            id: "ev1".to_string(),
            resource_type: "order".to_string(),
            event_type: "create".to_string(),
            created_at: "2026-01-01T00:00:00+01:00".to_string(),
            previous_state: None,
            new_state: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn the_sign_messages_topic() {
        assert_eq!(
            EngineEvent::FdmSignMessages(sign_messages()).topic(),
            "fdm-sign-messages"
        );
    }

    #[test]
    fn the_fdm_status_topic_and_payload() {
        let event = EngineEvent::FdmStatus(crate::commands::fdm_monitor::FdmHealth {
            connected: false,
            answered: true,
            severity: crate::commands::fdm_monitor::FdmSeverity::Blocking,
            code: Some("BUFFER_FULL".to_string()),
            mandatory: true,
            checked_at: "2026-10-07T10:00:00.000Z".to_string(),
            next_check_at: None,
        });
        assert_eq!(event.topic(), "fdm-status");
        let EngineEvent::FdmStatus(payload) = event else {
            panic!("an fdm status event");
        };
        assert_eq!(
            serde_json::to_value(&payload).unwrap(),
            serde_json::json!({
                "connected": false,
                "answered": true,
                "severity": "blocking",
                "code": "BUFFER_FULL",
                "mandatory": true,
                "checkedAt": "2026-10-07T10:00:00.000Z",
                "nextCheckAt": null
            })
        );
    }

    #[test]
    fn the_session_locked_topic_and_payload() {
        let event = EngineEvent::SessionLocked(crate::commands::session::SessionLocked {
            reason: crate::commands::session::LockReason::Idle,
            order_error: None,
        });
        assert_eq!(event.topic(), "session-locked");
        let EngineEvent::SessionLocked(payload) = event else {
            panic!("a session locked event");
        };
        assert_eq!(
            serde_json::to_value(&payload).unwrap(),
            serde_json::json!({ "reason": "idle", "orderError": null })
        );
    }

    #[test]
    fn the_presence_failed_topic_and_payload() {
        let event = EngineEvent::PresenceFailed(crate::commands::social::PresenceFailed {
            insz: "12345678901".to_string(),
            clock_in: true,
            error: "FDM error".to_string(),
        });
        assert_eq!(event.topic(), "presence-failed");
        let EngineEvent::PresenceFailed(payload) = event else {
            panic!("a presence failed event");
        };
        assert_eq!(
            serde_json::to_value(&payload).unwrap(),
            serde_json::json!({
                "insz": "12345678901",
                "clockIn": true,
                "error": "FDM error"
            })
        );
    }

    #[cfg(feature = "hubrise")]
    #[test]
    fn the_six_topics_are_the_ones_the_ui_listens_to() {
        let all = [
            EngineEvent::FdmSignMessages(sign_messages()),
            EngineEvent::WebshopOrderReceived(callback()),
            EngineEvent::WebshopOrderUpdated(callback()),
            EngineEvent::CatalogUpdated(callback()),
            EngineEvent::CatalogRejected(callback()),
            EngineEvent::CatalogAwaitingAdoption(callback()),
        ];
        let topics: Vec<&str> = all.iter().map(EngineEvent::topic).collect();
        assert_eq!(
            topics,
            [
                "fdm-sign-messages",
                "webshop-order-received",
                "webshop-order-updated",
                "catalog-updated",
                "catalog-rejected",
                "catalog-awaiting-adoption",
            ]
        );
    }

    #[cfg(feature = "hubrise")]
    #[test]
    fn each_outcome_maps_to_its_topic() {
        let cases = [
            (DispatchOutcome::OrderCreated, "webshop-order-received"),
            (DispatchOutcome::OrderUpdated, "webshop-order-updated"),
            (DispatchOutcome::CatalogUpdated, "catalog-updated"),
            (DispatchOutcome::CatalogRejected, "catalog-rejected"),
            (
                DispatchOutcome::CatalogAwaitingAdoption,
                "catalog-awaiting-adoption",
            ),
        ];
        for (outcome, topic) in cases {
            let event = event_for_outcome(outcome.clone(), callback())
                .unwrap_or_else(|| panic!("{outcome:?} must emit"));
            assert_eq!(event.topic(), topic);
        }
    }

    #[cfg(feature = "hubrise")]
    #[test]
    fn the_event_carries_the_callback_payload() {
        let Some(EngineEvent::WebshopOrderReceived(ev)) =
            event_for_outcome(DispatchOutcome::OrderCreated, callback())
        else {
            panic!("an order created event");
        };
        assert_eq!(ev.id, "ev1");
    }

    #[cfg(feature = "hubrise")]
    #[test]
    fn an_ignored_outcome_emits_nothing() {
        assert!(event_for_outcome(DispatchOutcome::Ignored, callback()).is_none());
    }

    #[test]
    fn the_null_sink_takes_any_event() {
        NullSink.emit(EngineEvent::FdmSignMessages(sign_messages()));
    }
}
