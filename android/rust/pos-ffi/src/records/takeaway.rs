//! Records of the numbered takeaway (engine module `takeaway`): the numbering and the tag of a
//! takeaway order. Mirror the engine types (PD56): same field names, same serde forms,
//! `deny_unknown_fields` where the engine has it.

/// `pos_engine::commands::takeaway::TakeawayNumbering`: the first takeaway number of each day.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeawayNumbering {
    /// Between 1 and 999; the default is 61.
    pub first_number: i32,
}

/// `pos_engine::commands::takeaway::TakeawayTag`: the number of the day and the customer of a
/// takeaway order (local data, never fiscal).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TakeawayTag {
    pub order_id: i64,
    /// The business day the number belongs to (`YYYY-MM-DD`).
    pub day: String,
    pub number: i64,
    /// `E-61`.
    pub label: String,
    pub customer_name: Option<String>,
    pub customer_phone: Option<String>,
}
