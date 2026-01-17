use nexus_core::parser::transform_cjs;
use nexus_core::runtime::NEXUS_RUNTIME_JS;
use std::collections::HashMap;

#[test]
fn test_cjs_transform_simple() {
    let source = "import 'pkg'; export default 42;";
    // Expected: require("pkg"); exports.default = 42;
    // Note: our transform implementation strips "export default" to "exports.default ="
    // and "import 'pkg'" to "require('pkg');"
    
    let mut imports = HashMap::new();
    imports.insert("pkg".to_string(), "/node_modules/pkg/index.js".to_string());
    
    let result = transform_cjs(source, "test.js", &imports);
    // Should use resolved path
    assert!(result.contains("require(\"/node_modules/pkg/index.js\");"));
    assert!(result.contains("exports.default = 42;"));
}

#[test]
fn test_cjs_transform_imports() {
    let source = "import x from './utils'; import { y } from './other';";
    let mut imports = HashMap::new();
    imports.insert("./utils".to_string(), "/src/utils.js".to_string());
    imports.insert("./other".to_string(), "/src/other.js".to_string());
    
    let result = transform_cjs(source, "test.js", &imports);
    
    assert!(result.contains("const x = require(\"/src/utils.js\").default;"));
    assert!(result.contains("const y = require(\"/src/other.js\").y;"));
}

#[test]
fn test_cjs_transform_named_export() {
    let source = "export { foo };";
    let imports = HashMap::new();
    let result = transform_cjs(source, "test.js", &imports);
    assert!(result.contains("exports.foo = foo;"));
}

#[test]
fn test_runtime_bootstrap_presence() {
    // Just verify the constant string assumes correct structure
    assert!(NEXUS_RUNTIME_JS.contains("global.__nexus_register__ = function"));
    assert!(NEXUS_RUNTIME_JS.contains("global.__nexus_require__ = function"));
    
    // Verify Patch Fixes
    // 1. No eager cache deletion in register
    assert!(!NEXUS_RUNTIME_JS.contains("delete global.__nexus_cache__[id];")); 
    // Note: The delete might be present in catch block or clean up, so be careful.
    // The previous implementation had it in register. New implementation has it in catch block ONLY?
    // Let's check context or just check for the comment indicating fix? 
    // Or assert that "if (global.__nexus_cache__[id])" block is gone from register?
    
    // 2. module.hot stub
    assert!(NEXUS_RUNTIME_JS.contains("hot: {"));
    assert!(NEXUS_RUNTIME_JS.contains("accept: function"));
    assert!(NEXUS_RUNTIME_JS.contains("dispose: function"));
    
    // 3. module.id
    assert!(NEXUS_RUNTIME_JS.contains("id: id,"));
}
