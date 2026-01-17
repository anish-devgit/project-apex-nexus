use nexus_core::graph::ModuleGraph;

#[test]
fn test_chunk_linearization() {
    let mut graph = ModuleGraph::new();

    // Graph: A -> B -> C
    // A depends on B
    // B depends on C
    
    // Dependencies should be emitted first: [C, B, A]
    
    let c_id = graph.add_module("c.js", "const c = 3;");
    
    let b_id = graph.add_module("b.js", "import './c.js';\nconst b = 2;");
    graph.add_dependency(b_id, c_id).unwrap();
    
    let a_id = graph.add_module("a.js", "import './b.js';\nconst a = 1;");
    graph.add_dependency(a_id, b_id).unwrap();
    
    let chunk_order = graph.linearize(a_id);
    
    assert_eq!(chunk_order.len(), 3);
    assert_eq!(chunk_order[0], c_id, "C should be first (leaf)");
    assert_eq!(chunk_order[1], b_id, "B should be second");
    assert_eq!(chunk_order[2], a_id, "A should be last (root)");
}

#[test]
fn test_chunk_cycle_handling() {
     let mut graph = ModuleGraph::new();
     
     // Cycle: A <-> B
     let a_id = graph.add_module("a.js", "");
     let b_id = graph.add_module("b.js", "");
     
     graph.add_dependency(a_id, b_id).unwrap();
     graph.add_dependency(b_id, a_id).unwrap();
     
     // Linearize from A
     // Expect A, B (in some valid order, not infinite loop)
     let order = graph.linearize(a_id);
     
     assert_eq!(order.len(), 2);
     assert!(order.contains(&a_id));
     assert!(order.contains(&b_id));
}
