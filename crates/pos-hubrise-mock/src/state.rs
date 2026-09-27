//! In-memory state for the HubRise mock.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use pos_hubrise_types::{CallbackEvent, CallbackRegistration, Catalog, Order};

/// Global mock state. Wrapped in `Arc<Mutex>` for sharing across axum handlers.
pub struct MockState {
    pub inner: Mutex<Inner>,
}

pub struct Inner {
    /// Clients indexed by access token.
    pub clients: HashMap<String, ClientState>,
    /// All orders keyed by HubRise order id.
    pub orders: HashMap<String, Order>,
    /// All catalogs keyed by catalog id.
    pub catalogs: HashMap<String, Catalog>,
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
}

pub struct ClientState {
    #[allow(dead_code)]
    pub access_token: String,
    #[allow(dead_code)]
    pub client_name: String,
    pub account_name: String,
    pub location_name: String,
    pub catalog_name: String,
    pub customer_list_name: String,
    pub events: VecDeque<CallbackEvent>,
    pub registered_callback: Option<CallbackRegistration>,
}

impl MockState {
    pub fn new_seeded() -> Self {
        let mut clients = HashMap::new();
        clients.insert(
            "dev-token-webshop".to_string(),
            ClientState {
                access_token: "dev-token-webshop".to_string(),
                client_name: "tsb-webshop".to_string(),
                account_name: "Tokyo Sushi Bar (dev)".to_string(),
                location_name: "Liège".to_string(),
                catalog_name: "Menu principal".to_string(),
                customer_list_name: "Clients TSB".to_string(),
                events: VecDeque::new(),
                registered_callback: None,
            },
        );
        clients.insert(
            "dev-token-pos".to_string(),
            ClientState {
                access_token: "dev-token-pos".to_string(),
                client_name: "pili-pos".to_string(),
                account_name: "Tokyo Sushi Bar (dev)".to_string(),
                location_name: "Liège".to_string(),
                catalog_name: "Menu principal".to_string(),
                customer_list_name: "Clients TSB".to_string(),
                events: VecDeque::new(),
                registered_callback: None,
            },
        );

        Self {
            inner: Mutex::new(Inner {
                clients,
                orders: HashMap::new(),
                catalogs: HashMap::new(),
                pending_auth_codes: HashMap::new(),
                default_location_id: "pili-dev-loc-1".to_string(),
                default_catalog_id: "pili-dev-cat-1".to_string(),
                default_customer_list_id: "pili-dev-cl-1".to_string(),
                default_account_id: "pili-dev-acc-1".to_string(),
            }),
        }
    }

    /// Broadcast an event to all clients except the one that triggered it.
    ///
    /// HubRise does not echo events back to the creator - a client should
    /// never see an `order.create` event for an order it POSTed itself.
    pub fn broadcast_event(&self, source_token: &str, event: CallbackEvent) {
        let mut inner = self.inner.lock().expect("poisoned mock lock");
        let source_token = source_token.to_string();
        for (token, client) in inner.clients.iter_mut() {
            if token != &source_token {
                client.events.push_back(event.clone());
            }
        }
    }
}
