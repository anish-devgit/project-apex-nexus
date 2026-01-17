use axum::{routing::get_service, Router};
use std::net::SocketAddr;
use tower_http::{services::ServeDir, trace::TraceLayer};

pub async fn start_server(root: String, port: u16) -> Result<(), std::io::Error> {
    // Tracing init removed from here, moved to binding/CLI to ensure single init
    
    let app = Router::new()
        .fallback_service(get_service(ServeDir::new(&root)))
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    // Log intent to bind
    tracing::info!("starting server on {}", addr);
    tracing::info!("serving root: {}", root);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    
    tracing::info!("signal received, starting graceful shutdown");
}
