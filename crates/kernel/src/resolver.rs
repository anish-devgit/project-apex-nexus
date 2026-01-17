//! Module resolution using oxc_resolver

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Resolve a module specifier to an absolute path
pub fn resolve_module(specifier: &str, importer: &Path) -> Result<PathBuf> {
    // TODO: Implement using oxc_resolver (Issue #3)
    // Placeholder implementation
    anyhow::bail!("Resolution not yet implemented for: {}", specifier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_relative() {
        // Placeholder test
        let result = resolve_module("./foo", Path::new("/project/src/index.js"));
        assert!(result.is_err()); // Should fail until implemented
    }
}
