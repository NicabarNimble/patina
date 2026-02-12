---
type: feat
id: plugin-system
status: ready
created: 2026-02-11
revised: 2026-02-11
sessions:
  origin: 20260211-125648
  amended: [20260211-133159, 20260211-143337, 20260211-185411]
  research: [20260205-102402, 20260205-115835, 20260205-130049]
blocked_by: []
blocks: []
related:
  - layer/surface/build/feat/patina-platform/SPEC.md
  - layer/surface/build/explore/wit-interfaces/SPEC.md
  - layer/surface/build/explore/agents-and-yolo/SPEC.md
  - layer/surface/build/feat/mother-environment/SPEC.md
  - layer/surface/build/feat/mother-repos/SPEC.md
beliefs:
  - patina-is-knowledge-layer
  - unix-philosophy
  - compiler-enforced-safety
  - transparent-complexity
  - separate-worlds-for-isolation
  - two-layer-capability-grants
  - wasi-sandboxed-filesystem
  - version-in-binary
  - sync-first
  - use-whats-in-the-tree
  - work-triages-specs
  - de-risk-runtime-with-simplest-payload
  - coupling-is-complexity
  - mother-is-the-daemon
  - dependable-rust
references:
  - "wasmtime v41 (Bytecode Alliance)"
  - "zed-industries/zed extension system (wasmtime 33, 77 WIT files)"
  - "WIT Component Model"
  - "tree-sitter WASM grammars"
  - "Zed Decoded: Extensions video (Marshall + Max)"
  - "No Boilerplate: Async Rust Is A Bad Language"
---

# feat: Plugin System

> wasmtime v41 + WIT Component Model. Synchronous. Separate worlds.
> MotherChild becomes the first WASM plugin. PluginEngine is the shared
> runtime. Mother uses it for daemon children. CLI uses it for one-shot
> commands. Grammars come last.

## Problem

Patina is a 52MB monolith. Every feature — forge, eval, yolo, 9 compiled-in
tree-sitter grammars — ships in one binary. The consequences:

1. **No extensibility** — adding a language means recompiling patina
2. **No community** — others can't extend patina without forking
3. **Binary bloat** — tree-sitter grammars alone are significant
4. **Slow iteration** — changing eval means releasing all of patina
5. **Mother has zero children** — the MotherChild trait exists but only
   SecretsCacheChild implements it (compiled-in)

The MotherChild trait (`src/mother/child.rs`) already defines the plugin shape:
`name()`, `on_load()`, `handle()`, `health()`, `tick()`. The ChildRegistry
(`src/commands/mother/registry.rs`) already loads, routes, and health-checks
children. The MotherHost trait already defines the capability surface. The WIT
interfaces are sketched in [[wit-interfaces]]. The runtime decision (wasmtime)
is made. What's missing is the concrete build.

## Consumes From Frozen Specs

This spec selectively consumes from frozen specs per [[work-triages-specs]]:

| Frozen Spec | What we consume | What we leave |
|-------------|----------------|---------------|
| [[patina-platform]] | wasmtime decision, two-layer capability grants, core/plugin boundary table, plugin manifest format, plugin lifecycle | Work plugin (patina-work), plugin registry, distribution/marketplace |
| [[wit-interfaces]] | host.wit (eventlog, layer, database), oracle.wit, scraper.wit, sync/async transparency pattern, WASI sandboxing pattern, parallelism options table | adapter.wit (adapters stay compiled for now), work.wit (deferred) |
| [[agents-and-yolo]] | Yolo extraction decision (extract to plugin) | Agent concept (defer indefinitely) |
| [[mother-environment]] | Models child design (MotherChild for embedding models) | Cold-start optimization (separate concern) |
| [[mother-repos]] | Repos child design (MotherChild for ref repo lifecycle) | Belief extraction from ref repos (future) |

Specs not consumed after this build completes are candidates for archival.

---

## Solution

### Runtime: wasmtime v41 + WIT Component Model

**Version:** Pin `wasmtime = "=41"`. wasmtime releases major versions every
~3 months. Pin exact major, update deliberately. Zed uses v33 — we go latest
since we have no legacy to support. (Originally planned v43 but latest on
crates.io is v41.0.3 as of 2026-02-11.)

**Minimum Rust:** 1.90.0 (wasmtime v41 requirement). Our current toolchain
supports this.

**Not Extism.** wasmtime is production-proven (Zed, Fastly, Fermyon), supports
WIT Component Model for typed interfaces, and the Bytecode Alliance maintains
it. Decision made in session [[20260205-115835]] after Zed deep dive.

**Cargo.toml features (Phase 1 — wasmtime only, no wasmtime-wasi):**

```toml
wasmtime = { version = "=41", default-features = false, features = [
    "runtime",          # Execution engine (required)
    "cranelift",        # JIT compiler backend
    "component-model",  # WIT Component Model support
] }
```

Phase 1's `mother-child` world only imports `patina:host/log`, which we
implement ourselves on `HostState`. No WASI interfaces needed yet.

**Cargo.toml features (Phase 2+ — add when WASI sandboxing needed):**

```toml
wasmtime-wasi = { version = "=41", default-features = false, features = [
    "p2",               # WASIp2 component model (note: enables wasmtime/async
                        # Cargo feature at compile time, but this does NOT force
                        # async runtime — use add_to_linker_sync())
] }
```

**Async Cargo feature vs runtime clarification:** wasmtime-wasi's `p2` feature
enables `wasmtime/async` as a Cargo feature (compile-time gate). This makes
async APIs *available* but does NOT force `Config::async_support(true)`. The
default is `async_support(false)` — sync APIs work fine. Use
`wasmtime_wasi::p2::add_to_linker_sync()` when WASI is added. The `async`
Cargo feature adds `wasmtime-fiber` to the dep tree (some compile cost) but
zero runtime impact.

**Features we do NOT enable:**
- `async` (on wasmtime itself) — per [[sync-first]], we never call
  `Config::async_support(true)`. The `async` Cargo feature pulled in
  transitively by wasmtime-wasi `p2` is acceptable — it's compile-time only.
- `cache` — incremental compilation cache. Add later if cold start is too slow.
- `demangle` — nice for debugging stack traces, add if needed.

### Target: `wasm32-wasip2`

Plugins compile to `wasm32-wasip2` (WASI Preview 2 = Component Model).
Not `wasm32-wasi` (Preview 1, core modules only).
Not `wasm32-unknown-unknown` (no WASI, too limited).

```bash
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --release
```

### Architecture: PluginEngine (Option C)

PluginEngine is the shared wasmtime infrastructure. Mother uses it for
resident daemon children. CLI uses it directly for one-shot command plugins.
Same WASM loading, same capability grants, same manifest format — different
lifecycles.

```
┌───────────────────────────────────────────────────────────┐
│                     patina (core binary)                   │
│                                                            │
│  PluginEngine (shared wasmtime guts)                       │
│  ├── wasmtime::Engine (singleton via OnceLock)             │
│  ├── wasmtime::component::Linker<HostState>                │
│  ├── load_component(path) → Component                     │
│  ├── instantiate(component, manifest) → instance           │
│  └── capability_check(manifest, grants) → Result           │
│                                                            │
│  Mother daemon                    CLI direct               │
│  ├── PluginEngine ref             ├── PluginEngine ref     │
│  ├── ChildRegistry                └── load on demand       │
│  │   ├── SecretsCacheChild          doctor, eval, yolo     │
│  │   │   (compiled-in)              run and exit           │
│  │   └── WasmChild (WASM)                                  │
│  │       resident, heartbeat                               │
│  └── daemon-specific:                                      │
│      graph, cross-project,                                 │
│      model caching                                         │
└───────────────────────────────────────────────────────────┘
        │              │              │
        ▼              ▼              ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │ children│   │ commands│   │ grammars│
   │ (.wasm) │   │ (.wasm) │   │ (.wasm) │
   │         │   │         │   │         │
   │ models  │   │ doctor  │   │ rust    │
   │ repos   │   │ yolo    │   │ python  │
   │         │   │ eval    │   │ go      │
   │         │   │ report  │   │ ...     │
   └─────────┘   └─────────┘   └─────────┘
   Mother-managed  ~/.patina/    ~/.patina/
   (MotherChild)   plugins/      grammars/
```

Mother's role is clear per [[mother-is-the-daemon]]: **Mother is the daemon
that runs long-lived plugins and provides cross-project awareness.** She uses
the same PluginEngine everyone else does, but adds the resident lifecycle
(load, tick, health, toys). She's not the gatekeeper for all plugins — she's
the home for plugins that need to stay alive.

CLI plugins don't need Mother running. `patina doctor` works offline and
daemonless, same as today.

### Threading Model: Sync-First with Scoped Threads

Per [[sync-first]] and session [[20260205-115835]] (Zed deep dive + No
Boilerplate video analysis):

**Plugins see synchronous APIs. Always.**

**Host execution model:** `std::thread::scope` for plugin calls. No async
feature in wasmtime. No tokio. Preserves borrowing, zero async infection.

```rust
// Host calls into WASM — synchronous, scoped
fn call_plugin_handle(
    store: &mut Store<HostState>,
    instance: &MotherChildInstance,
    request: &ChildRequest,
) -> Result<ChildResponse> {
    // wasmtime component call is synchronous
    instance.call_handle(store, &request.action, &request.payload)
}
```

**Host functions (host → WASM imports) are also synchronous:**

```rust
// wasmtime::component::bindgen! WITHOUT async: true
wasmtime::component::bindgen!({
    path: "wit/",
    // NO async: true — everything is sync
    // NO trappable_imports: true — use Result instead
});
```

**If we ever need async I/O in host functions** (e.g., HTTP for forge plugins
in Phase 4), the escalation path is:

1. **First try:** Blocking I/O in the host function (reqwest blocking client
   is already in tree). WASM call blocks until I/O completes. Fine for CLI.
2. **If blocking isn't enough:** Contained tokio runtime (2 threads max) in
   ONE module (`src/plugin/runtime.rs`). Never export async types. Same
   pattern as Zed's `gpui_tokio` bridge.
3. **Never:** `async` feature on wasmtime, tokio in public API, `'static`
   lifetime infection.

Reference: [[wit-interfaces]] parallelism options table.

### Separate Worlds Per Plugin Type

Per [[separate-worlds-for-isolation]], each plugin type gets its own WIT world
with only the imports it needs. This **diverges from Zed** which uses a single
`world extension` for everything. We trade flexibility for security: oracle
plugins can't see HTTP imports, grammar plugins can't see the eventlog.

| World | Exports | Imports | Capabilities |
|-------|---------|---------|-------------|
| `mother-child` | `handle()`, `health()`, `tick()` | `patina:host/log` | Log only (Phase 1). Add more per child. |
| `command` | `run(args)` → exit code | `patina:host/*` | Full host access |
| `oracle` | `query()`, `name()`, `is-available()` | none | Pure computation |
| `scraper` | `scrape-file()`, `patterns()` | `wasi:filesystem` (read-only) | Filesystem read |
| `forge-reader` | `list-issues()`, `get-issue()`, etc. | `wasi:http` | Network access |
| `grammar` | tree-sitter parse API | none | Pure computation, fully sandboxed |

### WIT Package Definitions

Top-level `wit/` directory in the repo root. Package versioning (not directory
versioning like Zed's `since_v0.x.0/`). Add directory versioning later when
we need backward compatibility.

```
wit/
├── host.wit           # patina:host@0.1.0 — log, layer, database, eventlog
├── mother-child.wit   # patina:mother-child@0.1.0 — world definition
├── command.wit        # patina:command@0.1.0 — world definition (Phase 2)
├── oracle.wit         # patina:oracle@0.1.0 — world definition (Phase 4)
└── scraper.wit        # patina:scraper@0.1.0 — world definition (Phase 4)
```

**host.wit** (from [[wit-interfaces]], refined for Phase 1):

```wit
package patina:host@0.1.0;

/// Structured logging for plugins
interface log {
    /// Log a message at the given level
    log: func(level: log-level, message: string);

    enum log-level {
        debug,
        info,
        warn,
        error,
    }
}

/// Layer file access for plugins
interface layer {
    /// Read a layer file
    read: func(path: string) -> result<option<string>, string>;

    /// Write a layer file (git-tracked)
    write: func(path: string, content: string) -> result<_, string>;

    /// List files matching glob in layer
    glob: func(pattern: string) -> result<list<string>, string>;
}

/// Database access for plugins (plugin-scoped)
interface database {
    /// Execute SQL (CREATE, INSERT, UPDATE, DELETE)
    execute: func(sql: string, params: list<string>) -> result<u64, string>;

    /// Query SQL (SELECT), returns JSON rows
    query: func(sql: string, params: list<string>) -> result<string, string>;
}

/// Eventlog access for plugins
interface eventlog {
    /// Emit an event to the eventlog
    emit: func(event-type: string, data: string) -> result<s64, string>;

    /// Query events by type prefix
    query: func(type-prefix: string, limit: u32) -> result<list<string>, string>;
}
```

**mother-child.wit** (Phase 1):

```wit
package patina:mother-child@0.1.0;

/// World for Mother daemon children — long-lived, heartbeat, toys
world mother-child {
    /// Import host logging
    import patina:host/log;

    /// Plugin identity
    export name: func() -> string;

    /// Called when Mother loads this child
    export on-load: func() -> result<_, string>;

    /// Called when Mother shuts down
    export on-unload: func();

    /// Health check — Mother calls on heartbeat
    export health: func() -> child-health;

    /// Handle a routed request
    export handle: func(action: string, payload: string) -> result<string, string>;

    /// Heartbeat tick — return toy requests as JSON list
    export tick: func() -> list<toy>;
}

/// Child health status
enum child-health {
    healthy,
    degraded,
    unhealthy,
}

/// Work request from child to Mother
record toy {
    name: string,
    command: string,
    args: list<string>,
}
```

### Bindgen Strategy

**Host side (patina binary):** `wasmtime::component::bindgen!` macro. Built
into wasmtime, generates traits we implement. No external `wit-bindgen` crate
needed on the host.

```rust
// src/plugin/internal.rs
wasmtime::component::bindgen!({
    path: "wit/",
    world: "mother-child",
    // Sync — no async: true
});
```

This generates:
- `MotherChild` struct with `instantiate()` and `call_*()` methods
- `Host` trait for `patina:host/log` that we implement on `HostState`
- Type mappings for `child-health`, `toy`, etc.

**Guest side (plugin crates):** `wit-bindgen` crate (v0.41). Generates
guest-side bindings that match the WIT signatures.

```toml
# patina-plugin-api/Cargo.toml
[dependencies]
wit-bindgen = "0.41"

[package.metadata.component]
target = { path = "../wit" }
```

This is exactly Zed's pattern: `wasmtime::component::bindgen!` on host,
`wit-bindgen` on guest.

### Plugin Manifest (plugin.toml)

```toml
[plugin]
name = "patina-models"
version = "0.1.0"
description = "Embedding model path resolution for Mother daemon"
world = "mother-child"          # Which WIT world
patina_min = "0.17.0"          # Minimum core version

[capabilities]
# Only what this plugin needs — host checks against granted set
host_log = true                # Structured logging (always granted)

[provides]
child = "models"               # Registers as MotherChild with this name
```

### Two-Layer Capability Grants

Per [[two-layer-capability-grants]] (learned from Zed's `CapabilityGranter`):

1. **Manifest declares** — plugin.toml `[capabilities]` says what it wants
2. **Host decides** — PluginEngine checks manifest against user's grant config

```toml
# ~/.patina/plugin-config/grants.toml
[patina-models]
host_log = true

[patina-eval]
host_database = true
host_layer = true

[patina-forge-gitlab]
wasi_http = true          # Network access for GitLab API
```

Plugins that request capabilities not in the grant config are loaded in
degraded mode (capabilities denied, plugin notified via on_load error).

Default plugins (shipped with patina) are auto-granted. Third-party plugins
require explicit grants.

### Version In Binary

Per [[version-in-binary]] (learned from Zed): embed the plugin API version
in the WASM binary at build time. The host reads it to dispatch to the correct
interface version at load time. `patina-plugin-api` handles this automatically.

```rust
// patina-plugin-api/src/lib.rs
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0]; // major.minor.patch
```

Host reads this section before instantiation. Version mismatch = fail fast
with clear error, not runtime crash.

### WASI Sandboxing

Per [[wasi-sandboxed-filesystem]] (learned from Zed's `path_from_extension()`
pattern):

Each plugin gets an isolated work directory:
```
~/.patina/plugins/{plugin-name}/work/
```

The WASI context maps `/work/` in the plugin's virtual filesystem to this
real path. Plugins cannot escape their sandbox.

```rust
// Host sets up WASI context per plugin (Phase 2+ — requires wasmtime-wasi)
let wasi = WasiCtxBuilder::new()
    .preopened_dir(&plugin_work_dir, "/work", DirPerms::all(), FilePerms::all())?
    .build();
```

Plugins that don't declare filesystem capabilities get no preopened directories.

---

## Phase 1: PluginEngine + First MotherChild (v0.17.0)

### Goal

Add wasmtime, build PluginEngine, implement the first MotherChild (models)
as a WASM plugin. This proves the full host↔plugin communication pattern —
WIT interfaces, host functions, capability grants — where the trait already
exists.

### Why MotherChild First

The MotherChild trait (`src/mother/child.rs`) already defines the plugin
shape. The ChildRegistry already loads, routes, and health-checks children.
Building the first child as WASM lets the PluginEngine API emerge from a real
use case, not a spec diagram.

### Why NOT Grammars First

Session [[20260211-133159]] discovered grammars are the most coupled existing
subsystem (ABI versioning, patina-metal build, 8 language processors), not
the least. The original ordering assumed "no host imports = simplest" but
ignored infrastructure coupling. Per [[coupling-is-complexity]]: simplest
payload means lowest coupling, not fewest interface requirements.

### Models Child Scope (Phase 1)

The `models` child owns **model path resolution only**.

- `handle("resolve_model", {"name": "e5-base-v2"})` → returns model directory path
- `handle("model_status", {"name": "e5-base-v2"})` → returns cache/local/provenance info
- `health()` → checks if default model is available

The models child does **NOT** own:
- Embed requests (ONNX runtime stays in core — it's Foundation per [[patina-identity]])
- Model downloads (stays in `src/models/download.rs` — needs HTTP, not appropriate for Phase 1 child)
- Model registry (stays in `src/embeddings/models.rs`)

This is the lowest-coupling starting point. The child imports `patina:host/log`
only. Future children (repos) will test more capabilities.

### Files Created

```
wit/
├── host.wit                           # patina:host@0.1.0
└── mother-child.wit                   # patina:mother-child@0.1.0

src/plugin/
├── mod.rs                             # PluginEngine pub interface
└── internal.rs                        # wasmtime guts, bindgen, loading

patina-plugin-api/                     # Workspace member — guest bindings
├── Cargo.toml                         # wit-bindgen, crate-type = ["cdylib"]
├── src/lib.rs                         # MotherChild trait, register! macro
└── wit -> ../wit                      # Symlink to shared WIT

patina-plugin-models/                  # Workspace member — first child
├── Cargo.toml                         # depends on patina-plugin-api
├── plugin.toml                        # Manifest
└── src/lib.rs                         # Models child implementation
```

### Files Modified

```
Cargo.toml                             # Add wasmtime deps, workspace members
src/lib.rs                             # pub mod plugin
src/commands/mother/daemon.rs          # Init PluginEngine, discover WASM children
src/commands/mother/registry.rs        # WasmChild adapter (Box<dyn MotherChild>)
src/paths.rs                           # plugin module (plugin dirs, children dir)
```

### PluginEngine Struct

```rust
// src/plugin/mod.rs — public interface
mod internal;
pub use internal::{PluginEngine, PluginManifest};

// src/plugin/internal.rs — implementation
use std::path::Path;
use std::sync::OnceLock;
use anyhow::Result;
use wasmtime::{Config, Engine, Store};
use wasmtime::component::{Component, Linker};

// Generated by wasmtime::component::bindgen!
// (trait impls for patina:host/log, etc.)

/// Host state passed to WASM via Store<HostState>
pub(crate) struct HostState {
    // WASI context (if plugin needs filesystem)
    // Capability grants
    // Plugin name (for logging)
}

/// Shared wasmtime engine — singleton per process.
/// OnceLock pattern from Zed's wasm_engine().
fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config.wasm_component_model(true);
        // NO config.async_support(true) — sync-first
        Engine::new(&config).expect("failed to create wasmtime engine")
    })
}

pub struct PluginEngine {
    linker: Linker<HostState>,
}

impl PluginEngine {
    pub fn new() -> Result<Self>;

    /// Load and parse a plugin manifest
    pub fn load_manifest(path: &Path) -> Result<PluginManifest>;

    /// Load a WASM component from bytes
    pub fn load_component(&self, wasm: &[u8]) -> Result<Component>;

    /// Instantiate a MotherChild from WASM component + manifest.
    /// Returns Box<dyn MotherChild> for ChildRegistry compatibility.
    pub fn instantiate_child(
        &self,
        component: &Component,
        manifest: &PluginManifest,
    ) -> Result<Box<dyn patina::mother::MotherChild>>;
}

#[derive(Debug)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub world: String,
    pub patina_min: String,
    pub capabilities: Vec<String>,
    pub provides: PluginProvides,
}

#[derive(Debug)]
pub struct PluginProvides {
    pub child: Option<String>,       // MotherChild name
    pub commands: Vec<String>,       // CLI commands (Phase 2+)
}
```

### ChildRegistry Integration

The registry stays unchanged. WASM children are wrapped in a `WasmChild`
adapter that implements `MotherChild`:

```rust
// src/plugin/internal.rs

/// Adapter: wraps a WASM component instance as a MotherChild
struct WasmChild {
    name: String,
    store: Store<HostState>,
    instance: MotherChildInstance, // Generated by bindgen
}

impl MotherChild for WasmChild {
    fn name(&self) -> &str { &self.name }

    fn on_load(&mut self, host: &dyn MotherHost) -> Result<()> {
        self.instance.call_on_load(&mut self.store)
            .map_err(|e| anyhow::anyhow!("WASM on_load failed: {}", e))
    }

    fn health(&self) -> ChildHealth {
        // call_health requires &mut store — use RefCell or similar
        // This is an implementation detail
    }

    fn handle(&self, request: &ChildRequest) -> Result<ChildResponse> {
        let payload_json = serde_json::to_string(&request.payload)?;
        let result = self.instance.call_handle(
            &mut self.store, &request.action, &payload_json
        )?;
        Ok(ChildResponse {
            payload: serde_json::from_str(&result)?,
        })
    }

    fn tick(&mut self) -> Vec<Toy> {
        // call_tick, deserialize toys
    }
}
```

**Daemon startup** (`src/commands/mother/daemon.rs`):

```rust
let mut registry = ChildRegistry::new();

// Compiled-in children (always available)
registry.register(Box::new(SecretsCacheChild::new()));

// WASM children (discovered from ~/.patina/children/)
match PluginEngine::new() {
    Ok(plugin_engine) => {
        let children_dir = paths::plugin::children_dir();
        if children_dir.exists() {
            for entry in std::fs::read_dir(&children_dir)?.flatten() {
                let path = entry.path();
                if path.extension() == Some("wasm".as_ref()) {
                    let manifest_path = path.with_extension("toml");
                    match load_wasm_child(&plugin_engine, &path, &manifest_path) {
                        Ok(child) => {
                            eprintln!("[mother] loaded WASM child: {}", child.name());
                            registry.register(child);
                        }
                        Err(e) => eprintln!("[mother] failed to load {}: {}", path.display(), e),
                    }
                }
            }
        }
    }
    Err(e) => eprintln!("[mother] plugin engine init failed: {} (WASM children disabled)", e),
}
```

**No fallback for Phase 1.** The models child is new — it doesn't exist as
compiled-in code today. If WASM loading fails, Mother starts without it
(exactly like today — Mother has zero model children). Fallback becomes
relevant in Phase 2+ when extracting existing commands.

### Build Steps

1. Add `wasmtime` to `Cargo.toml` with exact features (wasmtime-wasi deferred to Phase 2+)
2. Create `wit/host.wit` — `patina:host@0.1.0` with `log` interface
3. Create `wit/mother-child.wit` — `patina:mother-child@0.1.0` world
4. Create `src/plugin/mod.rs` — PluginEngine public interface
5. Create `src/plugin/internal.rs` — wasmtime bindgen, Engine singleton, WasmChild adapter
6. Add `plugin` paths to `src/paths.rs` — children_dir, plugin work dirs
7. Create `patina-plugin-api/` crate — guest-side wit-bindgen, MotherChild trait, register macro
8. Create `patina-plugin-models/` crate — models child using patina-plugin-api
9. Compile models child to `wasm32-wasip2`
10. Update `src/commands/mother/daemon.rs` — PluginEngine init, WASM child discovery
11. Implement `plugin.toml` manifest parsing (toml — already in tree per [[use-whats-in-the-tree]])
12. Implement capability grant checking

### Acceptance Criteria

- [ ] `wasmtime` compiles and links (binary size delta measured)
- [ ] PluginEngine initializes wasmtime::Engine in <100ms (measured)
- [ ] `patina-plugin-api` crate compiles to `wasm32-wasip2` target
- [ ] `patina-plugin-models` crate compiles to `.wasm` file
- [ ] Models child loads as WASM plugin in Mother daemon
- [ ] `handle("resolve_model", ...)` returns model path through WASM boundary
- [ ] `handle("model_status", ...)` returns status through WASM boundary
- [ ] `health()` returns correct status through WASM boundary
- [ ] Plugin sees sync APIs only — no async anywhere
- [ ] Plugin crash doesn't crash the host (WASM isolation)
- [ ] `patina mother status` shows WASM-loaded child with health
- [ ] `patina mother start` with WASM child works end-to-end

### Exit Criteria

- [ ] Round-trip latency through WASM boundary <1ms for `handle()` calls
- [ ] At least one MotherChild loaded from WASM in CI test (`cargo test`)
- [ ] `wasmtime::Engine::new()` time measured and documented

---

## Repos Child (Phase 1+)

The repos child from [[mother-repos]] is the second MotherChild after models.
It owns ref repo lifecycle: git pull, scrape, index, freshness monitoring.

**Build after Phase 1 proves the pattern.** Repos child needs more capabilities
than models (shell commands for git via toys, scrape pipeline access) and is a
good test of the toy system (child requests work, Mother runs it).

Not a separate phase — it's the natural second child once the MotherChild WASM
pattern works.

---

## Phase 2: Command Plugins — First Extraction (v0.17.0)

**Goal:** Extract `doctor` (278 lines) from the binary into a WASM command
plugin. This proves the `command` world — a plugin that adds CLI subcommands
and runs without Mother.

**Why doctor:** Smallest extractable command. Reads files, checks state,
prints output. No hot-path risk. Proves PluginEngine works for CLI-direct
loading (no daemon required).

**Build steps:**

1. Define `wit/command.wit` — `patina:command@0.1.0` world; exports `run(args: list<string>) -> s32`; imports `patina:host/layer` (read-only)
2. Create `patina-doctor` crate (workspace member, compiles to WASM)
3. Move doctor logic from `src/commands/doctor/` to `patina-doctor` crate
4. CLI loads command plugin via PluginEngine when `patina doctor` is invoked
5. Feature-gate compiled-in doctor during transition (`--features bundled-doctor`)

**CLI plugin discovery:** Manifest cache file at `~/.patina/plugin-cache.toml`
listing installed plugin commands. Updated on `patina plugin install/remove`.
Avoids scanning `~/.patina/plugins/` on every CLI invocation.

**Acceptance criteria:**

- [ ] `patina doctor` works identically from WASM plugin
- [ ] Works without Mother daemon running
- [ ] `patina plugin list` shows patina-doctor with version and status
- [ ] Main binary smaller with doctor extracted (measurable delta)

---

## Phase 3: Remaining Command Extractions (v0.18.0)

**Goal:** Extract yolo, eval+bench, report, upgrade into WASM command plugins.
These are the remaining "Definitely Plugin" modules from [[patina-identity]].

| Plugin | Lines | World | Capabilities |
|--------|-------|-------|-------------|
| `patina-yolo` | 1,613 | command | host_layer (read), environment detection |
| `patina-eval` | 2,476 | command | host_database (read), host_layer (read) |
| `patina-bench` | 753 | command | host_database (read) |
| `patina-report` | ~400 | command | host_layer (read), host_database (read) |
| `patina-upgrade` | 162 | command | wasi:http (check GitHub releases) |

**Acceptance criteria:**

- [ ] All 5 plugins work identically as WASM
- [ ] Binary size reduced measurably (target: <40MB from 52MB)
- [ ] `patina plugin list` shows all default plugins
- [ ] Removing a plugin.wasm file gracefully degrades (command not found, not crash)

---

## Phase 4: Oracle & Scraper Plugins (v0.19.0)

**Goal:** Make the serve and capture pipelines extensible. Third-party oracles
and scrapers can be loaded as WASM plugins.

**Build steps:**

1. Define `wit/oracle.wit` — `patina:oracle@0.1.0` world (from [[wit-interfaces]] — already sketched)
2. Define `wit/scraper.wit` — `patina:scraper@0.1.0` world (from [[wit-interfaces]])
3. Refactor `retrieval/oracle.rs` — oracle fusion queries both compiled-in and WASM oracles
4. Refactor `scrape code` — scraper pipeline checks for WASM scrapers matching file extension
5. Create example oracle plugin
6. Create example scraper plugin

**Acceptance criteria:**

- [ ] WASM oracle participates in scry fusion alongside compiled-in oracles
- [ ] WASM scraper runs during `patina scrape code` for matching file patterns
- [ ] Oracle plugin: pure computation, no capabilities required
- [ ] Scraper plugin: `wasi:filesystem` read-only, sandboxed to project directory

---

## Phase 5: Grammar Plugins (v0.20.0)

**Goal:** Load tree-sitter grammars from WASM instead of compiling them in.
Most complex integration due to tree-sitter ABI versioning and scrape hot path.

**Why last:** Grammars are entangled with tree-sitter ABI versioning (0.24
expects ABI 13-14), patina-metal `cc::Build` + vendored C sources, and 8
language processors on the scrape hot path. By Phase 5, PluginEngine is proven.

**Grammar fallback:** If WASM grammar not found in `~/.patina/grammars/`,
fall back to compiled-in. Zero regression for existing users.

**Acceptance criteria:**

- [ ] `patina scrape code` uses WASM grammar when present
- [ ] Falls back to compiled-in grammar when WASM not present
- [ ] Adding a new language is: drop a `.wasm` file, no recompile
- [ ] WASM grammar parse speed within 2x of compiled-in

---

## Resolved Decisions

### 1. MotherChild first, grammars last

Per [[coupling-is-complexity]] and [[de-risk-runtime-with-simplest-payload]]:
de-risk means de-risk the *plugin system*, not the *tree-sitter integration*.
The trait exists, the registry works, the host capability surface exists.
Grammars have the highest coupling and regression risk.

**Amendment:** Session [[20260211-133159]].

### 2. PluginEngine shared, Mother is daemon face (Option C)

PluginEngine holds wasmtime::Engine, both Mother and CLI use it. Difference
is lifecycle: resident vs one-shot. `patina doctor` works without daemon.

**Origin:** Session [[20260211-133159]].

### 3. Sync-first, no async runtime

wasmtime without `Config::async_support(true)`. `std::thread::scope` for
plugin calls. Plugins see sync APIs. Host uses blocking I/O (reqwest blocking
already in tree). Escalation path to contained tokio only if needed.

**Clarification (session [[20260211-185411]]):** The `async` **Cargo feature**
and `Config::async_support(true)` **runtime setting** are distinct.
wasmtime-wasi `p2` enables the async Cargo feature (compile-time), but we
never call `async_support(true)` (runtime). Sync APIs work fine with the
async feature compiled in. Phase 1 avoids this entirely by not using
wasmtime-wasi — we implement `patina:host/log` ourselves.

**Origin:** Session [[20260205-115835]], [[sync-first]] belief, No Boilerplate video.

### 4. Separate worlds, not Zed's single world

Each plugin type gets its own WIT world. Stricter capability isolation.
Oracle plugins can't see HTTP. Grammar plugins can't see eventlog.

**Origin:** Session [[20260205-115835]], [[separate-worlds-for-isolation]] belief.

### 5. wasmtime v41, pinned exact

Latest stable as of 2026-02-11 is v41.0.3. Pin `=41`. Update deliberately.
Zed is on 33 but we have no legacy. Minimum Rust 1.90.0. Originally planned
v43 but it doesn't exist on crates.io yet. wasmtime-wasi deferred to Phase 2+
(feature name is `p2`, not `preview2` — the `preview2` module was promoted to
crate root in wasmtime-wasi v22+).

**Origin:** Session [[20260211-143337]] research.
**Amended:** Session [[20260211-185411]] — version corrected, wasmtime-wasi deferred.

### 6. `wasmtime::component::bindgen!` host, `wit-bindgen` guest

Exactly Zed's pattern. Host uses wasmtime's built-in macro. Guest uses
wit-bindgen crate. No external bindgen tool needed.

**Origin:** Session [[20260205-115835]] Zed analysis.

### 7. Version embedded in WASM binary

Per [[version-in-binary]]. `patina-plugin-api` embeds version in link section.
Host reads before instantiation. Fail fast on mismatch.

**Origin:** Session [[20260205-115835]], Zed Decoded video.

### 8. WASI sandboxed filesystem

Per [[wasi-sandboxed-filesystem]]. Each plugin gets isolated work directory.
Virtual `/work/` path mapped to `~/.patina/plugins/{name}/work/`. Plugins
can't escape sandbox.

**Origin:** Session [[20260205-115835]], Zed's `path_from_extension()`.

### 9. Separate crates, not separate repos

Plugins are workspace members in monorepo. Community plugins use separate repos.

### 10. Feature flags during transition

Compiled-in versions behind `--features bundled-*` during transition.
Remove once WASM versions are stable.

### 11. Models child scope: path resolution only

`resolve_model` and `model_status` only. ONNX runtime stays in core (Foundation
per [[patina-identity]]). Model downloads stay in `src/models/`. This is the
lowest-coupling starting point for proving the WASM boundary.

### 12. Plugin discovery: manifest cache

`~/.patina/plugin-cache.toml` lists installed plugin commands. Updated on
install/remove. Avoids scanning plugin directory on every CLI invocation.

---

## What We Don't Build

Per [[patina-identity]] "What Patina IS NOT":

- **Plugin registry/marketplace** — manual install first. Registry is a future spec.
- **Hot reloading** — restart patina to load new plugins. KISS.
- **Plugin dependencies** — plugins don't depend on other plugins. No dependency hell.
- **adapter.wit** — adapters stay compiled-in for now. Mother may manage them later.
- **patina-work plugin** — beads-like work tracking is a future plugin, not this spec.
- **Agent system** — per [[agents-and-yolo]], defer indefinitely.
- **Async wasmtime runtime** — per [[sync-first]], never call
  `Config::async_support(true)`. The `async` Cargo feature may be compiled in
  transitively (via wasmtime-wasi `p2`) but that's compile-time only.
- **Directory-versioned WIT** — start with package versions. Add Zed-style
  `since_v*` directories when backward compatibility requires it.

## Non-Goals

- Plugin monetization
- Plugin sandboxing beyond WASI (no seccomp, no namespace isolation)
- Multi-version plugin support (one version at a time)
- Plugin auto-update (manual for now)
- Cross-platform WASM testing (compile on macOS, test on Linux — future)

---

## Discoveries (pushed from audit)

Pushed from [[plugin-system-audit-remediation]] per [[specs-push-discoveries-outbound]].
Source: `layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md`

### Phase 2: Move WIT types outside world block

**Audit ref:** 1.3 (minor)

`child-health` and `toy` are defined inside `world mother-child {}` in
`wit/mother-child.wit`. Types inside a world are scoped to that world and
cannot be imported by other worlds. When Phase 2 defines `wit/command.wit`,
the `command` world may need `child-health`. Move types outside the world
block before defining new worlds.

### Phase 2+: Re-entrancy invariant for host functions

**Audit ref:** 2.5 (minor)

The store Mutex is held during WASM calls. If a future host function (beyond
`patina:host/log`) tries to acquire the store Mutex or call WASM methods on
the same instance, it will deadlock. When adding `patina:host/layer`,
`patina:host/database`, or `patina:host/eventlog` host functions, document
and enforce this invariant:

> Host function implementations MUST NOT acquire the store Mutex or call
> WASM methods on the same instance.

### Phase 2: ChildHealth WIT type needs reason string

**Audit ref:** 3.3 (important)

The WIT `child-health` enum is `{ healthy, degraded, unhealthy }` — no reason
string. The Rust `ChildHealth` has `Degraded(String)` and `Unhealthy(String)`.
The WasmChild adapter hardcodes "degraded"/"unhealthy" as the reason, losing
diagnostic information. When revising WIT for Phase 2, change to:

```wit
record child-health {
    status: health-status,
    reason: option<string>,
}
enum health-status { healthy, degraded, unhealthy }
```

### Future: static mut deprecated in Rust 2024 edition

**Audit ref:** 2.3 (minor)

`patina-plugin-api/src/lib.rs` uses `static mut PLUGIN` for the guest-side
singleton. This is correct for WASM (single-threaded) but `static mut` is
deprecated in Rust 2024 edition. When upgrading from edition 2021, replace
with `UnsafeCell<Option<Box<dyn MotherChildPlugin>>>` or equivalent.

---

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-11 | draft | Created from bible session. Consumes from 5 frozen specs. Concrete 5-phase build with grammars first. |
| 2026-02-11 | amended | Session [[20260211-133159]]: Reordered phases — MotherChild first, grammars last. Architecture changed to Option C (shared PluginEngine). Rationale: grammars have highest coupling/regression risk. |
| 2026-02-11 | ready | Session [[20260211-143337]]: Full spec lockdown. Resolved all open questions. Pinned wasmtime v43 (sync, no async feature). Locked threading model (scoped threads). Locked WIT definitions (host.wit, mother-child.wit). Locked models child scope (path resolution only). Locked file paths, struct shapes, bindgen strategy. Incorporated all research from sessions [[20260205-102402]], [[20260205-115835]], [[20260205-130049]]. All 14 beliefs linked. |
| 2026-02-11 | amended | Session [[20260211-185411]]: Version corrected v43→v41 (v43 doesn't exist on crates.io, latest is 41.0.3). wasmtime-wasi deferred to Phase 2+ — Phase 1 mother-child world only imports patina:host/log (self-implemented, no WASI needed). Feature name corrected preview2→p2. Clarified async Cargo feature vs async_support(true) runtime distinction. Minimum Rust corrected 1.91→1.90. |
| 2026-02-12 | discoveries | Session [[20260212-075642]]: Full WASM audit completed. 4 Phase 2+ discoveries pushed inbound from [[plugin-system-audit-remediation]]: WIT types inside world block (Phase 2), re-entrancy invariant (Phase 2+), ChildHealth reason string (Phase 2), static mut edition migration (future). See audit report: `layer/surface/reports/audit/2026-02-12-plugin-system-phase1.md`. |
| 2026-02-12 | amended | Session [[20260212-083400]]: Audit remediation (I1). 5 spec text inaccuracies documented — spec body preserved as historical record, corrections here: **(1)** Phase 1 Cargo.toml section says "no wasmtime-wasi" — wasmtime-wasi IS required in Phase 1 because wasm32-wasip2 components always import basic WASI interfaces even for pure computation (see belief [[wasm32-wasip2-always-imports-wasi]]). **(2)** Files Created lists `patina-plugin-api/Cargo.toml` with `crate-type = ["cdylib"]` — cdylib belongs on `patina-plugin-models`, not the API crate. The API crate is a library dependency. **(3)** Files Modified lists `src/commands/mother/registry.rs` for "WasmChild adapter" — WasmChild lives entirely in `src/plugin/internal.rs`. The registry needed no modification (already accepts `Box<dyn MotherChild>`). **(4)** Files Created shows `[package.metadata.component]` for patina-plugin-api — not present or needed; `wit_bindgen::generate!` uses explicit `path:` parameter, not cargo-component metadata. **(5)** WIT layout shows `wit/host.wit` at top level — actual location is `wit/deps/patina-host/host.wit` per WIT dependency resolution convention (imported packages live in `deps/`). Benchmark results: PluginEngine::new() 1.36ms (<100ms PASS), handle() 0.002ms (<1ms PASS), Component::new() 73.47ms, instantiate_child() 0.44ms. All exit criteria met. |
