//! Driver stub: to be implemented (see the crate documentation).

use pos_core::config::PaymentTerminalConfig;

use crate::{EftError, Outcome, PaymentRequest};

pub struct CcvOpi {
    _config: PaymentTerminalConfig,
}

impl CcvOpi {
    #[allow(unused_variables)]
    pub fn new(config: &PaymentTerminalConfig, software: &str) -> Result<Self, EftError> {
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
