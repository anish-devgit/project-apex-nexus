use std::collections::{HashSet, VecDeque, HashMap};
use std::path::{Path, PathBuf};
use crate::resolver::NexusResolver;
use crate::compiler;
use crate::parser::{analyze_module, transform_tree_shake, transform_cjs, ImportInfo};
use crate::runtime::NEXUS_RUNTIME_JS;

struct BuildNode {
    id: String, // Virtual Path (e.g. /src/utils.ts)
    fs_path: PathBuf,
    code: String, // Compiled JS
    is_vendor: bool,
    imports: HashMap<String, String>, // Import Source -> Resolved Virtual ID
    sync_deps: Vec<String>, // Resolved Virtual IDs
    async_deps: Vec<String>, // Resolved Virtual IDs
    css: Option<String>,
    asset: Option<(String, Vec<u8>)>,
    exports: Vec<String>,
    import_info: Vec<ImportInfo>,
}

struct Chunk {
    name: String,
    modules: Vec<String>, // List of virtual IDs
    is_entry: bool,
}

pub async fn build(root_dir: &str) -> std::io::Result<()> {
    tracing::info!("Starting Production Build with Tree Shaking...");
    let root = Path::new(root_dir);
    let dist = root.join("dist");
    let assets_dir = dist.join("assets");

    // 1. Clean & Create dist
    if dist.exists() {
        tokio::fs::remove_dir_all(&dist).await?;
    }
    tokio::fs::create_dir_all(&assets_dir).await?;

    // 2. Resolve Entry
    let resolver = NexusResolver::new(root);
    let entry_candidates = vec!["./src/main.tsx", "./src/index.tsx", "./src/main.js", "./src/index.js"];
    let mut entry_abs = PathBuf::new();
    let mut found_entry = false;

    for c in entry_candidates {
        if let Ok(p) = resolver.resolve(root, c) {
            entry_abs = p;
            found_entry = true;
            break;
        }
    }

    if !found_entry {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Entry point not found"));
    }

    // 3. Build Graph
    let mut nodes: HashMap<String, BuildNode> = HashMap::new();
    let mut queue = VecDeque::new();
    let mut visited_paths = HashSet::new();

    // Canonical ID for entry
    let entry_str = entry_abs.to_string_lossy().to_string();
    queue.push_back(entry_abs.clone());
    visited_paths.insert(entry_str);

    let normalize_id = |p: &Path| -> String {
        let s = p.to_string_lossy().to_string();
        let rel = s.replace(root.to_string_lossy().as_ref(), "").replace("\\", "/");
        if rel.starts_with('/') { rel } else { format!("/{}", rel) }
    };
    
    let entry_virtual_id = normalize_id(&entry_abs);

    while let Some(current_path) = queue.pop_front() {
        let virtual_id = normalize_id(&current_path);

        if nodes.contains_key(&virtual_id) {
            continue;
        }

        let bytes = tokio::fs::read(&current_path).await?;
        let ext = current_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let is_vendor = current_path.to_string_lossy().contains("node_modules");

        // Compile
        let compiled = match ext {
             "css" => {
                 let text = String::from_utf8_lossy(&bytes);
                 compiler::compile_css(&text, &virtual_id, true)
             },
             "png" | "jpg" | "jpeg" | "gif" | "svg" | "wasm" | "json" => {
                 compiler::compile_asset(&bytes, &virtual_id, true)
             },
             _ => {
                 let text = String::from_utf8_lossy(&bytes);
                 compiler::compile(&text, &virtual_id, true)
             }
        };

        // Extract Deps
        let mut sync_deps = Vec::new();
        let mut async_deps = Vec::new();
        let mut imports_map = HashMap::new();
        let mut exports = Vec::new();
        let mut import_info = Vec::new();

        if ext != "css" && !matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "wasm") {
            let (ex, im) = analyze_module(&compiled.code, &virtual_id);
            exports = ex;
            import_info = im;
            
            for info in &import_info {
                if let Ok(resolved) = resolver.resolve(&current_path.parent().unwrap(), &info.source) {
                    let resolved_vid = normalize_id(&resolved);
                    
                    imports_map.insert(info.source.clone(), resolved_vid.clone());

                    if info.is_dynamic {
                        async_deps.push(resolved_vid.clone());
                    } else {
                        // is_star (export * or import *) is treated as sync dependency usually
                        sync_deps.push(resolved_vid.clone());
                    }

                    let res_str = resolved.to_string_lossy().to_string();
                    if !visited_paths.contains(&res_str) {
                        visited_paths.insert(res_str);
                        queue.push_back(resolved);
                    }
                }
            }
        }

        nodes.insert(virtual_id.clone(), BuildNode {
            id: virtual_id,
            fs_path: current_path,
            code: compiled.code,
            is_vendor,
            imports: imports_map,
            sync_deps,
            async_deps,
            css: compiled.css,
            asset: compiled.asset,
            exports,
            import_info,
        });
    }

    // 4. Liveness Analysis (Mark & Sweep)
    let mut used_exports: HashMap<String, HashSet<String>> = HashMap::new();
    let mut live_modules = VecDeque::new();
    let mut visited_live = HashSet::new();

    // Start with Entry
    live_modules.push_back(entry_virtual_id.clone());
    visited_live.insert(entry_virtual_id.clone());

    while let Some(mid) = live_modules.pop_front() {
        if let Some(node) = nodes.get(&mid) {
            // Process Imports
            for info in &node.import_info {
                if let Some(resolved_id) = node.imports.get(&info.source) {
                    // Mark target module as Live
                    if !visited_live.contains(resolved_id) {
                        visited_live.insert(resolved_id.clone());
                        live_modules.push_back(resolved_id.clone());
                    }

                    // Mark explicitly requested exports as Used
                    let entry_set = used_exports.entry(resolved_id.clone()).or_default();
                    for spec in &info.specifiers {
                        entry_set.insert(spec.clone());
                    }
                    
                    // If import * or export *, mark ALL as used?
                    // Safe logic: If I `import * as ns`, I might use any key. 
                    // So we must verify ALL exports.
                    // Also `export * from 'mod'` usage is propagated later.
                    // But here, if we encounter `import *`, we optimistically mark all.
                    if info.is_star && !info.specifiers.is_empty() { 
                         // `import * as ns` (specifier is `ns`? No, visitor logic) 
                         // Visitor: `import *` sets `is_star=true` and NO specifiers?
                         // Re-check parser.rs:
                         // ImportNamespaceSpecifier -> `is_star = true`. specifiers empty?
                         // "specifiers.push(s.imported.name())" was logic for ImportSpecifier.
                         // ImportNamespaceSpecifier logic: `is_star=true`. Specifiers list left empty?
                         // Yes.
                         // So if `is_star`, we treat it as "Uses Everything".
                         if let Some(target) = nodes.get(resolved_id) {
                             for e in &target.exports {
                                 entry_set.insert(e.clone());
                             }
                         }
                    }
                }
            }
        }
    }

    // Propagate `export *` usage
    // If A uses `x` from B, and B has `export * from C`.
    // If B doesn't export `x` directly, `x` might come from C.
    // So we add `x` to C.used.
    let mut changed = true;
    while changed {
        changed = false;
        let mut updates: Vec<(String, String)> = Vec::new();
        
        for (mid, node) in &nodes {
            if let Some(used) = used_exports.get(mid) {
                // Check if any used symbol is NOT in local exports (handled by export *) 
                // OR simpler: Just propagate ALL used symbols to all `export *` children.
                // This is over-approximation but safe.
                for info in &node.import_info {
                     // export * from ... -> is_star=true, specifiers empty?
                     // Visitor: `visit_export_all_declaration` -> `is_star=true`, specifiers empty.
                     if info.is_star && info.specifiers.is_empty() {
                         if let Some(child_id) = node.imports.get(&info.source) {
                             // Propagate all used symbols
                             for sym in used {
                                 updates.push((child_id.clone(), sym.clone()));
                             }
                         }
                     }
                }
            }
        }
        
        for (child, sym) in updates {
            let entry = used_exports.entry(child).or_default();
            if entry.insert(sym) {
                changed = true;
            }
        }
    }

    // 5. Partitioning / Chunking
    let mut module_chunk_map: HashMap<String, String> = HashMap::new(); 
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut chunk_queue = VecDeque::new();
    
    // Entry Chunk
    chunk_queue.push_back((entry_virtual_id.clone(), "main.js".to_string(), true));
    
    while let Some((root_id, chunk_name, is_entry)) = chunk_queue.pop_front() {
        let mut chunk_modules = Vec::new();
        let mut bfs = VecDeque::new();
        bfs.push_back(root_id);
        
        while let Some(curr) = bfs.pop_front() {
            if module_chunk_map.contains_key(&curr) { continue; }
            
            module_chunk_map.insert(curr.clone(), chunk_name.clone());
            chunk_modules.push(curr.clone());
            
            if let Some(node) = nodes.get(&curr) {
                for dep in &node.sync_deps {
                    bfs.push_back(dep.clone());
                }
                for async_dep in &node.async_deps {
                    if !module_chunk_map.contains_key(async_dep) {
                        let name = format!("chunk-{}.js", async_dep.replace("/", "-").trim_start_matches('-'));
                        chunk_queue.push_back((async_dep.clone(), name, false));
                    }
                }
            }
        }
        
        if !chunk_modules.is_empty() {
             chunks.push(Chunk {
                 name: chunk_name,
                 modules: chunk_modules,
                 is_entry
             });
        }
    }
    
    // 6. Vendor Extraction
    let mut vendor_modules = Vec::new();
    for chunk in &mut chunks {
        let (vendors, app): (Vec<_>, Vec<_>) = chunk.modules.drain(..).partition(|id| {
            nodes.get(id).map(|n| n.is_vendor).unwrap_or(false)
        });
        chunk.modules = app;
        for v in vendors {
            if !vendor_modules.contains(&v) {
                vendor_modules.push(v);
            }
        }
    }

    // 7. Build Mapping for Runtime
    let mut nexus_chunk_map = HashMap::new();
    for chunk in &chunks {
        if !chunk.is_entry {
            for mod_id in &chunk.modules {
                nexus_chunk_map.insert(mod_id.clone(), format!("/assets/{}", chunk.name));
            }
        }
    }
    
    // 8. Emit Bundles (With Tree Shaking)
    let mut css_bundle = String::new();
    
    for node in nodes.values() {
        if let Some(css) = &node.css {
            css_bundle.push_str(css);
            css_bundle.push('\n');
        }
        if let Some((name, data)) = &node.asset {
            tokio::fs::write(dist.join(name), data).await?;
        }
    }

    // Helper to process code
    let fallback_set = HashSet::new();
    let process_code = |mid: &str| -> String {
        if let Some(node) = nodes.get(mid) {
            let used = used_exports.get(mid).unwrap_or(&fallback_set);
            
            // 1. Tree Shake
            let shaken = transform_tree_shake(&node.code, &node.id, used);
            
            // 2. Transform CJS
            let transformed = transform_cjs(&shaken, &node.id, &node.imports);
            
            return format!(
                "__nexus_register__(\"{}\", function(require, module, exports) {{\n// Using: {:?}\n{}\n}});\n",
                node.id, used, transformed
            );
        }
        "".to_string()
    };

    let mut vendor_code = String::new();
    vendor_code.push_str(NEXUS_RUNTIME_JS);
    vendor_code.push('\n');
    for vid in &vendor_modules {
        vendor_code.push_str(&process_code(vid));
    }

    for chunk in chunks {
        let mut code = String::new();
        for mid in &chunk.modules {
            code.push_str(&process_code(mid));
        }
        
        if chunk.is_entry {
            if !nexus_chunk_map.is_empty() {
                let map_json = serde_json::to_string(&nexus_chunk_map).unwrap();
                code.push_str(&format!("\nwindow.__nexus_chunk_map__ = {};\n", map_json));
            }
            code.push_str(&format!("\n__nexus_require__(\"{}\");\n", entry_virtual_id));
        }
        
        tokio::fs::write(assets_dir.join(&chunk.name), code).await?;
    }
    
    tokio::fs::write(assets_dir.join("vendor.js"), vendor_code).await?;
    tokio::fs::write(assets_dir.join("style.css"), css_bundle).await?;
    
    // 9. HTML
     let html_path = root.join("index.html");
     let tags = r#"
    <link rel="stylesheet" href="/assets/style.css">
    <script src="/assets/vendor.js"></script>
    <script src="/assets/main.js"></script>
"#;
    if html_path.exists() {
        let mut html = tokio::fs::read_to_string(&html_path).await?;
        if html.contains("</body>") {
            html = html.replace("</body>", &format!("{}</body>", tags));
        } else {
            html.push_str(tags);
        }
         tokio::fs::write(dist.join("index.html"), html).await?;
    } else {
        let html = format!(r#"<!DOCTYPE html><html><body>{}</body></html>"#, tags);
        tokio::fs::write(dist.join("index.html"), html).await?;
    }

    tracing::info!("Build Complete.");
    Ok(())
}
