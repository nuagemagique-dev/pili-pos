use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::vat::VatCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    DineIn,
    Takeaway,
}

impl OrderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DineIn => "dine_in",
            Self::Takeaway => "takeaway",
        }
    }
}

impl std::str::FromStr for OrderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dine_in" => Ok(Self::DineIn),
            "takeaway" => Ok(Self::Takeaway),
            other => Err(format!("invalid order type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    Closed,
    Voided,
}

impl OrderStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Voided => "voided",
        }
    }
}

impl std::str::FromStr for OrderStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "voided" => Ok(Self::Voided),
            other => Err(format!("invalid order status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i64,
    pub table_number: Option<i32>,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub user_insz: String,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub subtotal_cents: i64,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderLine {
    pub id: i64,
    pub order_id: i64,
    pub menu_item_id: i64,
    pub quantity: i32,
    pub unit_price_cents: i64,
    pub vat_code: VatCode,
    pub name: String,
    pub modifiers_json: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl OrderLine {
    pub fn line_total(&self) -> i64 {
        self.unit_price_cents * self.quantity as i64
    }
}
