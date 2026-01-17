use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower_http::{services::ServeDir, trace::TraceLayer};

pub mod graph;
use graph::*;

// --- DATA STRUCTURES ---

#[derive(Clone)]
struct AppState {
    graph: Arc<RwLock<ModuleGraph>>,
    root_dir: String,
}

// --- MODULE HANDLER ---

async fn handle_module(
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    handle_module_logic(state, uri).await
}

async fn handle_module_logic(state: AppState, uri: Uri) -> Response {
    let path_str = uri.path();
    tracing::info!("Intercepted request: {}", path_str);

    // FIX 2: Path Sanitization
    let mut safe_path = std::path::PathBuf::new();
    for component in std::path::Path::new(path_str).components() {
        match component {
            std::path::Component::Normal(c) => safe_path.push(c),
            _ => {}
        }
    }
    
    if safe_path.as_os_str().is_empty() {
         return (StatusCode::BAD_REQUEST, "Invalid path").into_response();
    }

    let abs_path = std::path::Path::new(&state.root_dir).join(&safe_path);

    // FIX 1: Non-blocking I/O
    let content = match tokio::fs::read_to_string(&abs_path).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read file {}: {}", abs_path.display(), e);
            return (StatusCode::NOT_FOUND, "File not found").into_response();
        }
    };

    {
        let mut graph = state.graph.write().unwrap();
        
        // Find existing module by path or add new
        // The path in graph should probably match the request path (path_str) for identification
        let id_opt = graph.find_by_path(path_str);
        
        if let Some(id) = id_opt {
            graph.update_source(id, &content);
        } else {
            graph.add_module(path_str, &content);
        }
        
        let count = graph.modules.len();
        tracing::info!("Graph Node Updated/Created. Total Nodes: {}", count);
    }

    // FIX 3: Header Consistency
    let mut headers = HeaderMap::new();
    headers.insert("X-Apex-Intercept", "true".parse().unwrap());
    headers.insert("Content-Type", "application/javascript".parse().unwrap());

    (StatusCode::OK, headers, content).into_response()
}

// --- SERVER ---

pub async fn start_server(root: String, port: u16) -> Result<(), std::io::Error> {
    let state = AppState {
        graph: Arc::new(RwLock::new(ModuleGraph::new())),
        root_dir: root.clone(),
    };

    let serve_dir = ServeDir::new(&root);

    let service = tower::service_fn(move |req: axum::extract::Request| {
        let state = state.clone();
        let serve_dir = serve_dir.clone();
        
        async move {
            let uri = req.uri().clone();
            let path = uri.path();
            
            if path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") || path.ends_with(".jsx") {
                let response = handle_module_logic(state, uri).await;
                Ok::<_, std::io::Error>(response)
            } else {
                let res = serve_dir.oneshot(req).await;
                match res {
                    Ok(r) => Ok(r.into_response()),
                    Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()) 
                }
            }
        }
    });

    let app = Router::new()
        .fallback_service(service)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
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
