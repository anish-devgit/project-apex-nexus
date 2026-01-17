use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::{ModuleDeclaration, ImportDeclarationSpecifier}; // Adjust based on exact structure if needed

pub fn extract_dependencies(source: &str, path: &str) -> Vec<String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    // Log parsing errors but don't fail
    if !ret.errors.is_empty() {
        tracing::warn!("Parsing errors in {}: {:?}", path, ret.errors);
        // We continue to return what we found, or just return partial? 
        // Best effort: inspect the AST we got.
    }

    let program = ret.program;
    let mut deps = Vec::new();

    for stmt in program.body {
        if let oxc_ast::ast::Statement::ModuleDeclaration(decl) = stmt {
            match decl.0 {
                ModuleDeclaration::ImportDeclaration(import_decl) => {
                    deps.push(import_decl.source.value.to_string());
                }
                ModuleDeclaration::ExportAllDeclaration(export_all) => {
                    deps.push(export_all.source.value.to_string());
                }
                ModuleDeclaration::ExportNamedDeclaration(export_named) => {
                    if let Some(source) = export_named.source {
                         deps.push(source.value.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    deps
}
