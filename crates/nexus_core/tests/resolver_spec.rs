use nexus_core::resolver::NexusResolver;
use std::fs;
use std::path::{Path, PathBuf};

// Helper to create temp workspace
fn setup_workspace(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("nexus_tests");
    path.push(name);
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn test_resolve_relative_extension() {
    let root = setup_workspace("resolve_relative");
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    
    // Create main.ts
    fs::write(src.join("main.ts"), "import './utils'").unwrap();
    // Create utils.ts (target)
    fs::write(src.join("utils.ts"), "export const x = 1;").unwrap();
    
    let resolver = NexusResolver::new(&root);
    
    // Resolve ./utils from src/main.ts
    // Note: resolve 'from' usually expects directory. 
    // Is NexusResolver wrapper expecting file or dir?
    // Implementation: "if from.is_file() { from.parent() } else { from }"
    // So passing src/main.ts is fine.
    
    let result = resolver.resolve(&src.join("main.ts"), "./utils");
    assert!(result.is_ok());
    let path = result.unwrap();
    assert_eq!(path, src.join("utils.ts"));
}

#[test]
fn test_resolve_node_modules() {
    let root = setup_workspace("resolve_node_modules");
    let node_modules = root.join("node_modules");
    let pkg_dir = node_modules.join("react");
    fs::create_dir_all(&pkg_dir).unwrap();
    
    // package.json for react
    fs::write(pkg_dir.join("package.json"), r#"{"main": "index.js"}"#).unwrap();
    fs::write(pkg_dir.join("index.js"), "module.exports = {}").unwrap();
    
    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("app.tsx"), "import React from 'react'").unwrap();
    
    let resolver = NexusResolver::new(&root);
    let result = resolver.resolve(&src.join("app.tsx"), "react");
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), pkg_dir.join("index.js"));
}

#[test]
fn test_resolve_index_expansion() {
    let root = setup_workspace("resolve_index");
    let src = root.join("src");
    let comp = src.join("components");
    fs::create_dir_all(&comp).unwrap();
    
    fs::write(src.join("main.ts"), "import './components'").unwrap();
    fs::write(comp.join("index.tsx"), "").unwrap();
    
    let resolver = NexusResolver::new(&root);
    let result = resolver.resolve(&src.join("main.ts"), "./components");
    
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), comp.join("index.tsx"));
}
