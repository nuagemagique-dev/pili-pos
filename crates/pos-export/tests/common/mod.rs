//! Shared test support of the export tests: the fixture database and the
//! privacy proof.
//!
//! # The fixture
//!
//! [`fixture_db`] writes a migrated `pos.db` (WAL, `pos_db::schema::run_migrations`)
//! that looks like a restaurant after two days of service. Event bodies are
//! shaped like the archive of the engine (the requests of
//! `docs/SUBMISSION_ARTIFACTS/evidence/uc`), the FDM answers carry a VAT
//! calculation, and the Z report is computed by the engine's own
//! `pos_fiscal::report_totals::turnover` from the events of the period, so the
//! totals are those the engine would sign.
//!
//! | Z | booking date | by | content |
//! |---|--------------|----|---------|
//! | 41 | 2026-10-03 | system (automatic Z) | one takeaway sale of 6,00 EUR |
//! | 42 | 2026-10-04 | admin | the day described below |
//! | none | 2026-10-05 (open) | | a training Z (label T, no Z number, no closing) |
//!
//! The period of Z 42 holds, in this order: a presence (S), a pro forma (P)
//! and a dine-in sale of table 7 (2 lines, a 10 % line discount, card), a
//! takeaway sale (cash with 1 cent rounding plus a meal voucher), an online
//! delivery order of the webshop (ticket-wide discount, paid online, system
//! user), a refund of one beer of the first sale (cash), a sale of table 9
//! closed unpaid, an online collection order of a platform (paid in its app,
//! system user), a copy (C) of the first sale, a training sale (T), a drawer
//! event (F) and an invoice (I). Six of them are sales (label N) and are the
//! content of the statistics file: [`Fixture::tickets`], 9 lines, total
//! [`Fixture::z_total_cents`] (91,24 EUR). Everything else must stay out.
//!
//! Personal and secret data planted in the database, all listed in
//! [`Fixture::secrets`]: the INSZ of three users, their names and PIN hashes,
//! the `HubRise` access token, a customer name, phone, e-mail and address in
//! `raw_hubrise_json` and in a `hubrise_events` payload, the e-mail of a
//! `ticket_deliveries` row and the ticket text sent to it, the VAT number of
//! the invoiced customer, and the FDM signatures and verification URLs of the
//! events. `pseudonym.key` is not part of the database.
//!
//! Reuse: X1-T5 and X1-T6 call [`fixture_db`] for a realistic `pos.db`
//! (backup, snapshot, whole export), [`build_fixture_stats`] for a statistics
//! file, and [`assert_no_personal_data`] to prove a produced file is clean.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]
#![expect(
    dead_code,
    reason = "each test binary uses a different part of these shared helpers"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pos_export::stats::{StatsMeta, StatsSummary, build_stats};
use pos_export::zref::{ZRef, real_zs_after};
use rusqlite::{Connection, OpenFlags, params};
use serde_json::{Map, Value, json};

pub(crate) const ADMIN_INSZ: &str = "75061189702";
pub(crate) const CASHIER_INSZ: &str = "84022899837";
pub(crate) const WAITER_INSZ: &str = "91051234567";
pub(crate) const POS_ID: &str = "CPOS0031234567";
pub(crate) const TERMINAL_ID: &str = "POS-1-BAR";
pub(crate) const SW_VERSION: &str = "pili-pos/1.0.0";
pub(crate) const FDM_ID: &str = "FOD01000001";

/// What the statistics of one sale must say.
#[derive(Debug, Clone)]
pub(crate) struct ExpectedTicket {
    /// Local id of the sale event (`fiscal_events.id`).
    pub ticket_id: i64,
    pub channel: &'static str,
    pub kind: &'static str,
    pub table_number: Option<i64>,
    pub total_cents: i64,
    pub unpaid: bool,
    pub lines: i64,
    /// INSZ of the user who signed it.
    pub employee: &'static str,
}

/// What the fixture database contains.
#[derive(Debug, Clone)]
pub(crate) struct Fixture {
    pub db_path: PathBuf,
    /// The Z the tests export: number 42.
    pub z_no: i64,
    pub booking_date: String,
    pub period_id: String,
    pub closing_event_id: i64,
    /// The earlier real Z (number 41, an automatic Z).
    pub earlier_z_no: i64,
    /// The sales of Z 42 in archive order.
    pub tickets: Vec<ExpectedTicket>,
    /// Lines of those sales.
    pub lines: i64,
    /// Total of the sales of Z 42, signed, in cents.
    pub z_total_cents: i64,
    /// VAT of Z 42: (label, rate in basis points, base, vat, total) in cents.
    pub z_vat: Vec<(&'static str, i64, i64, i64, i64)>,
    /// Payments of Z 42: (method, kind, amount) in cents.
    pub z_payments: Vec<(&'static str, &'static str, i64)>,
    /// Ids of the events of the period that must not reach the statistics:
    /// presence, pro forma, copy, training sale, drawer, invoice, user Z.
    pub excluded_event_ids: Vec<i64>,
    /// Ids of the other events of the database that must not reach it either
    /// (training Z, the sale of the earlier day).
    pub other_event_ids: Vec<i64>,
    /// Every string that must never appear in a statistics file.
    pub secrets: Vec<String>,
}

pub(crate) fn eur(cents: i64) -> f64 {
    #[expect(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        reason = "fixture amounts are a few hundred euros, exact in an f64"
    )]
    let e = cents as f64 / 100.0;
    e
}

/// The base of a VAT-inclusive total at `rate` percent, rounded to the cent.
fn base_of(total: i64, rate: f64) -> i64 {
    #[expect(
        clippy::as_conversions,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "fixture amounts are a few hundred euros, exact in an f64"
    )]
    let base = (total as f64 / (1.0 + rate / 100.0)).round() as i64;
    base
}

// ── Building blocks of the events ──────────────────────────────────

struct Chg {
    grouping: i64,
    id: &'static str,
    name: &'static str,
    scope: &'static str,
    kind: &'static str,
    cents: i64,
}

struct Ln {
    product: i64,
    name: &'static str,
    dept: (&'static str, &'static str),
    qty: i64,
    unit: i64,
    label: &'static str,
    changes: Vec<Chg>,
    refund: bool,
}

impl Ln {
    fn net(&self) -> i64 {
        self.qty * self.unit + self.changes.iter().map(|c| c.cents).sum::<i64>()
    }
}

fn line(
    product: i64,
    name: &'static str,
    dept: (&'static str, &'static str),
    qty: i64,
    unit: i64,
    label: &'static str,
) -> Ln {
    Ln {
        product,
        name,
        dept,
        qty,
        unit,
        label,
        changes: vec![],
        refund: false,
    }
}

fn transaction(lines: &[Ln]) -> Value {
    let tx_lines: Vec<Value> = lines
        .iter()
        .map(|l| {
            let mut vat = json!({ "label": l.label, "price": eur(l.qty * l.unit) });
            if !l.changes.is_empty() {
                vat["priceChanges"] = l
                    .changes
                    .iter()
                    .map(|c| {
                        json!({
                            "amount": eur(c.cents),
                            "groupingId": c.grouping,
                            "id": c.id,
                            "name": c.name,
                            "scope": c.scope,
                            "type": c.kind,
                        })
                    })
                    .collect();
            }
            let mut product = json!({
                "departmentId": l.dept.0,
                "departmentName": l.dept.1,
                "productId": l.product.to_string(),
                "productName": l.name,
                "quantity": eur(l.qty * 100),
                "quantityType": "PIECE",
                "unitPrice": eur(l.unit),
                "vats": [vat],
            });
            if l.refund {
                product["negQuantityReason"] = json!("REFUND");
            }
            json!({ "lineTotal": eur(l.net()), "lineType": "SINGLE_PRODUCT", "mainProduct": product })
        })
        .collect();
    json!({
        "transactionLines": tx_lines,
        "transactionTotal": eur(lines.iter().map(Ln::net).sum::<i64>()),
    })
}

/// The FDM's VAT calculation of a ticket: per label, the taxable amount is
/// the total divided by 1 + rate, rounded to the cent.
fn vat_calc(lines: &[Ln]) -> Value {
    let mut per_label: BTreeMap<&str, i64> = BTreeMap::new();
    for l in lines {
        *per_label.entry(l.label).or_default() += l.net();
    }
    per_label
        .into_iter()
        .map(|(label, total)| {
            let rate = match label {
                "A" => 21.0,
                "B" => 12.0,
                "C" => 6.0,
                _ => 0.0,
            };
            let base = base_of(total, rate);
            json!({
                "label": label,
                "outOfScope": false,
                "rate": rate,
                "taxableAmount": eur(base),
                "totalAmount": eur(total),
                "vatAmount": eur(total - base),
            })
        })
        .collect()
}

struct Pay {
    kind: &'static str,
    id: &'static str,
    name: &'static str,
    cents: i64,
    amount_type: &'static str,
}

fn pay(kind: &'static str, cents: i64) -> Pay {
    let (id, name) = match kind {
        "CASH" => ("1", "Cash"),
        "CARD_DEBIT" => ("2", "Card"),
        "CHEQUE_MEAL" => ("3", "Meal voucher"),
        "APP" => ("4", "App"),
        _ => ("5", "Online"),
    };
    Pay {
        kind,
        id,
        name,
        cents,
        amount_type: "PAYMENT",
    }
}

fn financials(pays: &[Pay]) -> Value {
    pays.iter()
        .map(|p| {
            json!({
                "amount": eur(p.cents),
                "amountType": p.amount_type,
                "id": p.id,
                "inputMethod": "MANUAL",
                "name": p.name,
                "type": p.kind,
            })
        })
        .collect()
}

/// Writes the fixture events and the rows around them.
struct Builder {
    conn: Connection,
    sequence: BTreeMap<String, i64>,
    counters: BTreeMap<String, i64>,
    total_counter: i64,
    ticket_no: i64,
    clock: u32,
    secrets: Vec<String>,
    period: String,
    date: String,
}

impl Builder {
    fn tick(&mut self) -> u32 {
        self.clock += 1;
        self.clock
    }

    /// Archive one event the way the engine does: the request as sent, the
    /// FDM answer, the columns of the answer.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "event bodies are built inline with json! at every call site"
    )]
    fn event(
        &mut self,
        event_type: &str,
        operation: &str,
        order_id: Option<i64>,
        employee: &str,
        body: Value,
        vat_calc: Option<Value>,
    ) -> (i64, Value) {
        self.ticket_no += 1;
        let t = self.tick();
        let pos_dt = format!("{}T{:02}:{:02}:07.000+02:00", self.date, 9 + t / 60, t % 60);
        let fdm_dt = format!("{}T{:02}:{:02}:08.000Z", self.date, 7 + t / 60, t % 60);
        let mut request = Map::new();
        request.insert("bookingDate".into(), json!(self.date));
        request.insert("bookingPeriodId".into(), json!(self.period));
        request.insert("deviceId".into(), json!("F0:1D:3D:CC:13:C8"));
        request.insert("employeeId".into(), json!(employee));
        request.insert("estNo".into(), json!("2000000042"));
        request.insert("language".into(), json!("FR"));
        request.insert("posDateTime".into(), json!(pos_dt));
        request.insert("posFiscalTicketNo".into(), json!(self.ticket_no));
        request.insert("posId".into(), json!(POS_ID));
        request.insert("posSwVersion".into(), json!(SW_VERSION));
        request.insert("terminalId".into(), json!(TERMINAL_ID));
        request.insert("ticketMedium".into(), json!("PAPER"));
        request.insert("vatNo".into(), json!("BE0000000097"));
        for (k, v) in body.as_object().expect("event body is an object") {
            request.insert(k.clone(), v.clone());
        }
        // Training events are signed and archived under the label T.
        let label = if event_type == "T" { "T" } else { event_type };
        let event_counter = {
            let c = self.counters.entry(label.to_string()).or_default();
            *c += 1;
            *c
        };
        self.total_counter += 1;
        let signature = format!(
            "SIG{}{:04}pQ9xT2vLw8KfH3mZc7YdR5uN1aBjE6sGoXi4WtV0yCqUeIlDk",
            label, self.total_counter
        );
        let short_signature = format!(
            "{:040x}",
            0xe2c4_3d6b_be9b_e265_u64 + u64::try_from(self.total_counter).unwrap()
        );
        let url = format!("https://gs2.be/?v={}-{}", FDM_ID, self.total_counter);
        self.secrets.push(signature.clone());
        self.secrets.push(short_signature.clone());
        self.secrets.push(url.clone());
        let fdm_ref = json!({
            "eventCounter": event_counter,
            "eventLabel": label,
            "fdmDateTime": fdm_dt,
            "fdmId": FDM_ID,
            "totalCounter": self.total_counter,
        });
        let mut response = json!({
            "bufferCapacityUsed": 0.25,
            "deviceId": "F0:1D:3D:CC:13:C8",
            "digitalSignature": signature,
            "eventOperation": operation,
            "fdmRef": fdm_ref,
            "fdmSwVersion": "1.2.0",
            "footer": [],
            "informations": [],
            "posDateTime": request["posDateTime"],
            "posFiscalTicketNo": self.ticket_no,
            "posId": POS_ID,
            "shortSignature": short_signature,
            "terminalId": TERMINAL_ID,
            "verificationUrl": url,
            "warnings": [],
        });
        if let Some(calc) = vat_calc {
            response["vatCalc"] = calc;
        }
        let sequence = {
            let s = self.sequence.entry(event_type.to_string()).or_default();
            *s += 1;
            *s
        };
        self.conn
            .execute(
                "INSERT INTO fiscal_events (event_type, sequence_number, order_id, pos_id, terminal_id,
                    user_insz, event_data_json, fdm_response_json, fdm_id, fdm_datetime, event_label,
                    short_signature, qr_data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    event_type,
                    sequence,
                    order_id,
                    POS_ID,
                    TERMINAL_ID,
                    employee,
                    Value::Object(request).to_string(),
                    response.to_string(),
                    FDM_ID,
                    fdm_dt,
                    label,
                    short_signature,
                    url,
                ],
            )
            .expect("archive an event");
        (self.conn.last_insert_rowid(), fdm_ref)
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::needless_pass_by_value,
        reason = "a fixture row with one argument per column, built inline"
    )]
    fn order(
        &self,
        id: i64,
        order_type: &str,
        table: Option<i64>,
        user: &str,
        status: &str,
        total: i64,
        source: &str,
        service: Option<&str>,
        hubrise_id: Option<&str>,
        raw: Option<String>,
    ) {
        self.conn
            .execute(
                "INSERT INTO orders (id, table_number, order_type, status, user_insz, total_cents,
                    subtotal_cents, hubrise_order_id, is_fiscal, source, service_type, raw_hubrise_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    id,
                    table,
                    order_type,
                    status,
                    user,
                    total,
                    hubrise_id,
                    i64::from(hubrise_id.is_none()),
                    source,
                    service,
                    raw,
                ],
            )
            .expect("insert an order");
    }

    fn order_lines(&self, order_id: i64, lines: &[Ln], pays: &[Pay]) {
        for l in lines {
            self.conn
                .execute(
                    "INSERT INTO order_lines (order_id, menu_item_id, quantity, unit_price_cents, vat_code,
                        name, department_id, department_name)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![order_id, l.product, l.qty, l.unit, l.label, l.name, l.dept.0, l.dept.1],
                )
                .expect("insert an order line");
        }
        for p in pays {
            let method = match p.kind {
                "CASH" => "cash",
                "CARD_DEBIT" => "card",
                "CHEQUE_MEAL" => "meal_voucher",
                _ => "other",
            };
            self.conn
                .execute(
                    "INSERT INTO payments (order_id, method, amount_cents) VALUES (?1, ?2, ?3)",
                    params![order_id, method, p.cents],
                )
                .expect("insert a payment");
        }
    }

    /// The Z of the open period `self.period`: both R events, the numbered
    /// row and the closing. The turnover is the engine's own computation.
    fn close_with_z(&mut self, z_no: i64, employee: &str) -> i64 {
        let events = pos_fiscal::archive::events_in_booking_period(&self.conn, &self.period)
            .expect("read the period");
        let report_events: Vec<_> = events
            .iter()
            .map(|e| {
                let mut r = pos_fiscal::report_totals::ReportEvent::from_archive(e)
                    .expect("readable event");
                if e.event_type == pos_fiscal::types::EventLabel::I {
                    let mut amount = 0.0;
                    for fdm_ref in r.request["fdmRefs"].as_array().into_iter().flatten() {
                        amount +=
                            pos_fiscal::archive::transaction_total_by_fdm_ref(&self.conn, fdm_ref)
                                .expect("read an invoiced sale")
                                .unwrap_or(0.0);
                    }
                    r.invoice_amount = Some((amount * 100.0).round() / 100.0);
                }
                r
            })
            .collect();
        let turnover = pos_fiscal::report_totals::turnover(&report_events);
        let device = json!([{ "posId": POS_ID, "terminalId": TERMINAL_ID,
            "firstPosDateTime": format!("{}T09:01:07.000+02:00", self.date),
            "lastPosDateTime": format!("{}T21:59:07.000+02:00", self.date) }]);
        let body = json!({
            "reportNo": z_no,
            "reportBookingDate": self.date,
            "posDevices": device,
            "fdmDevices": [],
            "turnover": serde_json::to_value(&turnover).expect("serialize the turnover"),
        });
        let (z_event, _) = self.event("R", "REPORT_TURNOVER_Z", None, employee, body, None);
        let user_body = json!({
            "reportNo": z_no,
            "reportBookingDate": self.date,
            "posDevices": device,
            "fdmDevices": [],
            "users": [{ "employeeId": employee, "totalAmount": 0.0, "socialEvents": [], "payments": [] }],
        });
        let (user_z_event, _) = self.event("R", "REPORT_USER_Z", None, employee, user_body, None);
        // The numbered row carries the request of the turnover Z.
        let mut request: Value = self
            .conn
            .query_row(
                "SELECT event_data_json FROM fiscal_events WHERE id = ?1",
                [z_event],
                |r| r.get::<_, String>(0),
            )
            .map(|s| serde_json::from_str(&s).unwrap())
            .unwrap();
        request["reportNo"] = json!(z_no);
        pos_db::queries::reports::save_z_report(
            &self.conn,
            &self.date,
            z_no,
            &request.to_string(),
            Some(z_event),
        )
        .expect("save the Z");
        pos_db::queries::booking::close_period(&self.conn, &self.period, z_event)
            .expect("close the period");
        user_z_event
    }
}

/// Write the fixture database at `path`.
#[expect(
    clippy::too_many_lines,
    reason = "one linear story of the fixture day; every step builds on the previous ones"
)]
pub(crate) fn fixture_db(path: &Path) -> Fixture {
    let conn = Connection::open(path).expect("create the fixture database");
    conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get::<_, String>(0))
        .expect("WAL");
    pos_db::schema::run_migrations(&conn).expect("migrate");
    // The menu code exists once the catalog migrations ran (feature
    // `catalog-sync` of pos-db); add it when this build did not enable it.
    let has_code = conn
        .prepare("SELECT 1 FROM pragma_table_info('menu_items') WHERE name = 'code'")
        .unwrap()
        .exists([])
        .unwrap();
    if !has_code {
        conn.execute_batch("ALTER TABLE menu_items ADD COLUMN code TEXT;")
            .unwrap();
    }

    let mut secrets: Vec<String> = Vec::new();
    // Users, with INSZ, names and bcrypt PIN hashes.
    let users = [
        (
            ADMIN_INSZ,
            "Aiko Tanaka",
            "admin",
            "$2b$12$Qx7mT3yV1nLw8KfH3mZc7eR5uN1aBjE6sGoXi4WtV0yCqUeIlDkPa",
        ),
        (
            CASHIER_INSZ,
            "Luc Peeters",
            "cashier",
            "$2b$12$Zk2pD9sW4aHc6VbN8mXr1uJ3tY5eQfL7oGiA0wRnSdCvMhKxEyTqO",
        ),
        (
            WAITER_INSZ,
            "Sofie Janssens",
            "waiter",
            "$2a$10$Hn5vB1cX8zLq3MdR7tYk9eW2uJ6fAoPsGi4NxT0yCrUeIlDkQbVwS",
        ),
    ];
    for (insz, name, role, hash) in users {
        conn.execute(
            "INSERT INTO users (insz, name, role, pin_hash) VALUES (?1, ?2, ?3, ?4)",
            params![insz, name, role, hash],
        )
        .unwrap();
        secrets.extend([insz.to_string(), name.to_string(), hash.to_string()]);
    }
    for part in ["Aiko", "Tanaka", "Peeters", "Janssens"] {
        secrets.push(part.to_string());
    }

    // The menu.
    conn.execute_batch(
        "INSERT INTO categories (id) VALUES (1), (2), (3);
         INSERT INTO menu_items (id, category_id, price_cents, vat_code, code) VALUES
            (101, 1, 650, 'B', 'S1'), (102, 2, 450, 'A', 'D4'), (103, 1, 1890, 'C', 'P12'),
            (104, 1, 322, 'C', NULL), (105, 3, 2100, 'C', 'M7'), (106, 3, 600, 'C', 'G2'),
            (107, 1, 600, 'C', 'Y1');",
    )
    .unwrap();

    // The HubRise connection and an event payload carry a token and a customer.
    let token = "hr_at_7f3k2Jq9XvB1mLw8KfH3mZc7YdR5uN1aBjE6sGoXi4WtV0yCqUeIlDk";
    conn.execute(
        "INSERT INTO hubrise_connection (id, access_token, account_id, account_name, location_id,
            location_name, catalog_id, scope, base_url)
         VALUES (1, ?1, 'acc01', 'Restaurant Example', 'loc01', 'Liege', 'cat01', 'location', 'https://api.hubrise.com/v1')",
        [token],
    )
    .unwrap();
    secrets.push(token.to_string());
    let customer = json!({
        "first_name": "Jean", "last_name": "Dupont", "email": "jean.dupont@example.com",
        "phone": "+32471234567", "address_1": "Rue de la Station 12", "postal_code": "4000", "city": "Liege",
    });
    for s in [
        "Jean",
        "Dupont",
        "jean.dupont@example.com",
        "+32471234567",
        "32471234567",
        "Rue de la Station 12",
    ] {
        secrets.push(s.to_string());
    }
    let raw_hubrise = |id: &str, service: &str| {
        json!({ "id": id, "service_type": service, "customer": customer, "status": "accepted" })
            .to_string()
    };
    conn.execute(
        "INSERT INTO hubrise_events (id, resource_type, event_type, payload_json)
         VALUES ('ev01', 'order', 'create', ?1)",
        [json!({ "new_state": { "customer": customer } }).to_string()],
    )
    .unwrap();

    let mut b = Builder {
        conn,
        sequence: BTreeMap::new(),
        counters: BTreeMap::new(),
        total_counter: 49,
        ticket_no: 0,
        clock: 0,
        secrets,
        period: "bp-fixture-0003".into(),
        date: "2026-10-03".into(),
    };

    // ── Earlier day: Z 41, an automatic Z of the system identity ─────
    open_period(&b.conn, "bp-fixture-0003", "2026-10-03");
    b.order(
        20,
        "takeaway",
        None,
        CASHIER_INSZ,
        "closed",
        600,
        "pos_local",
        None,
        None,
        None,
    );
    let l = vec![line(107, "Yakitori", ("3", "Grill"), 1, 600, "C")];
    let p = vec![pay("CASH", 600)];
    let (earlier_sale, _) = b.event(
        "N",
        "SALE",
        Some(20),
        CASHIER_INSZ,
        json!({ "transaction": transaction(&l), "financials": financials(&p) }),
        Some(vat_calc(&l)),
    );
    b.order_lines(20, &l, &p);
    b.close_with_z(41, pos_core::user::SYSTEM_INSZ);

    // ── The day of Z 42 ─────────────────────────────────────────────
    b.period = "bp-fixture-0004".into();
    b.date = "2026-10-04".into();
    open_period(&b.conn, "bp-fixture-0004", "2026-10-04");
    let mut excluded = Vec::new();
    let mut tickets = Vec::new();
    let mut all_lines = 0;

    // Presence (S).
    let (s_event, _) = b.event("S", "WORK_IN", None, WAITER_INSZ, json!({}), None);
    excluded.push(s_event);

    // Sale 1: table 7, two lines, a 10 % discount on the first, card.
    b.order(
        1,
        "dine_in",
        Some(7),
        WAITER_INSZ,
        "closed",
        2070,
        "pos_local",
        None,
        None,
        None,
    );
    let mut first = line(101, "Nigiri saumon", ("1", "Cuisine"), 2, 650, "B");
    first.changes.push(Chg {
        grouping: 1,
        id: "PROMO10",
        name: "Promo -10 %",
        scope: "LINE",
        kind: "PUBLIC",
        cents: -130,
    });
    let l1 = vec![
        first,
        line(102, "Biere Asahi", ("2", "Boissons"), 2, 450, "A"),
    ];
    let cost_center = json!({ "id": "T07", "reference": "O1", "type": "TABLE" });
    // The order was registered first (P, ORDER), then the sale (N).
    let (p_event, _) = b.event(
        "P",
        "ORDER",
        Some(1),
        WAITER_INSZ,
        json!({ "costCenter": cost_center, "transaction": transaction(&l1) }),
        None,
    );
    excluded.push(p_event);
    let p1 = vec![pay("CARD_DEBIT", 2070)];
    let (sale1, sale1_ref) = b.event(
        "N",
        "SALE",
        Some(1),
        WAITER_INSZ,
        json!({ "costCenter": cost_center, "transaction": transaction(&l1), "financials": financials(&p1) }),
        Some(vat_calc(&l1)),
    );
    b.order_lines(1, &l1, &p1);
    tickets.push(ExpectedTicket {
        ticket_id: sale1,
        channel: "table",
        kind: "sale",
        table_number: Some(7),
        total_cents: 2070,
        unpaid: false,
        lines: 2,
        employee: WAITER_INSZ,
    });

    // Sale 2: takeaway, meal voucher and cash with a 1 cent rounding.
    b.order(
        2,
        "takeaway",
        None,
        CASHIER_INSZ,
        "closed",
        2534,
        "pos_local",
        None,
        None,
        None,
    );
    let l2 = vec![
        line(103, "Plateau 12 pieces", ("1", "Cuisine"), 1, 1890, "C"),
        line(104, "Soupe miso", ("1", "Cuisine"), 2, 322, "C"),
    ];
    let mut p2 = vec![pay("CHEQUE_MEAL", 1000), pay("CASH", 1534)];
    p2.push(Pay {
        amount_type: "ROUNDING",
        ..pay("CASH", 1)
    });
    let (sale2, _) = b.event(
        "N",
        "SALE",
        Some(2),
        CASHIER_INSZ,
        json!({ "transaction": transaction(&l2), "financials": financials(&p2) }),
        Some(vat_calc(&l2)),
    );
    b.order_lines(2, &l2, &p2);
    tickets.push(ExpectedTicket {
        ticket_id: sale2,
        channel: "takeaway",
        kind: "sale",
        table_number: None,
        total_cents: 2534,
        unpaid: false,
        lines: 2,
        employee: CASHIER_INSZ,
    });

    // Sale 3: online delivery of the webshop, a ticket-wide discount, paid online.
    b.order(
        3,
        "takeaway",
        None,
        pos_core::user::SYSTEM_INSZ,
        "closed",
        2430,
        "webshop",
        Some("delivery"),
        Some("hr-order-0003"),
        Some(raw_hubrise("hr-order-0003", "delivery")),
    );
    let mut bento = line(105, "Menu bento", ("4", "Menus"), 1, 2100, "C");
    bento.changes.push(Chg {
        grouping: 1,
        id: "WELCOME10",
        name: "Code WELCOME10",
        scope: "EVENT",
        kind: "PUBLIC",
        cents: -210,
    });
    let mut gyoza = line(106, "Gyoza", ("3", "Grill"), 1, 600, "C");
    gyoza.changes.push(Chg {
        grouping: 1,
        id: "WELCOME10",
        name: "Code WELCOME10",
        scope: "EVENT",
        kind: "PUBLIC",
        cents: -60,
    });
    let l3 = vec![bento, gyoza];
    let p3 = vec![pay("ONLINE", 2430)];
    let (sale3, _) = b.event(
        "N",
        "SALE",
        Some(3),
        pos_core::user::SYSTEM_INSZ,
        json!({ "transaction": transaction(&l3), "financials": financials(&p3) }),
        Some(vat_calc(&l3)),
    );
    b.order_lines(3, &l3, &p3);
    tickets.push(ExpectedTicket {
        ticket_id: sale3,
        channel: "delivery",
        kind: "sale",
        table_number: None,
        total_cents: 2430,
        unpaid: false,
        lines: 2,
        employee: pos_core::user::SYSTEM_INSZ,
    });

    // Refund of one beer of sale 1, in cash, by the admin: a new takeaway order.
    b.order(
        4,
        "takeaway",
        None,
        ADMIN_INSZ,
        "closed",
        -450,
        "pos_local",
        None,
        None,
        None,
    );
    let mut beer = line(102, "Biere Asahi", ("2", "Boissons"), -1, 450, "A");
    beer.refund = true;
    let l4 = vec![beer];
    let p4 = vec![pay("CASH", -450)];
    let (refund, _) = b.event(
        "N",
        "SALE",
        Some(4),
        ADMIN_INSZ,
        json!({ "fdmRef": sale1_ref, "transaction": transaction(&l4), "financials": financials(&p4) }),
        Some(vat_calc(&l4)),
    );
    b.order_lines(4, &l4, &p4);
    tickets.push(ExpectedTicket {
        ticket_id: refund,
        channel: "table",
        kind: "refund",
        table_number: Some(7),
        total_cents: -450,
        unpaid: false,
        lines: 1,
        employee: ADMIN_INSZ,
    });

    // Sale 5: table 9 closed unpaid (no payment line).
    b.order(
        5,
        "dine_in",
        Some(9),
        WAITER_INSZ,
        "closed",
        650,
        "pos_local",
        None,
        None,
        None,
    );
    let l5 = vec![line(101, "Nigiri saumon", ("1", "Cuisine"), 1, 650, "B")];
    let (unpaid, _) = b.event(
        "N",
        "SALE",
        Some(5),
        WAITER_INSZ,
        json!({ "costCenter": { "id": "T09", "reference": "O5", "type": "TABLE" },
                "transaction": transaction(&l5), "financials": [] }),
        Some(vat_calc(&l5)),
    );
    b.order_lines(5, &l5, &[]);
    tickets.push(ExpectedTicket {
        ticket_id: unpaid,
        channel: "table",
        kind: "sale",
        table_number: Some(9),
        total_cents: 650,
        unpaid: true,
        lines: 1,
        employee: WAITER_INSZ,
    });

    // Sale 6: collection order of a platform, paid in its app.
    b.order(
        6,
        "takeaway",
        None,
        pos_core::user::SYSTEM_INSZ,
        "closed",
        1890,
        "deliveroo",
        Some("collection"),
        Some("hr-order-0006"),
        Some(raw_hubrise("hr-order-0006", "collection")),
    );
    let l6 = vec![line(
        103,
        "Plateau 12 pieces",
        ("1", "Cuisine"),
        1,
        1890,
        "C",
    )];
    let p6 = vec![pay("APP", 1890)];
    let (collection, _) = b.event(
        "N",
        "SALE",
        Some(6),
        pos_core::user::SYSTEM_INSZ,
        json!({ "transaction": transaction(&l6), "financials": financials(&p6) }),
        Some(vat_calc(&l6)),
    );
    b.order_lines(6, &l6, &p6);
    tickets.push(ExpectedTicket {
        ticket_id: collection,
        channel: "online",
        kind: "sale",
        table_number: None,
        total_cents: 1890,
        unpaid: false,
        lines: 1,
        employee: pos_core::user::SYSTEM_INSZ,
    });

    // A copy (C) of sale 1, a training sale (T), a drawer opening (F), an invoice (I).
    let (copy, _) = b.event(
        "C",
        "COPY",
        None,
        CASHIER_INSZ,
        json!({ "fdmRef": sale1_ref }),
        None,
    );
    excluded.push(copy);
    b.order(
        7,
        "dine_in",
        Some(3),
        CASHIER_INSZ,
        "closed",
        650,
        "pos_local",
        None,
        None,
        None,
    );
    let l7 = vec![line(101, "Nigiri saumon", ("1", "Cuisine"), 1, 650, "B")];
    let (training, _) = b.event(
        "T",
        "SALE",
        Some(7),
        CASHIER_INSZ,
        json!({ "costCenter": { "id": "T03", "reference": "O7", "type": "TABLE" },
                "transaction": transaction(&l7), "financials": financials(&[pay("CASH", 650)]) }),
        Some(vat_calc(&l7)),
    );
    excluded.push(training);
    let (drawer, _) = b.event(
        "F",
        "DRAWER_OPEN",
        None,
        ADMIN_INSZ,
        json!({ "drawer": { "id": "1", "name": "Tiroir" } }),
        None,
    );
    excluded.push(drawer);
    let customer_vat = "BE0999999999";
    b.secrets.push(customer_vat.to_string());
    let (invoice, _) = b.event(
        "I",
        "INVOICE",
        None,
        ADMIN_INSZ,
        json!({ "invoiceNo": "2026-0007", "customerVatNo": customer_vat, "fdmRefs": [sale1_ref] }),
        None,
    );
    excluded.push(invoice);

    // The e-mail delivery of the first ticket.
    b.conn
        .execute(
            "INSERT INTO ticket_deliveries (fiscal_event_id, medium, recipient, recipient_hint, ticket_text,
                verification_url, status)
             VALUES (?1, 'DIGITAL', 'marie.martin@example.com', 'm***@e***.com',
                     'TICKET Merci Marie Martin pour votre visite', 'https://gs2.be/?v=1', 'pending')",
            [sale1],
        )
        .unwrap();
    for s in ["marie.martin@example.com", "Marie Martin", "m***@e***.com"] {
        b.secrets.push(s.to_string());
    }

    for t in &tickets {
        all_lines += t.lines;
    }
    let user_z = b.close_with_z(42, ADMIN_INSZ);
    excluded.push(user_z);
    let closing_event_id: i64 = b
        .conn
        .query_row(
            "SELECT closing_event_id FROM booking_periods WHERE id = 'bp-fixture-0004'",
            [],
            |r| r.get(0),
        )
        .unwrap();

    // The next day is open; a training Z does not close it.
    b.period = "bp-fixture-0005".into();
    b.date = "2026-10-05".into();
    open_period(&b.conn, "bp-fixture-0005", "2026-10-05");
    let (training_z, _) = b.event(
        "T",
        "REPORT_TURNOVER_Z",
        None,
        ADMIN_INSZ,
        json!({ "reportNo": 43, "reportBookingDate": "2026-10-05", "posDevices": [], "fdmDevices": [],
                "turnover": { "transactions": [], "departments": [], "vats": [], "payments": [], "negQuantities": [] } }),
        None,
    );

    let Builder { conn, secrets, .. } = b;
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
        r.get::<_, i64>(0)
    })
    .unwrap();
    drop(conn);

    let mut secrets = secrets;
    secrets.sort();
    secrets.dedup();
    Fixture {
        db_path: path.to_path_buf(),
        z_no: 42,
        booking_date: "2026-10-04".into(),
        period_id: "bp-fixture-0004".into(),
        closing_event_id,
        earlier_z_no: 41,
        tickets,
        lines: all_lines,
        z_total_cents: 9124,
        z_vat: vec![
            ("A", 2100, 372, 78, 450),
            ("B", 1200, 1625, 195, 1820),
            ("C", 600, 6466, 388, 6854),
        ],
        z_payments: vec![
            ("app", "payment", 1890),
            ("card", "payment", 2070),
            ("cash", "payment", 1084),
            ("cash", "rounding", 1),
            ("meal_voucher", "payment", 1000),
            ("online", "payment", 2430),
        ],
        excluded_event_ids: excluded,
        other_event_ids: vec![training_z, earlier_sale],
        secrets,
    }
}

fn open_period(conn: &Connection, id: &str, date: &str) {
    conn.execute(
        "INSERT INTO booking_periods (id, booking_date, opened_at) VALUES (?1, ?2, ?3)",
        params![id, date, format!("{date}T07:30:00.000Z")],
    )
    .unwrap();
}

// ── Opening and building ───────────────────────────────────────────

/// Open a database read-only, as the export does with its snapshot.
pub(crate) fn open_ro(path: &Path) -> Connection {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only")
}

/// The pseudonym key of the tests.
pub(crate) fn test_key() -> [u8; 32] {
    [0x42; 32]
}

pub(crate) fn test_meta() -> StatsMeta {
    StatsMeta {
        export_id: "20261004-z000042".into(),
        pos_id: POS_ID.into(),
        terminal_id: TERMINAL_ID.into(),
        pos_sw_version: SW_VERSION.into(),
        generated_at: "2026-10-05T01:00:00Z".into(),
    }
}

/// The Z reference of the fixture's Z 42, read through the export's own lookup.
pub(crate) fn fixture_z(fx: &Fixture) -> ZRef {
    let conn = open_ro(&fx.db_path);
    real_zs_after(&conn, Some(fx.earlier_z_no))
        .expect("list the Z reports")
        .into_iter()
        .find(|z| z.z_report_no == fx.z_no)
        .expect("Z 42 is listed")
}

/// Build the statistics file of Z 42 of the fixture at `out`.
pub(crate) fn build_fixture_stats(fx: &Fixture, out: &Path) -> StatsSummary {
    let conn = open_ro(&fx.db_path);
    build_stats(&conn, &fixture_z(fx), &test_meta(), &test_key(), out)
        .expect("build the statistics")
}

// ── The privacy proof ──────────────────────────────────────────────

/// Substrings that must not appear in any column name of a statistics file.
pub(crate) const FORBIDDEN_COLUMN_PARTS: [&str; 13] = [
    "insz",
    "niss",
    "pin",
    "hash",
    "token",
    "secret",
    "password",
    "mail",
    "phone",
    "address",
    "customer",
    "signature",
    "qr",
];

/// Names of the tables of an SQLite file (without internal tables), sorted.
pub(crate) fn table_names(conn: &Connection) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Column names of a table, in order.
pub(crate) fn column_names(conn: &Connection, table: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// True when `s` holds a run of at least `n` ASCII digits.
pub(crate) fn has_digit_run(s: &str, n: usize) -> bool {
    let mut run = 0;
    for c in s.chars() {
        if c.is_ascii_digit() {
            run += 1;
            if run >= n {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// True when `s` has the exact shape of a `user_ref` pseudonym: `u-` and 16
/// lowercase hex characters (`pos_export::pseudonym::user_ref`). Its hex
/// part comes from a hash under a random key, so it can hold a long digit
/// run by chance (an 11-digit run in about 2 % of pseudonyms).
pub(crate) fn is_pseudonym(s: &str) -> bool {
    s.strip_prefix("u-").is_some_and(|hex| {
        hex.len() == 16 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

/// Fail unless the SQLite file at `path` holds no personal data.
///
/// 1. No column name contains one of [`FORBIDDEN_COLUMN_PARTS`] (any case),
///    and no column is called `name` or `user_name` (`product_name` is fine).
/// 2. No TEXT value of any table contains a run of 11 digits (an INSZ), an
///    `@`, a bcrypt prefix (`$2a$`, `$2b$`, `$2y$`) or any of `secrets`. A
///    value of the exact pseudonym shape ([`is_pseudonym`]) is spared the
///    digit run rule only (a valid pseudonym never trips it); it is still
///    checked against `secrets`, which hold the fixture's INSZ.
/// 3. The raw bytes of the file contain none of the `secrets` either, so
///    nothing survives in free pages or in the schema text.
pub(crate) fn assert_no_personal_data(path: &Path, secrets: &[String]) {
    let conn = open_ro(path);
    for table in table_names(&conn) {
        for column in column_names(&conn, &table) {
            let lower = column.to_lowercase();
            for part in FORBIDDEN_COLUMN_PARTS {
                assert!(
                    !lower.contains(part),
                    "column {table}.{column} matches {part:?}"
                );
            }
            assert!(
                lower != "name" && lower != "user_name",
                "column {table}.{column} is a personal name column"
            );
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT \"{column}\" FROM \"{table}\" WHERE typeof(\"{column}\") = 'text'"
                ))
                .unwrap();
            let values: Vec<String> = stmt
                .query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            for value in values {
                assert!(
                    is_pseudonym(&value) || !has_digit_run(&value, 11),
                    "{table}.{column} holds 11 digits: {value:?}"
                );
                assert!(
                    !value.contains('@'),
                    "{table}.{column} holds an @: {value:?}"
                );
                for prefix in ["$2a$", "$2b$", "$2y$"] {
                    assert!(
                        !value.contains(prefix),
                        "{table}.{column} holds a bcrypt hash: {value:?}"
                    );
                }
                for secret in secrets {
                    assert!(
                        !value.contains(secret.as_str()),
                        "{table}.{column} holds a secret: {secret:?}"
                    );
                }
            }
        }
    }
    let bytes = std::fs::read(path).expect("read the file");
    for secret in secrets {
        let needle = secret.as_bytes();
        assert!(
            needle.is_empty() || !bytes.windows(needle.len()).any(|w| w == needle),
            "the file bytes contain a secret: {secret:?}"
        );
    }
}

// ── The outbox tests (card X1-T5) ──────────────────────────────────

/// What the outbox tests need to export: settings, the owner's secret key
/// and the bearer token (random per run, nothing secret is committed).
pub(crate) struct ExportKit {
    pub settings: pos_export::settings::ExportSettings,
    /// `AGE-SECRET-KEY-1...` of the owner, to decrypt the backups.
    pub owner_secret: String,
    pub token: String,
}

/// Settings for the endpoint `url` with a fresh owner identity and a token
/// file in `dir`, keeping the exports `keep_days` days.
pub(crate) fn export_kit(dir: &Path, url: &str, keep_days: i64) -> ExportKit {
    use std::str::FromStr;
    let (owner_secret, owner_public) = pos_export::crypt::generate_identity();
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("random bytes");
    let token = format!("tok-{}", hex::encode(bytes));
    let token_file = dir.join("export.token");
    std::fs::write(&token_file, &token).expect("write the token file");
    ExportKit {
        settings: pos_export::settings::ExportSettings {
            url: url.trim_end_matches('/').to_string(),
            token_file,
            recipients: vec![age::x25519::Recipient::from_str(&owner_public).expect("recipient")],
            keep_days,
        },
        owner_secret,
        token,
    }
}

/// Append a real Z report `z_no` for the booking date `date` to the fixture
/// database: an empty period closed by a copy of the closing event of Z 42.
/// Returns the new export id.
pub(crate) fn add_empty_z(fx: &Fixture, z_no: i64, date: &str) -> String {
    let conn = Connection::open(&fx.db_path).expect("open the fixture database");
    let mut body: Value = conn
        .query_row(
            "SELECT event_data_json FROM fiscal_events WHERE id = ?1",
            [fx.closing_event_id],
            |r| r.get::<_, String>(0),
        )
        .map(|s| serde_json::from_str(&s).unwrap())
        .expect("read the closing event of Z 42");
    let period = format!("period-z{z_no}");
    body["reportNo"] = json!(z_no);
    body["reportBookingDate"] = json!(date);
    body["bookingDate"] = json!(date);
    body["bookingPeriodId"] = json!(period);
    conn.execute(
        "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz,
                event_data_json, fdm_response_json, fdm_id, fdm_datetime, event_label,
                short_signature, qr_data)
         SELECT event_type, (SELECT MAX(sequence_number) + 1 FROM fiscal_events WHERE event_type = 'R'),
                pos_id, terminal_id, user_insz, ?2, fdm_response_json, fdm_id, fdm_datetime,
                event_label, short_signature, qr_data
         FROM fiscal_events WHERE id = ?1",
        params![fx.closing_event_id, body.to_string()],
    )
    .expect("insert the Z event");
    let event_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO booking_periods (id, booking_date, opened_at, closed_at, closing_event_id)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            period,
            date,
            format!("{date}T07:30:00.000Z"),
            format!("{date}T22:00:00.000Z"),
            event_id
        ],
    )
    .expect("insert the period");
    conn.execute(
        "INSERT INTO z_reports (report_date, sequence_number, data_json, fiscal_event_id)
         VALUES (?1, ?2, ?3, ?4)",
        params![date, z_no, body.to_string(), event_id],
    )
    .expect("insert the Z report");
    pos_export::naming::export_id(date, z_no).expect("export id")
}
