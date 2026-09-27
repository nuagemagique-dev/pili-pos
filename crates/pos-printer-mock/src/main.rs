#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "9100".to_string());
    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "9180".to_string());
    let printer = pos_printer_mock::Printer::from_env();

    let addr = format!("0.0.0.0:{port}").parse().unwrap();
    pos_printer_mock::bind(addr, printer.clone()).await.unwrap();
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{http_port}"))
        .await
        .unwrap();
    println!("Mock printer on 127.0.0.1:{port} (printer_address in pos.json)");
    println!("Jobs and states on http://localhost:{http_port}/");
    println!("Press Ctrl+C to stop");
    axum::serve(listener, pos_printer_mock::http_app(printer))
        .await
        .unwrap();
}
