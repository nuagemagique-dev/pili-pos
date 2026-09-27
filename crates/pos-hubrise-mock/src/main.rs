//! `cargo run -p pos-hubrise-mock`: the HubRise mock on port 3001
//! (`HUBRISE_MOCK_PORT` overrides it).

use std::sync::Arc;

use pos_hubrise_mock::{HUBRISE_DOCS_REVISION, app, state::MockState};

#[tokio::main]
async fn main() {
    let port = std::env::var("HUBRISE_MOCK_PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{port}");
    let state = Arc::new(MockState::new_seeded());

    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    println!("HubRise mock listening on http://localhost:{port}");
    println!("Docs revision baseline: {HUBRISE_DOCS_REVISION}");
    println!("Pre-seeded clients:");
    println!("  - tsb-webshop  token=dev-token-webshop");
    println!("  - pili-pos    token=dev-token-pos");
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app(state)).await.expect("axum serve");
}
