#![expect(
    clippy::print_stdout,
    clippy::unwrap_used,
    reason = "a standalone tool: the console is its output and a bind failure ends it"
)]

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let addr = format!("0.0.0.0:{port}");
    // MOCK_FDM_TLS_DIR: HTTPS like module2 (server.crt, server.key);
    // MOCK_FDM_TLS_REQUIRE_CLIENT=1 makes the client certificate
    // mandatory.
    if let Ok(dir) = std::env::var("MOCK_FDM_TLS_DIR") {
        let client_cert = if std::env::var("MOCK_FDM_TLS_REQUIRE_CLIENT").is_ok_and(|v| v == "1") {
            pos_fdm_mock::tls::ClientCert::Required
        } else {
            pos_fdm_mock::tls::ClientCert::Optional
        };
        let listener = std::net::TcpListener::bind(&addr).unwrap();
        println!("Mock FDM listening on https://localhost:{port}/graphql (certificates: {dir})");
        println!("Press Ctrl+C to stop");
        pos_fdm_mock::tls::serve(
            listener,
            pos_fdm_mock::app(),
            std::path::Path::new(&dir),
            client_cert,
        )
        .await
        .unwrap();
        return;
    }
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Mock FDM listening on http://localhost:{port}/graphql");
    println!(
        "Mock email (Scaleway TEM) on http://localhost:{port}{}",
        pos_fdm_mock::TEM_ROUTE
    );
    println!("Press Ctrl+C to stop");
    axum::serve(listener, pos_fdm_mock::app()).await.unwrap();
}
