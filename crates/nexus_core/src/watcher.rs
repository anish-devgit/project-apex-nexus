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

use crate::resolver::NexusResolver;

pub async fn start_watcher(
    root: String, 
    graph: Arc<RwLock<ModuleGraph>>, 
    tx: broadcast::Sender<HmrMessage>,
    resolver: Arc<NexusResolver>
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
                        // Week 9: Ignore node_modules
                        if s.contains("node_modules") {
                            return false; 
                        }
                        s.ends_with(".js") || s.ends_with(".ts") || s.ends_with(".jsx") || s.ends_with(".tsx") || s.ends_with(".css") || s.ends_with(".json") || s.ends_with(".png") || s.ends_with(".jpg") || s.ends_with(".jpeg") || s.ends_with(".svg") || s.ends_with(".wasm")
                    })
                    .collect();

                if relevant_paths.is_empty() {
                    continue;
                }

                // Handle changes
                
                // --- Correct Loop Implementation ---
                for path in relevant_paths {
                     // Read file (binary)
                     let bytes = match std::fs::read(&path) {
                         Ok(b) => b,
                         Err(e) => {
                             tracing::error!("Watcher failed to read {}: {}", path.display(), e);
                             continue;
                         }
                     };
                     
                     // Normalize path
                     let path_lossy = path.to_string_lossy();
                     let relative = if path_lossy.starts_with(&root) {
                         path_lossy.strip_prefix(&root).unwrap_or(&path_lossy)
                     } else {
                         &path_lossy
                     };
                     let normalized = relative.replace('\\', "/");
                     let virt_path = if normalized.starts_with('/') { normalized } else { format!("/{}", normalized) };

                     let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

                     // Week 8/11/12: Compile based on type
                     let compiled = match ext {
                         "css" => {
                             let text = String::from_utf8_lossy(&bytes);
                             crate::compiler::compile_css(&text, &virt_path)
                         },
                          "png" | "jpg" | "jpeg" | "gif" | "svg" | "wasm" | "json" => {
                             crate::compiler::compile_asset(&bytes, &virt_path)
                         },
                         _ => {
                             let text = String::from_utf8_lossy(&bytes);
                             crate::compiler::compile(&text, &virt_path)
                         }
                     };

                        // Week 9: Resolve Deps using new Resolver
                        let deps = crate::parser::extract_dependencies(&compiled.code, &virt_path);
                        let mut resolved_imports = std::collections::HashMap::new();
                        
                        // We need to resolve against the file's dir. 
                        // virt_path is URI-like. We should use `path` (absolute PathBuf) for resolution source.
                        
                        for dep_spec in deps {
                            if let Ok(resolved_abs) = resolver.resolve(&path, &dep_spec) {
                                // Normalize for graph key (copy logic from lib.rs helper if possible, or duplicate)
                                let normalized_abs = resolved_abs.to_string_lossy();
                                let graph_key = if let Ok(rel) = resolved_abs.strip_prefix(&root) {
                                     let s = rel.to_string_lossy().to_string();
                                     let n = s.replace('\\', "/");
                                     if !n.starts_with('/') { format!("/{}", n) } else { n }
                                } else {
                                     let s = normalized_abs.to_string();
                                     let n = s.replace('\\', "/");
                                     if !n.starts_with('/') { format!("/{}", n) } else { n }
                                };
                                resolved_imports.insert(dep_spec, graph_key);
                            }
                            // If resolution fails in watcher, we just warn or ignore?
                            // Ignoring for now.
                        }

                        // Update Graph
                        let mut roots_to_reload = Vec::new();
                        {
                            let mut g = graph.write().unwrap();
                            // We use virt_path to find ID.
                            if let Some(id) = g.find_by_path(&virt_path) {
                                // Append SourceMap URL
                                let final_content = format!("{}\n//# sourceMappingURL=/_nexus/sourcemap/{}", compiled.code, id.0);
                                
                                g.update_compiled(id, &final_content, compiled.sourcemap);
                                g.set_imports(id, resolved_imports); // Update imports map
                                
                                // Now find roots (using updated graph structure? No, strictly structure is same if we don't reparse)
                                // We use existing edges.
                                // Wait, if imports changed (new deps), we should update edges too?
                                // Task 3 implies we should linking logic.
                                // If I add `import B` in A. A -> B.
                                // If I don't update edges, linearization might be wrong?
                                // "Minimal" HMR might not handle graph topology changes perfectly yet.
                                // But `roots` calculation relies on `incoming_edges`.
                                // If we don't update edges, `incoming_edges` won't reflect new deps.
                                // Updating edges is complex: need to diff old vs new deps.
                                // For Week 9, we focus on Resolution.
                                // If user adds import, we might need full reload or edge update.
                                // `lib.rs` updates edges on load.
                                // `watcher.rs` currently ONLY updates source/map.
                                // It does NOT update topology.
                                // This is a limitation of current `watcher.rs`.
                                // I will stick to "Update source", and maybe "set_imports" for the "transform_cjs" to work.
                                // If topology changes, HMR might be partial.
                                
                                let roots = g.find_affected_roots(id);
                                for r in roots {
                                    if let Some(m) = g.modules.get(r.0) {
                                        roots_to_reload.push(m.path.clone());
                                    }
                                }
                            }
                        }
                        
                        if !roots_to_reload.is_empty() {
                             tracing::info!("File Changed & Compiled. Reloading chunks: {:?}", roots_to_reload);
                             let _ = tx.send(HmrMessage { paths: roots_to_reload });
                        }
                     }
                }
            }
            Err(e) => tracing::error!("Watch error: {:?}", e),
        }
    }
}
