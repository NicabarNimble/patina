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
  - layer/surface/build/feat/plugin-command-extractions/SPEC.md
  - layer/surface/build/feat/plugin-oracle-scraper/SPEC.md
  - layer/surface/build/feat/plugin-grammars/SPEC.md
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

See `Cargo.toml` for actual wasmtime feature configuration. Key points:
wasmtime-wasi IS required in Phase 1 (wasm32-wasip2 always imports WASI —
see belief [[wasm32-wasip2-always-imports-wasi]] and status log amendment).

**Async Cargo feature vs runtime clarification:** wasmtime-wasi's `p2` feature
enables `wasmtime/async` as a Cargo feature (compile-time only). We never call
`Config::async_support(true)` at runtime. Use `add_to_linker_sync()`.

**Features we do NOT enable:** `async` (runtime), `cache`, `demangle`.

### Target: `wasm32-wasip2`

Plugins compile to `wasm32-wasip2` (WASI Preview 2 = Component Model).
Not `wasm32-wasi` (Preview 1) or `wasm32-unknown-unknown` (too limited).

### Architecture: PluginEngine (Option C)

PluginEngine is the shared wasmtime infrastructure. Mother uses it for
resident daemon children. CLI uses it directly for one-shot command plugins.
Same WASM loading, same capability grants, same manifest format — different
lifecycles.

See `src/plugin/mod.rs` (interface) and `src/plugin/internal.rs` (implementation).

Mother's role per [[mother-is-the-daemon]]: daemon for long-lived plugins with
heartbeat lifecycle. CLI plugins don't need Mother running.

### Threading Model: Sync-First

Per [[sync-first]]: plugins see synchronous APIs, always. No async runtime.
See `src/plugin/internal.rs` for the actual bindgen call (no `async: true`).

**Async escalation path** (if ever needed for host functions):
1. Blocking I/O first (reqwest blocking already in tree)
2. Contained tokio (2 threads, one module) if blocking isn't enough
3. Never: async wasmtime, tokio in public API

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

Package versioning (not directory versioning like Zed's `since_v0.x.0/`).
All WIT files live in `wit/` with deps in `wit/deps/` per WIT convention.

Built files:
- `wit/deps/patina-host/host.wit` — `patina:host@0.1.0` (log interface)
- `wit/mother-child.wit` — `patina:mother-child@0.1.0` (mother-child world)

Future WIT files (Phase 2+): `command.wit`, `oracle.wit`, `scraper.wit`.

**Note:** Versioned import syntax is `patina:host/log@0.1.0` (version on
interface path). Discovered during audit remediation — see status log.

### Bindgen Strategy

Zed's pattern: `wasmtime::component::bindgen!` on host (`src/plugin/internal.rs`),
`wit-bindgen::generate!` on guest (`patina-plugin-api/src/lib.rs`). See those
files for actual bindgen configuration.

### Plugin Manifest (plugin.toml)

See `patina-plugin-models/plugin.toml` for the reference manifest format.
Parsing: `src/plugin/internal.rs` `PluginManifest::from_path()`.

### Two-Layer Capability Grants

Per [[two-layer-capability-grants]]: manifest declares, host decides.
Phase 1: `host_log` auto-granted, all others denied. Future: reads from
`~/.patina/plugin-config/grants.toml`. See `PluginEngine::check_capabilities()`
in `src/plugin/internal.rs`.

### Version In Binary

Per [[version-in-binary]]: API version embedded in WASM binary via link section.
See `patina-plugin-api/src/lib.rs:16-17`. Host reads before instantiation.

### WASI Sandboxing

Per [[wasi-sandboxed-filesystem]]: each plugin gets isolated work directory at
`~/.patina/plugins/{name}/work/`. Plugins that don't declare filesystem
capabilities get no preopened directories. Phase 1 uses minimal WasiCtx with
no filesystem access — see `PluginEngine::instantiate_child()` in
`src/plugin/internal.rs`.

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

### Implementation (built)

All code is built and tested. Key files:

| File | Role |
|------|------|
| `src/plugin/mod.rs` | Public interface (dependable-rust pattern) |
| `src/plugin/internal.rs` | PluginEngine, WasmChild adapter, bindgen, tests |
| `src/commands/mother/daemon.rs` | WASM child discovery, orphaned .toml diagnostic |
| `src/commands/mother/registry.rs` | ChildRegistry with duplicate name check |
| `src/paths.rs` | Plugin path construction (no I/O) |
| `patina-plugin-api/src/lib.rs` | Guest API, MotherChildPlugin trait, register_plugin! macro |
| `patina-plugin-models/src/lib.rs` | Models child (resolve_model, model_status) |
| `patina-plugin-models/plugin.toml` | Reference manifest format |
| `wit/deps/patina-host/host.wit` | patina:host@0.1.0 (log interface) |
| `wit/mother-child.wit` | patina:mother-child@0.1.0 (world definition) |

**No fallback for Phase 1.** Models child is new — if WASM loading fails,
Mother starts without it. Fallback becomes relevant in Phase 2+ when
extracting existing commands.

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

- [x] Round-trip latency through WASM boundary <1ms for `handle()` calls — 0.002ms
- [x] At least one MotherChild loaded from WASM in CI test (`cargo test`)
- [x] `wasmtime::Engine::new()` time measured and documented — 1.36ms

### Repos Child (Phase 1 — second MotherChild)

The repos child from [[mother-repos]] is the second MotherChild after models.
It owns ref repo lifecycle: git pull, scrape, index, freshness monitoring.

Repos child needs more capabilities than models (shell commands for git via
toys, scrape pipeline access) and is a good test of the toy system (child
requests work, Mother runs it).

[[mother-repos]] promoted from `design` to `ready` with Phase 1 boundaries
in session [[20260212-091430]]. Host-fed state pattern: host pushes repo info
via handle(), tick() returns toys. No filesystem/database access needed.

#### Repos Child Exit Criteria

- [x] Repos child implements MotherChild as WASM plugin — `patina-plugin-repos/` (178KB)
- [x] `tick()` detects stale repos and requests re-index toys — pull + scrape toys
- [x] Toy system proven end-to-end (child requests work, Mother runs it) — 4 tests
- [x] At least one repos child test in `cargo test` — 4 tests in `plugin::internal::tests`

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

## Later Phases (extracted to own specs)

Phases 3-5 were extracted to their own specs during session [[20260212-083400]]
because the combined spec was too large. Each links back here and to each other.
Build order is preserved through `blocked_by` relationships.

| Phase | Version | Spec | Summary |
|-------|---------|------|---------|
| 3 | v0.18.0 | [[plugin-command-extractions]] | Extract yolo, eval, bench, report, upgrade to WASM |
| 4 | v0.19.0 | [[plugin-oracle-scraper]] | Extensible oracle + scraper WIT worlds |
| 5 | v0.20.0 | [[plugin-grammars]] | Tree-sitter grammars from WASM |

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

## Discoveries (pushed from final audit review)

Pushed from session [[20260212-093831]] (final audit + design review).
Source: `layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md`

### Design decision: String dispatch within worlds is intentional

**Source:** Post-audit design review, session [[20260212-093831]]
**Scope:** All phases

The `handle()` WIT signature uses `string, string` for action and payload:

```wit
export handle: func(action: string, payload: string) -> result<string, string>;
```

This is JSON-RPC over WASM — we have wasmtime's type system but don't use
typed WIT variants for payloads. This is **intentional**, not accidental.

**The reasoning:** Patina uses separate worlds per plugin *type* (mother-child,
command, oracle, scraper, grammar), diverging from Zed's single `world
extension`. The world boundary provides type safety — an oracle plugin
can't see HTTP imports, a grammar plugin can't see the eventlog. Within a
world, children negotiate payload shapes by convention (JSON).

The alternative — per-child typed WIT variants — would mean per-child
*worlds*, requiring different Linker setups per child instance. That's a
different architecture than "separate worlds per plugin type."

Zed avoids this by putting everything in one world with all capabilities.
We split by type for isolation. Going further to split by instance would
add significant complexity for marginal type safety gain — the real
safety comes from capability isolation at the world level, not payload
types within handle().

Per [[coupling-is-complexity]]: typed payloads couple the WIT definition
to each child's implementation. String dispatch keeps the WIT stable as
children evolve their action sets.

**Document this in code:** Add a comment to `wit/mother-child.wit` above
the `handle` export explaining this design choice.

### Design decision: tick(&mut self) vs handle(&self) split is intentional

**Source:** Post-audit design review, session [[20260212-093831]]
**Scope:** All phases

The `MotherChild` trait has two calling conventions:

- `handle(&self)` — concurrent, request-driven (daemon serves requests)
- `tick(&mut self)` — sequential, time-driven (heartbeat loop)

This forces `ChildRegistry` to use `Arc<RwLock<Box<dyn MotherChild>>>`
to mediate read (handle) vs write (tick) access.

**Zed context:** Zed is purely event-driven — no tick concept, no heartbeat.
All extension methods are effectively `&self`. Patina's daemon heartbeat
is a fundamentally different model.

**Why this split is correct:** Compiled-in children (like `SecretsCacheChild`)
benefit from `&mut self` in tick — they can mutate state directly without
interior mutability overhead. WASM children pay an adapter cost (Mutex
anyway because of wasmtime's `&mut Store` requirement), but that's a cost
of the adapter pattern, not a flaw in the trait.

Making tick() take `&self` would push interior mutability into every child
implementation. The current split lets compiled-in children stay simple.

**Document this in code:** Add a comment to `src/mother/child.rs` above
`tick()` explaining why it takes `&mut self` while `handle()` takes `&self`.

### Pre-community: Toy trust model needs capability gating

**Source:** Post-audit design review, session [[20260212-093831]]
**Scope:** Must resolve before community/third-party plugins

The toy system bypasses the two-layer capability grant system. A WASM child
running in a sandbox with only `host_log` capability can return a `Toy`
with `command: "rm", args: ["-rf", "/"]` and `spawn_toy()` runs it with
the daemon's full privileges.

**Zed context:** Zed enforces per-command capability grants:

```toml
[capabilities]
process = { exec = { command = "ls", args = ["-la"] } }
```

Extensions declare exactly what commands they want. The host's
`CapabilityGranter` checks manifest AND host grants before allowing
execution. Patina's toy system has no equivalent check.

Currently safe because only first-party children exist and their toy
commands are hardcoded (`git`, `patina`). But this must be addressed
before any community plugin support.

**Recommended approach:** Extend `plugin.toml` manifest to declare allowed
toy commands:

```toml
[capabilities]
host_log = true

[capabilities.toys]
commands = ["git", "patina"]
```

Check each toy's command against the manifest allowlist in `spawn_toy()`
or in a new `check_toy()` function. Unrecognized commands are rejected
and logged. This preserves [[two-layer-capability-grants]]: manifest
declares, host decides.

**Not needed for Phase 2** (first-party extraction only). Required before
any third-party or community plugin mechanism.

### Document: PluginEngine is create-once

**Source:** Post-audit design review, session [[20260212-093831]]
**Scope:** Phase 2 (when CLI plugins use PluginEngine directly)

`PluginEngine::new()` creates a fresh `Linker` with WASI and host function
registration. The `wasmtime::Engine` is a `OnceLock` singleton (process-wide),
but the Linker setup runs on each `PluginEngine::new()` call.

**Zed context:** Zed's `WasmHost` (equivalent) is explicitly `Arc`-shared
across the application. Patina's usage is de facto singleton — `daemon.rs`
creates one and passes by reference. But the API allows creating multiple.

**Fix:** Add a doc comment to `PluginEngine::new()`:

```rust
/// Create a new PluginEngine with host functions registered.
///
/// PluginEngine should be created once per process and reused for all
/// plugin loading. The underlying wasmtime::Engine is a process singleton,
/// but the Linker setup (WASI + host functions) runs on each call.
```

Phase 2 (CLI command plugins) will need to decide whether to share the
daemon's PluginEngine or create a separate one for one-shot CLI use.

### Immediate: Fix spec F0-F3

**Source:** Final audit + review, session [[20260212-093831]]
**Scope:** Immediate — before Phase 2 work begins

Fix spec: `layer/surface/build/fix/plugin-system-final-audit-fixes/SPEC.md`

4 concrete code fixes:

| Fix | What | Files |
|-----|------|-------|
| F0 | Eliminate `unsafe impl Sync` — instance behind Mutex with store | `src/plugin/internal.rs` |
| F1 | Registry RwLock poison recovery (4 sites) | `src/commands/mother/registry.rs` |
| F2 | Toy dedup with spawn failure self-healing | `src/commands/mother/daemon.rs` |
| F3 | WIT consistency CI check in pre-push | `resources/git/pre-push-checks.sh` |

Build order: F0 → F3 → F1 → F2. See fix spec for full details.

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
| 2026-02-12 | amended | Session [[20260212-083400]]: Folded repos child into Phase 1 proper. The spec labeled it "Phase 1+" and said "not a separate phase" but placed it outside Phase 1's exit criteria — an internal contradiction. The audit remediation closed Phase 1 without repos child because the exit criteria didn't include it. Correcting this: repos child IS Phase 1 scope, Phase 1 exit criteria now include it, and Phase 1 is not complete until repos child ships. The original exit criteria (PluginEngine, models child, benchmarks) are met; repos child exit criteria are added. [[mother-repos]] spec needs promotion from `design` to `ready` before building. |
| 2026-02-12 | extracted | Session [[20260212-083400]]: Extracted Phases 3-5 into own specs — spec was too large for a single document. Phase 3 → [[plugin-command-extractions]] (v0.18.0), Phase 4 → [[plugin-oracle-scraper]] (v0.19.0), Phase 5 → [[plugin-grammars]] (v0.20.0). Build order preserved via blocked_by chains. This spec now owns Phases 1-2 only (runtime + first extractions). Resolved Decisions and Discoveries sections remain here as they are runtime-level concerns. |
| 2026-02-12 | phase-1-complete | Session [[20260212-091430]]: Repos child built and tested. [[mother-repos]] promoted to `ready` with Phase 1 scope (host-fed state, no filesystem). `patina-plugin-repos/` crate: 178KB WASM, handle() for report_repo + check_freshness, tick() returns pull + scrape toys for stale repos. 4 integration tests prove toy system end-to-end. All Phase 1 exit criteria (original + repos child) now met. Phase 1 complete. |
| 2026-02-12 | discoveries | Session [[20260212-093831]]: Final audit (0 critical) + post-audit design review with Zed context. 5 discoveries pushed: **(1)** String dispatch in handle() is intentional — world boundary = type safety, string dispatch within world = low coupling (contrasted with Zed's single-world typed approach). **(2)** tick(&mut self) vs handle(&self) split is intentional — compiled-in children benefit from direct mutation, WASM children pay adapter cost (Zed has no tick/heartbeat equivalent). **(3)** Toy trust model needs capability gating before community plugins — Zed enforces per-command grants, Patina toys bypass capability system. **(4)** PluginEngine is create-once — document like Zed's Arc-shared WasmHost. **(5)** Fix spec [[plugin-system-final-audit-fixes]] created: F0 unsafe Sync elimination, F1 registry poison, F2 toy dedup, F3 WIT CI check. See final audit: `layer/surface/reports/audit/2026-02-12-plugin-system-phase1-final.md`. |
