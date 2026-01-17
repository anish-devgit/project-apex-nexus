$ErrorActionPreference = "Stop"

Write-Host "Setting up Labels..."

$labels = @(
    @{name="area:kernel"; color="FBCA04"; desc="Rust core logic"},
    @{name="area:chunker"; color="C0C0C0"; desc="Virtual Chunking"},
    @{name="area:cli"; color="1D76DB"; desc="CLI and UI"},
    @{name="area:hmr"; color="d93f0b"; desc="Hot Module Replacement"},
    @{name="area:cache"; color="0e8a16"; desc="Persistent Graph / Database"},
    @{name="area:plugin"; color="5319e7"; desc="Vite Plugin Adapter"},
    @{name="area:docs"; color="0075ca"; desc="Documentation"},
    @{name="kind:benchmark"; color="c5def5"; desc="Performance testing"},
    @{name="kind:infra"; color="e99695"; desc="CI/CD, Repo setup"}
)

foreach ($label in $labels) {
    try {
        gh label create $label.name --color $label.color --description $label.desc --force
    } catch {
        Write-Host "Label $($label.name) might already exist or error: $_"
    }
}

Write-Host "Creating Issues..."

$issues = @(
    # Milestone 1: Skeleton
    @{title="[INFRA] Initialize Monorepo and CI"; body="Criteria: Cargo workspace and pnpm package structure created. CI passes on main."; label="kind:infra"},
    @{title="[KERNEL] Implement Axum Dev Server"; body="Criteria: nexus dev starts a server on localhost:3000 with proper graceful shutdown."; label="area:kernel"},
    @{title="[KERNEL] Basic File Resolution via Oxc"; body="Criteria: Can request src/main.js and receive content. Integrate oxc_resolver."; label="area:kernel"},
    @{title="[CLI] Create nexus-cli package"; body="Criteria: napi bindings configured. nexus --version works and prints version."; label="area:cli"},
    @{title="[BENCHMARK] Create Repo Generation Script"; body="Criteria: gen_repo.js capable of creating 10k module projects with realistic dependencies."; label="kind:benchmark"},
    
    # Milestone 2: Virtual Chunking
    @{title="[KERNEL] Implement Module Graph Structure"; body="Criteria: Graph struct holds Import relationships and Source content safely with Arc<RwLock>."; label="area:kernel"},
    @{title="[KERNEL] Implement Virtual Chunker Logic"; body="Criteria: Requests to /chunk/ABC return concatenated content of A, B, and C with correct delimiters."; label="area:chunker"},
    @{title="[KERNEL] Integrate oxc_parser for Import Rewriting"; body="Criteria: Imports in source files are rewritten to point to Virtual Chunk IDs instead of raw paths."; label="area:kernel"},
    @{title="[CACHE] Implement Persistent Cache (redb)"; body="Criteria: Graph state survives server restart. Cold start < 500ms using redb."; label="area:cache"},
    @{title="[HMR] WebSocket HMR Integration"; body="Criteria: Editing a file triggers a message to the client via WS."; label="area:hmr"},

    # Milestone 3: Plugin Compatibility
    @{title="[PLUGIN] Implement Shim Layer for Vite Plugins"; body="Criteria: Basic Vite React plugin works with Nexus. Shim supports minimal PluginContext."; label="area:plugin"},
    @{title="[PLUGIN] Batch napi Calls"; body="Criteria: Reduce JS-Rust boundary crossing overhead by batching events."; label="area:plugin"},
    @{title="[DOCS] Write Getting Started Guide"; body="Criteria: Clear instructions for migrating a Vite app to Nexus."; label="area:docs"},
    @{title="[BENCHMARK] Micro/Mid/Macro Benchmark Report"; body="Criteria: Comparison vs Vite recorded and published for 1k, 10k, 50k modules."; label="kind:benchmark"},
    @{title="[INFRA] Publish v0.1 to npm"; body="Criteria: npm install nexus-cli works from registry."; label="kind:infra"},

    # Detailed Issues - Kernel
    @{title="[KERNEL] Set up axum router with state management"; body="Ensure State is properly typed and sharable across handlers."; label="area:kernel"},
    @{title="[KERNEL] Implement Oxc Resolver wrapper"; body="Create a Rust-friendly wrapper around oxc_resolver for internal use."; label="area:kernel"},
    @{title="[KERNEL] Native file watcher implementation (notify)"; body="Integrate `notify` crate to listen for file changes with debouncing."; label="area:kernel"},
    @{title="[KERNEL] Build Graph struct with RWLock"; body="Design thread-safe Graph data structure."; label="area:kernel"},
    @{title="[KERNEL] Implement caching layer traits"; body="Abstract cache backend (sled/redb) behind a trait."; label="area:cache"},
    @{title="[KERNEL] Handle Sourcemap concatenation"; body="Use `magic-string` (Rust port) to generate valid sourcemaps for virtual chunks."; label="area:kernel"},
    @{title="[KERNEL] Optimistic dependency scanning"; body="Scan files for imports without full AST parse if possible (regex fallback/pre-scan)."; label="area:kernel"},
    @{title="[KERNEL] Error handling for resolution failures"; body="Graceful error pages or 500s when imports miss."; label="area:kernel"},
    @{title="[KERNEL] Implement CSS injection logic"; body="Handle CSS imports by injecting style tags or sending CSS chunks."; label="area:kernel"},
    @{title="[KERNEL] Serve static assets"; body="Serve items from public/ directory efficiently."; label="area:kernel"},
    
    # Detailed Issues - CLI & Plugin
    @{title="[CLI] Argument parsing"; body="Use `clap` to handle arguments like `dev`, `build`, `--port`."; label="area:cli"},
    @{title="[CLI] Interactive terminal output"; body="Use `crossterm` or similar for nice TUI (spinners, bars)."; label="area:cli"},
    @{title="[PLUGIN] Map Vite configureServer hook"; body="Support `configureServer` to allow middleware extensions."; label="area:plugin"},
    @{title="[PLUGIN] Map Vite resolveId hook"; body="Call into JS plugins for resolution if Rust resolution fails."; label="area:plugin"},
    @{title="[PLUGIN] Map Vite transform hook"; body="Allow JS plugins to transform code (svelte, vue, etc)."; label="area:plugin"},
    @{title="[PLUGIN] Mock Vite PluginContext"; body="Provide `emitFile`, `resolve` context methods to plugins."; label="area:plugin"},
    @{title="[PLUGIN] Handle load hook"; body="Allow plugins to load virtual content."; label="area:plugin"},

    # Detailed Issues - HMR
    @{title="[HMR] Client-side HMR runtime"; body="Inject HMR update logic (overlay, replace listener) into client bundle."; label="area:hmr"},
    @{title="[HMR] Diffing logic for graph updates"; body="Calculate efficiently what changed in the graph on file save."; label="area:hmr"},
    @{title="[HMR] Hot boundary detection"; body="Identify acceptance boundaries to avoid full reload."; label="area:hmr"},

    # Detailed Issues - Benchmark & Test
    @{title="[BENCH] Setup Hyperfine CI workflow"; body="Automate performance testing in GitHub Actions."; label="kind:infra"},
    @{title="[TEST] Integration test suite for React"; body="Ensure a standard React app loads and updates."; label="kind:infra"},
    @{title="[TEST] Unit tests for Virtual Chunker"; body="Verify chunking logic handles edge cases (concat order, delimiters)."; label="area:chunker"},
    @{title="[TEST] Memory usage tracking"; body="Instrument kernel to track heap usage under load."; label="area:kernel"},

    # Risks & Mitigations
    @{title="[RISK] Napi-rs overhead optimization"; body="Profile and optimize calls across Js-Rust boundary."; label="area:plugin"},
    @{title="[RISK] Oxc stability tracking"; body="Maintain fallback or tight version pinning for Oxc parser."; label="area:kernel"},

    # Extras to reach target
    @{title="[DOCS] Contribute to Oxc"; body="Send PRs to Oxc if bugs found."; label="area:docs"},
    @{title="[INFRA] Setup Release Drafter"; body="Automate release notes generation."; label="kind:infra"},
    @{title="[CLI] Add init command"; body="Scaffold new projects via nexus init."; label="area:cli"},
    @{title="[KERNEL] Source map visualization"; body="Tool to inspect virtual chunks."; label="area:kernel"},
    @{title="[CACHE] Cache invalidation strategy"; body="Define when to blow away persistent cache (version change)."; label="area:cache"},
    @{title="[HMR] Error overlay"; body="Show build errors in browser overlay."; label="area:hmr"},
    @{title="[PLUGIN] Support env variables"; body="Load .env files."; label="area:plugin"},
    @{title="[KERNEL] Support JSON imports"; body="Handle .json files natively."; label="area:kernel"},
    @{title="[KERNEL] Support WASM imports"; body="Handle .wasm files."; label="area:kernel"}
)

foreach ($issue in $issues) {
    Write-Host "Creating issue: $($issue.title)"
    gh issue create --title $issue.title --body $issue.body --label $issue.label
}

Write-Host "Done!"
