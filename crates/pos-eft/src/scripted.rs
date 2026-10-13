//! A terminal that answers from a script, for the engine's tests
//! (feature `scripted`, never in a release build).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{EftError, Outcome, PaymentRequest};

/// What the scripted terminal answers, in order, and what it was asked.
#[derive(Default)]
pub struct Script {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    pay: VecDeque<Result<Outcome, EftError>>,
    status: VecDeque<Result<Outcome, EftError>>,
    check: VecDeque<Result<(), EftError>>,
    pay_delay: Duration,
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
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The next answer to [`crate::Terminal::pay`].
    pub fn on_pay(&self, answer: Result<Outcome, EftError>) {
        self.inner.lock().unwrap().pay.push_back(answer);
    }

    /// The next answer to [`crate::Terminal::status`].
    pub fn on_status(&self, answer: Result<Outcome, EftError>) {
        self.inner.lock().unwrap().status.push_back(answer);
    }

    /// The next answer to [`crate::Terminal::check`] (reachable when
    /// none is scripted).
    pub fn on_check(&self, answer: Result<(), EftError>) {
        self.inner.lock().unwrap().check.push_back(answer);
    }

    /// How long each payment takes before its answer (an abort test
    /// needs a payment in progress).
    pub fn set_pay_delay(&self, delay: Duration) {
        self.inner.lock().unwrap().pay_delay = delay;
    }

    /// Every call so far.
    pub fn calls(&self) -> Vec<Call> {
        self.inner.lock().unwrap().calls.clone()
    }

    pub(crate) async fn pay(&self, request: &PaymentRequest) -> Result<Outcome, EftError> {
        let delay = {
            let mut i = self.inner.lock().unwrap();
            i.calls.push(Call::Pay {
                reference: request.reference.clone(),
                amount_cents: request.amount_cents,
            });
            i.pay_delay
        };
        tokio::time::sleep(delay).await;
        self.inner
            .lock()
            .unwrap()
            .pay
            .pop_front()
            .unwrap_or_else(|| Err(EftError::Unreachable("no scripted answer".into())))
    }

    pub(crate) async fn status(&self, reference: &str) -> Result<Outcome, EftError> {
        let mut i = self.inner.lock().unwrap();
        i.calls.push(Call::Status {
            reference: reference.to_string(),
        });
        i.status
            .pop_front()
            .unwrap_or_else(|| Err(EftError::Unreachable("no scripted answer".into())))
    }

    pub(crate) async fn check(&self) -> Result<(), EftError> {
        let mut i = self.inner.lock().unwrap();
        i.calls.push(Call::Check);
        i.check.pop_front().unwrap_or(Ok(()))
    }

    pub(crate) async fn abort(&self, reference: &str) -> Result<(), EftError> {
        self.inner.lock().unwrap().calls.push(Call::Abort {
            reference: reference.to_string(),
        });
        Ok(())
    }
}
