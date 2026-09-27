//! Consistency check of every FDM sign response before it is archived.
//!
//! The FDM signs what the POS sends and echoes the request's identity
//! back. A response that does not match the request it answers (another
//! POS, another ticket, another event type, a short signature that is
//! not derived from the digital signature) must never become a fiscal
//! record: the sale is refused and nothing is printed.
//!
//! This is a consistency check, not a cryptographic one. The ECDSA
//! signature itself is kept untouched in the archive (full response
//! JSON) for verification by the administration.
//!
//! One exception: the module2 online simulator, the FDM of the
//! certification test machine, signs with a placeholder (see
//! [`SIMULATOR_SIGNATURE_PREFIX`]). Its signature is not base64 and its
//! short signature is not derived from it, so those two checks are
//! left out for it; every other check still applies.

use base64::Engine;
use chrono::DateTime;
use serde_json::Value;

use crate::response::SignResult;
use crate::signature::compute_short_signature;
use crate::types::{EventLabel, EventOperation};

/// The simulator's placeholder signature: `dummy_` followed by base64
/// (seen on module2's simulator FDM02000425, FDM software 2.2.0.5). A
/// real FDM returns base64 only, which never contains `_`.
pub const SIMULATOR_SIGNATURE_PREFIX: &str = "dummy_";

/// A simulator placeholder rather than a real signature.
fn is_simulator_placeholder(signature: &str) -> bool {
    signature
        .strip_prefix(SIMULATOR_SIGNATURE_PREFIX)
        .is_some_and(|rest| !rest.is_empty())
}

/// A short signature as printed: 40 hexadecimal characters (SHA-1).
fn is_sha1_hex(short: &str) -> bool {
    short.len() == 40 && short.bytes().all(|b| b.is_ascii_hexdigit())
}

/// What the POS asked the FDM to sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedEvent {
    pub operation: EventOperation,
    pub label: EventLabel,
}

impl ExpectedEvent {
    /// Expected operation and label for a sign mutation, per the
    /// `tb_eventOperation` table (POS detailed description v2.2
    /// §2.2.4). In training mode every operation is labelled T.
    /// `None` for a mutation name this POS does not know.
    pub fn for_mutation(mutation: &str, is_training: bool) -> Option<Self> {
        use EventLabel as L;
        use EventOperation as O;
        let (operation, label) = match mutation {
            "signWorkIn" => (O::WorkIn, L::S),
            "signWorkOut" => (O::WorkOut, L::S),
            "signSale" => (O::Sale, L::N),
            "signInvoice" => (O::Invoice, L::I),
            "signCostCenterChange" => (O::CostCenterChange, L::P),
            "signOrder" => (O::Order, L::P),
            "signPreBill" => (O::PreBill, L::P),
            "signMoneyInOut" => (O::MoneyInOut, L::F),
            "signDrawerOpen" => (O::DrawerOpen, L::F),
            "signPaymentCorrection" => (O::PaymentCorrection, L::F),
            "signReportTurnoverX" => (O::ReportTurnoverX, L::R),
            "signReportTurnoverZ" => (O::ReportTurnoverZ, L::R),
            "signReportUserX" => (O::ReportUserX, L::R),
            "signReportUserZ" => (O::ReportUserZ, L::R),
            "signCopy" => (O::Copy, L::C),
            _ => return None,
        };
        Some(Self {
            operation,
            label: label.in_mode(is_training),
        })
    }
}

/// First field of the response that does not match the request.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{field} attendu {expected}, reçu {received}")]
pub struct Mismatch {
    pub field: &'static str,
    pub expected: String,
    pub received: String,
}

impl Mismatch {
    fn new(field: &'static str, expected: impl ToString, received: impl ToString) -> Self {
        Self {
            field,
            expected: expected.to_string(),
            received: received.to_string(),
        }
    }
}

fn request_str<'a>(request: &'a Value, key: &str) -> Option<&'a str> {
    request.get(key).and_then(Value::as_str)
}

fn same_text(expected: &str, received: &str) -> bool {
    expected.trim().eq_ignore_ascii_case(received.trim())
}

/// Two timestamps are the same instant, whatever offset notation the
/// FDM chose ("Z" or "+00:00"). Unparseable values fall back to text.
fn same_instant(expected: &str, received: &str) -> bool {
    match (
        DateTime::parse_from_rfc3339(expected),
        DateTime::parse_from_rfc3339(received),
    ) {
        (Ok(a), Ok(b)) => a == b,
        _ => expected == received,
    }
}

/// Check a sign response against the request it answers.
///
/// `request` is the serialized `data` input (the common fields are
/// flattened at its top level).
pub fn check_sign_result(
    request: &Value,
    expected: ExpectedEvent,
    res: &SignResult,
) -> Result<(), Mismatch> {
    if let Some(pos_id) = request_str(request, "posId")
        && !same_text(pos_id, &res.pos_id)
    {
        return Err(Mismatch::new("posId", pos_id, &res.pos_id));
    }
    if let Some(ticket_no) = request.get("posFiscalTicketNo").and_then(Value::as_i64)
        && ticket_no != res.pos_fiscal_ticket_no
    {
        return Err(Mismatch::new(
            "posFiscalTicketNo",
            ticket_no,
            res.pos_fiscal_ticket_no,
        ));
    }
    if let Some(date_time) = request_str(request, "posDateTime")
        && !same_instant(date_time, &res.pos_date_time)
    {
        return Err(Mismatch::new("posDateTime", date_time, &res.pos_date_time));
    }
    if let Some(device_id) = request_str(request, "deviceId")
        && !same_text(device_id, &res.device_id)
    {
        return Err(Mismatch::new("deviceId", device_id, &res.device_id));
    }
    if let Some(terminal_id) = request_str(request, "terminalId") {
        let received = res.terminal_id.as_deref().unwrap_or("");
        if !same_text(terminal_id, received) {
            return Err(Mismatch::new("terminalId", terminal_id, received));
        }
    }

    if res.event_operation != expected.operation {
        return Err(Mismatch::new(
            "eventOperation",
            format!("{:?}", expected.operation),
            format!("{:?}", res.event_operation),
        ));
    }
    if res.fdm_ref.event_label != expected.label {
        return Err(Mismatch::new(
            "eventLabel",
            format!("{:?}", expected.label),
            format!("{:?}", res.fdm_ref.event_label),
        ));
    }
    if res.fdm_ref.fdm_id.trim().is_empty() {
        return Err(Mismatch::new("fdmId", "non vide", "vide"));
    }

    let signature = res.digital_signature.trim();
    let placeholder = is_simulator_placeholder(signature);
    let decoded = base64::engine::general_purpose::STANDARD.decode(signature);
    if signature.is_empty() || decoded.as_ref().is_ok_and(|raw| raw.is_empty()) {
        return Err(Mismatch::new("digitalSignature", "non vide", "vide"));
    }
    if decoded.is_err() && !placeholder {
        return Err(Mismatch::new(
            "digitalSignature",
            "base64",
            "valeur illisible",
        ));
    }

    // Only a normal sale (N) carries a short signature and a
    // verification URL, both printed on the customer ticket.
    if expected.label == EventLabel::N {
        let Some(short) = res
            .short_signature
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        else {
            return Err(Mismatch::new("shortSignature", "présente", "absente"));
        };
        if placeholder {
            // Nothing to derive it from: it must still look like one.
            if !is_sha1_hex(short.trim()) {
                return Err(Mismatch::new("shortSignature", "40 hexadécimaux", short));
            }
        } else {
            let computed = compute_short_signature(signature)
                .map_err(|_| Mismatch::new("digitalSignature", "base64", "valeur illisible"))?;
            if !computed.eq_ignore_ascii_case(short.trim()) {
                return Err(Mismatch::new("shortSignature", computed, short));
            }
        }
        if res
            .verification_url
            .as_deref()
            .is_none_or(|url| url.trim().is_empty())
        {
            return Err(Mismatch::new("verificationUrl", "présente", "absente"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::FdmReference;
    use serde_json::json;

    const SIG: &[u8] = b"sixty-four bytes of raw r||s standing in for an ECDSA signature";

    fn sig_b64() -> String {
        base64::engine::general_purpose::STANDARD.encode(SIG)
    }

    fn request() -> Value {
        json!({
            "posId": "CFOD0061234567",
            "posFiscalTicketNo": 42,
            "posDateTime": "2026-09-26T12:00:00+02:00",
            "terminalId": "T1",
            "deviceId": "00:1b:44:11:3a:b7",
        })
    }

    fn sale_response() -> SignResult {
        SignResult {
            pos_id: "CFOD0061234567".into(),
            pos_fiscal_ticket_no: 42,
            pos_date_time: "2026-09-26T12:00:00+02:00".into(),
            terminal_id: Some("T1".into()),
            device_id: "00:1b:44:11:3a:b7".into(),
            event_operation: EventOperation::Sale,
            fdm_ref: FdmReference {
                fdm_id: "CBX01000042".into(),
                fdm_date_time: "2026-09-26T10:00:01Z".into(),
                event_label: EventLabel::N,
                event_counter: 7,
                total_counter: 19,
            },
            fdm_sw_version: "1.0".into(),
            digital_signature: sig_b64(),
            short_signature: Some(compute_short_signature(&sig_b64()).unwrap()),
            verification_url: Some("https://gs2.be/x".into()),
            vat_calc: None,
            buffer_capacity_used: 0.0,
            warnings: None,
            informations: None,
            footer: vec![],
        }
    }

    fn sale() -> ExpectedEvent {
        ExpectedEvent::for_mutation("signSale", false).unwrap()
    }

    fn field_of(res: &SignResult, expected: ExpectedEvent) -> &'static str {
        check_sign_result(&request(), expected, res)
            .unwrap_err()
            .field
    }

    #[test]
    fn a_consistent_sale_passes() {
        assert_eq!(
            check_sign_result(&request(), sale(), &sale_response()),
            Ok(())
        );
    }

    #[test]
    fn every_mutation_maps_to_its_spec_operation_and_label() {
        use EventLabel as L;
        use EventOperation as O;
        let table = [
            ("signWorkIn", O::WorkIn, L::S),
            ("signWorkOut", O::WorkOut, L::S),
            ("signSale", O::Sale, L::N),
            ("signInvoice", O::Invoice, L::I),
            ("signCostCenterChange", O::CostCenterChange, L::P),
            ("signOrder", O::Order, L::P),
            ("signPreBill", O::PreBill, L::P),
            ("signMoneyInOut", O::MoneyInOut, L::F),
            ("signDrawerOpen", O::DrawerOpen, L::F),
            ("signPaymentCorrection", O::PaymentCorrection, L::F),
            ("signReportTurnoverX", O::ReportTurnoverX, L::R),
            ("signReportTurnoverZ", O::ReportTurnoverZ, L::R),
            ("signReportUserX", O::ReportUserX, L::R),
            ("signReportUserZ", O::ReportUserZ, L::R),
            ("signCopy", O::Copy, L::C),
        ];
        for (mutation, operation, label) in table {
            assert_eq!(
                ExpectedEvent::for_mutation(mutation, false),
                Some(ExpectedEvent { operation, label }),
                "{mutation}"
            );
            assert_eq!(
                ExpectedEvent::for_mutation(mutation, true),
                Some(ExpectedEvent {
                    operation,
                    label: L::T
                }),
                "{mutation} in training"
            );
        }
        assert_eq!(
            ExpectedEvent::for_mutation("signSomethingElse", false),
            None
        );
    }

    #[test]
    fn another_pos_id_is_refused() {
        let mut res = sale_response();
        res.pos_id = "CPOS0031234567".into();
        assert_eq!(field_of(&res, sale()), "posId");
    }

    #[test]
    fn another_ticket_number_is_refused() {
        let mut res = sale_response();
        res.pos_fiscal_ticket_no = 41;
        assert_eq!(field_of(&res, sale()), "posFiscalTicketNo");
    }

    #[test]
    fn another_pos_date_time_is_refused() {
        let mut res = sale_response();
        res.pos_date_time = "2026-09-26T12:00:01+02:00".into();
        assert_eq!(field_of(&res, sale()), "posDateTime");
    }

    #[test]
    fn the_same_instant_in_another_offset_notation_passes() {
        let mut res = sale_response();
        res.pos_date_time = "2026-09-26T10:00:00Z".into();
        assert_eq!(check_sign_result(&request(), sale(), &res), Ok(()));
    }

    #[test]
    fn another_device_is_refused_but_letter_case_is_not_a_mismatch() {
        let mut res = sale_response();
        res.device_id = "00:1B:44:11:3A:B7".into();
        assert_eq!(check_sign_result(&request(), sale(), &res), Ok(()));
        res.device_id = "00:1b:44:11:3a:b8".into();
        assert_eq!(field_of(&res, sale()), "deviceId");
    }

    #[test]
    fn a_missing_or_different_terminal_is_refused() {
        let mut res = sale_response();
        res.terminal_id = None;
        assert_eq!(field_of(&res, sale()), "terminalId");
        res.terminal_id = Some("T2".into());
        assert_eq!(field_of(&res, sale()), "terminalId");
    }

    #[test]
    fn a_request_without_terminal_accepts_any_terminal_echo() {
        let mut req = request();
        req.as_object_mut().unwrap().remove("terminalId");
        let mut res = sale_response();
        res.terminal_id = None;
        assert_eq!(check_sign_result(&req, sale(), &res), Ok(()));
    }

    #[test]
    fn another_operation_is_refused() {
        let mut res = sale_response();
        res.event_operation = EventOperation::PreBill;
        assert_eq!(field_of(&res, sale()), "eventOperation");
    }

    #[test]
    fn a_training_label_for_a_real_sale_is_refused() {
        let mut res = sale_response();
        res.fdm_ref.event_label = EventLabel::T;
        assert_eq!(field_of(&res, sale()), "eventLabel");
    }

    #[test]
    fn a_real_label_for_a_training_sale_is_refused() {
        let training = ExpectedEvent::for_mutation("signSale", true).unwrap();
        let res = sale_response();
        assert_eq!(field_of(&res, training), "eventLabel");
    }

    #[test]
    fn a_training_sale_needs_no_short_signature() {
        let training = ExpectedEvent::for_mutation("signSale", true).unwrap();
        let mut res = sale_response();
        res.fdm_ref.event_label = EventLabel::T;
        res.short_signature = None;
        res.verification_url = None;
        assert_eq!(check_sign_result(&request(), training, &res), Ok(()));
    }

    #[test]
    fn an_empty_fdm_id_is_refused() {
        let mut res = sale_response();
        res.fdm_ref.fdm_id = " ".into();
        assert_eq!(field_of(&res, sale()), "fdmId");
    }

    #[test]
    fn an_empty_or_unreadable_digital_signature_is_refused() {
        for bad in ["", "   ", "not base64!!!"] {
            let mut res = sale_response();
            res.digital_signature = bad.into();
            assert_eq!(field_of(&res, sale()), "digitalSignature", "{bad:?}");
        }
    }

    /// What module2's simulator answered to a sale on 2026-09-27.
    const SIMULATOR_SIGNATURE: &str = "dummy_BU15o87GrSHWp7WJT8OLT86vLPrj09/RHQFa9g";
    const SIMULATOR_SHORT: &str = "415342205c2c51304ccf415071f15cec716b6e86";

    fn simulator_sale() -> SignResult {
        let mut res = sale_response();
        res.digital_signature = SIMULATOR_SIGNATURE.into();
        res.short_signature = Some(SIMULATOR_SHORT.into());
        res
    }

    #[test]
    fn the_simulator_placeholder_signature_passes() {
        assert_eq!(
            check_sign_result(&request(), sale(), &simulator_sale()),
            Ok(())
        );
    }

    #[test]
    fn the_simulator_placeholder_still_needs_a_short_signature_shaped_like_one() {
        for bad in [None, Some(""), Some("not-hex"), Some("41534220")] {
            let mut res = simulator_sale();
            res.short_signature = bad.map(str::to_string);
            assert_eq!(field_of(&res, sale()), "shortSignature", "{bad:?}");
        }
    }

    #[test]
    fn the_simulator_placeholder_keeps_every_other_check() {
        let mut res = simulator_sale();
        res.pos_fiscal_ticket_no += 1;
        assert_eq!(field_of(&res, sale()), "posFiscalTicketNo");
        let mut res = simulator_sale();
        res.verification_url = None;
        assert_eq!(field_of(&res, sale()), "verificationUrl");
    }

    #[test]
    fn only_the_exact_prefix_is_a_placeholder() {
        // An unreadable signature that merely looks alike is refused.
        for bad in ["dummy_", "Dummy_BU15o87", "xdummy_BU15o87", "dummy BU15o87"] {
            let mut res = simulator_sale();
            res.digital_signature = bad.into();
            assert_eq!(field_of(&res, sale()), "digitalSignature", "{bad:?}");
        }
    }

    #[test]
    fn a_sale_without_short_signature_is_refused() {
        for missing in [None, Some(String::new())] {
            let mut res = sale_response();
            res.short_signature = missing;
            assert_eq!(field_of(&res, sale()), "shortSignature");
        }
    }

    #[test]
    fn a_short_signature_not_derived_from_the_signature_is_refused() {
        let mut res = sale_response();
        // The placeholder value of the official v1.7 fixtures.
        res.short_signature = Some("ca39a3ee5e6b4b0d3255bfef95601890afd80709".into());
        let err = check_sign_result(&request(), sale(), &res).unwrap_err();
        assert_eq!(err.field, "shortSignature");
        assert_eq!(err.expected, compute_short_signature(&sig_b64()).unwrap());
    }

    #[test]
    fn a_short_signature_in_upper_case_passes() {
        let mut res = sale_response();
        res.short_signature = res.short_signature.map(|s| s.to_uppercase());
        assert_eq!(check_sign_result(&request(), sale(), &res), Ok(()));
    }

    #[test]
    fn a_sale_without_verification_url_is_refused() {
        for missing in [None, Some(" ".to_string())] {
            let mut res = sale_response();
            res.verification_url = missing;
            assert_eq!(field_of(&res, sale()), "verificationUrl");
        }
    }

    #[test]
    fn events_other_than_n_need_no_short_signature_or_url() {
        let cases = [
            ("signPreBill", EventOperation::PreBill, EventLabel::P),
            ("signCopy", EventOperation::Copy, EventLabel::C),
            ("signWorkIn", EventOperation::WorkIn, EventLabel::S),
            ("signDrawerOpen", EventOperation::DrawerOpen, EventLabel::F),
            (
                "signReportTurnoverZ",
                EventOperation::ReportTurnoverZ,
                EventLabel::R,
            ),
        ];
        for (mutation, operation, label) in cases {
            let mut res = sale_response();
            res.event_operation = operation;
            res.fdm_ref.event_label = label;
            res.short_signature = None;
            res.verification_url = None;
            let expected = ExpectedEvent::for_mutation(mutation, false).unwrap();
            assert_eq!(
                check_sign_result(&request(), expected, &res),
                Ok(()),
                "{mutation}"
            );
        }
    }

    #[test]
    fn the_message_names_field_expected_and_received() {
        let mut res = sale_response();
        res.pos_fiscal_ticket_no = 41;
        let msg = check_sign_result(&request(), sale(), &res)
            .unwrap_err()
            .to_string();
        assert_eq!(msg, "posFiscalTicketNo attendu 42, reçu 41");
    }
}
