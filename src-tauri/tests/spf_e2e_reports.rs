//! Application-level e2e tests for the SPF Finances Reports v1.3 set.
//!
//! Coverage:
//!
//! | UC | Pattern | Command |
//! |---|---|---|
//! | M180 | Turnover-X archive (no booking-date) | `report::do_generate_x_report` |
//! | M181 | Turnover-Z archive + counter reset + DB backup write | `report::do_generate_z_report` |
//!
//! Currently NOT covered (but parsed by `crates/pos-fiscal/tests/reports_fixtures.rs`):
//!
//! - **M182 / M183** (User-X / User-Z): the production `report::*`
//!   commands don't emit User reports yet; they only emit the
//!   Turnover envelope. The cert plan (Phase 10) tracks this as a
//!   follow-up — adding `do_generate_user_z_report` is a small task,
//!   but until the IPC surface exists there's nothing to drive from
//!   the harness.
//! - **M184** (atomic Turnover-Z + User-Z co-emission): same reason —
//!   the orchestration doesn't exist in `commands::report` yet. Once
//!   it does, a sibling test here exercises the atomic-tx rollback
//!   path (kill the 2nd mutation → no rows persisted at all).
//!
//! The "T1–T8 input staging" published by SPF is also not replayed
//! here. Those fixtures pre-populate `fiscal_events` so the X/Z
//! aggregate maths have something to sum over. For the cert dossier
//! the maths are covered by `pos-db::queries::reports::tests` (unit);
//! the e2e checks the FDM round-trip + archive, which is independent
//! of the aggregate content.

mod common;

use common::*;

use moyun_pos::commands::report::{do_generate_x_report, do_generate_z_report};
use pos_fiscal::types::EventLabel;

const TEST_INSZ: &str = "12345678901";

#[tokio::test]
async fn m180_generate_x_report_archives_r_event_without_resetting_counter() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M180")).await;

    let result = do_generate_x_report(&app.state, TEST_INSZ.to_string())
        .await
        .expect("x-report returned");
    assert!(result.success, "M180 X-report failed: {:?}", result.error);
    assert_eq!(result.report_type, "X");

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "R");
    // X-reports do NOT carry a report number — only Z does.
    assert!(
        result.report_no.is_none(),
        "X-report must not assign a Z report number"
    );
    assert_eq!(app.counter_for(EventLabel::R), 1);
    app.dump_evidence("M180");
}

#[tokio::test]
async fn m181_generate_z_report_archives_r_event_with_z_report_record() {
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M181")).await;

    let result = do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .expect("z-report returned");
    assert!(result.success, "M181 Z-report failed: {:?}", result.error);
    assert_eq!(result.report_type, "Z");
    assert_eq!(
        result.report_no,
        Some(1),
        "first Z of the day must carry report_no = 1"
    );

    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "R");

    // Z-report record persisted alongside the fiscal event.
    let conn = app.state.db.lock().unwrap();
    let z_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM z_reports", [], |row| row.get(0))
        .unwrap();
    assert_eq!(z_count, 1, "exactly one z_reports row expected");
    let fiscal_event_id: i64 = conn
        .query_row(
            "SELECT fiscal_event_id FROM z_reports WHERE sequence_number = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        fiscal_event_id, events[0].id,
        "z_reports must link back to the R fiscal event"
    );
    // Release the DB guard — dump_evidence re-locks state.db.
    drop(conn);
    app.dump_evidence("M181");
}

#[tokio::test]
async fn z_report_starts_new_booking_period() {
    // Per cert manual §IV.4, every Z-report MUST start a new
    // booking period so subsequent sales are aggregated under a
    // fresh window. The booking-period UUID lives on AppState and
    // is rotated inside `do_generate_z_report`.
    let app = TestApp::new().await;
    mock_from_fixture(&app, &load_uc(&reports_root(), "M181")).await;

    let bp_before = app.state.booking_period_id.lock().unwrap().clone();
    do_generate_z_report(&app.state, TEST_INSZ.to_string())
        .await
        .unwrap();
    let bp_after = app.state.booking_period_id.lock().unwrap().clone();

    assert_ne!(
        bp_before, bp_after,
        "booking_period_id must rotate on Z-report"
    );
}
