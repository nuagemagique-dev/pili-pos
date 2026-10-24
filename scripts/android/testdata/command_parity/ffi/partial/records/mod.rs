#[derive(Debug, Clone, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thing {
    pub id: i64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, uniffi::Enum, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    One,
    Two { n: i32 },
}

#[derive(Debug, uniffi::Error, thiserror::Error)]
pub enum PosError {
    #[error("failed")]
    Failed,
}
