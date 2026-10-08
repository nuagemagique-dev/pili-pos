const NAME: &'static str = "ok";

// contract: custom
#[derive(Debug, uniffi::Record, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Free {
    pub id: i64,
}

/// doc comment mentioning peppol and invoice is fine
fn private(s: &'static str) -> &'static str { s }
