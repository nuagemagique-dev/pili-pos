//! A terminal that answers from a script, for the engine's tests
//! (feature `scripted`, never in a release build).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{EftError, Outcome, PaymentRequest};

/// What the scripted terminal answers, in order, and what it was asked.
#[derive(Debug, Default)]
pub struct Script {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    pay: VecDeque<Result<Outcome, EftError>>,
    status: VecDeque<Result<Outcome, EftError>>,
    check: VecDeque<Result<(), EftError>>,
    pay_delay: Duration,
    check_delay: Duration,
    calls: Vec<Call>,
}

/// One call the scripted terminal received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Pay {
        reference: String,
        amount_cents: i64,
    },
    Status {
        reference: String,
    },
    Abort {
        reference: String,
    },
    Check,
}

impl Script {
    /// The recorded state; a poisoned lock (a panicking test thread) still holds valid data.
    fn state(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The next answer to [`crate::Terminal::pay`].
    pub fn on_pay(&self, answer: Result<Outcome, EftError>) {
        self.state().pay.push_back(answer);
    }

    /// The next answer to [`crate::Terminal::status`].
    pub fn on_status(&self, answer: Result<Outcome, EftError>) {
        self.state().status.push_back(answer);
    }

    /// The next answer to [`crate::Terminal::check`] (reachable when
    /// none is scripted).
    pub fn on_check(&self, answer: Result<(), EftError>) {
        self.state().check.push_back(answer);
    }

    /// How long each payment takes before its answer (an abort test
    /// needs a payment in progress).
    pub fn set_pay_delay(&self, delay: Duration) {
        self.state().pay_delay = delay;
    }

    /// How long each reachability check takes before its answer (a
    /// probe test needs a terminal that hangs).
    pub fn set_check_delay(&self, delay: Duration) {
        self.state().check_delay = delay;
    }

    /// Every call so far.
    pub fn calls(&self) -> Vec<Call> {
        self.state().calls.clone()
    }

    pub(crate) async fn pay(&self, request: &PaymentRequest) -> Result<Outcome, EftError> {
        let delay = {
            let mut i = self.state();
            i.calls.push(Call::Pay {
                reference: request.reference.clone(),
                amount_cents: request.amount_cents,
            });
            i.pay_delay
        };
        tokio::time::sleep(delay).await;
        self.state()
            .pay
            .pop_front()
            .unwrap_or_else(|| Err(EftError::Unreachable("no scripted answer".into())))
    }

    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "same async signature as the real drivers"
    )]
    pub(crate) async fn status(&self, reference: &str) -> Result<Outcome, EftError> {
        let mut i = self.state();
        i.calls.push(Call::Status {
            reference: reference.to_string(),
        });
        i.status
            .pop_front()
            .unwrap_or_else(|| Err(EftError::Unreachable("no scripted answer".into())))
    }

    pub(crate) async fn check(&self) -> Result<(), EftError> {
        let delay = {
            let mut i = self.state();
            i.calls.push(Call::Check);
            i.check_delay
        };
        tokio::time::sleep(delay).await;
        self.state().check.pop_front().unwrap_or(Ok(()))
    }

    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "same async signature as the real drivers"
    )]
    pub(crate) async fn abort(&self, reference: &str) -> Result<(), EftError> {
        self.state().calls.push(Call::Abort {
            reference: reference.to_string(),
        });
        Ok(())
    }
}
