pub const NEXUS_RUNTIME_JS: &str = r#"
(function(global) {
  // 1. The Module Registry
  global.__nexus_modules__ = global.__nexus_modules__ || {};

  // 2. The Module Cache
  global.__nexus_cache__ = global.__nexus_cache__ || {};

  // 3. The Register Function
  global.__nexus_register__ = function(id, factoryFn) {
    global.__nexus_modules__[id] = factoryFn;
    if (global.__nexus_cache__[id]) {
      delete global.__nexus_cache__[id];
    }
  };

  // 4. The Require Function
  global.__nexus_require__ = function(id) {
    if (global.__nexus_cache__[id]) {
      return global.__nexus_cache__[id].exports;
    }

    const factory = global.__nexus_modules__[id];
    if (!factory) {
      throw new Error(`[Nexus] Module not found: ${id}`);
    }

    const module = { exports: {} };
    global.__nexus_cache__[id] = module;

    try {
      factory(
        global.__nexus_require__, // require
        module,                   // module
        module.exports            // exports
      );
    } catch (err) {
      delete global.__nexus_cache__[id];
      throw err;
    }

    return module.exports;
  };
})(typeof window !== 'undefined' ? window : this);
"#;
