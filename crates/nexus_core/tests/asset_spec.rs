use nexus_core::compiler::compile_asset;

#[test]
fn test_compile_json() {
    let bytes = r#"{ "foo": "bar" }"#.as_bytes();
    let result = compile_asset(bytes, "data.json");
    
    // Should export default parsed JSON
    assert!(result.code.contains("export default { \"foo\": \"bar\" };"));
}

#[test]
fn test_compile_small_image_inline() {
    // 5 bytes < 8KB
    let bytes = vec![1, 2, 3, 4, 5]; 
    let result = compile_asset(&bytes, "icon.png");
    
    // Should be Data URI
    // Expect: export default "data:image/png;base64,AQIDBAU="; (AQIDBAU= is base64 of 1,2,3,4,5)
    // Actually Base64 encoded.
    assert!(result.code.contains("export default \"data:image/png;base64,"));
    // check mime
    assert!(result.code.contains("image/png"));
}

#[test]
fn test_compile_large_image_url() {
    // > 8KB
    let bytes = vec![0; 9000];
    let result = compile_asset(&bytes, "/src/large.png"); // Virtual path
    
    // Should export raw URL
    assert!(result.code.contains("export default \"/src/large.png?raw\";"));
}

#[test]
fn test_mime_guess() {
    let bytes = vec![0];
    let result = compile_asset(&bytes, "test.svg");
    assert!(result.code.contains("image/svg+xml"));
    
    let result2 = compile_asset(&bytes, "test.wasm");
    assert!(result2.code.contains("application/wasm"));
}
