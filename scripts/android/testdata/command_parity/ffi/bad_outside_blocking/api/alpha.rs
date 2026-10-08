use pos_engine::commands::alpha;

#[uniffi::export]
impl PosEngine {
    /// get_alpha: a sync engine fn run on the blocking pool.
    pub async fn get_alpha(&self) -> Result<String, PosError> {
        let _ = self.blocking(move |_e| Ok(())).await;
        pos_engine::commands::alpha::do_get_alpha(&self.state)
    }

    pub async fn set_beta(&self, item_id: i64, label: String) -> Result<(), PosError> {
        // multi-line call through a `use`d module path
        self.blocking(move |e| {
            alpha::do_set_beta(
                e,
                item_id,
                label,
            )
        })
        .await
    }
}
