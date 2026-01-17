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
pub mod parser;
use parser::extract_dependencies;
pub mod watcher;

// --- DATA STRUCTURES ---

#[derive(Clone)]
struct AppState {
    graph: Arc<RwLock<ModuleGraph>>,
    root_dir: String,
    hmr_tx: tokio::sync::broadcast::Sender<watcher::HmrMessage>,
}

// --- MODULE HANDLER ---

async fn handle_module(
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    handle_module_logic(state, uri).await
}

fn resolve_import(base_path: &str, import_spec: &str) -> String {
    let base = std::path::Path::new(base_path);
    let parent = base.parent().unwrap_or_else(|| std::path::Path::new("/"));
    
    // Join and normalize
    let joined = parent.join(import_spec);
    
    // Normalize (std::fs::canonicalize requires file existence, so we do logical normaliztion)
    let mut normalized = std::path::PathBuf::new();
    for component in joined.components() {
        match component {
            std::path::Component::RootDir => normalized.push("/"),
            std::path::Component::Normal(c) => normalized.push(c),
            std::path::Component::ParentDir => { normalized.pop(); },
            std::path::Component::CurDir => {},
            _ => {},
        }
    }
    
    // Ensure it starts with / 
    let s = normalized.to_string_lossy().to_string();
    if !s.starts_with('/') && !s.starts_with('\\') { // windows?
         // On windows joined might be `\src\utils.js`.
         // We want slash consistency? 
         // The `uri.path()` usually has forward slashes.
         // `Path` operations might use backslashes on Windows.
         // Let's force forward slashes for the Graph IDs to be consistent with Uris.
         // `to_string_lossy` uses native.
         // We should replace `\` with `/`.
    }
    s.replace('\\', "/")
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

    // Week 4: Extract Dependencies
    let deps = extract_dependencies(&content, path_str);

    {
        let mut graph = state.graph.write().unwrap();
        
        // Find existing module by path or add new
        // The path in graph should probably match the request path (path_str) for identification
        let id_opt = graph.find_by_path(path_str);
        
        let current_id = if let Some(id) = id_opt {
            graph.update_source(id, &content);
            id
        } else {
            graph.add_module(path_str, &content)
        };
        
        // Populate dependencies
        for dep_spec in deps {
            // Filter only relative imports for Week 4
            if dep_spec.starts_with("./") || dep_spec.starts_with("../") {
                let resolved_path = resolve_import(path_str, &dep_spec);
                
                let dep_id = if let Some(id) = graph.find_by_path(&resolved_path) {
                    id
                } else {
                    // Add missing module with empty source (placeholder)
                    graph.add_module(&resolved_path, "")
                };
                
                let _ = graph.add_dependency(current_id, dep_id);
            }
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
    // Week 6: Start Watcher Channel
    let (tx, _) = tokio::sync::broadcast::channel(100);
    
    // Spawn Watcher
    let watcher_tx = tx.clone();
    let graph = Arc::new(RwLock::new(ModuleGraph::new()));
    
    let watcher_graph = graph.clone();
    let watcher_root = root.clone();
    let server_root = root.clone();
    
    tokio::spawn(async move {
        watcher::start_watcher(watcher_root, watcher_graph, watcher_tx).await;
    });

    let state = AppState {
        graph,
        root_dir: server_root.clone(),
        hmr_tx: tx,
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
            } else if path.starts_with("/_nexus/chunk") {
                 let response = handle_chunk(state, uri).await;
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
        .route("/ws", get(handle_ws))
        .fallback_service(service)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    tracing::info!("starting server on {}", addr);
    tracing::info!("serving root: {}", root);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", addr);

    axum::serve(listener, app.with_state(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
}


// Week 7: Runtime
pub mod runtime;
use runtime::NEXUS_RUNTIME_JS;
use parser::transform_cjs;

async fn handle_chunk(state: AppState, uri: Uri) -> Response {
    let path = uri.path().strip_prefix("/_nexus/chunk").unwrap_or("/");
    let graph = state.graph.read().unwrap();
    
    let root_id = match graph.find_by_path(path) {
        Some(id) => id,
        None => return (StatusCode::NOT_FOUND, "Chunk entry not found").into_response(),
    };
    
    let sorted_ids = graph.linearize(root_id);
    
    let mut chunk_content = String::new();
    
    // 1. Inject Runtime
    chunk_content.push_str(NEXUS_RUNTIME_JS);
    chunk_content.push('\n');

    // 2. Inject HMR Client (Week 6) - We keep this for reloading
    // Note: In a real linker, HMR client would also be a module.
    // For now, we append it as a global script side-effect.
    chunk_content.push_str(&format!(r#"
(function() {{
    const chunkPath = "{}";
    const ws = new WebSocket("ws://" + location.host + "/ws");
    ws.onmessage = (e) => {{
        const msg = JSON.parse(e.data);
        if (msg.type === "reload" && msg.chunk === chunkPath) {{
            console.log("[Nexus] Reloading", chunkPath);
            location.reload();
        }}
    }};
}})();
"#, path));

    // 3. Emit Wrapped Modules
    for id in sorted_ids {
        if let Some(module) = graph.modules.get(id.0) {
            // A. Transform Imports (ESM -> CJS)
            let transformed_source = transform_cjs(&module.source, &module.path);
            
            // B. Wrap in Registry
            // __nexus_register__("path", function(require, module, exports) { ... })
            chunk_content.push_str(&format!(
                "__nexus_register__(\"{}\", function(require, module, exports) {{\n// Source: {}\n{}\n}});\n",
                module.path,
                module.path,
                transformed_source
            ));
        }
    }
    
    // 4. Bootstrap Entry
    if let Some(entry_module) = graph.modules.get(root_id.0) {
         chunk_content.push_str(&format!("__nexus_require__(\"{}\");", entry_module.path));
    }
    
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/javascript".parse().unwrap());
    
    (StatusCode::OK, headers, chunk_content).into_response()
}


async fn handle_ws(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: AppState) {
    let mut rx = state.hmr_tx.subscribe();
    
    while let Ok(msg) = rx.recv().await {
        for path in msg.paths {
            let payload = format!(r#"{{"type":"reload","chunk":"{}"}}"#, path);
            if socket.send(axum::extract::ws::Message::Text(payload)).await.is_err() {
                return;
            }
        }
    }
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
