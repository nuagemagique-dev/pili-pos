//! Helpers shared by the integration tests: a synthetic archive builder, copies of a
//! database, a small CSV reader and the export runner.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pos_accounting::{Decimal, ExportRequest, ExportSummary, PeriodSpec, Profile};
use rusqlite::Connection;
use serde_json::{Value, json};

pub const FDM_ID: &str = "SYN00000001";

/// A line of a synthetic sale: department id, VAT label, price with VAT.
pub struct L(pub &'static str, pub &'static str, pub f64);

/// A payment line: PaymentType, amountType, amount.
pub struct P(pub &'static str, pub &'static str, pub f64);

#[derive(Default)]
pub struct SaleSpec {
    pub lines: Vec<(String, String, f64)>,
    pub payments: Vec<(String, String, f64)>,
    pub cost_center: Option<&'static str>,
    pub refers_to: Option<Value>,
}

impl SaleSpec {
    pub fn new(lines: &[L], payments: &[P]) -> Self {
        Self {
            lines: lines
                .iter()
                .map(|l| (l.0.to_string(), l.1.to_string(), l.2))
                .collect(),
            payments: payments
                .iter()
                .map(|p| (p.0.to_string(), p.1.to_string(), p.2))
                .collect(),
            ..Self::default()
        }
    }

    pub fn table(mut self) -> Self {
        self.cost_center = Some("TABLE");
        self
    }

    pub fn online(mut self) -> Self {
        self.cost_center = Some("WEBSHOP");
        self
    }

    pub fn refund_of(mut self, fdm_ref: Value) -> Self {
        self.refers_to = Some(fdm_ref);
        self
    }
}

fn rate(label: &str) -> f64 {
    match label {
        "A" => 21.0,
        "B" => 12.0,
        "C" => 6.0,
        _ => 0.0,
    }
}

fn cents(v: f64) -> i64 {
    (v * 100.0).round() as i64
}

fn eur(c: i64) -> f64 {
    c as f64 / 100.0
}

/// The FDM's VAT calculation of lines: per label, base and VAT from the total.
pub fn vat_calc(lines: &[(String, String, f64)]) -> Value {
    let mut totals: std::collections::BTreeMap<String, i64> = Default::default();
    for (_, label, price) in lines {
        *totals.entry(label.clone()).or_insert(0) += cents(*price);
    }
    Value::Array(
        totals
            .into_iter()
            .map(|(label, total)| {
                let r = rate(&label);
                let base = (total as f64 / (1.0 + r / 100.0)).round() as i64;
                json!({"label": label, "rate": r, "taxableAmount": eur(base),
                       "vatAmount": eur(total - base), "totalAmount": eur(total)})
            })
            .collect(),
    )
}

/// A synthetic archive in a real migrated database file.
pub struct Synth {
    pub dir: tempfile::TempDir,
    pub path: PathBuf,
    conn: Connection,
    sequence: HashMap<String, i64>,
    total_counter: i64,
    sales: HashMap<String, Vec<(Value, Value)>>,
    ticket_no: i64,
}

impl Synth {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pos.db");
        let conn = Connection::open(&path).unwrap();
        pos_db::schema::run_migrations(&conn).unwrap();
        Self {
            dir,
            path,
            conn,
            sequence: HashMap::new(),
            total_counter: 0,
            sales: HashMap::new(),
            ticket_no: 0,
        }
    }

    pub fn open_period(&self, id: &str, date: &str) {
        self.conn
            .execute(
                "INSERT INTO booking_periods (id, booking_date, opened_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, date, format!("{date}T06:00:00.000Z")],
            )
            .unwrap();
        self.flush();
    }

    fn insert(&mut self, label: &str, period: &str, mut request: Value, response: Value) -> i64 {
        self.ticket_no += 1;
        let object = request.as_object_mut().unwrap();
        object.insert("bookingPeriodId".into(), json!(period));
        object.insert("posFiscalTicketNo".into(), json!(self.ticket_no));
        object
            .entry("posDateTime")
            .or_insert(json!("2026-10-01T12:00:00.000+02:00"));
        let seq = self.sequence.entry(label.to_string()).or_insert(0);
        *seq += 1;
        let fdm_ref = response["fdmRef"].clone();
        self.conn
            .execute(
                "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz,
                    event_data_json, fdm_response_json, fdm_id, fdm_datetime, event_label)
                 VALUES (?1, ?2, 'CPOS0031234567', 'POS-1', '00000000029', ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    label,
                    *seq,
                    request.to_string(),
                    response.to_string(),
                    fdm_ref["fdmId"].as_str(),
                    fdm_ref["fdmDateTime"].as_str(),
                    label,
                ],
            )
            .unwrap();
        let id = self.conn.last_insert_rowid();
        self.flush();
        id
    }

    /// The tool reads the file in immutable mode: keep everything in the main file.
    fn flush(&self) {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
    }

    fn next_ref(&mut self, label: &str) -> Value {
        self.total_counter += 1;
        let event_counter = self.sequence.get(label).copied().unwrap_or(0) + 1;
        json!({"fdmId": FDM_ID, "fdmDateTime": format!("2026-10-01T10:{:02}:00.000Z", self.total_counter % 60),
               "eventLabel": label, "eventCounter": event_counter, "totalCounter": self.total_counter})
    }

    /// Sign a sale; returns its FDM reference (for a refund or an invoice).
    pub fn sale(&mut self, period: &str, spec: SaleSpec) -> Value {
        let lines: Vec<Value> = spec
            .lines
            .iter()
            .map(|(dept, label, price)| {
                json!({"mainProduct": {"productId": "1", "productName": "Item", "departmentId": dept,
                    "quantity": 1, "unitPrice": price,
                    "vats": [{"label": label, "price": price, "priceChanges": []}]}})
            })
            .collect();
        let financials: Vec<Value> = spec
            .payments
            .iter()
            .map(|(t, a, v)| json!({"id": "1", "name": t, "type": t, "amountType": a, "amount": v}))
            .collect();
        let mut request = json!({
            "transaction": {"transactionLines": lines, "transactionTotal": 0},
            "financials": financials,
            "employeeId": "75061189702",
        });
        match spec.cost_center {
            Some("TABLE") => {
                request["costCenter"] = json!({"id": "T4", "type": "TABLE", "reference": "T4"})
            }
            Some(other) => {
                request["costCenter"] = json!({"id": "9", "type": other, "reference": "W"})
            }
            None => {}
        }
        if let Some(r) = &spec.refers_to {
            request["fdmRef"] = r.clone();
        }
        let fdm_ref = self.next_ref("N");
        let response = json!({"fdmRef": fdm_ref, "vatCalc": vat_calc(&spec.lines)});
        self.insert("N", period, request.clone(), response.clone());
        self.sales
            .entry(period.to_string())
            .or_default()
            .push((request, response));
        fdm_ref
    }

    pub fn money(&mut self, period: &str, amount: f64, label: &str) {
        let request = json!({"financials": [{"id": "1", "name": label, "type": "CASH",
            "amount": amount, "amountType": "MONEY_IN_OUT"}]});
        let fdm_ref = self.next_ref("F");
        self.insert("F", period, request, json!({"fdmRef": fdm_ref}));
    }

    pub fn invoice(&mut self, period: &str, refs: &[Value], number: &str, vat_no: &str) {
        let request = json!({"customerVatNo": vat_no, "invoiceNo": number,
            "costCenter": {"id": "100", "type": "CUSTOMER", "reference": "FIN"}, "fdmRefs": refs});
        let fdm_ref = self.next_ref("I");
        self.insert("I", period, request, json!({"fdmRef": fdm_ref}));
    }

    /// An invoice with a request taken as is (a published fixture).
    pub fn invoice_raw(&mut self, period: &str, request: Value) {
        let fdm_ref = self.next_ref("I");
        self.insert("I", period, request, json!({"fdmRef": fdm_ref}));
    }

    /// An event the export must ignore: a pro forma (P), a training sale (T) or a copy (C).
    pub fn ignored(&mut self, label: &str, period: &str) {
        let request = json!({"transaction": {"transactionLines": [{"mainProduct": {"productId": "1",
            "departmentId": "10", "vats": [{"label": "A", "price": 99.0, "priceChanges": []}]}}],
            "transactionTotal": 99.0},
            "financials": [{"type": "CASH", "amountType": "PAYMENT", "amount": 99.0}]});
        let fdm_ref = self.next_ref(label);
        self.insert(
            label,
            period,
            request,
            json!({"fdmRef": fdm_ref, "vatCalc": []}),
        );
    }

    /// A sale whose FDM answer is missing (never signed).
    pub fn unsigned_sale(&mut self, period: &str) {
        self.ticket_no += 1;
        let request = json!({"bookingPeriodId": period, "transaction": {"transactionLines": [],
            "transactionTotal": 0}});
        self.conn
            .execute(
                "INSERT INTO fiscal_events (event_type, sequence_number, pos_id, terminal_id, user_insz, event_data_json)
                 VALUES ('N', 900, 'CPOS0031234567', 'POS-1', '00000000029', ?1)",
                [request.to_string()],
            )
            .unwrap();
    }

    /// Close the period with a Z report computed from its sales.
    pub fn close(&mut self, period: &str, z_no: i64) {
        let sales = self.sales.get(period).cloned().unwrap_or_default();
        let z = z_json(&sales);
        let request = json!({"turnover": z});
        let fdm_ref = self.next_ref("R");
        let id = self.insert("R", period, request, json!({"fdmRef": fdm_ref}));
        self.conn
            .execute(
                "INSERT INTO z_reports (report_date, sequence_number, data_json, fiscal_event_id)
                 VALUES ('2026-10-01', ?1, '{}', ?2)",
                rusqlite::params![z_no, id],
            )
            .unwrap();
        self.conn
            .execute(
                "UPDATE booking_periods SET closed_at = '2026-10-01T22:00:00.000Z', closing_event_id = ?2 WHERE id = ?1",
                rusqlite::params![period, id],
            )
            .unwrap();
        self.flush();
    }
}

/// The Z `turnover` object of a set of signed sales (independent of the library).
pub fn z_json(sales: &[(Value, Value)]) -> Value {
    let mut amount = 0i64;
    let mut vats: std::collections::BTreeMap<String, [i64; 4]> = Default::default();
    let mut payments: std::collections::BTreeMap<(String, String), i64> = Default::default();
    for (request, response) in sales {
        for line in request["transaction"]["transactionLines"]
            .as_array()
            .unwrap()
        {
            for vat in line["mainProduct"]["vats"].as_array().unwrap() {
                amount += cents(vat["price"].as_f64().unwrap());
            }
        }
        for v in response["vatCalc"].as_array().unwrap() {
            let e = vats
                .entry(v["label"].as_str().unwrap().to_string())
                .or_default();
            e[0] = (v["rate"].as_f64().unwrap() * 100.0) as i64;
            e[1] += cents(v["taxableAmount"].as_f64().unwrap());
            e[2] += cents(v["vatAmount"].as_f64().unwrap());
            e[3] += cents(v["totalAmount"].as_f64().unwrap());
        }
        for f in request["financials"].as_array().unwrap() {
            *payments
                .entry((
                    f["type"].as_str().unwrap().to_string(),
                    f["amountType"].as_str().unwrap().to_string(),
                ))
                .or_insert(0) += cents(f["amount"].as_f64().unwrap());
        }
    }
    let refunds = sales
        .iter()
        .filter(|(r, _)| r.get("fdmRef").is_some())
        .count();
    json!({
        "transactions": [{"eventLabel": "N", "ticketCount": sales.len(), "amount": eur(amount)}],
        "vats": vats.iter().map(|(l, v)| json!({"label": l, "rate": v[0] as f64 / 100.0,
            "taxableAmount": eur(v[1]), "vatAmount": eur(v[2]), "totalAmount": eur(v[3])})).collect::<Vec<_>>(),
        "payments": payments.iter().map(|((t, a), v)| json!({"id": "1", "name": t, "type": t,
            "amount": [{"type": a, "totalAmount": eur(*v)}]})).collect::<Vec<_>>(),
        "negQuantities": [{"negQuantityReason": "REFUND", "ticketCount": refunds, "amount": 0}],
    })
}

/// Copy a live database to a file with `VACUUM INTO` (a consistent snapshot).
pub fn snapshot(conn: &Connection, dest: &Path) {
    conn.execute("VACUUM INTO ?1", [dest.to_string_lossy().as_ref()])
        .unwrap();
}

pub fn request(db: &Path, out: &Path, period: PeriodSpec) -> ExportRequest {
    ExportRequest {
        db: db.to_path_buf(),
        period,
        out_parent: out.to_path_buf(),
        accounts: None,
        profile: Profile::Generic,
        decimal: Decimal::Comma,
        generated_at: "2026-10-17 10:00".to_string(),
    }
}

pub fn month(m: &str) -> PeriodSpec {
    PeriodSpec::Month(m.to_string())
}

pub fn run(db: &Path, out: &Path, period: PeriodSpec) -> pos_accounting::Result<ExportSummary> {
    pos_accounting::export(&request(db, out, period))
}

/// Read a file of the export folder as text, without the BOM.
pub fn read(folder: &Path, name: &str) -> String {
    let bytes = std::fs::read(folder.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    let text = String::from_utf8(bytes).unwrap();
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_string()
}

/// Rows of a CSV file (header first), `;` separated, no quoting support needed here.
pub fn rows(folder: &Path, name: &str) -> Vec<Vec<String>> {
    read(folder, name)
        .split("\r\n")
        .filter(|l| !l.is_empty())
        .map(|l| l.split(';').map(str::to_string).collect())
        .collect()
}

/// `(Σ debit, Σ credit)` in cents of a generic `journal.csv`, per document.
pub fn journal_balances(folder: &Path) -> Vec<(String, i64, i64)> {
    let parse = |s: &str| (s.parse::<f64>().unwrap() * 100.0).round() as i64;
    let mut out: Vec<(String, i64, i64)> = Vec::new();
    for row in rows(folder, "journal.csv").iter().skip(1) {
        let (d, c) = (parse(&row[5]), parse(&row[6]));
        match out.iter_mut().find(|(doc, _, _)| *doc == row[2]) {
            Some(e) => {
                e.1 += d;
                e.2 += c;
            }
            None => out.push((row[2].clone(), d, c)),
        }
    }
    out
}

pub fn sha256_of(path: &Path) -> String {
    pos_accounting::source::sha256_file(path).unwrap()
}
