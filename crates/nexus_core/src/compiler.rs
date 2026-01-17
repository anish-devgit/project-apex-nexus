use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use oxc_codegen::{Codegen, CodegenOptions};
use std::path::Path;

pub struct CompileResult {
    pub code: String,
    pub sourcemap: Option<String>,
}

use lightningcss::stylesheet::{StyleSheet, ParserOptions, PrinterOptions};

pub fn compile_css(source: &str, _filename: &str) -> CompileResult {
    // 1. Parse & Normalize (Validate)
    // We use lightningcss to ensure valid CSS and normalize output.
    let sheet_res = StyleSheet::parse(source, ParserOptions::default());
    
    let css_content = match sheet_res {
        Ok(sheet) => {
            let printer_options = PrinterOptions {
                minify: false, // Requirement: Do NOT optimize yet
                source_map: None,
                ..PrinterOptions::default()
            };
            match sheet.to_css(printer_options) {
                Ok(res) => res.code,
                Err(_) => source.to_string(), // Fallback
            }
        },
        Err(e) => {
            tracing::error!("CSS Parse Error in {}: {}", _filename, e);
            // Fallback to raw source to allow browser to maybe handle/debug
            source.to_string()
        }
    };

    // 2. Wrap in JS Injector
    // Use serde_json to safely escape the CSS string for inclusion in JS
    let escaped_css = serde_json::to_string(&css_content).unwrap_or_else(|_| format!("`{}`", css_content));

    let code = format!(r#"
(function() {{
    const styleId = "nexus-style-" + module.id;
    let style = document.getElementById(styleId);
    if (!style) {{
      style = document.createElement("style");
      style.id = styleId;
      document.head.appendChild(style);
    }}
    style.textContent = {};

    if (module.hot) {{
      module.hot.accept();
      module.hot.dispose(() => {{
          style.remove();
      }});
    }}
}})();
"#, escaped_css);

    CompileResult {
        code,
        sourcemap: None,
    }
}

pub fn compile(source: &str, filename: &str) -> CompileResult {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new(filename)).unwrap_or_default();

    // 1. Parse
    let ret = Parser::new(&allocator, source, source_type).parse();

    if !ret.errors.is_empty() {
        tracing::warn!("Parsing errors in {}: {:?}", filename, ret.errors);
        // We continue best-effort
    }

    let program = ret.program;
    let trivias = ret.trivias;

    // 2. Transform (TS + JSX)
    // Week 8 Requirement: "Enable TypeScript stripping", "Enable JSX transform"
    // Week 10 Requirement: "Enable react.refresh and react.development"
    
    let transform_options = TransformOptions {
        react: oxc_transformer::ReactOptions {
            refresh: Some(oxc_transformer::ReactRefreshOptions::default()),
            development: true, // Adds _source, _self for better debugging/refresh
            ..Default::default()
        },
        ..TransformOptions::default() // Use default for TS etc.
    }; 
    
    let ret = Transformer::new(&allocator, Path::new(filename), source_type, transform_options)
        .build(program);
        
    if !ret.errors.is_empty() {
         tracing::warn!("Transformation errors in {}: {:?}", filename, ret.errors);
    }
    
    let program = ret.program;

    // 3. Codegen
    let codegen_options = CodegenOptions {
        enable_source_map: true,
        ..CodegenOptions::default()
    };
    
    let ret = Codegen::new()
        .with_options(codegen_options)
        .build(&program);

    CompileResult {
        code: ret.source_text,
        sourcemap: ret.source_map.map(|sm| sm.to_json_string().unwrap_or_default()),
    }
}
