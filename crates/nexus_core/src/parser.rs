use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_ast::ast::{ModuleDeclaration, ImportDeclarationSpecifier}; // Adjust based on exact structure if needed


use oxc_ast::visit::Visit;
use oxc_ast::ast::*;

struct DependencyVisitor {
    deps: Vec<(String, bool)>, 
}

impl<'a> Visit<'a> for DependencyVisitor {
    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(s) = &expr.source {
            self.deps.push((s.value.to_string(), true));
        }
        // Recurse for arguments
        for arg in &expr.arguments {
             self.visit_expression(arg);
        }
        // Recurse source
        self.visit_expression(&expr.source);
    }

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
         self.deps.push((decl.source.value.to_string(), false));
    }
    
    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        self.deps.push((decl.source.value.to_string(), false));
    }
    
    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source {
             self.deps.push((source.value.to_string(), false));
        }
    }
}

pub fn extract_dependencies(source: &str, path: &str) -> Vec<String> {
    extract_dependencies_detailed(source, path).into_iter().map(|(s, _)| s).collect()
}

pub fn extract_dependencies_detailed(source: &str, path: &str) -> Vec<(String, bool)> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    if !ret.errors.is_empty() {
        tracing::warn!("Parsing errors in {}: {:?}", path, ret.errors);
    }
    
    let mut visitor = DependencyVisitor { deps: Vec::new() };
    visitor.visit_program(&ret.program);
    visitor.deps
}

struct DynamicImportRewriter<'b> {
    imports: &'b std::collections::HashMap<String, String>,
    replacements: Vec<(u32, u32, String)>,
}

impl<'a, 'b> Visit<'a> for DynamicImportRewriter<'b> {
    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
         if let Expression::StringLiteral(s) = &expr.source {
             let source_val = s.value.as_str();
             let resolved = self.imports.get(source_val).cloned().unwrap_or_else(|| source_val.to_string());
             
             let start = expr.span.start;
             let end = expr.span.end;
             
             self.replacements.push((start, end, format!("__nexus_import__(\"{}\")", resolved)));
         }
         
         self.visit_expression(&expr.source);
         for arg in &expr.arguments {
             self.visit_expression(arg);
         }
    }
}

pub fn transform_cjs(source: &str, path: &str, imports: &std::collections::HashMap<String, String>) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    if !ret.errors.is_empty() {
        return source.to_string();
    }

    let program = ret.program;
    let mut replacements: Vec<(u32, u32, String)> = Vec::new();

    // 1. Dynamic Imports (Visitor)
    let mut visitor = DynamicImportRewriter {
        imports,
        replacements: Vec::new(),
    };
    visitor.visit_program(&program);
    replacements.extend(visitor.replacements);

    // 2. Declarations (Loop)
    for stmt in &program.body {
        if let oxc_ast::ast::Statement::ModuleDeclaration(decl) = stmt {
             match &decl.0 {
                ModuleDeclaration::ImportDeclaration(import_decl) => {
                     // import "pkg" -> require("pkg")
                     let start = import_decl.span.start;
                     let end = import_decl.span.end;
                     let source_val = import_decl.source.value.as_str();
                     
                     let resolved = imports.get(source_val).cloned().unwrap_or_else(|| source_val.to_string());
                     
                     if let Some(specifiers) = &import_decl.specifiers {
                         if specifiers.is_empty() {
                             replacements.push((start, end, format!("require(\"{}\");", resolved)));
                         } else {
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
                                          decls.push(format!("const {} = require(\"{}\").{};", local, resolved, imported));
                                     }
                                 }
                             }
                             replacements.push((start, end, decls.join("\n")));
                         }
                     }
                }
                ModuleDeclaration::ExportDefaultDeclaration(export_default) => {
                    let start = export_default.span.start;
                    
                    match &export_default.declaration {
                         oxc_ast::ast::ExportDefaultDeclarationKind::Expression(expr) => {
                              replacements.push((start, expr.span.start, "exports.default = ".to_string()));
                         }
                         _ => {
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
                    let start = export_named.span.start;
                    
                    if let Some(decl) = &export_named.declaration {
                        let mut names = Vec::new();
                        let mut decl_start = start;
                        
                        match decl {
                            oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                                decl_start = var_decl.span.start;
                                for d in &var_decl.declarations {
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
                            replacements.push((start, decl_start, "".to_string()));
                            let defines: Vec<String> = names.into_iter().map(|name| {
                                format!("Object.defineProperty(exports, \"{}\", {{ enumerable: true, get: function() {{ return {}; }} }});", name, name)
                            }).collect();
                            replacements.push((export_named.span.end, export_named.span.end, format!("\n{}", defines.join("\n"))));
                        }
                        
                    } else if !export_named.specifiers.is_empty() {
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

    // Apply
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    
    let mut result = source.to_string();
    for (start, end, text) in replacements {
        result.replace_range(start as usize..end as usize, &text);
    }
    
    result
}

