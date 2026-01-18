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
use tower::ServiceExt;

pub mod graph;
use graph::*;
pub mod parser;
use parser::extract_dependencies_detailed;
pub mod compiler;
use compiler::{compile, compile_css};
pub mod bundler;
pub mod watcher;
pub mod resolver;
use resolver::NexusResolver;
pub mod runtime;

// --- DATA STRUCTURES ---

#[derive(Clone)]
struct AppState {
    graph: Arc<RwLock<ModuleGraph>>,
    root_dir: String,
    hmr_tx: tokio::sync::broadcast::Sender<watcher::HmrMessage>,
    resolver: Arc<NexusResolver>,
}

// --- MODULE HANDLER ---

async fn handle_module(
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    handle_module_logic(state, uri).await
}

async fn handle_sourcemap(
    State(state): State<AppState>,
    Path(id): Path<usize>,
) -> Response {
    let graph = state.graph.read().unwrap();
    if let Some(module) = graph.modules.get(id) {
        if let Some(map) = &module.map {
             let mut headers = HeaderMap::new();
             headers.insert("Content-Type", "application/json".parse().unwrap());
             return (StatusCode::OK, headers, map.clone()).into_response();
        }
    }
    (StatusCode::NOT_FOUND, "Sourcemap not found").into_response()
}

// Helper to normalize path specifically for Graph Keys (URI-like)
fn normalize_path_for_graph(path: &std::path::Path) -> String {
    let s = path.to_string_lossy().to_string();
    let normalized = s.replace('\\', "/");
    if !normalized.starts_with('/') {
        format!("/{}", normalized)
    } else {
        normalized
    }
}

async fn handle_module_logic(state: AppState, uri: Uri) -> Response {
    let path_str = uri.path();
    tracing::info!("Intercepted request: {}", path_str);

    // Week 10: Virtual Refresh Runtime
    if path_str == "/__nexus_react_refresh" {
        let runtime_path = std::path::Path::new(&state.root_dir).join("node_modules/react-refresh/runtime.js");
        match tokio::fs::read_to_string(&runtime_path).await {
            Ok(c) => {
                let headers = HeaderMap::new();
                return (StatusCode::OK, headers, c).into_response();
            },
            Err(e) => {
                 tracing::error!("Could not find react-refresh: {}", e);
                 return (StatusCode::INTERNAL_SERVER_ERROR, "react-refresh not found").into_response();
            }
        }
    }

    // FIX 2: Path Sanitization (Still needed for initial entry point from browser)
    // Browser requests http://localhost:3000/src/index.tsx
    // We map this to File System.
    
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
    
    // Determine if vendor
    let is_vendor = abs_path.to_string_lossy().contains("node_modules");

    // Week 12: Binary Reading
    let bytes = match tokio::fs::read(&abs_path).await {
        Ok(b) => b,
        Err(e) => return (StatusCode::NOT_FOUND, format!("File not found: {}", e)).into_response(),
    };

    // Week 12: Raw Asset Serving
    if uri.query() == Some("raw") {
        let mime = mime_guess::from_path(&abs_path).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
        return (StatusCode::OK, headers, bytes).into_response();
    }
    
    // Determine Compiler
    let compiled_code;
    let sourcemap;
    
    // Check extensions
    // Asset extensions: png, jpg, jpeg, gif, svg, wasm, json
    // We can use a helper or simplistic check
    let ext = std::path::Path::new(path_str).extension().and_then(|s| s.to_str()).unwrap_or("");
    
    if is_vendor {
        // Vendor usually JS text
        compiled_code = String::from_utf8_lossy(&bytes).to_string();
        sourcemap = None;
    } else {
        match ext {
            "css" => {
                let text = String::from_utf8_lossy(&bytes);
                let res = compiler::compile_css(&text, path_str, false);
                compiled_code = res.code;
                sourcemap = res.sourcemap;
            },
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "wasm" | "json" => {
                let res = compiler::compile_asset(&bytes, path_str, false);
                compiled_code = res.code;
                sourcemap = res.sourcemap;
            },
            _ => {
                // Default JS/TS
                let text = String::from_utf8_lossy(&bytes);
                let res = compile(&text, path_str, false);
                compiled_code = res.code;
                sourcemap = res.sourcemap;
            }
        }
    }

    // Week 4: Extract Dependencies (from compiled/raw JS)
    let deps = extract_dependencies_detailed(&compiled_code, path_str);

    let final_content;
    let module_id;

    {
        let mut graph = state.graph.write().unwrap();
        
        // Find existing module by path or add new.
        // For the *entry* request, we use the requested path as key.
        // Ideally we should resolve it too to ensure canonical key?
        // But browser requested `/src/index.tsx`.
        let id_opt = graph.find_by_path(path_str);
        
        let current_id = if let Some(id) = id_opt {
            id
        } else {
            graph.add_module(path_str, "")
        };
        module_id = current_id;
        
        // Append SourceMap URL if present
        if sourcemap.is_some() {
            final_content = format!("{}\n//# sourceMappingURL=/_nexus/sourcemap/{}", compiled_code, current_id.0);
        } else {
            final_content = compiled_code;
        }
        
        // Week 10: Append React Refresh Footer
        if !is_vendor && (path_str.ends_with(".tsx") || path_str.ends_with(".jsx")) {
             final_content.push_str(r#"
if (module.hot) {
  window.$RefreshReg$ = (prev, id) => {
    const fullId = module.id + ' ' + id;
    if (window.__NEXUS_REFRESH__) window.__NEXUS_REFRESH__.register(prev, fullId);
  };

  module.hot.accept();

  if (window.__NEXUS_REFRESH__ && !window.__nexus_is_refreshing) {
    window.__nexus_is_refreshing = true;
    setTimeout(() => {
      window.__NEXUS_REFRESH__.performReactRefresh();
      window.__nexus_is_refreshing = false;
    }, 30);
  }
}
"#);
        }
        
        // Update Graph
        graph.update_compiled(current_id, &final_content, sourcemap);
        graph.mark_vendor(current_id, is_vendor);
        
        // Resolve Dependencies
        let mut resolved_imports = std::collections::HashMap::new();
        
        for (dep_spec, is_dynamic) in deps {
            // Week 9: Use Resolver
            match state.resolver.resolve(&abs_path, &dep_spec) {
                Ok(resolved_abs_path) => {
                     // Convert absolute fs path to "virtual" graph path (URI)
                     // If it's inside root_dir, make relative to root.
                     // If outside (e.g. symlink?), we might have issues.
                     // Generally assume inside root or node_modules inside root.
                     
                     let normalized_abs = resolved_abs_path.to_string_lossy(); // normalize slashes?
                     
                     // Create a graph key.
                     // If inside root, strip root prefix.
                     let graph_key = if let Ok(rel) = resolved_abs_path.strip_prefix(&state.root_dir) {
                         normalize_path_for_graph(rel)
                     } else {
                         // Fallback? Or treat as absolute?
                         normalize_path_for_graph(&resolved_abs_path)
                     };
                     
                     resolved_imports.insert(dep_spec.clone(), graph_key.clone());
                     
                     let dep_id = if let Some(id) = graph.find_by_path(&graph_key) {
                         id
                     } else {
                         // Add missing module with empty source (placeholder)
                         graph.add_module(&graph_key, "")
                     };
                     
                     let _ = graph.add_dependency(current_id, dep_id, is_dynamic);
                }
                Err(e) => {
                    tracing::error!("Failed to resolve import '{}' from '{}': {}", dep_spec, path_str, e);
                    // We don't panic here, but linker might fail later if ID missing.
                }
            }
        }
        
        graph.set_imports(current_id, resolved_imports);
        
        let count = graph.modules.len();
        tracing::info!("Graph Node compile update. Total Nodes: {}", count);
    }

    // FIX 3: Header Consistency
    let mut headers = HeaderMap::new();
    headers.insert("X-Apex-Intercept", "true".parse().unwrap());
    headers.insert("Content-Type", "application/javascript".parse().unwrap());

    (StatusCode::OK, headers, final_content).into_response()
}

// --- CHUNK HANDLER ---

async fn handle_chunk(
    State(state): State<AppState>,
    uri: Uri,
) -> impl IntoResponse {
    // 1. Identify Entry Module
    // URL: /_nexus/chunk?entry=/src/main.js
    let query = uri.query().unwrap_or("");
    let mut entry_path = "";
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "entry" {
                entry_path = v;
            }
        }
    }
    
    if entry_path.is_empty() {
         return (StatusCode::BAD_REQUEST, "Missing entry param").into_response();
    }
    
    let decoded_entry = urlencoding::decode(entry_path).unwrap_or(std::borrow::Cow::Borrowed(entry_path));
    
    let graph = state.graph.read().unwrap();
    let entry_id_opt = graph.find_by_path(&decoded_entry);
    
    if entry_id_opt.is_none() {
        return (StatusCode::NOT_FOUND, format!("Entry module not found: {}", decoded_entry)).into_response();
    }
    let entry_id = entry_id_opt.unwrap();

    // 2. Linearize Graph (DFS/BFS Topo Sort)
    let modules = graph.linearize(entry_id);
    
    // 3. Runtime Kernel (Week 7)
    use runtime::NEXUS_RUNTIME_JS;
    
    let mut chunk = String::new();
    chunk.push_str(NEXUS_RUNTIME_JS);
    chunk.push('\n');
    
    // Week 10: Inject React Refresh Runtime
    let rr_path = std::path::Path::new(&state.root_dir).join("node_modules/react-refresh/runtime.js");
    if let Ok(rr_code) = tokio::fs::read_to_string(&rr_path).await {
         chunk.push_str(&format!(
             "__nexus_register__(\"/__nexus_react_refresh\", function(require, module, exports) {{\n{}\n}});\n",
             rr_code
         ));
         
         chunk.push_str(r#"
(function() {
  try {
      const Runtime = __nexus_require__("/__nexus_react_refresh");
      Runtime.injectIntoGlobalHook(window);
      window.$RefreshReg$ = () => {};
      window.$RefreshSig$ = () => (type) => type;
      window.__NEXUS_REFRESH__ = Runtime;
  } catch(e) { console.warn("[Nexus] React Refresh failed to load", e); }
})();
"#);
    }
    
    // 3.5 HMR Client (Week 10)
    chunk.push_str(r#"
// --- HMR Client ---
(function() {
    const socket = new WebSocket("ws://" + window.location.host + "/ws");
    socket.onmessage = async function(event) {
        const msg = JSON.parse(event.data);
        if (msg.type === 'update') {
            console.log("[HMR] Update received", msg.paths);
            
            for (const path of msg.paths) {
                // Check if we can hot update
                const cached = window.__nexus_cache__ && window.__nexus_cache__[path];
                const isAccepted = cached && cached.hot && cached.hot._accepted;
                
                if (isAccepted) {
                    try {
                        const res = await fetch(path);
                        const code = await res.text();
                        
                        // Update Registry
                        // We wrap the code in a factory similar to handle_chunk
                        // Note: We use 'eval' to execute the code in correct scope?
                        // Actually, we pass a new factory function to register.
                        // The factory function string:
                        // "function(require, module, exports) { " + code + " }"
                        // But we can't eval a function easily across scopes without `new Function` or `eval`.
                        
                        // We construct a factory wrapper
                        const factory = new Function("require", "module", "exports", code);
                        
                        window.__nexus_register__(path, factory);
                        
                        // Invalidate Cache
                        delete window.__nexus_cache__[path];
                        
                        // Re-require to execute (and trigger React Refresh registration)
                        window.__nexus_require__(path);
                        
                        console.log("[HMR] Hot Updated: " + path);
                    } catch (e) {
                        console.error("[HMR] Update Failed", e);
                        window.location.reload();
                    }
                } else {
                    console.log("[HMR] Not accepted. Full Reload.", path);
                    window.location.reload();
                    return;
                }
            }
        }
    };
    console.log("[HMR] Connected");
})();
"#);
    chunk.push('\n');

    // 4. Wrap Modules
    for module_id in modules {
        if let Some(module) = graph.modules.get(module_id.0) {
             // 4. A. Transform Imports (Week 7 + 9)
             let wrapped_source = transform_cjs(&module.source, &module.path, &module.imports);
             
             // 4. B. Wrap in Register
             // __nexus_register__("path", function(require, module, exports) { ... })
             // Note: module.path is the Registry Key.
             chunk.push_str(&format!(
                 "__nexus_register__(\"{}\", function(require, module, exports) {{\n{}\n}});\n",
                 module.path, wrapped_source
             ));
        }
    }

    // 5. Bootstrap
    chunk.push_str(&format!("__nexus_require__(\"{}\");\n", decoded_entry));

    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/javascript".parse().unwrap());
    (StatusCode::OK, headers, chunk).into_response()
}

// --- SERVER ---

pub async fn start_server(root: String, port: u16) -> Result<(), std::io::Error> {
    // Week 6: Start Watcher Channel
    let (tx, _) = tokio::sync::broadcast::channel(100);
    
    // Init Resolver
    let resolver = Arc::new(NexusResolver::new(std::path::Path::new(&root)));
    
    // Spawn Watcher
    let watcher_tx = tx.clone();
    let graph = Arc::new(RwLock::new(ModuleGraph::new()));
    
    let watcher_graph = graph.clone();
    let watcher_root = root.clone();
    let server_root = root.clone();
    let watcher_resolver = resolver.clone(); // If watcher needs compilation, it needs resolver too?
    // Watcher logic: "Compile on change".
    // compilation doesn't need resolver.
    // BUT graph updating needs resolution to find deps.
    // So watcher MUST use resolver?
    // Yes.
    
    tokio::spawn(async move {
        watcher::start_watcher(watcher_root, watcher_graph, watcher_tx, watcher_resolver).await;
    });

    let state = AppState {
        graph,
        root_dir: server_root.clone(),
        hmr_tx: tx,
        resolver,
    };

    let serve_dir = ServeDir::new(&root);

    let service = tower::service_fn(move |req: axum::extract::Request| {
        let state = state.clone();
        let serve_dir = serve_dir.clone();
        
        async move {
            let uri = req.uri().clone();
            let path = uri.path();
            
            // Week 9: Handle implicit extensions if we were serving files directly?
            // "Implicit extensons (.ts, .tsx...)"
            // `handle_module_logic` handles specific requests.
            // If browser asks for `/src/App`, we might fail if we don't map it.
            // But usually linker produces correct paths with extensions (resolved).
            // So browser asks for what Linker told it (rewritten paths? No, linker embeds modules).
            // Browser only asks for `/src/index.tsx` (entry) -> Handled by `handle_module`.
            // THEN `handle_chunk` serves the bundle.
            // All other modules are INSIDE bundle. Browser never requests them individually!
            // So `handle_module` is ONLY for the entry point (or HMR updates if we fetch individually).
            
            if path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") || path.ends_with(".jsx") {
                let response = handle_module_logic(state, uri).await;
                Ok::<_, std::io::Error>(response)
            } else if path.starts_with("/_nexus/chunk") {
                 let response = handle_chunk(state, uri).await;
                 Ok::<_, std::io::Error>(response)
            } else {
                let res: Result<axum::response::Response, _> = serve_dir.oneshot(req).await;
                match res {
                    Ok(r) => Ok(r.into_response()),
                    Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response()) 
                }
            }
        }
    });

    let app = Router::new()
        .route("/ws", get(handle_ws))
        .route("/_nexus/sourcemap/:id", get(handle_sourcemap))
        .fallback_service(service)
        .layer(TraceLayer::new_for_http());


    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    tracing::info!("starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service()).await
}
