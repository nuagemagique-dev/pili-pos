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

mod meal_voucher {
    use super::*;
    use pos_engine::commands::checkout::CheckoutPaymentMethod::MealVoucher;
    use pos_engine::commands::copy::do_copy_ticket;
    use pos_engine::commands::report::{
        do_generate_user_x_report, do_generate_x_report, do_generate_z_report,
    };
    use serde_json::Value;

    async fn app() -> (TestApp, Arc<Script>, PrinterSink) {
        let mut app = TestApp::new().await;
        app.use_admission_mock().await;
        app.login_as(MANAGER, pos_core::user::UserRole::Admin);
        let script = app.link_terminal();
        let printer = PrinterSink::start().await;
        app.set_printer(&printer.addr);
        (app, script, printer)
    }

    fn request_of(app: &TestApp, event_id: i64) -> Value {
        let event = app
            .fiscal_events()
            .into_iter()
            .find(|e| e.id == event_id)
            .unwrap();
        serde_json::from_str(event.event_data_json.as_deref().unwrap()).unwrap()
    }

    /// `(type, inputMethod, provider, reference, amountType, amount)`.
    type Line = (String, String, Option<String>, Option<String>, String, f64);

    fn lines(request: &Value) -> Vec<Line> {
        let text = |v: &Value| v.as_str().map(str::to_string);
        request["financials"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    text(&f["type"]).unwrap(),
                    text(&f["inputMethod"]).unwrap(),
                    text(&f["provider"]),
                    text(&f["reference"]),
                    text(&f["amountType"]).unwrap(),
                    f["amount"].as_f64().unwrap(),
                )
            })
            .collect()
    }

    fn line(
        t: &str,
        input: &str,
        provider: Option<&str>,
        reference: Option<&str>,
        amount_type: &str,
        amount: f64,
    ) -> Line {
        (
            t.into(),
            input.into(),
            provider.map(str::to_string),
            reference.map(str::to_string),
            amount_type.into(),
            amount,
        )
    }

    fn payment_rows(app: &TestApp) -> Vec<(String, i64)> {
        let db = app.state.db.lock().unwrap();
        let mut stmt = db
            .prepare("SELECT method, amount_cents FROM payments ORDER BY id")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    async fn pay_card(
        app: &TestApp,
        script: &Script,
        order: i64,
        cents: i64,
        brand: &str,
        card_type: CardType,
    ) -> i64 {
        script.on_pay(Ok(approval(cents, brand, card_type)));
        let p = do_eft_pay(&app.state, order, cents).await.unwrap();
        assert_eq!(p.status, "approved");
        p.id
    }

    #[tokio::test]
    async fn an_edenred_approval_plus_cash_signs_a_meal_voucher_line() {
        let (app, script, _printer) = app().await;
        let order = app.order_of(1000);
        let id = pay_card(&app, &script, order, 600, "EDENRED", CardType::Unknown).await;
        // The brand is stored normalized; the card type stays unknown.
        let row = &app.eft_rows()[0];
        assert_eq!(row.brand.as_deref(), Some("Edenred"));
        let listed = do_list_eft_payments(&app.state, order).unwrap();
        assert!(listed[0].meal_voucher);
        assert_eq!(listed[0].card_type.as_deref(), Some("unknown"));

        let sale = do_checkout_split(
            &app.state,
            order,
            &[PaymentPart::terminal(600, id), PaymentPart::cash()],
            None,
        )
        .await
        .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let request = request_of(&app, sale.fiscal_event_id.unwrap());
        assert_eq!(request["financials"][0]["id"], "3");
        assert_eq!(request["financials"][0]["name"], "Meal voucher");
        assert_eq!(
            lines(&request),
            [
                line(
                    "CHEQUE_MEAL",
                    "AUTOMATIC",
                    Some("Edenred"),
                    Some("TXN-600"),
                    "PAYMENT",
                    6.0
                ),
                line("CASH", "MANUAL", None, None, "PAYMENT", 4.0),
            ]
        );
        assert_eq!(
            payment_rows(&app),
            [("meal_voucher".to_string(), 600), ("cash".to_string(), 400)]
        );
    }

    #[tokio::test]
    async fn the_terminal_meal_voucher_part_is_exact_and_only_the_cash_rest_is_rounded() {
        let (app, script, _printer) = app().await;
        // 10,03 cannot be paid in exact cash: the rest rounds, the voucher does not.
        let order = app.order_of(1003);
        let id = pay_card(&app, &script, order, 600, "Edenred", CardType::Unknown).await;
        let sale = do_checkout_split(
            &app.state,
            order,
            &[PaymentPart::terminal(600, id), PaymentPart::cash()],
            None,
        )
        .await
        .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let request = request_of(&app, sale.fiscal_event_id.unwrap());
        let financials = request["financials"].as_array().unwrap();
        let amounts: Vec<(String, String, String, f64)> = financials
            .iter()
            .map(|f| {
                (
                    f["type"].as_str().unwrap().to_string(),
                    f["inputMethod"].as_str().unwrap().to_string(),
                    f["amountType"].as_str().unwrap().to_string(),
                    f["amount"].as_f64().unwrap(),
                )
            })
            .collect();
        let row =
            |t: &str, m: &str, a: &str, x: f64| (t.to_string(), m.to_string(), a.to_string(), x);
        assert_eq!(
            amounts,
            [
                row("CHEQUE_MEAL", "AUTOMATIC", "PAYMENT", 6.0),
                row("CASH", "MANUAL", "PAYMENT", 4.03),
                row("CASH", "MANUAL", "ROUNDING", 0.02),
            ],
            "the voucher is exact, the cash rest 4,03 rounds to 4,05"
        );
        assert_eq!(
            payment_rows(&app),
            [("meal_voucher".to_string(), 600), ("cash".to_string(), 405)],
            "the rounding applies to cash only"
        );
    }

    #[tokio::test]
    async fn an_approved_meal_voucher_charge_is_still_listed_as_unsettled() {
        use pos_engine::commands::eft::do_list_unsettled_card_payments;
        let (app, script, _printer) = app().await;
        let order = app.order_of(1000);
        let id = pay_card(&app, &script, order, 600, "Edenred", CardType::Unknown).await;
        let listed = do_list_unsettled_card_payments(&app.state).unwrap();
        assert_eq!(listed.len(), 1, "money taken must reach the Z warning");
        assert_eq!(listed[0].payment_id, id);
        assert_eq!(listed[0].order_id, order);
        assert_eq!(listed[0].amount_cents, 600);
        assert_eq!(listed[0].brand.as_deref(), Some("Edenred"));

        // Settled by its sale, it leaves the list.
        let sale = do_checkout_split(
            &app.state,
            order,
            &[PaymentPart::terminal(600, id), PaymentPart::cash()],
            None,
        )
        .await
        .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        assert!(
            do_list_unsettled_card_payments(&app.state)
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn a_scheme_branded_card_stays_a_card_and_an_eco_cheque_is_no_meal_voucher() {
        let (app, script, _printer) = app().await;
        let order = app.order_of(1500);
        let a = pay_card(&app, &script, order, 500, "Mastercard", CardType::Unknown).await;
        let b = pay_card(&app, &script, order, 500, "Edenred Eco", CardType::Unknown).await;
        let c = pay_card(&app, &script, order, 500, "Bancontact", CardType::Debit).await;
        let parts = [
            PaymentPart::terminal(500, a),
            PaymentPart::terminal(500, b),
            PaymentPart::terminal(500, c),
        ];
        let sale = do_checkout_split(&app.state, order, &parts, None)
            .await
            .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let types: Vec<_> = lines(&request_of(&app, sale.fiscal_event_id.unwrap()))
            .into_iter()
            .map(|l| l.0)
            .collect();
        assert_eq!(types, ["CARD_UNKNOWN", "CARD_UNKNOWN", "CARD_DEBIT"]);
        assert_eq!(
            payment_rows(&app).iter().filter(|p| p.0 == "card").count(),
            3
        );
    }

    #[tokio::test]
    async fn manual_and_terminal_meal_vouchers_share_one_row_on_every_report_and_ticket() {
        let (app, script, printer) = app().await;
        let order = app.order_of(3000);
        let edenred = pay_card(
            &app,
            &script,
            order,
            1000,
            "Ticket Restaurant",
            CardType::Unknown,
        )
        .await;
        let bancontact = pay_card(&app, &script, order, 1200, "Bancontact", CardType::Debit).await;
        let parts = [
            PaymentPart::of(MealVoucher, 500),
            PaymentPart::terminal(1000, edenred),
            PaymentPart::terminal(1200, bancontact),
            PaymentPart::cash(),
        ];
        let sale = do_checkout_split(&app.state, order, &parts, None)
            .await
            .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let event = sale.fiscal_event_id.unwrap();
        assert_eq!(
            lines(&request_of(&app, event)),
            [
                line("CHEQUE_MEAL", "MANUAL", None, None, "PAYMENT", 5.0),
                line(
                    "CHEQUE_MEAL",
                    "AUTOMATIC",
                    Some("Edenred"),
                    Some("TXN-1000"),
                    "PAYMENT",
                    10.0
                ),
                line(
                    "CARD_DEBIT",
                    "AUTOMATIC",
                    Some("Bancontact"),
                    Some("TXN-1200"),
                    "PAYMENT",
                    12.0
                ),
                line("CASH", "MANUAL", None, None, "PAYMENT", 3.0),
            ]
        );
        assert_eq!(
            payment_rows(&app),
            [
                ("meal_voucher".to_string(), 500),
                ("meal_voucher".to_string(), 1000),
                ("card".to_string(), 1200),
                ("cash".to_string(), 300),
            ]
        );

        // The ticket and its copy both name the issuer.
        printer.settle(1).await;
        let original = printer
            .rendered()
            .into_iter()
            .find(|r| r.contains("INFORMATIONS FDM"))
            .expect("the ticket");
        let meal_rows = |text: &str| -> Vec<String> {
            text.lines()
                .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|l| l.starts_with("Chèques-repas"))
                .collect()
        };
        assert_eq!(
            meal_rows(&original),
            ["Chèques-repas 5,00", "Chèques-repas Edenred 10,00"],
            "{original}"
        );
        let copy = do_copy_ticket(&app.state, event, MANAGER.into())
            .await
            .unwrap();
        assert!(copy.success, "{:?}", copy.error);
        printer.settle(2).await;
        let copied = printer
            .rendered()
            .into_iter()
            .filter(|r| r.contains("INFORMATIONS FDM"))
            .nth(1)
            .expect("the copy");
        assert!(copied.contains("COPIE"), "{copied}");
        assert_eq!(meal_rows(&copied), meal_rows(&original));

        // X, user X and Z: one meal-voucher row of 15,00.
        let meal_total = |report: &Value| -> Vec<f64> {
            report["payments"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["type"] == "CHEQUE_MEAL")
                .flat_map(|p| {
                    assert_eq!(p["id"], "3");
                    p["amount"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|a| a["totalAmount"].as_f64().unwrap())
                })
                .collect()
        };
        let last_request = |app: &TestApp| -> Value {
            let e = app.fiscal_events().pop().unwrap();
            serde_json::from_str(e.event_data_json.as_deref().unwrap()).unwrap()
        };
        let x = do_generate_x_report(&app.state, MANAGER.into())
            .await
            .unwrap();
        assert!(x.success, "{:?}", x.error);
        assert_eq!(meal_total(&last_request(&app)["turnover"]), [15.0]);
        let u = do_generate_user_x_report(&app.state, MANAGER.into())
            .await
            .unwrap();
        assert!(u.success, "{:?}", u.error);
        let user_report = last_request(&app);
        let users = user_report["users"].as_array().unwrap();
        assert_eq!(meal_total(&users[0]), [15.0]);
        let z = do_generate_z_report(&app.state, MANAGER.into())
            .await
            .unwrap();
        assert!(z.success, "{:?}", z.error);
        // The Z signs the turnover report, then the user report.
        let events = app.fiscal_events();
        let json = |e: &FiscalEventRow| -> Value {
            serde_json::from_str(e.event_data_json.as_deref().unwrap()).unwrap()
        };
        let z_user = json(&events[events.len() - 1]);
        let z_turnover = json(&events[events.len() - 2]);
        assert_eq!(meal_total(&z_turnover["turnover"]), [15.0]);
        assert_eq!(meal_total(&z_user["users"].as_array().unwrap()[0]), [15.0]);
    }
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
    async fn the_hubrise_echo_names_a_terminal_meal_voucher_part_meal_voucher() {
        let w = world().await;
        let order = w.app.order_of(1000);
        let id = {
            w.script
                .on_pay(Ok(approval(600, "Edenred", CardType::Unknown)));
            do_eft_pay(&w.app.state, order, 600).await.unwrap().id
        };
        let sale = do_checkout_split(
            &w.app.state,
            order,
            &[PaymentPart::terminal(600, id), PaymentPart::cash()],
            None,
        )
        .await
        .unwrap();
        assert!(sale.success, "{:?}", sale.error);
        let body: String = w
            .app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT body_json FROM hubrise_outbox WHERE kind = 'create_order'",
                [],
                |r| r.get(0),
            )
            .expect("the till sale is queued for HubRise");
        let body: serde_json::Value = serde_json::from_str(&body).unwrap();
        let payments: Vec<(String, String)> = body["payments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                (
                    p["name"].as_str().unwrap().to_string(),
                    p["amount"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        assert_eq!(
            payments,
            [
                ("Meal voucher".to_string(), "6.00 EUR".to_string()),
                ("Cash".to_string(), "4.00 EUR".to_string()),
            ],
            "{body}"
        );
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
