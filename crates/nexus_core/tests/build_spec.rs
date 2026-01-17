use nexus_core::bundler;
use std::path::{Path, PathBuf};
use tokio::fs;

#[tokio::test]
async fn test_production_build() {
    // Setup temp dir
    let root = std::env::current_dir().unwrap().join("tests/fixtures/build_app");
    let dist = root.join("dist");
    
    // Clean prev result
    if dist.exists() {
        fs::remove_dir_all(&dist).await.unwrap();
    }
    
    // Create minimal app if not exists (or rely on fixtures?)
    // I'll assume fixtures/build_app exists or create it dynamically?
    // Better to create dynamically to be self-contained.
    let src = root.join("src");
    fs::create_dir_all(&src).await.unwrap();
    
    fs::write(src.join("index.tsx"), r#"
import { log } from './utils';
import './style.css';
import data from './data.json';
import logo from './logo.png';

log("Hello " + data.foo);
console.log(logo);
"#).await.unwrap();

    fs::write(src.join("utils.ts"), "export function log(msg: string) { console.log(msg); }").await.unwrap();
    fs::write(src.join("style.css"), "body { color: red; }").await.unwrap();
    fs::write(src.join("data.json"), r#"{ "foo": "bar" }"#).await.unwrap();
    // Dummy png
    fs::write(src.join("logo.png"), vec![0; 100]).await.unwrap(); // < 8KB -> Inline
    
    // Also a large asset
    fs::write(src.join("large.png"), vec![0; 9000]).await.unwrap(); // > 8KB -> Emit
    // Add import for large
    let mut main_content = fs::read_to_string(src.join("index.tsx")).await.unwrap();
    main_content.push_str("\nimport large from './large.png'; console.log(large);");
    fs::write(src.join("index.tsx"), main_content).await.unwrap();

    // Run Build
    let res = bundler::build(root.to_str().unwrap()).await;
    assert!(res.is_ok(), "Build failed");
    
    // Verify Dist
    assert!(dist.exists());
    assert!(dist.join("assets/vendor.js").exists());
    assert!(dist.join("assets/main.js").exists());
    assert!(dist.join("assets/style.css").exists());
    assert!(dist.join("index.html").exists());
    
    // Verify Large Asset Copied
    // We don't know exact name if handled by bundler logic (bundler implementations use filename)
    // bundler.rs implemented logic: `assets/{filename}`.
    assert!(dist.join("assets/large.png").exists());
    
    // Verify Content
    let css = fs::read_to_string(dist.join("assets/style.css")).await.unwrap();
    assert!(css.contains("color: red"));
    
    let main_js = fs::read_to_string(dist.join("assets/main.js")).await.unwrap();
    assert!(main_js.contains("__nexus_register__"));
    assert!(main_js.contains("Hello"));
    assert!(main_js.contains("data:image/png")); // Inlined logo
    assert!(main_js.contains("/assets/large.png")); // URL for large
    
    // Cleanup
    // fs::remove_dir_all(&root).await.unwrap();
}
