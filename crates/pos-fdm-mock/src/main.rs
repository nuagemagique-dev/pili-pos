#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Mock FDM listening on http://localhost:{port}/graphql");
    println!(
        "Mock email (Scaleway TEM) on http://localhost:{port}{}",
        pos_fdm_mock::TEM_ROUTE
    );
    println!("Press Ctrl+C to stop");
    axum::serve(listener, pos_fdm_mock::app()).await.unwrap();
}
