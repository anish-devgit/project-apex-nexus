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

// --- DATA STRUCTURES ---

type ModuleId = String;

#[derive(Clone, Debug)]
struct ModuleNode {
    id: ModuleId,
    abs_path: String,
    content: String,
    version: u32,
}

struct ModuleGraph {
    nodes: HashMap<ModuleId, ModuleNode>,
}

impl ModuleGraph {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }
}

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
    let path = uri.path();
    tracing::info!("Intercepted request: {}", path);

    // simple normalization: remove leading slash if present to join with root
    let relative_path = path.strip_prefix('/').unwrap_or(path);
    let abs_path = std::path::Path::new(&state.root_dir).join(relative_path);

    // Read file content
    let content = match tokio::fs::read_to_string(&abs_path).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read file {}: {}", abs_path.display(), e);
            return (StatusCode::NOT_FOUND, "File not found").into_response();
        }
    };

    // Update Graph
    {
        let mut graph = state.graph.write().unwrap();
        // Assuming path as ID for now
        let id = path.to_string();
        
        // Check if exists to increment version, else 1
        let version = if let Some(node) = graph.nodes.get(&id) {
            node.version + 1
        } else {
            1
        };

        let node = ModuleNode {
            id: id.clone(),
            abs_path: abs_path.to_string_lossy().to_string(),
            content: content.clone(),
            version,
        };

        graph.nodes.insert(id, node);
        
        let count = graph.nodes.len();
        tracing::info!("Graph Node Created. Total Nodes: {}", count);
    }

    // Response
    let mut headers = HeaderMap::new();
    headers.insert("X-Apex-Intercept", "True".parse().unwrap());
    headers.insert("Content-Type", "application/javascript".parse().unwrap());

    (StatusCode::OK, headers, content).into_response()
}

// --- SERVER ---

pub async fn start_server(root: String, port: u16) -> Result<(), std::io::Error> {
    let state = AppState {
        graph: Arc::new(RwLock::new(ModuleGraph::new())),
        root_dir: root.clone(),
    };

    // Manual route matching for specific extensions to ensure precedence
    // Or we could use define explicit routes if we knew the full structure, 
    // but the requirement implies catching *any* file with extensions.
    // Axum doesn't support regex in paths easily, so we can use a fallback with manual check? 
    // Or better: use a specialized handler for known routes if possible?
    // User requirement: "ServeDir (static assets)" + "Explicitly intercept... *.ts, *.tsx..."
    // Since ServeDir is a fallback, we need a way to match these extensions BEFORE ServeDir.
    // We can use a wildcard route `/*path` but that conflicts with ServeDir if put carelessly.
    // However, if we put the wildcard route first, it matches everything.
    // Strategy: Use a middleware or a wildcard handler that checks extension and falls back?
    // Better Strategy: `route("/*path", get(handler))` will match everything. 
    // We can make `handler` check extension. If match -> serve module. If not -> delegate to ServeDir?
    // But ServeDir is a Service, not easily called from inside a handler without some work.
    // Alternative: Axum `nest_service` or simply relying on the fact that we can define 
    // routes but standard file serving usually implies arbitrary paths.
    
    // Simplest robust way for "Intercept specific extensions":
    // Define a fallback service that is a custom service.
    // This custom service checks extension. If match -> call `handle_module`. Else -> call `ServeDir`.
    // Actually, axum's routing is exact match or labeled capture. 
    // For "any file ending in .ts", a wildcard `/*key` is needed.
    
    // Let's use `fallback` which handles everything not matched by specific routes.
    // But ServeDir is usually the fallback.
    // So we wrap ServeDir.
    
    // Let's try explicit route matching with `get` using a wildcard and verifying extension inside?
    // BUT we need to support "All other requests go to ServeDir".
    
    // Correct approach using standard axum patterns for SPA/Static+API:
    // 1. Handlers for fixed api endpoints (none here).
    // 2. A fallback service that checks the request.
    
    // Implementation:
    // We will use a `fallback` that points to a function `dispatch`.
    // `dispatch` checks extension.
    // If extension matches -> handle_module
    // Else -> ServeDir
    
    // However, ServeDir is a `tower::Service`. Calling it from a handler is possible but `fallback_service` takes a Service.
    // We can build a Service that does this branching.
    
    // Let's implement `handle_request` that takes the `State` and `Request`.
    // This seems slightly complex to wire up `ServeDir` inside a handler.
    
    // Pragmantic/Simple route:
    // User said "These routes MUST take precedence over ServeDir".
    // Axum routes are ordered? No, they are based on specificity.
    // Generic wildcard `/*path` matches everything.
    
    // Let's try: 
    // `route("/*key", get(maybe_intercept))`
    // `maybe_intercept` checks extension. If yes -> `handle_module`.
    // If no -> We need to invoke ServeDir.
    
    // To make this cleaner, we can use `axum::middleware::from_fn`? 
    // Or just a fallback that does the logic.
    
    // Let's go with: `fallback_service` that is a custom `service_fn`?
    // Or simply:
    // A wildcard route `route("/{*path}", get(dispatch))`
    // And inside `dispatch`, if we want to serve static, we can call ServeDir?
    // Invoking ServeDir manually requires constructing it.
    
    // Let's stick to the simplest Axum 0.7 way:
    // Use `any` route with a wildcard.
    
    let serve_dir = ServeDir::new(&root);

    let app = Router::new()
        // We catch everything with a wildcard
        .route("/{*path}", get(move |path: Path<String>, uri: Uri, state: State<AppState>| async move {
            let p = uri.path();
            if p.ends_with(".ts") || p.ends_with(".tsx") || p.ends_with(".js") || p.ends_with(".jsx") {
                handle_module(state, uri).await.into_response()
            } else {
                // Delegate to ServeDir
                // We need to create a new request or just call the service.
                // Re-creating the request is tricky safely.
                // 
                // Better: 
                // `fallback_service` handles unmatched routes.
                // If we don't have explicit routes, fallback handles everything.
                // So the logic is IN the fallback.
                // But `fallback_service` takes a Service.
                // `service_fn` can create a Service from an async function.
                panic!("Use branching service logic via tower::service_fn or similar seems best")
            }
        }));
        
    // Wait, let's step back.
    // To satisfy "Explicitly intercept ... others go to ServeDir":
    // We can implement a `Tower::Service` wrapper (middleware style) or just use logic in a handler if we can easily invoke ServeDir.
    // Actually, `ServeDir` implements `Service<Request<Body>>`.
    
    // Correct simpler implementation:
    // Use `tower::service_fn` to create a service that checks the URI.
    // If intercept -> call `handle_module` (need to bridge handler to service or just write logic there).
    // Else -> call `ServeDir`.
    
    // Note: `handle_module` needs `State`. Service doesn't extract state automatically like handlers.
    // We have to close over the state.
    
    // Refined Implementation Plan:
    // 1. Define `state` as Arc/Cloneable.
    // 2. Create `serve_dir = ServeDir::new(&root)`.
    // 3. Define a closure/service for the branching.
    
    let service = tower::service_fn(move |req: axum::extract::Request| {
        let state = state.clone();
        let serve_dir = serve_dir.clone();
        
        async move {
            let uri = req.uri().clone();
            let path = uri.path();
            
            if path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") || path.ends_with(".jsx") {
                // We need to call `handle_module`. 
                // Since we are in a Service, we can't easily use the `Handler` trait magic directly without `call`.
                // But we can just duplicate the logic or call a helper.
                // `handle_module` takes `State<AppState>` and `Uri`.
                // Let's just call the logic function directly!
                
                // Helper to bridge the gap
                let response = handle_module_logic(state, uri).await;
                Ok::<_, std::io::Error>(response)
            } else {
                // Call ServeDir
                // ServeDir::call requires a Request.
                // We have ownership of `req`.
                let res = serve_dir.oneshot(req).await;
                // Map error correctly? ServeDir error is Infallible.
                // Wait, ServeDir::Response is NOT axum::Response immediately? 
                // It is http::Response.
                // `oneshot` returns `Result<Response, Error>`.
                match res {
                    Ok(r) => Ok(r.into_response()),
                    Err(e) => {
                        // This should ideally strictly match the return type
                        // But let's assume we map it to our error type or response
                       Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()) 
                    }
                }
            }
        }
    });

    let app = Router::new()
        .fallback_service(service) // EVERYTHING goes through here
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

// Logic extracted for easy calling from Service
async fn handle_module_logic(state: AppState, uri: Uri) -> Response {
    let path = uri.path();
    tracing::info!("Intercepted request: {}", path);

    let relative_path = path.strip_prefix('/').unwrap_or(path);
    let abs_path = std::path::Path::new(&state.root_dir).join(relative_path);

    let content = match tokio::fs::read_to_string(&abs_path).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to read file {}: {}", abs_path.display(), e);
            // If it matches extension but is missing, 404
            return (StatusCode::NOT_FOUND, "File not found").into_response();
        }
    };

    {
        let mut graph = state.graph.write().unwrap();
        let id = path.to_string();
        let version = if let Some(node) = graph.nodes.get(&id) {
            node.version + 1
        } else {
            1
        };

        let node = ModuleNode {
            id: id.clone(),
            abs_path: abs_path.to_string_lossy().to_string(),
            content: content.clone(),
            version,
        };

        graph.nodes.insert(id, node);
        let count = graph.nodes.len();
        tracing::info!("Graph Node Created. Total Nodes: {}", count);
    }

    let mut headers = HeaderMap::new();
    headers.insert("X-Apex-Intercept", "True".parse().unwrap());
    headers.insert("Content-Type", "application/javascript".parse().unwrap());

    (StatusCode::OK, headers, content).into_response()
}

// ... shutdown_signal ...
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
