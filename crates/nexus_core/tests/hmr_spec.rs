use nexus_core::graph::ModuleGraph;

#[test]
fn test_find_affected_roots_simple() {
    let mut graph = ModuleGraph::new();

    // Graph: Main -> Lib -> Utils
    // Main depends on Lib. Lib depends on Utils.
    // If Utils changes, Main should be affected (as it includes Utils content via bundling).
    
    let utils_id = graph.add_module("src/utils.js", "export const u = 1;");
    let lib_id = graph.add_module("src/lib.js", "import './utils.js';");
    let main_id = graph.add_module("src/main.js", "import './lib.js';");

    graph.add_dependency(main_id, lib_id).unwrap();
    graph.add_dependency(lib_id, utils_id).unwrap();

    // Case 1: Change Utils. Who is the root?
    // Reverse traversal: Utils -> Lib -> Main. Main has no incoming (root).
    let roots = graph.find_affected_roots(utils_id);
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0], main_id);

    // Case 2: Change Lib.
    // Reverse: Lib -> Main.
    let roots_lib = graph.find_affected_roots(lib_id);
    assert_eq!(roots_lib.len(), 1);
    assert_eq!(roots_lib[0], main_id);
    
    // Case 3: Change Main.
    let roots_main = graph.find_affected_roots(main_id);
    assert_eq!(roots_main.len(), 1);
    assert_eq!(roots_main[0], main_id);
}

#[test]
fn test_hmr_multiple_roots() {
    let mut graph = ModuleGraph::new();
    
    // Graph:
    // App1 -> Shared
    // App2 -> Shared
    
    let shared_id = graph.add_module("shared.js", "");
    let app1_id = graph.add_module("app1.js", "");
    let app2_id = graph.add_module("app2.js", "");
    
    graph.add_dependency(app1_id, shared_id).unwrap();
    graph.add_dependency(app2_id, shared_id).unwrap();
    
    // Change Shared
    let roots = graph.find_affected_roots(shared_id);
    assert_eq!(roots.len(), 2);
    assert!(roots.contains(&app1_id));
    assert!(roots.contains(&app2_id));
}

#[test]
fn test_hmr_circular() {
     let mut graph = ModuleGraph::new();
     
     // Cycle A <-> B. User requested A (implicitly entry).
     // Wait, graph doesn't know "User requested".
     // But A and B depends on each other.
     // Incoming of A is B.
     // Incoming of B is A.
     // Neither has empty incoming.
     // Our logic says: "No incoming edges -> likely an entry point".
     // In a cycle, NO node satisfies "No incoming edges" from within the cycle.
     // Unless they have incoming from outside?
     // If A <-> B is isolated.
     // `find_affected_roots(A)`:
     // Queue A. Dependents B. Queue B. Visited A.
     // Queue B. Dependents A. Visited B.
     // Loop ends. Roots empty.
     // This is the known limitation of "No Incoming" heuristic for cycles.
     // For Week 6 Minimal: We accept this. 
     // Ideally, we treat Top-Level nodes as roots.
     // But for now, assert empty or handle gracefully.
     // If the test fails, I'll know behavior. Note: I logged "Week 6 Minimal: Just report top-level roots".
     // If I assume users invoke `main` -> `cycle`, then `main` is root.
     
     let a_id = graph.add_module("a.js", "");
     let b_id = graph.add_module("b.js", "");
     
     graph.add_dependency(a_id, b_id).unwrap();
     graph.add_dependency(b_id, a_id).unwrap();
     
     // Case: Main -> A <-> B
     let main_id = graph.add_module("main.js", "");
     graph.add_dependency(main_id, a_id).unwrap();
     
     let roots = graph.find_affected_roots(b_id); // Change B
     // B -> A -> Main. Main is root.
     assert_eq!(roots.len(), 1);
     assert_eq!(roots[0], main_id);
}
