//! The driver of `pos-scale` against the simulator, over a real socket.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use pos_scale::frame::{CheckValue, PriceDigits};
use pos_scale::{DriverConfig, Failure, Progress, Reading, ScaleDriver, WeighRequest};
use pos_scale_mock::MockScale;

const CHECK: &str = "4711F336";

fn values() -> Vec<CheckValue> {
    vec![CheckValue {
        checksum: 0x4711,
        correction: 0xF336,
    }]
}

fn config(check_values: Vec<CheckValue>) -> DriverConfig {
    let mut c = DriverConfig::new(PriceDigits::Five, check_values, Duration::from_secs(5));
    c.answer_timeout = Duration::from_millis(150);
    c.poll_interval = Duration::from_millis(5);
    c
}

async fn driver(scale: &MockScale) -> ScaleDriver {
    let address = scale.spawn().await;
    ScaleDriver::new(config(values()), address.to_string())
}

fn request(unit_price_cents: i64) -> WeighRequest {
    WeighRequest {
        unit_price_cents,
        tare_grams: None,
        text: None,
    }
}

const UC4112: Reading = Reading {
    weight_grams: 278,
    unit_price_cents: 4500,
    amount_cents: 1251,
};

#[tokio::test]
async fn a_weighing_returns_what_the_scale_weighed() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Ok(UC4112));
    assert!(outcome.checks.is_empty());
}

#[tokio::test]
async fn the_tare_and_the_text_reach_the_scale() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    let outcome = driver(&scale)
        .await
        .weigh(
            WeighRequest {
                unit_price_cents: 4500,
                tare_grams: Some(30),
                text: Some("Bol malatang ".into()),
            },
            |_| {},
        )
        .await;
    assert_eq!(outcome.result, Ok(UC4112));
    let state = scale.state();
    assert_eq!(state.tare_grams, Some(30));
    assert_eq!(state.text.as_deref(), Some("Bol malatang "));
}

#[tokio::test]
async fn motion_is_waited_out_and_reported() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.motion = 3);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let outcome = driver(&scale)
        .await
        .weigh(request(4500), move |p| log.lock().unwrap().push(p))
        .await;
    assert_eq!(outcome.result, Ok(UC4112));
    let seen = seen.lock().unwrap();
    assert_eq!(seen[0], Progress::Waiting);
    assert_eq!(
        seen.iter().filter(|p| **p == Progress::Stabilizing).count(),
        3
    );
}

#[tokio::test]
async fn the_check_runs_at_power_on_then_the_weighing_goes_on() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.require_check();
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Ok(UC4112));
    assert_eq!(scale.state().checks_answered, 1);
    assert!(scale.state().verified);
    assert_eq!(outcome.checks.len(), 3, "{:?}", outcome.checks);
}

#[tokio::test]
async fn a_refused_check_leaves_no_price_on_the_scale() {
    let scale = MockScale::new("00000000");
    scale.load(278);
    scale.require_check();
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::CheckFailed));
    assert_eq!(scale.state().unit_price_cents, None);
    assert_eq!(scale.state().weighings, 0);
}

#[tokio::test]
async fn without_values_the_check_fails_closed() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.require_check();
    let address = scale.spawn().await;
    let driver = ScaleDriver::new(config(Vec::new()), address.to_string());
    let outcome = driver.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::CheckValuesMissing));
    assert_eq!(scale.state().weighings, 0);
}

#[tokio::test]
async fn the_fiftieth_weighing_asks_for_the_check_again() {
    let scale = MockScale::new(CHECK);
    let driver = driver(&scale).await;
    for i in 0..51 {
        scale.load(100 + i);
        let outcome = driver.weigh(request(1000), |_| {}).await;
        assert!(outcome.result.is_ok(), "weighing {i}: {outcome:?}");
        assert_eq!(outcome.checks.is_empty(), i != 50, "weighing {i}");
    }
    assert_eq!(scale.state().weighings, 51);
    assert_eq!(scale.state().checks_answered, 1);
}

#[tokio::test]
async fn the_same_load_is_never_weighed_twice() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    let driver = driver(&scale).await;
    assert!(driver.weigh(request(4500), |_| {}).await.result.is_ok());
    assert_eq!(
        driver.weigh(request(4500), |_| {}).await.result,
        Err(Failure::NoMotion)
    );
    scale.load(279);
    assert!(driver.weigh(request(4500), |_| {}).await.result.is_ok());
}

#[tokio::test]
async fn each_scale_state_ends_with_its_failure() {
    for (grams, failure) in [
        (10, Failure::BelowMinimum),
        (-5, Failure::Underload),
        (20_000, Failure::Overload),
    ] {
        let scale = MockScale::new(CHECK);
        scale.load(grams);
        let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
        assert_eq!(outcome.result, Err(failure), "{grams} g");
    }
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.forced_status = Some(1));
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::ScaleError));
}

#[tokio::test]
async fn a_wrong_price_from_the_scale_is_refused() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.price_error_cents = 5);
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::PriceMismatch));

    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.echo_price_cents = Some(4600));
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::PriceMismatch));
}

#[tokio::test]
async fn another_unit_is_refused() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.status_byte = Some(0x31));
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::UnitUnsupported));
}

#[tokio::test]
async fn a_damaged_answer_is_asked_again() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.corrupt_next = true);
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Ok(UC4112));
}

#[tokio::test]
async fn a_scale_that_stops_answering_is_unreachable() {
    let scale = MockScale::new(CHECK);
    scale.with(|s| s.off = true);
    let outcome = driver(&scale).await.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::Unreachable));

    let driver = ScaleDriver::new(config(values()), "127.0.0.1:1".into());
    assert_eq!(driver.probe().await.result, Err(Failure::Unreachable));
}

#[tokio::test]
async fn cancelling_ends_the_weighing_with_nothing() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.motion = 100_000);
    let driver = Arc::new(driver(&scale).await);
    let running = {
        let driver = driver.clone();
        tokio::spawn(async move { driver.weigh(request(4500), |_| {}).await })
    };
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(driver.is_busy());
    driver.cancel();
    let outcome = running.await.unwrap();
    assert_eq!(outcome.result, Err(Failure::Cancelled));
    assert!(!driver.is_busy());
    assert_eq!(scale.state().weighings, 0);
}

#[tokio::test]
async fn a_load_that_never_settles_times_out() {
    let scale = MockScale::new(CHECK);
    scale.load(278);
    scale.with(|s| s.motion = 100_000);
    let address = scale.spawn().await;
    let mut c = config(values());
    c.weigh_timeout = Duration::from_millis(60);
    let driver = ScaleDriver::new(c, address.to_string());
    let outcome = driver.weigh(request(4500), |_| {}).await;
    assert_eq!(outcome.result, Err(Failure::TimedOut));
}

#[tokio::test]
async fn a_probe_hears_the_scale_and_passes_the_check() {
    let scale = MockScale::new(CHECK);
    scale.require_check();
    let driver = driver(&scale).await;
    let outcome = driver.probe().await;
    assert_eq!(outcome.result, Ok(()));
    assert!(scale.state().verified);
    assert_eq!(driver.probe().await.result, Ok(()));
}

#[tokio::test]
async fn the_link_comes_back_after_the_scale_went_away() {
    // First connection refused, then the same driver works.
    let scale = MockScale::new(CHECK);
    scale.load(278);
    let address = scale.spawn().await;
    let driver = ScaleDriver::new(config(values()), address.to_string());
    assert!(driver.weigh(request(4500), |_| {}).await.result.is_ok());
    scale.load(300);
    assert!(driver.weigh(request(4500), |_| {}).await.result.is_ok());
}
