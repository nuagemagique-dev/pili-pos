//! Outbound updates to `HubRise` - used when staff acts on a webshop
//! order from the POS (accept / `in_preparation` / completed / delay).

use pos_hubrise_types::Order;
use serde_json::{Value, json};

use crate::client::Client;
use crate::errors::Result;
use crate::util::path_segment;

/// Update the status of an order on `HubRise`. Returns the refreshed
/// order body returned by the API.
pub async fn patch_order_status(
    client: &Client,
    location_id: &str,
    order_id: &str,
    new_status: &str,
) -> Result<Order> {
    let path = format!(
        "/locations/{}/orders/{}",
        path_segment(location_id),
        path_segment(order_id)
    );
    let body = json!({ "status": new_status });
    client.patch_json(&path, &body).await
}

/// Create a new order on `HubRise`. Used by the POS to push completed
/// dine-in / takeaway orders so they appear in the centralized
/// reporting (tsb-dashboard). The body is a pre-built `serde_json::Value`
/// because the mapping from local `pos_core::Order` to `HubRise` wire
/// shape lives in the Tauri command layer (avoids a circular dep
/// between pos-hubrise and pos-core).
pub async fn create_order(client: &Client, location_id: &str, body: &Value) -> Result<Value> {
    let path = format!("/locations/{}/orders", path_segment(location_id));
    client.post_json(&path, body).await
}

/// Push a new `confirmed_time` (RFC3339, UTC) on a `HubRise` order.
/// Used by the POS "+15/+30/+45/+60 min" delay chips so the POS
/// staff and the platform agree on the new ready-at time. Returns
/// the refreshed order body.
pub async fn patch_order_confirmed_time(
    client: &Client,
    location_id: &str,
    order_id: &str,
    confirmed_time_rfc3339: &str,
) -> Result<Order> {
    let path = format!(
        "/locations/{}/orders/{}",
        path_segment(location_id),
        path_segment(order_id)
    );
    let body = json!({ "confirmed_time": confirmed_time_rfc3339 });
    client.patch_json(&path, &body).await
}
