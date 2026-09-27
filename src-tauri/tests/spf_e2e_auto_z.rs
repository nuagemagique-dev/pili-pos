//! Producers FAQ Q501c: "En cas de dépassement, la caisse créera
//! automatiquement un event R par période de booking date manquante."
//!
//! - a business day left without its Z is closed by an automatic Z,
//!   signed in the name of the system identity (`00000000029`);
//! - until that Z is made, no new event goes into the old day;
//! - a day that is not over, a POS in training mode and a POS with
//!   nothing recorded make no automatic Z;
//! - an FDM failure leaves the day open, to be retried.
//!
//! Events are signed by the admission image's mock FDM.

mod common;

use common::*;

use pili_pos::commands::checkout::{CheckoutPaymentMethod, do_checkout};
use pili_pos::commands::errors;
use pili_pos::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pili_pos::commands::report::close_overdue_day;
use pos_core::order::OrderType;
use pos_core::user::SYSTEM_INSZ;

const OPERATOR: &str = "12345678958";

async fn app() -> TestApp {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    let conn = app.state.db.lock().unwrap();
    conn.execute_batch(
        "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, sort_order, active) VALUES
            (1, 1, 1200, 'A', 0, 1);",
    )
    .unwrap();
    drop(conn);
    app
}

async fn sell(app: &TestApp) -> Result<bool, String> {
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::Takeaway,
            table_number: None,
            user_insz: OPERATOR.into(),
        },
    )?
    .id;
    do_add_order_line(
        &app.state,
        AddOrderLineInput {
            order_id: order,
            menu_item_id: 1,
            quantity: 1,
            modifiers_json: None,
            unit_price_cents: None,
        },
    )?;
    do_checkout(&app.state, order, CheckoutPaymentMethod::Card)
        .await
        .map(|r| r.success)
}

/// Move the open period `days` business days back, as if the POS had
/// been left on since then without a Z. Counted from the business day,
/// which runs until the day end hour (05:00): between midnight and
/// 05:00 today's business day is still yesterday's date.
fn age_open_period(app: &TestApp, days: i64) -> String {
    let day_end = i64::from(app.state.config.lock().unwrap().day_end_hour);
    let date = (pos_fiscal::event::now_in_belgium()
        - chrono::Duration::hours(day_end)
        - chrono::Duration::days(days))
    .date_naive()
    .to_string();
    app.state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE booking_periods SET booking_date = ?1 WHERE closed_at IS NULL",
            rusqlite::params![date],
        )
        .unwrap();
    date
}

fn z_reports(app: &TestApp) -> Vec<(String, i64)> {
    let conn = app.state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT report_date, sequence_number FROM z_reports ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn open_period_date(app: &TestApp) -> Option<String> {
    let conn = app.state.db.lock().unwrap();
    pos_db::queries::booking::open_period(&conn)
        .unwrap()
        .map(|p| p.booking_date)
}

#[tokio::test]
async fn faq_q501_a_day_left_open_is_closed_by_an_automatic_z() {
    let app = app().await;
    assert!(sell(&app).await.unwrap());
    let day = age_open_period(&app, 2);

    assert!(close_overdue_day(&app.state).await.unwrap());

    assert_eq!(z_reports(&app), [(day, 1)]);
    assert_eq!(open_period_date(&app), None, "the old day is closed");
    let events = app.fiscal_events();
    let reports: Vec<_> = events.iter().filter(|e| e.event_type == "R").collect();
    assert_eq!(reports.len(), 2, "turnover Z and user Z (M184)");
    assert!(reports.iter().all(|e| e.user_insz == SYSTEM_INSZ));

    // The next sale opens today's period.
    assert!(sell(&app).await.unwrap());
    let today = pos_fiscal::event::now_in_belgium().date_naive().to_string();
    let hour = pos_fiscal::event::now_in_belgium().format("%H").to_string();
    // Before the day end hour (05:00) the business day is yesterday.
    if hour.as_str() >= "05" {
        assert_eq!(open_period_date(&app), Some(today));
    }
}

#[tokio::test]
async fn faq_q501_no_event_goes_into_a_day_that_is_over() {
    let app = app().await;
    assert!(sell(&app).await.unwrap());
    age_open_period(&app, 1);
    let before = app.fiscal_events().len();

    let refused = sell(&app).await;
    assert_eq!(refused, Err(errors::PREVIOUS_DAY_OPEN.to_string()));
    assert_eq!(app.fiscal_events().len(), before, "nothing signed");

    // Once the automatic Z is made, the sale goes through.
    assert!(close_overdue_day(&app.state).await.unwrap());
    assert!(sell(&app).await.unwrap());
}

#[tokio::test]
async fn faq_q501_nothing_to_close_makes_no_z() {
    let app = app().await;
    // Nothing recorded: no period, no Z.
    assert!(!close_overdue_day(&app.state).await.unwrap());
    // Today's period is not over.
    assert!(sell(&app).await.unwrap());
    assert!(!close_overdue_day(&app.state).await.unwrap());
    assert!(z_reports(&app).is_empty());
}

#[tokio::test]
async fn faq_q501_training_mode_makes_no_automatic_z() {
    let app = app().await;
    assert!(sell(&app).await.unwrap());
    age_open_period(&app, 1);
    app.state.config.lock().unwrap().training_mode = true;

    assert!(!close_overdue_day(&app.state).await.unwrap());
    assert!(z_reports(&app).is_empty());
}

#[tokio::test]
async fn faq_q501_an_fdm_failure_leaves_the_day_open() {
    let app = app().await;
    assert!(sell(&app).await.unwrap());
    let day = age_open_period(&app, 1);
    app.state.config.lock().unwrap().fdm_url = "http://127.0.0.1:1/graphql".into();

    assert!(close_overdue_day(&app.state).await.is_err());
    assert!(z_reports(&app).is_empty());
    assert_eq!(open_period_date(&app), Some(day));
}
