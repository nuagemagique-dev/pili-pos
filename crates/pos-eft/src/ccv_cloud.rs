//! Driver stub: to be implemented (see the crate documentation).

use pos_core::config::PaymentTerminalConfig;

use crate::{EftError, Outcome, PaymentRequest};

pub struct CcvPaymentApi {
    _config: PaymentTerminalConfig,
}

impl CcvPaymentApi {
    #[allow(unused_variables)]
    pub fn new(config: &PaymentTerminalConfig, http: reqwest::Client) -> Result<Self, EftError> {
        Ok(Self {
            _config: config.clone(),
        })
    }

    pub async fn pay(&self, _request: &PaymentRequest) -> Result<Outcome, EftError> {
        Err(EftError::Config("driver not implemented".into()))
    }

    pub async fn status(&self, _reference: &str) -> Result<Outcome, EftError> {
        Err(EftError::Config("driver not implemented".into()))
    }

    pub async fn abort(&self, _reference: &str) -> Result<(), EftError> {
        Err(EftError::Config("driver not implemented".into()))
    }
}
