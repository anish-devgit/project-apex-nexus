use std::collections::{HashSet, VecDeque, HashMap};
use std::path::{Path, PathBuf};
use crate::resolver::NexusResolver;
use crate::compiler;
use crate::parser::extract_dependencies;
use crate::runtime::NEXUS_RUNTIME_JS;
use tokio::io::AsyncWriteExt;

pub async fn build(root_dir: &str) -> std::io::Result<()> {
    tracing::info!("Starting Production Build...");
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
        tracing::error!("No entry point found (checked src/main.tsx, src/index.tsx)");
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Entry point not found"));
    }

    tracing::info!("Entry point: {:?}", entry_abs);

    // 3. Traversal
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    
    // Modules storage
    struct BuildModule {
        id: String,
        code: String,
        is_vendor: bool,
    }
    
    let mut modules = Vec::new();
    let mut css_bundle = String::new();
    
    // Initial enqueue
    // We use absolute path as canonical ID
    let entry_str = entry_abs.to_string_lossy().to_string();
    queue.push_back(entry_abs.clone());
    visited.insert(entry_str.clone());

    while let Some(current_path) = queue.pop_front() {
        let path_str = current_path.to_string_lossy().to_string();
        let relative_path = path_str.replace(root.to_string_lossy().as_ref(), "").replace("\\", "/"); 
        // Ensure leading slash for virtual ID
        let virtual_id = if relative_path.starts_with('/') { relative_path.clone() } else { format!("/{}", relative_path) };
        
        let is_vendor = path_str.contains("node_modules");

        // Read
        let bytes = tokio::fs::read(&current_path).await?;
        
        // Compile (is_prod = true)
        let ext = current_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        
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

        // Handle Outputs
        if let Some(css) = compiled.css {
            css_bundle.push_str(&css);
            css_bundle.push('\n');
        }
        
        if let Some((name, data)) = compiled.asset {
            let out = dist.join(&name);
            tokio::fs::write(&out, &data).await?;
        }

        // Store JS Module
        // We Wrap it: __nexus_register__("id", function(require, module, exports) { ... });
        // NOTE: In prod, we optimize `__nexus_register__`.
        // But for MVP, keeping same runtime format is safest.
        let wrapped = format!("__nexus_register__(\"{}\", function(require, module, exports) {{\n{}\n}});\n", virtual_id, compiled.code);
        
        modules.push(BuildModule {
            id: virtual_id.clone(),
            code: wrapped,
            is_vendor
        });

        // Resolve Deps (if JS)
        // Only if it has code (assets might have code too if URL export)
        // But asset deps? No.
        if ext != "css" && !matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "wasm") { 
            // Only extract deps for JS modules
            // For JSON, we compiled to JS (export object). No deps.
            // For normal JS/TS:
            let deps = extract_dependencies(&compiled.code, &virtual_id); // compiled.code is the inner body
            for dep in deps {
                if let Ok(resolved) = resolver.resolve(&current_path.parent().unwrap(), &dep) {
                    let res_str = resolved.to_string_lossy().to_string();
                    if !visited.contains(&res_str) {
                        visited.insert(res_str);
                        queue.push_back(resolved);
                    }
                }
            }
        }
    }

    // 4. Bundle Generation
    let mut vendor_bundle = String::new();
    let mut main_bundle = String::new();

    // Inject Runtime into Vendor
    vendor_bundle.push_str(NEXUS_RUNTIME_JS);
    vendor_bundle.push('\n');

    for m in modules {
        if m.is_vendor {
            vendor_bundle.push_str(&m.code);
        } else {
            main_bundle.push_str(&m.code);
        }
    }

    // Append Entry execution to main
    // We assume the first module added was entry? No, queue order might differ?
    // But we know entry_abs.
    let entry_lossy = entry_abs.to_string_lossy().replace(root.to_string_lossy().as_ref(), "").replace("\\", "/");
    let entry_id = if entry_lossy.starts_with('/') { entry_lossy } else { format!("/{}", entry_lossy) };
    
    main_bundle.push_str(&format!("\n__nexus_require__(\"{}\");\n", entry_id));

    // Write Bundles
    tokio::fs::write(assets_dir.join("vendor.js"), vendor_bundle).await?;
    tokio::fs::write(assets_dir.join("main.js"), main_bundle).await?;
    tokio::fs::write(assets_dir.join("style.css"), css_bundle).await?;

    // 5. HTML Generation
    let html_path = root.join("index.html");
    if html_path.exists() {
        let mut html = tokio::fs::read_to_string(&html_path).await?;
        // Inject tags before </head> or </body>
        // Simple replace for MVP
        let tags = r#"
    <link rel="stylesheet" href="/assets/style.css">
    <script src="/assets/vendor.js"></script>
    <script src="/assets/main.js"></script>
"#;
        if html.contains("</body>") {
            html = html.replace("</body>", &format!("{}</body>", tags));
        } else {
            html.push_str(tags);
        }
        
        tokio::fs::write(dist.join("index.html"), html).await?;
    } else {
        tracing::warn!("index.html not found, generating minimal");
        let html = format!(r#"<!DOCTYPE html><html><body>{}</body></html>"#, r#"
    <link rel="stylesheet" href="/assets/style.css">
    <script src="/assets/vendor.js"></script>
    <script src="/assets/main.js"></script>
"#);
        tokio::fs::write(dist.join("index.html"), html).await?;
    }

    tracing::info!("Build Complete! Output in dist/");
    Ok(())
}
