pub const NEXUS_RUNTIME_JS: &str = r#"
(function(global) {
  // 1. The Module Registry
  global.__nexus_modules__ = global.__nexus_modules__ || {};

  // 2. The Module Cache
  global.__nexus_cache__ = global.__nexus_cache__ || {};

  // 3. The Register Function
  global.__nexus_register__ = function(id, factoryFn) {
    global.__nexus_modules__[id] = factoryFn;
    // FIX 1: Stop eager cache eviction.
    // We let HMR logic handle cache invalidation externally or via hot.dispose.
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

    // FIX 2 & 3: Inject module.hot stub and module.id
    // Week 10: State-Preserving HMR Support
    const module = {
      id: id,
      exports: {},
      hot: {
        _accepted: false, // Internal flag for HMR client
        accept: function(dep, cb) {
            // Minimal implementation:
            // If called without args or with self-ID, mark as accepted.
            if (!dep || dep === id) {
                this._accepted = true;
            }
            // Bubbling not implemented in Week 10
        },
        dispose: function() {},
        data: null
      }
    };
    
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

  // 5. Async Import
  global.__nexus_chunk_map__ = global.__nexus_chunk_map__ || {};
  
  global.__nexus_import__ = function(id) {
    if (global.__nexus_modules__[id]) {
      return Promise.resolve(global.__nexus_require__(id));
    }

    let url = id;
    if (global.__nexus_chunk_map__[id]) {
        url = global.__nexus_chunk_map__[id];
    }
    
    // Normalize URL? If ID is /src/foo and map has it, use map.
    // If not in map, assume Dev Mode and fetch ID directly.

    return fetch(url)
      .then(res => {
          if (!res.ok) throw new Error("[Nexus] Failed to load chunk: " + url);
          return res.text();
      })
      .then(code => {
          (0, eval)(code);
          if (!global.__nexus_modules__[id]) {
              throw new Error("[Nexus] Async chunk loaded but module not registered: " + id);
          }
          return global.__nexus_require__(id);
      });
  };
})(typeof window !== 'undefined' ? window : this);
"#;

