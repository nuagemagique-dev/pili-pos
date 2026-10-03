//! The HubRise mock (`pos-hubrise-mock`) served in-process on an
//! ephemeral port, and a TestApp connected to it.

#![cfg(feature = "hubrise")]

use std::sync::Arc;

use pos_hubrise::token_store::{self, StoredConnection};
use pos_hubrise_mock::state::{Fault, LoggedRequest, MockState};
pub use pos_hubrise_mock::state::{POS_TOKEN, WEBSHOP_TOKEN};
use serde_json::Value;

use super::harness::TestApp;

/// A running mock. Its server stops when the test's runtime ends.
pub struct HubriseMock {
    pub state: Arc<MockState>,
    /// `http://127.0.0.1:<port>`.
    pub url: String,
}

impl HubriseMock {
    pub async fn start() -> Self {
        let state = Arc::new(MockState::new_seeded());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let app = pos_hubrise_mock::app(state.clone());
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve mock");
        });
        Self { state, url }
    }

    /// The API base URL, as `hubrise_connection.base_url` stores it.
    pub fn api(&self) -> String {
        format!("{}/v1", self.url)
    }

    /// Place a template order as the ordering side (`builders::TEMPLATES`).
    pub fn inject(&self, template: &str) -> Value {
        let body = pos_hubrise_mock::builders::template(template)
            .expect("template")
            .build();
        self.inject_body(body)
    }

    /// Place `body` as the ordering side.
    pub fn inject_body(&self, body: Value) -> Value {
        let mut inner = self.state.inner.lock().unwrap();
        let loc = inner.default_location_id.clone();
        inner
            .create_order(WEBSHOP_TOKEN, &loc, body)
            .expect("order")
    }

    /// Change an order as the ordering side (a platform cancelling).
    pub fn update(&self, order_id: &str, patch: Value) -> Value {
        self.state
            .inner
            .lock()
            .unwrap()
            .update_order(WEBSHOP_TOKEN, order_id, &patch)
            .expect("update")
    }

    /// The order as HubRise holds it.
    pub fn order(&self, order_id: &str) -> Value {
        self.state.inner.lock().unwrap().orders[order_id].clone()
    }

    pub fn fault(&self, method: &str, path: &str, status: u16, times: Option<u32>) {
        self.state.inner.lock().unwrap().faults.push(Fault {
            method: method.to_string(),
            path: path.to_string(),
            status,
            times,
            delay_ms: 0,
        });
    }

    pub fn clear_faults(&self) {
        self.state.inner.lock().unwrap().faults.clear();
    }

    /// Requests received whose method is `method` and path starts
    /// with `path`.
    pub fn requests(&self, method: &str, path: &str) -> Vec<LoggedRequest> {
        self.state
            .inner
            .lock()
            .unwrap()
            .requests
            .iter()
            .filter(|r| r.method == method && r.path.starts_with(path))
            .cloned()
            .collect()
    }

    /// Events waiting for the POS.
    pub fn pos_events(&self) -> usize {
        self.state.inner.lock().unwrap().clients[POS_TOKEN]
            .events
            .len()
    }
}

/// A TestApp on one database file, connected to a fresh mock.
pub async fn connected_app() -> (TestApp, tempfile::TempDir, HubriseMock) {
    let (app, dir) = TestApp::with_shared_db().await;
    let mock = HubriseMock::start().await;
    connect(&app, &mock);
    (app, dir, mock)
}

/// The poller context the app runs with.
pub fn poller(app: &TestApp) -> pos_hubrise::poller::PollerCtx {
    pili_pos::commands::hubrise::poller_ctx(&app.state)
}

/// One outbox pass.
pub async fn drain_outbox(app: &TestApp) -> pos_hubrise::outbox::PassReport {
    pos_hubrise::outbox::process_pending(&app.state.shared_http, &app.state.poller_db)
        .await
        .expect("outbox pass")
}

/// Make every retried outbox row due now.
pub fn outbox_due_now(app: &TestApp) {
    app.state
        .db
        .lock()
        .unwrap()
        .execute("UPDATE hubrise_outbox SET next_attempt_at = NULL", [])
        .unwrap();
}

/// Store a HubRise connection to `mock` in `app`, as the OAuth flow
/// does, on both connections.
pub fn connect(app: &TestApp, mock: &HubriseMock) -> StoredConnection {
    let (location_id, catalog_id) = {
        let inner = mock.state.inner.lock().unwrap();
        (
            inner.default_location_id.clone(),
            inner.default_catalog_id.clone(),
        )
    };
    let stored = StoredConnection {
        access_token: POS_TOKEN.to_string(),
        account_id: "acc".to_string(),
        account_name: "Test account".to_string(),
        location_id,
        location_name: "Test location".to_string(),
        catalog_id,
        catalog_name: Some("Menu".to_string()),
        customer_list_id: None,
        customer_list_name: None,
        scope: "location[orders.write,catalog.read]".to_string(),
        base_url: mock.api(),
        last_event_cursor: None,
    };
    let db = app.state.poller_db.lock().unwrap();
    token_store::save(&db, &stored).unwrap();
    // A connected installation has the integration switched on.
    app.state.hubrise_switch.set(true);
    stored
}
