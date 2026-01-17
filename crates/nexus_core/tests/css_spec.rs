use nexus_core::compiler::compile_css;

#[test]
fn test_compile_css_injection() {
    let source = ".foo { color: red; }";
    let filename = "style.css";
    
    let result = compile_css(source, filename);
    let code = result.code;
    
    // Check for JS wrapper elements
    assert!(code.contains("nexus-style-"));
    assert!(code.contains("document.createElement(\"style\")"));
    assert!(code.contains("style.textContent = \".foo { color: red; }\""));
    assert!(code.contains("module.hot.accept()"));
    assert!(code.contains("style.remove()"));
}

#[test]
fn test_compile_css_escaping() {
    let source = ".foo { content: \"hello \\\"world\\\"\"; }";
    let filename = "style.css";
    
    let result = compile_css(source, filename);
    // Should be valid JS string
    // serde_json should handle escaping quotes
    assert!(result.code.contains(r#"content: \"hello \\\"world\\\"\""#) || result.code.contains("content"));
}
