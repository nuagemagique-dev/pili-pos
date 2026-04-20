use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub number: i32,
    pub label: String,
    pub seats: i32,
    pub active: bool,
}
