# Project Apex v0.1 MVP Specification

> **Goal**: Ship a minimal viable Nexus build tool in 90 days that proves Virtual Chunking works at scale.

## Product Scope: What's IN v0.1

### ✅ Core Features (Must-Have)

#### 1. Basic Dev Server
- **Virtual Chunking Engine**: Serve in-memory concatenated modules
- **HTTP Server**: Axum-based server on `localhost:3000` (configurable port)
- **Module Resolution**: oxc_resolver for Node.js resolution algorithm
- **ESM Parsing**: oxc parser for JavaScript/TypeScript/JSX
- **Static File Serving**: Serve public assets and HTML entry points

#### 2. Hot Module Replacement (HMR)
- **WebSocket Connection**: Bidirectional client ↔ server communication
- **File Watcher**: Detect file changes in project directory
- **Module Invalidation**: Clear affected modules from virtual chunks
- **Browser Refresh Protocol**: Send update commands to client
- **Accept "dumb" HMR**: Full page reload is acceptable; granular updates are v0.2+

#### 3. Persistent Graph (Minimal)
- **Dependency Graph Storage**: Use `sled` or `redb` to cache file → dependencies
- **Cache Invalidation**: Recompute graph on file change
- **Cold Start Optimization**: Load graph from disk on restart
- **No Cross-Session Persistence**: Graph resets on version changes (v0.2 feature)

#### 4. TypeScript Support
- **Transpilation**: Convert TS → JS via oxc (no type checking)
- **JSX/TSX**: Support React syntax out of the box
- **No Type Checking**: Use `tsc --noEmit` separately (not our responsibility)

#### 5. Basic Plugin System
- **Vite Plugin Adapter (Limited)**: Support 5 critical hooks via napi-rs:
  1. `resolveId` - Custom module resolution
  2. `load` - Custom file loading
  3. `transform` - Code transformation
  4. `configResolved` - Access final config
  5. `configureServer` - Modify dev server
- **Bundled Plugins**: Ship adapters for:
  - `@vitejs/plugin-react` (React Fast Refresh)
  - `vite-plugin-inspect` (debugging)
  - `unplugin-auto-import` (top request from ecosystem)

#### 6. Configuration
- **File**: `nexus.config.ts` (TypeScript-first)
- **Options**:
  - `root` (project root directory)
  - `port` (dev server port)
  - `plugins` (array of Vite-compatible plugins)
  - `optimizeDeps.include` (manual pre-bundle list)
- **Default**: Zero-config for React projects

#### 7. CLI Interface
- **Commands**:
  - `nexus dev` - Start dev server
  - `nexus build` - Production build (calls Rollup/esbuild for v0.1)
  - `nexus --version` - Show version
- **Location**: TypeScript package in `packages/nexus-cli`

#### 8. Production Build (Passthrough)
- **Strategy**: Delegate to **Rollup** for v0.1
- **Why**: Virtual Chunking for production requires code splitting logic (v0.2 scope)
- **User Experience**: `nexus build` runs Rollup under the hood with same config

#### 9. Benchmarking Script
- **Included**: `benchmarks/gen_repo.js` (generate 1k/10k/50k module repos)
- **Metrics**: Measure cold start, HMR latency, memory usage
- **CI Integration**: GitHub Actions runs benchmarks on every PR

#### 10. Documentation
- **README.md**: Elevator pitch, installation, quick start
- **docs/ARCHITECTURE.md**: Virtual Chunking explanation
- **docs/BENCHMARKS.md**: Performance comparisons vs Vite
- **examples/**: React app (1k modules) + monorepo example (10k)

---

## ❌ What's OUT of v0.1 (Deferred to v0.2+)

### Explicitly Cut Features

#### 1. CSS Processing
- **Removed**: CSS Modules, PostCSS, Sass
- **Workaround**: Use `<style>` tags or external CSS for now
- **v0.2 Target**: Add `transform` hooks for CSS

#### 2. Asset Handling (Images/Fonts)
- **Removed**: Automatic import of `.png`, `.svg`, `.woff`
- **Workaround**: Reference assets via `/public` directory
- **v0.2 Target**: Asset pipeline with oxc transform

#### 3. Advanced HMR
- **Removed**: React Fast Refresh, Vue HMR, Svelte HMR
- **Workaround**: Full page reload on change
- **v0.2 Target**: Integrate framework-specific HMR

#### 4. Source Maps (Partial)
- **Removed**: Column-accurate sourcemaps
- **Kept**: Line-only mappings (good enough for debugging)
- **v0.2 Target**: Use `magic-string` Rust port for full maps

#### 5. Code Splitting (Production)
- **Removed**: Automatic chunk splitting in dev mode
- **Kept**: Single virtual chunk per entry point
- **v0.2 Target**: Smart chunking based on import frequency

#### 6. Environment Variables
- **Removed**: `.env` file support, `import.meta.env`
- **Workaround**: Use Node.js `process.env` directly
- **v0.2 Target**: Add `dotenv` integration

#### 7. Server-Side Rendering (SSR)
- **Removed**: Entire SSR pipeline
- **v0.3 Target**: SSR is complex, defer until core is stable

#### 8. Multi-Framework Support
- **Removed**: Vue, Svelte, Solid, Preact
- **Kept**: React only
- **v0.2+ Target**: Add via plugin adapters

#### 9. Legacy Browser Support
- **Removed**: Polyfills, ES5 transpilation
- **Target**: Modern browsers only (ES2020+)

#### 10. Windows Support
- **Removed**: Windows paths, line endings
- **Target**: Mac/Linux only for v0.1
- **v0.2 Target**: Add Windows CI testing

#### 11. Monorepo Optimizations
- **Removed**: Turborepo-style caching, workspace awareness
- **Workaround**: Run `nexus dev` in each package separately
- **v0.3 Target**: Workspace-aware graph

#### 12. Plugin Marketplace/Discovery
- **Removed**: Plugin registry, compatibility badges
- **v0.3 Target**: Community-driven plugin ecosystem

---

## Acceptance Criteria for v0.1 Release

### Functional Requirements

1. ✅ **Cold Start < 500ms**: A 10k module React app starts dev server in under 0.5s
2. ✅ **HMR < 100ms**: File change → browser update in under 100ms (full reload acceptable)
3. ✅ **Memory < 300MB**: Dev server uses < 300MB RAM for 10k module project
4. ✅ **Vite Plugin Compat**: Top-5 plugins work without modification:
   - `@vitejs/plugin-react`
   - `unplugin-auto-import`
   - `vite-plugin-inspect`
   - `@vitejs/plugin-legacy` (partial)
   - `vite-tsconfig-paths`

### Non-Functional Requirements

1. ✅ **Documentation**: Complete README, architecture doc, 2 working examples
2. ✅ **CI/CD**: GitHub Actions runs tests, benchmarks, and Rust lints
3. ✅ **Developer Experience**: `npm create nexus@latest` scaffolds a new project
4. ✅ **Error Messages**: Clear Rust panics with actionable suggestions
5. ✅ **License**: Apache-2.0 with proper attribution

### Testing Requirements

1. ✅ **Unit Tests**: Core Rust modules (parser, resolver, chunker) have >70% coverage
2. ✅ **Integration Tests**: End-to-end dev server test (spawn server, fetch chunk, verify)
3. ✅ **Benchmark Suite**: Automated comparison vs Vite on 1k/10k/50k repos
4. ✅ **Manual QA**: Founder tests on 3 real-world projects (personal blog, SaaS app, docs site)

---

## Feature Cut Decision Framework

**For any feature request during v0.1 sprint, ask:**

1. **Does it block the Virtual Chunking proof?** → If no, defer.
2. **Can we fake it with a workaround?** → If yes, document workaround and defer.
3. **Is it needed for benchmarks?** → If no, defer.
4. **Does it require >2 days of work?** → If yes, break into smaller piece or defer.

**ONLY implement features that pass ALL four checks.**

---

## Team Allocation (90 Days)

| Role | Responsibility | Time % |
|------|---------------|--------|
| **Founder** | Rust kernel (chunker, graph, server), Architecture decisions | 60% |
| **Dev 1** | TypeScript CLI, napi-rs bridge, Plugin adapter | 30% |
| **Dev 2** | Examples, benchmarks, documentation, CI/CD | 10% |

**Note**: This is a **1 founder + 1.5 dev** allocation (Dev 2 is part-time).

---

## Success Criteria (Demo Day)

At the end of 90 days, we must be able to:

1. **Live Demo**: Show side-by-side Vite vs Nexus on a 10k module app
   - Nexus starts in < 0.5s
   - Vite takes > 4s
2. **GitHub Metrics**: 100+ stars, 10+ community issues filed
3. **Proof of Concept**: At least 1 external company testing Nexus in staging
4. **Blog Post**: Technical deep-dive published (Hacker News, Reddit r/rust)

---

## What Happens After v0.1?

### v0.2 (Next 90 Days)
- CSS processing
- Advanced HMR (React Fast Refresh)
- Source maps (column-accurate)
- Asset handling
- Windows support

### v0.3 (6-12 Months)
- Production Virtual Chunking (retire Rollup)
- SSR pipeline
- Multi-framework support
- Monorepo workspace awareness

### v1.0 (12-18 Months)
- Feature parity with Vite
- Plugin marketplace
- Enterprise support contracts

---

**Status**: Frozen as of 2026-01-18  
**Owner**: Project Apex Team  
**Review Cycle**: No changes without founder approval
