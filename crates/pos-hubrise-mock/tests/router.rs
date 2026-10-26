//! The mock's HTTP API, driven in-process with `tower::oneshot`.

#![expect(
    clippy::unwrap_used,
    reason = "test code: a failed setup is the test's failure"
)]

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use pos_hubrise_mock::app;
use pos_hubrise_mock::state::{MockState, POS_TOKEN, WEBSHOP_TOKEN};
use serde_json::{Value, json};
use tower::ServiceExt;

struct Mock {
    state: Arc<MockState>,
    router: Router,
}

struct Reply {
    status: StatusCode,
    cursor_next: Option<String>,
    json: Value,
}

impl Mock {
    fn new() -> Self {
        let state = Arc::new(MockState::new_seeded());
        Self {
            router: app(Arc::clone(&state)),
            state,
        }
    }

    async fn send(
        &self,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> Reply {
        let mut req = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            req = req.header("X-Access-Token", t);
        }
        let req = match body {
            Some(b) => req
                .header("Content-Type", "application/json")
                .body(Body::from(b.to_string())),
            None => req.body(Body::empty()),
        }
        .unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let cursor_next = resp
            .headers()
            .get("X-Cursor-Next")
            .map(|v| v.to_str().unwrap().to_string());
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            cursor_next,
            json: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn register_pos(&self) {
        let r = self
            .send(
                "POST",
                "/v1/callback",
                Some(POS_TOKEN),
                Some(json!({ "url": null, "events": {
                    "order": ["create", "update"], "catalog": ["create", "update"]
                }})),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK);
    }

    async fn inject(&self, template: &str) -> Value {
        let r = self
            .send(
                "POST",
                "/mock/orders",
                None,
                Some(json!({ "template": template })),
            )
            .await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.json);
        r.json
    }
}

#[tokio::test]
async fn the_router_builds_and_answers_health() {
    let m = Mock::new();
    assert_eq!(m.send("GET", "/", None, None).await.status, StatusCode::OK);
}

#[tokio::test]
async fn a_request_without_token_is_unauthorized() {
    let m = Mock::new();
    let r = m.send("GET", "/v1/location", None, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    assert_eq!(r.json["error_type"], "unauthorized");
    let r = m.send("GET", "/v1/location", Some("bogus"), None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn an_unregistered_pos_gets_no_events() {
    let m = Mock::new();
    m.inject("deliveroo_prepaid").await;
    let r = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    assert_eq!(r.json, json!([]));
}

#[tokio::test]
async fn a_registered_pos_is_told_without_state_fields() {
    let m = Mock::new();
    m.register_pos().await;
    let order = m.inject("deliveroo_prepaid").await;
    let r = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    let events = r.json.as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["resource_type"], "order");
    assert_eq!(events[0]["event_type"], "create");
    assert_eq!(events[0]["order_id"], order["id"]);
    assert!(events[0]["new_state"].is_null(), "lists carry no state");

    // The single event still has it; deleting acknowledges it.
    let id = events[0]["id"].as_str().unwrap();
    let one = m
        .send(
            "GET",
            &format!("/v1/callback/events/{id}"),
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(one.json["new_state"]["id"], order["id"]);
    let del = m
        .send(
            "DELETE",
            &format!("/v1/callback/events/{id}"),
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(del.status, StatusCode::OK);
    let r = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    assert_eq!(r.json, json!([]));
    let again = m
        .send(
            "DELETE",
            &format!("/v1/callback/events/{id}"),
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(again.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_pos_is_not_told_about_its_own_changes() {
    let m = Mock::new();
    m.register_pos().await;
    let order = m.inject("webshop_online").await;
    let id = order["id"].as_str().unwrap();
    let r = m
        .send(
            "PATCH",
            &format!("/v1/location/orders/{id}"),
            Some(POS_TOKEN),
            Some(json!({ "status": "received" })),
        )
        .await;
    assert_eq!(r.json["status"], "received");
    let events = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    assert_eq!(events.json.as_array().unwrap().len(), 1, "only the create");
}

#[tokio::test]
async fn events_are_paginated() {
    let m = Mock::new();
    m.register_pos().await;
    for _ in 0..3 {
        m.inject("eat_in").await;
    }
    let first = m
        .send("GET", "/v1/callback/events?count=2", Some(POS_TOKEN), None)
        .await;
    assert_eq!(first.json.as_array().unwrap().len(), 2);
    let cursor = first.cursor_next.expect("more to read");
    let rest = m
        .send(
            "GET",
            &format!("/v1/callback/events?count=2&cursor={cursor}"),
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(rest.json.as_array().unwrap().len(), 1);
    assert!(rest.cursor_next.is_none());
}

#[tokio::test]
async fn transitions_are_permissive_when_strict_mode_is_off() {
    let m = Mock::new();
    // The default is `strict_transitions: true` (the mock's original
    // policy, see `README.md` "Toggles"); this test opts into the
    // docs-permissive behaviour instead.
    m.send(
        "PUT",
        "/mock/settings",
        None,
        Some(json!({ "strict_transitions": false })),
    )
    .await;
    let order = m.inject("eat_in").await;
    let id = order["id"].as_str().unwrap();
    m.send("POST", &format!("/mock/orders/{id}/cancel"), None, None)
        .await;
    // The docs define no transition graph: with strict mode off, even
    // a "final" status accepts a further change.
    let r = m
        .send(
            "PATCH",
            &format!("/v1/location/orders/{id}"),
            Some(POS_TOKEN),
            Some(json!({ "status": "accepted" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["status"], "accepted");
}

#[tokio::test]
async fn a_final_status_cannot_change_under_strict_transitions() {
    let m = Mock::new();
    m.send(
        "PUT",
        "/mock/settings",
        None,
        Some(json!({ "strict_transitions": true })),
    )
    .await;
    let order = m.inject("eat_in").await;
    let id = order["id"].as_str().unwrap();
    m.send("POST", &format!("/mock/orders/{id}/cancel"), None, None)
        .await;
    let r = m
        .send(
            "PATCH",
            &format!("/v1/location/orders/{id}"),
            Some(POS_TOKEN),
            Some(json!({ "status": "accepted" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["error_type"], "unprocessable_entity");
    assert_eq!(r.json["errors"][0]["field"], "status");
}

#[tokio::test]
async fn orders_are_listed_by_status_and_private_ref() {
    let m = Mock::new();
    let a = m.inject("webshop_online").await;
    m.inject("eat_in").await;
    let id = a["id"].as_str().unwrap();
    m.send(
        "PATCH",
        &format!("/v1/location/orders/{id}"),
        Some(POS_TOKEN),
        Some(json!({ "status": "accepted" })),
    )
    .await;
    let new = m
        .send(
            "GET",
            "/v1/location/orders?status=new",
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(new.json.as_array().unwrap().len(), 1);

    let sale = json!({ "status": "completed", "private_ref": "pili-sale-7", "items": [] });
    m.send(
        "POST",
        "/v1/location/orders",
        Some(POS_TOKEN),
        Some(sale.clone()),
    )
    .await;
    let found = m
        .send(
            "GET",
            "/v1/location/orders?private_ref=pili-sale-7",
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(found.json.as_array().unwrap().len(), 1);

    // `private_ref` is a per-client lookup convenience by default,
    // not a location-wide uniqueness constraint: a duplicate is fine.
    let dup = m
        .send(
            "POST",
            "/v1/location/orders",
            Some(POS_TOKEN),
            Some(sale.clone()),
        )
        .await;
    assert_eq!(dup.status, StatusCode::OK);

    // Another client's private_ref of the same value never shows up
    // in this client's lookup.
    let other = m
        .send(
            "POST",
            "/v1/location/orders",
            Some(WEBSHOP_TOKEN),
            Some(sale.clone()),
        )
        .await;
    assert_eq!(other.status, StatusCode::OK);
    let found = m
        .send(
            "GET",
            "/v1/location/orders?private_ref=pili-sale-7",
            Some(POS_TOKEN),
            None,
        )
        .await;
    assert_eq!(
        found.json.as_array().unwrap().len(),
        2,
        "both of this client's own orders, not the webshop's"
    );

    // Strict mode restores the old location-wide uniqueness 422.
    m.send(
        "PUT",
        "/mock/settings",
        None,
        Some(json!({ "strict_private_ref": true })),
    )
    .await;
    let strict_dup = m
        .send("POST", "/v1/location/orders", Some(POS_TOKEN), Some(sale))
        .await;
    assert_eq!(strict_dup.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(strict_dup.json["errors"][0]["field"], "private_ref");
}

#[tokio::test]
async fn a_catalog_update_is_announced_without_state() {
    let m = Mock::new();
    m.register_pos().await;
    let r = m.send("PUT", "/mock/catalog", None, Some(json!({}))).await;
    assert_eq!(r.status, StatusCode::OK);
    let events = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    let e = &events.json[0];
    assert_eq!(e["resource_type"], "catalog");
    assert_eq!(e["event_type"], "update");
    assert_eq!(e["catalog_id"], r.json["id"]);
}

#[tokio::test]
async fn catalog_reads_past_the_limit_get_429() {
    let m = Mock::new();
    m.send(
        "POST",
        "/mock/limits",
        None,
        Some(json!({ "catalog_reads_per_minute": 1 })),
    )
    .await;
    let id = m.state.inner.lock().unwrap().default_catalog_id.clone();
    let ok = m
        .send("GET", &format!("/v1/catalogs/{id}"), Some(POS_TOKEN), None)
        .await;
    assert_eq!(ok.status, StatusCode::OK);
    assert!(ok.json["data"]["products"].as_array().unwrap().len() > 1);
    let limited = m
        .send("GET", &format!("/v1/catalogs/{id}"), Some(POS_TOKEN), None)
        .await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.json["error_type"], "too_many_requests");
}

#[tokio::test]
async fn an_injected_fault_fails_the_next_requests_then_clears() {
    let m = Mock::new();
    m.send(
        "POST",
        "/mock/faults",
        None,
        Some(json!({ "method": "GET", "path": "/v1/callback/events", "status": 500, "times": 2 })),
    )
    .await;
    for _ in 0..2 {
        let r = m
            .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
            .await;
        assert_eq!(r.status, StatusCode::INTERNAL_SERVER_ERROR);
    }
    let r = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test]
async fn every_api_request_is_logged_with_its_client() {
    let m = Mock::new();
    m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    m.send(
        "POST",
        "/v1/callback",
        Some(WEBSHOP_TOKEN),
        Some(json!({ "url": null, "events": {} })),
    )
    .await;
    let log = m.send("GET", "/mock/requests?path=/v1", None, None).await;
    let log = log.json.as_array().unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0]["client"], "pos");
    assert_eq!(log[0]["status"], 200);
    assert_eq!(log[1]["client"], "webshop");
    assert_eq!(log[1]["body"]["url"], Value::Null);
}

#[tokio::test]
async fn the_token_exchange_takes_a_form_body_and_basic_auth() {
    use base64::Engine;
    let m = Mock::new();
    let code = m
        .send(
            "GET",
            "/oauth2/v1/authorize?client_id=pili-pos&scope=x",
            None,
            None,
        )
        .await
        .json["code"]
        .as_str()
        .unwrap()
        .to_string();
    let basic = base64::engine::general_purpose::STANDARD.encode("pili-pos:secret");
    let req = Request::post("/oauth2/v1/token")
        .header("Authorization", format!("Basic {basic}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(format!("code={code}")))
        .unwrap();
    let resp = m.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let tok: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(tok["access_token"], POS_TOKEN);

    // A code works once.
    let again = m
        .send(
            "POST",
            "/oauth2/v1/token",
            None,
            Some(json!({ "code": code })),
        )
        .await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_unknown_template_is_refused() {
    let m = Mock::new();
    let r = m
        .send(
            "POST",
            "/mock/orders",
            None,
            Some(json!({ "template": "nope" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn reset_brings_back_the_seed() {
    let m = Mock::new();
    m.inject("eat_in").await;
    m.send("POST", "/mock/reset", None, None).await;
    let r = m
        .send("GET", "/v1/location/orders", Some(POS_TOKEN), None)
        .await;
    assert_eq!(r.json, json!([]));
}

#[tokio::test]
async fn an_unmatched_route_is_a_routing_error() {
    let m = Mock::new();
    let r = m
        .send("GET", "/v1/not-a-real-route", Some(POS_TOKEN), None)
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.json["error_type"], "routing_error");
}

#[tokio::test]
async fn a_write_with_the_wrong_content_type_is_refused() {
    let m = Mock::new();
    let req = Request::builder()
        .method("POST")
        .uri("/v1/location/orders")
        .header("X-Access-Token", POS_TOKEN)
        .header("Content-Type", "text/plain")
        .body(Body::from(json!({ "status": "new" }).to_string()))
        .unwrap();
    let resp = m.router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["error_type"], "unsupported_media_type");
}

#[tokio::test]
async fn a_money_error_carries_a_field_level_detail() {
    let m = Mock::new();
    let r = m
        .send(
            "POST",
            "/v1/location/orders",
            Some(POS_TOKEN),
            Some(json!({ "items": [{ "product_name": "X", "price": "abc", "quantity": "1" }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["errors"][0]["field"], "items.price");
}

#[tokio::test]
async fn a_revoked_token_is_unauthorized_until_reset() {
    let m = Mock::new();
    let ok = m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    assert_eq!(ok.status, StatusCode::OK);
    let revoke = m
        .send("POST", "/oauth2/v1/revoke", Some(POS_TOKEN), None)
        .await;
    assert_eq!(revoke.status, StatusCode::OK);
    let after = m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    assert_eq!(after.status, StatusCode::UNAUTHORIZED);
    m.send("POST", "/mock/reset", None, None).await;
    let restored = m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    assert_eq!(restored.status, StatusCode::OK);
}

#[tokio::test]
async fn registering_an_active_callback_is_refused() {
    let m = Mock::new();
    let r = m
        .send(
            "POST",
            "/v1/callback",
            Some(POS_TOKEN),
            Some(json!({ "url": "https://example.com/hook", "events": {} })),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["error_type"], "unprocessable_entity");
}

#[tokio::test]
async fn inventory_is_scoped_by_catalog_and_validated() {
    let m = Mock::new();
    let catalog_id = m.state.inner.lock().unwrap().default_catalog_id.clone();

    let set = m
        .send(
            "PUT",
            &format!("/v1/catalogs/{catalog_id}/location/inventory"),
            Some(WEBSHOP_TOKEN),
            Some(json!([
                { "sku_ref": "MAKI-SAUMON-6", "stock": "5" },
                { "sku_ref": "COCA-33", "stock": "0", "expires_at": "2026-12-01T00:00:00Z" },
            ])),
        )
        .await;
    assert_eq!(set.status, StatusCode::OK);

    let get = m
        .send("GET", "/v1/location/inventory", Some(POS_TOKEN), None)
        .await;
    assert_eq!(get.json.as_array().unwrap().len(), 2);

    // A dangling ref refuses the whole write.
    let bad = m
        .send(
            "PUT",
            "/v1/location/inventory",
            Some(WEBSHOP_TOKEN),
            Some(json!([{ "sku_ref": "NO-SUCH-SKU", "stock": "1" }])),
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);

    // `expires_at` only makes sense when stock is exactly 0.
    let bad_expiry = m
        .send(
            "PUT",
            "/v1/location/inventory",
            Some(WEBSHOP_TOKEN),
            Some(json!([{
                "sku_ref": "MAKI-SAUMON-6", "stock": "1", "expires_at": "2026-12-01T00:00:00Z"
            }])),
        )
        .await;
    assert_eq!(bad_expiry.status, StatusCode::UNPROCESSABLE_ENTITY);

    // PATCH with `stock: null` removes an entry and returns only the
    // modified ones.
    let patch = m
        .send(
            "PATCH",
            "/v1/location/inventory",
            Some(WEBSHOP_TOKEN),
            Some(json!([{ "sku_ref": "MAKI-SAUMON-6" }])),
        )
        .await;
    assert_eq!(patch.status, StatusCode::OK);
    assert_eq!(patch.json.as_array().unwrap().len(), 1);
    let get = m
        .send("GET", "/v1/location/inventory", Some(POS_TOKEN), None)
        .await;
    assert_eq!(get.json.as_array().unwrap().len(), 1, "one entry left");
}

#[tokio::test]
async fn inventory_updates_announce_the_other_registered_client() {
    let m = Mock::new();
    // Register for inventory events (the default `register_pos`
    // helper only asks for order/catalog).
    m.send(
        "POST",
        "/v1/callback",
        Some(POS_TOKEN),
        Some(json!({ "url": null, "events": {
            "order": ["create", "update"], "catalog": ["create", "update"],
            "inventory": ["update", "patch"],
        }})),
    )
    .await;
    let r = m
        .send(
            "PUT",
            "/mock/inventory",
            None,
            Some(json!({ "entries": [{ "sku_ref": "MISO-1", "stock": "4" }] })),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let events = m
        .send("GET", "/v1/callback/events", Some(POS_TOKEN), None)
        .await;
    let events = events.json.as_array().unwrap();
    assert!(
        events
            .iter()
            .any(|e| e["resource_type"] == "inventory" && e["event_type"] == "update")
    );
}

#[tokio::test]
async fn connection_wide_rate_limit_answers_429() {
    let m = Mock::new();
    m.send(
        "PUT",
        "/mock/settings",
        None,
        Some(json!({ "rate_limit_per_minute": 2 })),
    )
    .await;
    m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    let limited = m.send("GET", "/v1/location", Some(POS_TOKEN), None).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.json["error_type"], "too_many_requests");
}
