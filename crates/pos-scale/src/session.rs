//! One conversation with the scale as a state machine: it takes messages
//! and silences, and returns bytes to send and what happened. It has no
//! clock and no I/O (the driver has both).
//!
//! A weighing runs: unit price, ACK, then `EOT ENQ` until a record 02. A
//! NAK asks for the status (record 08): still moving or price not ready
//! polls on, anything else ends the weighing with nothing weighed. The
//! scale may ask for the software check (record 11, 32h) at any of those
//! steps: record 10 goes out, the next ENQ gets the verdict, and the price
//! is sent again. There is no way around the check.

use pos_core::money::weighed_amount_cents;

use crate::frame::{self, CheckValue, Message, PriceDigits, STATUS_KG, WeightRecord};

/// The largest gap in cents between the scale's price and ours. The scale
/// rounds by a rule of its own; past a cent the frame is not trusted.
pub const MAX_PRICE_GAP_CENTS: i64 = 1;
/// How many times a request is sent again after a silence or a parity error.
pub const MAX_RETRIES: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeighRequest {
    pub unit_price_cents: i64,
    pub tare_grams: Option<i64>,
    /// Exactly [`frame::TEXT_LEN`] ASCII characters ([`frame::scale_text`]).
    pub text: Option<String>,
}

/// A checked weighing: what the ticket and the signed line take as they
/// are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub weight_grams: i64,
    pub unit_price_cents: i64,
    pub amount_cents: i64,
}

/// Why nothing was weighed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// No answer, or the link is down.
    Unreachable,
    /// Code 30: under the scale's minimum weight.
    BelowMinimum,
    /// Code 31.
    Underload,
    /// Code 32.
    Overload,
    /// Code 21: the last load was never taken off. Protects from weighing
    /// one bowl twice.
    NoMotion,
    /// Codes 11, 12 and 13: the scale refused the price, tare or text.
    PriceRefused,
    /// Code 01, or a code this table does not know.
    ScaleError,
    /// Code 02 again after a retry, or frames that stay unreadable.
    Damaged,
    /// The scale refused the software check.
    CheckFailed,
    /// The scale asked for the check and no value is configured.
    CheckValuesMissing,
    /// The record 02 disagrees with the price sent or with the weight.
    PriceMismatch,
    /// The scale weighs in something else than kilograms.
    UnitUnsupported,
    /// The price, tare or text does not fit the record.
    PriceTooLarge,
    /// The operator cancelled, or the weighing timed out.
    Cancelled,
    TimedOut,
}

/// Where a weighing stands, for the operator's dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    /// The price is on the scale, waiting for the load.
    Waiting,
    /// Code 20: the load still moves.
    Stabilizing,
    /// The software check is running.
    Checking,
}

/// A step of the software check, for the audit log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckEvent {
    Requested {
        z: String,
    },
    Answered {
        values: usize,
    },
    Passed,
    Refused,
    /// Asked for with nothing configured to answer.
    NoValues,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Send(Vec<u8>),
    /// Send after the polling interval.
    SendLater(Vec<u8>),
    Progress(Progress),
    Check(CheckEvent),
    Done(Reading),
    Failed(Failure),
    /// A probe got an answer.
    Connected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Waiting for the ACK of the price.
    Price,
    /// Waiting for the answer to `EOT ENQ`.
    Polling,
    /// Waiting for record 09.
    Status,
    /// Waiting for the ACK of record 10.
    CheckAck,
    /// Waiting for the verdict of record 11.
    CheckVerdict,
    Finished,
}

#[derive(Debug)]
pub struct Session {
    request: Option<WeighRequest>,
    digits: PriceDigits,
    values: Vec<CheckValue>,
    phase: Phase,
    last_sent: Vec<u8>,
    retries: u8,
    parity_retries: u8,
}

impl Session {
    /// A session that weighs once.
    pub fn weighing(request: WeighRequest, digits: PriceDigits, values: Vec<CheckValue>) -> Self {
        Self {
            request: Some(request),
            digits,
            values,
            phase: Phase::Price,
            last_sent: Vec::new(),
            retries: 0,
            parity_retries: 0,
        }
    }

    /// A session that only asks whether the scale answers, running the
    /// software check if the scale wants it.
    pub fn probe(digits: PriceDigits, values: Vec<CheckValue>) -> Self {
        Self {
            request: None,
            digits,
            values,
            phase: Phase::Polling,
            last_sent: Vec::new(),
            retries: 0,
            parity_retries: 0,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.phase == Phase::Finished
    }

    pub fn start(&mut self) -> Vec<Output> {
        if self.request.is_some() {
            self.send_price()
        } else {
            self.send(Phase::Polling, frame::enq())
        }
    }

    fn send(&mut self, phase: Phase, bytes: Vec<u8>) -> Vec<Output> {
        self.phase = phase;
        self.last_sent.clone_from(&bytes);
        vec![Output::Send(bytes)]
    }

    fn poll_later(&mut self) -> Vec<Output> {
        self.phase = Phase::Polling;
        self.last_sent = frame::enq();
        vec![Output::SendLater(frame::enq())]
    }

    fn fail(&mut self, failure: Failure) -> Vec<Output> {
        self.phase = Phase::Finished;
        vec![Output::Failed(failure)]
    }

    fn send_price(&mut self) -> Vec<Output> {
        let Some(request) = &self.request else {
            return self.send(Phase::Polling, frame::enq());
        };
        match frame::price_record(
            request.unit_price_cents,
            self.digits,
            request.tare_grams,
            request.text.as_deref(),
        ) {
            Some(bytes) => self.send(Phase::Price, bytes),
            None => self.fail(Failure::PriceTooLarge),
        }
    }

    /// No answer in time: the last request again, then give up.
    pub fn on_silence(&mut self) -> Vec<Output> {
        if self.phase == Phase::Finished {
            return Vec::new();
        }
        if self.retries >= MAX_RETRIES {
            return self.fail(Failure::Unreachable);
        }
        self.retries += 1;
        vec![Output::Send(self.last_sent.clone())]
    }

    pub fn on_message(&mut self, message: Message) -> Vec<Output> {
        if self.phase == Phase::Finished {
            return Vec::new();
        }
        self.retries = 0;
        if let Message::CheckRequest(z) = message {
            return self.answer_check(z);
        }
        if matches!(message, Message::Garbled) {
            return self.on_garbled();
        }
        match self.phase {
            Phase::Price => match message {
                Message::Ack => {
                    let mut out = vec![Output::Progress(Progress::Waiting)];
                    out.extend(self.send(Phase::Polling, frame::enq()));
                    out
                }
                Message::Nak => self.send(Phase::Status, frame::status_request()),
                _ => self.on_garbled(),
            },
            Phase::Polling => self.on_poll_answer(message),
            Phase::Status => match message {
                Message::Status(code) => self.on_status(code),
                _ => self.on_garbled(),
            },
            Phase::CheckAck => match message {
                Message::Ack => self.send(Phase::CheckVerdict, frame::enq()),
                Message::Nak => self.send(Phase::Status, frame::status_request()),
                _ => self.on_garbled(),
            },
            Phase::CheckVerdict => match message {
                Message::CheckOk => {
                    let mut out = vec![Output::Check(CheckEvent::Passed)];
                    out.extend(self.after_check());
                    out
                }
                Message::CheckFailed => {
                    let mut out = vec![Output::Check(CheckEvent::Refused)];
                    out.extend(self.fail(Failure::CheckFailed));
                    out
                }
                // Asked again while it is working the answer out.
                Message::Nak => self.poll_later_verdict(),
                _ => self.on_garbled(),
            },
            Phase::Finished => Vec::new(),
        }
    }

    fn poll_later_verdict(&mut self) -> Vec<Output> {
        self.phase = Phase::CheckVerdict;
        self.last_sent = frame::enq();
        vec![Output::SendLater(frame::enq())]
    }

    fn after_check(&mut self) -> Vec<Output> {
        if self.request.is_some() {
            self.send_price()
        } else {
            self.phase = Phase::Finished;
            vec![Output::Connected]
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "takes the message by value like the other phase handlers"
    )]
    fn on_poll_answer(&mut self, message: Message) -> Vec<Output> {
        match message {
            Message::Weight(record) if self.request.is_some() => self.on_weight(record),
            // A probe only wants to hear the scale.
            _ if self.request.is_none() => {
                self.phase = Phase::Finished;
                vec![Output::Connected]
            }
            // The late ACK of a request sent twice: the answer is still coming.
            Message::Ack => Vec::new(),
            Message::Nak => self.send(Phase::Status, frame::status_request()),
            Message::CheckOk => {
                let mut out = vec![Output::Check(CheckEvent::Passed)];
                out.extend(self.poll_later());
                out
            }
            Message::CheckFailed => {
                let mut out = vec![Output::Check(CheckEvent::Refused)];
                out.extend(self.fail(Failure::CheckFailed));
                out
            }
            _ => self.on_garbled(),
        }
    }

    fn on_weight(&mut self, record: WeightRecord) -> Vec<Output> {
        let Some(request) = &self.request else {
            return self.on_garbled();
        };
        if record.status != STATUS_KG {
            return self.fail(Failure::UnitUnsupported);
        }
        let expected = weighed_amount_cents(record.weight_grams, request.unit_price_cents);
        if record.weight_grams <= 0
            || record.unit_price_cents != request.unit_price_cents
            || (record.selling_price_cents - expected).abs() > MAX_PRICE_GAP_CENTS
        {
            return self.fail(Failure::PriceMismatch);
        }
        self.phase = Phase::Finished;
        vec![Output::Done(Reading {
            weight_grams: record.weight_grams,
            unit_price_cents: record.unit_price_cents,
            amount_cents: record.selling_price_cents,
        })]
    }

    fn on_status(&mut self, code: u8) -> Vec<Output> {
        match code {
            // No error, price not ready yet: ask again.
            0 | 22 => self.poll_later(),
            20 => {
                let mut out = vec![Output::Progress(Progress::Stabilizing)];
                out.extend(self.poll_later());
                out
            }
            2 => {
                if self.parity_retries >= 1 {
                    return self.fail(Failure::Damaged);
                }
                self.parity_retries += 1;
                self.send_price()
            }
            21 => self.fail(Failure::NoMotion),
            30 => self.fail(Failure::BelowMinimum),
            31 => self.fail(Failure::Underload),
            32 => self.fail(Failure::Overload),
            11..=13 => self.fail(Failure::PriceRefused),
            _ => self.fail(Failure::ScaleError),
        }
    }

    fn answer_check(&mut self, z: String) -> Vec<Output> {
        let mut out = vec![
            Output::Progress(Progress::Checking),
            Output::Check(CheckEvent::Requested { z }),
        ];
        if self.values.is_empty() {
            out.push(Output::Check(CheckEvent::NoValues));
            out.extend(self.fail(Failure::CheckValuesMissing));
            return out;
        }
        out.push(Output::Check(CheckEvent::Answered {
            values: self.values.len().min(frame::MAX_CHECK_VALUES),
        }));
        let record = frame::check_record(&self.values);
        out.extend(self.send(Phase::CheckAck, record));
        out
    }

    fn on_garbled(&mut self) -> Vec<Output> {
        if self.parity_retries >= MAX_RETRIES {
            return self.fail(Failure::Damaged);
        }
        self.parity_retries += 1;
        vec![Output::Send(self.last_sent.clone())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECK: CheckValue = CheckValue {
        checksum: 0x4711,
        correction: 0xF336,
    };

    fn session() -> Session {
        Session::weighing(
            WeighRequest {
                unit_price_cents: 4500,
                tare_grams: None,
                text: None,
            },
            PriceDigits::Five,
            vec![CHECK],
        )
    }

    fn weight(grams: i64, unit: i64, price: i64) -> Message {
        Message::Weight(WeightRecord {
            status: STATUS_KG,
            weight_grams: grams,
            unit_price_cents: unit,
            selling_price_cents: price,
        })
    }

    fn sent(out: &[Output]) -> Vec<Vec<u8>> {
        out.iter()
            .filter_map(|o| match o {
                Output::Send(b) | Output::SendLater(b) => Some(b.clone()),
                _ => None,
            })
            .collect()
    }

    fn failed(out: &[Output]) -> Option<Failure> {
        out.iter().find_map(|o| match o {
            Output::Failed(f) => Some(*f),
            _ => None,
        })
    }

    /// Up to the first poll.
    fn polling() -> Session {
        let mut s = session();
        s.start();
        s.on_message(Message::Ack);
        s
    }

    #[test]
    fn a_weighing_goes_from_the_price_to_the_reading() {
        let mut s = session();
        assert_eq!(
            sent(&s.start()),
            vec![b"\x04\x0201\x1b04500\x1b\x03".to_vec()]
        );
        let out = s.on_message(Message::Ack);
        assert!(out.contains(&Output::Progress(Progress::Waiting)));
        assert_eq!(sent(&out), vec![frame::enq()]);
        let out = s.on_message(weight(278, 4500, 1251));
        assert_eq!(
            out,
            vec![Output::Done(Reading {
                weight_grams: 278,
                unit_price_cents: 4500,
                amount_cents: 1251,
            })]
        );
        assert!(s.is_finished());
    }

    #[test]
    fn a_nak_asks_for_the_status_and_motion_polls_on() {
        let mut s = polling();
        assert_eq!(
            sent(&s.on_message(Message::Nak)),
            vec![frame::status_request()]
        );
        let out = s.on_message(Message::Status(20));
        assert!(out.contains(&Output::Progress(Progress::Stabilizing)));
        assert!(out.contains(&Output::SendLater(frame::enq())));
        s.on_message(Message::Nak);
        assert!(
            s.on_message(Message::Status(22))
                .contains(&Output::SendLater(frame::enq()))
        );
        assert!(!s.is_finished());
    }

    #[test]
    fn every_final_status_ends_the_weighing_with_its_failure() {
        for (code, failure) in [
            (21, Failure::NoMotion),
            (30, Failure::BelowMinimum),
            (31, Failure::Underload),
            (32, Failure::Overload),
            (11, Failure::PriceRefused),
            (12, Failure::PriceRefused),
            (13, Failure::PriceRefused),
            (1, Failure::ScaleError),
            (77, Failure::ScaleError),
        ] {
            let mut s = polling();
            s.on_message(Message::Nak);
            assert_eq!(failed(&s.on_message(Message::Status(code))), Some(failure));
            assert!(s.is_finished());
        }
    }

    #[test]
    fn a_parity_error_sends_the_price_once_more_then_gives_up() {
        let mut s = session();
        s.start();
        s.on_message(Message::Nak);
        assert_eq!(
            sent(&s.on_message(Message::Status(2))),
            vec![b"\x04\x0201\x1b04500\x1b\x03".to_vec()]
        );
        s.on_message(Message::Nak);
        assert_eq!(
            failed(&s.on_message(Message::Status(2))),
            Some(Failure::Damaged)
        );
    }

    #[test]
    fn the_check_runs_then_the_price_goes_again() {
        let mut s = session();
        s.start();
        let out = s.on_message(Message::CheckRequest("EF".into()));
        assert!(out.contains(&Output::Check(CheckEvent::Requested { z: "EF".into() })));
        assert_eq!(sent(&out), vec![b"\x04\x0210\x1b4711F336\x03".to_vec()]);
        assert_eq!(sent(&s.on_message(Message::Ack)), vec![frame::enq()]);
        let out = s.on_message(Message::CheckOk);
        assert!(out.contains(&Output::Check(CheckEvent::Passed)));
        assert_eq!(sent(&out), vec![b"\x04\x0201\x1b04500\x1b\x03".to_vec()]);
    }

    #[test]
    fn the_check_can_come_in_the_middle_of_a_weighing() {
        let mut s = polling();
        let out = s.on_message(Message::CheckRequest("A1".into()));
        assert_eq!(sent(&out), vec![b"\x04\x0210\x1b4711F336\x03".to_vec()]);
        s.on_message(Message::Ack);
        s.on_message(Message::CheckOk);
        s.on_message(Message::Ack);
        assert_eq!(
            s.on_message(weight(278, 4500, 1251)).first(),
            Some(&Output::Done(Reading {
                weight_grams: 278,
                unit_price_cents: 4500,
                amount_cents: 1251,
            }))
        );
    }

    #[test]
    fn a_refused_check_sends_no_price() {
        let mut s = session();
        s.start();
        s.on_message(Message::CheckRequest("EF".into()));
        s.on_message(Message::Ack);
        let out = s.on_message(Message::CheckFailed);
        assert_eq!(failed(&out), Some(Failure::CheckFailed));
        assert_eq!(sent(&out), Vec::<Vec<u8>>::new());
        assert_eq!(s.on_message(Message::Ack), Vec::<Output>::new());
    }

    #[test]
    fn without_check_values_the_scale_stays_unusable() {
        let mut s = Session::weighing(
            WeighRequest {
                unit_price_cents: 4500,
                tare_grams: None,
                text: None,
            },
            PriceDigits::Five,
            Vec::new(),
        );
        s.start();
        let out = s.on_message(Message::CheckRequest("EF".into()));
        assert_eq!(failed(&out), Some(Failure::CheckValuesMissing));
        assert_eq!(sent(&out), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn a_reading_that_disagrees_is_refused() {
        for message in [
            weight(278, 4600, 1279), // another unit price than the one sent
            weight(278, 4500, 1253), // two cents off
            weight(0, 4500, 0),      // nothing on the scale
        ] {
            let mut s = polling();
            assert_eq!(failed(&s.on_message(message)), Some(Failure::PriceMismatch));
        }
        let mut s = polling();
        let out = s.on_message(Message::Weight(WeightRecord {
            status: 0x31,
            weight_grams: 278,
            unit_price_cents: 4500,
            selling_price_cents: 1251,
        }));
        assert_eq!(failed(&out), Some(Failure::UnitUnsupported));
    }

    #[test]
    fn a_price_a_cent_off_is_the_scales_rule_and_stands() {
        let mut s = polling();
        assert_eq!(
            s.on_message(weight(278, 4500, 1250)).first(),
            Some(&Output::Done(Reading {
                weight_grams: 278,
                unit_price_cents: 4500,
                amount_cents: 1250,
            }))
        );
    }

    #[test]
    fn silence_repeats_the_request_then_gives_up() {
        let mut s = session();
        s.start();
        for _ in 0..MAX_RETRIES {
            assert_eq!(s.on_silence().len(), 1);
        }
        assert_eq!(failed(&s.on_silence()), Some(Failure::Unreachable));
    }

    #[test]
    fn garbled_frames_repeat_the_request_then_give_up() {
        let mut s = polling();
        for _ in 0..MAX_RETRIES {
            assert_eq!(sent(&s.on_message(Message::Garbled)), vec![frame::enq()]);
        }
        assert_eq!(
            failed(&s.on_message(Message::Garbled)),
            Some(Failure::Damaged)
        );
    }

    #[test]
    fn a_price_that_does_not_fit_fails_before_anything_is_sent() {
        let mut s = Session::weighing(
            WeighRequest {
                unit_price_cents: 100_000,
                tare_grams: None,
                text: None,
            },
            PriceDigits::Five,
            vec![CHECK],
        );
        let out = s.start();
        assert_eq!(failed(&out), Some(Failure::PriceTooLarge));
        assert_eq!(sent(&out), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn a_probe_hears_the_scale_and_runs_the_check_if_asked() {
        let mut s = Session::probe(PriceDigits::Five, vec![CHECK]);
        assert_eq!(sent(&s.start()), vec![frame::enq()]);
        assert_eq!(s.on_message(Message::Nak), vec![Output::Connected]);

        let mut s = Session::probe(PriceDigits::Five, vec![CHECK]);
        s.start();
        s.on_message(Message::CheckRequest("EF".into()));
        s.on_message(Message::Ack);
        let out = s.on_message(Message::CheckOk);
        assert!(out.contains(&Output::Check(CheckEvent::Passed)));
        assert!(out.contains(&Output::Connected));
    }

    #[test]
    fn no_sequence_of_messages_makes_a_reading_without_a_valid_record() {
        // Everything but a matching record 02, in every order of three.
        let messages = [
            Message::Ack,
            Message::Nak,
            Message::Status(0),
            Message::Status(20),
            Message::Status(21),
            Message::CheckOk,
            Message::CheckFailed,
            Message::CheckRequest("EF".into()),
            Message::Garbled,
            weight(278, 1, 1),
        ];
        for a in &messages {
            for b in &messages {
                for c in &messages {
                    let mut s = session();
                    s.start();
                    for m in [a, b, c] {
                        let out = s.on_message(m.clone());
                        assert!(
                            !out.iter().any(|o| matches!(o, Output::Done(_))),
                            "{a:?} {b:?} {c:?}"
                        );
                    }
                }
            }
        }
    }
}
