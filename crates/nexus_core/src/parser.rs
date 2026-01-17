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


#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source: String,
    pub specifiers: Vec<String>,
    pub is_dynamic: bool,
    pub is_star: bool,
}

struct AnalysisVisitor {
    exports: Vec<String>,
    imports: Vec<ImportInfo>,
}

impl<'a> Visit<'a> for AnalysisVisitor {
    fn visit_import_expression(&mut self, expr: &ImportExpression<'a>) {
        if let Expression::StringLiteral(s) = &expr.source {
            self.imports.push(ImportInfo {
                source: s.value.to_string(),
                specifiers: Vec::new(),
                is_dynamic: true,
                is_star: false,
            });
        }
        self.visit_expression(&expr.source);
    }

    fn visit_import_declaration(&mut self, decl: &ImportDeclaration<'a>) {
         let source = decl.source.value.to_string();
         let mut specifiers = Vec::new();
         let mut is_star = false;
         
         if let Some(specs) = &decl.specifiers {
             for spec in specs {
                 match spec {
                     ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                         specifiers.push("default".to_string());
                     }
                     ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                         is_star = true;
                     }
                     ImportDeclarationSpecifier::ImportSpecifier(s) => {
                         specifiers.push(s.imported.name().to_string());
                     }
                 }
             }
         }
         
         self.imports.push(ImportInfo {
             source,
             specifiers,
             is_dynamic: false,
             is_star,
         });
    }

    fn visit_export_all_declaration(&mut self, decl: &ExportAllDeclaration<'a>) {
        // export * from "mod"
        // This is effectively an import AND an export.
        // We track it as import with is_star=true.
        // And we should track it as export? "Re-export". 
        // Liveness analysis handles this by following the star.
        self.imports.push(ImportInfo {
            source: decl.source.value.to_string(),
            specifiers: Vec::new(),
            is_dynamic: false,
            is_star: true,
        });
        // We can't list specific exports here without resolving.
    }

    fn visit_export_default_declaration(&mut self, _decl: &ExportDefaultDeclaration<'a>) {
        self.exports.push("default".to_string());
    }

    fn visit_export_named_declaration(&mut self, decl: &ExportNamedDeclaration<'a>) {
        if let Some(source) = &decl.source {
            // Re-export: export { foo } from 'bar'
            let src = source.value.to_string();
            let mut specs = Vec::new();
            for spec in &decl.specifiers {
                 specs.push(spec.local.name().to_string()); // In re-export, local is the IMPORTED name.
                 self.exports.push(spec.exported.name().to_string());
            }
            self.imports.push(ImportInfo {
                source: src,
                specifiers: specs,
                is_dynamic: false,
                is_star: false,
            });
        } else {
            // Regular export: export const x = 1; or export { x };
            if let Some(d) = &decl.declaration {
                match d {
                     oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                         for d in &var_decl.declarations {
                             if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(id) = &d.id.kind {
                                 self.exports.push(id.name.as_str().to_string());
                             }
                         }
                     }
                     oxc_ast::ast::Declaration::FunctionDeclaration(f) => {
                         if let Some(id) = &f.id {
                             self.exports.push(id.name.as_str().to_string());
                         }
                     }
                     oxc_ast::ast::Declaration::ClassDeclaration(c) => {
                         if let Some(id) = &c.id {
                             self.exports.push(id.name.as_str().to_string());
                         }
                     }
                     _ => {}
                }
            }
            for spec in &decl.specifiers {
                self.exports.push(spec.exported.name().to_string());
            }
        }
    }
}

pub fn analyze_module(source: &str, path: &str) -> (Vec<String>, Vec<ImportInfo>) {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    if !ret.errors.is_empty() {
        return (Vec::new(), Vec::new());
    }
    
    let mut visitor = AnalysisVisitor { exports: Vec::new(), imports: Vec::new() };
    visitor.visit_program(&ret.program);
    (visitor.exports, visitor.imports)
}

pub fn transform_tree_shake(source: &str, path: &str, used_exports: &std::collections::HashSet<String>) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    if !ret.errors.is_empty() {
        return source.to_string();
    }
    
    let mut replacements: Vec<(u32, u32, String)> = Vec::new();
    let program = ret.program;
    
    for stmt in program.body {
        if let oxc_ast::ast::Statement::ModuleDeclaration(decl) = stmt {
            match decl.0 {
                ModuleDeclaration::ExportDefaultDeclaration(d) => {
                    if !used_exports.contains("default") {
                        // Remove entire statement
                         replacements.push((d.span.start, d.span.end, "".to_string()));
                    }
                }
                ModuleDeclaration::ExportNamedDeclaration(d) => {
                    if let Some(declaration) = &d.declaration {
                        // export const x = 1;
                        let mut keep = false;
                        
                        match declaration {
                             oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                                 for decl in &var_decl.declarations {
                                     if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(id) = &decl.id.kind {
                                         if used_exports.contains(id.name.as_str()) {
                                             keep = true;
                                         }
                                     }
                                 }
                             }
                             oxc_ast::ast::Declaration::FunctionDeclaration(f) => {
                                 if let Some(id) = &f.id {
                                      if used_exports.contains(id.name.as_str()) {
                                          keep = true;
                                      }
                                 }
                             }
                             oxc_ast::ast::Declaration::ClassDeclaration(c) => {
                                 if let Some(id) = &c.id {
                                      if used_exports.contains(id.name.as_str()) {
                                          keep = true;
                                      }
                                 }
                             }
                             _ => {}
                        }
                        
                        if !keep {
                            // Safely remove
                            replacements.push((d.span.start, d.span.end, "".to_string()));
                        }
                    } else if !d.specifiers.is_empty() {
                        // export { x, y }
                        let mut kept_specs = Vec::new();
                        for spec in &d.specifiers {
                            let exported = spec.exported.name().as_str();
                            if used_exports.contains(exported) {
                                let local = spec.local.name().as_str();
                                if local == exported {
                                    kept_specs.push(local.to_string());
                                } else {
                                    kept_specs.push(format!("{} as {}", local, exported));
                                }
                            }
                        }
                        
                        if kept_specs.is_empty() {
                            replacements.push((d.span.start, d.span.end, "".to_string()));
                        } else {
                            // Rewrite
                            let new_stmt = if let Some(src) = &d.source {
                                format!("export {{ {} }} from \"{}\";", kept_specs.join(", "), src.value.as_str())
                            } else {
                                format!("export {{ {} }};", kept_specs.join(", "))
                            };
                            replacements.push((d.span.start, d.span.end, new_stmt));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    
    // Sort reverse
    replacements.sort_by(|a, b| b.0.cmp(&a.0));
    
    let mut result = source.to_string();
    for (start, end, text) in replacements {
        result.replace_range(start as usize..end as usize, &text);
    }
    
    result
}
