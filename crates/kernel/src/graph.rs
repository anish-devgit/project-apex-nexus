//! Dependency graph management

use std::collections::HashMap;
use std::path::PathBuf;

/// In-memory dependency graph
pub struct DependencyGraph {
    /// Map of file path -> list of dependencies
    dependencies: HashMap<PathBuf, Vec<PathBuf>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    /// Add a module and its dependencies to the graph
    pub fn add_module(&mut self, path: PathBuf, deps: Vec<PathBuf>) {
        self.dependencies.insert(path, deps);
    }

    /// Get all modules that depend on the given module
    pub fn get_dependents(&self, path: &PathBuf) -> Vec<PathBuf> {
        self.dependencies
            .iter()
            .filter(|(_, deps)| deps.contains(path))
            .map(|(p, _)| p.clone())
            .collect()
    }

    /// Clear a module from the graph
    pub fn invalidate(&mut self, path: &PathBuf) {
        self.dependencies.remove(path);
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        
        let a = PathBuf::from("/src/a.js");
        let b = PathBuf::from("/src/b.js");
        let c = PathBuf::from("/src/c.js");

        graph.add_module(a.clone(), vec![b.clone()]);
        graph.add_module(b.clone(), vec![c.clone()]);

        let dependents = graph.get_dependents(&b);
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], a);
    }
}
