//! Pre-submission rehearsal against a real virtual FDM.
//!
//! This file is `#[ignore]`d by default - `cargo test --workspace`
//! does NOT run it. Activate explicitly:
//!
//! ```bash
//! PILI_STAGING_FDM_URL=https://<virtual-fdm>/graphql \
//! PILI_STAGING_DUMP=1 \
//!     cargo test -p pos-fiscal --test e2e_staging -- --ignored --nocapture
//! ```
//!
//! Environment knobs (all optional except the URL):
//!
//! | Variable | Default | Purpose |
//! |---|---|---|
//! | `PILI_STAGING_FDM_URL`      | -                | FDM GraphQL endpoint (required) |
//! | `PILI_STAGING_FDM_TOKEN`    | unset            | bearer token, for a cloud FDM (module2 simulator) |
//! | `PILI_STAGING_POS_ID`       | `CPOS0031234567` | posId registered with the FDM (the test setup's) |
//! | `PILI_STAGING_VAT`          | `BE0000000097`   | vatNo (SPF test identity) |
//! | `PILI_STAGING_EST_NO`       | `2000000042`     | estNo (SPF test identity) |
//! | `PILI_STAGING_INSZ`         | `12345678901`    | employee INSZ for S events |
//! | `PILI_STAGING_TICKET_START` | `1`              | first posFiscalTicketNo - bump on re-runs so numbering stays strictly increasing across rehearsals |
//! | `PILI_STAGING_PAUSE_MS`     | `1000`           | pause after each signed event, to go easy on a shared FDM |
//! | `PILI_STAGING_DUMP`         | unset            | `1` → write every request/response pair to `docs/SUBMISSION_ARTIFACTS/evidence/staging/rehearsal_log.md` |
//!
//! `staging_status_returns_operational` is the intake preflight;
//! `staging_basic_set_smoke` replays the full Basic Set sequence the
//! journal (`JOURNAL_TESTS_PRE_SOUMISSION_FR.md`) documents:
//!
//! S work-in → P order → P pre-bill → N sale → N refund → C copy →
//! F drawer-open → F money-in → R turnover-X → R user-X → S work-out.
//!
//! For every `SignResult` it asserts: echoed identification fields,
//! expected event label, strictly increasing `totalCounter`, per-label
//! monotonic `eventCounter`, non-empty `digitalSignature`, and - when
//! `shortSignature` is present - that it equals SHA-1(digitalSignature)
//! per the spec's short-signature derivation.
//!
//! The Z report is deliberately NOT part of the smoke: it resets FDM
//! period counters, and the journal exercises it separately together
//! with the app-layer atomic-commit test.
//!
//! Why ignored by default: a CI run with no virtual-FDM credentials
//! would hammer a stranger's URL. The opt-in keeps this honest.

use std::collections::HashMap;
use std::env;

use pos_core::config::PosConfig;
use pos_core::order::{Order, OrderLine, OrderStatus, OrderType};
use pos_core::vat::VatCode;
use pos_fiscal::event::{
    TurnoverReportData, UserReportData, build_copy_input, build_order_input, build_pre_bill_input,
    build_report_turnover_x, build_report_user_x, build_sale_input, build_work_in_out,
};
use pos_fiscal::fdm_client::FdmClient;
use pos_fiscal::inputs::{
    DrawerOpenInput, EventTotalInput, FdmDeviceInput, InOutItemInput, MoneyInOutInput,
    MoneyInOutLineInput, NegQuantityTotalInput, PaymentTotalAmountInput, PaymentTotalInput,
    PosDeviceInput, TurnoverInput, UserItemInput, VatTotalInput,
};
use pos_fiscal::response::SignResult;
use pos_fiscal::signature::verify_short_signature;
use pos_fiscal::types::{
    EventLabel, InOut, InputMethod, MoneyInOutLineType, NegQuantityReason, PaymentLineType,
    PaymentType, VatLabel,
};
use uuid::Uuid;

/// A booking period for tests.
fn bk(period_id: &str) -> pos_fiscal::event::Booking {
    pos_fiscal::event::Booking {
        period_id: period_id.to_string(),
        date: "2026-07-03".to_string(),
    }
}

fn staging_url() -> Option<String> {
    env::var("PILI_STAGING_FDM_URL").ok()
}

fn env_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}

/// A client for the staging FDM, with its bearer token when one is set.
fn staging_client(url: &str) -> FdmClient {
    FdmClient::new(url).with_bearer_token(
        env::var("PILI_STAGING_FDM_TOKEN")
            .ok()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty()),
    )
}

fn staging_config(url: &str) -> PosConfig {
    PosConfig {
        pos_id: env_or("PILI_STAGING_POS_ID", "CPOS0031234567"),
        terminal_id: "POS1".to_string(),
        device_id: "DEV001".to_string(),
        // SPF test identity (UC_TestSetup_20240806) unless overridden
        // with the identity the virtual FDM was provisioned for.
        restaurant_name: "The FPS Finances Experience".to_string(),
        restaurant_address: "Boulevard Albert II 33, 1030 Brussels".to_string(),
        restaurant_vat: env_or("PILI_STAGING_VAT", "BE0000000097"),
        restaurant_est_no: env_or("PILI_STAGING_EST_NO", "2000000042"),
        fdm_url: url.to_string(),
        ..Default::default()
    }
}

/// Sanity check - talk to the real virtual FDM, get a Status response,
/// confirm it claims to be operational.
#[tokio::test]
#[ignore]
async fn staging_status_returns_operational() {
    let url = match staging_url() {
        Some(u) => u,
        None => {
            eprintln!(
                "PILI_STAGING_FDM_URL not set - skipping. \
                 Provision a virtual FDM and re-run with the URL in env."
            );
            return;
        }
    };

    let client = staging_client(&url);
    let status = client
        .status()
        .await
        .expect("Status query against real virtual FDM");

    // The exact assertion list mirrors the cert manual's intake test:
    // device record present, no blocking errors, buffer not full.
    assert!(
        !status.device.fdm_id.is_empty(),
        "fdmId must be populated by the FDM"
    );
    // bufferCapacityUsed is a percentage (POS spec v2.2: "exprimée en
    // pourcentage"; the Q110 fixture shows 10.02). Warn at the FDM's
    // own BUFFER_NEAR_FULL threshold, fail only when actually full.
    assert!(
        status.device.buffer_capacity_used < 100.0,
        "buffer must have headroom at intake start"
    );
    if status.device.buffer_capacity_used >= 70.0 {
        eprintln!(
            "⚠ buffer at {:.2}% - FDM will raise BUFFER_NEAR_FULL; \
             sync with SPF cloud before the intake appointment",
            status.device.buffer_capacity_used
        );
    }
    if let Some(errs) = status.errors.as_ref() {
        // `show_pos` is the cert-meaningful flag - only
        // `Mandatory`-display errors are intake-blocking.
        let blocking: Vec<_> = errs
            .iter()
            .filter(|e| matches!(e.extensions.show_pos, pos_fiscal::types::Display::Mandatory))
            .collect();
        assert!(
            blocking.is_empty(),
            "no mandatory-display error should be active at intake start; got: {:#?}",
            blocking
        );
    }
}

// ── Rehearsal bookkeeping ──────────────────────────────────────

/// Tracks cross-event invariants (counter monotonicity) and, when
/// `PILI_STAGING_DUMP=1`, accumulates a markdown log of every
/// request/response pair for the dossier.
struct Rehearsal {
    pos_id: String,
    log: String,
    dump: bool,
    last_total_counter: i64,
    per_label_counter: HashMap<String, i64>,
    results: Vec<SignResult>,
}

impl Rehearsal {
    fn new(pos_id: &str, fdm_url: &str) -> Self {
        Self {
            pos_id: pos_id.to_string(),
            log: format!(
                "# Journal de répétition - staging_basic_set_smoke\n\n\
                 - Date : {}\n- FDM : `{}`\n- posId : `{}`\n",
                chrono::Local::now().to_rfc3339(),
                fdm_url,
                pos_id
            ),
            dump: env::var("PILI_STAGING_DUMP").as_deref() == Ok("1"),
            last_total_counter: 0,
            per_label_counter: HashMap::new(),
            results: Vec::new(),
        }
    }

    /// Verify the invariants every signed event must satisfy, record
    /// the pair in the log, and keep the result for the X report.
    fn check<T: serde::Serialize>(
        &mut self,
        step: &str,
        expected_label: EventLabel,
        expected_ticket_no: i64,
        request: &T,
        result: SignResult,
    ) {
        assert_eq!(
            result.fdm_ref.event_label, expected_label,
            "{step}: FDM echoed wrong event label"
        );
        assert_eq!(result.pos_id, self.pos_id, "{step}: FDM echoed wrong posId");
        assert_eq!(
            result.pos_fiscal_ticket_no, expected_ticket_no,
            "{step}: FDM echoed wrong posFiscalTicketNo"
        );
        assert!(
            result.fdm_ref.total_counter > self.last_total_counter,
            "{step}: totalCounter must be strictly increasing \
             (got {}, previous {})",
            result.fdm_ref.total_counter,
            self.last_total_counter
        );
        self.last_total_counter = result.fdm_ref.total_counter;

        let label_key = format!("{:?}", expected_label);
        if let Some(prev) = self.per_label_counter.get(&label_key) {
            assert!(
                result.fdm_ref.event_counter > *prev,
                "{step}: eventCounter for label {label_key} must be \
                 monotonic (got {}, previous {prev})",
                result.fdm_ref.event_counter
            );
        }
        self.per_label_counter
            .insert(label_key, result.fdm_ref.event_counter);

        assert!(
            !result.digital_signature.is_empty(),
            "{step}: digitalSignature must be present"
        );
        if let Some(short) = result.short_signature.as_deref() {
            assert!(
                verify_short_signature(&result.digital_signature, short)
                    .expect("short-signature derivation"),
                "{step}: shortSignature does not match \
                 SHA-1(digitalSignature)"
            );
        }

        if self.dump {
            let req_json =
                serde_json::to_string_pretty(request).unwrap_or_else(|_| "<unserializable>".into());
            let resp_json =
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| "<unserializable>".into());
            self.log.push_str(&format!(
                "\n## {step}\n\n### Requête\n\n```json\n{req_json}\n```\n\
                 \n### Réponse\n\n```json\n{resp_json}\n```\n"
            ));
        }
        eprintln!(
            "✓ {step}: label={:?} eventCounter={} totalCounter={} short={}",
            result.fdm_ref.event_label,
            result.fdm_ref.event_counter,
            result.fdm_ref.total_counter,
            result.short_signature.as_deref().unwrap_or("-"),
        );
        self.results.push(result);

        // One event at a time, spaced out: the FDM may be shared (the
        // module2 simulator is). Blocking is fine, nothing else runs.
        let pause: u64 = env_or("PILI_STAGING_PAUSE_MS", "1000")
            .parse()
            .expect("PILI_STAGING_PAUSE_MS must be an integer");
        std::thread::sleep(std::time::Duration::from_millis(pause));
    }

    /// Write the accumulated log to the dossier evidence directory.
    fn finish(self) {
        if !self.dump {
            return;
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/SUBMISSION_ARTIFACTS/evidence/staging");
        std::fs::create_dir_all(&dir).expect("create staging evidence dir");
        let path = dir.join("rehearsal_log.md");
        std::fs::write(&path, self.log).expect("write rehearsal log");
        eprintln!("rehearsal log written to {}", path.display());
    }
}

// ── Fixture data (mirrors the journal's scripted service) ──────

fn rehearsal_order(table: i32, insz: &str) -> Order {
    Order {
        id: 1,
        table_number: Some(table),
        order_type: OrderType::DineIn,
        status: OrderStatus::Open,
        user_insz: insz.to_string(),
        created_at: chrono::Utc::now(),
        closed_at: None,
        subtotal_cents: 1200,
        total_cents: 1200,
    }
}

/// Two lines exercising both restaurant VAT rates: B (dine-in food)
/// and A (beverage). Total €12.00.
fn rehearsal_lines(sign: i32) -> Vec<OrderLine> {
    vec![
        OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 42,
            quantity: sign,
            unit_price_cents: 850,
            vat_code: VatCode::B,
            name: "Saumon roll x8".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Test".to_string(),
        },
        OrderLine {
            id: 2,
            order_id: 1,
            menu_item_id: 7,
            quantity: sign,
            unit_price_cents: 350,
            vat_code: VatCode::A,
            name: "Coca 33cl".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Test".to_string(),
        },
    ]
}

/// Full Basic Set rehearsal against the real FDM. See module docs for
/// the sequence and the per-event assertions.
#[tokio::test]
#[ignore]
async fn staging_basic_set_smoke() {
    let url = match staging_url() {
        Some(u) => u,
        None => {
            eprintln!("PILI_STAGING_FDM_URL not set - skipping.");
            return;
        }
    };

    let config = staging_config(&url);
    let insz = env_or("PILI_STAGING_INSZ", "12345678901");
    let mut ticket_no: i64 = env_or("PILI_STAGING_TICKET_START", "1")
        .parse()
        .expect("PILI_STAGING_TICKET_START must be an integer");
    let booking_period_id = Uuid::new_v4().to_string();

    let client = staging_client(&url);
    let mut r = Rehearsal::new(&config.pos_id, &url);

    let mut next_ticket = || {
        let n = ticket_no;
        ticket_no += 1;
        n
    };

    // 1. S - work-in (M140). The service day starts with staff
    //    presence registration; the FDM refuses sales before it.
    let n = next_ticket();
    let input = build_work_in_out(&config, n, &insz, &bk(&booking_period_id));
    let result = client
        .sign_work_in(input.clone(), false)
        .await
        .expect("signWorkIn");
    r.check("1. signWorkIn (S) - M140", EventLabel::S, n, &input, result);

    // 2. P - order (M121): table 4 opens.
    let order = rehearsal_order(4, &insz);
    let lines = rehearsal_lines(1);
    let n = next_ticket();
    let input = build_order_input(&config, &order, &lines, n, &bk(&booking_period_id));
    let result = client
        .sign_order(input.clone(), false)
        .await
        .expect("signOrder");
    r.check("2. signOrder (P) - M121", EventLabel::P, n, &input, result);

    // 3. P - pre-bill (M123): provisional bill for the same table.
    let n = next_ticket();
    let input = build_pre_bill_input(&config, &order, &lines, n, &bk(&booking_period_id));
    let result = client
        .sign_pre_bill(input.clone(), false)
        .await
        .expect("signPreBill");
    r.check(
        "3. signPreBill (P) - M123",
        EventLabel::P,
        n,
        &input,
        result,
    );

    // 4. N - sale (M110): the table pays cash.
    let n = next_ticket();
    let input = build_sale_input(
        &config,
        &order,
        &lines,
        n,
        &bk(&booking_period_id),
        PaymentType::Cash,
        1200,
        0,
        None,
    );
    let result = client
        .sign_sale(input.clone(), false)
        .await
        .expect("signSale");
    assert!(
        result.short_signature.is_some(),
        "N sale must carry a shortSignature (printed on the ticket)"
    );
    let sale_ref = result.fdm_ref.clone();
    r.check("4. signSale (N) - M110", EventLabel::N, n, &input, result);

    // 5. N - full refund (M111), referencing the original sale.
    let refund_lines = rehearsal_lines(-1);
    let n = next_ticket();
    let input = build_sale_input(
        &config,
        &order,
        &refund_lines,
        n,
        &bk(&booking_period_id),
        PaymentType::Cash,
        -1200,
        0,
        Some(&sale_ref),
    );
    let result = client
        .sign_sale(input.clone(), false)
        .await
        .expect("signSale (refund)");
    r.check(
        "5. signSale refund (N) - M111",
        EventLabel::N,
        n,
        &input,
        result,
    );

    // 6. C - copy of the original sale (M160).
    let n = next_ticket();
    let input = build_copy_input(&config, n, &insz, &bk(&booking_period_id), &sale_ref);
    let result = client
        .sign_copy(input.clone(), false)
        .await
        .expect("signCopy");
    r.check("6. signCopy (C) - M160", EventLabel::C, n, &input, result);

    // 7. F - drawer open without transaction (M131).
    let n = next_ticket();
    let input = DrawerOpenInput {
        common: pos_fiscal::event::build_common_fields(
            &config,
            n,
            &insz,
            &bk(&booking_period_id),
            pos_fiscal::types::TicketMedium::None,
        ),
        drawer: None,
    };
    let result = client
        .sign_drawer_open(input.clone(), false)
        .await
        .expect("signDrawerOpen");
    r.check(
        "7. signDrawerOpen (F) - M131",
        EventLabel::F,
        n,
        &input,
        result,
    );

    // 8. F - money in: €50 float added to the drawer (M130).
    let n = next_ticket();
    let input = MoneyInOutInput {
        common: pos_fiscal::event::build_common_fields(
            &config,
            n,
            &insz,
            &bk(&booking_period_id),
            pos_fiscal::types::TicketMedium::None,
        ),
        drawer: None,
        financials: vec![MoneyInOutLineInput {
            id: "MIO1".to_string(),
            name: "Fond de caisse".to_string(),
            payment_type: PaymentType::Cash,
            provider: None,
            input_method: InputMethod::Manual,
            amount: 50.0,
            amount_type: MoneyInOutLineType::MoneyInOut,
            foreign_currency: None,
            reference: None,
            drawer: None,
        }],
    };
    let result = client
        .sign_money_in_out(input.clone(), false)
        .await
        .expect("signMoneyInOut");
    r.check(
        "8. signMoneyInOut (F) - M130",
        EventLabel::F,
        n,
        &input,
        result,
    );

    // 9. R - turnover X report (M180). Totals reflect exactly what
    //    this rehearsal signed: one sale + its full refund (net 0),
    //    with the drawer opened once.
    let first = r.results.first().expect("at least one result");
    let last = r.results.last().expect("at least one result");
    let report_data = TurnoverReportData {
        pos_devices: vec![PosDeviceInput {
            pos_id: config.pos_id.clone(),
            terminal_id: Some(config.terminal_id.clone()),
            first_pos_date_time: first.pos_date_time.clone(),
            last_pos_date_time: last.pos_date_time.clone(),
        }],
        fdm_devices: vec![FdmDeviceInput {
            fdm_id: first.fdm_ref.fdm_id.clone(),
            first_fdm_date_time: first.fdm_ref.fdm_date_time.clone(),
            first_total_counter: first.fdm_ref.total_counter,
            last_fdm_date_time: last.fdm_ref.fdm_date_time.clone(),
            last_total_counter: last.fdm_ref.total_counter,
        }],
        turnover: TurnoverInput {
            transactions: vec![EventTotalInput {
                event_label: EventLabel::N,
                ticket_count: 2,
                amount: 0.0,
            }],
            departments: vec![],
            vats: vec![
                VatTotalInput {
                    label: VatLabel::A,
                    rate: 21.0,
                    taxable_amount: 0.0,
                    vat_amount: 0.0,
                    total_amount: 0.0,
                },
                VatTotalInput {
                    label: VatLabel::B,
                    rate: 12.0,
                    taxable_amount: 0.0,
                    vat_amount: 0.0,
                    total_amount: 0.0,
                },
            ],
            payments: vec![PaymentTotalInput {
                id: "PAY1".to_string(),
                name: "Cash".to_string(),
                payment_type: PaymentType::Cash,
                amount: vec![PaymentTotalAmountInput {
                    amount_type: PaymentLineType::Payment,
                    normal_amount: 0.0,
                    negative_corrections: 0.0,
                    positive_corrections: 0.0,
                    corrections_count: 0,
                    total_amount: 0.0,
                    normal_foreign: None,
                    corrections_foreign: None,
                }],
            }],
            drawers_open_count: 1,
            neg_quantities: vec![NegQuantityTotalInput {
                neg_quantity_reason: NegQuantityReason::Refund,
                neg_quantity_count: 2,
                ticket_count: 1,
                amount: -12.0,
            }],
            price_changes: vec![],
            invoices: None,
        },
    };
    let n = next_ticket();
    let input = build_report_turnover_x(&config, n, &insz, &bk(&booking_period_id), report_data);
    let result = client
        .sign_report_turnover_x(input.clone(), false)
        .await
        .expect("signReportTurnoverX");
    r.check(
        "9. signReportTurnoverX (R) - M180",
        EventLabel::R,
        n,
        &input,
        result,
    );

    // 10. R - user X report (M182): per-operator turnover +
    //     presence, referencing the same device blocks.
    let first = r.results.first().expect("results");
    let last = r.results.last().expect("results");
    let user_data = UserReportData {
        pos_devices: vec![PosDeviceInput {
            pos_id: config.pos_id.clone(),
            terminal_id: Some(config.terminal_id.clone()),
            first_pos_date_time: first.pos_date_time.clone(),
            last_pos_date_time: last.pos_date_time.clone(),
        }],
        fdm_devices: vec![FdmDeviceInput {
            fdm_id: first.fdm_ref.fdm_id.clone(),
            first_fdm_date_time: first.fdm_ref.fdm_date_time.clone(),
            first_total_counter: first.fdm_ref.total_counter,
            last_fdm_date_time: last.fdm_ref.fdm_date_time.clone(),
            last_total_counter: last.fdm_ref.total_counter,
        }],
        users: vec![UserItemInput {
            employee_id: insz.clone(),
            total_amount: 0.0, // sale + full refund net out
            first_pos_date_time: first.pos_date_time.clone(),
            last_pos_date_time: last.pos_date_time.clone(),
            social_events: vec![InOutItemInput {
                pos_date_time: first.pos_date_time.clone(),
                in_out: InOut::In,
            }],
            payments: vec![],
        }],
    };
    let n = next_ticket();
    let input = build_report_user_x(&config, n, &insz, &bk(&booking_period_id), user_data);
    let result = client
        .sign_report_user_x(input.clone(), false)
        .await
        .expect("signReportUserX");
    r.check(
        "10. signReportUserX (R) - M182",
        EventLabel::R,
        n,
        &input,
        result,
    );

    // 11. S - work-out (M141): the service day closes.
    let n = next_ticket();
    let input = build_work_in_out(&config, n, &insz, &bk(&booking_period_id));
    let result = client
        .sign_work_out(input.clone(), false)
        .await
        .expect("signWorkOut");
    r.check(
        "11. signWorkOut (S) - M141",
        EventLabel::S,
        n,
        &input,
        result,
    );

    eprintln!(
        "staging_basic_set_smoke: 11/11 events signed, counters \
         monotonic, all short signatures verified."
    );
    r.finish();
}
