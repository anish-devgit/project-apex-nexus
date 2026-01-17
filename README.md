# Nexus (Project Apex) 🚀

**The Build OS for Scale.**

Nexus is a next-generation frontend toolchain written in Rust. It replaces Vite, Webpack, and Turbopack. It solves the "Vite Ceiling" in large monorepos by using **Virtual Chunking**—serving your app 100x faster by bundling in memory, not on disk.

## Why Nexus?

- ⚡ **Instantly Ready**: Sub-500ms cold start, regardless of project size (1k or 50k modules).
- 📉 **No Waterfalls**: Virtual Chunking reduces 10,000 HTTP requests to ~50.
- 🔄 **Persistent Graph**: Zero "dual-engine" drift. Same Rust core for Dev and Prod.
- 🔌 **Vite Compatible**: Drop-in adapter for the Vite plugin ecosystem.

## Quick Start

```bash
npx nexus@latest dev
```

## Architecture

"Rust Kernel, Node.js Shell." 

We move heavy graph logic to Rust (Oxidation Compiler, Axum, Redb) while keeping the flexibility of a Node.js-based plugin system.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to get started.

## License

Apache-2.0
