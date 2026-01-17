//! Module parser using oxc

use anyhow::Result;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::path::Path;

/// Parse a JavaScript/TypeScript module and extract imports
pub fn parse_module(path: &Path, source: &str) -> Result<Vec<String>> {
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let allocator = Allocator::default();
    let ret = Parser::new(&allocator, source, source_type).parse();

    if !ret.errors.is_empty() {
        anyhow::bail!("Parse errors in {:?}", path);
    }

    // TODO: Extract imports from AST
    // For now, return empty vec (placeholder for Issue #2)
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_module() {
        let source = "export const foo = 'bar';";
        let path = Path::new("test.js");
        let result = parse_module(path, source);
        assert!(result.is_ok());
    }
}
