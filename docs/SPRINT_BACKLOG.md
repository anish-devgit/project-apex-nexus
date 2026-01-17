# Project Apex - 90-Day Sprint Backlog

**Timeline**: Day 1 → Day 90  
**Team**: 1 Founder + 2 Developers  
**Target**: v0.1 MVP Release

---

## Epic 1: Core Rust Kernel (Days 1-30)

**Owner**: Founder  
**Goal**: Build the foundation - parsing, resolution, and basic HTTP server

### Milestone 1.1: Project Setup & Scaffolding (Week 1)

#### Issue #1: Initialize Rust workspace structure
**Labels**: `kernel`, `infrastructure`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Cargo workspace with 3 crates: `nexus-kernel`, `nexus-chunker`, `nexus-cache`
- [ ] Basic `Cargo.toml` with shared dependencies
- [ ] Directory structure: `crates/kernel/`, `crates/chunker/`, `crates/cache/`
- [ ] CI passes with empty lib files

#### Issue #2: Set up oxc parser integration
**Labels**: `kernel`, `parsing`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Add `oxc_parser`, `oxc_allocator`, `oxc_span` dependencies
- [ ] Create `parse_module()` function that accepts file path
- [ ] Return AST with imports/exports extracted
- [ ] Unit test: Parse a simple React component

#### Issue #3: Implement oxc_resolver for module resolution
**Labels**: `kernel`, `resolution`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Add `oxc_resolver` dependency
- [ ] Resolve Node.js-style imports (`./file`, `package`, `@scope/package`)
- [ ] Support `tsconfig.json` paths
- [ ] Unit test: Resolve `import React from 'react'` → `node_modules/react/index.js`

#### Issue #4: Build basic Axum HTTP server
**Labels**: `kernel`, `server`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Start server on `localhost:3000` (configurable port)
- [ ] Serve static files from `/public`
- [ ] Return 404 for missing files
- [ ] Integration test: `curl http://localhost:3000/index.html` → 200 OK

#### Issue #5: Add file system watcher with notify crate
**Labels**: `kernel`, `hmr`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Watch project directory for `.js`, `.ts`, `.jsx`, `.tsx` changes
- [ ] Log file path on change event
- [ ] Debounce rapid changes (100ms window)
- [ ] Test: Touch file → see log entry

### Milestone 1.2: Dependency Graph Construction (Week 2)

#### Issue #6: Build in-memory dependency graph
**Labels**: `kernel`, `graph`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] `DependencyGraph` struct with `HashMap<PathBuf, Vec<PathBuf>>`
- [ ] `add_module()` method parses imports and adds edges
- [ ] `get_dependents()` returns reverse dependencies
- [ ] Unit test: 3-file chain (A → B → C) resolves correctly

#### Issue #7: Implement module invalidation on file change
**Labels**: `kernel`, `graph`, `hmr`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] When file X changes, invalidate X and all dependents
- [ ] Remove stale entries from graph
- [ ] Unit test: Change B → invalidates A (if A imports B)

#### Issue #8: Add support for circular dependency detection
**Labels**: `kernel`, `graph`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Detect cycles using DFS
- [ ] Log warning (don't crash)
- [ ] Unit test: A → B → A detects cycle

#### Issue #9: Support TypeScript file resolution (.ts, .tsx)
**Labels**: `kernel`, `resolution`, `typescript`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Resolve `import './file'` → checks `.ts`, `.tsx`, `.js`, `.jsx`
- [ ] Parse `tsconfig.json` for path aliases
- [ ] Test: Resolve `@/components/Button` → `src/components/Button.tsx`

### Milestone 1.3: Persistent Graph with sled (Week 3)

#### Issue #10: Integrate sled embedded database
**Labels**: `kernel`, `cache`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Add `sled` dependency to `nexus-cache` crate
- [ ] Open database at `.nexus/cache.db`
- [ ] Basic put/get operations
- [ ] Test: Write key → restart → read key (persists)

#### Issue #11: Serialize dependency graph to sled
**Labels**: `kernel`, `cache`, `graph`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] On graph update, write to sled (key: file path, value: JSON of dependencies)
- [ ] On cold start, load graph from sled
- [ ] Test: Build graph → restart server → graph loads instantly

#### Issue #12: Add cache invalidation on package.json change
**Labels**: `kernel`, `cache`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Watch `package.json`, `package-lock.json`
- [ ] On change, clear entire cache (dependencies may have changed)
- [ ] Test: Change `package.json` → cache cleared

### Milestone 1.4: TypeScript Transpilation (Week 4)

#### Issue #13: Transpile TypeScript to JavaScript using oxc
**Labels**: `kernel`, `typescript`, `transform`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Convert TS → JS (strip types)
- [ ] Support JSX → React.createElement
- [ ] No type checking (users run `tsc --noEmit` separately)
- [ ] Test: `const x: number = 5` → `const x = 5`

#### Issue #14: Handle JSX/TSX transformation
**Labels**: `kernel`, `typescript`, `jsx`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Convert `<div>Hi</div>` → `React.createElement('div', null, 'Hi')`
- [ ] Support fragment syntax (`<>...</>`)
- [ ] Test: Complex component with props transforms correctly

#### Issue #15: Add source map generation (line-only)
**Labels**: `kernel`, `sourcemaps`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Generate inline source maps for transformed files
- [ ] Line mappings only (defer column mappings to v0.2)
- [ ] Test: Set breakpoint in TS → browser shows correct TS line

---

## Epic 2: Virtual Chunking Engine (Days 20-45)

**Owner**: Founder  
**Goal**: Core innovation - serve concatenated ESM modules in-memory

### Milestone 2.1: Basic Virtual Chunk Serving (Week 4-5)

#### Issue #16: Design virtual chunk ID scheme
**Labels**: `chunker`, `design`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Chunk ID format: `/_nexus/chunk/{hash}.js`
- [ ] Hash based on entry module path + dependencies
- [ ] Document in `docs/ARCHITECTURE.md`

#### Issue #17: Implement in-memory module concatenation
**Labels**: `chunker`, `core`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Given list of modules, concatenate code in memory
- [ ] Wrap each module in IIFE or ESM scope
- [ ] Preserve export/import statements (rewrite to internal references)
- [ ] Test: 3 modules → single concatenated chunk

#### Issue #18: Add chunk caching (in-memory LRU)
**Labels**: `chunker`, `cache`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Use `lru` crate to cache generated chunks
- [ ] Max 100 chunks in memory (configurable)
- [ ] Evict oldest on overflow
- [ ] Test: Request same chunk twice → cache hit (logs show)

#### Issue #19: Serve virtual chunks via Axum route
**Labels**: `chunker`, `server`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Route: `GET /_nexus/chunk/:id`
- [ ] Return `Content-Type: application/javascript`
- [ ] Return 404 if chunk not found
- [ ] Test: `curl /_nexus/chunk/abc123.js` → concatenated code

### Milestone 2.2: Module Graph → Chunk Mapping (Week 5-6)

#### Issue #20: Implement chunk splitting strategy
**Labels**: `chunker`, `algorithm`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Group modules by entry point (e.g., `index.html` → main chunk)
- [ ] Shared dependencies → separate chunk
- [ ] Max chunk size: 500KB (configurable)
- [ ] Document algorithm in `docs/ARCHITECTURE.md`

#### Issue #21: Generate HTML with chunk script tags
**Labels**: `chunker`, `html`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Parse `index.html`
- [ ] Inject `<script type="module" src="/_nexus/chunk/main.js"></script>`
- [ ] Remove original `<script>` tags
- [ ] Test: Serve modified HTML → browser loads chunk

#### Issue #22: Handle dynamic imports (`import()`)
**Labels**: `chunker`, `dynamic-import`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Detect `import('./lazy.js')` in AST
- [ ] Create separate chunk for lazy module
- [ ] Rewrite to `import('/_nexus/chunk/lazy-{hash}.js')`
- [ ] Test: Lazy load component → separate chunk fetched

### Milestone 2.3: Import Rewriting (Week 6-7)

#### Issue #23: Rewrite bare imports to node_modules
**Labels**: `chunker`, `transform`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] `import React from 'react'` → `import React from '/_nexus/chunk/react.js'`
- [ ] Use oxc_resolver to find actual file
- [ ] Test: Import from npm package → correct chunk loaded

#### Issue #24: Rewrite relative imports within chunks
**Labels**: `chunker`, `transform`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Internal module references use chunk-local IDs
- [ ] `import { foo } from './utils'` → internal call (same chunk)
- [ ] Test: Chunk contains 2 modules with cross-import

#### Issue #25: Handle external dependencies (CDN fallback)
**Labels**: `chunker`, `externals`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Config option: `externals: ['react', 'react-dom']`
- [ ] Don't bundle externals, rewrite to CDN (e.g., esm.sh)
- [ ] Test: React external → loads from CDN

---

## Epic 3: Hot Module Replacement (Days 30-50)

**Owner**: Dev 1  
**Goal**: WebSocket-based HMR with full page reload fallback

### Milestone 3.1: WebSocket Server (Week 5-6)

#### Issue #26: Add WebSocket support to Axum server
**Labels**: `hmr`, `server`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] WebSocket route: `ws://localhost:3000/_nexus/hmr`
- [ ] Accept client connections
- [ ] Broadcast messages to all clients
- [ ] Test: Connect via `wscat` → send/receive works

#### Issue #27: Implement HMR client script
**Labels**: `hmr`, `client`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] JavaScript file: `client/hmr-client.js`
- [ ] Connect to WebSocket on page load
- [ ] Listen for `reload` message → `location.reload()`
- [ ] Inject into HTML via `<script>` tag

#### Issue #28: Send file change events to connected clients
**Labels**: `hmr`, `server`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] On file change, send JSON: `{ type: 'reload', path: '/src/App.tsx' }`
- [ ] All connected clients receive message
- [ ] Test: Edit file → browser reloads

### Milestone 3.2: Module Invalidation (Week 7)

#### Issue #29: Clear affected chunks on file change
**Labels**: `hmr`, `chunker`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] When file X changes, find chunks containing X
- [ ] Evict from LRU cache
- [ ] Regenerate on next request
- [ ] Test: Edit file → chunk cache miss → regenerated

#### Issue #30: Implement granular HMR (accept API)
**Labels**: `hmr`, `client`, `v0.2-candidate`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Support `import.meta.hot.accept()` in client code
- [ ] On update, replace module without full reload
- [ ] Track module dependencies for boundary detection
- [ ] Test: Edit accepted module → updates without reload
- [ ] **NOTE**: Defer to v0.2 if blocked

---

## Epic 4: TypeScript CLI Package (Days 20-55)

**Owner**: Dev 1  
**Goal**: User-facing command-line tool built with TypeScript

### Milestone 4.1: CLI Scaffolding (Week 4)

#### Issue #31: Initialize TypeScript package
**Labels**: `cli`, `infrastructure`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Directory: `packages/nexus-cli/`
- [ ] `package.json` with `bin` entry: `nexus`
- [ ] TypeScript setup: `tsconfig.json`
- [ ] Test: `npm link` → `nexus --version` works

#### Issue #32: Implement `nexus dev` command
**Labels**: `cli`, `dev`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Spawn Rust binary: `cargo run --release --bin nexus-kernel`
- [ ] Pass config options as CLI args or JSON
- [ ] Stream stdout/stderr to console
- [ ] Test: `nexus dev` → server starts

#### Issue #33: Implement `nexus build` command
**Labels**: `cli`, `build`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] For v0.1, delegate to Rollup
- [ ] Read `nexus.config.ts`, convert to Rollup config
- [ ] Run Rollup programmatically
- [ ] Test: `nexus build` → `dist/` folder created

#### Issue #34: Add `nexus init` scaffolding command
**Labels**: `cli`, `init`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Prompt: "Choose template: React / Vue / Vanilla"
- [ ] Copy template files to current directory
- [ ] Run `npm install`
- [ ] Test: `nexus init` → working React app

### Milestone 4.2: Configuration Loading (Week 5)

#### Issue #35: Parse `nexus.config.ts`
**Labels**: `cli`, `config`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Use `esbuild` to transpile TS → JS
- [ ] Import as ES module
- [ ] Merge with defaults
- [ ] Test: Custom config overrides default port

#### Issue #36: Support `.env` file loading (v0.2 candidate)
**Labels**: `cli`, `config`, `v0.2-candidate`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Use `dotenv` package
- [ ] Load `.env` before starting server
- [ ] Expose as `import.meta.env`
- [ ] **NOTE**: Defer if time-constrained

### Milestone 4.3: napi-rs Bridge (Week 6-7)

#### Issue #37: Set up napi-rs bindings
**Labels**: `cli`, `napi`, `infrastructure`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Create `crates/nexus-node` with napi-rs
- [ ] Expose Rust function: `parse_file(path: string) -> AST`
- [ ] Build `.node` binary for Mac/Linux
- [ ] Test: Node.js calls Rust function → returns result

#### Issue #38: Implement Vite plugin adapter shim
**Labels**: `cli`, `napi`, `plugins`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Accept array of Vite plugins in config
- [ ] Call plugin hooks from Rust via napi-rs
- [ ] Support hooks: `resolveId`, `load`, `transform`
- [ ] Test: `@vitejs/plugin-react` works

#### Issue #39: Batch plugin calls to reduce overhead
**Labels**: `cli`, `napi`, `performance`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Group multiple hook calls into single JS invocation
- [ ] Pass array of file paths, receive array of results
- [ ] Benchmark: Overhead < 10ms for 100 files
- [ ] Document in `docs/PLUGIN_ARCHITECTURE.md`

---

## Epic 5: Examples & Benchmarks (Days 40-70)

**Owner**: Dev 2  
**Goal**: Prove Virtual Chunking works in real scenarios

### Milestone 5.1: Example Projects (Week 6-8)

#### Issue #40: Create React example (1k modules)
**Labels**: `examples`, `react`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Directory: `examples/react-1k/`
- [ ] Use `gen_repo.js` to generate 1000 components
- [ ] Add README with instructions
- [ ] Test: `nexus dev` → loads in < 500ms

#### Issue #41: Create monorepo example (10k modules)
**Labels**: `examples`, `monorepo`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Directory: `examples/monorepo-10k/`
- [ ] Generate 10,000 modules across 3 packages
- [ ] Shared dependencies between packages
- [ ] Test: Cold start < 1s

#### Issue #42: Create TypeScript + React example
**Labels**: `examples`, `typescript`, `react`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Full TypeScript setup: `tsconfig.json`, `.tsx` files
- [ ] Use path aliases (`@/components`)
- [ ] Test: TypeScript resolves and transpiles correctly

### Milestone 5.2: Benchmark Suite (Week 8-9)

#### Issue #43: Build benchmark harness
**Labels**: `benchmarks`, `infrastructure`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Script: `benchmarks/run.sh`
- [ ] Metrics: cold start time, HMR latency, memory usage
- [ ] Output: JSON file with results
- [ ] Test: Run on example projects

#### Issue #44: Add Vite comparison baseline
**Labels**: `benchmarks`, `vite`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Run same benchmarks on Vite dev server
- [ ] Record metrics in parallel
- [ ] Generate comparison table
- [ ] Test: Nexus is faster (or identify regressions)

#### Issue #45: Integrate benchmarks into CI
**Labels**: `benchmarks`, `ci`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] GitHub Actions job: `benchmark`
- [ ] Run on every PR
- [ ] Comment results on PR (bot)
- [ ] Fail if performance regresses >20%

#### Issue #46: Create benchmark visualization dashboard
**Labels**: `benchmarks`, `viz`, `v0.2-candidate`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Web page: charts of metrics over time
- [ ] Compare Nexus vs Vite side-by-side
- [ ] Host on GitHub Pages
- [ ] **NOTE**: Nice-to-have, defer if time-constrained

---

## Epic 6: Documentation & Developer Experience (Days 50-80)

**Owner**: Dev 2  
**Goal**: Make Nexus approachable for new users

### Milestone 6.1: Core Documentation (Week 8-10)

#### Issue #47: Write comprehensive README.md
**Labels**: `docs`, `readme`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Elevator pitch (2 paragraphs)
- [ ] Installation instructions
- [ ] Quick start guide
- [ ] Link to examples
- [ ] Badge: build status, license

#### Issue #48: Create ARCHITECTURE.md
**Labels**: `docs`, `architecture`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Explain Virtual Chunking
- [ ] Diagram: Module Graph → Chunks
- [ ] Explain Persistent Graph (sled)
- [ ] Code snippets from Rust kernel

#### Issue #49: Write CONTRIBUTING.md
**Labels**: `docs`, `contributing`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] How to set up dev environment
- [ ] How to run tests
- [ ] Code style guide (Clippy, Prettier)
- [ ] PR process

#### Issue #50: Add CODE_OF_CONDUCT.md
**Labels**: `docs`, `conduct`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Use Contributor Covenant template
- [ ] Specify enforcement contact

#### Issue #51: Create BENCHMARKS.md
**Labels**: `docs`, `benchmarks`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Tables: Nexus vs Vite on 1k/10k/50k repos
- [ ] Methodology explanation
- [ ] Charts (if available from #46)

### Milestone 6.2: Plugin Documentation (Week 10-11)

#### Issue #52: Document supported Vite plugins
**Labels**: `docs`, `plugins`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] List top-20 plugins
- [ ] Compatibility status: ✅ Works / ⚠️ Partial / ❌ Not supported
- [ ] Workarounds for partial support
- [ ] File: `docs/PLUGIN_COMPATIBILITY.md`

#### Issue #53: Write plugin authoring guide
**Labels**: `docs`, `plugins`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] How to write a Nexus-compatible plugin
- [ ] Example: custom `transform` hook
- [ ] Explain napi-rs bridge limitations
- [ ] Test: External contributor follows guide → working plugin

---

## Epic 7: Testing & Quality Assurance (Days 60-85)

**Owner**: All  
**Goal**: Ship with confidence - no critical bugs

### Milestone 7.1: Unit Tests (Week 9-11)

#### Issue #54: Write unit tests for Rust kernel
**Labels**: `tests`, `kernel`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Coverage >70% for core modules
- [ ] Tests: parser, resolver, graph, chunker
- [ ] Run via `cargo test`

#### Issue #55: Write unit tests for TypeScript CLI
**Labels**: `tests`, `cli`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Coverage >60%
- [ ] Tests: config parsing, command handling
- [ ] Run via `npm test`

#### Issue #56: Add integration tests for dev server
**Labels**: `tests`, `integration`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] Spawn server, make HTTP requests, verify responses
- [ ] Test: Serve chunk, HMR socket, static files
- [ ] Test isolation (separate ports)

### Milestone 7.2: End-to-End Tests (Week 11-12)

#### Issue #57: Set up Playwright for E2E testing
**Labels**: `tests`, `e2e`, `infrastructure`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Install Playwright
- [ ] Configure to test against local dev server
- [ ] Test: Load page, verify content, edit file, verify HMR

#### Issue #58: Write E2E test: React app HMR flow
**Labels**: `tests`, `e2e`, `react`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Start dev server with React example
- [ ] Load page in browser
- [ ] Edit component file
- [ ] Verify browser updates (full reload for v0.1)

#### Issue #59: Write E2E test: TypeScript compilation
**Labels**: `tests`, `e2e`, `typescript`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] TypeScript project with errors (intentionally)
- [ ] Verify build fails gracefully (error messages shown)
- [ ] Fix errors, verify build succeeds

### Milestone 7.3: Manual QA (Week 12)

#### Issue #60: Manual QA: Test on 3 real-world projects
**Labels**: `tests`, `manual-qa`  
**Estimate**: L  
**Acceptance Criteria**:
- [ ] **Project 1**: Personal blog (Gatsby/Next.js equivalent)
- [ ] **Project 2**: SaaS dashboard (10k+ components)
- [ ] **Project 3**: Documentation site (VitePress equivalent)
- [ ] Document bugs found → file issues
- [ ] Fix critical bugs before release

---

## Epic 8: Infrastructure & CI/CD (Days 1-90, ongoing)

**Owner**: Dev 2  
**Goal**: Automate everything - testing, benchmarks, releases

### Milestone 8.1: GitHub Actions Setup (Week 1-2)

#### Issue #61: Create CI workflow for Rust
**Labels**: `ci`, `rust`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Run on PR and push to `main`
- [ ] Jobs: `cargo build`, `cargo test`, `cargo clippy`
- [ ] Cache dependencies
- [ ] Fail on warnings

#### Issue #62: Create CI workflow for TypeScript
**Labels**: `ci`, `typescript`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Jobs: `npm install`, `npm test`, `npm run lint`
- [ ] Run Prettier format check
- [ ] Cache `node_modules`

#### Issue #63: Add E2E test job to CI
**Labels**: `ci`, `e2e`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Run Playwright tests
- [ ] Upload video artifacts on failure
- [ ] Run in parallel with unit tests

### Milestone 8.2: Automation (Week 10-13)

#### Issue #64: Set up automatic releases with `cargo-release`
**Labels**: `ci`, `release`  
**Estimate**: M  
**Acceptance Criteria**:
- [ ] Tag-based releases (e.g., `v0.1.0`)
- [ ] Build binaries for Mac/Linux
- [ ] Publish to crates.io (Rust) and npm (CLI)
- [ ] Generate release notes from commits

#### Issue #65: Create issue templates
**Labels**: `infrastructure`, `github`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Templates: Bug Report, Feature Request, Performance Issue
- [ ] Fields: environment, repro steps, expected behavior
- [ ] File: `.github/ISSUE_TEMPLATE/`

#### Issue #66: Create PR template
**Labels**: `infrastructure`, `github`  
**Estimate**: S  
**Acceptance Criteria**:
- [ ] Checklist: tests added, docs updated, benchmarks run
- [ ] Link to related issue
- [ ] File: `.github/pull_request_template.md`

---

## Summary Statistics

**Total Issues**: 66  
**Total Epics**: 8  
**Total Milestones**: 16

**Estimates Breakdown**:
- **Small (S)**: 15 issues (~1-2 days each) = 22 days
- **Medium (M)**: 32 issues (~3-4 days each) = 112 days
- **Large (L)**: 19 issues (~5-7 days each) = 114 days

**Total Estimated Effort**: ~248 developer-days  
**Team Capacity**: 1 founder (60 days) + 2 devs (90 days each) = **240 total days**

**Status**: Slightly over capacity - will need to defer v0.2-candidate issues if needed.

---

## Labels to Create

- `kernel` - Rust kernel work
- `cli` - TypeScript CLI work
- `chunker` - Virtual chunking logic
- `hmr` - Hot module replacement
- `cache` - Persistent graph / caching
- `plugins` - Plugin system
- `typescript` - TypeScript support
- `react` - React-specific
- `examples` - Example projects
- `benchmarks` - Performance benchmarks
- `docs` - Documentation
- `tests` - Testing (unit/integration/e2e)
- `ci` - CI/CD infrastructure
- `infrastructure` - Tooling/setup
- `v0.2-candidate` - Nice-to-have, defer if needed
- `design` - Architecture decisions

---

## Week-by-Week Roadmap

| Week | Focus | Key Milestones |
|------|-------|----------------|
| 1 | Project setup, Rust kernel scaffolding | Issues #1-5 |
| 2 | Dependency graph construction | Issues #6-9 |
| 3 | Persistent graph with sled | Issues #10-12 |
| 4 | TypeScript transpilation, CLI init | Issues #13-15, #31-34 |
| 5 | Virtual chunking basics, HMR start | Issues #16-21, #26-28 |
| 6 | Chunk mapping, napi-rs bridge | Issues #22-25, #37-39 |
| 7 | Import rewriting, HMR module invalidation | Issues #23-25, #29-30 |
| 8 | Examples, benchmarks, docs start | Issues #40-45, #47-51 |
| 9 | Unit tests, plugin docs | Issues #52-56 |
| 10 | Documentation polish | Issues #47-53 |
| 11 | Integration & E2E tests | Issues #56-59 |
| 12 | Manual QA, bug fixes | Issue #60 |
| 13 | Final polish, release prep | Issues #64-66 |

---

**Next Executable Milestone**: **Week 1 - Project Setup & Scaffolding**  
**Start with**: Issue #1 (Initialize Rust workspace structure)
