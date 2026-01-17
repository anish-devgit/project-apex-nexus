# v0.1 MVP Specification: Nexus

## 1. Goal
deliver a working frontend toolchain in **90 days** that demonstrates "Virtual Chunking" and valid sub-500ms cold start times for a 10k module benchmark, while maintaining enough Vite plugin compatibility to run a standard React project.

## 2. Technical Stack ("The Apex Stack")
*   **Language**: Rust (Kernel), TypeScript (Shell)
*   **Parsing**: `oxc` (The Oxidation Compiler) - 3x faster than SWC.
*   **Resolution**: `oxc_resolver` - Node.js compatible, 20x faster than enhanced-resolve.
*   **HTTP Server**: `axum` + `tower` - Best-in-class async I/O and WebSocket support.
*   **JS Bridge**: `napi-rs` - Zero-copy string/buffer passing.
*   **Caching**: `redb` (or `sled`) - Embedded persistent key-value store.
*   **Bundling**: Custom "Virtual Chunker" (Memory-based concatenation).

## 3. Scope of Work (v0.1)

### IN Scope (Must Build)
1.  **Rust Kernel (`nexus_core`)**:
    *   Development Server based on `axum`.
    *   Virtual Chunking logic (concatenating ESM modules on the fly).
    *   Persistent Dependency Graph (stored in `redb`).
2.  **Plugin Adapter (`nexus_plugin`)**:
    *   Shim layer to support basic Vite plugins.
    *   `napi-rs` bindings to expose Rust core to Node.js.
3.  **CLI (`nexus-cli`)**:
    *   `nexus dev`: Start dev server.
    *   `nexus build`: Production build (delegating to Rolldown or internal bundler if ready, but likely simple concatenation for v0.1).
4.  **HMR (Hot Module Replacement)**:
    *   Basic HMR propagation over WebSockets.
    *   Handling of CSS updates and JS module invalidation.
5.  **Benchmarks**:
    *   Automated scripts to generate 1k, 10k, and 50k module repos.
    *   Comparison reports vs Vite.

### OUT of Scope (Defer to v0.2+)
1.  **Full Webpack/Rollup Logic Compliance**: We will not support every edge case of module federation or exotic module types initially.
2.  **Legacy CommonJS Support**: Focus strictly on ESM first.
3.  **Production Optimization**: Tree-shaking and minification are nice-to-have but not critical for the "Virtual Chunking" proof of concept. The focus is on DevUX speed.
4.  **Windows/Linux Edge Cases**: Initial heavy optimization for POSIX/Mac, with generic support for others. (Though user is on Windows, we ensure it builds, but deep OS optimization is secondary).
5.  **UI Framework Specifics**: No "Next.js" or "Nuxt" adapters yet. Plain React/Vue via Vite plugins only.
