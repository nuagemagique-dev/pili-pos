//! HubRise Order resource.
//!
//! Reference: `docs/hubrise/03-orders.md`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::common::{Money, ServiceType, null_default};

/// HubRise order status (see `docs/hubrise/03-orders.md` - Status Enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    New,
    Received,
    Accepted,
    InPreparation,
    AwaitingCollection,
    InDelivery,
    Completed,
    Rejected,
    Cancelled,
    DeliveryFailed,
    /// Deprecated alias for `AwaitingCollection` - kept for compatibility.
    AwaitingShipment,
}

impl OrderStatus {
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Received => "received",
            Self::Accepted => "accepted",
            Self::InPreparation => "in_preparation",
            Self::AwaitingCollection => "awaiting_collection",
            Self::InDelivery => "in_delivery",
            Self::Completed => "completed",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::DeliveryFailed => "delivery_failed",
            Self::AwaitingShipment => "awaiting_shipment",
        }
    }
}

/// Full HubRise order response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub location_id: String,
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
    #[serde(default)]
    pub private_ref: Option<String>,
    pub status: OrderStatus,
    #[serde(default)]
    pub service_type: Option<ServiceType>,
    #[serde(default)]
    pub service_type_ref: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub connection_name: Option<String>,
    #[serde(default)]
    pub expected_time: Option<String>,
    #[serde(default)]
    pub expected_time_pickup: Option<bool>,
    #[serde(default)]
    pub asap: Option<bool>,
    #[serde(default)]
    pub confirmed_time: Option<String>,
    #[serde(default)]
    pub driver_pickup_url: Option<String>,
    #[serde(default)]
    pub customer_notes: Option<String>,
    #[serde(default)]
    pub seller_notes: Option<String>,
    #[serde(default)]
    pub collection_code: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub coupon_codes: Vec<String>,
    #[serde(default)]
    pub total: Option<Money>,
    #[serde(default)]
    pub payment_discrepancy: Option<Money>,
    #[serde(default, deserialize_with = "null_default")]
    pub items: Vec<OrderItem>,
    #[serde(default, deserialize_with = "null_default")]
    pub deals: HashMap<String, OrderDealEntry>,
    #[serde(default, deserialize_with = "null_default")]
    pub discounts: Vec<OrderDiscount>,
    #[serde(default, deserialize_with = "null_default")]
    pub charges: Vec<OrderCharge>,
    #[serde(default, deserialize_with = "null_default")]
    pub payments: Vec<OrderPayment>,
    #[serde(default)]
    pub customer: Option<Customer>,
    /// Delivery tracking (carrier, driver), when a platform delivers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<serde_json::Value>,
    #[serde(default)]
    pub custom_fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub private_ref: Option<String>,
    pub product_name: String,
    #[serde(default)]
    pub sku_name: Option<String>,
    #[serde(default)]
    pub sku_ref: Option<String>,
    pub price: Money,
    pub quantity: String,
    #[serde(default)]
    pub subtotal: Option<Money>,
    #[serde(default)]
    pub tax_rate: Option<String>,
    #[serde(default)]
    pub subset: Option<String>,
    #[serde(default)]
    pub customer_notes: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub options: Vec<OrderOption>,
    /// The deal this item is a line of (`deal_key` into `Order.deals`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deal_line: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "null_default")]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderOption {
    pub option_list_name: String,
    pub name: String,
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
    #[serde(default)]
    pub price: Option<Money>,
    #[serde(
        default = "default_option_quantity",
        deserialize_with = "option_quantity"
    )]
    pub quantity: u32,
    #[serde(default, deserialize_with = "null_default")]
    pub removed: bool,
}

fn default_option_quantity() -> u32 {
    1
}

/// An option's quantity: `null` counts as the default 1.
fn option_quantity<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    Ok(Option::<u32>::deserialize(d)?.unwrap_or(1))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDealEntry {
    pub name: String,
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderDiscount {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub private_ref: Option<String>,
    pub name: String,
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
    pub price_off: Money,
    #[serde(default, deserialize_with = "null_default")]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCharge {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub private_ref: Option<String>,
    pub name: String,
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
    pub price: Money,
    #[serde(default)]
    pub tax_rate: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderPayment {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub private_ref: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "ref")]
    pub ref_: Option<String>,
    pub amount: Money,
    #[serde(default)]
    pub info: Option<serde_json::Value>,
    #[serde(default, deserialize_with = "null_default")]
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub customer_list_id: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    pub anonymised: bool,
    #[serde(default)]
    pub private_ref: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub phone_access_code: Option<String>,
    #[serde(default)]
    pub address_1: Option<String>,
    #[serde(default)]
    pub address_2: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub latitude: Option<String>,
    #[serde(default)]
    pub longitude: Option<String>,
    #[serde(default)]
    pub delivery_notes: Option<String>,
    #[serde(default)]
    pub sms_marketing: Option<bool>,
    #[serde(default)]
    pub email_marketing: Option<bool>,
    #[serde(default)]
    pub custom_fields: Option<serde_json::Value>,
}
