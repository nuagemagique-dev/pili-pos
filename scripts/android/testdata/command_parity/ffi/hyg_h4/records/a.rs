#[derive(uniffi::Record, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Same {
    pub id: i64,
}
