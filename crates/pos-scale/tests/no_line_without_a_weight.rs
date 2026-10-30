//! Whatever the scale link delivers, a weighing ends with a reading only
//! after one complete record 02 whose price the POS can check.

use pos_scale::frame::{CheckValue, Decoder, Message, PriceDigits, WeightRecord};
use pos_scale::session::{Output, Session};
use pos_scale::{Reading, WeighRequest};
use proptest::prelude::*;

const GOOD_WEIGHT: &[u8] = b"\x0202\x1b3\x1b00278\x1b04500\x1b001251\x03";

fn polling() -> Session {
    let mut session = Session::weighing(
        WeighRequest {
            unit_price_cents: 4500,
            tare_grams: None,
            text: None,
        },
        PriceDigits::Five,
        vec![CheckValue {
            checksum: 0x4711,
            correction: 0xF336,
        }],
    );
    session.start();
    session.on_message(Message::Ack);
    session
}

/// Feeds `bytes`; the readings and the weight records heard on the way.
fn feed(bytes: &[u8]) -> (Vec<Reading>, Vec<WeightRecord>) {
    let mut session = polling();
    let mut decoder = Decoder::default();
    let mut readings = Vec::new();
    let mut records = Vec::new();
    for message in decoder.push(bytes) {
        if let Message::Weight(record) = &message {
            records.push(*record);
        }
        for output in session.on_message(message) {
            if let Output::Done(reading) = output {
                readings.push(reading);
            }
        }
    }
    (readings, records)
}

#[test]
fn a_complete_record_gives_the_reading() {
    let (readings, _) = feed(GOOD_WEIGHT);
    assert_eq!(
        readings,
        vec![Reading {
            weight_grams: 278,
            unit_price_cents: 4500,
            amount_cents: 1251,
        }]
    );
}

#[test]
fn every_cut_of_the_record_gives_nothing() {
    for end in 0..GOOD_WEIGHT.len() {
        let (readings, _) = feed(&GOOD_WEIGHT[..end]);
        assert!(readings.is_empty(), "cut at {end}");
    }
}

#[test]
fn every_flipped_digit_of_the_weight_or_the_amount_is_caught() {
    for at in 0..GOOD_WEIGHT.len() {
        for bad in [b'A', 0xF8] {
            let mut bytes = GOOD_WEIGHT.to_vec();
            bytes[at] = bad;
            let (readings, _) = feed(&bytes);
            assert!(readings.is_empty(), "byte {at} = {bad:#x}: {readings:?}");
        }
    }
}

proptest! {
    #[test]
    fn no_reading_without_a_record_the_price_agrees_with(
        noise in prop::collection::vec(any::<u8>(), 0..300),
    ) {
        let (readings, records) = feed(&noise);
        for reading in &readings {
            let heard = records.iter().any(|r| {
                r.weight_grams == reading.weight_grams
                    && r.selling_price_cents == reading.amount_cents
                    && r.unit_price_cents == 4500
            });
            prop_assert!(heard);
        }
    }

    #[test]
    fn noise_around_a_cut_record_gives_nothing(
        before in prop::collection::vec(any::<u8>(), 0..64),
        end in 0..GOOD_WEIGHT.len(),
    ) {
        // Noise that happens to hold STX..ETX is a frame of its own, judged
        // by the property above; here it may not complete the record.
        prop_assume!(!before.contains(&0x02));
        let mut bytes = before;
        bytes.extend_from_slice(&GOOD_WEIGHT[..end]);
        let (readings, _) = feed(&bytes);
        prop_assert!(readings.is_empty());
    }
}
