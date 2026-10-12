//! Card payments taken on the linked terminal and the signature of the
//! sale that settles them: they are marked used in the sale's own
//! commit, also when the FDM's answer comes late; a crash between the
//! terminal call and its answer is found again at start-up; an online
//! order collected at the till takes a terminal payment like any sale.

use std::sync::Arc;

use pos_engine_testkit as common;

use common::*;
use pos_eft::scripted::Script;
use pos_eft::{CardType, Outcome};
use pos_engine::commands::checkout::{PaymentPart, do_checkout_split};
use pos_engine::commands::eft::{do_eft_check, do_eft_pay, do_list_eft_payments};
use pos_engine::commands::errors;
use pos_engine::commands::report::close_overdue_day;
use pos_engine::engine::{ConfigSource, Engine, EngineConfig, EnginePaths, EngineTimers};

const MANAGER: &str = "12345678958";

async fn lossy_app(fdm: &LossyFdm) -> (TestApp, Arc<Script>) {
    let mut app = TestApp::new().await;
    app.use_lossy_fdm(fdm);
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let script = app.link_terminal();
    (app, script)
}

async fn approved_payment(app: &TestApp, script: &Script, order: i64, cents: i64) -> i64 {
    script.on_pay(Ok(approval(cents, "Bancontact", CardType::Debit)));
    let payment = do_eft_pay(&app.state, order, cents).await.unwrap();
    assert_eq!(payment.status, "approved");
    payment.id
}

#[tokio::test]
async fn a_lost_fdm_answer_leaves_the_payment_unused_until_the_sale_is_archived() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signSale");
    let (app, script) = lossy_app(&fdm).await;
    let order = app.order_of(2200);
    let payment = approved_payment(&app, &script, order, 2200).await;
    let parts = [PaymentPart::terminal(2200, payment)];

    let first = do_checkout_split(&app.state, order, &parts, None)
        .await
        .unwrap();
    assert!(!first.success);
    assert!(first.error.unwrap().contains(errors::FDM_ANSWER_PENDING));
    // Not signed as far as the till knows: the card payment waits, unused.
    assert!(app.fiscal_events().is_empty());
    assert_eq!(app.eft_rows()[0].used_event_id, None);
    assert_eq!(do_list_eft_payments(&app.state, order).unwrap().len(), 1);
    // The pending request carries the payment, to settle it on archive.
    let pending = app.open_pending().expect("the open request");
    assert!(
        pending
            .context_json
            .contains(&format!("\"eft_payment_ids\":[{payment}]")),
        "{}",
        pending.context_json
    );

    // The answer comes on a later send (the background tick): the sale
    // is archived and the payment settled by it, in one commit.
    assert_eq!(close_overdue_day(&app.state).await, Ok(false));
    let events = app.fiscal_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "N");
    assert_eq!(app.eft_rows()[0].used_event_id, Some(events[0].id));
    assert_eq!(app.order_status(order).as_deref(), Some("closed"));
    assert!(do_list_eft_payments(&app.state, order).unwrap().is_empty());
    let request: serde_json::Value =
        serde_json::from_str(events[0].event_data_json.as_deref().unwrap()).unwrap();
    assert_eq!(request["financials"][0]["inputMethod"], "AUTOMATIC");
    assert_eq!(request["financials"][0]["provider"], "Bancontact");
}

#[tokio::test]
async fn a_retried_checkout_after_a_lost_answer_settles_the_payment_once() {
    let fdm = LossyFdm::start().await;
    fdm.lose_first("signSale");
    let (app, script) = lossy_app(&fdm).await;
    let order = app.order_of(2200);
    let payment = approved_payment(&app, &script, order, 2200).await;
    let parts = [PaymentPart::terminal(2200, payment)];

    assert!(
        !do_checkout_split(&app.state, order, &parts, None)
            .await
            .unwrap()
            .success
    );
    let second = do_checkout_split(&app.state, order, &parts, None)
        .await
        .unwrap();
    assert!(second.success, "{:?}", second.error);
    let event = second.fiscal_event_id.unwrap();
    assert_eq!(app.eft_rows()[0].used_event_id, Some(event));
    assert_eq!(fdm.sent_op("signSale").len(), 2);
    assert_eq!(
        fdm.sent_op("signSale")[0].body,
        fdm.sent_op("signSale")[1].body
    );
    assert_eq!(app.fiscal_events().len(), 1);
}

#[tokio::test]
async fn a_payment_still_sent_at_start_up_becomes_unknown_and_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let open = || {
        Engine::open(EngineConfig {
            paths: EnginePaths::under(dir.path().to_path_buf()),
            config: ConfigSource::DevelopmentDefaults,
            events: Arc::new(pos_engine::events::NullSink),
            timers: EngineTimers::default(),
            identity: pos_engine::SoftwareIdentity::new("pili-pos/1.0.0").unwrap(),
            http_tls: pos_engine::HttpTls::Platform,
            device_id_override: None,
            support_program: None,
        })
        .expect("open the engine")
    };

    // The till asked the terminal, then stopped before the answer.
    let engine = open();
    let (sent, approved) = {
        let db = engine.db.lock().unwrap();
        db.execute(
            "INSERT INTO orders (order_type, status, user_insz) VALUES ('takeaway', 'open', ?1)",
            [MANAGER],
        )
        .unwrap();
        let order = db.last_insert_rowid();
        let add = |reference: &str| {
            pos_db::queries::eft::insert_sent(
                &db,
                &pos_db::queries::eft::NewPayment {
                    reference,
                    order_id: order,
                    terminal_kind: "ccv_opi",
                    amount_cents: 1500,
                    user_insz: MANAGER,
                },
            )
            .unwrap()
        };
        let sent = add("ref-sent");
        let done = add("ref-done");
        pos_db::queries::eft::resolve(
            &db,
            done,
            &pos_db::queries::eft::Resolution::Declined {
                reason: "no".into(),
            },
        )
        .unwrap();
        (sent, done)
    };
    drop(engine);

    let engine = open();
    let status = |id: i64| {
        let db = engine.db.lock().unwrap();
        pos_db::queries::eft::get(&db, id)
            .unwrap()
            .unwrap()
            .status
            .as_str()
    };
    assert_eq!(status(sent), "unknown");
    assert_eq!(status(approved), "declined", "a final answer stays");
    // And a later start leaves it alone.
    drop(engine);
    assert_eq!(
        {
            let engine = open();
            let db = engine.db.lock().unwrap();
            pos_db::queries::eft::get(&db, sent)
                .unwrap()
                .unwrap()
                .detail
                .unwrap()
        },
        "the till stopped before the terminal answered"
    );
}

#[cfg(feature = "hubrise")]
mod webshop {
    use super::*;
    use common::hubrise::{HubriseMock, connected_app, poller};
    use pos_engine::commands::webshop::{do_accept_webshop_order, do_handover_webshop_order};
    use pos_hubrise::poller::{poll_once, start};

    struct World {
        app: TestApp,
        _db: tempfile::TempDir,
        mock: HubriseMock,
        script: Arc<Script>,
    }

    async fn world() -> World {
        let (mut app, db, mock) = connected_app().await;
        app.use_admission_mock().await;
        let script = app.link_terminal();
        start(&poller(&app)).await.unwrap();
        app.login_as(MANAGER, pos_core::user::UserRole::Cashier);
        World {
            app,
            _db: db,
            mock,
            script,
        }
    }

    /// An accepted order to be paid at the till: its local id and total.
    async fn accepted_unpaid(w: &World) -> (i64, i64) {
        let hr = w.mock.inject("webshop_cash_collection");
        let hr_id = hr["id"].as_str().unwrap().to_owned();
        poll_once(&poller(&w.app)).await.unwrap();
        let (id, total): (i64, i64) = w
            .app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT id, total_cents FROM orders WHERE hubrise_order_id = ?1",
                [&hr_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        do_accept_webshop_order(&w.app.state, id, None)
            .await
            .unwrap();
        (id, total)
    }

    #[tokio::test]
    async fn an_online_order_collected_at_the_till_takes_a_terminal_card() {
        let w = world().await;
        let (id, total) = accepted_unpaid(&w).await;
        assert!(total > 0);
        let payment = approved_payment(&w.app, &w.script, id, total).await;

        let sale = do_handover_webshop_order(
            &w.app.state,
            id,
            &[PaymentPart::terminal(total, payment)],
            None,
        )
        .await
        .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let event = sale.fiscal_event_id.unwrap();
        assert_eq!(w.app.eft_rows()[0].used_event_id, Some(event));
        let n = w
            .app
            .fiscal_events()
            .into_iter()
            .find(|e| e.id == event)
            .unwrap();
        let request: serde_json::Value =
            serde_json::from_str(n.event_data_json.as_deref().unwrap()).unwrap();
        assert_eq!(request["costCenter"]["type"], "WEBSHOP");
        assert_eq!(request["financials"][0]["type"], "CARD_DEBIT");
        assert_eq!(request["financials"][0]["inputMethod"], "AUTOMATIC");
        assert_eq!(request["financials"][0]["provider"], "Bancontact");
    }

    #[tokio::test]
    async fn the_handover_is_refused_while_a_terminal_payment_waits_or_is_left_out() {
        let w = world().await;
        let (id, total) = accepted_unpaid(&w).await;
        w.script.on_pay(Ok(Outcome::Unknown {
            detail: "timeout".into(),
        }));
        let payment = do_eft_pay(&w.app.state, id, total).await.unwrap();
        let refused = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
            .await
            .unwrap();
        assert_eq!(refused.error.as_deref(), Some(errors::EFT_PENDING));

        w.script
            .on_status(Ok(approval(total, "Visa", CardType::Credit)));
        do_eft_check(&w.app.state, payment.id).await.unwrap();
        let forgotten = do_handover_webshop_order(&w.app.state, id, &[PaymentPart::cash()], None)
            .await
            .unwrap();
        assert_eq!(
            forgotten.error.as_deref(),
            Some(errors::EFT_PAYMENT_NOT_USABLE)
        );
        // Refused before anything was signed: the order can still be collected.
        let sale = do_handover_webshop_order(
            &w.app.state,
            id,
            &[PaymentPart::terminal(total, payment.id)],
            None,
        )
        .await
        .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let n = &w.app.fiscal_events().into_iter().last().unwrap();
        let request: serde_json::Value =
            serde_json::from_str(n.event_data_json.as_deref().unwrap()).unwrap();
        assert_eq!(request["financials"][0]["type"], "CARD_CREDIT");
    }
}
