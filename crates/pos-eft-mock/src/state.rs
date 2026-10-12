//! What the mock remembers: the payments it was asked for, the requests
//! it received, and what to do with the next payment.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What the terminal does with a payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Behaviour {
    /// Approved after the delay.
    #[default]
    Approve,
    /// Refused after the delay.
    Decline,
    /// Approved after the delay, but the answer never reaches the till:
    /// HTTP 202 (Worldline), HTTP 504 with the transaction (CCV), a cut
    /// connection (OPI). The status calls then say approved.
    Timeout,
    /// The terminal is busy: nothing is charged.
    Busy,
    /// The terminal never reaches a final answer (still in progress for
    /// Worldline, `manualintervention` for CCV, busy for OPI).
    Unknown,
}

fn default_brand() -> String {
    "Bancontact".to_string()
}

fn default_delay() -> u64 {
    1500
}

/// The behaviour of one payment, as `POST /mock/next` sets it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Next {
    #[serde(default)]
    pub result: Behaviour,
    #[serde(default = "default_brand")]
    pub brand: String,
    /// How long the cardholder takes, in milliseconds.
    #[serde(default = "default_delay")]
    pub delay_ms: u64,
    /// The amount the terminal reports as approved, when it is not the
    /// requested one (cents). For the "amount mismatch" tests.
    #[serde(default)]
    pub approved_cents: Option<i64>,
}

impl Default for Next {
    fn default() -> Self {
        Self {
            result: Behaviour::Approve,
            brand: default_brand(),
            delay_ms: default_delay(),
            approved_cents: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Worldline,
    Ccv,
    Opi,
}

/// Where a payment stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    InProgress,
    Approved,
    Declined,
    Cancelled,
    /// Past its delay, with no final answer ever.
    Stuck,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Self::InProgress => "in_progress",
            Self::Approved => "approved",
            Self::Declined => "declined",
            Self::Cancelled => "cancelled",
            Self::Stuck => "unknown",
        }
    }
}

/// One payment.
#[derive(Debug, Clone)]
pub struct Payment {
    pub id: u64,
    pub protocol: Protocol,
    /// The till's reference: the Worldline `ExchangeIdentification`, the
    /// CCV `merchantOrderReference`, the OPI `RequestID`.
    pub reference: String,
    /// The terminal it runs on: two payments never run on one.
    pub terminal: String,
    /// CCV's own reference (`TL...`), empty for the others.
    pub psp_reference: String,
    pub amount_cents: i64,
    pub approved_cents: i64,
    pub brand: String,
    pub behaviour: Behaviour,
    /// OPI: the `ApplicationSender` and `WorkstationID` to echo. CCV: the
    /// management system id and the terminal id.
    pub sender: String,
    pub workstation: String,
    pub created_at: String,
    pub done_at: Instant,
    pub aborted_at: Option<Instant>,
}

impl Payment {
    pub fn phase(&self, now: Instant) -> Phase {
        if let Some(at) = self.aborted_at
            && at < self.done_at
        {
            return Phase::Cancelled;
        }
        if now < self.done_at {
            return Phase::InProgress;
        }
        match self.behaviour {
            Behaviour::Approve | Behaviour::Timeout => Phase::Approved,
            Behaviour::Decline => Phase::Declined,
            Behaviour::Unknown | Behaviour::Busy => Phase::Stuck,
        }
    }
}

/// A payment as the control API shows it.
#[derive(Debug, Clone, Serialize)]
pub struct PaymentView {
    pub id: u64,
    pub protocol: Protocol,
    pub reference: String,
    pub amount_cents: i64,
    pub brand: String,
    pub outcome: &'static str,
    pub at: String,
}

/// One request the mock received.
#[derive(Debug, Clone, Serialize)]
pub struct RequestLog {
    pub seq: u64,
    pub at: String,
    pub protocol: Protocol,
    pub method: String,
    pub path: String,
    /// The JSON body, or the XML text for OPI.
    pub body: Value,
    /// The HTTP status answered (0 for OPI and while pending).
    pub status: u16,
}

pub struct Inner {
    pub next: VecDeque<Next>,
    pub default: Next,
    pub payments: Vec<Payment>,
    pub requests: Vec<RequestLog>,
    pub api_key: Option<String>,
    pub device_port: u16,
    /// The OPI payment `RepeatLastMessage` repeats.
    pub opi_last: Option<String>,
    seq: u64,
}

pub type Shared = Arc<Mutex<Inner>>;

pub fn new_shared(api_key: Option<String>, device_port: u16) -> Shared {
    Arc::new(Mutex::new(Inner {
        next: VecDeque::new(),
        default: Next::default(),
        payments: Vec::new(),
        requests: Vec::new(),
        api_key,
        device_port,
        opi_last: None,
        seq: 0,
    }))
}

pub fn lock(shared: &Shared) -> MutexGuard<'_, Inner> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn clock() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

/// What starting a payment came to.
pub enum Begin {
    /// The reference was seen before: no second charge.
    Existing(usize),
    Created(usize),
    /// The terminal is busy (scripted, or another payment is running).
    Busy,
}

impl Inner {
    pub fn log(&mut self, protocol: Protocol, method: &str, path: &str, body: Value) -> usize {
        self.seq += 1;
        self.requests.push(RequestLog {
            seq: self.seq,
            at: clock(),
            protocol,
            method: method.to_string(),
            path: path.to_string(),
            body,
            status: 0,
        });
        self.requests.len() - 1
    }

    pub fn set_status(&mut self, index: usize, status: u16) {
        if let Some(r) = self.requests.get_mut(index) {
            r.status = status;
        }
    }

    pub fn find(&self, protocol: Protocol, reference: &str) -> Option<usize> {
        self.payments
            .iter()
            .position(|p| p.protocol == protocol && p.reference == reference)
    }

    pub fn find_psp(&self, psp: &str) -> Option<usize> {
        self.payments
            .iter()
            .position(|p| p.protocol == Protocol::Ccv && p.psp_reference == psp)
    }

    fn take_next(&mut self) -> Next {
        self.next
            .pop_front()
            .unwrap_or_else(|| self.default.clone())
    }

    /// Start the payment `reference` on `terminal`, or find it again.
    pub fn begin(
        &mut self,
        protocol: Protocol,
        reference: &str,
        terminal: &str,
        amount_cents: i64,
        sender: &str,
        workstation: &str,
    ) -> Begin {
        if let Some(i) = self.find(protocol, reference) {
            return Begin::Existing(i);
        }
        let next = self.take_next();
        let now = Instant::now();
        let running = self.payments.iter().any(|p| {
            p.terminal == terminal && matches!(p.phase(now), Phase::InProgress | Phase::Stuck)
        });
        if next.result == Behaviour::Busy || running {
            return Begin::Busy;
        }
        let id = self.payments.len() as u64 + 1;
        let psp_reference = if protocol == Protocol::Ccv {
            format!(
                "TL{:015}{:08X}.1",
                261_012_000_000_000 + id,
                0xCB87_E19B_u32
            )
        } else {
            String::new()
        };
        self.payments.push(Payment {
            id,
            protocol,
            reference: reference.to_string(),
            terminal: terminal.to_string(),
            psp_reference,
            amount_cents,
            approved_cents: next.approved_cents.unwrap_or(amount_cents),
            brand: next.brand,
            behaviour: next.result,
            sender: sender.to_string(),
            workstation: workstation.to_string(),
            created_at: clock(),
            done_at: now + Duration::from_millis(next.delay_ms),
            aborted_at: None,
        });
        Begin::Created(self.payments.len() - 1)
    }

    /// Stop a payment that is still waiting for the cardholder. A
    /// terminal that never answers ignores the stop.
    pub fn abort(&mut self, index: usize) -> bool {
        let now = Instant::now();
        let Some(p) = self.payments.get_mut(index) else {
            return false;
        };
        if p.behaviour != Behaviour::Unknown
            && p.aborted_at.is_none()
            && p.phase(now) == Phase::InProgress
        {
            p.aborted_at = Some(now);
            return true;
        }
        false
    }

    pub fn views(&self) -> Vec<PaymentView> {
        let now = Instant::now();
        self.payments
            .iter()
            .map(|p| PaymentView {
                id: p.id,
                protocol: p.protocol,
                reference: p.reference.clone(),
                amount_cents: p.amount_cents,
                brand: p.brand.clone(),
                outcome: p.phase(now).name(),
                at: p.created_at.clone(),
            })
            .collect()
    }
}

/// Wait until payment `index` is past its delay or stopped; the payment
/// as it stands then.
pub async fn wait_done(shared: &Shared, index: usize) -> (Payment, Phase) {
    loop {
        {
            let inner = lock(shared);
            let p = &inner.payments[index];
            let phase = p.phase(Instant::now());
            if phase != Phase::InProgress {
                return (p.clone(), phase);
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// `cents` as "12.50".
pub fn amount_text(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// The brand as a terminal prints it.
pub fn brand_upper(brand: &str) -> String {
    brand.to_ascii_uppercase()
}

/// Debit or credit, as a Belgian terminal receipt says.
pub fn account_type(brand: &str) -> &'static str {
    match brand.to_ascii_lowercase().as_str() {
        "visa" | "mastercard" | "american express" | "amex" => "Credit",
        _ => "Debit",
    }
}

/// The OPI `CardCircuit` for a brand (Bancontact is `BCMC`: the real
/// label of a Belgian terminal is not public).
pub fn circuit(brand: &str) -> String {
    match brand.to_ascii_lowercase().as_str() {
        "bancontact" => "BCMC".to_string(),
        "mastercard" => "MASTERCARD".to_string(),
        other => other.to_ascii_uppercase(),
    }
}

/// The cardholder receipt of an approved payment.
pub fn customer_receipt(p: &Payment) -> Vec<String> {
    vec![
        "--------------------------------".into(),
        "         PILI EFT MOCK".into(),
        "--------------------------------".into(),
        "      ***Cardholder Receipt***".into(),
        String::new(),
        "Purchase".into(),
        brand_upper(&p.brand),
        "XXXXXXXXXXXX0119".into(),
        format!("Account Type:             {}", account_type(&p.brand)),
        String::new(),
        p.created_at.clone(),
        "Trm-Id:                 00000001".into(),
        format!("Auth. Code:               {:06}", auth_code(p)),
        String::new(),
        format!(
            "Total-EFT EUR:             {}",
            amount_text(p.approved_cents)
        ),
        String::new(),
        "Transaction OK".into(),
    ]
}

pub fn auth_code(p: &Payment) -> u64 {
    57_000 + p.id
}
