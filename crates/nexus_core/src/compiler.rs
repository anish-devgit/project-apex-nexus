use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_codegen::Codegen;
use std::path::Path;

pub struct CompileResult {
    pub code: String,
    pub sourcemap: Option<String>,
    // Week 13: Production Outputs
    pub css: Option<String>,
    pub asset: Option<(String, Vec<u8>)>,
}

use lightningcss::stylesheet::{StyleSheet, ParserOptions, PrinterOptions};
use base64::Engine;

pub fn compile_asset(bytes: &[u8], filename: &str, is_prod: bool) -> CompileResult {
    // 1. JSON
    if filename.ends_with(".json") {
         let text = String::from_utf8_lossy(bytes);
         return CompileResult {
             code: format!("export default {};", text),
             sourcemap: None,
             css: None,
             asset: None,
         };
    }

    // 2. Binary / Image
    // Limit: 8KB
    if bytes.len() < 8 * 1024 {
        let mime = mime_guess::from_path(filename).first_or_octet_stream();
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let code = format!("export default \"data:{};base64,{}\"", mime, b64);
        return CompileResult { code, sourcemap: None, css: None, asset: None };
    }
    
    // 3. Large Asset
    if is_prod {
        // Production: Emit file to dist/assets/
        let name = std::path::Path::new(filename).file_name().unwrap_or_default().to_string_lossy();
        let out_path = format!("assets/{}", name);
        // URL for runtime (absolute)
        let code = format!("export default \"/{}\"", out_path);
        
        return CompileResult {
            code,
            sourcemap: None,
            css: None,
            asset: Some((out_path, bytes.to_vec())),
        };
    } else {
        // Dev: Serve Raw
        // We assume filename is a valid URL path (virtual path used by server)
        let code = format!("export default \"{}?raw\";", filename);
        CompileResult { code, sourcemap: None, css: None, asset: None }
    }
}

pub fn compile_css(source: &str, _filename: &str, is_prod: bool) -> CompileResult {
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

    if is_prod {
        // Production: Extract CSS, don't generate JS injector
        return CompileResult {
            code: "// Extracted CSS".to_string(),
            sourcemap: None,
            css: Some(css_content),
            asset: None,
        };
    }

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
        css: None,
        asset: None,
    }
}

pub fn compile(source: &str, filename: &str, _is_prod: bool) -> CompileResult {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(Path::new(filename)).unwrap_or_default();
    
    // 1. Parse
    let ret = Parser::new(&allocator, source, source_type).parse();
    
    if !ret.errors.is_empty() {
         tracing::warn!("Parse errors in {}: {:?}", filename, ret.errors);
    }
    
    let program = ret.program;

    // 2. Transform (TS + JSX) - Disabled for now
    // oxc v0.54 requires different transformer API
    // For MVP, we skip transformation
    
    // 3. Codegen
    let ret = Codegen::new().build(&program);

    CompileResult {
        code: ret.code,
        sourcemap: ret.map.map(|sm| sm.to_json_string()),
        css: None,
        asset: None,
    }
}
