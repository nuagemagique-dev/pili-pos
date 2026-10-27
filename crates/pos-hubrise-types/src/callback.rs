//! `HubRise` callback (webhook) events - polling mode.
//!
//! Reference: `docs/hubrise/06-callbacks.md`.

use serde::{Deserialize, Serialize};

/// Type of `HubRise` resource emitting the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventResource {
    Catalog,
    Customer,
    CustomerList,
    Delivery,
    Inventory,
    Location,
    Order,
}

impl EventResource {
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Catalog => "catalog",
            Self::Customer => "customer",
            Self::CustomerList => "customer_list",
            Self::Delivery => "delivery",
            Self::Inventory => "inventory",
            Self::Location => "location",
            Self::Order => "order",
        }
    }
}

/// Generic callback event envelope - `HubRise` uses the same shape for
/// all resource types, with the state fields carried as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackEvent {
    pub id: String,
    pub resource_type: String,
    pub event_type: String,
    pub created_at: String,
    #[serde(default)]
    pub previous_state: Option<serde_json::Value>,
    #[serde(default)]
    pub new_state: Option<serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl CallbackEvent {
    pub fn is_order_create(&self) -> bool {
        self.resource_type == "order" && self.event_type == "create"
    }
    pub fn is_order_update(&self) -> bool {
        self.resource_type == "order" && self.event_type == "update"
    }
    pub fn is_catalog_update(&self) -> bool {
        self.resource_type == "catalog" && self.event_type == "update"
    }
}

/// Callback registration payload for `POST /callback`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackRegistration {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub events: serde_json::Value,
}

/// Strongly typed shape of an `order.create` event (parsed on demand).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderCreateEvent {
    pub order_id: String,
    #[serde(default)]
    pub location_id: Option<String>,
    pub new_state: crate::order::Order,
}

/// Strongly typed shape of an `order.update` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderUpdateEvent {
    pub order_id: String,
    #[serde(default)]
    pub location_id: Option<String>,
    pub previous_state: serde_json::Value,
    pub new_state: serde_json::Value,
}

/// Strongly typed shape of a `catalog.update` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogUpdateEvent {
    pub catalog_id: String,
    #[serde(default)]
    pub location_id: Option<String>,
}
