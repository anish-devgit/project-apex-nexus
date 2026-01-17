use nexus_core::compiler::compile;
use nexus_core::parser::transform_cjs;
use std::collections::HashMap;

// Note: Testing actual React Refresh runtime execution requires a browser or jsdom.
// Here we verifying:
// 1. Compiler output contains $RefreshReg$ logic (via checking Oxc opts).
// 2. Footer injection logic is correct (we can't test lib.rs logic easily in unit test without mocking server state, 
//    but we can verify compilation).

#[test]
fn test_react_refresh_transform_enabled() {
    // This verifies that compiler.rs is configured correctly 
    // to output React Refresh signatures if oxc supports it.
    let source = r#"
    import { useState } from 'react';
    export default function App() {
        const [count, setCount] = useState(0);
        return <div>{count}</div>;
    }
    "#;
    
    // We check if "react-refresh" related code is present?
    // Oxc with `development: true` and `refresh: Some(...)` should emit `_s = $RefreshSig$()` etc.
    // Or at least `_source` properties.
    
    let compiled = compile(source, "src/App.tsx");
    println!("Compiled code:\n{}", compiled.code);
    
    // Oxc Refresh transform usually emits:
    // var _s = $RefreshSig$();
    // _s();
    
    // If enabled, we should see $RefreshSig$.
    assert!(compiled.code.contains("$RefreshSig$") || compiled.code.contains("_s = $RefreshSig$"));
}
