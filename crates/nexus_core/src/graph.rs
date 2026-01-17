pub type ModuleId = usize;

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
        let id = self.modules.len();
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
        if from >= self.modules.len() || to >= self.modules.len() {
            return Err("ModuleId out of bounds".to_string());
        }
        if from == to {
            return Err("Self-dependency not allowed".to_string());
        }

        // Check for duplicates
        if self.outgoing_edges[from].contains(&to) {
            return Ok(()); // Already exists
        }

        self.outgoing_edges[from].push(to);
        self.incoming_edges[to].push(from);
        Ok(())
    }

    pub fn update_source(&mut self, id: ModuleId, new_source: &str) {
        if let Some(module) = self.modules.get_mut(id) {
            module.source = new_source.to_string();
            module.version += 1;
        }
    }

    pub fn get_version(&self, id: ModuleId) -> Option<u64> {
        self.modules.get(id).map(|m| m.version)
    }

    pub fn get_dependencies(&self, id: ModuleId) -> Option<&Vec<ModuleId>> {
        self.outgoing_edges.get(id)
    }

    pub fn get_dependents(&self, id: ModuleId) -> Option<&Vec<ModuleId>> {
        self.incoming_edges.get(id)
    }

    // Helper for integration: find ID by path
    pub fn find_by_path(&self, path: &str) -> Option<ModuleId> {
        self.modules.iter().find(|m| m.path == path).map(|m| m.id)
    }
}
