use nexus_core::graph::ModuleGraph;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn test_large_graph_construction_sim() {
    // Simulate 1000 nodes (Mini-Stress)
    // Full 10k might be slow for CI, but we want to verify algo complexity.
    
    let mut graph = ModuleGraph::new();
    let start = Instant::now();
    
    for i in 0..1000 {
        let p = PathBuf::from(format!("/src/mod_{}.ts", i));
        graph.add_module(format!("/src/mod_{}.ts", i), p);
    }
    
    // Chain dependencies: 0 -> 1 -> 2 ...
    for i in 0..999 {
        let _ = graph.add_dependency(
            format!("/src/mod_{}.ts", i), 
            format!("/src/mod_{}.ts", i+1), 
            false
        );
    }
    
    let duration = start.elapsed();
    println!("Graph construction (1000 nodes linear): {:?}", duration);
    assert!(duration.as_millis() < 500, "Graph construction too slow");
    
    // Traversal check
    let deps = graph.get_dependencies("/src/mod_0.ts");
    assert!(deps.is_some());
}
