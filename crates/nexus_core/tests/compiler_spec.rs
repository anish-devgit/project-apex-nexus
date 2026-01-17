use nexus_core::compiler::compile;

#[test]
fn test_compile_ts_strip() {
    let source = "const add = (a: number, b: number): number => a + b;";
    let res = compile(source, "test.ts");
    
    // Types should be gone
    assert!(!res.code.contains(": number"));
    assert!(res.code.contains("const add = (a, b) => a + b"));
}

#[test]
fn test_compile_jsx() {
    let source = "export default () => <h1>Hello</h1>;";
    let res = compile(source, "test.tsx");
    
    // Should contain factory call (e.g. React.createElement or similar default)
    // Oxc default is usually React.createElement or jsx() depending on config.
    // Assuming React classic or automatic.
    // Let's assert broadly that it transformed tags.
    assert!(!res.code.contains("<h1>"));
    assert!(res.code.contains("createElement") || res.code.contains("jsx"));
}

#[test]
fn test_sourcemap_generation() {
    let source = "const x: number = 1;";
    let res = compile(source, "test.ts");
    
    assert!(res.sourcemap.is_some());
    let map = res.sourcemap.unwrap();
    assert!(map.contains("\"version\":3"));
    assert!(map.contains("\"sources\":[\"test.ts\"]"));
}
