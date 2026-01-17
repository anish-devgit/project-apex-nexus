use nexus_core::bundler;
use std::path::{Path, PathBuf};
use tokio::fs;

#[tokio::test]
async fn test_code_splitting() {
    let root = std::env::current_dir().unwrap().join("tests/fixtures/split_app");
    let dist = root.join("dist");
    
    if dist.exists() {
        fs::remove_dir_all(&dist).await.unwrap();
    }
    
    let src = root.join("src");
    fs::create_dir_all(&src).await.unwrap();
    
    // Main App
    fs::write(src.join("index.tsx"), r#"
import { log } from './utils';

console.log("Main loaded");
log("Sync dep");

// Dynamic Import
setTimeout(() => {
    import('./dynamic').then(mod => {
        mod.run();
    });
}, 100);
"#).await.unwrap();

    // Utils (Sync)
    fs::write(src.join("utils.ts"), "export function log(msg: string) { console.log(msg); }").await.unwrap();
    
    // Dynamic Module
    fs::write(src.join("dynamic.ts"), r#"
import { log } from './utils';
export function run() {
    log("Dynamic loaded!");
}
"#).await.unwrap();

    // Run Build
    let res = bundler::build(root.to_str().unwrap()).await;
    assert!(res.is_ok(), "Build failed");
    
    // Check files
    assert!(dist.join("assets/main.js").exists());
    assert!(dist.join("assets/vendor.js").exists());
    
    // Find Dynamic Chunk
    let mut chunk_found = false;
    let mut entries = fs::read_dir(dist.join("assets")).await.unwrap();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("chunk-") && name.ends_with(".js") {
            chunk_found = true;
            let content = fs::read_to_string(entry.path()).await.unwrap();
            assert!(content.contains("Dynamic loaded!"));
            // Check utility presence? 
            // Utils is used by both Main and Dynamic.
            // Main loaded it first.
            // So Dynamic chunk should NOT duplicate it if our dedup logic works?
            // "If node already assigned... LINK it".
            // Since Main is entry, Utils is assigned to Main.
            // Dynamic chunk should depend on it but not contain it.
            // MVP Check: Check content doesn't have `__nexus_register__("/src/utils.ts"` multiple times across files?
            // Just check chunk exists for now.
        }
    }
    assert!(chunk_found, "Async chunk not generated");
    
    // Check main.js has map
    let main = fs::read_to_string(dist.join("assets/main.js")).await.unwrap();
    assert!(main.contains("__nexus_chunk_map__"));
    assert!(main.contains("chunk-"));
}
