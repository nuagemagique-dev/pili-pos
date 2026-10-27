//! Builder of the cleaned statistics SQLite (schema v1, decision X1D9).
//!
//! After a real Z, [`build_stats`] reads the plaintext snapshot of `pos.db`
//! and writes a NEW SQLite file with the sales of the booking period that Z
//! closed, the Z report itself and nothing else. The file is made for a
//! receiver and a dashboard: `snake_case` tables, integer cents, signed
//! amounts, closed value sets guarded by CHECK constraints, a SQL comment on
//! every column. Its contract is `crates/pos-export/STATS_SCHEMA.md`.
//!
//! What it holds: the sale events (label N) of the period and the turnover Z
//! that closed it, as the FDM signed them: amounts are read from the archived
//! event, never recomputed from the menu. What it never holds: training,
//! presence (S), copy (C), order and pro forma (P), money, drawer and invoice
//! events, and no INSZ or NISS, user name, PIN hash, token, e-mail, phone,
//! address, customer name, `HubRise` payload, ticket text, FDM signature or QR
//! data. Users are pseudonyms ([`crate::pseudonym`]).
//!
//! The source is only read. The result is written to `<out>.partial` and
//! renamed when complete, so a reader never sees half a file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pos_core::money::{round_to_i64, to_f64};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::pseudonym;
use crate::zref::ZRef;

/// Version of the statistics schema, `meta.schema_version`. A breaking change
/// bumps it; columns may be added within a version, never removed or renamed.
pub const STATS_SCHEMA_VERSION: i64 = 1;

/// Value of `meta.format`.
pub const STATS_FORMAT: &str = "pili-stats";

/// Identity of the installation and of the export, copied into `meta`.
#[derive(Debug, Clone)]
pub struct StatsMeta {
    pub export_id: String,
    pub pos_id: String,
    pub terminal_id: String,
    pub pos_sw_version: String,
    /// When the file is generated (RFC 3339 UTC). The only value that differs
    /// between two builds of the same input.
    pub generated_at: String,
}

/// What a statistics file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsSummary {
    /// Rows of `tickets`.
    pub tickets: i64,
    /// Rows of `ticket_lines`.
    pub lines: i64,
    /// Size of the finished file.
    pub bytes: i64,
}

/// Tables of the statistics file, in creation order.
pub const STATS_TABLES: [&str; 10] = [
    "meta",
    "users",
    "tickets",
    "ticket_lines",
    "ticket_vat",
    "ticket_payments",
    "ticket_discounts",
    "z_report",
    "z_vat",
    "z_payments",
];

const SCHEMA: &str = r"
CREATE TABLE meta (
    key TEXT PRIMARY KEY,   -- name of the value: format, schema_version, export_id, pos_id, terminal_id, pos_sw_version, z_report_no, booking_date, period_opened_at, period_closed_at, currency, generated_at
    value TEXT NOT NULL     -- the value, always text (numbers in decimal)
);

CREATE TABLE users (
    user_ref TEXT PRIMARY KEY,  -- pseudonym of a user: 'system' or 'u-' and 16 hex characters, stable for one installation
    role TEXT NOT NULL          -- role of the user in the POS
        CHECK (role IN ('admin', 'cashier', 'waiter', 'system'))
);

CREATE TABLE tickets (
    ticket_id INTEGER PRIMARY KEY,  -- local id of the signed sale event in the POS archive; the key of every ticket_* table
    event_label TEXT NOT NULL,      -- label under which the FDM signed the event, N for a sale
    ticket_no INTEGER NOT NULL,     -- fiscal ticket number of the POS (posFiscalTicketNo)
    kind TEXT NOT NULL              -- sale, or refund (a sale that refers to an earlier ticket, amounts negative)
        CHECK (kind IN ('sale', 'refund')),
    channel TEXT NOT NULL           -- how the customer ordered: table, takeaway, delivery (online order to be delivered), online (other online order)
        CHECK (channel IN ('table', 'takeaway', 'delivery', 'online')),
    table_number INTEGER,           -- table of a table sale, NULL for the other channels
    user_ref TEXT NOT NULL,         -- pseudonym of the user who signed the sale, see users
    fdm_datetime TEXT NOT NULL,     -- time of the FDM signature as the FDM gave it (RFC 3339, UTC)
    total_cents INTEGER NOT NULL,   -- value of the ticket in euro cents after price changes, negative for a refund
    unpaid INTEGER NOT NULL         -- 1 when the sale was closed without payment, else 0
        CHECK (unpaid IN (0, 1))
);

CREATE TABLE ticket_lines (
    ticket_id INTEGER NOT NULL,     -- ticket of the line, see tickets
    line_no INTEGER NOT NULL,       -- position of the line in the ticket, from 1
    product_code TEXT,              -- menu code of the product (menu_items.code), NULL when the menu has none
    product_name TEXT NOT NULL,     -- name of the product as signed
    department TEXT,                -- department (category) name as signed, NULL when empty
    quantity REAL NOT NULL,         -- quantity sold, negative on a refund or a correction
    unit_price_cents INTEGER NOT NULL,  -- unit price in euro cents, VAT included, before price changes
    amount_cents INTEGER NOT NULL,  -- value of the line in euro cents after price changes (quantity x unit price + discount_cents), signed
    discount_cents INTEGER NOT NULL,    -- sum of the price changes of the line in euro cents, negative for a discount
    vat_label TEXT NOT NULL         -- VAT label of the line (A 21 %, B 12 %, C 6 %, D 0 %, X out of scope)
        CHECK (vat_label IN ('A', 'B', 'C', 'D', 'X')),
    vat_rate_bp INTEGER,            -- VAT rate in basis points (2100 = 21 %) from the signed VAT calculation of the ticket, NULL when unknown
    PRIMARY KEY (ticket_id, line_no)
);

CREATE TABLE ticket_vat (
    ticket_id INTEGER NOT NULL,     -- ticket, see tickets
    vat_label TEXT NOT NULL         -- VAT label
        CHECK (vat_label IN ('A', 'B', 'C', 'D', 'X')),
    vat_rate_bp INTEGER NOT NULL,   -- VAT rate in basis points
    base_cents INTEGER NOT NULL,    -- taxable amount in euro cents, as calculated by the FDM
    vat_cents INTEGER NOT NULL,     -- VAT amount in euro cents, as calculated by the FDM
    total_cents INTEGER NOT NULL,   -- amount with VAT in euro cents (base_cents + vat_cents)
    PRIMARY KEY (ticket_id, vat_label)
);

CREATE TABLE ticket_payments (
    ticket_id INTEGER NOT NULL,     -- ticket, see tickets
    line_no INTEGER NOT NULL,       -- position of the payment line in the ticket, from 1
    method TEXT NOT NULL            -- payment method: cash, card, meal_voucher, app (ordering platform), online (webshop), other
        CHECK (method IN ('cash', 'card', 'meal_voucher', 'app', 'online', 'other')),
    kind TEXT NOT NULL              -- payment, rounding (cash rounding to 5 cents, own line), tip
        CHECK (kind IN ('payment', 'rounding', 'tip')),
    amount_cents INTEGER NOT NULL,  -- amount in euro cents, negative on a refund
    PRIMARY KEY (ticket_id, line_no)
);

CREATE TABLE ticket_discounts (
    ticket_id INTEGER NOT NULL,     -- ticket, see tickets
    line_no INTEGER,                -- line of the discount, NULL for a discount on the whole ticket
    kind TEXT NOT NULL              -- public (shown to the customer) or internal (recalculation)
        CHECK (kind IN ('public', 'internal')),
    amount_cents INTEGER NOT NULL   -- amount in euro cents, negative for a discount
);

CREATE TABLE z_report (
    z_report_no INTEGER PRIMARY KEY,    -- number of the Z report
    booking_date TEXT NOT NULL,         -- booking date of the closed period (YYYY-MM-DD)
    event_label TEXT NOT NULL,          -- label under which the FDM signed the Z, R
    ticket_no INTEGER NOT NULL,         -- fiscal ticket number of the Z
    user_ref TEXT NOT NULL,             -- pseudonym of the user who made the Z ('system' for the automatic Z)
    fdm_datetime TEXT NOT NULL,         -- time of the FDM signature of the Z (RFC 3339, UTC)
    sales_count INTEGER NOT NULL,       -- number of sale tickets (label N) of the period, refunds included
    refunds_count INTEGER NOT NULL,     -- number of those tickets that are refunds
    total_cents INTEGER NOT NULL        -- value of all sale tickets in euro cents, as signed in the Z
);

CREATE TABLE z_vat (
    z_report_no INTEGER NOT NULL,   -- Z report, see z_report
    vat_label TEXT NOT NULL         -- VAT label
        CHECK (vat_label IN ('A', 'B', 'C', 'D', 'X')),
    vat_rate_bp INTEGER NOT NULL,   -- VAT rate in basis points
    base_cents INTEGER NOT NULL,    -- taxable amount in euro cents, as signed in the Z
    vat_cents INTEGER NOT NULL,     -- VAT amount in euro cents, as signed in the Z
    total_cents INTEGER NOT NULL,   -- amount with VAT in euro cents, as signed in the Z
    PRIMARY KEY (z_report_no, vat_label)
);

CREATE TABLE z_payments (
    z_report_no INTEGER NOT NULL,   -- Z report, see z_report
    method TEXT NOT NULL            -- payment method, as in ticket_payments
        CHECK (method IN ('cash', 'card', 'meal_voucher', 'app', 'online', 'other')),
    kind TEXT NOT NULL              -- payment, rounding or tip, as in ticket_payments
        CHECK (kind IN ('payment', 'rounding', 'tip')),
    amount_cents INTEGER NOT NULL,  -- total in euro cents, as signed in the Z
    PRIMARY KEY (z_report_no, method, kind)
);
";

/// Build the statistics file of `z` from `source`, the plaintext snapshot of
/// `pos.db` (opened read-only by the caller), into `out`. `key` is the
/// installation's pseudonym key. Returns what the file holds.
pub fn build_stats(
    source: &Connection,
    z: &ZRef,
    meta: &StatsMeta,
    key: &[u8; 32],
    out: &Path,
) -> Result<StatsSummary, String> {
    let mut partial = out.as_os_str().to_os_string();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    clear(&partial)?;
    let built = write_file(source, z, meta, key, &partial);
    if built.is_err() {
        let _ = clear(&partial);
    }
    let (tickets, lines) = built?;
    std::fs::File::open(&partial)
        .and_then(|f| f.sync_all())
        .map_err(|e| format!("stats: cannot sync {}: {e}", partial.display()))?;
    std::fs::rename(&partial, out)
        .map_err(|e| format!("stats: cannot move the file to {}: {e}", out.display()))?;
    let bytes = std::fs::metadata(out)
        .map_err(|e| format!("stats: cannot stat {}: {e}", out.display()))?
        .len();
    Ok(StatsSummary {
        tickets,
        lines,
        bytes: i64::try_from(bytes).map_err(|_| "stats: size out of range".to_string())?,
    })
}

/// Remove a file and the journal beside it, if they exist.
fn clear(path: &Path) -> Result<(), String> {
    for suffix in ["", "-journal"] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        match std::fs::remove_file(&name) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "stats: cannot clear {}: {e}",
                    Path::new(&name).display()
                ));
            }
        }
    }
    Ok(())
}

fn sql<T>(r: rusqlite::Result<T>, what: &str) -> Result<T, String> {
    r.map_err(|e| format!("stats: {what}: {e}"))
}

// ── Reading the archive ────────────────────────────────────────────

/// Euro amount of the archived JSON (a multiple of 0.01) in cents. An error naming
/// the event when the value is not a number: a malformed archive row is never read
/// as zero.
fn cents(event_id: i64, v: &Value, what: &str) -> Result<i64, String> {
    pos_core::money::cents_of(v)
        .ok_or_else(|| format!("stats: archived event {event_id}: {what} is not a number"))
}

/// VAT rate of the archived JSON (percent) in basis points, same rule as [`cents`].
fn rate_bp(event_id: i64, v: &Value, what: &str) -> Result<i64, String> {
    cents(event_id, v, what)
}

/// An integer of the archived JSON, an error naming the event when it is not one.
fn integer(event_id: i64, v: &Value, what: &str) -> Result<i64, String> {
    v.as_i64()
        .ok_or_else(|| format!("stats: archived event {event_id}: {what} is not an integer"))
}

/// Parse the archived JSON of an event, keeping the serde error.
fn parse_json(event_id: i64, json: &str) -> Result<Value, String> {
    serde_json::from_str(json)
        .map_err(|e| format!("stats: archived event {event_id} is unreadable: {e}"))
}

fn text(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty())
}

/// Payment method of the statistics from the payment type of the event.
fn method_of(payment_type: &str) -> &'static str {
    match payment_type {
        "CASH" => "cash",
        "CHEQUE_MEAL" => "meal_voucher",
        "APP" => "app",
        "ONLINE" => "online",
        t if t.starts_with("CARD") => "card",
        _ => "other",
    }
}

/// `payment`, `rounding` or `tip` from the amount type of the event.
fn payment_kind(amount_type: &str) -> &'static str {
    match amount_type {
        "ROUNDING" => "rounding",
        "TIP" => "tip",
        _ => "payment",
    }
}

/// Rate in percent the FDM applies to a label (used only when a signed sale
/// carries no VAT calculation).
fn standard_rate(label: &str) -> f64 {
    match label {
        "A" => 21.0,
        "B" => 12.0,
        "C" => 6.0,
        _ => 0.0,
    }
}

/// A row of `orders`: type, source, `HubRise` order id, service type, table.
type OrderRow = (String, String, Option<String>, Option<String>, Option<i64>);

struct Line {
    code: Option<String>,
    name: String,
    department: Option<String>,
    quantity: f64,
    unit_price: i64,
    amount: i64,
    discount: i64,
    label: String,
}

struct Discount {
    line_no: Option<i64>,
    kind: &'static str,
    amount: i64,
}

struct Vat {
    label: String,
    rate_bp: i64,
    base: i64,
    tax: i64,
    total: i64,
}

struct Payment {
    method: &'static str,
    kind: &'static str,
    amount: i64,
}

struct Ticket {
    id: i64,
    event_label: String,
    fiscal_no: i64,
    refund: bool,
    channel: &'static str,
    table: Option<i64>,
    user_ref: String,
    fdm_datetime: String,
    total: i64,
    unpaid: bool,
    lines: Vec<Line>,
    discounts: Vec<Discount>,
    vats: Vec<Vat>,
    payments: Vec<Payment>,
}

/// Everything needed to read the source.
struct Reader<'a> {
    src: &'a Connection,
    key: &'a [u8; 32],
    has_code: bool,
    /// Users that appear in the file: `user_ref` to role.
    users: BTreeMap<String, String>,
}

impl Reader<'_> {
    /// Pseudonym of `insz`, and the user is remembered for the `users` table.
    fn user(&mut self, insz: &str) -> Result<String, String> {
        let user_ref = pseudonym::user_ref(self.key, insz);
        if !self.users.contains_key(&user_ref) {
            let role = if user_ref == pseudonym::SYSTEM_REF {
                "system".to_string()
            } else {
                let found: Option<String> = sql(
                    self.src
                        .query_row("SELECT role FROM users WHERE insz = ?1", [insz], |r| {
                            r.get(0)
                        })
                        .optional(),
                    "read the role of a user",
                )?;
                // An employee missing from the users table keeps a valid role.
                found
                    .filter(|r| ["admin", "cashier", "waiter"].contains(&r.as_str()))
                    .unwrap_or_else(|| "cashier".to_string())
            };
            self.users.insert(user_ref.clone(), role);
        }
        Ok(user_ref)
    }

    fn product_code(&self, product_id: &Value) -> Result<Option<String>, String> {
        if !self.has_code {
            return Ok(None);
        }
        let Some(id) = product_id.as_str().and_then(|s| s.parse::<i64>().ok()) else {
            return Ok(None);
        };
        let code: Option<Option<String>> = sql(
            self.src
                .query_row("SELECT code FROM menu_items WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .optional(),
            "read a product code",
        )?;
        Ok(code.flatten().filter(|c| !c.is_empty()))
    }

    /// The channel of a sale and the table of a table sale. A refund takes the
    /// channel of the ticket it refers to when that ticket is in the archive.
    fn channel(
        &self,
        order_id: Option<i64>,
        request: &Value,
        refers_to: Option<&Value>,
    ) -> Result<(&'static str, Option<i64>), String> {
        let origin = match refers_to {
            Some(fdm_ref) => self.original(fdm_ref)?,
            None => None,
        };
        let (order_id, cost_center) = match &origin {
            Some((order_id, request)) => (*order_id, request["costCenter"].clone()),
            None => (order_id, request["costCenter"].clone()),
        };
        let table_of_cost_center = || {
            (cost_center["type"].as_str() == Some("TABLE"))
                .then(|| {
                    cost_center["id"]
                        .as_str()
                        .and_then(|id| id.trim_start_matches('T').parse::<i64>().ok())
                })
                .flatten()
        };
        let order: Option<OrderRow> = match order_id {
            Some(id) => sql(
                self.src
                    .query_row(
                        "SELECT order_type, source, hubrise_order_id, service_type, table_number
                         FROM orders WHERE id = ?1",
                        [id],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                    )
                    .optional(),
                "read an order",
            )?,
            None => None,
        };
        Ok(match order {
            Some((order_type, source, hubrise_id, service_type, table_number)) => {
                if hubrise_id.is_some() || source != "pos_local" {
                    if service_type.as_deref() == Some("delivery") {
                        ("delivery", None)
                    } else {
                        ("online", None)
                    }
                } else if order_type == "dine_in" {
                    ("table", table_of_cost_center().or(table_number))
                } else {
                    ("takeaway", None)
                }
            }
            None => match table_of_cost_center() {
                Some(table) => ("table", Some(table)),
                None => ("takeaway", None),
            },
        })
    }

    /// The archived event an `fdmRef` names: its order and its request.
    fn original(&self, fdm_ref: &Value) -> Result<Option<(Option<i64>, Value)>, String> {
        let found: Option<(i64, Option<i64>, String)> = sql(
            self.src
                .query_row(
                    "SELECT id, order_id, event_data_json FROM fiscal_events
                     WHERE fdm_id = ?1 AND fdm_datetime = ?2 AND event_label = ?3
                       AND json_extract(fdm_response_json, '$.fdmRef.eventCounter') = ?4
                       AND json_extract(fdm_response_json, '$.fdmRef.totalCounter') = ?5
                     LIMIT 1",
                    params![
                        fdm_ref["fdmId"].as_str(),
                        fdm_ref["fdmDateTime"].as_str(),
                        fdm_ref["eventLabel"].as_str(),
                        fdm_ref["eventCounter"].as_i64(),
                        fdm_ref["totalCounter"].as_i64(),
                    ],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional(),
            "read the ticket a refund refers to",
        )?;
        found
            .map(|(id, order_id, json)| Ok((order_id, parse_json(id, &json)?)))
            .transpose()
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one pass over the lines, price changes, VAT and payments of a ticket; split helpers would share a dozen locals"
    )]
    fn ticket(&mut self, event: &pos_fiscal::archive::ArchivedEvent) -> Result<Ticket, String> {
        let id = event.id;
        let request = parse_json(id, &event.event_data_json)?;
        let response: Value = match event.fdm_response_json.as_deref() {
            Some(json) => parse_json(id, json)?,
            None => Value::Null,
        };
        let transaction = &request["transaction"];
        let total = cents(id, &transaction["transactionTotal"], "transactionTotal")?;
        let refers_to = request.get("fdmRef").filter(|v| v.is_object());
        let refund = refers_to.is_some() || total < 0;

        let mut lines = Vec::new();
        let mut discounts = Vec::new();
        // Ticket-wide price changes, one row per change: (grouping id, change id) to (kind, cents).
        let mut ticket_wide: BTreeMap<(i64, String), (&'static str, i64)> = BTreeMap::new();
        for (line_no, line) in (1_i64..).zip(
            transaction["transactionLines"]
                .as_array()
                .map_or(&[][..], Vec::as_slice),
        ) {
            let product = &line["mainProduct"];
            let vats = product["vats"].as_array().map_or(&[][..], Vec::as_slice);
            let mut price = 0;
            let mut discount = 0;
            for vat in vats {
                price += cents(id, &vat["price"], "a line price")?;
                for change in vat["priceChanges"].as_array().into_iter().flatten() {
                    let amount = cents(id, &change["amount"], "a price change amount")?;
                    discount += amount;
                    let kind = if change["type"].as_str() == Some("INTERNAL") {
                        "internal"
                    } else {
                        "public"
                    };
                    if change["scope"].as_str() == Some("EVENT") {
                        let key = (
                            change["groupingId"].as_i64().unwrap_or(0),
                            change["id"].as_str().unwrap_or_default().to_string(),
                        );
                        ticket_wide.entry(key).or_insert((kind, 0)).1 += amount;
                    } else {
                        discounts.push(Discount {
                            line_no: Some(line_no),
                            kind,
                            amount,
                        });
                    }
                }
            }
            lines.push(Line {
                code: self.product_code(&product["productId"])?,
                name: product["productName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                department: text(&product["departmentName"]).map(str::to_string),
                quantity: product["quantity"].as_f64().ok_or_else(|| {
                    format!("stats: archived event {id}: a line quantity is not a number")
                })?,
                unit_price: cents(id, &product["unitPrice"], "a unit price")?,
                amount: price + discount,
                discount,
                label: vats
                    .first()
                    .and_then(|v| v["label"].as_str())
                    .unwrap_or("X")
                    .to_string(),
            });
        }
        for (_, (kind, amount)) in ticket_wide {
            discounts.push(Discount {
                line_no: None,
                kind,
                amount,
            });
        }

        let vats = vat_rows(id, &response, &lines)?;
        let mut payments = Vec::new();
        for f in request["financials"].as_array().into_iter().flatten() {
            payments.push(Payment {
                method: method_of(f["type"].as_str().unwrap_or_default()),
                kind: payment_kind(f["amountType"].as_str().unwrap_or_default()),
                amount: cents(id, &f["amount"], "a payment amount")?,
            });
        }

        let (channel, table) = self.channel(event.order_id, &request, refers_to)?;
        let employee = request["employeeId"]
            .as_str()
            .unwrap_or(&event.user_insz)
            .to_string();
        Ok(Ticket {
            id: event.id,
            event_label: event.event_label.clone().unwrap_or_else(|| "N".into()),
            fiscal_no: integer(id, &request["posFiscalTicketNo"], "posFiscalTicketNo")?,
            refund,
            channel,
            table: if channel == "table" { table } else { None },
            user_ref: self.user(&employee)?,
            fdm_datetime: event
                .fdm_datetime
                .clone()
                .or_else(|| text(&response["fdmRef"]["fdmDateTime"]).map(str::to_string))
                .or_else(|| text(&request["posDateTime"]).map(str::to_string))
                .unwrap_or_default(),
            total,
            unpaid: pos_fiscal::ticket::is_unpaid_sale(&request),
            lines,
            discounts,
            vats,
            payments,
        })
    }
}

/// The VAT of a ticket: the FDM's calculation when the response has one, else
/// the sum of the lines per label at the standard rate.
fn vat_rows(event_id: i64, response: &Value, lines: &[Line]) -> Result<Vec<Vat>, String> {
    let mut rows: BTreeMap<String, Vat> = BTreeMap::new();
    if let Some(calc) = response["vatCalc"].as_array() {
        for item in calc {
            let label = item["label"].as_str().unwrap_or("X").to_string();
            let row = rows.entry(label.clone()).or_insert(Vat {
                label,
                rate_bp: rate_bp(event_id, &item["rate"], "a vatCalc rate")?,
                base: 0,
                tax: 0,
                total: 0,
            });
            row.base += cents(event_id, &item["taxableAmount"], "a vatCalc taxableAmount")?;
            row.tax += cents(event_id, &item["vatAmount"], "a vatCalc vatAmount")?;
            row.total += cents(event_id, &item["totalAmount"], "a vatCalc totalAmount")?;
        }
    } else {
        for line in lines {
            let rate = standard_rate(&line.label);
            let row = rows.entry(line.label.clone()).or_insert(Vat {
                label: line.label.clone(),
                rate_bp: round_to_i64(rate * 100.0),
                base: 0,
                tax: 0,
                total: 0,
            });
            row.total += line.amount;
        }
        for row in rows.values_mut() {
            let rate = to_f64(row.rate_bp) / 100.0;
            row.base = round_to_i64(to_f64(row.total) / (1.0 + rate / 100.0));
            row.tax = row.total - row.base;
        }
    }
    Ok(rows.into_values().collect())
}

// ── Writing the file ───────────────────────────────────────────────

#[expect(
    clippy::too_many_lines,
    reason = "writes every table of the statistics file in order inside one transaction; a split would only pass the transaction around"
)]
fn write_file(
    source: &Connection,
    z: &ZRef,
    meta: &StatsMeta,
    key: &[u8; 32],
    partial: &Path,
) -> Result<(i64, i64), String> {
    // The closed period and the Z event.
    let (opened_at, closed_at): (String, Option<String>) = sql(
        source
            .query_row(
                "SELECT opened_at, closed_at FROM booking_periods WHERE id = ?1",
                [&z.period_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional(),
        "read the booking period",
    )?
    .ok_or_else(|| format!("stats: booking period {} not found", z.period_id))?;
    let closed_at = closed_at.unwrap_or_else(|| z.closed_at.clone());
    let z_event = pos_fiscal::archive::events_in_booking_period(source, &z.period_id)
        .map_err(|e| format!("stats: read the period events: {e}"))?;
    let mut z_row: Option<(Value, Option<String>, Option<String>, String)> = None;
    let mut sales = Vec::new();
    for event in z_event {
        if event.id == z.closing_event_id {
            if event.event_type != pos_fiscal::types::EventLabel::R {
                return Err(format!("stats: event {} is not a Z report", event.id));
            }
            let request = parse_json(event.id, &event.event_data_json)?;
            z_row = Some((
                request,
                event.event_label.clone(),
                event.fdm_datetime.clone(),
                event.user_insz.clone(),
            ));
        } else if event.event_type == pos_fiscal::types::EventLabel::N
            && event.event_label.as_deref().is_none_or(|l| l == "N")
        {
            sales.push(event);
        }
    }
    let (z_request, z_label, z_fdm_datetime, z_user) = if let Some(row) = z_row {
        row
    } else {
        // The Z event is not tagged with the period (should not happen): read it by id.
        let (json, label, fdm, user): (String, Option<String>, Option<String>, String) = sql(
            source.query_row(
                "SELECT event_data_json, event_label, fdm_datetime, user_insz
                 FROM fiscal_events WHERE id = ?1 AND event_type = 'R'",
                [z.closing_event_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ),
            "read the Z event",
        )?;
        let request = parse_json(z.closing_event_id, &json)?;
        (request, label, fdm, user)
    };

    let has_code = {
        let mut stmt = sql(
            source.prepare("SELECT name FROM pragma_table_info('menu_items')"),
            "read the menu columns",
        )?;
        let names = sql(
            stmt.query_map([], |r| r.get::<_, String>(0)),
            "read the menu columns",
        )?;
        let mut found = false;
        for name in names {
            found |= sql(name, "read the menu columns")? == "code";
        }
        found
    };
    let mut reader = Reader {
        src: source,
        key,
        has_code,
        users: BTreeMap::new(),
    };
    let mut tickets = Vec::with_capacity(sales.len());
    for event in &sales {
        tickets.push(reader.ticket(event)?);
    }
    let z_employee = z_request["employeeId"]
        .as_str()
        .unwrap_or(&z_user)
        .to_string();
    let z_user_ref = reader.user(&z_employee)?;

    let mut conn = sql(Connection::open(partial), "create the file")?;
    sql(
        conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;"),
        "set the pragmas",
    )?;
    sql(conn.execute_batch(SCHEMA), "create the tables")?;
    let tx = sql(conn.transaction(), "begin")?;

    let z_no = z.z_report_no.to_string();
    let meta_rows: [(&str, &str); 12] = [
        ("format", STATS_FORMAT),
        ("schema_version", &STATS_SCHEMA_VERSION.to_string()),
        ("export_id", &meta.export_id),
        ("pos_id", &meta.pos_id),
        ("terminal_id", &meta.terminal_id),
        ("pos_sw_version", &meta.pos_sw_version),
        ("z_report_no", &z_no),
        ("booking_date", &z.booking_date),
        ("period_opened_at", &opened_at),
        ("period_closed_at", &closed_at),
        ("currency", "EUR"),
        ("generated_at", &meta.generated_at),
    ];
    for (k, v) in meta_rows {
        sql(
            tx.execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2)",
                params![k, v],
            ),
            "write meta",
        )?;
    }

    let mut line_count = 0;
    for t in &tickets {
        sql(
            tx.execute(
                "INSERT INTO tickets (ticket_id, event_label, ticket_no, kind, channel, table_number,
                                      user_ref, fdm_datetime, total_cents, unpaid)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    t.id,
                    t.event_label,
                    t.fiscal_no,
                    if t.refund { "refund" } else { "sale" },
                    t.channel,
                    t.table,
                    t.user_ref,
                    t.fdm_datetime,
                    t.total,
                    t.unpaid,
                ],
            ),
            "write a ticket",
        )?;
        // The rate of a line comes from the ticket's VAT calculation.
        for (line_no, l) in (1_i64..).zip(&t.lines) {
            let rate = t
                .vats
                .iter()
                .find(|v| v.label == l.label)
                .map(|v| v.rate_bp);
            sql(
                tx.execute(
                    "INSERT INTO ticket_lines (ticket_id, line_no, product_code, product_name, department,
                                               quantity, unit_price_cents, amount_cents, discount_cents,
                                               vat_label, vat_rate_bp)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        t.id,
                        line_no,
                        l.code,
                        l.name,
                        l.department,
                        l.quantity,
                        l.unit_price,
                        l.amount,
                        l.discount,
                        l.label,
                        rate,
                    ],
                ),
                "write a ticket line",
            )?;
            line_count += 1;
        }
        for v in &t.vats {
            sql(
                tx.execute(
                    "INSERT INTO ticket_vat (ticket_id, vat_label, vat_rate_bp, base_cents, vat_cents, total_cents)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![t.id, v.label, v.rate_bp, v.base, v.tax, v.total],
                ),
                "write the VAT of a ticket",
            )?;
        }
        for (line_no, p) in (1_i64..).zip(&t.payments) {
            sql(
                tx.execute(
                    "INSERT INTO ticket_payments (ticket_id, line_no, method, kind, amount_cents)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![t.id, line_no, p.method, p.kind, p.amount],
                ),
                "write a payment",
            )?;
        }
        for d in &t.discounts {
            sql(
                tx.execute(
                    "INSERT INTO ticket_discounts (ticket_id, line_no, kind, amount_cents)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![t.id, d.line_no, d.kind, d.amount],
                ),
                "write a discount",
            )?;
        }
    }

    // The Z report, as signed.
    let turnover = &z_request["turnover"];
    let sales_total = turnover["transactions"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|t| t["eventLabel"].as_str() == Some("N"));
    let zid = z.closing_event_id;
    let mut refunds_count = 0;
    for n in turnover["negQuantities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|n| n["negQuantityReason"].as_str() == Some("REFUND"))
    {
        refunds_count += integer(zid, &n["ticketCount"], "a refund ticketCount")?;
    }
    let (sales_count, sales_cents) = match sales_total {
        Some(t) => (
            integer(zid, &t["ticketCount"], "the N ticketCount")?,
            cents(zid, &t["amount"], "the N amount")?,
        ),
        None => (0, 0),
    };
    let z_ticket_no = integer(zid, &z_request["posFiscalTicketNo"], "posFiscalTicketNo")?;
    sql(
        tx.execute(
            "INSERT INTO z_report (z_report_no, booking_date, event_label, ticket_no, user_ref,
                                   fdm_datetime, sales_count, refunds_count, total_cents)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                z.z_report_no,
                z.booking_date,
                z_label.unwrap_or_else(|| "R".into()),
                z_ticket_no,
                z_user_ref,
                z_fdm_datetime
                    .or_else(|| text(&z_request["posDateTime"]).map(str::to_string))
                    .unwrap_or_default(),
                sales_count,
                refunds_count,
                sales_cents,
            ],
        ),
        "write the Z report",
    )?;
    for v in turnover["vats"].as_array().into_iter().flatten() {
        let (rate, base, vat, total) = (
            rate_bp(zid, &v["rate"], "a Z VAT rate")?,
            cents(zid, &v["taxableAmount"], "a Z VAT taxableAmount")?,
            cents(zid, &v["vatAmount"], "a Z VAT vatAmount")?,
            cents(zid, &v["totalAmount"], "a Z VAT totalAmount")?,
        );
        sql(
            tx.execute(
                "INSERT INTO z_vat (z_report_no, vat_label, vat_rate_bp, base_cents, vat_cents, total_cents)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    z.z_report_no,
                    v["label"].as_str().unwrap_or("X"),
                    rate,
                    base,
                    vat,
                    total,
                ],
            ),
            "write the VAT of the Z",
        )?;
    }
    let mut z_payments: BTreeMap<(&'static str, &'static str), i64> = BTreeMap::new();
    for p in turnover["payments"].as_array().into_iter().flatten() {
        let method = method_of(p["type"].as_str().unwrap_or_default());
        for amount in p["amount"].as_array().into_iter().flatten() {
            let kind = payment_kind(amount["type"].as_str().unwrap_or_default());
            *z_payments.entry((method, kind)).or_insert(0) +=
                cents(zid, &amount["totalAmount"], "a Z payment totalAmount")?;
        }
    }
    for ((method, kind), amount) in z_payments {
        sql(
            tx.execute(
                "INSERT INTO z_payments (z_report_no, method, kind, amount_cents) VALUES (?1, ?2, ?3, ?4)",
                params![z.z_report_no, method, kind, amount],
            ),
            "write the payments of the Z",
        )?;
    }

    for (user_ref, role) in &reader.users {
        sql(
            tx.execute(
                "INSERT INTO users (user_ref, role) VALUES (?1, ?2)",
                params![user_ref, role],
            ),
            "write a user",
        )?;
    }
    sql(
        tx.execute_batch(&format!("PRAGMA user_version = {STATS_SCHEMA_VERSION};")),
        "set the version",
    )?;
    sql(tx.commit(), "commit")?;
    conn.close()
        .map_err(|(_, e)| format!("stats: close: {e}"))?;
    Ok((i64::try_from(tickets.len()).unwrap_or(i64::MAX), line_count))
}
