#[uniffi::export]
impl PosEngine {
    pub async fn sign_invoice_fiscal(&self) -> Result<(), PosError> {
        Ok(())
    }
}

#[cfg(feature = "peppol")]
pub fn gated() {}
