use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use crate::graph::ModuleGraph;

// HMR Message
#[derive(Clone, Debug)]
pub struct HmrMessage {
    pub paths: Vec<String>,
}

pub async fn start_watcher(
    root: String, 
    graph: Arc<RwLock<ModuleGraph>>, 
    tx: broadcast::Sender<HmrMessage>
) {
    let (notif_tx, mut notif_rx) = tokio::sync::mpsc::channel(100);

    // Create watcher
    let mut watcher = RecommendedWatcher::new(move |res| {
        let _ = notif_tx.blocking_send(res);
    }, Config::default()).expect("Failed to create watcher");

    // Watch root
    watcher.watch(Path::new(&root), RecursiveMode::Recursive).expect("Failed to watch root");
    
    tracing::info!("Watcher started on {}", root);

    // Loop
    while let Some(res) = notif_rx.recv().await {
        match res {
            Ok(event) => {
                // Minimal: treat any Modify/Create as reload candidate
                // Filter for .js/.ts files
                let relevant_paths: Vec<_> = event.paths.into_iter()
                    .filter(|p| {
                        let s = p.to_string_lossy();
                        s.ends_with(".js") || s.ends_with(".ts") || s.ends_with(".jsx") || s.ends_with(".tsx")
                    })
                    .collect();

                if relevant_paths.is_empty() {
                    continue;
                }

                // Handle changes
                let graph_lock = graph.read().unwrap();
                let mut affected_roots = std::collections::HashSet::new();

                for path in &relevant_paths {
                    // Convert abs path to virtual path (uri path)
                    // We need to strip root and ensuring starting /
                    // Ideally we reuse the same logic as handle_module
                    // Warning: The graph keys use slashes. OS paths might use backslashes.
                    // We must normalize.
                    let path_lossy = path.to_string_lossy();
                    let relative = if path_lossy.starts_with(&root) {
                        path_lossy.strip_prefix(&root).unwrap_or(&path_lossy)
                    } else {
                        &path_lossy
                    };
                    
                    let normalized = relative.replace('\\', "/");
                    let virt_path = if normalized.starts_with('/') { normalized } else { format!("/{}", normalized) };

                    if let Some(id) = graph_lock.find_by_path(&virt_path) {
                        let roots = graph_lock.find_affected_roots(id);
                        for r in roots {
                            if let Some(m) = graph_lock.modules.get(r.0) {
                                affected_roots.insert(m.path.clone());
                            }
                        }
                    }
                }
                drop(graph_lock); // Release lock

                if !affected_roots.is_empty() {
                    let paths: Vec<String> = affected_roots.into_iter().collect();
                    tracing::info!("File change detected. Reloading: {:?}", paths);
                    let _ = tx.send(HmrMessage { paths });
                    
                    // Note: We are strictly relying on the Browser to fetch the NEW content.
                    // We did NOT update the graph source here in "Minimal" implementation.
                    // Week 6 Requirement: "Call update_source(module)".
                    // If we don't, the Linearization in `handle_chunk` will use OLD source?
                    // YES. `handle_chunk` returns `module.source`.
                    // So we MUST update source here.
                    // But `update_source` requires `&mut graph`.
                    // And reading file.
                    // So we should do:
                    // 1. Drop read lock.
                    // 2. Read contents of changed files.
                    // 3. Acquire write lock.
                    // 4. Update source (and maybe re-parse if we were fancy, but strict scope says "Call update_source").
                    // 5. THEN find affected roots.
                    // Let's refactor loop.
                } 
                
                // --- Correct Loop Implementation ---
                for path in relevant_paths {
                     // Read file
                     if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        // Normalize path
                        let path_lossy = path.to_string_lossy();
                        let relative = if path_lossy.starts_with(&root) {
                            path_lossy.strip_prefix(&root).unwrap_or(&path_lossy)
                        } else {
                            &path_lossy
                        };
                        let normalized = relative.replace('\\', "/");
                        let virt_path = if normalized.starts_with('/') { normalized } else { format!("/{}", normalized) };

                        // Update Graph
                        let mut roots_to_reload = Vec::new();
                        {
                            let mut g = graph.write().unwrap();
                            if let Some(id) = g.find_by_path(&virt_path) {
                                g.update_source(id, &content);
                                
                                // Now find roots (using updated graph structure? No, strictly structure is same if we don't reparse)
                                // We use existing edges.
                                let roots = g.find_affected_roots(id);
                                for r in roots {
                                    if let Some(m) = g.modules.get(r.0) {
                                        roots_to_reload.push(m.path.clone());
                                    }
                                }
                            }
                        }
                        
                        if !roots_to_reload.is_empty() {
                             tracing::info!("Reloading chunks: {:?}", roots_to_reload);
                             let _ = tx.send(HmrMessage { paths: roots_to_reload });
                        }
                     }
                }
            }
            Err(e) => tracing::error!("Watch error: {:?}", e),
        }
    }
}
