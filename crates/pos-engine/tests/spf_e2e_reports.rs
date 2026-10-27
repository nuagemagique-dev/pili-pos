//! Application-level e2e tests for the SPF Finances Reports v1.3 set.
//!
//! Coverage:
//!
//! | UC | Pattern | Command |
//! |---|---|---|
//! | M180 | Turnover-X archive (no booking-date) | `report::do_generate_x_report` |
//! | M182 | User-X archive (per-operator block) | `report::do_generate_user_x_report` |
//! | M184 | Combined Z: Turnover-Z + User-Z in ONE GraphQL op, both archived atomically | `report::do_generate_z_report` |
//!
//! M181/M183 (the standalone Z mutations) are exercised through the
//! M184 flow - the POS always emits the end-of-day Z as the combined
//! operation, which carries both `signReportTurnoverZ` and
//! `signReportUserZ` mutations from those UCs.
//!
//! The "T1–T8 input staging" published by SPF is not replayed here.
//! Those fixtures pre-populate `fiscal_events` so the X/Z aggregate
//! maths have something to sum over; the aggregate maths are covered
//! by `pos-db::queries::reports::tests` (unit). Here a minimal
//! service day is seeded directly in SQLite so the reports carry
//! real per-user and FDM-device data.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use pos_engine_testkit as common;

use common::*;

use pos_engine::commands::report::{
    do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
};
use pos_fiscal::types::EventLabel;

const TEST_INSZ: &str = "12345678901";

/// Seed one operator's service day in the open booking period: a
/// work-in (S) and a cash sale (N), as archived after signing. Gives the
/// report builders real per-user aggregates and an archived FDM
/// reference range to read.
fn seed_service_day(app: &TestApp) {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::booking::current_period(
        &conn,
        "0b0e7c1e-0000-4000-8000-000000000001",
        "2026-07-03",
    )
    .unwrap();
    conn.execute_batch(
        r#"
        INSERT INTO fiscal_events (
            event_type, sequence_number, pos_id, terminal_id, user_insz,
            event_data_json, fdm_response_json
        ) VALUES (
            'S', 900, 'CPOS0031234567', 'POS-1-BAR', '12345678901',
            '{"posId":"CPOS0031234567","terminalId":"POS-1-BAR","employeeId":"12345678901","bookingPeriodId":"0b0e7c1e-0000-4000-8000-000000000001","posDateTime":"2026-07-03T09:00:00+02:00"}',
            '{"eventOperation":"WORK_IN","fdmRef":{"fdmId":"CBX0MOCK0001","fdmDateTime":"2026-07-03T09:00:01+02:00","eventLabel":"S","eventCounter":1,"totalCounter":41}}'
        );
        INSERT INTO fiscal_events (
            event_type, sequence_number, pos_id, terminal_id, user_insz,
            event_data_json, fdm_response_json
        ) VALUES (
            'N', 901, 'CPOS0031234567', 'POS-1-BAR', '12345678901',
            '{"posId":"CPOS0031234567","terminalId":"POS-1-BAR","employeeId":"12345678901","bookingPeriodId":"0b0e7c1e-0000-4000-8000-000000000001","posDateTime":"2026-07-03T12:00:00+02:00","transaction":{"transactionLines":[{"lineType":"SINGLE_PRODUCT","mainProduct":{"productId":"1","productName":"Menu","departmentId":"1","departmentName":"Plats","quantity":1,"vats":[{"label":"B","price":12.0}]},"lineTotal":12.0}],"transactionTotal":12.0},"financials":[{"id":"1","name":"Cash","type":"CASH","inputMethod":"MANUAL","amount":12.0,"amountType":"PAYMENT"}]}',
            '{"eventOperation":"SALE","fdmRef":{"fdmId":"CBX0MOCK0001","fdmDateTime":"2026-07-03T12:00:01+02:00","eventLabel":"N","eventCounter":2,"totalCounter":42},"vatCalc":[{"label":"B","rate":12,"taxableAmount":10.71,"vatAmount":1.29,"totalAmount":12.0,"outOfScope":false}]}'
        );
        "#,
    )
    .unwrap();
}

#[tokio::test]
async fn m180_generate_x_report_archives_r_event_without_resetting_counter() {
    let app = TestApp::new().await;
    seed_service_day(&app);
    mock_from_fixture(&app, &load_uc(&reports_root(), "M180")).await;

    let result = do_generate_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .expect("x-report returned");
    assert!(result.success, "M180 X-report failed: {:?}", result.error);
    assert_eq!(result.report_type, "X");

    let events = app.fiscal_events();
    let r_events: Vec<_> = events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(r_events.len(), 1);
    // X-reports do NOT carry a report number - only Z does.
    assert!(
        result.report_no.is_none(),
        "X-report must not assign a Z report number"
    );
    assert_eq!(app.counter_for(EventLabel::R), 1);

    // The fdmDevices block must carry the REAL device identity read
    // from the archived responses, not a placeholder.
    let request: serde_json::Value =
        serde_json::from_str(r_events[0].event_data_json.as_ref().unwrap()).unwrap();
    assert_eq!(
        request["fdmDevices"][0]["fdmId"], "CBX0MOCK0001",
        "X-report must embed the archived FDM identity"
    );
    assert_eq!(request["fdmDevices"][0]["firstTotalCounter"], 41);
    assert_eq!(request["fdmDevices"][0]["lastTotalCounter"], 42);
    app.dump_evidence("M180");
}

#[tokio::test]
async fn m182_generate_user_x_report_archives_r_event_with_user_block() {
    let app = TestApp::new().await;
    seed_service_day(&app);
    mock_from_fixture(&app, &load_uc(&reports_root(), "M182")).await;

    let result = do_generate_user_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .expect("user x-report returned");
    assert!(result.success, "M182 failed: {:?}", result.error);
    assert_eq!(result.report_type, "X-USER");
    assert!(result.report_no.is_none());

    let events = app.fiscal_events();
    let r_events: Vec<_> = events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(r_events.len(), 1);

    // The archived request must carry the per-operator block with the
    // seeded INSZ, its sales total, and the WORK_IN registration.
    let request: serde_json::Value =
        serde_json::from_str(r_events[0].event_data_json.as_ref().unwrap()).unwrap();
    let users = request["users"].as_array().expect("users array");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["employeeId"], TEST_INSZ);
    assert_eq!(users[0]["totalAmount"], 12.0);
    assert_eq!(users[0]["socialEvents"][0]["inOut"], "IN");
    assert_eq!(users[0]["payments"][0]["amount"][0]["totalAmount"], 12.0);
    app.dump_evidence("M182");
}

#[tokio::test]
async fn m184_generate_z_report_signs_both_reports_in_one_operation() {
    let app = TestApp::new().await;
    seed_service_day(&app);
    // The published M184 fixture response IS the combined envelope:
    // data.signReportTurnoverZ + data.signReportUserZ.
    mock_from_fixture(&app, &load_uc(&reports_root(), "M184")).await;

    let result = do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .expect("z-report returned");
    assert!(result.success, "M184 Z failed: {:?}", result.error);
    assert_eq!(result.report_type, "Z");
    assert_eq!(
        result.report_no,
        Some(1),
        "first Z of the day must carry report_no = 1"
    );

    // Exactly ONE HTTP round-trip to the FDM - both mutations travel
    // in the same GraphQL document.
    let requests = app.fdm.received_requests().await.unwrap();
    assert_eq!(
        requests.len(),
        1,
        "M184 must sign both Z reports in a single FDM call"
    );
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    let query = body["query"].as_str().unwrap();
    assert!(query.contains("signReportTurnoverZ"));
    assert!(query.contains("signReportUserZ"));

    // Both R events archived, with sequential ticket numbers.
    let events = app.fiscal_events();
    let r_events: Vec<_> = events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(r_events.len(), 2, "turnover Z + user Z must both archive");
    assert_eq!(r_events[0].sequence_number + 1, r_events[1].sequence_number);
    assert_eq!(app.counter_for(EventLabel::R), 2);

    let op = |e: &&FiscalEventRow| -> String {
        serde_json::from_str::<serde_json::Value>(e.fdm_response_json.as_ref().unwrap())
            .unwrap()["eventOperation"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert_eq!(op(&r_events[0]), "REPORT_TURNOVER_Z");
    assert_eq!(op(&r_events[1]), "REPORT_USER_Z");

    // The user Z request carries the per-operator block.
    let user_request: serde_json::Value =
        serde_json::from_str(r_events[1].event_data_json.as_ref().unwrap()).unwrap();
    assert_eq!(user_request["users"][0]["employeeId"], TEST_INSZ);
    assert_eq!(user_request["reportNo"], 1);

    // The z_reports row links back to the TURNOVER R event.
    let conn = app.state.db.lock().unwrap();
    let fiscal_event_id: i64 = conn
        .query_row(
            "SELECT fiscal_event_id FROM z_reports WHERE sequence_number = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        fiscal_event_id, r_events[0].id,
        "z_reports must link back to the turnover R fiscal event"
    );
    // Release the DB guard - dump_evidence re-locks state.db.
    drop(conn);
    app.dump_evidence("M184");
}

#[tokio::test]
async fn z_report_starts_new_booking_period() {
    // Per cert manual §IV.4, every Z-report MUST start a new
    // booking period so subsequent sales are aggregated under a
    // fresh window. The Z closes the open period; the next event
    // opens the next one.
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M184")).await;

    let open = |app: &TestApp| {
        pos_db::queries::booking::open_period(&app.state.db.lock().unwrap()).unwrap()
    };
    do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    assert_eq!(open(&app), None, "the Z closes the booking period");

    let z_request: serde_json::Value =
        serde_json::from_str(app.fiscal_events()[0].event_data_json.as_deref().unwrap()).unwrap();
    mock_from_fixture(&app, &load_uc(&basic_set_root(), "M140")).await;
    pos_engine::commands::social::do_work_in(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    let next = open(&app).expect("the next event opens a period");
    assert_ne!(
        Some(next.id.as_str()),
        z_request["bookingPeriodId"].as_str(),
        "events after the Z belong to a new booking period"
    );
}
