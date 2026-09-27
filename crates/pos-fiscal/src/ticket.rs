use serde::{Deserialize, Serialize};

use crate::response::{FdmReference, SignResult, VatCalcItem};
use crate::types::EventLabel;

/// Language of the printed fiscal ticket.
///
/// §VI of the certification manual requires the official ticket
/// mentions in a national language (FR/NL/DE). The exact FR wording
/// comes from the POS detailed description v2.2; NL from the GKS
/// circulars ("DIT IS GEEN GELDIG BTW-KASTICKET"); DE is the official
/// -style translation, to be cross-checked against the DE spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReceiptLanguage {
    #[default]
    Fr,
    Nl,
    De,
}

impl ReceiptLanguage {
    /// Map a POS display-language code to a ticket language.
    /// Non-official languages (zh, en, ja…) fall back to FR - the
    /// fiscal ticket must be in a Belgian national language.
    pub fn from_code(code: &str) -> Self {
        match code {
            "nl" => Self::Nl,
            "de" => Self::De,
            _ => Self::Fr,
        }
    }

    /// Language code used for catalogue translations (`translations.lang`).
    pub fn code(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::Nl => "nl",
            Self::De => "de",
        }
    }
}

/// All data needed to print a fiscal ticket / VAT receipt.
/// Built from order data + FDM SignResult response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiscalTicket {
    // ── Restaurant identification ──
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub restaurant_vat: String,
    pub restaurant_est_no: String,
    /// Legal name of the issuer ("Denomination legale", AM of 29 April
    /// 2024 art. 20 b); the commercial name when they are the same.
    #[serde(default)]
    pub restaurant_legal_name: String,

    // ── Transaction data ──
    pub pos_id: String,
    /// POS software version (`posSwVersion`), printed with `pos_id`
    /// (manual §VI row V).
    #[serde(default)]
    pub pos_sw_version: String,
    pub terminal_id: String,
    /// Input device identification echoed by the FDM (`deviceId`,
    /// the terminal's MAC address - manual §VI row VII).
    #[serde(default)]
    pub device_id: String,
    /// User identification printed on the ticket (manual §VI row
    /// VIII): the operator's name, never the INSZ.
    #[serde(default)]
    pub operator: String,
    pub pos_datetime: String,
    pub ticket_number: i64,
    pub event_label: EventLabel,
    pub is_training: bool,

    // ── Line items ──
    pub lines: Vec<TicketLine>,

    // ── Totals ──
    pub subtotal: f64,
    pub total: f64,

    // ── VAT breakdown ──
    pub vat_breakdown: Vec<VatCalcItem>,

    // ── Payment ──
    pub payments: Vec<TicketPayment>,
    pub rounding: Option<f64>,

    // ── FDM control data (from SignResult) ──
    pub fdm_ref: Option<FdmReference>,
    pub short_signature: Option<String>,
    pub verification_url: Option<String>,
    pub footer: Vec<String>,

    // ── Ticket labels ──
    pub ticket_type_label: String,
    pub is_refund: bool,
    /// Language of the printed mentions (defaults to FR for tickets
    /// archived before this field existed).
    #[serde(default)]
    pub lang: ReceiptLanguage,
    /// Control data of the event this ticket refers to (the sale a
    /// refund reverses, the event a copy reproduces), printed under
    /// "Informations FDM d'origine".
    #[serde(default)]
    pub original: Option<OriginalControlData>,
    /// Copies only: when the copy was made (`posDateTime` of the C
    /// event), printed as "copie etablie le" (POS description v2.2
    /// §1.8).
    #[serde(default)]
    pub copy_made_at: Option<String>,
    /// A sale closed without payment (the customer left without
    /// paying, Advanced Set UC4109): its `financials` is empty and the
    /// ticket says so where the payments would be.
    #[serde(default)]
    pub unpaid: bool,
}

/// The control data of an earlier signed event (POS description v2.2
/// §6.6: fdmId, fdmDateTime, eventLabel, counters, shortSignature).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginalControlData {
    pub fdm_ref: FdmReference,
    pub short_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketLine {
    pub quantity: f64,
    pub description: String,
    pub unit_price: f64,
    pub total: f64,
    pub vat_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketPayment {
    pub method: String,
    pub amount: f64,
}

/// Parameters for building a FiscalTicket.
pub struct TicketParams<'a> {
    pub result: &'a SignResult,
    pub restaurant_name: &'a str,
    pub restaurant_address: &'a str,
    pub restaurant_vat: &'a str,
    pub restaurant_est_no: &'a str,
    /// Legal name of the issuer; empty when it is `restaurant_name`.
    pub restaurant_legal_name: &'a str,
    pub lines: Vec<TicketLine>,
    pub payments: Vec<TicketPayment>,
    pub total: f64,
    pub is_training: bool,
    /// Rounding amount in euros (e.g. 0.02 or -0.01). None for card.
    pub rounding: Option<f64>,
    /// Language of the printed mentions.
    pub lang: ReceiptLanguage,
    /// Name of the user who registered the event (row VIII).
    pub operator: &'a str,
}

impl FiscalTicket {
    /// Create a fiscal ticket from a SignResult and order data.
    pub fn from_sign_result(params: TicketParams<'_>) -> Self {
        let result = params.result;
        let lines = params.lines;
        let is_refund = !lines.is_empty() && lines.iter().all(|l| l.quantity < 0.0);

        let ticket_type_label =
            ticket_type_label(result.fdm_ref.event_label, params.lang).to_string();

        Self {
            restaurant_name: params.restaurant_name.to_string(),
            restaurant_address: params.restaurant_address.to_string(),
            restaurant_vat: params.restaurant_vat.to_string(),
            restaurant_est_no: params.restaurant_est_no.to_string(),
            restaurant_legal_name: if params.restaurant_legal_name.is_empty() {
                params.restaurant_name.to_string()
            } else {
                params.restaurant_legal_name.to_string()
            },
            pos_id: result.pos_id.clone(),
            pos_sw_version: crate::event::POS_SW_VERSION.to_string(),
            terminal_id: result.terminal_id.clone().unwrap_or_default(),
            device_id: result.device_id.clone(),
            operator: params.operator.to_string(),
            pos_datetime: result.pos_date_time.clone(),
            ticket_number: result.pos_fiscal_ticket_no,
            event_label: result.fdm_ref.event_label,
            is_training: params.is_training,
            lines,
            subtotal: params.total,
            total: match params.rounding {
                Some(r) => params.total + r,
                None => params.total,
            },
            vat_breakdown: result.vat_calc.clone().unwrap_or_default(),
            payments: params.payments,
            rounding: params.rounding,
            fdm_ref: Some(result.fdm_ref.clone()),
            short_signature: result.short_signature.clone(),
            verification_url: result.verification_url.clone(),
            footer: result.footer.clone(),
            ticket_type_label,
            is_refund,
            lang: params.lang,
            original: None,
            copy_made_at: None,
            unpaid: false,
        }
    }

    /// The copy (event C) of this ticket, `self` being the original
    /// ticket rebuilt from its archive. The copy keeps every field of
    /// the original, its QR code included (a copy gets no new URL,
    /// §6.5), and adds the COPIE mention and the copy event's control
    /// data; the original's move under "Informations FDM d'origine"
    /// (POS description v2.2 §1.8, SPF example copy ticket of
    /// 2026-09-24).
    pub fn into_copy(mut self, copy: &SignResult) -> Self {
        let original_ref = self.fdm_ref.take();
        self.original = original_ref.map(|fdm_ref| OriginalControlData {
            fdm_ref,
            short_signature: self.short_signature.take(),
        });
        self.fdm_ref = Some(copy.fdm_ref.clone());
        self.short_signature = copy.short_signature.clone();
        if let Some(url) = copy.verification_url.clone() {
            self.verification_url = Some(url);
        }
        if !copy.footer.is_empty() {
            self.footer = copy.footer.clone();
        }
        self.copy_made_at = Some(copy.pos_date_time.clone());
        self
    }

    /// This ticket with the control data of the event it refers to
    /// (the sale a refund reverses).
    pub fn with_original(mut self, original: OriginalControlData) -> Self {
        self.original = Some(original);
        self
    }

    /// Whether this is the copy of an earlier event.
    pub fn is_copy(&self) -> bool {
        self.copy_made_at.is_some()
    }

    /// Check whether this ticket needs the "not a valid VAT receipt"
    /// disclaimer mandated by the POS detailed description §1.3–1.8.
    pub fn needs_invalid_disclaimer(&self) -> bool {
        !matches!(self.event_label, EventLabel::N)
    }

    /// The spec-mandated bottom mention for non-N tickets, in the
    /// ticket's language.
    pub fn invalid_disclaimer(&self) -> &'static str {
        invalid_disclaimer(self.lang)
    }
}

/// The payment rows and cash rounding printed on the ticket of a
/// signed sale, rebuilt from the `financials` of its archived request
/// the way the checkout prints them: one row per payment line, and the
/// rounding total (its own line, FAQ Q403).
pub fn printed_payments(request: &serde_json::Value) -> (Vec<TicketPayment>, Option<f64>) {
    let cents = |v: &serde_json::Value| (v.as_f64().unwrap_or(0.0) * 100.0).round() as i64;
    let financials = request["financials"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let payments = financials
        .iter()
        .filter(|f| f["amountType"] == "PAYMENT")
        .map(|f| TicketPayment {
            method: printed_method(f),
            amount: cents(&f["amount"]) as f64 / 100.0,
        })
        .collect();
    let rounding: i64 = financials
        .iter()
        .filter(|f| f["amountType"] == "ROUNDING")
        .map(|f| cents(&f["amount"]))
        .sum();
    (payments, (rounding != 0).then(|| rounding as f64 / 100.0))
}

/// True for a sale closed without payment: an archived sale request
/// whose `financials` is empty while something was sold.
pub fn is_unpaid_sale(request: &serde_json::Value) -> bool {
    request["financials"]
        .as_array()
        .is_some_and(|f| f.is_empty())
        && request["transaction"]["transactionTotal"]
            .as_f64()
            .is_some_and(|t| t != 0.0)
}

/// The payment name a ticket prints for a `financials` line, from its
/// type: the printer translates "Cash", "Card" and "Meal voucher".
fn printed_method(line: &serde_json::Value) -> String {
    match line["type"].as_str().unwrap_or("") {
        "CASH" => "Cash".to_string(),
        "CHEQUE_MEAL" => "Meal voucher".to_string(),
        t if t.starts_with("CARD") => "Card".to_string(),
        _ => line["name"].as_str().unwrap_or("-").to_string(),
    }
}

/// Spec-mandated bottom mention for every printed non-N event
/// (POS detailed description v2.2, §1.3–1.8 and the ticket-content
/// table of the certification manual §VI). The French wording is the
/// law's: AM of 29 April 2024, art. 38.
pub fn invalid_disclaimer(lang: ReceiptLanguage) -> &'static str {
    match lang {
        ReceiptLanguage::Fr => "CECI N'EST PAS UN TICKET DE CAISSE T.V.A. VALABLE",
        ReceiptLanguage::Nl => "DIT IS GEEN GELDIG BTW-KASTICKET",
        ReceiptLanguage::De => "DIES IST KEIN GUELTIGER MWST.-KASSENZETTEL",
    }
}

/// Header label ("dénomination complète", cert manual §VI element I)
/// per event type and language. The French VAT ticket title is the
/// law's: « TICKET DE CAISSE T.V.A. » (AR of 30 December 2009 art. 2,
/// AM of 29 April 2024 art. 20 a). A refund is a VAT ticket: its
/// REMBOURSEMENT mention is printed under this title (SPF example
/// refund ticket of 2026-09-24).
pub fn ticket_type_label(label: EventLabel, lang: ReceiptLanguage) -> &'static str {
    use EventLabel::*;
    use ReceiptLanguage::*;
    match (label, lang) {
        (N, Fr) => "TICKET DE CAISSE T.V.A.",
        (N, Nl) => "BTW-KASTICKET",
        (N, De) => "MWST.-KASSENZETTEL",
        // "PRO FORMA" and "TRAINING" are the literal mentions used by
        // the spec in all three language versions.
        (P, _) => "PRO FORMA",
        (T, _) => "TRAINING",
        (C, Fr) => "COPIE",
        (C, Nl | De) => "KOPIE",
        (I, Fr) => "FACTURE",
        (I, Nl) => "FACTUUR",
        (I, De) => "RECHNUNG",
        (F, Fr) => "FINANCIER",
        (F, Nl) => "FINANCIEEL",
        (F, De) => "FINANZBEWEGUNG",
        (S, Fr) => "SOCIAL",
        (S, Nl) => "SOCIAAL",
        (S, De) => "SOZIAL",
        (R, Fr | Nl) => "RAPPORT",
        (R, De) => "BERICHT",
    }
}

/// Belgian 5-cent cash rounding. Returns the rounding delta in cents.
/// Rules: round to nearest 5 centimes.
/// - 0,5 → no change
/// - 1,2 → round down (negative delta)
/// - 3,4 → round up (positive delta)
/// - 6,7 → round down
/// - 8,9 → round up
pub fn round_to_5_cents(total_cents: i64) -> i64 {
    let remainder = total_cents % 5;
    if remainder == 0 {
        0
    } else if remainder <= 2 {
        -remainder
    } else {
        5 - remainder
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn printed_payments_keep_the_rounding_apart() {
        let request = json!({"financials": [
            {"id": "1", "type": "CASH", "amountType": "PAYMENT", "amount": 6.08},
            {"id": "1", "type": "CASH", "amountType": "ROUNDING", "amount": 0.02},
        ]});
        let (payments, rounding) = printed_payments(&request);
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0].method, "Cash");
        assert_eq!(payments[0].amount, 6.08);
        assert_eq!(rounding, Some(0.02));
    }

    #[test]
    fn printed_payments_of_a_card_sale_have_no_rounding() {
        let request = json!({"financials": [
            {"id": "2", "type": "CARD_DEBIT", "amountType": "PAYMENT", "amount": 12.5},
        ]});
        let (payments, rounding) = printed_payments(&request);
        assert_eq!(payments[0].method, "Card");
        assert_eq!(payments[0].amount, 12.5);
        assert_eq!(rounding, None);
    }

    #[test]
    fn an_unpaid_sale_has_no_payment_line_and_something_sold() {
        let sale = |financials: serde_json::Value, total: f64| {
            serde_json::json!({
                "transaction": { "transactionTotal": total },
                "financials": financials,
            })
        };
        assert!(is_unpaid_sale(&sale(serde_json::json!([]), 58.0)));
        let cash = serde_json::json!([{ "type": "CASH", "amountType": "PAYMENT", "amount": 58.0 }]);
        assert!(!is_unpaid_sale(&sale(cash, 58.0)));
        assert!(!is_unpaid_sale(&sale(serde_json::json!([]), 0.0)));
        assert!(!is_unpaid_sale(&serde_json::json!({ "reportType": "X" })));
    }

    #[test]
    fn printed_payments_of_a_split_sale_keep_each_method() {
        let request = json!({"financials": [
            {"id": "3", "name": "Meal voucher", "type": "CHEQUE_MEAL", "amountType": "PAYMENT", "amount": 3.13},
            {"id": "2", "name": "Card", "type": "CARD_DEBIT", "amountType": "PAYMENT", "amount": 3.97},
            {"id": "1", "name": "Cash", "type": "CASH", "amountType": "PAYMENT", "amount": 7.76},
            {"id": "1", "name": "Cash", "type": "CASH", "amountType": "ROUNDING", "amount": -0.01},
        ]});
        let (payments, rounding) = printed_payments(&request);
        let rows: Vec<_> = payments
            .iter()
            .map(|p| (p.method.as_str(), p.amount))
            .collect();
        assert_eq!(
            rows,
            [("Meal voucher", 3.13), ("Card", 3.97), ("Cash", 7.76)]
        );
        assert_eq!(rounding, Some(-0.01));
    }

    #[test]
    fn printed_payments_of_an_order_are_empty() {
        assert!(printed_payments(&json!({})).0.is_empty());
    }

    use super::*;
    use crate::response::{FdmReference, SignResult};
    use crate::types::{EventLabel, EventOperation};

    fn mock_sign_result(event_label: EventLabel) -> SignResult {
        SignResult {
            pos_id: "CFOD0061234567".to_string(),
            pos_fiscal_ticket_no: 1,
            pos_date_time: "2026-04-12T19:30:00+02:00".to_string(),
            terminal_id: Some("POS1".to_string()),
            device_id: "DEV001".to_string(),
            event_operation: EventOperation::Sale,
            fdm_ref: FdmReference {
                fdm_id: "FDM001".to_string(),
                fdm_date_time: "2026-04-12T19:30:01+02:00".to_string(),
                event_label,
                event_counter: 1,
                total_counter: 1,
            },
            fdm_sw_version: "1.0".to_string(),
            digital_signature: "AAAA".to_string(),
            short_signature: Some("abc123".to_string()),
            verification_url: Some(
                "https://www.gs2.be/12345678901234567890123456789012345678".to_string(),
            ),
            vat_calc: None,
            buffer_capacity_used: 0.01,
            warnings: None,
            informations: None,
            footer: vec!["www.minfin.fgov.be".to_string()],
        }
    }

    // ── Ticket construction ────────────────────────────────────

    #[test]
    fn normal_sale_label() {
        let result = mock_sign_result(EventLabel::N);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Tokyo Sushi Bar",
            restaurant_address: "Rue Test 1",
            restaurant_vat: "BE0499999960",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![TicketLine {
                quantity: 1.0,
                description: "Test".into(),
                unit_price: 10.0,
                total: 10.0,
                vat_code: "B".into(),
            }],
            payments: vec![TicketPayment {
                method: "Cash".into(),
                amount: 10.0,
            }],
            total: 10.0,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert_eq!(ticket.ticket_type_label, "TICKET DE CAISSE T.V.A.");
        assert!(!ticket.is_refund);
        assert!(!ticket.needs_invalid_disclaimer());
    }

    #[test]
    fn refund_detected_when_all_negative() {
        let result = mock_sign_result(EventLabel::N);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![TicketLine {
                quantity: -2.0,
                description: "Refund item".into(),
                unit_price: 5.0,
                total: -10.0,
                vat_code: "B".into(),
            }],
            payments: vec![],
            total: -10.0,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert!(ticket.is_refund);
        // A refund is a VAT ticket; REMBOURSEMENT is printed under it.
        assert_eq!(ticket.ticket_type_label, "TICKET DE CAISSE T.V.A.");
    }

    #[test]
    fn training_label() {
        let result = mock_sign_result(EventLabel::T);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![],
            payments: vec![],
            total: 0.0,
            is_training: true,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert_eq!(ticket.ticket_type_label, "TRAINING");
        assert!(ticket.needs_invalid_disclaimer());
    }

    #[test]
    fn pro_forma_needs_disclaimer() {
        let result = mock_sign_result(EventLabel::P);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![],
            payments: vec![],
            total: 0.0,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert_eq!(ticket.ticket_type_label, "PRO FORMA");
        assert!(ticket.needs_invalid_disclaimer());
    }

    #[test]
    fn copy_label_and_disclaimer() {
        let result = mock_sign_result(EventLabel::C);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![],
            payments: vec![],
            total: 0.0,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert_eq!(ticket.ticket_type_label, "COPIE");
        assert!(ticket.needs_invalid_disclaimer());
    }

    #[test]
    fn fdm_data_populated() {
        let result = mock_sign_result(EventLabel::N);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![],
            payments: vec![],
            total: 0.0,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert!(ticket.fdm_ref.is_some());
        assert_eq!(ticket.fdm_ref.unwrap().fdm_id, "FDM001");
        assert_eq!(ticket.short_signature, Some("abc123".to_string()));
        assert!(ticket.verification_url.is_some());
        assert_eq!(ticket.footer, vec!["www.minfin.fgov.be"]);
    }

    // ── Rounding ───────────────────────────────────────────────

    #[test]
    fn rounding_applied_to_total() {
        let result = mock_sign_result(EventLabel::N);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![TicketLine {
                quantity: 1.0,
                description: "Item".into(),
                unit_price: 12.53,
                total: 12.53,
                vat_code: "B".into(),
            }],
            payments: vec![TicketPayment {
                method: "Cash".into(),
                amount: 12.55,
            }],
            total: 12.53,
            is_training: false,
            rounding: Some(0.02),
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert_eq!(ticket.subtotal, 12.53);
        assert!((ticket.total - 12.55).abs() < 0.001);
        assert_eq!(ticket.rounding, Some(0.02));
    }

    #[test]
    fn no_rounding_for_card() {
        let result = mock_sign_result(EventLabel::N);
        let ticket = FiscalTicket::from_sign_result(TicketParams {
            result: &result,
            restaurant_name: "Test",
            restaurant_address: "",
            restaurant_vat: "",
            restaurant_est_no: "",
            restaurant_legal_name: "",
            lines: vec![],
            payments: vec![],
            total: 12.53,
            is_training: false,
            rounding: None,
            lang: ReceiptLanguage::default(),
            operator: "Test",
        });
        assert_eq!(ticket.subtotal, 12.53);
        assert_eq!(ticket.total, 12.53);
        assert!(ticket.rounding.is_none());
    }

    // ── Belgian 5-cent rounding function ───────────────────────

    #[test]
    fn round_exact_multiples_of_5() {
        assert_eq!(round_to_5_cents(1000), 0); // 10.00
        assert_eq!(round_to_5_cents(1005), 0); // 10.05
        assert_eq!(round_to_5_cents(1250), 0); // 12.50
        assert_eq!(round_to_5_cents(2500), 0); // 25.00
    }

    #[test]
    fn round_down_1_and_2() {
        assert_eq!(round_to_5_cents(1001), -1); // 10.01 → 10.00
        assert_eq!(round_to_5_cents(1002), -2); // 10.02 → 10.00
        assert_eq!(round_to_5_cents(1251), -1); // 12.51 → 12.50
        assert_eq!(round_to_5_cents(1252), -2); // 12.52 → 12.50
    }

    #[test]
    fn round_up_3_and_4() {
        assert_eq!(round_to_5_cents(1003), 2); // 10.03 → 10.05
        assert_eq!(round_to_5_cents(1004), 1); // 10.04 → 10.05
        assert_eq!(round_to_5_cents(1253), 2); // 12.53 → 12.55
        assert_eq!(round_to_5_cents(1254), 1); // 12.54 → 12.55
    }

    #[test]
    fn round_down_6_and_7() {
        assert_eq!(round_to_5_cents(1006), -1); // 10.06 → 10.05
        assert_eq!(round_to_5_cents(1007), -2); // 10.07 → 10.05
    }

    #[test]
    fn round_up_8_and_9() {
        assert_eq!(round_to_5_cents(1008), 2); // 10.08 → 10.10
        assert_eq!(round_to_5_cents(1009), 1); // 10.09 → 10.10
    }

    #[test]
    fn rounding_never_exceeds_2_cents() {
        // Belgian law: max rounding is ±2 centimes
        for cents in 0..10000 {
            let delta = round_to_5_cents(cents);
            assert!(delta.abs() <= 2, "cents={cents}, delta={delta}");
        }
    }
}
