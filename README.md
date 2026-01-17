# Nexus (Project Apex)

> **Experimental High-Performance Web Bundler in Rust**

Nexus is a next-generation web bundler built for speed, simplicity, and modern web development. It leverages Rust's performance and the OXC parser's speed to deliver instant development server start times and highly optimized production builds.

**Status**: v0.1.0-alpha (Experimental)  
**License**: MIT

## 🚀 Why Nexus?

- **Instant Dev Server**: Starts in milliseconds. No bundling in dev mode.
- **Native TypeScript Support**: First-class support for TS/TSX without configuration.
- **Tree Shaking**: Eliminates dead code efficiently using AST analysis (Mark-and-Sweep).
- **Code Splitting**: Automatic chunk generation for dynamic `import()` statements.
- **Modern Architecture**: Built on a highly parallelized dependency graph and asset pipeline.

## 📦 Features (v0.1)

- [x] **Dev Server**: HMR (Hot Module Replacement) & Fast Refresh (Basic).
- [x] **Production Build**: 
    - Minified bundle generation (basic).
    - CSS extraction and bundling.
    - Static asset handling.
- [x] **Optimization**:
    - **Tree Shaking**: Removes unused exports.
    - **Code Splitting**: Dynamic chunks for lazy loading.
    - **Vendor Splitting**: Separate `vendor.js` for dependencies.
- [x] **Resolving**: Node-resolution algorithm compatible (supports `node_modules`).

## 🛠️ Quick Start

### Prerequisites
- Rust (latest stable)
- Node.js (for dependencies)

### Installation

Clone the repository:
```bash
git clone https://github.com/your-org/project-apex.git
cd project-apex
```

### Running the Example

1. Navigate to the example directory:
   ```bash
   cd examples/react-basic
   npm install
   ```

2. Run the Dev Server (from root):
   ```bash
   cargo run --bin nexus dev --cwd ./examples/react-basic
   ```

3. Open `http://localhost:3000`.

### Building for Production

```bash
cargo run --bin nexus build --cwd ./examples/react-basic
```
Output will be in `examples/react-basic/dist`.

## ⚠️ Limitations (Alpha)

- **Experimental**: APIs and internal logic may change significantly.
- **Plugin System**: Not yet implemented (Coming in v0.2).
- **CSS**: Basic concatenation support; no PostCSS/Tailwind integration built-in yet (can be used externally).
- **Frameworks**: Primarily tested with React. Vue/Svelte support is planned.

## 🤝 Contributing

We welcome contributions! Please check `ROADMAP.md` for upcoming features and `CONTRIBUTING.md` (coming soon) for guidelines.

1. Fork the repo.
2. Create feature branch (`git checkout -b feature/amazing`).
3. Commit changes (`git commit -m 'Add amazing feature'`).
4. Push to branch (`git push origin feature/amazing`).
5. Open a Pull Request.

---
Built with ❤️ by the Project Apex Team.
