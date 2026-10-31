//! Records of the checkout scale (PD56): one type per engine type, same field names, same serde
//! attributes. `ScaleProgress` is an event payload, reached from `PosEvent`, so it lives with the
//! records of this domain and is imported by `events.rs`.

/// `pos_engine::commands::scale::ScaleWeighInput`: what to weigh, on which order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct ScaleWeighInput {
    pub order_id: i64,
    pub menu_item_id: i64,
}

/// `pos_engine::commands::scale::ScaleStatus`: how the scale is doing.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct ScaleStatus {
    pub configured: bool,
    pub connected: bool,
    pub busy: bool,
    /// The last verdict of its software check: `ok`, `refused`, `no_values`, or `unknown`.
    pub check: String,
}

/// `pos_engine::commands::scale::ScaleProgress`: `waiting`, `stabilizing` or `checking`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, uniffi::Record)]
#[serde(deny_unknown_fields)]
pub struct ScaleProgress {
    pub stage: String,
}
