use axum::{routing::get_service, Router};
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub async fn start_server(root: String, port: u16) {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .fallback_service(get_service(ServeDir::new(&root)))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);
    tracing::info!("serving root: {}", root);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
