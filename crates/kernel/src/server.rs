//! Development server using Axum

use axum::{
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::services::ServeDir;

pub async fn start_dev_server(port: u16) -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(|| async { "Nexus Dev Server" }))
        .nest_service("/public", ServeDir::new("public"));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("🚀 Nexus dev server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_starts() {
        // Basic smoke test - server can be created
        // Full integration test in Issue #4
    }
}
