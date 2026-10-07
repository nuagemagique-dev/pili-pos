//! The wait for the printer's DLE EOT answer is the engine's
//! `EngineTimers::printer_status_read` (PD37): a printer that answers
//! late is read with a longer wait and not with the default one.

use pos_engine_testkit as common;

use common::*;

use std::time::Duration;

use pos_core::order::OrderType;
use pos_engine::commands::bar_slip::SlipOutcome;
use pos_engine::commands::order::{
    AddOrderLineInput, CreateOrderInput, do_add_order_line, do_create_order,
};
use pos_engine::commands::tables::do_sign_table_order;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const OPERATOR: &str = "12345678958";

/// A printer whose answers to DLE EOT come `delay` after the query and
/// say "cover open" (DLE EOT 2) then "fine" (DLE EOT 4).
async fn slow_cover_open_printer(delay: Duration) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut data = Vec::new();
                let mut chunk = [0u8; 4096];
                let mut answered = 0;
                loop {
                    let n = match stream.read(&mut chunk).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => n,
                    };
                    data.extend_from_slice(&chunk[..n]);
                    let asked = data.windows(2).filter(|w| w == &[0x10, 0x04]).count();
                    while answered < asked {
                        tokio::time::sleep(delay).await;
                        // Fixed bits, plus the cover open bit on the offline cause.
                        let byte = if answered == 0 { 0x12 | 0x04 } else { 0x12 };
                        if stream.write_all(&[byte]).await.is_err() {
                            return;
                        }
                        answered += 1;
                    }
                }
            });
        }
    });
    addr
}

async fn app_with_one_bar_line() -> (TestApp, i64) {
    let app = TestApp::new().await;
    app.use_admission_mock().await;
    app.login_as(OPERATOR, pos_core::user::UserRole::Admin);
    {
        let conn = app.state.db.lock().unwrap();
        conn.execute_batch(
            "INSERT INTO categories (id, sort_order, active) VALUES (1, 0, 1);
             INSERT INTO menu_items (id, category_id, price_cents, vat_code, kitchen_printer, sort_order, active) VALUES
                (1, 1, 650, 'A', 1, 0, 1);
             INSERT INTO translations (entity_type, entity_id, field, lang, value) VALUES
                ('menu_item', 1, 'name', 'fr', 'Sake');",
        )
        .unwrap();
    }
    let order = do_create_order(
        &app.state,
        CreateOrderInput {
            order_type: OrderType::DineIn,
            table_number: Some(4),
            user_insz: OPERATOR.into(),
        },
    )
    .unwrap()
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
    )
    .unwrap();
    (app, order)
}

#[tokio::test]
async fn a_late_answer_is_lost_with_the_default_printer_status_wait() {
    let (app, order) = app_with_one_bar_line().await;
    assert_eq!(
        app.state.timers.printer_status_read,
        Duration::from_millis(500)
    );
    app.set_printer(&slow_cover_open_printer(Duration::from_millis(800)).await);
    // The cover-open answer comes after 500 ms: unread, so the slip passes.
    let outcome = do_sign_table_order(&app.state, order).await.unwrap();
    assert_eq!(outcome, SlipOutcome::Printed);
}

#[tokio::test]
async fn the_engine_timer_sets_the_printer_status_wait() {
    let (mut app, order) = app_with_one_bar_line().await;
    app.state.timers.printer_status_read = Duration::from_secs(3);
    app.set_printer(&slow_cover_open_printer(Duration::from_millis(800)).await);
    // The same printer, read with the engine timer: the cover is open.
    let outcome = do_sign_table_order(&app.state, order).await.unwrap();
    assert_eq!(outcome, SlipOutcome::Failed);
}
