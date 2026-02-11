---
type: feat
id: plugin-system
status: draft
created: 2026-02-11
revised: 2026-02-11
sessions:
  origin: 20260211-125648
  amended: 20260211-133159
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
  - de-risk-runtime-with-simplest-payload
  - mother-is-the-daemon
references:
  - "wasmtime (Bytecode Alliance)"
  - "zed-industries/zed extension system"
  - "WIT Component Model"
  - "tree-sitter WASM grammars"
---

# feat: Plugin System

> wasmtime + WIT. MotherChild becomes the first WASM plugin.
> PluginEngine is the shared runtime. Mother uses it for daemon children.
> CLI uses it for one-shot commands. Grammars come last.

## Problem

Patina is a 52MB monolith. Every feature — forge, eval, yolo, 9 compiled-in
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
│  ├── wasmtime::Engine (shared, one per process)            │
│  ├── load_wasm(path) → instance                           │
│  ├── capability_check(manifest, grants)                    │
│  └── call(instance, function, args) → result              │
│                                                            │
│  Mother daemon                    CLI direct               │
│  ├── PluginEngine                 ├── PluginEngine         │
│  ├── ChildRegistry                └── load on demand       │
│  │   └── MotherChild (WASM)           doctor, eval, yolo   │
│  │       resident, heartbeat          run and exit          │
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

Mother's role is clear: **Mother is the daemon that runs long-lived plugins
and provides cross-project awareness.** She uses the same PluginEngine
everyone else does, but adds the resident lifecycle (load, tick, health,
toys). She's not the gatekeeper for all plugins — she's the home for
plugins that need to stay alive.

CLI plugins don't need Mother running. `patina doctor` works offline and
daemonless, same as today.

### Separate Worlds Per Plugin Type

Per [[separate-worlds-for-isolation]], each plugin type gets its own WIT world
with only the imports it needs. Oracle plugins can't see HTTP imports.
Grammar plugins can't see the eventlog.

| World | Exports | Imports | Capabilities |
|-------|---------|---------|-------------|
| `mother-child` | `handle()`, `health()`, `tick()` | `patina:host/*` | Eventlog, layer, database |
| `command` | `run(args)` → exit code | `patina:host/*` | Full host access |
| `oracle` | `query()`, `name()`, `is-available()` | none | Pure computation |
| `scraper` | `scrape-file()`, `patterns()` | `wasi:filesystem` (read-only) | Filesystem read |
| `forge-reader` | `list-issues()`, `get-issue()`, etc. | `wasi:http` | Network access |
| `grammar` | tree-sitter parse API | none | Pure computation, fully sandboxed |

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
2. **Host decides** — PluginEngine checks manifest against user's grant config

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

### Phase 1: PluginEngine + First MotherChild (v0.17.0)

**Goal:** Add wasmtime, build PluginEngine, implement the first MotherChild
(models) as a WASM plugin. This proves the full host↔plugin communication
pattern — WIT interfaces, host functions, capability grants — where the
trait already exists.

**Why first:** The MotherChild trait (`src/mother/child.rs`) already defines
the plugin shape. The ChildRegistry already loads, routes, and health-checks
children. The MotherHost trait already defines the capability surface. Building
the first child as WASM lets the PluginEngine API emerge from a real use case,
not a spec diagram. The `models` child from [[mother-environment]] is the
simplest — it owns embedding model paths and serves embed requests. No
network, no filesystem writes, minimal capabilities.

**Why NOT grammars first:** The original spec assumed "no host imports =
simplest." Session [[20260211-133159]] discovered this was wrong. Grammars are
entangled with tree-sitter ABI versioning (the reason we vendor and compile C
sources directly in patina-metal), the hot scrape pipeline, and 8 language
processors. Grammar WASM is the highest regression risk, not the lowest. The
ABI version constraint (tree-sitter 0.24 expects ABI 13-14, C/C++ grammars
generate ABI 15) applies equally to WASM grammars — moving to WASM doesn't
fix the underlying version problem, just changes where it manifests.

**Build steps:**

1. Add `wasmtime` to Cargo.toml
2. Create `src/plugin/mod.rs` — PluginEngine struct with `wasmtime::Engine`
3. Create `src/plugin/internal.rs` — WASM loading, capability checking
4. Define `patina:host@0.1.0` WIT package — `log`, `layer`, `database` interfaces
5. Define `patina:mother-child@0.1.0` WIT world — exports `handle()`, `health()`, `tick()`; imports `patina:host/*`
6. Create `patina-plugin-api` crate — ergonomic Rust wrapper over WIT bindings (like Zed's `zed_extension_api`)
7. Implement models child as WASM plugin using `patina-plugin-api`
8. Mother's ChildRegistry loads WASM children via PluginEngine
9. Implement plugin.toml manifest parsing
10. Implement two-layer capability grant checking

**Acceptance criteria:**

- [ ] `wasmtime` compiles and links
- [ ] PluginEngine initializes wasmtime::Engine in <100ms
- [ ] `patina-plugin-api` crate compiles to `wasm32-wasip2` target
- [ ] Models child loads as WASM plugin in Mother daemon
- [ ] `handle("resolve_model", ...)` returns model path through WASM boundary
- [ ] `health()` returns correct status through WASM boundary
- [ ] Capability check: plugin without `host_database` grant cannot call `database.query()`
- [ ] Plugin sees sync APIs; host handles any async internally ([[sync-first]])
- [ ] WASI sandbox: plugin filesystem access scoped to its work directory
- [ ] Plugin crash doesn't crash the host (WASM isolation)
- [ ] `patina mother status` shows WASM-loaded child with health

**Exit criteria:**

- [ ] Round-trip latency through WASM boundary <1ms for `handle()` calls
- [ ] At least one MotherChild loaded from WASM in CI test

### Repos Child (Phase 1+)

The repos child from [[mother-repos]] is the second MotherChild after models.
It owns ref repo lifecycle: git pull, scrape, index, freshness monitoring.

**Build after Phase 1 proves the pattern.** Repos child needs more capabilities
than models (shell commands for git, scrape pipeline access) and is a good
test of the toy system (child requests work, Mother runs it).

Not a separate phase — it's the natural second child once the MotherChild WASM
pattern works.

### Phase 2: Command Plugins — First Extraction (v0.17.0)

**Goal:** Extract `doctor` (278 lines) from the binary into a WASM command
plugin. This proves the `command` world — a plugin that adds CLI subcommands
and runs without Mother.

**Why doctor:** Smallest extractable command. Reads files, checks state,
prints output. No hot-path risk. Proves PluginEngine works for CLI-direct
loading (no daemon required).

**Build steps:**

1. Define `patina:command@0.1.0` WIT world — exports `run(args: list<string>) -> s32`; imports `patina:host/layer` (read-only)
2. Create `patina-doctor` crate (workspace member, compiles to WASM)
3. Move doctor logic from `src/commands/doctor/` to `patina-doctor` crate
4. CLI loads command plugin via PluginEngine when `patina doctor` is invoked
5. Feature-gate compiled-in doctor during transition

**Acceptance criteria:**

- [ ] `patina doctor` works identically from WASM plugin
- [ ] Works without Mother daemon running
- [ ] `patina plugin list` shows patina-doctor with version and status
- [ ] Main binary smaller with doctor extracted (measurable delta)

### Phase 3: Remaining Command Extractions (v0.18.0)

**Goal:** Extract yolo, eval+bench, report, upgrade into WASM command plugins.
These are the remaining "Definitely Plugin" modules from [[patina-identity]].

**Why yolo next:** Per [[patina-identity]], yolo is "the strongest extraction
candidate." 1,613 lines of devcontainer generation that isn't knowledge
infrastructure. After doctor proves the pattern, yolo proves it at scale.

| Plugin | Lines | World | Capabilities |
|--------|-------|-------|-------------|
| `patina-yolo` | 1,613 | command | host_layer (read), environment detection |
| `patina-eval` | 2,476 | command | host_database (read), host_layer (read) |
| `patina-bench` | 753 | command | host_database (read) |
| `patina-report` | ~400 | command | host_layer (read), host_database (read) |
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

### Phase 5: Grammar Plugins (v0.20.0)

**Goal:** Load tree-sitter grammars from WASM instead of compiling them in.
This is the most complex WASM integration due to tree-sitter ABI versioning
and the hot scrape pipeline.

**Why last:** Grammars are entangled with:
- **tree-sitter ABI versioning** — our tree-sitter 0.24 expects ABI 13-14;
  this constraint caused us to vendor and compile C sources directly in
  patina-metal. WASM grammars have the same ABI constraint.
- **The scrape hot path** — 8 language processors in
  `src/commands/scrape/code/languages/*.rs` all call
  `Metal.tree_sitter_language_for_ext()`. Changes here risk regression.
- **patina-metal build system** — the `cc::Build` + vendored grammar
  architecture was built specifically to work around version hell.

By Phase 5, PluginEngine is proven. The only new complexity is the
tree-sitter-specific WASM loading, which can be isolated to `patina-metal`.

**Build steps:**

1. Enable tree-sitter `wasm` feature in patina-metal (uses wasmtime internally — already in tree from Phase 1)
2. Create grammar loading path in `patina-metal/src/metal.rs` — try `~/.patina/grammars/*.wasm` first
3. Fall back to compiled-in grammar when WASM not found (zero regression)
4. `patina grammar list` — show loaded grammars with source (wasm/compiled)
5. `patina grammar install <path>` — copy WASM file to `~/.patina/grammars/`
6. Ship pre-built grammar WASM files matching tree-sitter 0.24 ABI

**Acceptance criteria:**

- [ ] `patina scrape code` uses WASM grammar when present in `~/.patina/grammars/`
- [ ] Falls back to compiled-in grammar when WASM not present
- [ ] Adding a new language is: drop a `.wasm` file, no recompile
- [ ] `patina grammar list` shows available grammars with source (wasm/compiled)
- [ ] WASM grammar parse speed within 2x of compiled-in (acceptable for scrape)

**Exit criteria:**

- [ ] At least one grammar loaded from WASM in CI test
- [ ] Binary size does not grow (grammars stay compiled-in as fallback until Phase 5 is stable)

## What We Don't Build

Per [[patina-identity]] "What Patina IS NOT":

- **Plugin registry/marketplace** — manual install first. Registry is a future spec.
- **Hot reloading** — restart patina to load new plugins. KISS.
- **Plugin dependencies** — plugins don't depend on other plugins. No dependency hell.
- **adapter.wit** — adapters stay compiled-in for now. Mother may manage them later.
- **patina-work plugin** — beads-like work tracking is a future plugin, not this spec.
- **Agent system** — per [[agents-and-yolo]], defer indefinitely.

## Key Design Decisions

### 1. MotherChild first, grammars last

The trait already exists. The registry already works. The host capability
surface already exists. Building the first MotherChild as WASM lets the
PluginEngine API emerge from a real use case. Grammars are entangled with
tree-sitter ABI versioning and the scrape hot path — highest regression risk,
not lowest. Per [[de-risk-runtime-with-simplest-payload]]: de-risk means
de-risk the *plugin system*, not the *tree-sitter integration*.

**Amendment rationale:** Session [[20260211-133159]] discovered that grammars
are the most coupled existing subsystem (ABI versioning, patina-metal build,
8 language processors), not the least. The original ordering assumed "no host
imports = simplest" but ignored infrastructure coupling.

### 2. PluginEngine is shared, Mother is the daemon face (Option C)

PluginEngine holds the wasmtime::Engine, loads WASM modules, checks
capabilities, calls functions. Both Mother and CLI use the same engine.
The difference is lifecycle:
- Mother: load on startup, tick on heartbeat, unload on shutdown (resident)
- CLI: load on demand, run, exit (one-shot)

This avoids requiring Mother for basic commands (`patina doctor` works
standalone) while giving Mother the same plugin infrastructure.

### 3. Separate crates, not separate repos

Plugins are workspace members in the patina monorepo. `Cargo.toml` workspace
includes `patina-doctor`, `patina-yolo`, `patina-eval`, etc. They compile to
WASM but live next to the code they came from. Separate repos are for
community plugins.

### 4. Feature flags during transition

Compiled-in versions stay behind `--features bundled-yolo` etc. during
transition. This means we can ship WASM plugins while keeping the compiled-in
fallback. Remove feature flags once WASM versions are stable.

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

During Phase 5, if a WASM grammar isn't found in `~/.patina/grammars/`,
fall back to the compiled-in grammar. This means zero regression for existing
users. Grammars are opt-in WASM, not forced migration.

## Open Questions

1. **Plugin discovery for CLI commands** — When user types `patina yolo`,
   how does the CLI know to dispatch to a plugin? Options:
   - Scan `~/.patina/plugins/` at startup (slow if many plugins)
   - Manifest cache file listing installed plugin commands (fast)
   - Clap's external subcommand mechanism

2. **Plugin size budget** — Is there a maximum acceptable .wasm file size?
   ONNX model is 90MB. A grammar is ~500KB. Where's the line?

3. **Cross-platform WASM** — Do plugins compiled on macOS run on Linux?
   (Yes for pure WASM, but WASI capabilities may differ)

4. **Grammar build pipeline** — How do we build tree-sitter WASM grammars?
   tree-sitter has `tree-sitter build --wasm` but needs emscripten or
   wasi-sdk. Deferred to Phase 5.

5. **wasmtime version pinning** — wasmtime moves fast (major releases
   every ~3 months). Pin to a specific major version and update deliberately.

## Non-Goals

- Plugin monetization
- Plugin sandboxing beyond WASI (no seccomp, no namespace isolation)
- Multi-version plugin support (one version at a time, uninstall old, install new)
- Plugin auto-update (manual for now)

## Status Log

| Date | Status | Note |
|------|--------|------|
| 2026-02-11 | draft | Created from bible session. Consumes from 5 frozen specs. Concrete 5-phase build with grammars first. |
| 2026-02-11 | amended | Session [[20260211-133159]]: Reordered phases — MotherChild first, grammars last. Architecture changed to Option C (shared PluginEngine). Rationale: grammars have highest coupling/regression risk (ABI versioning, patina-metal, scrape hot path), MotherChild has lowest (trait exists, registry exists, host capability surface exists). |
