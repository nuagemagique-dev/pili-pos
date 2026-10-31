#![expect(
    clippy::print_stdout,
    reason = "a developer tool: its ports are its interface"
)]

use std::net::SocketAddr;

use pos_scale_mock::MockScale;

#[tokio::main]
async fn main() {
    let port = |name: &str, default: u16| {
        std::env::var(name)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let check = std::env::var("MOCK_SCALE_CHECK").unwrap_or_else(|_| "4711F336".to_string());
    let scale = MockScale::new(check);
    let dialog = SocketAddr::from(([0, 0, 0, 0], port("MOCK_SCALE_PORT", 9190)));
    let control = SocketAddr::from(([0, 0, 0, 0], port("MOCK_SCALE_CONTROL_PORT", 9191)));
    println!("pos-scale-mock: Dialog 06 on {dialog}, control on {control}");
    let listener = tokio::net::TcpListener::bind(control)
        .await
        .expect("control port");
    tokio::spawn({
        let scale = scale.clone();
        async move {
            scale.serve_dialog(dialog).await.expect("dialog port");
        }
    });
    axum::serve(listener, scale.control_router())
        .await
        .expect("control server");
}
