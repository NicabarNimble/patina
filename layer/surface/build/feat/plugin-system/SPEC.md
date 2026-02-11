---
type: feat
id: plugin-system
status: draft
created: 2026-02-11
sessions:
  origin: 20260211-125648
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
  - sync-first
  - use-whats-in-the-tree
  - work-triages-specs
references:
  - "wasmtime (Bytecode Alliance)"
  - "zed-industries/zed extension system"
  - "WIT Component Model"
  - "tree-sitter WASM grammars"
---

# feat: Plugin System

> wasmtime + WIT. MotherChild becomes the first plugin interface.
> Grammars become the first WASM payloads. Small core, extensible surface.

## Problem

Patina is a 52MB monolith. Every feature — forge, eval, yolo, 6 compiled-in
tree-sitter grammars — ships in one binary. The consequences:

1. **No extensibility** — adding a language means recompiling patina
2. **No community** — others can't extend patina without forking
3. **Binary bloat** — tree-sitter grammars alone are significant
4. **Slow iteration** — changing eval means releasing all of patina
5. **Mother has zero children** — the MotherChild trait exists but nothing implements it

The MotherChild trait (`src/mother/child.rs`) already defines the plugin shape:
`name()`, `on_load()`, `handle()`, `health()`, `tick()`. The WIT interfaces
are sketched in the frozen [[wit-interfaces]] exploration. The runtime decision
(wasmtime) is made. What's missing is the concrete build.

## Consumes From Frozen Specs

This spec selectively consumes from frozen specs per [[work-triages-specs]]:

| Frozen Spec | What we consume | What we leave |
|-------------|----------------|---------------|
| [[patina-platform]] | wasmtime decision, two-layer capability grants, core/plugin boundary table, plugin manifest format, plugin lifecycle | Work plugin (patina-work), plugin registry, distribution/marketplace |
| [[wit-interfaces]] | oracle.wit, embedding.wit, forge.wit, scraper.wit, host.wit interfaces, sync/async transparency pattern, WASI sandboxing pattern | adapter.wit (adapters stay compiled for now), work.wit (deferred) |
| [[agents-and-yolo]] | Yolo extraction decision (extract to plugin) | Agent concept (defer indefinitely) |
| [[mother-environment]] | Models child design (MotherChild for embedding models) | Cold-start optimization (separate concern) |
| [[mother-repos]] | Repos child design (MotherChild for ref repo lifecycle) | Belief extraction from ref repos (future) |

Specs not consumed after this build completes are candidates for archival.

## Solution

### Runtime: wasmtime + WIT Component Model

**Not** Extism. wasmtime is production-proven (Zed, Fastly, Fermyon), supports
WIT Component Model for typed interfaces, and the Bytecode Alliance maintains it.

Already in the ecosystem:
- tree-sitter uses wasmtime for WASM grammars
- Zed uses wasmtime for extensions (77 WIT files, studied in [[wit-interfaces]])

### Architecture

```
┌───────────────────────────────────────────────────────────┐
│                     patina (core binary)                   │
│                                                            │
│  PluginHost                                                │
│  ├── wasmtime::Engine (shared, one per process)            │
│  ├── plugins: Vec<LoadedPlugin>                            │
│  │   ├── manifest (plugin.toml)                            │
│  │   ├── instance (wasmtime::Instance)                     │
│  │   └── capabilities (granted set)                        │
│  └── grammars: Vec<LoadedGrammar>                          │
│      ├── tree-sitter WASM modules                          │
│      └── language → grammar mapping                        │
│                                                            │
│  Mother daemon loads PluginHost on startup.                │
│  CLI commands load PluginHost on demand.                   │
└───────────────────────────────────────────────────────────┘
        │              │              │
        ▼              ▼              ▼
   ┌─────────┐   ┌─────────┐   ┌─────────┐
   │ grammars│   │ children│   │ plugins │
   │ (.wasm) │   │ (.wasm) │   │ (.wasm) │
   │         │   │         │   │         │
   │ rust.wasm   │ models  │   │ yolo    │
   │ python.wasm │ repos   │   │ eval    │
   │ go.wasm │   │         │   │ report  │
   │ ...     │   │         │   │ doctor  │
   └─────────┘   └─────────┘   └─────────┘
   ~/.patina/    Mother-managed   ~/.patina/
   grammars/     (MotherChild)    plugins/
```

### Separate Worlds Per Plugin Type

Per [[separate-worlds-for-isolation]], each plugin type gets its own WIT world
with only the imports it needs. Oracle plugins can't see HTTP imports.
Grammar plugins can't see the eventlog.

| World | Exports | Imports | Capabilities |
|-------|---------|---------|-------------|
| `grammar` | tree-sitter parse API | none | Pure computation, fully sandboxed |
| `oracle` | `query()`, `name()`, `is-available()` | none | Pure computation |
| `scraper` | `scrape-file()`, `patterns()` | `wasi:filesystem` (read-only) | Filesystem read |
| `forge-reader` | `list-issues()`, `get-issue()`, etc. | `wasi:http` | Network access |
| `mother-child` | `handle()`, `health()`, `tick()` | `patina:host/*` | Eventlog, layer, database |
| `command` | `run(args)` → exit code | `patina:host/*` | Full host access |

### Plugin Manifest (plugin.toml)

```toml
[plugin]
name = "patina-eval"
version = "0.1.0"
description = "Retrieval quality evaluation and benchmarking"
world = "command"                    # Which WIT world
patina_min = "0.17.0"               # Minimum core version

[capabilities]
# Only what this plugin needs — host checks against granted set
host_database = true                 # SQLite read access
host_layer = true                    # Layer file read access

[provides]
commands = ["eval", "bench"]         # Adds `patina eval`, `patina bench`
```

### Two-Layer Capability Grants

Per [[two-layer-capability-grants]]:

1. **Manifest declares** — plugin.toml says what capabilities it wants
2. **Host decides** — PluginHost checks manifest against user's grant config

```toml
# ~/.patina/plugin-config/grants.toml
[patina-eval]
host_database = true
host_layer = true

[patina-forge-gitlab]
wasi_http = true          # Network access for GitLab API
```

Plugins that request capabilities not in the grant config are loaded in
degraded mode (capabilities denied, plugin notified via on_load error).

## Phased Build

### Phase 1: Grammar Plugins (v0.17.0)

**Goal:** Load tree-sitter grammars from WASM instead of compiling them in.
This is the simplest WASM integration — grammars are pure computation with
no host imports, no capabilities, no plugin manifest.

**Why first:** Grammars are already designed for WASM. tree-sitter has
`tree_sitter::wasmtime` support. This exercises the wasmtime integration
without the complexity of host↔plugin communication.

**Build steps:**

1. Add `wasmtime` to Cargo.toml (the one new dependency that pays for itself)
2. Create `src/plugin/mod.rs` — PluginHost struct with `wasmtime::Engine`
3. Create `src/plugin/grammar.rs` — grammar loading from `~/.patina/grammars/*.wasm`
4. Refactor `scrape code` to load grammars dynamically instead of compiled-in
5. Ship pre-built grammar WASM files for current 6 languages (Rust, Python, Go, TypeScript, Java, C)
6. `patina grammar list` — show loaded grammars
7. `patina grammar install <path>` — copy WASM file to `~/.patina/grammars/`
8. Keep compiled-in grammars as fallback if WASM grammar not found

**Acceptance criteria:**

- [ ] `wasmtime` compiles and links
- [ ] `patina scrape code` uses WASM grammar for Rust when `~/.patina/grammars/tree-sitter-rust.wasm` exists
- [ ] Falls back to compiled-in grammar when WASM not present
- [ ] Adding a new language is: drop a `.wasm` file, no recompile
- [ ] `patina grammar list` shows available grammars with source (wasm/compiled)
- [ ] Binary size does not grow (grammars no longer compiled in once WASM versions ship)

**Exit criteria:**

- [ ] wasmtime::Engine initializes in <100ms
- [ ] WASM grammar parse speed within 2x of compiled-in (acceptable for scrape)
- [ ] At least one grammar loaded from WASM in CI test

### Phase 2: MotherChild as WASM Plugin (v0.18.0)

**Goal:** Implement the first MotherChild (models) as a WASM plugin, proving
the host↔plugin communication pattern. This is the hard part — WIT interfaces,
host functions, capability grants.

**Why second:** Mother children are the designed plugin boundary. The trait
already exists. The `models` child from [[mother-environment]] is the simplest —
it owns embedding model paths and serves embed requests. No network, no
filesystem writes, minimal capabilities.

**Build steps:**

1. Define `patina:host@0.1.0` WIT package — `eventlog`, `layer`, `database` interfaces
2. Define `patina:mother-child@0.1.0` WIT world — exports `handle()`, `health()`, `tick()`; imports `patina:host/*`
3. Create `patina-plugin-api` crate — ergonomic Rust wrapper over WIT bindings (like Zed's `zed_extension_api`)
4. Implement models child as WASM plugin using `patina-plugin-api`
5. PluginHost loads models child from `~/.patina/plugins/patina-models.wasm`
6. Mother daemon delegates to WASM child instead of compiled-in child
7. Implement plugin.toml manifest parsing
8. Implement two-layer capability grant checking

**Acceptance criteria:**

- [ ] `patina-plugin-api` crate compiles to `wasm32-wasip2` target
- [ ] Models child loads as WASM plugin in Mother daemon
- [ ] `handle("resolve_model", ...)` returns model path through WASM boundary
- [ ] `health()` returns correct status through WASM boundary
- [ ] Capability check: plugin without `host_database` grant cannot call `database.query()`
- [ ] Plugin sees sync APIs; host handles any async internally ([[sync-first]])
- [ ] WASI sandbox: plugin filesystem access scoped to its work directory

**Exit criteria:**

- [ ] Round-trip latency through WASM boundary <1ms for `handle()` calls
- [ ] Plugin crash doesn't crash the host (WASM isolation)
- [ ] `patina mother status` shows WASM-loaded child with health

### Phase 3: First Extraction — Yolo (v0.18.0)

**Goal:** Extract `yolo` (1,613 lines) from the binary into a WASM command
plugin. This proves the `command` world — a plugin that adds CLI subcommands.

**Why yolo:** Per [[patina-identity]], yolo is "the strongest extraction
candidate." Devcontainer generation isn't knowledge infrastructure. It has
no dependencies on core internals beyond environment detection.

**Build steps:**

1. Define `patina:command@0.1.0` WIT world — exports `run(args: list<string>) -> s32`; imports `patina:host/layer` (read-only)
2. Create `patina-yolo` crate (workspace member, compiles to WASM)
3. Move yolo logic from `src/commands/yolo/` to `patina-yolo` crate
4. PluginHost registers command plugins, dispatches `patina yolo` to WASM
5. Remove yolo from main binary (behind feature flag first, then fully)
6. Ship `patina-yolo.wasm` in `~/.patina/plugins/` via `patina plugin install`

**Acceptance criteria:**

- [ ] `patina yolo --defaults` works identically from WASM plugin
- [ ] `patina yolo` output unchanged from user perspective
- [ ] Main binary smaller with yolo extracted
- [ ] `patina plugin list` shows patina-yolo with version and status

### Phase 4: Oracle & Scraper Plugins (v0.19.0)

**Goal:** Make the serve and capture pipelines extensible. Third-party oracles
and scrapers can be loaded as WASM plugins.

**Build steps:**

1. Define `patina:oracle@0.1.0` WIT world (from [[wit-interfaces]] — already sketched)
2. Define `patina:scraper@0.1.0` WIT world (from [[wit-interfaces]])
3. Refactor `retrieval/oracle.rs` — oracle fusion queries both compiled-in and WASM oracles
4. Refactor `scrape code` — scraper pipeline checks for WASM scrapers matching file extension
5. Create example oracle plugin (e.g., regex-based search as proof of concept)
6. Create example scraper plugin (e.g., Markdown heading extractor)

**Acceptance criteria:**

- [ ] WASM oracle participates in scry fusion alongside compiled-in oracles
- [ ] WASM scraper runs during `patina scrape code` for matching file patterns
- [ ] Oracle plugin: pure computation, no capabilities required
- [ ] Scraper plugin: `wasi:filesystem` read-only, sandboxed to project directory
- [ ] Plugin oracle results appear in `patina scry --explain` with plugin source attribution

### Phase 5: Remaining Extractions (v0.20.0)

**Goal:** Extract eval+bench, report, doctor, upgrade into WASM command plugins.
These are the remaining "Definitely Plugin" modules from [[patina-identity]].

| Plugin | Lines | World | Capabilities |
|--------|-------|-------|-------------|
| `patina-eval` | 2,476 | command | host_database (read), host_layer (read) |
| `patina-bench` | 753 | command | host_database (read) |
| `patina-report` | ~400 | command | host_layer (read), host_database (read) |
| `patina-doctor` | 278 | command | host_layer (read) |
| `patina-upgrade` | 162 | command | wasi:http (check GitHub releases) |

**Build steps:**

1. Extract each module into its own crate (workspace member)
2. Compile to `wasm32-wasip2`
3. Ship as default plugins (bundled with `patina init` or first run)
4. Feature-gate compiled-in versions during transition
5. Remove compiled-in versions once WASM versions are stable

**Acceptance criteria:**

- [ ] All 5 plugins work identically as WASM
- [ ] Binary size reduced measurably (target: <40MB from 52MB)
- [ ] `patina plugin list` shows all default plugins
- [ ] Removing a plugin.wasm file gracefully degrades (command not found, not crash)

## Repos Child (Phase 2+)

The repos child from [[mother-repos]] is the second MotherChild after models.
It owns ref repo lifecycle: git pull, scrape, index, freshness monitoring.

**Build after Phase 2 proves the pattern.** Repos child needs more capabilities
than models (shell commands for git, scrape pipeline access) and is a good
test of the toy system (child requests work, Mother runs it).

Not a separate phase — it's the natural second child once the MotherChild WASM
pattern works.

## What We Don't Build

Per [[patina-identity]] "What Patina IS NOT":

- **Plugin registry/marketplace** — manual install first. Registry is a future spec.
- **Hot reloading** — restart patina to load new plugins. KISS.
- **Plugin dependencies** — plugins don't depend on other plugins. No dependency hell.
- **adapter.wit** — adapters stay compiled-in for now. Mother may manage them later.
- **patina-work plugin** — beads-like work tracking is a future plugin, not this spec.
- **Agent system** — per [[agents-and-yolo]], defer indefinitely.

## Key Design Decisions

### 1. Grammars first, not children first

Grammars are pure computation — no host imports, no capabilities, no plugin
manifest needed. This de-risks the wasmtime integration before tackling the
hard problem (host↔plugin communication). If grammar WASM doesn't work,
we know before investing in WIT interfaces.

### 2. Separate crates, not separate repos

Plugins are workspace members in the patina monorepo. `Cargo.toml` workspace
includes `patina-yolo`, `patina-eval`, etc. They compile to WASM but live
next to the code they came from. Separate repos are for community plugins.

### 3. Feature flags during transition

Compiled-in versions stay behind `--features bundled-yolo` etc. during
transition. This means we can ship WASM plugins while keeping the compiled-in
fallback. Remove feature flags once WASM versions are stable.

### 4. Mother manages plugin lifecycle for daemon plugins

CLI-invoked plugins (yolo, eval, doctor) are loaded on demand by PluginHost.
Daemon plugins (MotherChild) are loaded by Mother on startup and stay resident.
This matches the existing Mother lifecycle: `on_load()` at start, `tick()` on
heartbeat, `on_unload()` at shutdown.

### 5. Sync APIs for plugins, always

Per [[sync-first]] and the Zed pattern: plugins see synchronous APIs.
When the host needs async (e.g., HTTP for forge plugins), the WASM runtime
suspends transparently. No async rust in plugins ever.

```rust
// Plugin sees:
fn query(q: &str, limit: u32) -> Result<Vec<OracleResult>>;

// Host implements (if async needed):
// wasmtime suspends WASM when host yields for I/O
```

### 6. Grammar fallback to compiled-in

During Phase 1, if a WASM grammar isn't found in `~/.patina/grammars/`,
fall back to the compiled-in grammar. This means zero regression for existing
users. Grammars are opt-in WASM, not forced migration.

## Open Questions

1. **Grammar build pipeline** — How do we build tree-sitter WASM grammars?
   tree-sitter has `tree-sitter build --wasm` but needs emscripten or
   wasi-sdk. Document the build process or provide pre-built WASMs.

2. **Plugin discovery for CLI commands** — When user types `patina yolo`,
   how does the CLI know to dispatch to a plugin? Options:
   - Scan `~/.patina/plugins/` at startup (slow if many plugins)
   - Manifest cache file listing installed plugin commands (fast)
   - Clap's external subcommand mechanism

3. **Plugin size budget** — Is there a maximum acceptable .wasm file size?
   ONNX model is 90MB. A grammar is ~500KB. Where's the line?

4. **Cross-platform WASM** — Do plugins compiled on macOS run on Linux?
   (Yes for pure WASM, but WASI capabilities may differ)

## Non-Goals

- Plugin monetization
- Plugin sandboxing beyond WASI (no seccomp, no namespace isolation)
- Multi-version plugin support (one version at a time, uninstall old, install new)
- Plugin auto-update (manual for now)

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-11 | draft | Created from bible session. Consumes from 5 frozen specs. Concrete 5-phase build with grammars first. |
