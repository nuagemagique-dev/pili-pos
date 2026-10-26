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
    /// Department sent to the FDM (`departmentId` / `departmentName`):
    /// the product's category, captured when the line is added.
    #[serde(default)]
    pub department_id: String,
    #[serde(default)]
    pub department_name: String,
}

impl OrderLine {
    /// Unit price times quantity; saturates instead of overflowing on an
    /// impossible quantity.
    #[must_use]
    pub fn line_total(&self) -> i64 {
        self.unit_price_cents
            .saturating_mul(i64::from(self.quantity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_total_saturates_instead_of_overflowing() {
        assert_eq!(line(i64::MAX, 2).line_total(), i64::MAX);
        assert_eq!(line(i64::MAX, -2).line_total(), i64::MIN);
        assert_eq!(line(250, 3).line_total(), 750);
    }

    fn line(unit_price_cents: i64, quantity: i32) -> OrderLine {
        OrderLine {
            id: 1,
            order_id: 1,
            menu_item_id: 1,
            quantity,
            unit_price_cents,
            vat_code: VatCode::B,
            name: "Test".to_string(),
            modifiers_json: None,
            created_at: chrono::Utc::now(),
            department_id: "1".to_string(),
            department_name: "Test".to_string(),
        }
    }

    #[test]
    fn line_total_multiplies_price_by_quantity() {
        assert_eq!(line(1250, 3).line_total(), 3750);
    }

    #[test]
    fn line_total_zero_quantity_is_zero() {
        assert_eq!(line(1250, 0).line_total(), 0);
    }

    #[test]
    fn line_total_negative_quantity_is_a_correction_or_refund() {
        assert_eq!(line(1250, -2).line_total(), -2500);
    }

    #[test]
    fn line_total_large_values_do_not_overflow() {
        // 100,000.00 EUR unit price x 1000 units - still far under i64::MAX.
        let unit_price_cents = 10_000_000; // 100,000.00 EUR
        let expected_total_cents = 10_000_000_000; // 100,000,000.00 EUR
        assert_eq!(
            line(unit_price_cents, 1000).line_total(),
            expected_total_cents
        );
    }

    #[test]
    fn order_type_as_str_and_from_str_round_trip() {
        for t in [OrderType::DineIn, OrderType::Takeaway] {
            assert_eq!(t.as_str().parse::<OrderType>().unwrap(), t);
        }
    }

    #[test]
    fn order_type_from_str_rejects_unknown() {
        assert!("delivery".parse::<OrderType>().is_err());
        assert!("".parse::<OrderType>().is_err());
    }

    #[test]
    fn order_status_as_str_and_from_str_round_trip() {
        for s in [OrderStatus::Open, OrderStatus::Closed, OrderStatus::Voided] {
            assert_eq!(s.as_str().parse::<OrderStatus>().unwrap(), s);
        }
    }

    #[test]
    fn order_status_from_str_rejects_unknown() {
        assert!("pending".parse::<OrderStatus>().is_err());
    }

    #[test]
    fn order_line_serde_round_trips_through_json() {
        let l = line(1250, 2);
        let json = serde_json::to_string(&l).unwrap();
        let back: OrderLine = serde_json::from_str(&json).unwrap();
        assert_eq!(back.unit_price_cents, l.unit_price_cents);
        assert_eq!(back.quantity, l.quantity);
        assert_eq!(back.vat_code, l.vat_code);
    }

    #[test]
    fn order_line_missing_department_fields_default_to_empty() {
        // Rows archived before department_id/department_name existed
        // must still deserialize (they are #[serde(default)]).
        let json = r#"{
            "id": 1, "order_id": 1, "menu_item_id": 1, "quantity": 1,
            "unit_price_cents": 1000, "vat_code": "B", "name": "Old row",
            "modifiers_json": null, "created_at": "2026-01-01T00:00:00Z"
        }"#;
        let l: OrderLine = serde_json::from_str(json).unwrap();
        assert_eq!(l.department_id, "");
        assert_eq!(l.department_name, "");
    }
}
