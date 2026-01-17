use nexus_core::bundler;
use std::path::{Path, PathBuf};
use tokio::fs;

#[tokio::test]
async fn test_tree_shaking() {
    let root = std::env::current_dir().unwrap().join("tests/fixtures/shake_app");
    let dist = root.join("dist");
    
    if dist.exists() {
        fs::remove_dir_all(&dist).await.unwrap();
    }
    
    let src = root.join("src");
    fs::create_dir_all(&src).await.unwrap();
    
    // Main App
    fs::write(src.join("index.tsx"), r#"
import { used } from './utils';
import { ConstUsed } from './constants';

use(used);
use(ConstUsed);

function use(x) { console.log(x); }
"#).await.unwrap();

    // Utils - has unused export
    fs::write(src.join("utils.ts"), r#"
export function used() { return "used"; }
export function unused() { return "unused"; }
"#).await.unwrap();

    // Constants - has unused constant
    fs::write(src.join("constants.ts"), r#"
export const ConstUsed = 1;
export const ConstUnused = 2;
"#).await.unwrap();

    // Run Build
    let res = bundler::build(root.to_str().unwrap()).await;
    assert!(res.is_ok(), "Build failed");
    
    // Check main.js
    let main = fs::read_to_string(dist.join("assets/main.js")).await.unwrap();
    
    // Check for used symbols
    assert!(main.contains("function used()"), "Used function missing");
    assert!(main.contains("const ConstUsed = 1"), "Used constant missing");
    
    // Check for unused symbols
    assert!(!main.contains("function unused()"), "Unused function PRESENT (Tree shaking failed)");
    assert!(!main.contains("const ConstUnused = 2"), "Unused constant PRESENT (Tree shaking failed)");
    
    // Manual check for "export" keyword removal (optional, but transformation ensures it)
    // The transformation removes the entire statement if unused.
}
