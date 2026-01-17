use std::collections::{HashSet, VecDeque, HashMap};
use std::path::{Path, PathBuf};
use crate::resolver::NexusResolver;
use crate::compiler;
use crate::parser::{extract_dependencies_detailed, transform_cjs};
use crate::runtime::NEXUS_RUNTIME_JS;

struct BuildNode {
    id: String, // Virtual Path (e.g. /src/utils.ts)
    fs_path: PathBuf,
    code: String, // Compiled JS
    is_vendor: bool,
    imports: HashMap<String, String>, // Import Specifier -> Resolved Virtual ID
    sync_deps: Vec<String>, // Resolved Virtual IDs
    async_deps: Vec<String>, // Resolved Virtual IDs
    css: Option<String>,
    asset: Option<(String, Vec<u8>)>,
}

struct Chunk {
    name: String,
    modules: Vec<String>, // List of virtual IDs
    is_entry: bool,
}

pub async fn build(root_dir: &str) -> std::io::Result<()> {
    tracing::info!("Starting Production Build with Code Splitting...");
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

        if ext != "css" && !matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "wasm") {
            let deps = extract_dependencies_detailed(&compiled.code, &virtual_id);
            for (spec, is_dynamic) in deps {
                if let Ok(resolved) = resolver.resolve(&current_path.parent().unwrap(), &spec) {
                    let resolved_vid = normalize_id(&resolved);
                    
                    imports_map.insert(spec, resolved_vid.clone());

                    if is_dynamic {
                        async_deps.push(resolved_vid.clone());
                    } else {
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
        });
    }

    // 4. Partitioning / Chunking
    // Strategy:
    // - Walk Sync graph from Entry -> Main Chunk
    // - Identify Async Edges (from node inside Main Chunk or Async Chunk).
    // - Each Async Target starts a new Chunk.
    // - Shared Logic: If a node is reachable from entry and async, keep in entry?
    //   If reachable from multiple async, put in shared?
    //   Simple MVP:
    //     - Assign every node to 'Entry' initially if reachable sync.
    //     - Then find Async roots. Assign their subgraphs to Async chunks.
    //     - BUT "No duplicate code". Only visit *unvisited* nodes?
    //     - Yes. Code Splitting: if module already loaded by Entry, Async chunk shouldn't include it.
    //     - So: 1. BFS Entry (Sync). Mark all as Main Bundle.
    //           2. Identify Async Deps from Main Bundle nodes. Queue them as Async Roots.
    //           3. Process Async Roots. BFS (Sync) from them.
    //              If node already assigned (to Main or other Async), LINK it (dep) but DON'T include code.
    //              If node unassigned, assign to current Async Chunk.
    //     - This creates a waterfall.
    
    let mut module_chunk_map: HashMap<String, String> = HashMap::new(); // mod_id -> chunk_name
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut chunk_queue = VecDeque::new();
    
    // Entry Chunk
    chunk_queue.push_back((entry_virtual_id.clone(), "main.js".to_string(), true));
    
    // We iterate chunks.
    // Be careful: async deps can appear ANYWHERE.
    // We need to scan assigned modules for async edges to spawn new chunks.
    // Better: Global Queue of (root_mod, chunk_name).
    
    while let Some((root_id, chunk_name, is_entry)) = chunk_queue.pop_front() {
        // Create Chunk
        let mut chunk_modules = Vec::new();
        let mut bfs = VecDeque::new();
        bfs.push_back(root_id);
        
        while let Some(curr) = bfs.pop_front() {
            if module_chunk_map.contains_key(&curr) {
                // Already in a chunk.
                // If in THIS chunk, ignore.
                // If in OTHER chunk, it's a shared dep (already loaded or parallel).
                // Existing presence implies we don't need to bundle it again here.
                continue;
            }
            
            // Assign to this chunk
            module_chunk_map.insert(curr.clone(), chunk_name.clone());
            chunk_modules.push(curr.clone());
            
            if let Some(node) = nodes.get(&curr) {
                // Follow SYNC deps
                for dep in &node.sync_deps {
                    bfs.push_back(dep.clone());
                }
                
                // Identify ASYNC deps -> Spawn new chunks
                for async_dep in &node.async_deps {
                    if !module_chunk_map.contains_key(async_dep) {
                        // Create name: chunk-[hash/id].js
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
    
    // 5. Vendor Extraction
    // Move "is_vendor" modules from their chunks to "vendor.js", UNLESS it breaks strict async isolation?
    // Actually, "vendor.js" usually loaded upfront.
    // If we move ALL vendors to `vendor.js` and load it in index.html, it's available globally.
    // So we can safely move any module with `is_vendor=true` to `vendor.js`.
    
    let mut vendor_modules = Vec::new();
    
    // Filter out vendors from chunks
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
    // Note: This naive vendor extraction removes dupes if same vendor in multiple async chunks.
    
    // 6. Build Mapping for Runtime
    // Map: Async Module ID -> Chunk Name
    // We iterate all nodes. If node matches an Async Root target, we put key.
    // Async Roots were those referenced in `async_deps`.
    // Actually, simple map: Any module whose Chunk is NOT main?
    // No, `__nexus_import__("/src/foo")` needs to know which file to fetch.
    // If `/src/foo` is in `chunk-foo.js`. Map: `"/src/foo": "/assets/chunk-foo.js"`.
    // If `/src/bar` is ALSO in `chunk-foo.js`. Map: `"/src/bar": "/assets/chunk-foo.js"`.
    // So map ALL modules in async chunks?
    // YES.
    
    let mut nexus_chunk_map = HashMap::new();
    for chunk in &chunks {
        if !chunk.is_entry {
            for mod_id in &chunk.modules {
                nexus_chunk_map.insert(mod_id.clone(), format!("/assets/{}", chunk.name));
            }
        }
    }
    // Also include vendors? Vendors are loaded via script tag generally (vendor.js).
    // If an async chunk depends on a vendor not yet loaded... but vendors are global.
    // We assume vendor.js loaded.
    
    // 7. Emit Bundles
    let mut css_bundle = String::new();
    
    // Assets & CSS aggregation
    for node in nodes.values() {
        if let Some(css) = &node.css {
            css_bundle.push_str(css);
            css_bundle.push('\n');
        }
        if let Some((name, data)) = &node.asset {
            tokio::fs::write(dist.join(name), data).await?;
        }
    }

    // Vendor Bundle
    let mut vendor_code = String::new();
    vendor_code.push_str(NEXUS_RUNTIME_JS);
    vendor_code.push('\n');
    for vid in &vendor_modules {
        if let Some(node) = nodes.get(vid) {
            let transformed = transform_cjs(&node.code, &node.id, &node.imports);
            vendor_code.push_str(&format!(
                "__nexus_register__(\"{}\", function(require, module, exports) {{\n{}\n}});\n",
                node.id, transformed
            ));
        }
    }

    // Chunks
    for chunk in chunks {
        let mut code = String::new();
        for mid in &chunk.modules {
            if let Some(node) = nodes.get(mid) {
                let transformed = transform_cjs(&node.code, &node.id, &node.imports);
                code.push_str(&format!(
                    "__nexus_register__(\"{}\", function(require, module, exports) {{\n{}\n}});\n",
                    node.id, transformed
                ));
            }
        }
        
        if chunk.is_entry {
            // Inject Chunk Map
            if !nexus_chunk_map.is_empty() {
                let map_json = serde_json::to_string(&nexus_chunk_map).unwrap();
                code.push_str(&format!("\nwindow.__nexus_chunk_map__ = {};\n", map_json));
            }
            // Bootstrap
            code.push_str(&format!("\n__nexus_require__(\"{}\");\n", entry_virtual_id));
        }
        
        tokio::fs::write(assets_dir.join(&chunk.name), code).await?;
    }
    
    tokio::fs::write(assets_dir.join("vendor.js"), vendor_code).await?;
    tokio::fs::write(assets_dir.join("style.css"), css_bundle).await?;
    
    // 8. HTML
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
