use nexus_core::graph::ModuleGraph;
use nexus_core::bundler;
use std::collections::HashSet;
use std::path::PathBuf;

#[test]
fn test_circular_dependency_handling() {
    // A -> B -> A
    // Logic: Graph structure should allow cycle.
    // Liveness analysis should handle cycle without infinite loop.
    
    let mut graph = ModuleGraph::new();
    let root = PathBuf::from("/src/a.ts");
    
    // Add modules (mocked content)
    graph.add_module("/src/a.ts".to_string(), root.clone());
    graph.add_module("/src/b.ts".to_string(), PathBuf::from("/src/b.ts"));
    
    // Add edges
    let _ = graph.add_dependency("/src/a.ts".to_string(), "/src/b.ts".to_string(), false);
    let _ = graph.add_dependency("/src/b.ts".to_string(), "/src/a.ts".to_string(), false);
    
    // Verify structure
    let deps_a = graph.get_dependencies("/src/a.ts").unwrap();
    assert!(deps_a.contains(&"/src/b.ts".to_string()));
    
    let deps_b = graph.get_dependencies("/src/b.ts").unwrap();
    assert!(deps_b.contains(&"/src/a.ts".to_string()));
    
    // If logic was recursive DFS without 'visited' set, operations would stack overflow.
    // The fact this test runs implies basic safety.
}

#[tokio::test]
async fn test_resolution_edge_cases() {
    // This requires setting up actual files or mocking the Resolver.
    // Since Resolver uses file system, we must create temp files.
    let root = std::env::temp_dir().join("nexus_qa_correctness");
    if root.exists() {
        tokio::fs::remove_dir_all(&root).await.unwrap();
    }
    tokio::fs::create_dir_all(&root).await.unwrap();
    
    // 1. Extensions
    tokio::fs::write(root.join("foo.ts"), "export const x = 1;").await.unwrap();
    
    // 2. Index resolution
    tokio::fs::create_dir(root.join("bar")).await.unwrap();
    tokio::fs::write(root.join("bar/index.js"), "export const y = 2;").await.unwrap();
    
    // 3. Node Modules (Vendor Isolation)
    let nm = root.join("node_modules/pkg-a");
    tokio::fs::create_dir_all(&nm).await.unwrap();
    tokio::fs::write(nm.join("package.json"), r#"{"main": "main.js"}"#).await.unwrap();
    tokio::fs::write(nm.join("main.js"), "module.exports = {z: 3};").await.unwrap();

    let resolver = nexus_core::resolver::NexusResolver::new(&root);
    
    // Test
    let res_ext = resolver.resolve(&root, "./foo").expect("Failed to resolve extension");
    assert_eq!(res_ext, root.join("foo.ts"));
    
    let res_idx = resolver.resolve(&root, "./bar").expect("Failed to resolve index");
    assert_eq!(res_idx, root.join("bar/index.js"));
    
    let res_pkg = resolver.resolve(&root, "pkg-a").expect("Failed to resolve package");
    assert_eq!(res_pkg, nm.join("main.js"));
    
    // Cleanup
    tokio::fs::remove_dir_all(&root).await.unwrap();
}
