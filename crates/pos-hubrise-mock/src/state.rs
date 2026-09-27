//! In-memory state for the HubRise mock, and the rules the real API
//! applies: who is told about an event, which status changes are
//! allowed, how fast a catalog may be read.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use pos_hubrise_types::{CallbackEvent, CallbackRegistration, Catalog, CatalogData};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::builders::{order_total_cents, payments_total_cents, seed_catalog};

/// The POS client's token in the seeded mock.
pub const POS_TOKEN: &str = "dev-token-pos";
/// The webshop client's token (the ordering side).
pub const WEBSHOP_TOKEN: &str = "dev-token-webshop";

/// Global mock state. Wrapped in `Arc` for sharing across axum handlers.
pub struct MockState {
    pub inner: Mutex<Inner>,
}

pub struct Inner {
    /// Clients indexed by access token.
    pub clients: HashMap<String, ClientState>,
    /// OAuth client ids and the token their code exchanges for.
    pub oauth_clients: HashMap<String, String>,
    /// All orders, as JSON, keyed by HubRise order id.
    pub orders: HashMap<String, Value>,
    /// Order ids in creation order (listing is newest first).
    pub order_ids: Vec<String>,
    /// All catalogs keyed by catalog id.
    pub catalogs: HashMap<String, Catalog>,
    /// Stock levels served by `GET /location/inventory`.
    pub inventory: Vec<Value>,
    /// Authorization codes (one-shot) waiting to be exchanged for tokens.
    pub pending_auth_codes: HashMap<String, String>,
    /// Default location id served by the mock.
    pub default_location_id: String,
    /// Default catalog id served by the mock.
    pub default_catalog_id: String,
    /// Default customer list id.
    pub default_customer_list_id: String,
    /// Default account id.
    pub default_account_id: String,
    /// Injected failures, checked in order before each API request.
    pub faults: Vec<Fault>,
    /// Every API request received, oldest first.
    pub requests: Vec<LoggedRequest>,
    /// `GET /catalogs/:id` calls in the last minute, per token.
    pub catalog_reads: HashMap<String, VecDeque<Instant>>,
    /// HubRise allows 10 catalog reads a minute per connection.
    pub catalog_reads_per_minute: usize,
    /// Source of short ids.
    pub next_id: u64,
}

pub struct ClientState {
    pub access_token: String,
    pub client_name: String,
    pub account_name: String,
    pub location_name: String,
    pub catalog_name: String,
    pub customer_list_name: String,
    /// Events waiting to be read and deleted, oldest first.
    pub events: VecDeque<CallbackEvent>,
    pub registered_callback: Option<CallbackRegistration>,
}

impl ClientState {
    fn new(token: &str, client_name: &str) -> Self {
        Self {
            access_token: token.to_string(),
            client_name: client_name.to_string(),
            account_name: "Restaurant (mock)".to_string(),
            location_name: "Liège".to_string(),
            catalog_name: "Menu principal".to_string(),
            customer_list_name: "Clients".to_string(),
            events: VecDeque::new(),
            registered_callback: None,
        }
    }

    /// Whether this client's callback asks for `resource.event`.
    fn wants(&self, resource: &str, event: &str) -> bool {
        let Some(cb) = &self.registered_callback else {
            return false;
        };
        cb.events
            .get(resource)
            .and_then(Value::as_array)
            .is_some_and(|types| types.iter().any(|t| t.as_str() == Some(event)))
    }
}

/// A failure the mock returns instead of serving a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fault {
    /// HTTP method, or `"*"` for any.
    #[serde(default = "any")]
    pub method: String,
    /// Path the request must start with (`"/v1/callback/events"`).
    pub path: String,
    /// Status to return (401, 429, 500, ...); 0 only delays.
    #[serde(default)]
    pub status: u16,
    /// How many requests to fail; `None` fails every one.
    #[serde(default)]
    pub times: Option<u32>,
    /// Delay before answering (or before the failure).
    #[serde(default)]
    pub delay_ms: u64,
}

fn any() -> String {
    "*".to_string()
}

/// One request the mock received.
#[derive(Debug, Clone, Serialize)]
pub struct LoggedRequest {
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    /// Which client sent it: `"pos"`, `"webshop"`, another client
    /// name, or `None` without a known token.
    pub client: Option<String>,
    pub body: Option<Value>,
    /// Status the mock answered.
    pub status: u16,
}

/// An error the HubRise API answers with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    pub status: u16,
    pub message: String,
}

impl ApiError {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

/// Order statuses no change can leave (HubRise's error states and
/// `completed`).
const FINAL_STATUSES: &[&str] = &["completed", "rejected", "cancelled", "delivery_failed"];

const STATUSES: &[&str] = &[
    "new",
    "received",
    "accepted",
    "in_preparation",
    "awaiting_shipment",
    "awaiting_collection",
    "in_delivery",
    "completed",
    "rejected",
    "cancelled",
    "delivery_failed",
];

/// Whether an order may go from `from` to `to`.
pub fn transition_allowed(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    if !STATUSES.contains(&to) || FINAL_STATUSES.contains(&from) {
        return false;
    }
    // Nothing goes back to `new`.
    to != "new"
}

impl Default for MockState {
    fn default() -> Self {
        Self::new_seeded()
    }
}

impl MockState {
    /// Two clients (the POS and the webshop), one location, the seed
    /// catalog, no orders.
    pub fn new_seeded() -> Self {
        Self {
            inner: Mutex::new(Inner::seeded()),
        }
    }

    /// Back to the seeded state (`POST /mock/reset`).
    pub fn reset(&self) {
        *self.inner.lock().expect("poisoned mock lock") = Inner::seeded();
    }
}

impl Inner {
    fn seeded() -> Self {
        let mut clients = HashMap::new();
        clients.insert(
            WEBSHOP_TOKEN.to_string(),
            ClientState::new(WEBSHOP_TOKEN, "tsb-webshop"),
        );
        clients.insert(
            POS_TOKEN.to_string(),
            ClientState::new(POS_TOKEN, "pili-pos"),
        );
        let mut oauth_clients = HashMap::new();
        for id in ["pili-pos", "pili-pos-dev"] {
            oauth_clients.insert(id.to_string(), POS_TOKEN.to_string());
        }
        oauth_clients.insert("tsb-webshop".to_string(), WEBSHOP_TOKEN.to_string());

        let default_catalog_id = "pili-dev-cat-1".to_string();
        let default_location_id = "pili-dev-loc-1".to_string();
        let mut catalogs = HashMap::new();
        catalogs.insert(
            default_catalog_id.clone(),
            Catalog {
                id: default_catalog_id.clone(),
                location_id: Some(default_location_id.clone()),
                name: "Menu principal".to_string(),
                created_at: Some(Utc::now().to_rfc3339()),
                data: Some(seed_catalog()),
            },
        );

        Self {
            clients,
            oauth_clients,
            orders: HashMap::new(),
            order_ids: Vec::new(),
            catalogs,
            inventory: Vec::new(),
            pending_auth_codes: HashMap::new(),
            default_location_id,
            default_catalog_id,
            default_customer_list_id: "pili-dev-cl-1".to_string(),
            default_account_id: "pili-dev-acc-1".to_string(),
            faults: Vec::new(),
            requests: Vec::new(),
            catalog_reads: HashMap::new(),
            catalog_reads_per_minute: 10,
            next_id: 1,
        }
    }

    /// A fresh short id with `prefix`.
    pub fn new_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}{:05}", self.next_id);
        self.next_id += 1;
        id
    }

    /// The name a request log shows for `token`.
    pub fn client_label(&self, token: &str) -> Option<String> {
        match token {
            POS_TOKEN => Some("pos".to_string()),
            WEBSHOP_TOKEN => Some("webshop".to_string()),
            other => self.clients.get(other).map(|c| c.client_name.clone()),
        }
    }

    /// Queue `event` for every client whose callback asks for it,
    /// except `source`: HubRise never tells a client about its own
    /// changes.
    pub fn emit(&mut self, source: &str, mut event: CallbackEvent) {
        event.id = self.new_id("ev");
        for (token, client) in self.clients.iter_mut() {
            if token != source && client.wants(&event.resource_type, &event.event_type) {
                client.events.push_back(event.clone());
            }
        }
    }

    /// Create an order as client `source`. Returns the stored order.
    pub fn create_order(
        &mut self,
        source: &str,
        location_id: &str,
        mut body: Value,
    ) -> Result<Value, ApiError> {
        if !body.is_object() {
            return Err(ApiError::new(422, "order body must be an object"));
        }
        validate_order_money(&body)?;
        let status = body
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("new")
            .to_string();
        if !STATUSES.contains(&status.as_str()) {
            return Err(ApiError::new(422, format!("unknown status {status}")));
        }
        if let Some(private_ref) = body.get("private_ref").and_then(Value::as_str)
            && self.orders.values().any(|o| {
                o["location_id"] == location_id && o["private_ref"].as_str() == Some(private_ref)
            })
        {
            return Err(ApiError::new(422, "private_ref has already been taken"));
        }
        let created_by = self
            .clients
            .get(source)
            .map(|c| c.client_name.clone())
            .unwrap_or_default();
        let id = self.new_id("o");
        body["id"] = json!(id);
        body["location_id"] = json!(location_id);
        body["status"] = json!(status);
        body["created_at"] = json!(Utc::now().to_rfc3339());
        body["created_by"] = json!(created_by.clone());
        if body.get("channel").and_then(Value::as_str).is_none() {
            body["channel"] = json!(created_by);
        }
        for key in ["items", "charges", "discounts", "payments"] {
            if let Some(list) = body.get_mut(key).and_then(Value::as_array_mut) {
                for (i, element) in list.iter_mut().enumerate() {
                    element["id"] = json!(format!("{id}-{key}-{i}"));
                    if element.get("deleted").is_none() {
                        element["deleted"] = json!(false);
                    }
                }
            }
        }
        refresh_totals(&mut body);
        self.orders.insert(id.clone(), body.clone());
        self.order_ids.push(id.clone());
        let event = order_event(&body, "create", None);
        self.emit(source, event);
        Ok(body)
    }

    /// Apply a PATCH from client `source` (`docs/hubrise/03-orders.md`
    /// "Update Order Rules").
    pub fn update_order(
        &mut self,
        source: &str,
        order_id: &str,
        patch: &Value,
    ) -> Result<Value, ApiError> {
        let Some(order) = self.orders.get(order_id) else {
            return Err(ApiError::new(404, "order not found"));
        };
        let previous = order.clone();
        let mut order = order.clone();
        if let Some(to) = patch.get("status").and_then(Value::as_str) {
            let from = order["status"].as_str().unwrap_or("new");
            if !transition_allowed(from, to) {
                return Err(ApiError::new(
                    422,
                    format!("status cannot change from {from} to {to}"),
                ));
            }
            order["status"] = json!(to);
        }
        for key in [
            "confirmed_time",
            "driver_pickup_url",
            "seller_notes",
            "collection_code",
            "private_ref",
            "custom_fields",
            "expected_time",
        ] {
            if let Some(v) = patch.get(key) {
                order[key] = v.clone();
            }
        }
        for key in ["items", "charges", "discounts", "payments"] {
            let Some(ops) = patch.get(key).and_then(Value::as_array) else {
                continue;
            };
            validate_order_money(&json!({ key: ops }))?;
            for op in ops {
                let list = order[key].as_array_mut();
                let Some(list) = list else {
                    order[key] = json!([]);
                    continue;
                };
                match op.get("id").and_then(Value::as_str) {
                    Some(element_id) => {
                        let Some(element) = list.iter_mut().find(|e| e["id"] == element_id) else {
                            return Err(ApiError::new(
                                422,
                                format!("{key} {element_id} not found"),
                            ));
                        };
                        if op.get("deleted").and_then(Value::as_bool) == Some(true) {
                            element["deleted"] = json!(true);
                        }
                        if let Some(r) = op.get("private_ref") {
                            element["private_ref"] = r.clone();
                        }
                    }
                    None => {
                        let mut added = op.clone();
                        added["id"] = json!(format!("{order_id}-{key}-{}", list.len()));
                        added["deleted"] = json!(false);
                        list.push(added);
                    }
                }
            }
        }
        refresh_totals(&mut order);
        self.orders.insert(order_id.to_string(), order.clone());
        let event = order_event(&order, "update", Some(previous));
        self.emit(source, event);
        Ok(order)
    }

    /// Replace a catalog's content (`PUT /catalogs/:id`) as `source`.
    pub fn replace_catalog(
        &mut self,
        source: &str,
        catalog_id: &str,
        name: Option<String>,
        data: CatalogData,
    ) -> Catalog {
        let location = self.default_location_id.clone();
        let existed = self.catalogs.contains_key(catalog_id);
        let catalog = self
            .catalogs
            .entry(catalog_id.to_string())
            .or_insert_with(|| Catalog {
                id: catalog_id.to_string(),
                location_id: Some(location.clone()),
                name: "Menu principal".to_string(),
                created_at: Some(Utc::now().to_rfc3339()),
                data: None,
            });
        if let Some(name) = name {
            catalog.name = name;
        }
        catalog.data = Some(data);
        let catalog = catalog.clone();
        let kind = if existed { "update" } else { "create" };
        let event = catalog_event(&catalog, kind);
        self.emit(source, event);
        catalog
    }

    /// Count a catalog read by `token`; `Err` past the per-minute limit.
    pub fn read_catalog_allowed(&mut self, token: &str) -> Result<(), ApiError> {
        let limit = self.catalog_reads_per_minute;
        let reads = self.catalog_reads.entry(token.to_string()).or_default();
        let now = Instant::now();
        while reads
            .front()
            .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(60))
        {
            reads.pop_front();
        }
        if reads.len() >= limit {
            return Err(ApiError::new(429, "Too many requests"));
        }
        reads.push_back(now);
        Ok(())
    }
}

/// Recompute `total` and `payment_discrepancy` as HubRise does.
fn refresh_totals(order: &mut Value) {
    let total = order_total_cents(order);
    let paid = payments_total_cents(order);
    order["total"] = json!(pos_hubrise_types::money::format_cents(total));
    order["payment_discrepancy"] = json!(pos_hubrise_types::money::format_cents(paid - total));
}

/// Refuse amounts that are not `<amount> EUR` (HubRise answers 422).
fn validate_order_money(order: &Value) -> Result<(), ApiError> {
    let check = |v: Option<&Value>, what: &str| -> Result<(), ApiError> {
        match v {
            None | Some(Value::Null) => Ok(()),
            Some(Value::String(s)) if pos_hubrise_types::money::parse_cents(s).is_ok() => Ok(()),
            Some(other) => Err(ApiError::new(
                422,
                format!("{other} is not a valid monetary amount ({what})"),
            )),
        }
    };
    let list = |key: &str| {
        order
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for item in list("items") {
        if item.get("id").is_none() || item.get("product_name").is_some() {
            check(item.get("price"), "item price")?;
        }
        for opt in item
            .get("options")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            check(opt.get("price"), "option price")?;
        }
    }
    for c in list("charges") {
        check(c.get("price"), "charge price")?;
    }
    for d in list("discounts") {
        check(d.get("price_off"), "discount price_off")?;
    }
    for p in list("payments") {
        check(p.get("amount"), "payment amount")?;
    }
    Ok(())
}

fn order_event(order: &Value, kind: &str, previous: Option<Value>) -> CallbackEvent {
    let mut extra = serde_json::Map::new();
    extra.insert("order_id".to_string(), order["id"].clone());
    extra.insert("location_id".to_string(), order["location_id"].clone());
    CallbackEvent {
        id: String::new(),
        resource_type: "order".to_string(),
        event_type: kind.to_string(),
        created_at: Utc::now().to_rfc3339(),
        previous_state: previous,
        new_state: Some(order.clone()),
        extra,
    }
}

fn catalog_event(catalog: &Catalog, kind: &str) -> CallbackEvent {
    let mut extra = serde_json::Map::new();
    extra.insert("catalog_id".to_string(), json!(catalog.id));
    extra.insert("location_id".to_string(), json!(catalog.location_id));
    // Catalog events carry no state fields (docs/hubrise/06-callbacks.md).
    CallbackEvent {
        id: String::new(),
        resource_type: "catalog".to_string(),
        event_type: kind.to_string(),
        created_at: Utc::now().to_rfc3339(),
        previous_state: None,
        new_state: None,
        extra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_follow_hubrise() {
        assert!(transition_allowed("new", "received"));
        assert!(transition_allowed("received", "accepted"));
        assert!(transition_allowed("accepted", "cancelled"));
        assert!(transition_allowed("new", "rejected"));
        assert!(!transition_allowed("completed", "accepted"));
        assert!(!transition_allowed("cancelled", "accepted"));
        assert!(!transition_allowed("accepted", "new"));
        assert!(!transition_allowed("accepted", "bogus"));
        assert!(transition_allowed("completed", "completed"));
    }

    fn register(inner: &mut Inner, token: &str, events: Value) {
        inner.clients.get_mut(token).unwrap().registered_callback =
            Some(CallbackRegistration { url: None, events });
    }

    #[test]
    fn only_registered_clients_other_than_the_source_get_events() {
        let mut inner = Inner::seeded();
        let body = crate::builders::template("webshop_online").unwrap().build();
        let loc = inner.default_location_id.clone();
        inner
            .create_order(WEBSHOP_TOKEN, &loc, body.clone())
            .unwrap();
        assert!(
            inner.clients[POS_TOKEN].events.is_empty(),
            "no callback yet"
        );

        register(&mut inner, POS_TOKEN, json!({ "order": ["create"] }));
        register(
            &mut inner,
            WEBSHOP_TOKEN,
            json!({ "order": ["create", "update"] }),
        );
        let order = inner.create_order(WEBSHOP_TOKEN, &loc, body).unwrap();
        assert_eq!(inner.clients[POS_TOKEN].events.len(), 1);
        assert!(
            inner.clients[WEBSHOP_TOKEN].events.is_empty(),
            "never your own"
        );

        // The POS registered for create only: an update does not reach it.
        let id = order["id"].as_str().unwrap().to_string();
        inner
            .update_order(WEBSHOP_TOKEN, &id, &json!({ "status": "cancelled" }))
            .unwrap();
        assert_eq!(inner.clients[POS_TOKEN].events.len(), 1);
    }

    #[test]
    fn the_total_and_discrepancy_are_computed() {
        let mut inner = Inner::seeded();
        let loc = inner.default_location_id.clone();
        let body = crate::builders::template("webshop_cash_collection")
            .unwrap()
            .build();
        let order = inner.create_order(WEBSHOP_TOKEN, &loc, body).unwrap();
        assert_eq!(order["total"], "14.90 EUR");
        assert_eq!(order["payment_discrepancy"], "-14.90 EUR");
        assert_eq!(order["created_by"], "tsb-webshop");
        assert!(order["items"][0]["id"].is_string());
    }

    #[test]
    fn a_bad_amount_or_a_taken_private_ref_is_refused() {
        let mut inner = Inner::seeded();
        let loc = inner.default_location_id.clone();
        let bad = json!({ "items": [{ "product_name": "X", "price": "abc", "quantity": "1" }] });
        assert_eq!(
            inner.create_order(POS_TOKEN, &loc, bad).unwrap_err().status,
            422
        );
        let one = json!({ "status": "completed", "private_ref": "sale-1" });
        inner.create_order(POS_TOKEN, &loc, one.clone()).unwrap();
        assert_eq!(
            inner.create_order(POS_TOKEN, &loc, one).unwrap_err().status,
            422
        );
    }

    #[test]
    fn catalog_reads_are_limited_per_minute() {
        let mut inner = Inner::seeded();
        inner.catalog_reads_per_minute = 2;
        assert!(inner.read_catalog_allowed(POS_TOKEN).is_ok());
        assert!(inner.read_catalog_allowed(POS_TOKEN).is_ok());
        assert_eq!(
            inner.read_catalog_allowed(POS_TOKEN).unwrap_err().status,
            429
        );
        assert!(
            inner.read_catalog_allowed(WEBSHOP_TOKEN).is_ok(),
            "per connection"
        );
    }
}
