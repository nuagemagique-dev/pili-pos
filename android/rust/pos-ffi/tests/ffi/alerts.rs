//! Tests of the owner alerts facade (engine module alerts): `get_owner_alerts`,
//! `set_owner_alerts`, `send_test_alert`, `list_owner_alert_log`.
//!
//! (a) Contract: the FFI records are the engine's field by field; an unknown key is a contract
//! error; the bot token is in no record.
//!
//! (b) Wiring without a token: managers only, the defaults are off, a bad value names its field,
//! the test message answers `alerts_token_missing`.
//!
//! (c) Wiring with a token and a local mock of the Bot API (never the real Telegram): the test
//! message arrives and is logged.

use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use pos_engine::commands::alerts as engine;
use pos_ffi::records::alerts::{
    AlertKinds, AlertLogRow, AlertTestResult, OwnerAlerts, OwnerAlertsInput,
};
use pos_ffi::records::common::{contract_inp, contract_out};
use pos_ffi::{PosEngine, PosError};
use serde_json::{Value, json};

use crate::support::{MANAGER_INSZ, MANAGER_PIN, options_for};

fn coded(err: &PosError) -> (&str, &std::collections::HashMap<String, String>) {
    match err {
        PosError::Coded { code, params } => (code, params),
        other => panic!("expected a coded error, got {other:?}"),
    }
}

fn input() -> OwnerAlertsInput {
    OwnerAlertsInput {
        enabled: true,
        chat_id: "-1001234567890".into(),
        label: "Salle".into(),
        language: "fr".into(),
        kinds: AlertKinds {
            fdm: true,
            printer: true,
            terminal: false,
            scale: true,
            z_missing: true,
            online_waiting: true,
        },
        z_deadline_hour: 6,
        online_wait_minutes: 5,
        reminder_minutes: 60,
    }
}

// ───────────────────────────── (a) contract ─────────────────────────────

#[test]
fn the_records_are_the_engines_field_by_field() {
    let engine_input: engine::OwnerAlertsInput = contract_inp(&input()).unwrap();
    assert_eq!(engine_input.chat_id, "-1001234567890");
    assert!(!engine_input.kinds.terminal);
    let back: OwnerAlertsInput = contract_out(&engine_input).unwrap();
    assert_eq!(back, input());

    let alerts = OwnerAlerts {
        enabled: true,
        chat_id: "1".into(),
        label: String::new(),
        language: "zh".into(),
        kinds: input().kinds,
        z_deadline_hour: 6,
        online_wait_minutes: 5,
        reminder_minutes: 0,
        token_configured: true,
    };
    let engine_alerts: Value = serde_json::to_value(&alerts).unwrap();
    assert_eq!(engine_alerts["token_configured"], true);
    assert_eq!(engine_alerts["kinds"]["z_missing"], true);

    let result: AlertTestResult = contract_out(&engine::AlertTestResult {
        sent: true,
        detail: "ok".into(),
    })
    .unwrap();
    assert!(result.sent);

    let row: AlertLogRow = contract_out(&pos_db::queries::owner_settings::AlertLogRow {
        id: 3,
        kind: "z_missing".into(),
        transition: "raised".into(),
        detail: Some("booking_date=2026-10-16".into()),
        outcome: "sent".into(),
        http_status: Some(200),
        created_at: "2026-10-17 04:00:00".into(),
    })
    .unwrap();
    assert_eq!(row.http_status, Some(200));
}

#[test]
fn an_unknown_key_is_a_contract_error_and_no_record_holds_a_token() {
    let mut value = serde_json::to_value(input()).unwrap();
    value["surprise"] = json!(1);
    assert!(serde_json::from_value::<OwnerAlertsInput>(value).is_err());
    let mut kinds = serde_json::to_value(input().kinds).unwrap();
    kinds["surprise"] = json!(true);
    assert!(serde_json::from_value::<AlertKinds>(kinds).is_err());
    for record in [
        serde_json::to_string(&input()).unwrap(),
        serde_json::to_string(&AlertTestResult {
            sent: false,
            detail: "HTTP 400".into(),
        })
        .unwrap(),
    ] {
        assert!(!record.contains("token"), "{record}");
    }
}

// ───────────────────────────── (b) without a token ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn without_an_operator_or_a_manager_the_methods_refuse() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    assert_eq!(
        coded(&e.get_owner_alerts().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.set_owner_alerts(input()).await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.send_test_alert().await.unwrap_err()).0,
        "not_logged_in"
    );
    assert_eq!(
        coded(&e.list_owner_alert_log(10).await.unwrap_err()).0,
        "not_logged_in"
    );
    // A cashier is not a manager.
    e.login("75061189702".into(), Some("4444".into()))
        .await
        .unwrap();
    assert_eq!(
        coded(&e.get_owner_alerts().await.unwrap_err()).0,
        "forbidden"
    );
    assert_eq!(
        coded(&e.set_owner_alerts(input()).await.unwrap_err()).0,
        "forbidden"
    );
    e.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_settings_start_off_and_round_trip_for_a_manager() {
    let te = crate::support::open_seeded().await;
    let e = &te.engine;
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    let start = e.get_owner_alerts().await.unwrap();
    assert!(!start.enabled, "off by default");
    assert!(!start.token_configured, "pos.json has no alerts section");
    assert_eq!(start.language, "fr");
    assert_eq!(start.reminder_minutes, 60);
    assert_eq!(start.online_wait_minutes, 5);
    assert_eq!(start.z_deadline_hour, 6, "day_end_hour 5 + 1");
    assert!(start.kinds.fdm && start.kinds.online_waiting);

    let mut bad = input();
    bad.chat_id = "abc".into();
    let err = e.set_owner_alerts(bad).await.unwrap_err();
    let (code, params) = coded(&err);
    assert_eq!(code, "alerts_invalid");
    assert_eq!(params["field"], "chat_id");

    let saved = e.set_owner_alerts(input()).await.unwrap();
    assert!(saved.enabled);
    assert_eq!(saved.chat_id, "-1001234567890");
    assert!(!saved.kinds.terminal);
    assert_eq!(e.get_owner_alerts().await.unwrap(), saved);

    // No token file: nothing to send with.
    assert_eq!(
        coded(&e.send_test_alert().await.unwrap_err()).0,
        "alerts_token_missing"
    );
    assert_eq!(e.list_owner_alert_log(10).await.unwrap(), vec![]);
    e.shutdown().await.unwrap();
}

// ───────────────────────────── (c) with the Bot API mock ─────────────────────────────

type Hits = Arc<Mutex<Vec<(String, Value)>>>;

async fn bot_api() -> (String, Hits) {
    let hits: Hits = Arc::new(Mutex::new(Vec::new()));
    let app = Router::new()
        .route(
            "/{bot}/sendMessage",
            post(
                |State(hits): State<Hits>, Path(bot): Path<String>, Json(body): Json<Value>| async move {
                    hits.lock().unwrap().push((bot, body));
                    Json(json!({ "ok": true, "result": {} }))
                },
            ),
        )
        .with_state(hits.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (base, hits)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_test_message_reaches_the_bot_api_and_is_logged() {
    let (base, hits) = bot_api().await;
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(bundle.join("telegram.token"), "123:FFI-TOKEN\n").unwrap();
    // A seeded engine's own mocks, with an `alerts` section added to its pos.json.
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(fdm, pos_fdm_mock::app()).await.unwrap() });
    std::fs::write(
        bundle.join("pos.json"),
        format!(
            r#"{{
            "pos_id": "CPOS0031234567",
            "terminal_id": "POS-2-ANDROID",
            "device_id": "bundle-device",
            "restaurant_name": "Restaurant Test",
            "restaurant_address": "Rue de l'Exemple 1, 4000 Liege",
            "restaurant_vat": "BE0499999960",
            "restaurant_est_no": "2123456791",
            "fdm_url": "{fdm_url}",
            "table_count": 25,
            "alerts": {{ "telegram_bot_token_file": "telegram.token", "telegram_api_base": "{base}" }}
            }}"#
        ),
    )
    .unwrap();
    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed))
        .expect("seed the database");
    let e: Arc<PosEngine> =
        PosEngine::open(options_for(dir.path())).unwrap_or_else(|e| panic!("open: {e}"));
    e.login(MANAGER_INSZ.into(), Some(MANAGER_PIN.into()))
        .await
        .unwrap();

    let saved = e.set_owner_alerts(input()).await.unwrap();
    assert!(
        saved.token_configured,
        "the bundle's relative path resolves"
    );
    assert!(!serde_json::to_string(&saved).unwrap().contains("FFI-TOKEN"));

    let result = e.send_test_alert().await.unwrap();
    assert!(result.sent);
    let sent = hits.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "bot123:FFI-TOKEN");
    assert_eq!(sent[0].1["chat_id"], "-1001234567890");
    assert!(
        sent[0].1["text"]
            .as_str()
            .unwrap()
            .starts_with("TEST [Salle]")
    );

    let log = e.list_owner_alert_log(10).await.unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(
        (log[0].kind.as_str(), log[0].outcome.as_str()),
        ("test", "sent")
    );
    assert_eq!(log[0].http_status, Some(200));
    e.shutdown().await.unwrap();
}
