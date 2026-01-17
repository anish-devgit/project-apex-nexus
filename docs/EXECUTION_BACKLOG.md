# 90-Day Execution Backlog

This backlog outlines the path to the v0.1 release of Nexus.

## Label Strategy
*   `area:kernel` - Rust core logic
*   `area:cli` - TypeScript CLI and User Interface
*   `area:hmr` - Hot Module Replacement
*   `area:cache` - Persistent Graph / Database
*   `area:plugin` - Vite Plugin Adapter
*   `area:docs` - Documentation
*   `kind:benchmark` - Performance testing
*   `kind:infra` - CI/CD, Repo setup

## Epics & Milestones

### Milestone 1: The Skeleton (Weeks 1-4)
*Focus: Repo setup, Rust HTTP server, Basic File Serving.*

- [ ] **[INFRA] Initialize Monorepo and CI**
    - Criteria: Cargo workspace and pnpm package structure created. CI passes on `main`.
    - Labels: `kind:infra`
- [ ] **[KERNEL] Implement Axum Dev Server**
    - Criteria: `nexus dev` starts a server on localhost:3000.
    - Labels: `area:kernel`
- [ ] **[KERNEL] Basic File Resolution via Oxc**
    - Criteria: Can request `src/main.js` and receive content. Uses `oxc_resolver`.
    - Labels: `area:kernel`
- [ ] **[CLI] Create `nexus-cli` package**
    - Criteria: `napi` bindings configured. `nexus --version` works.
    - Labels: `area:cli`
- [ ] **[BENCHMARKS] Create Repo Generation Script**
    - Criteria: `gen_repo.js` capable of creating 10k module projects.
    - Labels: `kind:benchmark`

### Milestone 2: Virtual Chunking (Weeks 5-8)
*Focus: The "Secret Sauce". Concatenating modules in memory.*

- [ ] **[KERNEL] Implement Module Graph Structure**
    - Criteria: Graph struct holds `Import` relationships and `Source` content.
    - Labels: `area:kernel`
- [ ] **[KERNEL] Implement Virtual Chunker Logic**
    - Criteria: Requests to `/chunk/ABC` return concatenated content of A, B, and C.
    - Labels: `area:kernel`
- [ ] **[KERNEL] Integrate `oxc_parser` for Import Rewriting**
    - Criteria: Imports in source files are rewritten to point to Virtual Chart IDs.
    - Labels: `area:kernel`
- [ ] **[CACHE] Implement Persistent Cache (redb/sled)**
    - Criteria: Graph state survives server restart. Cold start < 500ms.
    - Labels: `area:cache`
- [ ] **[HMR] WebSocket HMR Integration**
    - Criteria: Editing a file triggers a message to the client.
    - Labels: `area:hmr`

### Milestone 3: Plugin Compatibility & Launch (Weeks 9-12)
*Focus: Vite ecosystem adapters and polishing.*

- [ ] **[PLUGIN] Implement Shim Layer for Vite Plugins**
    - Criteria: Basic Vite React plugin works with Nexus.
    - Labels: `area:plugin`
- [ ] **[PLUGIN] Batch `napi` Calls**
    - Criteria: Reduce JS-Rust boundary crossing overhead.
    - Labels: `area:plugin`
- [ ] **[DOCS] Write "Getting Started" Guide**
    - Criteria: Clear instructions for migrating a Vite app.
    - Labels: `area:docs`
- [ ] **[BENCHMARK] Micro/Mid/Macro Benchmark Report**
    - Criteria: Comparison vs Vite recorded and published.
    - Labels: `kind:benchmark`
- [ ] **[INFRA] Publish v0.1 to npm**
    - Criteria: `npm install nexus-cli` works.
    - Labels: `kind:infra`

## Detailed Issues (Sample - to be expanded to ~60)

### Kernel
1.  `[KERNEL] Set up axum router with state management`
2.  `[KERNEL] Implement Oxc Resolver wrapper`
3.  `[KERNEL] Native file watcher implementation (notify)`
4.  `[KERNEL] Build "Graph" struct with RWLock`
5.  `[KERNEL] Implement caching layer traits`
6.  `[KERNEL] Handle Sourcemap concatenation (magic-string)`
7.  `[KERNEL] Optimistic dependency scanning`
8.  `[KERNEL] Error handling for resolution failures`
9.  `[KERNEL] Implement CSS injection logic`
10. `[KERNEL] Serve static assets`

### CLI & Plugin
11. `[CLI] Argument parsing (clap equivalent or JS side)`
12. `[CLI] Interactive terminal output (ink / specialized)`
13. `[PLUGIN] Map Vite `configureServer` hook`
14. `[PLUGIN] Map Vite `resolveId` hook`
15. `[PLUGIN] Map Vite `transform` hook`
16. `[PLUGIN] Mock Vite's `PluginContext``
17. `[PLUGIN] Handle `load` hook`

### HMR & Client
18. `[HMR] Client-side HMR runtime (overlay)`
19. `[HMR] Diffing logic for graph updates`
20. `[HMR] "Hot" boundary detection`

### Benchmark & Test
21. `[BENCH] Setup Hyperfine CI workflow`
22. `[TEST] Integration test suite for standard React app`
23. `[TEST] Unit tests for Virtual Chunker`
24. `[TEST] Memory usage tracking`

... (Remaining issues to be filled during Sprint Planning)
