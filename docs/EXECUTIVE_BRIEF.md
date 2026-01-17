# Project Apex (Nexus) - Executive Engineering Brief

## The Problem: Vite's Architectural Ceiling

Vite revolutionized frontend development by leveraging native ESM, but has hit a **hard ceiling at enterprise scale** (5k+ modules). The core issue: the "unbundled" philosophy shifts bottlenecks from CPU to **Network** and **Memory**.

### Critical Pain Points

**1. Network Waterfall Hell**
- Modern browsers choke on 10,000+ concurrent module requests
- Projects with >2.5k components: page loads regress to ~6 seconds (vs 0.1s bundled)
- HTTP/2 multiplexing can't save you from request stalling

**2. Node.js Heap Exhaustion**
- Vite's `ModuleGraph` stores full dependency maps in JavaScript heap
- Large monorepos trigger: `FATAL ERROR: Ineffective mark-compacts near heap limit`
- Especially catastrophic in CI environments

**3. HMR Fragility**
- Unbundled state creates race conditions
- Rapid saves or circular deps de-sync browser state
- Forces full page reloads, destroying developer flow

**4. Dev/Prod Drift**
- esbuild (dev) vs Rollup (prod) = subtle runtime bugs
- Code works locally, fails in production (regex, CSS ordering)
- Rolldown aims to fix this but is still beta

**5. The Market Already Voted**
- **Farm** (Partial Bundling) and **Rspack** (Rust Webpack) prove demand for "Rust + Bundling"
- But Rspack inherits Webpack's complexity; Farm lacks traction

## The Solution: Nexus - The Build OS

### Core Thesis

> **"Rust Kernel, Node.js Shell"**  
> Move heavy graph logic and serving to Rust. Keep config and plugins in JS/TS for ecosystem compatibility.

### The Three Pillars

#### 1. **Virtual Chunking**
- Serve concatenated ESM modules **in-memory**, not on disk
- Reduces 10,000 HTTP requests → ~50 virtual chunks
- Sub-500ms cold start regardless of project size (1k or 50k modules)

#### 2. **Persistent Rust Graph**
- Embedded database (sled/redb) caches dependency graph
- Zero "dual-engine" drift: same Rust core for dev AND production
- Eliminates filesystem scans and pre-bundling overhead

#### 3. **Vite Plugin Compatibility**
- Drop-in adapter via napi-rs bridge
- Supports existing Vite ecosystem without forcing migration
- Batched plugin calls minimize JS/Rust boundary overhead

## Technical Stack (The "Apex" Stack)

| Component | Technology | Why |
|-----------|------------|-----|
| **Parsing** | oxc (Oxidation Compiler) | 3× faster than SWC, built for performance |
| **Resolution** | oxc_resolver | 20× faster than enhanced-resolve |
| **HTTP Server** | axum + tower | Best-in-class async I/O, native WebSocket for HMR |
| **JS Bridge** | napi-rs | Zero-copy overhead for strings/buffers |
| **Caching** | sled or redb | Embedded Rust database for persistent graph |
| **Bundling** | Custom Virtual Chunker | In-memory concatenation, not disk writes |

## Success Metrics

### v0.1 Target (90 Days)

- **Cold Start**: < 500ms for 10k module project (Vite baseline: ~4.2s)
- **HMR**: < 50ms update latency
- **Memory**: < 200MB heap for 50k module monorepo (Vite: ~2GB+)
- **Compatibility**: Support top-20 Vite plugins (unplugin, vite-plugin-*)

### Benchmark: The "Apex Challenge"

Three scenarios:
1. **Micro** (1k modules): Personal blog
2. **Mid** (10k modules): SaaS dashboard  
3. **Macro** (50k modules): Enterprise monorepo ← **PRIMARY TARGET**

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| napi-rs overhead | Batch plugin calls; only invoke JS for relevant file types |
| oxc stability | Pin versions; maintain 3-month SWC fallback |
| Sourcemap complexity | Use magic-string port; accept line-only maps for v0.1 |

## Mission Statement

> **Stop waiting for your tools.**  
> Nexus eliminates the scale ceiling in frontend tooling by treating builds as a persistent, in-memory operating system—not a disposable filesystem operation.

## Primary Pain We Solve

**For engineers in large monorepos**: Your build tool should feel instant whether you have 100 files or 100,000. Nexus makes enterprise-scale development feel like working on a static site.

---

**Status**: Ready for v0.1 implementation  
**Timeline**: 90-day sprint to MVP  
**Ownership**: 1 founder + 1-2 developers  
**Target**: Mac/Linux, Rust + TypeScript
