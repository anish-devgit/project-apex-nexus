use nexus_core::graph::ModuleGraph;

#[test]
fn test_graph_topology_and_invalidation() {
    // 1. Create graph
    let mut graph = ModuleGraph::new();

    // 2. Add modules
    let main_id = graph.add_module("main.js", "import './lib.js';");
    let lib_id = graph.add_module("lib.js", "export const x = 1;");

    // 3. Add dependency: main -> lib
    let res = graph.add_dependency(main_id, lib_id);
    assert!(res.is_ok(), "Failed to add dependency");

    // 4. Assert: main depends on lib, lib has main as dependent
    let deps_main = graph.get_dependencies(main_id).expect("Should have outgoing edges");
    assert!(deps_main.contains(&lib_id), "main should depend on lib");

    let dependents_lib = graph.get_dependents(lib_id).expect("Should have incoming edges");
    assert!(dependents_lib.contains(&main_id), "lib should have main as dependent");

    // 5. Capture lib version (v1)
    let v1 = graph.get_version(lib_id).expect("Should have version");
    assert_eq!(v1, 1, "Initial version should be 1");

    // 6. Call update_source(lib)
    graph.update_source(lib_id, "export const x = 2;");

    // 7. Assert: lib version increments, dependents correct
    let v2 = graph.get_version(lib_id).expect("Should have version");
    assert!(v2 > v1, "Version should increment after update");
    assert_eq!(v2, 2, "Version should be 2");

    let dependents_lib_after = graph.get_dependents(lib_id).expect("Should have incoming edges");
    assert!(dependents_lib_after.contains(&main_id), "lib should still have main as dependent");
}
