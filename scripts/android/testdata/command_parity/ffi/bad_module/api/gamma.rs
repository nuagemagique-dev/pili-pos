#[uniffi::export]
impl PosEngine {
    pub async fn check_gamma(
        &self,
        order_id: i64,
    ) -> Result<bool, PosError> {
        self.spawn(move |e| async move {
            let r = { pos_engine::commands::delta::do_check_gamma(&e, order_id).await };
            r
        })
        .await
    }
}
