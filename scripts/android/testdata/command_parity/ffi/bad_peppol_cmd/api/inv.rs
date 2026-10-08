#[uniffi::export]
impl PosEngine {
    pub async fn sign_thing(&self, order_id: i64) -> Result<(), PosError> {
        self.spawn(move |e| pos_engine::commands::inv::do_sign_thing(&e, order_id)).await
    }
}
