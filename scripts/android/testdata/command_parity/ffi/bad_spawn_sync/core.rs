use std::sync::Arc;

#[derive(uniffi::Object)]
pub struct PosEngine {
    inner: u64,
}

#[uniffi::export]
impl PosEngine {
    #[uniffi::constructor]
    pub fn open(path: String) -> Result<Arc<Self>, PosError> {
        Ok(Arc::new(Self { inner: 0 }))
    }

    pub async fn shutdown(&self) {}

    pub async fn events(&self) -> Vec<String> {
        Vec::new()
    }
}
