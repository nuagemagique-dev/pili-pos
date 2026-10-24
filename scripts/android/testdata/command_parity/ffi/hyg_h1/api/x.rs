#[uniffi::export]
impl PosEngine {
    pub fn blocks_ui(&self) -> i64 {
        1
    }
}

#[uniffi::export]
pub fn free_sync() {}
