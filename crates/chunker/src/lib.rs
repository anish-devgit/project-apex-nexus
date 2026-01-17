//! Virtual Chunking Engine - Core innovation of Nexus

/// Generate a virtual chunk ID from module paths
pub fn generate_chunk_id(modules: &[String]) -> String {
    // TODO: Implement hashing strategy (Issue #16)
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    modules.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Concatenate multiple modules into a single virtual chunk
pub fn concatenate_modules(modules: Vec<(&str, &str)>) -> String {
    let mut output = String::new();
    
    for (path, content) in modules {
        output.push_str(&format!("// Module: {}\n", path));
        output.push_str(content);
        output.push_str("\n\n");
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_id_generation() {
        let modules = vec!["a.js".to_string(), "b.js".to_string()];
        let id = generate_chunk_id(&modules);
        assert!(!id.is_empty());
    }

    #[test]
    fn test_module_concatenation() {
        let modules = vec![
            ("a.js", "export const a = 1;"),
            ("b.js", "export const b = 2;"),
        ];
        let chunk = concatenate_modules(modules);
        assert!(chunk.contains("Module: a.js"));
        assert!(chunk.contains("Module: b.js"));
    }
}
