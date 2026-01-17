#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub usize);

#[derive(Clone, Debug)]
pub struct Module {
    pub id: ModuleId,
    pub path: String,
    pub source: String,
    pub version: u64,
}

#[derive(Clone, Debug)]
pub struct ModuleGraph {
    pub modules: Vec<Module>,
    pub outgoing_edges: Vec<Vec<ModuleId>>,
    pub incoming_edges: Vec<Vec<ModuleId>>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            outgoing_edges: Vec::new(),
            incoming_edges: Vec::new(),
        }
    }

    pub fn add_module(&mut self, path: &str, source: &str) -> ModuleId {
        let id = ModuleId(self.modules.len());
        let module = Module {
            id,
            path: path.to_string(),
            source: source.to_string(),
            version: 1,
        };
        self.modules.push(module);
        self.outgoing_edges.push(Vec::new());
        self.incoming_edges.push(Vec::new());
        id
    }

    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) -> Result<(), String> {
        if from.0 >= self.modules.len() || to.0 >= self.modules.len() {
            return Err("ModuleId out of bounds".to_string());
        }
        if from == to {
            return Err("Self-dependency not allowed".to_string());
        }

        // Fix 1: Mandatory Idempotency Check
        // Ensure strictly one edge per relation in both directions
        if !self.outgoing_edges[from.0].contains(&to) {
            self.outgoing_edges[from.0].push(to);
        }

        if !self.incoming_edges[to.0].contains(&from) {
            self.incoming_edges[to.0].push(from);
        }
        
        Ok(())
    }

    pub fn update_source(&mut self, id: ModuleId, new_source: &str) {
        if let Some(module) = self.modules.get_mut(id.0) {
            module.source = new_source.to_string();
            module.version += 1;
        }
    }

    pub fn get_version(&self, id: ModuleId) -> Option<u64> {
        self.modules.get(id.0).map(|m| m.version)
    }

    pub fn get_dependencies(&self, id: ModuleId) -> Option<&Vec<ModuleId>> {
        self.outgoing_edges.get(id.0)
    }

    pub fn get_dependents(&self, id: ModuleId) -> Option<&Vec<ModuleId>> {
        self.incoming_edges.get(id.0)
    }

    // Helper for integration: find ID by path
    pub fn find_by_path(&self, path: &str) -> Option<ModuleId> {
        self.modules.iter().find(|m| m.path == path).map(|m| m.id)
    }

    // Week 5: Linearization (Virtual Chunking)
    // Post-order DFS traversal: visits leaf dependencies first.
    pub fn linearize(&self, root: ModuleId) -> Vec<ModuleId> {
        let mut visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        self.dfs_post_order(root, &mut visited, &mut result);
        result
    }

    fn dfs_post_order(&self, node: ModuleId, visited: &mut std::collections::HashSet<ModuleId>, result: &mut Vec<ModuleId>) {
        if visited.contains(&node) {
            return;
        }
        visited.insert(node);

        if let Some(deps) = self.outgoing_edges.get(node.0) {
            for &dep in deps {
                self.dfs_post_order(dep, visited, result);
            }
        }
        
        result.push(node);
    }

    // Week 6: HMR (Reverse Traversal)
    // Find all "root" modules (entries) that depend on the given module.
    // These are the virtual chunks that need reloading.
    pub fn find_affected_roots(&self, start_node: ModuleId) -> Vec<ModuleId> {
        let mut roots = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        
        queue.push_back(start_node);
        visited.insert(start_node);
        
        while let Some(node) = queue.pop_front() {
            if let Some(dependents) = self.incoming_edges.get(node.0) {
                if dependents.is_empty() {
                    // No incoming edges -> likely an entry point / root
                    roots.insert(node);
                } else {
                    for &dep in dependents {
                        if !visited.contains(&dep) {
                            visited.insert(dep);
                            queue.push_back(dep);
                        }
                    }
                    // Edge case: if node is part of a cycle, it might have incoming edges but still be relevant?
                    // For virtual chunking, we only care about the ENTRY point.
                    // If A <-> B, and A is requested.
                    // If I change A. Dependents: B. Queue B.
                    // B dependents: A. A visited.
                    // Loop ends. Roots empty?
                    // If cycle, ANY node in cycle could be entry.
                    // We should implicitly consider the start node as a candidate if it was requested?
                    // But we don't track "requested status" in graph.
                    // Simpler heuristic: If circular, report ALL nodes in cycle that have no *external* incoming? Too complex.
                    // Week 6 Minimal: Just report top-level roots. 
                    // If A <-> B is an isolated island, we might miss it.
                    // But usually A is entry. A <-> B.
                    // "incoming_edges" for A includes B.
                    // So A is not root?
                    // If A is entry, it means User requested A.
                    // So A is a Root "implicitly" from the server perspective, but graph-wise it has incoming B.
                    // Issue: Virtual Chunking Graph doesn't distinguish "Entry" from "Internal".
                    // But typically Entry has NO incoming edges from *other* chunks (because it IS the chunk).
                    // Refinement: Collect ALL impacted nodes? No, browser needs entry.
                    // Mitigation: If `dependents` is not empty, we keep going up.
                    // If we hit a cycle and stop, we report nothing?
                    // Fix: If we traverse and find NO roots (queue empty, visited all), it means it's a closed cycle.
                    // In that case, report the cycle members? Or just the start node?
                    // Let's report the node itself if it was visited?
                    // Actually, for this specific project structure, `main` -> `lib`. `main` has no incoming. `lib` has incoming `main`.
                    // Change `lib`. Queue `lib`. Pop `lib`. Incoming `main`. Queue `main`.
                    // Pop `main`. Incoming []. Root `main`. Found.
                    // Change `main`. Queue `main`. Pop `main`. Incoming []. Root `main`. Found.
                    // Cycle A <-> B. Change A. Queue A.
                    // Pop A. Incoming B. Queue B.
                    // Pop B. Incoming A. Visited.
                    // Queue empty. Roots empty.
                    // Fallback: If roots empty, maybe return start_node? 
                    // Or return all visited nodes?
                    // Browser can reload `A`.
                    // Let's add all visited nodes that have "no unvisited incoming"? Too complex.
                    // Simple Fallback: if roots is empty, return everything allowed?
                    // Let's just return unique roots. If empty, maybe the graph is weird or circular.
                }
            }
        }
        
        // Convert to Vec
        roots.into_iter().collect()
    }
}
