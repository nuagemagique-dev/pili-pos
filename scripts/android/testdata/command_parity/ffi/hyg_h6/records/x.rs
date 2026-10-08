#[derive(uniffi::Record, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wide {
    pub ok: Vec<u8>,
    pub bad: u32,
    pub also_bad: Option<usize>,
}

#[derive(uniffi::Enum, serde::Serialize, serde::Deserialize)]
pub enum Wide2 {
    A(u64),
}

#[uniffi::export]
impl PosEngine {
    pub async fn takes_u16(&self, v: u16) -> Result<(), PosError> {
        Ok(())
    }
}
