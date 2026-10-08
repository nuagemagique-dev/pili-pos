//! Tests of the HubRise and webshop facade (engine modules hubrise, webshop):
//!
//! - contract tests: every output type of the domain is built from a real engine value (an
//!   ingested webshop order, a status, a catalog outcome, an adoption preview, a signed online
//!   order, all against the in-process HubRise mock of the testkit and the admission FDM mock)
//!   and converted with `convert::out`; the engine's serde form and the record's must be equal.
//!   Hand-built values cover what a real run leaves empty (every `Attention` variant, discounts,
//!   a rejected catalog). Input types are checked field by field.
//! - wiring tests: every one of the 19 methods called on an engine opened by `support::open_seeded`
//!   (no operator is signed in, the FFI has no login yet in this card: a method that needs one
//!   answers its coded refusal, never `Internal`), then an engine whose `pos.json` has a `hubrise`
//!   section pointing at the mock, for the OAuth round trip.
//!
//! No test reaches a real HubRise host: only the testkit mock, refusals, and the development
//! fallback URL (built, never called). No `HUBRISE_*` variable is set.

use pos_engine::commands::hubrise as engine_hubrise;
use pos_engine::commands::webshop as engine_webshop;
use pos_engine_testkit::hubrise::{
    HubriseMock, POS_TOKEN, WEBSHOP_TOKEN, connect, connected_app, drain_outbox, poller,
};
use pos_engine_testkit::{PrinterSink, TestApp};
use pos_ffi::PosError;
use pos_ffi::records::common::{
    CheckoutResult, PaymentMethod, PaymentPart, RequestedMedium, TicketDeliveryRequest,
    contract_inp, contract_out,
};
use pos_ffi::records::hubrise as rec;
use pos_fiscal::payment::PaymentPart as EnginePaymentPart;
use pos_hubrise::poller::{poll_once, start};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json, to_value};

use crate::support::{self, DEVICE_ID, MANAGER_INSZ};

const CASHIER: &str = "12345678958";
const MANAGER: &str = "85073003328";

// ───────────────────────────── helpers ─────────────────────────────

fn json_of<T: Serialize>(v: &T) -> Value {
    to_value(v).expect("serialize")
}

/// Engine value to FFI record through `convert::out`; the two serde forms must be equal.
fn out_equal<E: Serialize, F: Serialize + DeserializeOwned>(engine: &E) -> F {
    let record: F = contract_out(engine).unwrap_or_else(|e| panic!("convert::out: {e}"));
    assert_eq!(
        json_of(engine),
        json_of(&record),
        "engine vs FFI serde form"
    );
    record
}

/// FFI record to engine value through `convert::inp`.
fn inp<F: Serialize, E: DeserializeOwned>(record: &F) -> E {
    contract_inp(record).unwrap_or_else(|e| panic!("convert::inp: {e}"))
}

/// Whether `value` (an extra key added) is refused by the record `F`.
fn refuses_unknown_key<F: DeserializeOwned>(mut value: Value) -> bool {
    value["unexpected_key"] = json!(1);
    serde_json::from_value::<F>(value).is_err()
}

fn coded<T: std::fmt::Debug>(r: Result<T, PosError>) -> String {
    match r.expect_err("an error") {
        PosError::Coded { code, .. } => code,
        other => panic!("expected a coded refusal, got {other:?}"),
    }
}

/// The code a refused `CheckoutResult` carries.
fn refusal(result: &CheckoutResult) -> String {
    assert!(!result.success, "{result:?}");
    assert_eq!(result.fiscal_event_id, None);
    result.error.clone().expect("an error code")
}

fn assert_coded<T: std::fmt::Debug>(r: Result<T, PosError>, expected: &str) {
    let got = coded(r);
    assert_eq!(got, expected);
}

/// A connected app (HubRise mock) whose poller has started, signing on the admission FDM mock
/// with a recording printer, the cashier logged in.
struct World {
    app: TestApp,
    _db: tempfile::TempDir,
    mock: HubriseMock,
    _printer: PrinterSink,
}

async fn world() -> World {
    let (app, db, mock) = connected_app().await;
    app.use_admission_mock().await;
    let printer = PrinterSink::start().await;
    app.set_printer(&printer.addr);
    start(&poller(&app)).await.unwrap();
    app.login_as(CASHIER, pos_core::user::UserRole::Cashier);
    World {
        app,
        _db: db,
        mock,
        _printer: printer,
    }
}

impl World {
    /// Inject a template and let the POS fetch it: the local order id and the HubRise id.
    async fn order(&self, template: &str) -> (i64, String) {
        let hr = self.mock.inject(template);
        let hr_id = hr["id"].as_str().unwrap().to_owned();
        self.poll().await;
        let id = self
            .app
            .state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT id FROM orders WHERE hubrise_order_id = ?1",
                [&hr_id],
                |r| r.get(0),
            )
            .unwrap();
        (id, hr_id)
    }

    async fn poll(&self) {
        poll_once(&poller(&self.app)).await.unwrap();
    }

    /// An accepted order, then cancelled by the platform.
    async fn cancelled_after_acceptance(&self, template: &str) -> i64 {
        let (id, hr_id) = self.order(template).await;
        engine_webshop::do_accept_webshop_order(&self.app.state, id, None)
            .await
            .unwrap();
        drain_outbox(&self.app).await;
        self.mock.update(&hr_id, json!({ "status": "cancelled" }));
        self.poll().await;
        id
    }

    fn manager(&self) {
        self.app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    }
}

/// Replace the connected catalog by one the POS refuses (a negative price).
fn push_bad_catalog(mock: &HubriseMock) {
    let mut inner = mock.state.inner.lock().unwrap();
    let id = inner.default_catalog_id.clone();
    let data = inner.catalogs[&id].data.clone().expect("seed catalog");
    let mut json = to_value(&data).unwrap();
    json["products"][0]["skus"][0]["price"] = json!("-1.00 EUR");
    let bad: pos_hubrise_types::CatalogData = serde_json::from_value(json).unwrap();
    inner.replace_catalog(WEBSHOP_TOKEN, &id, None, bad);
}

/// A product entered by hand before HubRise, named and priced like the catalog's (`name` has no
/// quote).
fn hand_entered(app: &TestApp, name: &str, price: i64) -> i64 {
    let db = app.state.db.lock().unwrap();
    db.execute(
        "INSERT OR IGNORE INTO categories (id, sort_order) VALUES (900, 0)",
        [],
    )
    .unwrap();
    db.execute(
        &format!(
            "INSERT INTO menu_items (category_id, price_cents, vat_code) VALUES (900, {price}, 'B')"
        ),
        [],
    )
    .unwrap();
    let id = db.last_insert_rowid();
    db.execute(
        &format!(
            "INSERT INTO translations (entity_type, entity_id, field, lang, value)
             VALUES ('menu_item', {id}, 'name', 'fr', '{name}')"
        ),
        [],
    )
    .unwrap();
    id
}

/// A catalog outcome through `convert::out`: every variant, a rejected catalog included, has the
/// same serde form on both sides (PD71).
fn outcome_equal(engine: &pos_catalog::ApplyOutcome) -> rec::ApplyOutcome {
    out_equal(engine)
}

// ───────────────────────────── contract: connection ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn connection_info_converts_connected_or_not() {
    let app = TestApp::new().await;
    let info = engine_hubrise::do_hubrise_get_connection_status(&app.state).unwrap();
    assert!(!info.connected);
    let record: rec::HubriseConnectionInfo = out_equal(&info);
    assert!(!record.connected);
    assert_eq!(record.account_id, None);
    assert_eq!(record.catalog_name, None);

    let (app, _db, _mock) = connected_app().await;
    let info = engine_hubrise::do_hubrise_get_connection_status(&app.state).unwrap();
    let record: rec::HubriseConnectionInfo = out_equal(&info);
    assert!(record.connected);
    assert_eq!(record.account_name.as_deref(), Some("Test account"));
    assert_eq!(record.location_name.as_deref(), Some("Test location"));
    assert_eq!(record.catalog_name.as_deref(), Some("Menu"));
    assert_eq!(record.customer_list_id, None);

    // Every field set.
    let full = json!({
        "connected": true, "account_id": "a", "account_name": "An", "location_id": "l",
        "location_name": "Ln", "catalog_id": "c", "catalog_name": "Cn",
        "customer_list_id": "cl", "customer_list_name": "Cln"
    });
    let record: rec::HubriseConnectionInfo = contract_out(&full).unwrap();
    assert_eq!(json_of(&record), full);
    assert!(refuses_unknown_key::<rec::HubriseConnectionInfo>(full));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_authorize_link_converts_with_its_qr_code() {
    let (app, _db) = TestApp::with_shared_db().await;
    let mock = HubriseMock::start().await;
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("hubrise.secret");
    std::fs::write(&secret, "s3cret").unwrap();
    app.state.config.lock().unwrap().hubrise = Some(
        serde_json::from_value(json!({
            "client_id": "pili-pos",
            "client_secret_file": secret,
            "oauth_base_url": format!("{}/oauth2/v1", mock.url),
            "api_base_url": mock.api(),
        }))
        .unwrap(),
    );
    app.state.hubrise_switch.set(true);

    let link = engine_hubrise::do_hubrise_start_oauth_flow(&app.state).unwrap();
    let record: rec::AuthorizeLink = out_equal(&link);
    assert!(
        record
            .url
            .starts_with(&format!("{}/oauth2/v1/authorize?", mock.url))
    );
    assert!(record.qr_svg.starts_with("<svg"));
    assert!(refuses_unknown_key::<rec::AuthorizeLink>(json_of(&record)));
}

// ───────────────────────────── contract: the board ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn ingested_webshop_orders_convert_with_their_resolution() {
    let w = world().await;
    for template in [
        "deliveroo_prepaid",
        "ubereats_prepaid",
        "webshop_online",
        "webshop_cash_collection",
        "webshop_cash_delivery",
        "eat_in",
    ] {
        w.mock.inject(template);
    }
    w.poll().await;

    let orders = engine_hubrise::do_list_webshop_orders(&w.app.state).unwrap();
    assert_eq!(orders.len(), 6);
    let records: Vec<rec::WebshopOrderSummary> = out_equal(&orders);
    assert_eq!(records.len(), 6);

    // An order resolved against the menu: lines with their VAT label, options on the board.
    let deliveroo = records
        .iter()
        .find(|o| o.source == "Deliveroo")
        .expect("the platform order");
    assert_eq!(deliveroo.service_type.as_deref(), Some("delivery"));
    assert!(deliveroo.total_cents > 0);
    assert!(
        deliveroo
            .customer_name
            .as_deref()
            .unwrap()
            .starts_with("Alex")
    );
    assert!(deliveroo.items.iter().any(|i| !i.options.is_empty()));
    let state = deliveroo.webshop.as_ref().expect("webshop state");
    assert_eq!(state.resolution, "ok");
    assert_eq!(state.payment_type.as_deref(), Some("APP"));
    assert_eq!(state.fiscal_state, "pending");
    assert!(!state.lines.is_empty());
    assert!(state.lines.iter().any(|l| l.kind == rec::LineKind::Product));

    let delivered = records
        .iter()
        .find(|o| o.customer_address_1.as_deref() == Some("Rue de Test 1"))
        .expect("an order with a delivery address");
    assert_eq!(delivered.customer_postal_code.as_deref(), Some("4000"));
    assert_eq!(delivered.customer_city.as_deref(), Some("Liège"));

    // An order the menu cannot place: the reason is an `Attention` the screen translates.
    let table_qr = records
        .iter()
        .find(|o| o.source == "Table QR")
        .expect("the eat-in order");
    let state = table_qr.webshop.as_ref().unwrap();
    assert_eq!(state.resolution, "needs_review");
    assert!(
        state.attention.contains(&rec::Attention::UnknownChannel {
            channel: "table qr".into()
        }),
        "{:?}",
        state.attention
    );

    // The confirmed time appears on the board once accepted with one.
    let (id, _) = w.order("ubereats_prepaid").await;
    engine_webshop::do_accept_webshop_order(&w.app.state, id, Some("2026-09-30T12:30:00+02:00"))
        .await
        .unwrap();
    let orders = engine_hubrise::do_list_webshop_orders(&w.app.state).unwrap();
    let records: Vec<rec::WebshopOrderSummary> = out_equal(&orders);
    let accepted = records.iter().find(|o| o.id == id).unwrap();
    assert_eq!(accepted.webshop.as_ref().unwrap().fiscal_state, "signed_n");
    assert!(accepted.hubrise_status.is_some());
}

#[test]
fn every_attention_and_warning_variant_converts() {
    use pos_hubrise::webshop::{Attention, Warning};
    let all = vec![
        Attention::UnknownChannel {
            channel: "X".into(),
        },
        Attention::MissingServiceType,
        Attention::NoItems,
        Attention::ItemWithoutRef { name: "n".into() },
        Attention::UnknownProduct {
            sku_ref: "S".into(),
        },
        Attention::UnknownOption {
            sku_ref: "S".into(),
            option_ref: "O".into(),
        },
        Attention::UnknownCharge {
            charge_ref: "C".into(),
        },
        Attention::FractionalQuantity {
            sku_ref: "S".into(),
        },
        Attention::BadPrice {
            at: "items[0]".into(),
        },
        Attention::VatMismatch {
            at: "items[1]".into(),
            order: "21.0".into(),
            menu: pos_core::vat::VatCode::C,
        },
        Attention::Discount { name: "d".into() },
        Attention::TotalMismatch {
            total_cents: 1000,
            lines_cents: 900,
        },
        Attention::PaymentUnclear {
            paid_cents: 500,
            total_cents: 1000,
        },
        Attention::ChangedAfterSigning,
    ];
    assert_eq!(all.len(), 14);
    let records: Vec<rec::Attention> = out_equal(&all);
    // And back: the engine reads its own variants from the record's serde form.
    let back: Vec<Attention> = inp(&records);
    assert_eq!(back, all);
    assert_eq!(
        json_of(&records[0]),
        json!({"code": "unknown_channel", "channel": "X"})
    );

    let warnings = vec![Warning::PriceDiffers {
        at: "items[0]".into(),
        catalog_cents: 1000,
        charged_cents: 1100,
    }];
    let records: Vec<rec::Warning> = out_equal(&warnings);
    let back: Vec<Warning> = inp(&records);
    assert_eq!(back, warnings);
}

#[test]
fn a_webshop_state_with_discounts_and_charges_converts() {
    use pos_hubrise::webshop::{LineDiscount, LineKind, ResolvedLine, WebshopState};
    let state = WebshopState {
        order_id: 7,
        hubrise_order_id: "hr-7".into(),
        channel: "Deliveroo".into(),
        channel_kind: Some("platform".into()),
        service_type: Some("delivery".into()),
        payment_mode: Some("prepaid".into()),
        payment_type: Some("APP".into()),
        fiscal_state: "pending".into(),
        resolution: "ok".into(),
        attention: Vec::new(),
        warnings: vec![pos_hubrise::webshop::Warning::PriceDiffers {
            at: "items[0]".into(),
            catalog_cents: 1000,
            charged_cents: 1100,
        }],
        lines: vec![
            ResolvedLine {
                kind: LineKind::Product,
                menu_item_id: 3,
                name: "Maki saumon".into(),
                quantity: 2,
                unit_price_cents: 1000,
                vat_code: pos_core::vat::VatCode::C,
                department_id: "FOOD".into(),
                department_name: "Cuisine".into(),
                discounts: vec![LineDiscount {
                    grouping_id: 1,
                    id: "discount:1".into(),
                    name: "Bienvenue".into(),
                    amount_cents: -200,
                }],
            },
            ResolvedLine {
                kind: LineKind::Charge,
                menu_item_id: 9,
                name: "Livraison".into(),
                quantity: 1,
                unit_price_cents: 250,
                vat_code: pos_core::vat::VatCode::A,
                department_id: "FOOD".into(),
                department_name: "Cuisine".into(),
                discounts: Vec::new(),
            },
        ],
    };
    let record: rec::WebshopState = out_equal(&state);
    // `discounts` is left out when empty, like the engine's JSON, and read back as empty.
    assert!(json_of(&record)["lines"][1].get("discounts").is_none());
    assert_eq!(record.lines[0].discounts[0].amount_cents, -200);
    assert_eq!(record.lines[1].kind, rec::LineKind::Charge);
    assert!(refuses_unknown_key::<rec::WebshopState>(json_of(&record)));
    assert!(refuses_unknown_key::<rec::ResolvedLine>(json_of(
        &record.lines[0]
    )));
    assert!(refuses_unknown_key::<rec::LineDiscount>(json_of(
        &record.lines[0].discounts[0]
    )));
}

#[test]
fn a_webshop_order_item_and_summary_convert_with_every_option_set() {
    let item = engine_hubrise::WebshopOrderItem {
        product_name: "Maki saumon".into(),
        quantity: 2,
        unit_price_cents: 550,
        line_total_cents: 1100,
        options: vec!["Sauce soja".into(), "Extra wasabi".into()],
        customer_notes: Some("sans gingembre".into()),
        product_name_zh: Some("三文鱼寿司".into()),
    };
    let record: rec::WebshopOrderItem = out_equal(&item);
    assert_eq!(record.options.len(), 2);
    assert_eq!(record.product_name_zh.as_deref(), Some("三文鱼寿司"));
    assert!(refuses_unknown_key::<rec::WebshopOrderItem>(json_of(
        &record
    )));

    let summary = engine_hubrise::WebshopOrderSummary {
        id: 5,
        hubrise_order_id: Some("hr-5".into()),
        status: "open".into(),
        hubrise_status: Some("new".into()),
        source: "Website".into(),
        service_type: Some("collection".into()),
        total_cents: 1100,
        created_at: "2026-09-30 11:00:00".into(),
        items: vec![item],
        customer_notes: Some("sonner".into()),
        customer_name: Some("Alex Test".into()),
        confirmed_time: Some("2026-09-30T12:30:00Z".into()),
        customer_address_1: Some("Rue de Test 1".into()),
        customer_address_2: Some("2e".into()),
        customer_postal_code: Some("4000".into()),
        customer_city: Some("Liege".into()),
        webshop: None,
    };
    let record: rec::WebshopOrderSummary = out_equal(&summary);
    assert_eq!(record.customer_address_2.as_deref(), Some("2e"));
    assert_eq!(record.webshop, None);
    assert!(refuses_unknown_key::<rec::WebshopOrderSummary>(json_of(
        &record
    )));
}

// ───────────────────────────── contract: status and catalog ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn the_status_converts_and_keeps_the_health_keys_at_the_top() {
    let w = world().await;
    w.mock.inject("deliveroo_prepaid");
    w.poll().await;

    let status = engine_hubrise::do_hubrise_status(&w.app.state).unwrap();
    assert!(status.health.connected);
    assert!(status.outbox.pending > 0, "the acknowledgement is queued");
    assert!(status.catalog.is_some());
    let record: rec::HubriseStatus = out_equal(&status);
    let json = json_of(&record);
    assert_eq!(json["connected"], true);
    assert!(json.get("health").is_none(), "flattened like the engine");
    assert_eq!(json["outbox"]["pending"], status.outbox.pending);
    assert!(record.health.connected);
    assert_eq!(record.outbox.pending, status.outbox.pending);
    assert!(!record.catalog_awaiting_adoption);
    let sync = record.catalog.as_ref().expect("sync state");
    assert_eq!(sync.source, "hubrise");
    assert!(sync.manages_menu);
    assert!(sync.applied_at.is_some());
    assert!(sync.issues.is_empty());

    // A refused catalog shows its issues.
    push_bad_catalog(&w.mock);
    w.poll().await;
    let status = engine_hubrise::do_hubrise_status(&w.app.state).unwrap();
    let record: rec::HubriseStatus = out_equal(&status);
    let sync = record.catalog.as_ref().expect("sync state");
    assert!(sync.rejected_at.is_some());
    assert!(!sync.issues.is_empty());
    assert_eq!(sync.issues[0].code, rec::IssueCode::NegativePrice);
    assert!(!sync.issues[0].key.is_empty());
}

#[test]
fn a_status_with_every_field_set_and_the_flatten_contract() {
    let status = engine_hubrise::HubriseStatus {
        health: pos_hubrise::token_store::ConnectionHealth {
            connected: true,
            last_poll_at: Some("2026-09-30 11:00:00".into()),
            auth_error_at: Some("2026-09-30 10:00:00".into()),
            last_error: Some("boom".into()),
            callback_registered_at: Some("2026-09-30 09:00:00".into()),
            last_catalog_sync_at: Some("2026-09-30 08:00:00".into()),
        },
        outbox: pos_hubrise::outbox::Backlog {
            pending: 3,
            parked: 1,
        },
        outbox_error: Some("429".into()),
        parked_events: 2,
        stale_orders: 4,
        catalog: Some(pos_catalog::SyncState {
            source: "hubrise".into(),
            manages_menu: true,
            applied_at: Some("2026-09-30 08:00:00".into()),
            rejected_at: Some("2026-09-30 08:30:00".into()),
            issues: vec![
                pos_catalog::validate::Issue::new(
                    pos_catalog::validate::IssueCode::MissingName,
                    "K1",
                ),
                pos_catalog::validate::Issue::new(
                    pos_catalog::validate::IssueCode::UnknownOptionList,
                    "",
                ),
            ],
        }),
        catalog_awaiting_adoption: true,
    };
    let record: rec::HubriseStatus = out_equal(&status);
    assert_eq!(record.health.last_error.as_deref(), Some("boom"));
    assert_eq!(record.parked_events, 2);
    assert_eq!(record.stale_orders, 4);
    assert!(record.catalog_awaiting_adoption);
    assert_eq!(record.catalog.as_ref().unwrap().issues.len(), 2);
    assert_eq!(
        record.catalog.as_ref().unwrap().issues[1].code,
        rec::IssueCode::UnknownOptionList
    );

    // The flatten costs `deny_unknown_fields` on `HubriseStatus` itself (see the record's doc):
    // the engine-vs-record JSON equality above is what catches a field the engine adds. The
    // records inside refuse unknown keys.
    assert!(!refuses_unknown_key::<rec::HubriseStatus>(json_of(&record)));
    assert!(refuses_unknown_key::<rec::ConnectionHealth>(json_of(
        &record.health
    )));
    assert!(refuses_unknown_key::<rec::Backlog>(json_of(&record.outbox)));
    assert!(refuses_unknown_key::<rec::SyncState>(json_of(
        record.catalog.as_ref().unwrap()
    )));
    assert!(refuses_unknown_key::<rec::Issue>(json_of(
        &record.catalog.as_ref().unwrap().issues[0]
    )));
}

#[test]
fn every_issue_code_converts() {
    use pos_catalog::validate::IssueCode as Engine;
    let all = [
        Engine::EmptyKey,
        Engine::DuplicateKey,
        Engine::UnknownCategory,
        Engine::MissingName,
        Engine::NameTooLong,
        Engine::UnknownLanguage,
        Engine::NegativePrice,
        Engine::MinAboveMax,
        Engine::NothingToSell,
        Engine::BadPrice,
        Engine::UnknownVatRate,
        Engine::VatMismatch,
        Engine::MissingVat,
        Engine::UnknownOptionList,
    ];
    let records: Vec<rec::IssueCode> = out_equal(&all.to_vec());
    assert_eq!(records.len(), 14);
    let back: Vec<Engine> = inp(&records);
    assert_eq!(back, all.to_vec());
}

#[tokio::test(flavor = "multi_thread")]
async fn catalog_outcomes_convert_applied_unchanged_awaiting_and_rejected() {
    // Applied, then unchanged.
    let (app, _db, mock) = connected_app().await;
    app.login_as(CASHIER, pos_core::user::UserRole::Cashier);
    let applied = engine_hubrise::do_sync_catalog(&app.state)
        .await
        .unwrap()
        .expect("an outcome");
    let record = outcome_equal(&applied);
    let rec::ApplyOutcome::Applied {
        hash,
        active_items,
        deactivated_items,
    } = record
    else {
        panic!("applied, got {record:?}");
    };
    assert!(!hash.is_empty());
    assert!(active_items > 0);
    assert_eq!(deactivated_items, 0);
    let again = engine_hubrise::do_sync_catalog(&app.state)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcome_equal(&again), rec::ApplyOutcome::Unchanged);

    // Rejected: the menu is untouched and the issues come with the outcome.
    push_bad_catalog(&mock);
    let rejected = engine_hubrise::do_sync_catalog(&app.state)
        .await
        .unwrap()
        .unwrap();
    let rec::ApplyOutcome::Rejected { issues } = outcome_equal(&rejected) else {
        panic!("rejected, got {rejected:?}");
    };
    assert_eq!(issues[0].code, rec::IssueCode::NegativePrice);

    // Awaiting adoption: a hand-entered menu waits for the manager.
    let (app, _db) = TestApp::with_shared_db().await;
    hand_entered(&app, "Soupe miso", 350);
    let mock = HubriseMock::start().await;
    connect(&app, &mock);
    app.login_as(CASHIER, pos_core::user::UserRole::Cashier);
    let waiting = engine_hubrise::do_sync_catalog(&app.state)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(outcome_equal(&waiting), rec::ApplyOutcome::AwaitingAdoption);
}

#[test]
fn a_rejected_outcome_converts_with_its_issues() {
    // The engine's `Rejected` is a struct variant (PD71): it serializes with its tag and its
    // issues, and `convert::out` carries it like the other variants.
    let rejected = pos_catalog::ApplyOutcome::Rejected {
        issues: vec![
            pos_catalog::validate::Issue::new(pos_catalog::validate::IssueCode::EmptyKey, ""),
            pos_catalog::validate::Issue::new(
                pos_catalog::validate::IssueCode::NegativePrice,
                "MAKI-6",
            ),
        ],
    };
    let expected = json!({
        "outcome": "rejected",
        "issues": [
            {"code": "empty_key", "key": ""},
            {"code": "negative_price", "key": "MAKI-6"}
        ]
    });
    assert_eq!(
        to_value(&rejected).expect("the engine serializes it"),
        expected
    );
    let record = outcome_equal(&rejected);
    assert_eq!(
        record,
        rec::ApplyOutcome::Rejected {
            issues: vec![
                rec::Issue {
                    code: rec::IssueCode::EmptyKey,
                    key: String::new(),
                },
                rec::Issue {
                    code: rec::IssueCode::NegativePrice,
                    key: "MAKI-6".into(),
                },
            ],
        }
    );
    // The record reads its own form back.
    let back: rec::ApplyOutcome = serde_json::from_value(json_of(&record)).unwrap();
    assert_eq!(back, record);
}

#[test]
fn a_huge_count_is_a_contract_error_not_a_wrap() {
    // `usize` counts become `i64`; a count above `i64::MAX` cannot happen on a database but must
    // never wrap.
    let big = pos_catalog::ApplyOutcome::Applied {
        hash: "h".into(),
        active_items: usize::MAX,
        deactivated_items: 0,
    };
    // 32-bit and 64-bit targets both: the conversion is checked, never `as`.
    match contract_out::<_, rec::ApplyOutcome>(&big) {
        Err(PosError::Internal { detail }) => assert!(detail.starts_with("contract: "), "{detail}"),
        Ok(rec::ApplyOutcome::Applied { active_items, .. }) => {
            assert_eq!(active_items, i64::try_from(usize::MAX).unwrap());
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_adoption_preview_converts_and_adopting_applies() {
    let (app, _db) = TestApp::with_shared_db().await;
    let miso = hand_entered(&app, "Soupe miso", 350);
    hand_entered(&app, "Gyoza", 550);
    let mock = HubriseMock::start().await;
    connect(&app, &mock);
    start(&poller(&app)).await.unwrap();
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);

    let preview = engine_hubrise::do_adoption_preview(&app.state).unwrap();
    assert_eq!(preview.len(), 1);
    let records: Vec<rec::Adoption> = out_equal(&preview);
    assert_eq!(records[0].menu_item_id, miso);
    assert_eq!(records[0].key, "MISO-1");
    assert_eq!(records[0].price_cents, 350);
    assert_eq!(records[0].name, "Soupe miso");
    assert!(refuses_unknown_key::<rec::Adoption>(json_of(&records[0])));

    // The input goes the other way, field by field.
    let adoptions: Vec<pos_catalog::Adoption> = inp(&records);
    assert_eq!(adoptions, preview);

    let outcome = engine_hubrise::do_adopt_catalog(&app.state, &adoptions).unwrap();
    let record = outcome_equal(&outcome);
    assert!(
        matches!(record, rec::ApplyOutcome::Applied { .. }),
        "{record:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn channels_convert_both_ways() {
    let (app, _db, _mock) = connected_app().await;
    app.login_as(MANAGER, pos_core::user::UserRole::Admin);
    let channel = rec::Channel {
        channel: "foodora".into(),
        kind: "platform".into(),
        payment_type: "APP".into(),
        charge_policy: "exclude".into(),
    };
    let engine: pos_hubrise::webshop::Channel = inp(&channel);
    assert_eq!(engine.channel, "foodora");
    assert_eq!(engine.kind, "platform");
    assert_eq!(engine.payment_type, "APP");
    assert_eq!(engine.charge_policy, "exclude");
    engine_hubrise::do_set_channel(&app.state, &engine).unwrap();

    let listed = engine_hubrise::do_list_channels(&app.state).unwrap();
    let records: Vec<rec::Channel> = out_equal(&listed);
    assert!(records.contains(&channel), "{records:?}");
    assert!(records.len() >= 2, "the seeded channels and the new one");
    assert!(refuses_unknown_key::<rec::Channel>(json_of(&channel)));
}

// ───────────────────────────── contract: signing online orders ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn accepting_handing_over_refunding_and_closing_convert() {
    let w = world().await;

    // Prepaid: the sale is signed at acceptance.
    let (prepaid, _) = w.order("deliveroo_prepaid").await;
    let accepted = engine_webshop::do_accept_webshop_order(
        &w.app.state,
        prepaid,
        Some("2026-09-30T12:30:00+02:00"),
    )
    .await
    .unwrap();
    let record: rec::AcceptResult = out_equal(&accepted);
    assert_eq!(record.fiscal_state, "signed_n");
    assert_eq!(record.fiscal_event_id, accepted.fiscal_event_id);
    assert!(record.kitchen_printed);
    let sale = record.sale.as_ref().expect("the sale");
    assert!(sale.success, "{:?}", sale.error);
    assert!(sale.ticket.is_some());
    assert!(refuses_unknown_key::<rec::AcceptResult>(json_of(&record)));

    // Unpaid: registered (P) at acceptance, collected at handover.
    let (unpaid, _) = w.order("webshop_cash_collection").await;
    let accepted = engine_webshop::do_accept_webshop_order(&w.app.state, unpaid, None)
        .await
        .unwrap();
    let record: rec::AcceptResult = out_equal(&accepted);
    assert_eq!(record.fiscal_state, "signed_p");
    assert_eq!(record.sale, None);
    let handover = engine_webshop::do_handover_webshop_order(
        &w.app.state,
        unpaid,
        &[EnginePaymentPart::cash()],
        None,
    )
    .await
    .unwrap();
    let record: CheckoutResult = out_equal(&handover);
    assert!(record.success, "{:?}", record.error);

    // Cancelled by the platform after the N: a manager refunds it.
    let cancelled = w.cancelled_after_acceptance("ubereats_prepaid").await;
    w.manager();
    let refund = engine_webshop::do_refund_webshop_order(&w.app.state, cancelled, MANAGER, None)
        .await
        .unwrap();
    let record: CheckoutResult = out_equal(&refund);
    assert!(record.success, "{:?}", record.error);

    // Cancelled after the P: closed by corrections and an N at 0,00.
    w.app.login_as(CASHIER, pos_core::user::UserRole::Cashier);
    let cancelled = w.cancelled_after_acceptance("webshop_cash_delivery").await;
    let closed = engine_webshop::do_close_cancelled_webshop_order(&w.app.state, cancelled)
        .await
        .unwrap();
    let record: CheckoutResult = out_equal(&closed);
    assert!(record.success, "{:?}", record.error);
}

// ───────────────────────────── inputs, field by field ─────────────────────────────

#[test]
fn the_hubrise_request_records_convert_to_the_engines() {
    let patch: engine_hubrise::PatchStatusRequest = inp(&rec::PatchStatusRequest {
        hubrise_order_id: "hr-1".into(),
        new_status: "in_preparation".into(),
    });
    assert_eq!(patch.hubrise_order_id, "hr-1");
    assert_eq!(patch.new_status, "in_preparation");

    let time: engine_hubrise::SetConfirmedTimeRequest = inp(&rec::SetConfirmedTimeRequest {
        hubrise_order_id: "hr-2".into(),
        confirmed_time: "2026-04-11T11:30:00Z".into(),
    });
    assert_eq!(time.hubrise_order_id, "hr-2");
    assert_eq!(time.confirmed_time, "2026-04-11T11:30:00Z");

    assert!(refuses_unknown_key::<rec::PatchStatusRequest>(
        json!({"hubrise_order_id": "x", "new_status": "y"})
    ));
    assert!(refuses_unknown_key::<rec::SetConfirmedTimeRequest>(
        json!({"hubrise_order_id": "x", "confirmed_time": "y"})
    ));
    // A field left out is refused (no default).
    assert!(
        serde_json::from_value::<rec::PatchStatusRequest>(json!({"hubrise_order_id": "x"}))
            .is_err()
    );
}

#[test]
fn the_webshop_request_records_convert_to_the_engines() {
    let with_time: engine_webshop::AcceptRequest = inp(&rec::AcceptRequest {
        order_id: 4,
        confirmed_time: Some("2026-09-30T12:30:00+02:00".into()),
    });
    assert_eq!(with_time.order_id, 4);
    assert_eq!(
        with_time.confirmed_time.as_deref(),
        Some("2026-09-30T12:30:00+02:00")
    );
    let without: engine_webshop::AcceptRequest = inp(&rec::AcceptRequest {
        order_id: 5,
        confirmed_time: None,
    });
    assert_eq!(without.confirmed_time, None);
    // The default of the engine: an absent key reads as `None`.
    let absent: rec::AcceptRequest = serde_json::from_value(json!({"order_id": 6})).unwrap();
    assert_eq!(absent.confirmed_time, None);

    let handover: engine_webshop::HandoverRequest = inp(&rec::HandoverRequest {
        order_id: 7,
        payments: vec![
            PaymentPart {
                method: PaymentMethod::Cash,
                amount_cents: None,
            },
            PaymentPart {
                method: PaymentMethod::MealVoucher,
                amount_cents: Some(850),
            },
        ],
        delivery: Some(TicketDeliveryRequest {
            medium: RequestedMedium::PaperDigital,
            email: Some("client@example.test".into()),
        }),
    });
    assert_eq!(handover.order_id, 7);
    assert_eq!(handover.payments.len(), 2);
    assert_eq!(handover.payments[0], EnginePaymentPart::cash());
    assert_eq!(handover.payments[1].amount_cents, Some(850));
    let delivery = handover.delivery.expect("a delivery request");
    assert_eq!(delivery.email.as_deref(), Some("client@example.test"));
    let bare: rec::HandoverRequest =
        serde_json::from_value(json!({"order_id": 8, "payments": []})).unwrap();
    assert_eq!(bare.delivery, None);

    let refund: engine_webshop::RefundRequest = inp(&rec::RefundRequest {
        order_id: 9,
        employee_insz: MANAGER.into(),
        delivery: None,
    });
    assert_eq!(refund.order_id, 9);
    assert_eq!(refund.employee_insz, MANAGER);
    assert!(refund.delivery.is_none());
    let bare: rec::RefundRequest =
        serde_json::from_value(json!({"order_id": 1, "employee_insz": "x"})).unwrap();
    assert_eq!(bare.delivery, None);

    assert!(refuses_unknown_key::<rec::AcceptRequest>(
        json!({"order_id": 1})
    ));
    assert!(refuses_unknown_key::<rec::HandoverRequest>(
        json!({"order_id": 1, "payments": []})
    ));
    assert!(refuses_unknown_key::<rec::RefundRequest>(
        json!({"order_id": 1, "employee_insz": "x"})
    ));
}

#[test]
fn the_adoption_record_converts_to_the_engines() {
    let adoption: pos_catalog::Adoption = inp(&rec::Adoption {
        menu_item_id: 12,
        key: "MISO-1".into(),
        name: "Soupe miso".into(),
        price_cents: 350,
    });
    assert_eq!(adoption.menu_item_id, 12);
    assert_eq!(adoption.key, "MISO-1");
    assert_eq!(adoption.name, "Soupe miso");
    assert_eq!(adoption.price_cents, 350);
}

// ───────────────────────────── wiring: every method ─────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn all_nineteen_methods_answer_on_a_seeded_engine() {
    let t = support::open_seeded().await;
    let e = &t.engine;

    // Not switched on and nobody signed in on this engine: the answers are the engine's refusals.
    let info = e.hubrise_get_connection_status().await.unwrap();
    assert!(!info.connected);
    assert_eq!(info.account_id, None);
    assert_coded(e.hubrise_start_oauth_flow().await, "hubrise_disabled");
    assert_coded(
        e.hubrise_complete_oauth("a-code".into()).await,
        "hubrise_disabled",
    );
    e.hubrise_disconnect().await.unwrap();
    assert!(e.hubrise_list_webshop_orders().await.unwrap().is_empty());
    assert_coded(
        e.hubrise_patch_order_status(rec::PatchStatusRequest {
            hubrise_order_id: "hr-1".into(),
            new_status: "in_preparation".into(),
        })
        .await,
        "hubrise_disabled",
    );
    assert_coded(
        e.hubrise_set_confirmed_time(rec::SetConfirmedTimeRequest {
            hubrise_order_id: "hr-1".into(),
            confirmed_time: "2026-04-11T11:30:00Z".into(),
        })
        .await,
        "hubrise_disabled",
    );
    let status = e.hubrise_status().await.unwrap();
    assert!(!status.health.connected);
    assert_eq!(status.outbox.pending, 0);
    assert_eq!(status.catalog, None);
    assert_coded(e.hubrise_sync_catalog().await, "hubrise_disabled");
    assert_coded(e.hubrise_adoption_preview().await, "not_logged_in");
    assert_coded(
        e.hubrise_adopt_catalog(Vec::new()).await,
        "hubrise_disabled",
    );
    assert_coded(e.hubrise_list_channels().await, "not_logged_in");
    assert_coded(
        e.hubrise_set_channel(rec::Channel {
            channel: "Just Eat".into(),
            kind: "platform".into(),
            payment_type: "APP".into(),
            charge_policy: "include".into(),
        })
        .await,
        "hubrise_disabled",
    );
    assert_coded(
        e.webshop_accept_order(rec::AcceptRequest {
            order_id: 1,
            confirmed_time: None,
        })
        .await,
        "hubrise_disabled",
    );
    // The signing commands that answer with a `CheckoutResult` refuse inside it.
    let handover = e
        .webshop_handover_order(rec::HandoverRequest {
            order_id: 1,
            payments: vec![PaymentPart {
                method: PaymentMethod::Cash,
                amount_cents: None,
            }],
            delivery: None,
        })
        .await
        .unwrap();
    assert_eq!(refusal(&handover), "not_logged_in");
    assert_coded(e.webshop_reject_order(1).await, "hubrise_disabled");
    assert_coded(e.webshop_complete_order(1).await, "not_logged_in");
    let refund = e
        .webshop_refund_order(rec::RefundRequest {
            order_id: 1,
            employee_insz: MANAGER_INSZ.into(),
            delivery: None,
        })
        .await
        .unwrap();
    assert_eq!(refusal(&refund), "not_logged_in");
    let closed = e.webshop_close_cancelled_order(1).await.unwrap();
    assert_eq!(refusal(&closed), "not_logged_in");
    t.engine.shutdown().await.unwrap();
}

// ───────────────────────────── wiring: the OAuth round trip on the mock ─────────────────────────────

/// An engine like `support::open_seeded`, with the integration switched on and a `hubrise` section
/// in its `pos.json` that points at the in-process HubRise mock (the secret in a file of the
/// bundle). No `HUBRISE_*` variable is involved.
async fn open_on_mock(mock: &HubriseMock) -> support::TestEngine {
    let fdm = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let fdm_url = format!("http://{}/graphql", fdm.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(fdm, pos_fdm_mock::app()).await.unwrap();
    });
    let printer_mock = pos_printer_mock::Printer::new(pos_printer_mock::Control::default(), None);
    let printer = pos_printer_mock::bind("127.0.0.1:0".parse().unwrap(), printer_mock)
        .await
        .unwrap()
        .to_string();

    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    let secret = bundle.join("hubrise.secret");
    std::fs::write(&secret, "mock-secret\n").unwrap();
    let pos_json = json!({
        "pos_id": "CPOS0031234567",
        "terminal_id": "POS-2-ANDROID",
        "device_id": "bundle-device",
        "restaurant_name": "Restaurant Test",
        "restaurant_address": "Rue de l'Exemple 1, 4000 Liege",
        "restaurant_vat": "BE0499999960",
        "restaurant_est_no": "2123456791",
        "fdm_url": fdm_url,
        "printer_address": printer,
        "table_count": 25,
        "hubrise_enabled": true,
        "hubrise": {
            "client_id": "pili-pos",
            "client_secret_file": secret,
            "oauth_base_url": format!("{}/oauth2/v1", mock.url),
            "api_base_url": mock.api(),
        },
    });
    std::fs::write(bundle.join("pos.json"), pos_json.to_string()).unwrap();

    let seed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../src-tauri/seed_spf_test.sql");
    pos_engine::install::init_db(&dir.path().join("data").join("pos.db"), Some(&seed)).unwrap();
    let engine = pos_ffi::PosEngine::open(support::options_for(dir.path()))
        .unwrap_or_else(|e| panic!("open: {e}"));
    support::TestEngine {
        engine,
        dir,
        fdm_url,
        printer,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_oauth_round_trip_runs_against_the_mock() {
    let mock = HubriseMock::start().await;
    let t = open_on_mock(&mock).await;
    let e = &t.engine;
    assert_eq!(t.options().device_id.as_deref(), Some(DEVICE_ID));

    // Switched on and not connected yet.
    assert!(!e.hubrise_get_connection_status().await.unwrap().connected);
    let link = e.hubrise_start_oauth_flow().await.unwrap();
    assert!(
        link.url
            .starts_with(&format!("{}/oauth2/v1/authorize?", mock.url)),
        "{}",
        link.url
    );
    assert!(link.qr_svg.starts_with("<svg"));
    assert!(!link.url.contains("mock-secret"));

    // A code the mock did not issue is refused (an error, never a panic).
    assert!(e.hubrise_complete_oauth("not-a-code".into()).await.is_err());
    assert!(!e.hubrise_get_connection_status().await.unwrap().connected);

    // The installer logs in: the mock issues the code for the POS client.
    mock.state
        .inner
        .lock()
        .unwrap()
        .pending_auth_codes
        .insert("pos-code-1".into(), POS_TOKEN.into());
    let info = e.hubrise_complete_oauth("pos-code-1".into()).await.unwrap();
    assert!(info.connected);
    assert!(info.account_name.is_some());
    assert!(info.location_id.is_some());
    assert!(info.catalog_id.is_some());
    assert_eq!(e.hubrise_get_connection_status().await.unwrap(), info);
    let status = e.hubrise_status().await.unwrap();
    assert!(status.health.connected);

    // Connected, with nobody signed in: the commands that need an operator say so.
    assert_coded(e.hubrise_sync_catalog().await, "not_logged_in");
    assert_coded(
        e.hubrise_patch_order_status(rec::PatchStatusRequest {
            hubrise_order_id: "hr-1".into(),
            new_status: "in_preparation".into(),
        })
        .await,
        "not_logged_in",
    );
    assert_coded(
        e.hubrise_set_confirmed_time(rec::SetConfirmedTimeRequest {
            hubrise_order_id: "hr-1".into(),
            confirmed_time: "2026-04-11T11:30:00Z".into(),
        })
        .await,
        "not_logged_in",
    );
    assert_coded(e.hubrise_adopt_catalog(Vec::new()).await, "not_logged_in");
    assert_coded(
        e.hubrise_set_channel(rec::Channel {
            channel: "Just Eat".into(),
            kind: "platform".into(),
            payment_type: "APP".into(),
            charge_policy: "include".into(),
        })
        .await,
        "not_logged_in",
    );
    assert_coded(
        e.webshop_accept_order(rec::AcceptRequest {
            order_id: 1,
            confirmed_time: None,
        })
        .await,
        "not_logged_in",
    );
    assert_coded(e.webshop_reject_order(1).await, "not_logged_in");

    // Disconnecting forgets the connection.
    e.hubrise_disconnect().await.unwrap();
    assert!(!e.hubrise_get_connection_status().await.unwrap().connected);
    assert!(!e.hubrise_status().await.unwrap().health.connected);
    t.engine.shutdown().await.unwrap();
}
