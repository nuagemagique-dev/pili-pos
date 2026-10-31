//! Event tests: one per `PosEvent` variant, the conversion failure path, and the real sink.
//!
//! `events::from_engine` maps an `EngineEvent` to a `PosEvent` (PD57). Each variant test builds the
//! engine event, converts it and compares the payload's serde form with the engine's. The sink
//! tests plug the crate's `FfiSink` into a `TestApp` and run real engine commands.

use pos_engine::EngineEvent;
use pos_engine::commands::fdm_monitor::{
    FdmHealth as EngineFdmHealth, FdmSeverity, do_fdm_check_now,
};
use pos_engine::commands::session::{
    LockReason as EngineLockReason, SessionLocked as EngineSessionLocked, do_lock_session,
};
use pos_engine::commands::social::PresenceFailed as EnginePresenceFailed;
use pos_engine_testkit::TestApp;
use pos_ffi::events::{FfiSink, from_engine};
use pos_ffi::records::common::{self as ffi, CallbackEvent};
use pos_ffi::{EventStream, PosEvent};
use pos_fiscal::fdm_client::SignMessages as EngineSignMessages;
use pos_fiscal::response::{ExtensionItem, LocationItem, MessageItem};
use pos_fiscal::types::{FdmWarningCode, MessageCategory, ShowPos};
use serde::Serialize;
use serde_json::{Value, json, to_value};

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

fn sign_messages() -> EngineSignMessages {
    let item = |full: bool| MessageItem {
        message: "Buffer almost full".into(),
        locations: full.then(|| vec![LocationItem { line: 1, column: 2 }]),
        extensions: ExtensionItem {
            category: MessageCategory::Fdm,
            code: FdmWarningCode::BufferNearFull,
            data: None,
            show_pos: ShowPos::Optional,
        },
    };
    EngineSignMessages {
        mutation: "signSale".into(),
        booking_date: Some("2026-10-08".into()),
        warnings: vec![item(true)],
        informations: vec![item(false)],
    }
}

fn health() -> EngineFdmHealth {
    EngineFdmHealth {
        connected: false,
        answered: true,
        severity: FdmSeverity::Blocking,
        code: Some("BUFFER_FULL".into()),
        mandatory: true,
        checked_at: "2026-10-08T10:00:00.000Z".into(),
        next_check_at: None,
    }
}

fn callback() -> pos_hubrise_types::CallbackEvent {
    let Value::Object(extra) = json!({"location_id": "loc9", "order_id": "ord7"}) else {
        unreachable!()
    };
    pos_hubrise_types::CallbackEvent {
        id: "ev_1".into(),
        resource_type: "order".into(),
        event_type: "create".into(),
        created_at: "2026-10-08T10:00:00+02:00".into(),
        previous_state: None,
        new_state: Some(json!({"status": "new"})),
        extra,
    }
}

/// The record the event must carry for an engine callback, checked field by field and through
/// the engine's own wire form.
fn assert_callback(record: &CallbackEvent, engine: &pos_hubrise_types::CallbackEvent) {
    assert_eq!(record.id, engine.id);
    assert_eq!(record.resource_type, engine.resource_type);
    assert_eq!(record.event_type, engine.event_type);
    assert_eq!(record.created_at, engine.created_at);
    assert_eq!(
        record
            .previous_state
            .as_deref()
            .map(|t| serde_json::from_str::<Value>(t).unwrap()),
        engine.previous_state
    );
    assert_eq!(
        record
            .new_state
            .as_deref()
            .map(|t| serde_json::from_str::<Value>(t).unwrap()),
        engine.new_state
    );
    assert_eq!(
        serde_json::from_str::<Value>(&record.extra).unwrap(),
        Value::Object(engine.extra.clone())
    );
    assert_eq!(json_of(&record.to_engine().unwrap()), json_of(engine));
}

#[test]
fn fdm_sign_messages() {
    let payload = sign_messages();
    let event = from_engine(EngineEvent::FdmSignMessages(payload.clone()));
    assert_eq!(event.topic(), "fdm-sign-messages");
    let PosEvent::FdmSignMessages { messages } = event else {
        panic!("an fdm sign messages event");
    };
    assert_eq!(json_of(&messages), json_of(&payload));
    assert_eq!(
        messages.warnings[0].extensions.code,
        ffi::FdmWarningCode::BufferNearFull
    );
}

#[test]
fn fdm_status() {
    let payload = health();
    let event = from_engine(EngineEvent::FdmStatus(payload.clone()));
    assert_eq!(event.topic(), "fdm-status");
    let PosEvent::FdmStatus { health } = event else {
        panic!("an fdm status event");
    };
    assert_eq!(json_of(&health), json_of(&payload));
    assert_eq!(health.severity, ffi::FdmSeverity::Blocking);
}

#[test]
fn session_locked() {
    let payload = EngineSessionLocked {
        reason: EngineLockReason::Idle,
        order_error: Some("fdm_unavailable".into()),
    };
    let event = from_engine(EngineEvent::SessionLocked(payload.clone()));
    assert_eq!(event.topic(), "session-locked");
    let PosEvent::SessionLocked { locked } = event else {
        panic!("a session locked event");
    };
    assert_eq!(json_of(&locked), json_of(&payload));
    assert_eq!(locked.reason, ffi::LockReason::Idle);
}

#[test]
fn presence_failed() {
    let payload = EnginePresenceFailed {
        insz: "12345678958".into(),
        clock_in: true,
        error: "FDM error".into(),
    };
    let event = from_engine(EngineEvent::PresenceFailed(payload.clone()));
    assert_eq!(event.topic(), "presence-failed");
    let PosEvent::PresenceFailed { failure } = event else {
        panic!("a presence failed event");
    };
    assert_eq!(json_of(&failure), json_of(&payload));
}

#[test]
fn webshop_order_received() {
    let payload = callback();
    let event = from_engine(EngineEvent::WebshopOrderReceived(payload.clone()));
    assert_eq!(event.topic(), "webshop-order-received");
    let PosEvent::WebshopOrderReceived { event } = event else {
        panic!("a webshop order received event");
    };
    assert_callback(&event, &payload);
}

#[test]
fn webshop_order_updated() {
    let payload = callback();
    let event = from_engine(EngineEvent::WebshopOrderUpdated(payload.clone()));
    assert_eq!(event.topic(), "webshop-order-updated");
    let PosEvent::WebshopOrderUpdated { event } = event else {
        panic!("a webshop order updated event");
    };
    assert_callback(&event, &payload);
}

#[test]
fn catalog_updated() {
    let payload = callback();
    let event = from_engine(EngineEvent::CatalogUpdated(payload.clone()));
    assert_eq!(event.topic(), "catalog-updated");
    let PosEvent::CatalogUpdated { event } = event else {
        panic!("a catalog updated event");
    };
    assert_callback(&event, &payload);
}

#[test]
fn catalog_rejected() {
    let payload = callback();
    let event = from_engine(EngineEvent::CatalogRejected(payload.clone()));
    assert_eq!(event.topic(), "catalog-rejected");
    let PosEvent::CatalogRejected { event } = event else {
        panic!("a catalog rejected event");
    };
    assert_callback(&event, &payload);
}

#[test]
fn catalog_awaiting_adoption() {
    let payload = callback();
    let event = from_engine(EngineEvent::CatalogAwaitingAdoption(payload.clone()));
    assert_eq!(event.topic(), "catalog-awaiting-adoption");
    let PosEvent::CatalogAwaitingAdoption { event } = event else {
        panic!("a catalog awaiting adoption event");
    };
    assert_callback(&event, &payload);
}

#[test]
fn resync_has_its_own_topic() {
    assert_eq!(PosEvent::Resync { missed: 3 }.topic(), "resync");
}

#[test]
fn every_engine_topic_survives_the_conversion() {
    let all = [
        EngineEvent::FdmSignMessages(sign_messages()),
        EngineEvent::FdmStatus(health()),
        EngineEvent::SessionLocked(EngineSessionLocked {
            reason: EngineLockReason::Manual,
            order_error: None,
        }),
        EngineEvent::PresenceFailed(EnginePresenceFailed {
            insz: "1".into(),
            clock_in: true,
            error: "e".into(),
        }),
        EngineEvent::WebshopOrderReceived(callback()),
        EngineEvent::WebshopOrderUpdated(callback()),
        EngineEvent::CatalogUpdated(callback()),
        EngineEvent::CatalogRejected(callback()),
        EngineEvent::CatalogAwaitingAdoption(callback()),
    ];
    assert_eq!(all.len(), 9, "the nine topics of the engine");
    for event in all {
        let topic = event.topic();
        assert_eq!(from_engine(event).topic(), topic);
    }
}

// ───────────────────────────── the sink and the stream ─────────────────────────────

async fn next(stream: &EventStream) -> PosEvent {
    tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
        .await
        .expect("an event within 5 s")
        .expect("the stream is open")
}

#[tokio::test(flavor = "multi_thread")]
async fn the_sink_delivers_converted_events_in_order() {
    use pos_engine::EventSink;
    let (sink, stream) = FfiSink::with_stream(16);
    sink.emit(EngineEvent::FdmSignMessages(sign_messages()));
    sink.emit(EngineEvent::FdmStatus(health()));
    sink.emit(EngineEvent::CatalogUpdated(callback()));
    sink.emit(EngineEvent::PresenceFailed(EnginePresenceFailed {
        insz: "1".into(),
        clock_in: true,
        error: "e".into(),
    }));
    assert_eq!(next(&stream).await.topic(), "fdm-sign-messages");
    assert_eq!(next(&stream).await.topic(), "fdm-status");
    assert_eq!(next(&stream).await.topic(), "catalog-updated");
    assert_eq!(next(&stream).await.topic(), "presence-failed");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_slow_reader_of_the_sink_gets_a_resync() {
    use pos_engine::EventSink;
    let (sink, stream) = FfiSink::with_stream(2);
    for _ in 0..5 {
        sink.emit(EngineEvent::FdmStatus(health()));
    }
    assert_eq!(next(&stream).await, PosEvent::Resync { missed: 3 });
    assert_eq!(next(&stream).await.topic(), "fdm-status");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_real_fdm_check_reaches_the_stream_as_fdm_status() {
    let mut app = TestApp::new().await;
    app.use_admission_mock().await;
    let (sink, stream) = FfiSink::with_stream(16);
    app.state.events = sink;

    let checked = do_fdm_check_now(&app.state).await.unwrap();
    let PosEvent::FdmStatus { health } = next(&stream).await else {
        panic!("an fdm status event");
    };
    assert_eq!(json_of(&health), json_of(&checked));
    assert!(health.connected);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_real_lock_reaches_the_stream_as_session_locked() {
    let mut app = TestApp::new().await;
    let (sink, stream) = FfiSink::with_stream(16);
    app.state.events = sink;

    let outcome = do_lock_session(&app.state).await.unwrap();
    assert_eq!(outcome.order_error, None);
    let PosEvent::SessionLocked { locked } = next(&stream).await else {
        panic!("a session locked event");
    };
    assert_eq!(locked.reason, ffi::LockReason::Manual);
    assert_eq!(locked.order_error, None);
}
