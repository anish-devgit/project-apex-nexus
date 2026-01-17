# Project Apex Roadmap

## v0.1: The Foundation (Alpha) - **RELEASED**
- [x] Core Rust Architecture (Graph, Resolver, Compiler)
- [x] Fast Dev Server (No-bundle serving)
- [x] Production Bundler (Basic concatenation)
- [x] Dynamic Code Splitting (`import()`)
- [x] Tree Shaking (Dead Code Elimination)
- [x] CSS & Asset Support

## v0.2: Extensibility (Planned Q2 2026)
- [ ] **Plugin API**: NAPI-RS based plugin system for JS/Rust plugins.
- [ ] **Loader API**: Custom loaders for non-standard file types (.vue, .svelte, .mdx).
- [ ] **Configuration**: `nexus.config.ts` support.
- [ ] **CSS Modules**: Scoped CSS support.

## v0.3: Optimization & Scale (Planned Q3 2026)
- [ ] **Parallel Builds**: Fully parallelized artifact generation.
- [ ] **Persistent Caching**: Disk-based caching for instant cold starts.
- [ ] **Differential Serving**: Modern vs Legacy bundle generation.
- [ ] **Module Federation**: Micro-frontend support.

## v1.0: Stable Release
- [ ] Full test coverage.
- [ ] Production-grade documentation.
- [ ] Framework presets (Next.js-like features).
