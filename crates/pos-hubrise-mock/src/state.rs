//! In-memory state for the HubRise mock, and the rules the real API
//! applies: who is told about an event, which status changes are
//! allowed, how fast a catalog may be read.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use pos_hubrise_types::money::parse_stock_thousandths;
use pos_hubrise_types::{
    CallbackEvent, CallbackRegistration, Catalog, CatalogData, InventoryEntry,
};
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
    /// The token of the client that created each order - `private_ref`
    /// is a per-client lookup convenience, not a global constraint.
    pub order_owner: HashMap<String, String>,
    /// All catalogs keyed by catalog id.
    pub catalogs: HashMap<String, Catalog>,
    /// Stock levels per catalog id (unscoped routes use the default
    /// catalog).
    pub inventory: HashMap<String, Vec<InventoryEntry>>,
    /// Authorization codes (one-shot) waiting to be exchanged for tokens.
    pub pending_auth_codes: HashMap<String, String>,
    /// Tokens revoked via `POST /oauth2/v1/revoke` - unusable until
    /// `/mock/reset`.
    pub revoked_tokens: HashSet<String>,
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
    /// `GET /orders` (list) calls in the last minute, per token.
    pub order_list_reads: HashMap<String, VecDeque<Instant>>,
    /// Every request in the last minute/hour/day, per token (the
    /// connection-wide rate limit).
    pub requests_by_token: HashMap<String, VecDeque<Instant>>,
    /// HubRise allows 10 catalog reads a minute per connection. Kept
    /// as a top-level field (not in [`MockSettings`]): other crates'
    /// tests poke it directly (`mock.state.inner.lock()...`).
    pub catalog_reads_per_minute: usize,
    /// Behaviour toggles, see [`MockSettings`].
    pub settings: MockSettings,
    /// Source of short ids.
    pub next_id: u64,
}

/// Behaviour toggles for tests that need the documented, more
/// permissive HubRise behaviour instead of the mock's long-standing
/// default (see `README.md` "Toggles"). `strict_transitions` and
/// `strict_private_ref` default to `true`: the mock's original,
/// stricter policy, which every existing POS integration test
/// (`src-tauri/tests/hubrise_*.rs`) was written against. The docs
/// don't document either constraint as server-enforced (see
/// `hubrise-mock-gaps.md`), so a test that wants to exercise the
/// more permissive, docs-only behaviour can flip these to `false`
/// via `PUT /mock/settings`.
#[derive(Debug, Clone)]
pub struct MockSettings {
    /// When `true` (the default), use the mock's original transition
    /// graph (final statuses lock, nothing goes back to `new`). The
    /// docs document no transition graph at all, so setting this to
    /// `false` allows any documented status to any documented status.
    pub strict_transitions: bool,
    /// When `true`, refuse a `private_ref` already used at this
    /// location, across every client, with a 422 (the mock's old
    /// behaviour). The docs describe `private_ref` as a per-client
    /// lookup convenience, not a server-enforced constraint: the
    /// default (`false`) never refuses a duplicate.
    pub strict_private_ref: bool,
    /// Connection-wide rate limit: requests allowed per minute.
    pub rate_limit_per_minute: usize,
    /// Connection-wide rate limit: requests allowed per hour.
    pub rate_limit_per_hour: usize,
    /// Connection-wide rate limit: requests allowed per day.
    pub rate_limit_per_day: usize,
    /// `GET .../orders` (list) calls allowed per minute per connection.
    pub order_list_reads_per_minute: usize,
}

impl Default for MockSettings {
    fn default() -> Self {
        Self {
            strict_transitions: true,
            strict_private_ref: false,
            // Documented connection-wide limits (`01-general.md`).
            rate_limit_per_minute: 500,
            rate_limit_per_hour: 2500,
            rate_limit_per_day: 10_000,
            order_list_reads_per_minute: 10,
        }
    }
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
    /// Per-field details, HubRise's `errors: [{field, message}]`.
    pub fields: Vec<(String, String)>,
}

impl ApiError {
    pub fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            fields: Vec::new(),
        }
    }

    /// A 422 with one field-level detail.
    pub fn field(field: impl Into<String>, message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            status: 422,
            fields: vec![(field.into(), message.clone())],
            message,
        }
    }

    /// A 422 with several field-level details.
    pub fn fields(fields: Vec<(String, String)>) -> Self {
        let message = fields
            .first()
            .map(|(f, m)| format!("{f}: {m}"))
            .unwrap_or_else(|| "invalid".to_string());
        Self {
            status: 422,
            message,
            fields,
        }
    }
}

/// Order statuses no change can leave (the mock's own strict policy,
/// see [`MockSettings::strict_transitions`]).
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

/// Whether an order may go from `from` to `to`. The docs define no
/// transition graph and explicitly say orders don't have to go
/// through every step, so `strict = false` allows any documented
/// status to follow any other. `strict = true` (the default, see
/// [`MockSettings::strict_transitions`]) is the mock's original,
/// invented policy that every existing POS integration test is
/// written against; a test that wants the more permissive,
/// docs-only behaviour can flip it off (see `README.md` "Toggles").
pub fn transition_allowed(strict: bool, from: &str, to: &str) -> bool {
    if !STATUSES.contains(&to) {
        return false;
    }
    if !strict {
        return true;
    }
    if from == to {
        return true;
    }
    if FINAL_STATUSES.contains(&from) {
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
            order_owner: HashMap::new(),
            catalogs,
            inventory: HashMap::new(),
            pending_auth_codes: HashMap::new(),
            revoked_tokens: HashSet::new(),
            default_location_id,
            default_catalog_id,
            default_customer_list_id: "pili-dev-cl-1".to_string(),
            default_account_id: "pili-dev-acc-1".to_string(),
            faults: Vec::new(),
            requests: Vec::new(),
            catalog_reads: HashMap::new(),
            order_list_reads: HashMap::new(),
            requests_by_token: HashMap::new(),
            catalog_reads_per_minute: 10,
            settings: MockSettings::default(),
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
            return Err(ApiError::field(
                "status",
                format!("unknown status {status}"),
            ));
        }
        if self.settings.strict_private_ref
            && let Some(private_ref) = body.get("private_ref").and_then(Value::as_str)
            && self.orders.values().any(|o| {
                o["location_id"] == location_id && o["private_ref"].as_str() == Some(private_ref)
            })
        {
            return Err(ApiError::field(
                "private_ref",
                "private_ref has already been taken",
            ));
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
        self.order_owner.insert(id.clone(), source.to_string());
        let event = order_event(&body, "create", None);
        self.emit(source, event);
        Ok(body)
    }

    /// Orders at `location_id` whose `private_ref` is `private_ref`
    /// AND were created by `token` - a per-client lookup convenience,
    /// not a location-wide index (`docs/hubrise/03-orders.md`).
    pub fn orders_by_private_ref<'a>(
        &'a self,
        token: &'a str,
        location_id: &'a str,
        private_ref: &'a str,
    ) -> impl Iterator<Item = &'a Value> + 'a {
        self.order_ids.iter().rev().filter_map(move |id| {
            let order = self.orders.get(id)?;
            (order["location_id"] == location_id
                && order["private_ref"].as_str() == Some(private_ref)
                && self.order_owner.get(id).map(String::as_str) == Some(token))
            .then_some(order)
        })
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
            if !transition_allowed(self.settings.strict_transitions, from, to) {
                return Err(ApiError::field(
                    "status",
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
                            return Err(ApiError::field(
                                key,
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
    /// Does not validate `data` - callers serving the HTTP API must
    /// call [`validate_catalog`] first and refuse the whole body on
    /// error, so nothing is ever written from an invalid catalog
    /// (`crate::create_catalog`, `crate::replace_catalog`). Kept
    /// infallible here because other crates in this workspace seed
    /// catalogs directly through this function in tests.
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
        rate_check(
            &mut self.catalog_reads,
            token,
            limit,
            Duration::from_secs(60),
        )
    }

    /// Count an order-list read by `token`; `Err` past the per-minute
    /// limit (the documented extra limit on `GET .../orders`).
    pub fn read_order_list_allowed(&mut self, token: &str) -> Result<(), ApiError> {
        let limit = self.settings.order_list_reads_per_minute;
        rate_check(
            &mut self.order_list_reads,
            token,
            limit,
            Duration::from_secs(60),
        )
    }

    /// Count any API request by `token` against the connection-wide
    /// 500/min, 2,500/hour, 10,000/day limits.
    pub fn request_allowed(&mut self, token: &str) -> Result<(), ApiError> {
        let now = Instant::now();
        let history = self.requests_by_token.entry(token.to_string()).or_default();
        while history
            .front()
            .is_some_and(|t| now.duration_since(*t) > Duration::from_secs(24 * 3600))
        {
            history.pop_front();
        }
        let minute = history
            .iter()
            .filter(|t| now.duration_since(**t) <= Duration::from_secs(60))
            .count();
        let hour = history
            .iter()
            .filter(|t| now.duration_since(**t) <= Duration::from_secs(3600))
            .count();
        let day = history.len();
        if minute >= self.settings.rate_limit_per_minute
            || hour >= self.settings.rate_limit_per_hour
            || day >= self.settings.rate_limit_per_day
        {
            return Err(ApiError::new(429, "Too many requests"));
        }
        history.push_back(now);
        Ok(())
    }

    /// The inventory of `catalog_id` (or the default catalog when
    /// `catalog_id` is `None`, matching the unscoped routes).
    pub fn inventory_of(&self, catalog_id: Option<&str>) -> Vec<InventoryEntry> {
        let id = catalog_id.unwrap_or(&self.default_catalog_id);
        self.inventory.get(id).cloned().unwrap_or_default()
    }

    /// `PUT .../inventory`: replace the whole inventory of a catalog.
    /// Entries not sent are reset (removed).
    pub fn replace_inventory(
        &mut self,
        source: &str,
        catalog_id: &str,
        entries: Vec<InventoryEntry>,
    ) -> Result<Vec<InventoryEntry>, ApiError> {
        validate_inventory_entries(self, catalog_id, &entries)?;
        self.inventory
            .insert(catalog_id.to_string(), entries.clone());
        self.emit_inventory_event(source, catalog_id, &entries);
        Ok(entries)
    }

    /// `PATCH .../inventory`: merge entries in. `stock: null` removes
    /// the entry. Returns only the entries that were modified.
    pub fn patch_inventory(
        &mut self,
        source: &str,
        catalog_id: &str,
        patch: Vec<InventoryEntry>,
    ) -> Result<Vec<InventoryEntry>, ApiError> {
        validate_inventory_entries(self, catalog_id, &patch)?;
        let list = self.inventory.entry(catalog_id.to_string()).or_default();
        for entry in &patch {
            let target = entry.target_ref();
            list.retain(|e| e.target_ref() != target);
            if entry.stock.is_some() || entry.expires_at.is_some() {
                list.push(entry.clone());
            }
            // `stock: null` (both fields absent from `entry.stock`)
            // removes the entry - already handled by the retain above.
        }
        self.emit_inventory_event(source, catalog_id, &patch);
        Ok(patch)
    }

    fn emit_inventory_event(&mut self, source: &str, catalog_id: &str, entries: &[InventoryEntry]) {
        let mut extra = serde_json::Map::new();
        extra.insert("catalog_id".to_string(), json!(catalog_id));
        extra.insert("location_id".to_string(), json!(self.default_location_id));
        extra.insert("entries".to_string(), json!(entries));
        self.emit(
            source,
            CallbackEvent {
                id: String::new(),
                resource_type: "inventory".to_string(),
                event_type: "update".to_string(),
                created_at: Utc::now().to_rfc3339(),
                previous_state: None,
                new_state: None,
                extra,
            },
        );
    }

    /// `POST /mock/inventory/expire`: announce every entry with an
    /// `expires_at` in the default catalog as HubRise's documented
    /// `inventory.patch` expiry notice (one event, every expiring
    /// entry together).
    pub fn expire_inventory(&mut self) {
        let catalog_id = self.default_catalog_id.clone();
        let expiring: Vec<InventoryEntry> = self
            .inventory
            .get(&catalog_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.expires_at.is_some())
            .collect();
        if expiring.is_empty() {
            return;
        }
        let mut extra = serde_json::Map::new();
        extra.insert("catalog_id".to_string(), json!(catalog_id));
        extra.insert("location_id".to_string(), json!(self.default_location_id));
        extra.insert("entries".to_string(), json!(expiring));
        self.emit(
            // No real source: every registered client is told.
            "",
            CallbackEvent {
                id: String::new(),
                resource_type: "inventory".to_string(),
                event_type: "patch".to_string(),
                created_at: Utc::now().to_rfc3339(),
                previous_state: None,
                new_state: None,
                extra,
            },
        );
    }
}

/// A sliding one-window rate limiter shared by the catalog-read and
/// order-list-read limits.
fn rate_check(
    windows: &mut HashMap<String, VecDeque<Instant>>,
    token: &str,
    limit: usize,
    window: Duration,
) -> Result<(), ApiError> {
    let reads = windows.entry(token.to_string()).or_default();
    let now = Instant::now();
    while reads
        .front()
        .is_some_and(|t| now.duration_since(*t) > window)
    {
        reads.pop_front();
    }
    if reads.len() >= limit {
        return Err(ApiError::new(429, "Too many requests"));
    }
    reads.push_back(now);
    Ok(())
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
            Some(other) => Err(ApiError::field(
                what,
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
            check(item.get("price"), "items.price")?;
        }
        for opt in item
            .get("options")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            check(opt.get("price"), "items.options.price")?;
        }
    }
    for c in list("charges") {
        check(c.get("price"), "charges.price")?;
    }
    for d in list("discounts") {
        check(d.get("price_off"), "discounts.price_off")?;
    }
    for p in list("payments") {
        check(p.get("amount"), "payments.amount")?;
    }
    Ok(())
}

/// Validate a whole catalog body before it is written - a dangling
/// ref, an empty option list, an unenforced `min/max_selections`,
/// etc. refuses the whole body with 422 and per-field errors
/// (`docs/hubrise/04-catalogs.md`).
pub fn validate_catalog(data: &CatalogData) -> Result<(), ApiError> {
    let mut errors: Vec<(String, String)> = Vec::new();
    let seen = |errors: &mut Vec<(String, String)>,
                field: &str,
                seen: &mut HashSet<String>,
                value: &str| {
        if !seen.insert(value.to_string()) {
            errors.push((field.to_string(), format!("duplicate ref {value:?}")));
        }
    };

    let category_refs: HashSet<String> = data
        .categories
        .iter()
        .map(|c| c.ref_field.clone())
        .collect();
    {
        let mut s = HashSet::new();
        for c in &data.categories {
            seen(&mut errors, "categories.ref", &mut s, &c.ref_field);
        }
    }

    let option_list_refs: HashSet<String> = data
        .option_lists
        .iter()
        .map(|o| o.ref_field.clone())
        .collect();
    {
        let mut s = HashSet::new();
        for o in &data.option_lists {
            seen(&mut errors, "option_lists.ref", &mut s, &o.ref_field);
        }
    }

    {
        let mut s = HashSet::new();
        for (i, p) in data.products.iter().enumerate() {
            if let Some(r) = &p.ref_field {
                seen(&mut errors, "products.ref", &mut s, r);
            }
            if !p.category_ref.is_empty() && !category_refs.contains(&p.category_ref) {
                errors.push((
                    format!("products[{i}].category_ref"),
                    format!("unknown category ref {:?}", p.category_ref),
                ));
            }
            if p.skus.is_empty() {
                errors.push((
                    format!("products[{i}].skus"),
                    "a product must have at least one sku".to_string(),
                ));
            }
            if let Some(tax) = &p.tax_rate
                && !(tax.delivery.is_some() && tax.collection.is_some() && tax.eat_in.is_some())
            {
                errors.push((
                    format!("products[{i}].tax_rate"),
                    "tax_rate must set delivery, collection and eat_in together".to_string(),
                ));
            }
            let mut sku_names_seen = false;
            let mut sku_names = HashSet::new();
            for (j, sku) in p.skus.iter().enumerate() {
                if let Some(price) = Some(&sku.price)
                    && pos_hubrise_types::money::parse_cents(price).is_err()
                {
                    errors.push((
                        format!("products[{i}].skus[{j}].price"),
                        format!("{price:?} is not a valid monetary amount"),
                    ));
                }
                for list_ref in &sku.option_list_refs {
                    if !option_list_refs.contains(list_ref) {
                        errors.push((
                            format!("products[{i}].skus[{j}].option_list_refs"),
                            format!("unknown option list ref {list_ref:?}"),
                        ));
                    }
                }
                for barcode in &sku.barcodes {
                    if !matches!(barcode.len(), 8 | 12 | 13)
                        || !barcode.bytes().all(|b| b.is_ascii_digit())
                    {
                        errors.push((
                            format!("products[{i}].skus[{j}].barcodes"),
                            format!("{barcode:?} must be 8, 12 or 13 digits"),
                        ));
                    }
                }
                match &sku.name {
                    Some(name) => {
                        if !sku_names.insert(name.clone()) {
                            errors.push((
                                format!("products[{i}].skus[{j}].name"),
                                format!("sku name {name:?} repeated in this product"),
                            ));
                        }
                    }
                    None => {
                        if sku_names_seen {
                            errors.push((
                                format!("products[{i}].skus[{j}].name"),
                                "only one sku without a name is allowed per product".to_string(),
                            ));
                        }
                        sku_names_seen = true;
                    }
                }
            }
        }
    }

    for (i, o) in data.option_lists.iter().enumerate() {
        if o.options.is_empty() {
            errors.push((
                format!("option_lists[{i}].options"),
                "an option list must have at least one option".to_string(),
            ));
        }
        let (min, max) = (o.min_selections, o.max_selections);
        if let (Some(min), Some(max)) = (min, max)
            && min > max
        {
            errors.push((
                format!("option_lists[{i}].min_selections"),
                format!("min_selections ({min}) must not exceed max_selections ({max})"),
            ));
        }
        if max == Some(0) && o.multiple_selection != Some(true) {
            errors.push((
                format!("option_lists[{i}].max_selections"),
                "max_selections must be greater than 0 unless multiple_selection is true"
                    .to_string(),
            ));
        }
        let defaults = o
            .options
            .iter()
            .filter(|opt| opt.default == Some(true))
            .count();
        if let Some(max) = max
            && defaults > max as usize
        {
            errors.push((
                format!("option_lists[{i}].options"),
                format!("{defaults} default options exceed max_selections ({max})"),
            ));
        }
        for (j, opt) in o.options.iter().enumerate() {
            if let Some(price) = &opt.price
                && pos_hubrise_types::money::parse_cents(price).is_err()
            {
                errors.push((
                    format!("option_lists[{i}].options[{j}].price"),
                    format!("{price:?} is not a valid monetary amount"),
                ));
            }
        }
    }

    for (i, c) in data.charges.iter().enumerate() {
        if let Some(price) = &c.price
            && pos_hubrise_types::money::parse_cents(price).is_err()
        {
            errors.push((
                format!("charges[{i}].price"),
                format!("{price:?} is not a valid monetary amount"),
            ));
        }
    }

    // Sku refs must be unique catalog-wide (referenced by option
    // lists / orders / inventory by ref).
    {
        let mut s = HashSet::new();
        for (i, p) in data.products.iter().enumerate() {
            for (j, sku) in p.skus.iter().enumerate() {
                if let Some(r) = &sku.ref_field {
                    seen(
                        &mut errors,
                        &format!("products[{i}].skus[{j}].ref"),
                        &mut s,
                        r,
                    );
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::fields(errors))
    }
}

/// The refs of every sku and option in a catalog, for inventory
/// validation.
fn catalog_refs(catalog: &CatalogData) -> (HashSet<String>, HashSet<String>) {
    let mut sku_refs = HashSet::new();
    for p in &catalog.products {
        for sku in &p.skus {
            if let Some(r) = &sku.ref_field {
                sku_refs.insert(r.clone());
            }
        }
    }
    let mut option_refs = HashSet::new();
    for o in &catalog.option_lists {
        for opt in &o.options {
            if let Some(r) = &opt.ref_field {
                option_refs.insert(r.clone());
            }
        }
    }
    (sku_refs, option_refs)
}

/// Validate inventory entries against the documented shape: exactly
/// one of `sku_ref`/`option_ref`, both resolving in `catalog_id`;
/// `stock` a non-negative decimal with 0 to 3 decimals; `expires_at`
/// only when `stock` is exactly `0`.
fn validate_inventory_entries(
    inner: &Inner,
    catalog_id: &str,
    entries: &[InventoryEntry],
) -> Result<(), ApiError> {
    let Some(catalog) = inner.catalogs.get(catalog_id).and_then(|c| c.data.as_ref()) else {
        return Err(ApiError::new(404, "catalog not found"));
    };
    let (sku_refs, option_refs) = catalog_refs(catalog);
    let mut errors = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        match (&entry.sku_ref, &entry.option_ref) {
            (Some(_), Some(_)) => errors.push((
                format!("[{i}]"),
                "exactly one of sku_ref / option_ref is allowed, not both".to_string(),
            )),
            (None, None) => errors.push((
                format!("[{i}]"),
                "exactly one of sku_ref / option_ref is required".to_string(),
            )),
            (Some(r), None) if !sku_refs.contains(r) => {
                errors.push((format!("[{i}].sku_ref"), format!("unknown sku ref {r:?}")))
            }
            (None, Some(r)) if !option_refs.contains(r) => errors.push((
                format!("[{i}].option_ref"),
                format!("unknown option ref {r:?}"),
            )),
            _ => {}
        }
        let stock_thousandths = match &entry.stock {
            None => None,
            Some(s) => match parse_stock_thousandths(s) {
                Some(v) => Some(v),
                None => {
                    errors.push((
                        format!("[{i}].stock"),
                        format!("{s:?} must be a non-negative decimal with at most 3 decimals"),
                    ));
                    None
                }
            },
        };
        if entry.expires_at.is_some() && stock_thousandths != Some(0) {
            errors.push((
                format!("[{i}].expires_at"),
                "expires_at is only allowed when stock is 0".to_string(),
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ApiError::fields(errors))
    }
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
    fn transitions_are_permissive_with_strict_mode_off() {
        assert!(transition_allowed(false, "completed", "accepted"));
        assert!(transition_allowed(false, "cancelled", "accepted"));
        assert!(transition_allowed(false, "accepted", "new"));
        assert!(!transition_allowed(false, "accepted", "bogus"));
    }

    #[test]
    fn strict_transitions_follow_the_mocks_old_policy() {
        assert!(transition_allowed(true, "new", "received"));
        assert!(transition_allowed(true, "received", "accepted"));
        assert!(transition_allowed(true, "accepted", "cancelled"));
        assert!(transition_allowed(true, "new", "rejected"));
        assert!(!transition_allowed(true, "completed", "accepted"));
        assert!(!transition_allowed(true, "cancelled", "accepted"));
        assert!(!transition_allowed(true, "accepted", "new"));
        assert!(!transition_allowed(true, "accepted", "bogus"));
        assert!(transition_allowed(true, "completed", "completed"));
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
    fn a_bad_amount_is_refused() {
        let mut inner = Inner::seeded();
        let loc = inner.default_location_id.clone();
        let bad = json!({ "items": [{ "product_name": "X", "price": "abc", "quantity": "1" }] });
        assert_eq!(
            inner.create_order(POS_TOKEN, &loc, bad).unwrap_err().status,
            422
        );
    }

    #[test]
    fn a_duplicate_private_ref_is_allowed_by_default_but_refused_strict() {
        let mut inner = Inner::seeded();
        let loc = inner.default_location_id.clone();
        let one = json!({ "status": "completed", "private_ref": "sale-1" });
        inner.create_order(POS_TOKEN, &loc, one.clone()).unwrap();
        // Default: no location-wide uniqueness constraint.
        inner.create_order(POS_TOKEN, &loc, one.clone()).unwrap();

        inner.settings.strict_private_ref = true;
        assert_eq!(
            inner.create_order(POS_TOKEN, &loc, one).unwrap_err().status,
            422
        );
    }

    #[test]
    fn private_ref_lookup_is_scoped_to_the_creating_client() {
        let mut inner = Inner::seeded();
        let loc = inner.default_location_id.clone();
        inner
            .create_order(POS_TOKEN, &loc, json!({ "private_ref": "shared-ref" }))
            .unwrap();
        inner
            .create_order(WEBSHOP_TOKEN, &loc, json!({ "private_ref": "shared-ref" }))
            .unwrap();
        assert_eq!(
            inner
                .orders_by_private_ref(POS_TOKEN, &loc, "shared-ref")
                .count(),
            1
        );
        assert_eq!(
            inner
                .orders_by_private_ref(WEBSHOP_TOKEN, &loc, "shared-ref")
                .count(),
            1
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

    #[test]
    fn connection_wide_rate_limit_applies_per_token() {
        let mut inner = Inner::seeded();
        inner.settings.rate_limit_per_minute = 2;
        assert!(inner.request_allowed(POS_TOKEN).is_ok());
        assert!(inner.request_allowed(POS_TOKEN).is_ok());
        assert_eq!(inner.request_allowed(POS_TOKEN).unwrap_err().status, 429);
        assert!(inner.request_allowed(WEBSHOP_TOKEN).is_ok());
    }

    #[test]
    fn a_catalog_with_a_dangling_ref_is_refused_whole() {
        // `validate_catalog` is what the HTTP layer (`create_catalog`,
        // `replace_catalog` in lib.rs) calls before ever writing a
        // catalog; `Inner::replace_catalog` itself stays infallible
        // for other crates that seed catalogs directly in tests.
        let bad = CatalogData {
            products: vec![pos_hubrise_types::Product {
                id: None,
                ref_field: Some("P1".into()),
                category_ref: "MISSING".into(),
                category_id: None,
                name: "X".into(),
                description: None,
                tags: vec![],
                tax_rate: None,
                image_ids: vec![],
                skus: vec![],
            }],
            ..Default::default()
        };
        let err = validate_catalog(&bad).unwrap_err();
        assert_eq!(err.status, 422);
    }

    #[test]
    fn the_seed_catalog_is_valid() {
        validate_catalog(&seed_catalog()).expect("seed catalog must validate");
        // Every `CatalogBuilder` in `builders.rs` stays valid too.
        let extra = crate::builders::CatalogBuilder::new()
            .category("C", "Cat")
            .product(
                "P",
                "C",
                "Product",
                crate::builders::FOOD,
                &[("S", None, 100, &[])],
            )
            .option_list("O", "Options", 0, Some(1), &[("O1", "Opt", 0)])
            .charge("CH", "Charge", "delivery", Some(100))
            .build();
        validate_catalog(&extra).expect("CatalogBuilder output must validate");
    }

    #[test]
    fn inventory_entries_need_one_ref_and_a_valid_stock() {
        let mut inner = Inner::seeded();
        let catalog_id = inner.default_catalog_id.clone();
        let ok = inner.replace_inventory(
            WEBSHOP_TOKEN,
            &catalog_id,
            vec![InventoryEntry {
                sku_ref: Some("MAKI-SAUMON-6".into()),
                option_ref: None,
                stock: Some("2.5".into()),
                expires_at: None,
            }],
        );
        assert!(ok.is_ok(), "{ok:?}");

        let bad = inner.replace_inventory(
            WEBSHOP_TOKEN,
            &catalog_id,
            vec![InventoryEntry {
                sku_ref: None,
                option_ref: None,
                stock: Some("1".into()),
                expires_at: None,
            }],
        );
        assert_eq!(bad.unwrap_err().status, 422);

        let bad_expiry = inner.replace_inventory(
            WEBSHOP_TOKEN,
            &catalog_id,
            vec![InventoryEntry {
                sku_ref: Some("MAKI-SAUMON-6".into()),
                option_ref: None,
                stock: Some("1".into()),
                expires_at: Some("2026-01-01T00:00:00Z".into()),
            }],
        );
        assert_eq!(bad_expiry.unwrap_err().status, 422);
    }

    #[test]
    fn patch_inventory_removes_on_null_stock() {
        let mut inner = Inner::seeded();
        let catalog_id = inner.default_catalog_id.clone();
        inner
            .replace_inventory(
                WEBSHOP_TOKEN,
                &catalog_id,
                vec![InventoryEntry {
                    sku_ref: Some("MAKI-SAUMON-6".into()),
                    option_ref: None,
                    stock: Some("2".into()),
                    expires_at: None,
                }],
            )
            .unwrap();
        let patched = inner
            .patch_inventory(
                WEBSHOP_TOKEN,
                &catalog_id,
                vec![InventoryEntry {
                    sku_ref: Some("MAKI-SAUMON-6".into()),
                    option_ref: None,
                    stock: None,
                    expires_at: None,
                }],
            )
            .unwrap();
        assert_eq!(patched.len(), 1);
        assert!(inner.inventory_of(Some(&catalog_id)).is_empty());
    }
}
