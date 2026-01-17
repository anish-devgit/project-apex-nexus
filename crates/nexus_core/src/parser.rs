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

pub fn transform_cjs(source: &str, path: &str, imports: &std::collections::HashMap<String, String>) -> String {
    // Naive implementation for Week 7 (MVP)
    // In a real implementation we would distinct Source text spans and replace them.
    // However, oxc doesn't have a "Mutation" API easily accessible without re-printing.
    // For Week 7, we will do string replacement based on AST spans? 
    // Or just simple regex for MVP as allowed by "Minimal"?
    // "Implementation Note for Rust: Use oxc_parser to identify ... Use a simple span replacement".
    
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    if !ret.errors.is_empty() {
        return source.to_string(); // Fallback on error
    }

    let mut replacements: Vec<(u32, u32, String)> = Vec::new();
    let program = ret.program;

    for stmt in program.body {
        if let oxc_ast::ast::Statement::ModuleDeclaration(decl) = stmt {
             match decl.0 {
                ModuleDeclaration::ImportDeclaration(import_decl) => {
                     // import "pkg" -> require("pkg")
                     // import x from "./file" -> const x = require("./file").default
                     // Span: import_decl.span
                     let start = import_decl.span.start;
                     let end = import_decl.span.end;
                     let source_val = import_decl.source.value.as_str();
                     
                     // Resolve specifier
                     let resolved = imports.get(source_val).cloned().unwrap_or_else(|| source_val.to_string());
                     
                     if let Some(specifiers) = &import_decl.specifiers {
                         if specifiers.is_empty() {
                             // import "pkg"
                             replacements.push((start, end, format!("require(\"{}\");", resolved)));
                         } else {
                             // import x from "..."
                             // Identify default import vs named.
                             // Week 7 Assumption: "Treat imports as value copies".
                             // import x from "./file" -> const x = require("./file").default
                             
                             let mut decls = Vec::new();
                             for spec in specifiers {
                                 match spec {
                                     ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                         let local = s.local.name.as_str();
                                         decls.push(format!("const {} = require(\"{}\").default;", local, resolved));
                                     }
                                     ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                          let local = s.local.name.as_str();
                                          decls.push(format!("const {} = require(\"{}\");", local, resolved));
                                     }
                                     ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                          let local = s.local.name.as_str();
                                          let imported = s.imported.name().as_str();
                                          // import { x } from "..." -> const x = require("...").x
                                          decls.push(format!("const {} = require(\"{}\").{};", local, resolved, imported));
                                     }
                                 }
                             }
                             replacements.push((start, end, decls.join("\n")));
                         }
                     }
                }
                ModuleDeclaration::ExportDefaultDeclaration(export_default) => {
                    // export default ...
                    // If declaration is expression: exports.default = ...
                    let start = export_default.span.start;
                    let end = export_default.span.end; // This might capture the whole declaration
                    
                    match &export_default.declaration {
                         oxc_ast::ast::ExportDefaultDeclarationKind::Expression(expr) => {
                              // We want to replace "export default " (prefix) with "exports.default = "
                              // But we need to keep the expression.
                              // Simple span replacement of the whole thing:
                              // "exports.default = <expression source>"
                              
                              // We need the source text of the expression to preserve it?
                              // Or simply replace the "export default" keyword part?
                              // `export_default.span` covers the whole statement.
                              // `expr.span` covers the expression.
                              // Replace [start, expr.span.start) with "exports.default = ".
                              replacements.push((start, expr.span.start, "exports.default = ".to_string()));
                         }
                         _ => {
                              // Function declaration etc.
                              // export default function foo() {}
                              // -> exports.default = function foo() {}
                              // similar logic.
                               match &export_default.declaration {
                                   oxc_ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                                       replacements.push((start, f.span.start, "exports.default = ".to_string()));
                                   }
                                   oxc_ast::ast::ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                                       replacements.push((start, c.span.start, "exports.default = ".to_string()));
                                   }
                                   _ => {}
                               }
                         }
                    }
                }
                ModuleDeclaration::ExportNamedDeclaration(export_named) => {
                    // export const x = 1; -> const x = 1; Object.defineProperty...
                    // export { x }; -> Object.defineProperty...
                    
                    let start = export_named.span.start;
                    
                    if let Some(decl) = &export_named.declaration {
                        let mut names = Vec::new();
                        let mut decl_start = start; // Default to start, update based on decl type
                        
                        match decl {
                            oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                                decl_start = var_decl.span.start;
                                for d in &var_decl.declarations {
                                     // Simple Identifier support
                                     if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(id) = &d.id.kind {
                                         names.push(id.name.as_str().to_string());
                                     }
                                }
                            }
                            oxc_ast::ast::Declaration::FunctionDeclaration(f) => {
                                decl_start = f.span.start;
                                if let Some(id) = &f.id {
                                    names.push(id.name.as_str().to_string());
                                }
                            }
                            oxc_ast::ast::Declaration::ClassDeclaration(c) => {
                                decl_start = c.span.start;
                                if let Some(id) = &c.id {
                                    names.push(id.name.as_str().to_string());
                                }
                            }
                            _ => {}
                        }
                        
                        if !names.is_empty() {
                            // Remove "export " keyword
                            replacements.push((start, decl_start, "".to_string()));
                            
                            // Append DefineProperty calls
                            let defines: Vec<String> = names.into_iter().map(|name| {
                                format!("Object.defineProperty(exports, \"{}\", {{ enumerable: true, get: function() {{ return {}; }} }});", name, name)
                            }).collect();
                            
                            replacements.push((export_named.span.end, export_named.span.end, format!("\n{}", defines.join("\n"))));
                        }
                        
                    } else if !export_named.specifiers.is_empty() {
                         // export { x }
                         let mut defines = Vec::new();
                         for spec in &export_named.specifiers {
                             let exported = spec.exported.name().as_str();
                             let local = spec.local.name().as_str();
                             defines.push(format!("Object.defineProperty(exports, \"{}\", {{ enumerable: true, get: function() {{ return {}; }} }});", exported, local));
                         }
                         replacements.push((start, export_named.span.end, defines.join("\n")));
                    }
                }
                _ => {}
             }
        }
    }

    // Apply replacements in reverse order to keep indices valid
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    
    let mut result = source.to_string();
    for (start, end, text) in replacements {
        result.replace_range(start as usize..end as usize, &text);
    }
    
    result
}

