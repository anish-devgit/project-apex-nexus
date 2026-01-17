use nexus_core::parser::extract_dependencies;

#[test]
fn test_import_parsing() {
    let source = r#"
        import { foo } from "./utils.js";
        import * as bar from '../lib/bar';
        import "side-effect";
        export { baz } from "./baz.ts";
        const x = 1;
    "#;

    let deps = extract_dependencies(source, "test.tsx");
    
    assert!(deps.contains(&"./utils.js".to_string()));
    assert!(deps.contains(&"../lib/bar".to_string()));
    assert!(deps.contains(&"side-effect".to_string()));
    assert!(deps.contains(&"./baz.ts".to_string()));
    assert_eq!(deps.len(), 4);
}
